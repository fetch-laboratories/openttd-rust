/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tile scheduler (`RunTileLoop`): the LFSR walk, tile-zero handling and dispatch to the tile
//! loop owners. Map arrays stay in C++. Batch settings are read once per entry and only the map
//! descriptor's allocation metadata spans dispatch; `docs/design/world-state.md` ("Shared
//! map view") gives the interval that permits both.
#![allow(unsafe_code)]

use crate::{ground, industry, services, town, trees, water};
use std::ffi::c_uint;

const FEEDBACKS: [u32; 13] = [
    0xD8F,
    0x1296,
    0x2496,
    0x4357,
    0x8679,
    0x1030E,
    0x206CD,
    0x403FE,
    0x807B8,
    0x0010_04B2,
    0x0020_06A8,
    0x0040_04B2,
    0x0080_0B87,
];
// Serialized by the game-state mutex on both GUI threads; raw save/load access
// occurs only at the original serial boundaries. Never form a Rust reference.
static mut CURSOR: u32 = 0;

/// Stable canonical storage for DATE, old-loader, reset and afterload repair.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tile_loop_cursor() -> *mut u32 {
    &raw mut CURSOR
}

/// Existing industry tables consumed by this entry, borrowed for the batch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IndustryServices {
    pub production: *const industry::Services,
    pub construction: *const industry::ConstructionServices,
    pub lifecycle: *const industry::LifecycleServices,
    pub tiles: *const industry::TileServices,
    pub generation: *const industry::GenerationServices,
}

/// Direct ground owners and one independent native lazy-result context per batch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroundServices {
    pub clear: *const ground::ClearServices,
    pub rail: *const ground::RailServices,
    pub road: *const ground::RoadServices,
    pub road_command: *const crate::road_cmd::CommandContext,
    pub tunnel: *const ground::TunnelServices,
    pub water: *const water::WaterServices,
    pub costs: *const ground::CostServices,
    pub call: *mut ground::Call,
    pub climate: services::LandscapeType,
    pub editor: bool,
    pub rebuild: bool,
}

/// Each native handler remains separately typed; map reads use the shared table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    pub shared: *const services::Services,
    pub trees: *const trees::TreeServices,
    pub industry: IndustryServices,
    pub ground: GroundServices,
    pub town: *const town::Leaves,
    pub station: *const crate::station_lifecycle::Services,
    pub objects: *const crate::objects::ObjectServices,
}

// The measured scheduler budget requires dispatch inside the LFSR loop; an
// uninlined closure cost 47 instructions per visit versus the native 18.
// `AMBIENT` is the batch's `AmbientSoundEffect` feature gate, so a disabled gate costs
// nothing per clear, tree or water tile.
#[allow(clippy::inline_always, clippy::cast_possible_truncation)]
#[inline(always)]
unsafe fn dispatch<const AMBIENT: bool>(
    tile: u32,
    log_x: c_uint,
    size_y: c_uint,
    settings: trees::LoopSettings,
    s: &Services,
    types: &services::TileTypeAccess,
) {
    // SAFETY: Reached tables are immutable and live for the synchronous batch. Owner
    // entries end their accesses before callbacks; only scalar cursor is local.
    let tile_type = types.read(tile);
    let g = &s.ground;
    match tile_type {
        0 => unsafe {
            ground::clear_tile_loop(
                tile,
                g.climate,
                g.editor,
                AMBIENT,
                settings.size_x,
                g.clear,
                s.shared,
                types,
            );
        },
        1 => unsafe {
            ground::rail_tile_loop(
                tile,
                g.climate,
                settings.size_x,
                size_y,
                g.call,
                g.water,
                g.rail,
                s.shared,
                types,
            );
        },
        2 => unsafe {
            ground::road_tile_loop(
                tile,
                g.climate,
                settings.ambient,
                g.rebuild,
                settings.size_x,
                g.call,
                g.costs,
                g.road,
                g.road_command,
                s.shared,
            );
        },
        3 => unsafe { town::house::tile_loop(s.town, s.shared, tile) },
        4 => unsafe {
            let settings = trees::LoopSettings {
                ambient_callback: AMBIENT,
                ..settings
            };
            trees::tile_loop(tile, settings, g.call, s.trees, s.shared, types);
        },
        5 => unsafe {
            crate::station_lifecycle::tile_loop(
                tile,
                g.call,
                &*s.station,
                types,
                settings.size_x,
                size_y,
            );
        },
        // `TileLoop_Void` is exactly `TileLoop_Water(tile)`.
        6 | 7 => unsafe {
            water::water_tile_loop(
                tile,
                tile_type,
                AMBIENT,
                settings.size_x,
                size_y,
                g.call,
                g.water,
                s.shared,
                types,
            );
        },
        8 => unsafe {
            industry::tile_loop(
                crate::tile_type::TileIndex(tile),
                // Existing industry ABI stores legal map logarithms in a byte.
                crate::map_func::Map::from_log_x(log_x),
                g.call,
                &s.industry,
            );
        },
        9 => unsafe { ground::tunnel_tile_loop(tile, g.climate, g.tunnel, s.shared) },
        10 => unsafe {
            crate::objects::openttd_rust_object_tile_loop(s.objects, g.call, tile);
        },
        // The source dispatch table has handlers only for original live tile types.
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

/// Own the source's complete batch, including tile-zero phase and final commit.
/// # Safety
/// Dimensions are legal original map dimensions; cursor is nonzero. All service
/// reached pointers/tables are immutable and live; each native entry owns its lazy
/// result context independently. Calls are serialized with reset and
/// save/load. Callbacks are noexcept; reentry sees the cursor before this batch.
pub(crate) unsafe fn run(
    log_x: c_uint,
    log_y: c_uint,
    settings: trees::LoopSettings,
    services: *const Services,
) {
    // SAFETY: Same contract; the gate was read once for the batch.
    unsafe {
        if settings.ambient_callback {
            run_batch::<true>(log_x, log_y, settings, services);
        } else {
            run_batch::<false>(log_x, log_y, settings, services);
        }
    }
}

/// `run` with the batch's ambient feature gate as a constant.
/// # Safety
/// As `run`.
#[allow(clippy::inline_always)]
#[inline(always)]
unsafe fn run_batch<const AMBIENT: bool>(
    log_x: c_uint,
    log_y: c_uint,
    settings: trees::LoopSettings,
    services: *const Services,
) {
    // SAFETY: The caller supplies a live immutable table; no state borrow spans
    // dispatch. A nested call therefore preserves original commit-after-batch.
    // Copy only immutable pointer/function/settings metadata into an unescaped
    // entry-local value; unused/null child tables are never dereferenced.
    let snapshot = unsafe { *services };
    let s = &snapshot;
    // The batch loads the descriptor once. Its serialized no-reset/no-yield call
    // graph keeps it live; only descriptor metadata, never a map access, spans dispatch.
    let types = crate::map_func::World::new(unsafe { &*s.shared })
        .types
        .without_native_assert();
    let mut tile = unsafe { CURSOR };
    let log_sum = log_x + log_y;
    let feedback = FEEDBACKS[(log_sum - 12) as usize];
    let mut count: c_uint = 1 << (log_sum - 8);
    let size_y: c_uint = 1 << log_y;
    if settings.tick_counter.is_multiple_of(256) {
        unsafe { dispatch::<AMBIENT>(0, log_x, size_y, settings, s, &types) };
        count -= 1;
    }
    while count != 0 {
        unsafe { dispatch::<AMBIENT>(tile, log_x, size_y, settings, s, &types) };
        tile = (tile >> 1) ^ (0_u32.wrapping_sub(tile & 1) & feedback);
        count -= 1;
    }
    unsafe { CURSOR = tile };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGroundLoopServices" ["landscape"] => GroundServices {
        clear,
        rail,
        road,
        road_command,
        tunnel,
        water,
        costs,
        call,
        climate,
        editor,
        rebuild,
    };
    "OpenTTDIndustryLoopServices" ["landscape"] => IndustryServices {
        production,
        construction,
        lifecycle,
        tiles,
        generation,
    };
    "OpenTTDLandscapeLoopServices" ["landscape"] => Services {
        shared,
        trees,
        industry,
        ground,
        town,
        station,
        objects,
    };
}

/// `RunTileLoop` over the retired `tools/simulation/landscape-loop-reference.cpp` fixture's
/// maps (#736 W): every original map size, five tick phases, three start cursors, reentry and
/// both tile-type backends, plus the live LFSR-neighbour mutation, frozen from the unchanged
/// original only (`tile_loop/comparison-oracle.txt`).
#[cfg(test)]
mod comparison_tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use crate::oracle;
    use crate::test_tables::{INDUSTRY_TILES, OBJECTS, ROAD_STOPS, SHARED};
    use std::cell::{Cell, RefCell};
    use std::ptr;

    const ORACLE: &str = include_str!("tile_loop/comparison-oracle.txt");
    /// Station (5) and object (10) kinds; other kinds would dispatch elsewhere.
    const NATIVE_TYPES: [u8; 4] = [5, 10, 10, 5];

    /// The fixture's visit observation and reentry state.
    #[derive(Default)]
    struct Run {
        hash: u64,
        count: u64,
        reenter: bool,
        nested: bool,
        mutate: bool,
        start: u32,
        feedback: u32,
    }
    thread_local! {
        static RUN: RefCell<Run> = RefCell::new(Run::default());
        static TILES: Cell<*mut BaseTileLayout> = const { Cell::new(ptr::null_mut()) };
        static EXTENDED: Cell<*const ExtendedTileLayout> = const { Cell::new(ptr::null()) };
        static ENTRY: Cell<Option<(c_uint, c_uint, trees::LoopSettings, *const Services)>> =
            const { Cell::new(None) };
    }

    fn tile_type(tile: u32) -> u8 {
        // SAFETY: The test's live allocation covers every visited tile.
        unsafe { (*TILES.get().add(tile as usize)).kind >> 4 }
    }
    extern "C" fn shared_tile_type(tile: u32) -> u8 {
        tile_type(tile)
    }
    extern "C" fn station_type(_: u32) -> crate::station_type::StationType {
        crate::station_type::StationType::End
    }
    /// The fixture's `NativeVisit`: hash, count, optional mutation, then one nested entry.
    fn visit(tile: u32, kind: u8) {
        let (mutate, feedback) = RUN.with(|run| {
            let mut run = run.borrow_mut();
            run.hash = (run.hash ^ (u64::from(tile) << 4) ^ u64::from(kind))
                .wrapping_mul(1_099_511_628_211);
            run.count += 1;
            (run.mutate, run.feedback)
        });
        if mutate && tile != 0 {
            let next = (tile >> 1) ^ (0_u32.wrapping_sub(tile & 1) & feedback);
            // SAFETY: The LFSR successor stays inside the live allocation.
            unsafe {
                let cell = TILES.get().add(next as usize);
                (*cell).kind = (NATIVE_TYPES[((next + 1) & 3) as usize] << 4) | ((*cell).kind & 15);
            }
        }
        let nest = RUN.with(|run| {
            let mut run = run.borrow_mut();
            let nest = run.reenter && !run.nested;
            run.nested |= nest;
            nest.then_some(run.start)
        });
        if let Some(start) = nest {
            // Both entries must retain the pre-batch canonical cursor until return.
            // SAFETY: Serialized test access to the scheduler's cursor.
            assert_eq!(unsafe { *openttd_rust_tile_loop_cursor() }, start);
            let (log_x, log_y, settings, services) = ENTRY.get().expect("live entry");
            // SAFETY: The same live tables as the outer entry.
            unsafe { run(log_x, log_y, settings, services) };
        }
    }
    unsafe extern "C" fn station_by_tile(tile: u32) -> *mut crate::station::BaseStation {
        visit(tile, 5);
        ptr::null_mut()
    }
    unsafe extern "C" fn object_policy(tile: u32) -> crate::objects::LoopPolicy {
        visit(tile, 10);
        crate::objects::LoopPolicy {
            spec: ptr::null(),
            object: ptr::null_mut(),
            north: 0,
            flags: 0,
            kind: 0,
        }
    }
    unsafe extern "C" fn not_on_water(_: u32) -> bool {
        false
    }

    /// Run one batch from `start` and return the visit hash, count and committed cursor.
    fn batch(log_x: c_uint, log_y: c_uint, tick: u64, start: u32, raw: bool) -> String {
        let size = 1_u32 << (log_x + log_y);
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: TILES.get(),
                extended: EXTENDED.get(),
                count: if raw { size } else { 0 },
                assertions: true,
            },
            log_x,
        );
        let mut shared = SHARED;
        shared.tile_type = shared_tile_type;
        shared.station_type = station_type;
        shared.map = crate::map_func::fixture::statics();
        // The station table holds `&'static` shared services, as the native facade does.
        let shared: &'static crate::services::Services = Box::leak(Box::new(shared));
        let mut roadstops = ROAD_STOPS;
        roadstops.by_tile = station_by_tile;
        let mut station = crate::station_lifecycle::TEST_TABLE;
        station.shared = shared;
        station.roadstop = &raw const roadstops;
        let mut industry_tiles = INDUSTRY_TILES;
        industry_tiles.on_water = not_on_water;
        let mut objects = OBJECTS;
        objects.shared = shared;
        objects.industry_tiles = &raw const industry_tiles;
        objects.loop_policy = object_policy;
        let services = Services {
            shared,
            trees: ptr::null(),
            industry: IndustryServices {
                production: ptr::null(),
                construction: ptr::null(),
                lifecycle: ptr::null(),
                tiles: ptr::null(),
                generation: ptr::null(),
            },
            ground: GroundServices {
                clear: ptr::null(),
                rail: ptr::null(),
                road: ptr::null(),
                road_command: ptr::null(),
                tunnel: ptr::null(),
                water: ptr::null(),
                costs: ptr::null(),
                call: ptr::null_mut(),
                climate: crate::landscape_type::LandscapeType(0),
                editor: false,
                rebuild: false,
            },
            town: ptr::null(),
            station: &raw const station,
            objects: &raw const objects,
        };
        let settings = trees::LoopSettings {
            tick_counter: tick,
            size_x: 1 << log_x,
            climate: 0,
            extra: 0,
            ambient: false,
            ambient_callback: false,
        };
        ENTRY.set(Some((log_x, log_y, settings, &raw const services)));
        RUN.with(|run| {
            let mut run = run.borrow_mut();
            (run.hash, run.count, run.nested, run.start) = (0, 0, false, start);
        });
        // SAFETY: Serialized test access to the scheduler's cursor and live tables.
        let cursor = unsafe {
            *openttd_rust_tile_loop_cursor() = start;
            run(log_x, log_y, settings, &raw const services);
            *openttd_rust_tile_loop_cursor()
        };
        ENTRY.set(None);
        RUN.with(|run| {
            let run = run.borrow();
            format!("{} {} {cursor}", run.hash, run.count)
        })
    }

    /// Raw station tiles store `StationType::End` in m6 bits 3..6, as `GetStationType` reads them.
    fn station_end_tiles(size: u32) -> Vec<ExtendedTileLayout> {
        (0..size)
            .map(|_| ExtendedTileLayout {
                m6: crate::station_type::StationType::End.0 << 3,
                m7: 0,
                m8: 0,
            })
            .collect()
    }

    /// The fixture's map: kinds cycle station, object, object, station; low bits vary by tile.
    fn map_tiles(size: u32) -> Vec<BaseTileLayout> {
        (0..size)
            .map(|tile| BaseTileLayout {
                kind: (NATIVE_TYPES[(tile % 4) as usize] << 4) | (tile & 15) as u8,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect()
    }

    /// One map of the fixture's matrix: every tick, start and reentry, on one backend.
    fn map_cases(log_x: c_uint, log_y: c_uint, raw: bool) -> Vec<(String, String)> {
        let size = 1_u32 << (log_x + log_y);
        let mut tiles = map_tiles(size);
        TILES.set(tiles.as_mut_ptr());
        let extended = station_end_tiles(size);
        EXTENDED.set(extended.as_ptr());
        let mut cases = Vec::new();
        for tick in [0, 1, 255, 256, u64::MAX] {
            for start in [1, size - 1, 0x5A5] {
                for reenter in [false, true] {
                    RUN.with(|run| run.borrow_mut().reenter = reenter);
                    let inputs = format!(
                        "log_x={log_x} log_y={log_y} tick={tick} start={start} reenter={reenter} raw={raw}"
                    );
                    cases.push((inputs, batch(log_x, log_y, tick, start, raw)));
                }
            }
        }
        TILES.set(ptr::null_mut());
        cases
    }

    thread_local! {
        static CALLS: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
    }
    extern "C" fn fallback(tile: u32) -> u8 {
        CALLS.set((CALLS.get().0 + 1, tile));
        231
    }

    /// The scheduler's shared tile-type reader keeps the original callback for any tile outside
    /// the allocation, and for an empty allocation (the retired `landscape_tile_loop.cpp`
    /// fixture's checks).
    #[test]
    fn tile_type_reader_falls_back_to_callback_out_of_range() {
        let mut tiles = map_tiles(4096);
        let allocation = TileTypeAllocation {
            base: tiles.as_mut_ptr(),
            extended: ptr::null(),
            count: 4096,
            assertions: true,
        };
        let empty = TileTypeAllocation {
            base: ptr::null(),
            extended: ptr::null(),
            count: 0,
            assertions: true,
        };
        CALLS.set((0, 0));
        for (allocation, tile, calls) in [
            (allocation, 4096, 1),
            (allocation, u32::MAX, 2),
            (empty, 0, 3),
        ] {
            // SAFETY: The test's live allocation; out-of-range tiles reach only the callback.
            let read =
                unsafe { crate::map_func::TileTypeAccess::raw(allocation, fallback) }.read(tile);
            assert_eq!((read, CALLS.get()), (231, (calls, tile)), "tile={tile}");
        }
    }

    #[test]
    fn tile_loop_matches_original_scheduler() {
        let oracle = oracle::parse(ORACLE);
        // SAFETY: Serialized test access to the scheduler's cursor.
        let saved = unsafe { *openttd_rust_tile_loop_cursor() };
        for raw in [false, true] {
            let cases = (6..=12)
                .flat_map(|log_x| (6..=12).flat_map(move |log_y| map_cases(log_x, log_y, raw)));
            assert_eq!(oracle::check(&oracle, "loop", cases), 1470);
        }
        // The first callback mutates the next LFSR tile; nested entries mutate the same live
        // allocation. Each backend starts from a fresh map.
        for raw in [false, true] {
            let mut tiles: Vec<_> = (0..4096_u32)
                .map(|tile| BaseTileLayout {
                    kind: NATIVE_TYPES[(tile % 4) as usize] << 4,
                    height: 0,
                    m2: 0,
                    m1: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect();
            TILES.set(tiles.as_mut_ptr());
            let extended = station_end_tiles(4096);
            EXTENDED.set(extended.as_ptr());
            RUN.with(|run| {
                let mut run = run.borrow_mut();
                (run.reenter, run.mutate, run.feedback) = (true, true, 0xD8F);
            });
            let output = batch(6, 6, 1, 1, raw);
            RUN.with(|run| *run.borrow_mut() = Run::default());
            TILES.set(ptr::null_mut());
            assert_eq!(
                oracle::check(&oracle, "mutation", [(format!("raw={raw}"), output)]),
                1
            );
        }
        // SAFETY: Same serialized scalar.
        unsafe { *openttd_rust_tile_loop_cursor() = saved };
    }
}

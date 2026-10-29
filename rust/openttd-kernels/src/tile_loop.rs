/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tile scheduler (`RunTileLoop`): the LFSR walk, tile-zero handling and dispatch to the tile
//! loop owners. Map arrays stay in C++. Batch settings are read once per entry and only the map
//! descriptor's allocation metadata spans dispatch; `docs/design/world-state.md` ("Selected
//! shared map view") gives the interval that permits both.
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
    pub tile_types: services::TileTypeAllocation,
    pub trees: *const trees::TreeServices,
    pub industry: IndustryServices,
    pub ground: GroundServices,
    pub town: *const town::Leaves,
    pub station_loop:
        unsafe extern "C" fn(*const crate::station_lifecycle::Services, u32, *mut ground::Call),
    pub station: *const crate::station_lifecycle::Services,
    pub objects: *const crate::objects::ObjectServices,
}

// The measured scheduler budget requires dispatch inside the LFSR loop; an
// uninlined closure cost 47 instructions per visit versus the native 18.
#[allow(clippy::inline_always, clippy::cast_possible_truncation)]
#[inline(always)]
unsafe fn dispatch(
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
                settings.ambient_callback,
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
        3 => unsafe { town::house::openttd_rust_house_tile_loop(s.town, s.shared, tile) },
        4 => unsafe {
            trees::tile_loop(tile, settings, g.call, s.trees, s.shared, types);
        },
        5 => unsafe {
            (s.station_loop)(s.station, tile, g.call);
        },
        // `TileLoop_Void` is exactly `TileLoop_Water(tile)`.
        6 | 7 => unsafe {
            water::water_tile_loop(
                tile,
                tile_type,
                settings.ambient_callback,
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
                tile,
                // Existing industry ABI stores legal map logarithms in a byte.
                log_x as u8,
                g.call,
                s.industry.production,
                s.industry.construction,
                s.industry.lifecycle,
                s.industry.tiles,
                s.industry.generation,
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_run_tile_loop(
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
    // SAFETY: The native scheduler facade acquires the actual allocation once.
    // Its serialized no-reset/no-yield call graph keeps it live for this batch;
    // only descriptor metadata, never an actual map access, spans dispatch.
    let types = unsafe { services::TileTypeAccess::raw(s.tile_types, (*s.shared).tile_type) };
    let mut tile = unsafe { CURSOR };
    let log_sum = log_x + log_y;
    let feedback = FEEDBACKS[(log_sum - 12) as usize];
    let mut count: c_uint = 1 << (log_sum - 8);
    let size_y: c_uint = 1 << log_y;
    if settings.tick_counter.is_multiple_of(256) {
        unsafe { dispatch(0, log_x, size_y, settings, s, &types) };
        count -= 1;
    }
    while count != 0 {
        unsafe { dispatch(tile, log_x, size_y, settings, s, &types) };
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
        tile_types,
        trees,
        industry,
        ground,
        town,
        station_loop,
        station,
        objects,
    };
}

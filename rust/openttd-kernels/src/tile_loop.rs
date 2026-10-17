/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete tile scheduler. Native arrays stay canonical; existing Rust owners
//! dispatch directly without a native trampoline. All panics abort at the C ABI.
//!
//! Batch settings are invariant under the original serialized game-state mutex:
//! map allocation/load, tick advancement, setting commands and AI/Game script VMs
//! run outside `RunTileLoop`. Ambient callbacks run only the `NewGRF` sound resolver;
//! tile handlers may mutate tiles/pools, run landscape/road/crash commands or queue
//! script events, but none changes map dimensions, climate, tree placement, tick
//! counter, editor mode, road-rebuild or client ambient settings. Generation progress also runs outside this
//! entry. `_misc_grf_features` is changed by reset and GRF `ActionD` loading,
//! both excluded from this batch. The ambient callback feature bit is captured
//! per independent entry, never cached globally or substituted by client audio.
//! Enabled callbacks retain their source RNG position; the tree legacy adapter
//! still rereads the feature bit after its call. Thus settings are entry-local;
//! tree/map observations still occur inside each owner at original callback sites.
//!
//! Raw type descriptor lifetime: `Map::Allocate` is reached only from game reset
//! (`InitializeGame`), MAPS/legacy loaders and tests. Mutex yield (`GameLoopPause`)
//! occurs only in generation progress and GRF scanning; warmup callers place it
//! outside this entry. `VideoDriver::GameLoop`/GUI input hold the game-state mutex;
//! bootstrap is serial. House/ambient/animation `NewGRF` resolvers, road/landscape
//! commands, water foundations/crashes/destruction, cargo and industry callbacks
//! may mutate later tiles and reenter owners, but never run reset/load/progress or
//! a script VM. AI/Game event insertion only queues via `InsertEvent`; script VMs
//! run after this batch. Window invalidation cannot pump GUI commands here.
//! Therefore only allocation metadata can span dispatch; each u8 read ends first.
//! Nested entries acquire independently, and entries after reset reacquire. Link
//! graph workers own copies; sound/compression do not mutate the arrays. Threaded
//! and arbitrary `NewGRF` corpus limits remain source-review obligations in #156.
#![allow(unsafe_code)]

use crate::{ground, industry, services, trees, water};
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
    pub tunnel: *const ground::TunnelServices,
    pub water: *const water::WaterServices,
    pub costs: *const ground::CostServices,
    pub call: *mut ground::Call,
    pub climate: services::LandscapeType,
    pub editor: bool,
    pub rebuild: bool,
    pub ambient_callback: bool,
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
    pub house_loop: extern "C" fn(u32),
    pub station_loop: extern "C" fn(u32),
    pub object_loop: extern "C" fn(u32),
}

// The measured scheduler budget requires dispatch inside the LFSR loop; an
// uninlined closure cost 47 instructions/visit versus the native 18 (#212).
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
                g.ambient_callback,
                settings.size_x,
                g.clear,
                s.shared,
            );
        },
        1 => unsafe {
            ground::rail_tile_loop(
                tile,
                g.climate,
                g.ambient_callback,
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
            ground::openttd_rust_road_tile_loop(
                tile,
                g.climate,
                settings.ambient,
                g.rebuild,
                settings.size_x,
                g.call,
                g.costs,
                g.road,
                s.shared,
            );
        },
        3 => (s.house_loop)(tile),
        4 => unsafe { trees::openttd_rust_trees_tile_loop(tile, settings, s.trees, s.shared) },
        5 => (s.station_loop)(tile),
        // `TileLoop_Void` is exactly `TileLoop_Water(tile)`.
        6 | 7 => unsafe {
            water::water_tile_loop(
                tile,
                tile_type,
                settings.size_x,
                size_y,
                g.ambient_callback,
                g.call,
                g.water,
                s.shared,
                types,
            );
        },
        8 => unsafe {
            industry::openttd_rust_industry_tile_loop(
                tile,
                // Existing industry ABI stores legal map logarithms in a byte.
                log_x as u8,
                s.industry.production,
                s.industry.construction,
                s.industry.lifecycle,
                s.industry.tiles,
                s.industry.generation,
            );
        },
        9 => unsafe { ground::openttd_rust_tunnel_tile_loop(tile, g.climate, g.tunnel, s.shared) },
        10 => (s.object_loop)(tile),
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

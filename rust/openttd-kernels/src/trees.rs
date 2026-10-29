/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tree policy, iteration, grove geometry, commands and the persisted tick byte; tiles stay in
//! C++. Flooding, `NewGRF` ambient callbacks and nested clears may reenter, so records read
//! before them are read again afterwards; the tile loop reads single fields where the source
//! does. World-generation abort is the one C++ exception: its progress service reports it and
//! the entry returns.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_arguments,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
use crate::clear_map::{CLEAR_DESERT, CLEAR_FIELDS, CLEAR_GRASS, CLEAR_ROCKS, CLEAR_ROUGH};
use crate::random_func::chance16_i;
use crate::services::Services;
use crate::tile_type::{INVALID_TILE, MP_CLEAR, MP_TREES, MP_WATER};

use crate::landscape_type::{ARCTIC, TROPIC};
const TOYLAND: u8 = 3;
const GROUND_GRASS: u8 = 0;
const GROUND_ROUGH: u8 = 1;
const GROUND_SNOW_DESERT: u8 = 2;
const GROUND_SHORE: u8 = 3;
const GROUND_ROUGH_SNOW: u8 = 4;
const ZONE_NORMAL: u8 = 0;
const ZONE_DESERT: u8 = 1;
const ZONE_RAINFOREST: u8 = 2;
const GROWING1: u8 = 0;
const GROWN: u8 = 3;
const DEAD: u8 = 6;
const TREE_RAINFOREST: u8 = 20;
const TREE_CACTUS: u8 = 27;
const TREE_SUB_TROPICAL: u8 = 28;
const TREE_TOYLAND: u8 = 32;
const TREE_INVALID: u8 = 255;
const ETP_NO_SPREAD: u8 = 0;
const ETP_SPREAD_RAINFOREST: u8 = 1;
const ETP_SPREAD_ALL: u8 = 2;
const ETP_NO_GROWTH_NO_SPREAD: u8 = 3;
use crate::tree_cmd::TP_NONE;
const TP_ORIGINAL: u8 = 1;
const TP_IMPROVED: u8 = 2;
const DEFAULT_TREE_STEPS: u32 = 1000;
const DEFAULT_RAINFOREST_TREE_STEPS: u32 = 15000;
const EDITOR_TREE_DIV: i32 = 5;
const MAP_HEIGHT_LIMIT_ORIGINAL: u8 = 15;
const RAINFOREST_SOUNDS: [u16; 4] = [66, 67, 68, 72];
const ARCTIC_SNOW_1: u16 = 52;
const ARCTIC_SNOW_2: u16 = 57;
/// `_tileoffs_by_dir` in direction order N, NE, E, SE, S, SW, W, NW.
const DIRECTION_OFFSETS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
];

// Process-lifetime allocation, initialized without runtime construction. Access is
// serialized by the game thread; save/load dereferences its raw address only when
// no Rust call is active. In particular no Rust reference to it is ever formed.
static mut TREE_COUNTER: u8 = 0;

/// Stable address for unchanged DATE and legacy TTD/TTO serialization descriptors.
/// The game thread exclusively serializes access; callers never keep references
/// across a Rust invocation. `LoadCheck` deliberately does not access this byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tree_counter() -> *mut u8 {
    &raw mut TREE_COUNTER
}

/// Reset the private persisted byte when initializing a game.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trees_initialize() {
    // SAFETY: Game initialization is serialized with ticks and save/load.
    unsafe {
        TREE_COUNTER = 0;
    }
}

fn decrement(counter: u8, scale: u32) -> (u8, bool) {
    let next = u32::from(counter).wrapping_sub(scale) as u8;
    (next, counter <= next)
}

/// Fields `CanPlantTreesOnTile`, `GetRandomTreeType` and `PlantTreesOnTile` observe.
/// Clear fields are valid only on clear tiles and `coast` only on water tiles;
/// `coast` is `IsCoast && !IsSlopeWithOneCornerRaised(GetTileSlope)`, so the slope is
/// read only on coast tiles. `bridge` is read on clear and water tiles.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct PlantObservation {
    pub tile_type: u8,
    pub bridge: bool,
    pub ground: u8,
    pub density: u8,
    pub snow: bool,
    pub coast: bool,
    pub zone: u8,
}
impl PlantObservation {
    fn suitable(self, allow_desert: bool) -> bool {
        match self.tile_type {
            MP_WATER => !self.bridge && self.coast,
            MP_CLEAR => {
                !self.bridge
                    && self.ground != CLEAR_FIELDS
                    && self.ground != CLEAR_ROCKS
                    && (allow_desert || self.ground != CLEAR_DESERT)
            }
            _ => false,
        }
    }
}

/// Tree-tile fields of one `MP_TREES` tile.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct TreeObservation {
    pub ground: u8,
    pub density: u8,
    pub species: u8,
    pub count: u8,
    pub growth: u8,
    pub zone: u8,
}

/// Only the tree count and species read by `ClearTile_Trees`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearObservation {
    pub count: u32,
    pub species: u8,
}

/// `TileLoop_Trees` settings, read once per entry. `ambient` is the client sound
/// setting; `ambient_callback` is the `NewGRF` feature gate of `AmbientSoundEffect`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoopSettings {
    pub tick_counter: u64,
    pub size_x: u32,
    pub climate: u8,
    pub extra: u8,
    pub ambient: bool,
    pub ambient_callback: bool,
}

/// `OnTick_Trees` settings, read once per entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TickSettings {
    pub tick_counter: u64,
    pub size: u32,
    pub climate: u8,
    pub extra: u8,
}

/// `GenerateTrees` and `PlaceTreesRandomly` settings, read once per entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GenerateSettings {
    pub size: u32,
    pub size_x: u32,
    pub size_y: u32,
    pub climate: u8,
    pub placer: u8,
    pub height_limit: u8,
    pub editor: bool,
    pub freeform_edges: bool,
}

/// `CmdPlantTree` settings, read once per entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CommandSettings {
    pub build_price: i64,
    pub size: u32,
    pub climate: u8,
    pub editor: bool,
    pub execute: bool,
    pub company_valid: bool,
}

/// `CmdPlantTree` outcome. Status 0 returns the cost, or the indexed message when
/// the cost is zero; 1 is `CMD_ERROR`; 2 returns the failed nested landscape clear.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct CommandResult {
    pub cost: i64,
    pub status: u8,
    pub message: u8,
}

/// Nested `CMD_LANDSCAPE_CLEAR` outcome; C++ keeps the failed `CommandCost`.
#[derive(Clone, Copy, Default, Debug)]
pub struct LandscapeClear {
    pub cost: i64,
    pub failed: bool,
}

/// Opaque C++ command context (flags, company, tile iterator, nested result).
#[repr(C)]
pub struct Command {
    _private: [u8; 0],
}

/// Tree services. Reads are pure. Writes are the named `tree_map.h`/`clear_map.h`
/// functions. Direct water calls, `ambient` and `landscape_clear` can reenter tree
/// entries. The generic owner observes its enable bit after ambient returns. `progress` and
/// `progress_total` return true when world generation was aborted; C++ then holds
/// the exception and rethrows it after the entry returns.
#[repr(C)]
pub struct TreeServices {
    pub plant_observation: extern "C" fn(u32) -> PlantObservation,
    pub tree_observation: extern "C" fn(u32) -> TreeObservation,
    pub tree_count: extern "C" fn(u32) -> u32,
    pub clear_observation: extern "C" fn(u32) -> ClearObservation,
    pub snow_line: extern "C" fn() -> u8,
    pub sin: extern "C" fn(f32) -> f32,
    pub cos: extern "C" fn(f32) -> f32,
    pub make_tree: extern "C" fn(u32, u8, u32, u8, u8, u32),
    pub set_ground_density: extern "C" fn(u32, u8, u32),
    pub add_count: extern "C" fn(u32, i32),
    pub add_growth: extern "C" fn(u32, i32),
    pub set_growth: extern "C" fn(u32, u8),
    pub make_clear: extern "C" fn(u32, u8, u32),
    pub progress: extern "C" fn() -> bool,
    pub progress_total: extern "C" fn(u32) -> bool,
    pub town_rating: extern "C" fn(u16, u32, bool),
    pub command_begin: extern "C" fn(*mut Command, u32, u32, bool) -> i32,
    pub command_tile: extern "C" fn(*mut Command) -> u32,
    pub command_next: extern "C" fn(*mut Command) -> u32,
    pub command_debit: extern "C" fn(*mut Command),
}

/// `Map::ScaleBySize`: `CeilDiv(n << (LogX + LogY - 12), 16)` in uint arithmetic.
fn scale_by_size(size: u32, n: u32) -> u32 {
    n.wrapping_shl(size.trailing_zeros().wrapping_sub(12))
        .wrapping_add(15)
        / 16
}

/// `TileAddWrap` for the generation map geometry.
fn tile_add_wrap(g: &GenerateSettings, tile: u32, x: i32, y: i32) -> u32 {
    crate::map_func::Map::from_log_x(g.size_x.trailing_zeros()).tile_add_wrap(
        tile,
        x,
        y,
        g.size_y,
        g.freeform_edges,
    )
}

/// `GetRandomTreeType`; the zone is used only in the tropic climate.
fn random_tree_type(climate: u8, zone: u8, seed: u32) -> u8 {
    let (base, count) = match climate {
        0 => (0, 12),
        ARCTIC => (12, 8),
        TROPIC => match zone {
            ZONE_NORMAL => (u32::from(TREE_SUB_TROPICAL), 4),
            ZONE_DESERT => return if seed > 12 { TREE_INVALID } else { TREE_CACTUS },
            _ => (u32::from(TREE_RAINFOREST), 7),
        },
        _ => (u32::from(TREE_TOYLAND), 9),
    };
    (seed.wrapping_mul(count) / 256).wrapping_add(base) as u8
}

fn is_rainforest_tree(tree: u8) -> bool {
    (TREE_RAINFOREST..TREE_CACTUS).contains(&tree)
}

#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}
/// `IsPointInTriangle` with the third vertex at the origin.
fn in_triangle(x: i32, y: i32, v1: Point, v2: Point) -> bool {
    let s = v1.x.wrapping_mul(y).wrapping_sub(v1.y.wrapping_mul(x));
    let t = (v2.x.wrapping_sub(v1.x))
        .wrapping_mul(y.wrapping_sub(v1.y))
        .wrapping_sub((v2.y.wrapping_sub(v1.y)).wrapping_mul(x.wrapping_sub(v1.x)));
    if (s < 0) != (t < 0) && s != 0 && t != 0 {
        return false;
    }
    let d = (0_i32.wrapping_sub(v2.x))
        .wrapping_mul(y.wrapping_sub(v2.y))
        .wrapping_sub((0_i32.wrapping_sub(v2.y)).wrapping_mul(x.wrapping_sub(v2.x)));
    (d < 0) == (s.wrapping_add(t) <= 0)
}
/// `IsPointInStarShapedPolygon` over the sixteen grove segments.
fn in_grove(x: i32, y: i32, shape: &[Point; 16]) -> bool {
    (0..16).any(|i| in_triangle(x, y, shape[i], shape[(i + 1) % 16]))
}

/// Borrowed immutable service tables for one entry. They are C++ statics, never
/// world storage, so holding them across reentrant services is sound.
struct Trees<'a> {
    t: &'a TreeServices,
    s: &'a Services,
    map: Option<&'a crate::services::TileTypeAccess>,
}
impl Trees<'_> {
    fn plant_observation(&self, tile: u32) -> PlantObservation {
        (self.t.plant_observation)(tile)
    }
    /// One field read at its original observation point. Without the audited map, or when a
    /// native assertion fails, the record accessor reads it; its first read asserts the same.
    #[inline]
    fn field(
        &self,
        tile: u32,
        read: fn(&crate::services::TileTypeAccess, u32) -> Option<u8>,
        pick: fn(TreeObservation) -> u8,
    ) -> u8 {
        self.map
            .and_then(|map| read(map, tile))
            .unwrap_or_else(|| pick((self.t.tree_observation)(tile)))
    }
    fn ground(&self, tile: u32) -> u8 {
        self.field(tile, crate::tree_map::get_tree_ground, |t| t.ground)
    }
    fn density(&self, tile: u32) -> u8 {
        self.field(tile, crate::tree_map::get_tree_density, |t| t.density)
    }
    fn species(&self, tile: u32) -> u8 {
        self.field(tile, crate::tree_map::get_tree_type, |t| t.species)
    }
    fn count(&self, tile: u32) -> u8 {
        self.field(tile, crate::tree_map::get_tree_count, |t| t.count)
    }
    fn growth(&self, tile: u32) -> u8 {
        self.field(tile, crate::tree_map::get_tree_growth, |t| t.growth)
    }
    fn zone(&self, tile: u32) -> u8 {
        self.field(tile, crate::tile_map::get_tropic_zone, |t| t.zone)
    }
    /// `TreesOnTileCanSpread`; the zone is read only in the tropic climate.
    fn can_spread(&self, tile: u32, s: &LoopSettings) -> bool {
        if s.climate == TROPIC {
            return match self.zone(tile) {
                ZONE_DESERT => false,
                ZONE_RAINFOREST => s.extra == ETP_SPREAD_ALL || s.extra == ETP_SPREAD_RAINFOREST,
                _ => s.extra == ETP_SPREAD_ALL,
            };
        }
        s.extra == ETP_SPREAD_ALL
    }
    fn random(&self) -> u32 {
        self.s.random()
    }
    fn dirty(&self, tile: u32) {
        (self.s.mark_dirty)(tile);
    }
    fn set_ground(&self, tile: u32, ground: u8, density: u32) {
        (self.t.set_ground_density)(tile, ground, density);
    }

    /// `PlantTreesOnTile` from a record read since the last effect on this tile.
    /// Returns the ground written by `MakeTree`.
    fn plant(&self, tile: u32, p: PlantObservation, tree: u8, count: u32, growth: u8) -> u8 {
        let mut density = 3;
        let ground = if p.tile_type == MP_WATER {
            // SAFETY: Immutable tables and source-valid map dimensions; no owner borrow.
            unsafe {
                crate::water::openttd_rust_clear_neighbour_flooding(
                    tile,
                    (self.s.map_max_x)().wrapping_add(1),
                    (self.s.map_max_y)().wrapping_add(1),
                    (self.s.water_services)(),
                    self.s,
                );
            }
            GROUND_SHORE
        } else {
            let ground = if p.snow {
                if p.ground == CLEAR_ROUGH {
                    GROUND_ROUGH_SNOW
                } else {
                    GROUND_SNOW_DESERT
                }
            } else {
                match p.ground {
                    CLEAR_GRASS => GROUND_GRASS,
                    CLEAR_ROUGH => GROUND_ROUGH,
                    _ => GROUND_SNOW_DESERT,
                }
            };
            if p.ground != CLEAR_ROUGH {
                density = u32::from(p.density);
            }
            ground
        };
        (self.t.make_tree)(tile, tree, count, growth, ground, density);
        ground
    }

    /// `PlaceTree`; `GetTreeGround` after `MakeTree` is the ground just written.
    fn place(&self, tile: u32, p: PlantObservation, r: u32, keep_density: bool, climate: u8) {
        let tree = random_tree_type(climate, p.zone, r >> 24);
        if tree == TREE_INVALID {
            return;
        }
        let ground = self.plant(tile, p, tree, (r >> 22) & 3, ((r >> 16) & 7).min(6) as u8);
        self.dirty(tile);
        if keep_density {
            return;
        }
        if ground != GROUND_SNOW_DESERT && ground != GROUND_ROUGH_SNOW && ground != GROUND_SHORE {
            (self.t.set_ground_density)(tile, ((r >> 28) & 1) as u8, 3);
        }
    }

    /// `CreateRandomStarShapedPolygon(16, grove)` with native `sinf`/`cosf`.
    fn grove_shape(&self) -> [Point; 16] {
        let divisor = (f64::from(i32::MAX) / std::f64::consts::PI * 2.0) as f32;
        let mut phases = [0.0_f32; 4];
        for phase in &mut phases {
            *phase = self.random() as f32 / divisor;
        }
        let mut shape = [Point::default(); 16];
        let step = (std::f64::consts::PI * 2.0 / 16.0) as f32;
        let mut theta = 0.0_f32;
        for vertex in &mut shape {
            let mut deviation = 0.0_f32;
            for (i, phase) in phases.iter().enumerate() {
                let frequency = (i + 1) as f32;
                let amplitude = (8 >> i) as f32;
                deviation += (self.t.sin)((theta + phase) * frequency) * amplitude;
            }
            let radius = 16.0_f32 / 2.0 + deviation / 2.0;
            vertex.x = ((self.t.cos)(theta) * radius) as i32;
            vertex.y = ((self.t.sin)(theta) * radius) as i32;
            theta += step;
        }
        shape
    }

    /// `PlaceTreeGroups`; returns true on world-generation abort.
    fn place_groups(&self, g: &GenerateSettings, groups: u32) -> bool {
        let mut groups = groups;
        loop {
            let center = self.random() & g.size.wrapping_sub(1);
            let shape = self.grove_shape();
            for _ in 0..DEFAULT_TREE_STEPS {
                if (self.t.progress)() {
                    return true;
                }
                let r = self.random();
                let x = (r & 31) as i32 - 16;
                let y = ((r >> 8) & 31) as i32 - 16;
                let tile = tile_add_wrap(g, center, x, y);
                if tile == INVALID_TILE {
                    continue;
                }
                let p = self.plant_observation(tile);
                if !p.suitable(true) || !in_grove(x, y, &shape) {
                    continue;
                }
                self.place(tile, p, r, false, g.climate);
            }
            groups = groups.wrapping_sub(1);
            if groups == 0 {
                return false;
            }
        }
    }

    /// `PlaceTreeAtSameHeight`.
    fn place_at_same_height(&self, g: &GenerateSettings, tile: u32, height: i32) {
        for _ in 0..DEFAULT_TREE_STEPS {
            let r = self.random();
            let x = (r & 31) as i32 - 16;
            let y = ((r >> 8) & 31) as i32 - 16;
            let current = tile_add_wrap(g, tile, x, y);
            if current == INVALID_TILE || x.abs() + y.abs() > 16 {
                continue;
            }
            let p = self.plant_observation(current);
            if !p.suitable(true) {
                continue;
            }
            let z = (self.s.tile_z)(current);
            if (if z < height {
                height.wrapping_sub(z)
            } else {
                z.wrapping_sub(height)
            }) > 2
            {
                continue;
            }
            self.place(current, p, r, false, g.climate);
            break;
        }
    }

    /// One `PlaceTreesRandomly` attempt; returns true on world-generation abort.
    fn scatter_one(&self, g: &GenerateSettings) -> bool {
        let r = self.random();
        let tile = r & g.size.wrapping_sub(1);
        if (self.t.progress)() {
            return true;
        }
        let p = self.plant_observation(tile);
        if !p.suitable(true) {
            return false;
        }
        self.place(tile, p, r, false, g.climate);
        if g.placer != TP_IMPROVED {
            return false;
        }
        let ht = (self.s.tile_z)(tile);
        let mut j = (self.s.tile_z)(tile).wrapping_mul(2);
        if g.climate == ARCTIC && ht > i32::from((self.t.snow_line)()) {
            j = j.wrapping_mul(3);
        }
        if g.height_limit > MAP_HEIGHT_LIMIT_ORIGINAL {
            j = j.wrapping_mul(i32::from(MAP_HEIGHT_LIMIT_ORIGINAL)) / i32::from(g.height_limit);
        }
        while j != 0 {
            j = j.wrapping_sub(1);
            self.place_at_same_height(g, tile, ht);
        }
        false
    }

    /// One rainforest `PlaceTreesRandomly` attempt; returns true on abort.
    fn scatter_rainforest_one(&self, g: &GenerateSettings) -> bool {
        let r = self.random();
        let tile = r & g.size.wrapping_sub(1);
        if (self.t.progress)() {
            return true;
        }
        let p = self.plant_observation(tile);
        if p.zone == ZONE_RAINFOREST && p.suitable(false) {
            self.place(tile, p, r, false, g.climate);
        }
        false
    }

    /// `PlaceTreesRandomly`; returns true on world-generation abort.
    fn place_randomly(&self, g: &GenerateSettings) -> bool {
        let mut i = scale_by_size(g.size, DEFAULT_TREE_STEPS) as i32;
        if g.editor {
            i /= EDITOR_TREE_DIV;
        }
        loop {
            if self.scatter_one(g) {
                return true;
            }
            i = i.wrapping_sub(1);
            if i == 0 {
                break;
            }
        }
        if g.climate == TROPIC {
            let mut i = scale_by_size(g.size, DEFAULT_RAINFOREST_TREE_STEPS) as i32;
            if g.editor {
                i /= EDITOR_TREE_DIV;
            }
            loop {
                if self.scatter_rainforest_one(g) {
                    return true;
                }
                i = i.wrapping_sub(1);
                if i == 0 {
                    break;
                }
            }
        }
        false
    }

    /// `GenerateTrees`; returns true on world-generation abort.
    fn generate(&self, g: &GenerateSettings) -> bool {
        if g.placer == TP_NONE {
            return false;
        }
        let mut i: u32 = if g.placer == TP_ORIGINAL {
            if g.climate == ARCTIC { 15 } else { 6 }
        } else if g.climate == ARCTIC {
            4
        } else {
            2
        };
        let mut total = scale_by_size(g.size, DEFAULT_TREE_STEPS);
        if g.climate == TROPIC {
            total = total.wrapping_add(scale_by_size(g.size, DEFAULT_RAINFOREST_TREE_STEPS));
        }
        total = total.wrapping_mul(i);
        let groups = if g.climate == TOYLAND {
            0
        } else {
            scale_by_size(g.size, (self.random() & 31) + 25)
        };
        total = total.wrapping_add(groups.wrapping_mul(DEFAULT_TREE_STEPS));
        if (self.t.progress_total)(total) {
            return true;
        }
        if groups != 0 && self.place_groups(g, groups) {
            return true;
        }
        while i != 0 {
            if self.place_randomly(g) {
                return true;
            }
            i -= 1;
        }
        false
    }

    /// `TileLoopTreesDesert`.
    fn loop_desert(&self, tile: u32, s: &LoopSettings) {
        match self.zone(tile) {
            ZONE_DESERT => {
                if self.ground(tile) != GROUND_SNOW_DESERT {
                    self.set_ground(tile, GROUND_SNOW_DESERT, 3);
                    self.dirty(tile);
                }
            }
            ZONE_RAINFOREST => {
                let r = self.random();
                if chance16_i(1, 200, r) && s.ambient {
                    (self.s.play_sound)(tile, RAINFOREST_SOUNDS[((r >> 16) & 3) as usize]);
                }
            }
            _ => (),
        }
    }

    /// `TileLoopTreesAlps`. Repeated source reads with no write between are read once.
    fn loop_alps(&self, tile: u32, s: &LoopSettings) {
        let k = self
            .map
            .and_then(|map| {
                crate::tile_map::get_tile_z(map, tile, s.size_x, map.count() / s.size_x)
            })
            .unwrap_or_else(|| (self.s.tile_z)(tile))
            .wrapping_sub(i32::from((self.t.snow_line)()))
            .wrapping_add(1);
        if k < 0 {
            match self.ground(tile) {
                GROUND_SNOW_DESERT => self.set_ground(tile, GROUND_GRASS, 3),
                GROUND_ROUGH_SNOW => self.set_ground(tile, GROUND_ROUGH, 3),
                _ => return,
            }
        } else {
            let density = (k as u32).min(3);
            let ground = self.ground(tile);
            if ground != GROUND_SNOW_DESERT && ground != GROUND_ROUGH_SNOW {
                let ground = if ground == GROUND_ROUGH {
                    GROUND_ROUGH_SNOW
                } else {
                    GROUND_SNOW_DESERT
                };
                self.set_ground(tile, ground, density);
            } else {
                let current = self.density(tile);
                if u32::from(current) == density {
                    if current == 3 {
                        let r = self.random();
                        if chance16_i(1, 200, r) && s.ambient {
                            let sound = if r & 0x8000_0000 != 0 {
                                ARCTIC_SNOW_2
                            } else {
                                ARCTIC_SNOW_1
                            };
                            (self.s.play_sound)(tile, sound);
                        }
                    }
                    return;
                }
                self.set_ground(tile, ground, density);
            }
        }
        self.dirty(tile);
    }

    /// `TileLoop_Trees`. Fields are read where the source reads them, so flooding and the
    /// ambient callback are observed, and growth fields only behind the growth gate.
    fn tile_loop(&self, tile: u32, s: &LoopSettings, call: *mut crate::ground::Call) {
        if self.ground(tile) == GROUND_SHORE {
            // SAFETY: The native caller owns an independent stack result context;
            // flooding may reenter trees but no owner state is borrowed here.
            unsafe {
                if let Some(map) = self.map {
                    crate::water::water_tile_loop(
                        tile,
                        MP_TREES,
                        false,
                        s.size_x,
                        map.count() / s.size_x,
                        call,
                        (self.s.water_services)(),
                        self.s,
                        map,
                    );
                } else {
                    crate::water::tree_industry_water_loop(tile, call, self.s);
                }
            };
        } else {
            match s.climate {
                TROPIC => self.loop_desert(tile, s),
                ARCTIC => self.loop_alps(tile, s),
                _ => (),
            }
        }
        crate::newgrf_generic::ambient(s.ambient_callback, tile);

        let x = tile & s.size_x.wrapping_sub(1);
        let y = tile >> s.size_x.trailing_zeros();
        let cycle = x
            .wrapping_mul(11)
            .wrapping_add(y.wrapping_mul(9))
            .wrapping_add((s.tick_counter >> 8) as u32);

        if cycle & 7 == 7 && self.ground(tile) == GROUND_GRASS {
            let density = self.density(tile);
            if density < 3 {
                self.set_ground(tile, GROUND_GRASS, u32::from(density) + 1);
                self.dirty(tile);
            }
        }

        if s.extra == ETP_NO_GROWTH_NO_SPREAD || cycle % 16 != 15 {
            return;
        }

        let mut tile = tile;
        match self.growth(tile) {
            GROWN => {
                if s.climate == TROPIC
                    && self.species(tile) != TREE_CACTUS
                    && self.zone(tile) == ZONE_DESERT
                {
                    (self.t.add_growth)(tile, 1);
                } else {
                    match self.random() & 7 {
                        0 => (self.t.add_growth)(tile, 1),
                        1 if self.count(tile) < 4 && self.can_spread(tile, s) => {
                            (self.t.add_count)(tile, 1);
                            (self.t.set_growth)(tile, GROWING1);
                        }
                        1 | 2 => {
                            if self.can_spread(tile, s) {
                                let tree = self.species(tile);
                                let (dx, dy) = DIRECTION_OFFSETS[(self.random() % 8) as usize];
                                let diff = dy.wrapping_mul(s.size_x as i32).wrapping_add(dx);
                                tile = tile.wrapping_add(diff as u32);
                                let p = self.plant_observation(tile);
                                if !p.suitable(false) {
                                    return;
                                }
                                // Don't plant trees if the ground was freshly cleared.
                                if p.tile_type == MP_CLEAR
                                    && p.ground == CLEAR_GRASS
                                    && !p.snow
                                    && p.density != 3
                                {
                                    return;
                                }
                                self.plant(tile, p, tree, 0, GROWING1);
                            }
                        }
                        _ => return,
                    }
                }
            }
            DEAD => {
                if !self.can_spread(tile, s) {
                    (self.t.set_growth)(tile, GROWING1);
                } else if self.count(tile) > 1 {
                    (self.t.add_count)(tile, -1);
                    (self.t.set_growth)(tile, GROWN);
                } else {
                    match self.ground(tile) {
                        GROUND_SHORE => (self.s.make_shore)(tile),
                        GROUND_GRASS => {
                            let density = u32::from(self.density(tile));
                            (self.t.make_clear)(tile, CLEAR_GRASS, density);
                        }
                        GROUND_ROUGH => (self.t.make_clear)(tile, CLEAR_ROUGH, 3),
                        GROUND_ROUGH_SNOW => {
                            let density = u32::from(self.density(tile));
                            (self.t.make_clear)(tile, CLEAR_ROUGH, 3);
                            (self.s.make_snow)(tile, density);
                        }
                        _ if s.climate == TROPIC => {
                            let density = u32::from(self.density(tile));
                            (self.t.make_clear)(tile, CLEAR_DESERT, density);
                        }
                        _ => {
                            let density = u32::from(self.density(tile));
                            (self.t.make_clear)(tile, CLEAR_GRASS, 3);
                            (self.s.make_snow)(tile, density);
                        }
                    }
                }
            }
            _ => (self.t.add_growth)(tile, 1),
        }
        self.dirty(tile);
    }

    /// `PlantRandomTree`.
    fn plant_random_tree(&self, s: &TickSettings, rainforest: bool) {
        let r = self.random();
        let tile = r & s.size.wrapping_sub(1);
        let p = self.plant_observation(tile);
        if rainforest && p.zone != ZONE_RAINFOREST {
            return;
        }
        if !p.suitable(false) {
            return;
        }
        let tree = random_tree_type(s.climate, p.zone, r >> 24);
        if tree == TREE_INVALID {
            return;
        }
        self.plant(tile, p, tree, 0, GROWING1);
    }

    /// `OnTick_Trees`.
    fn tick(&self, s: &TickSettings) {
        if s.extra == ETP_NO_SPREAD || s.extra == ETP_NO_GROWTH_NO_SPREAD {
            return;
        }
        let skip = scale_by_size(s.size, 16);
        if skip < 16 && s.tick_counter & u64::from(16 / skip - 1) != 0 {
            return;
        }
        if s.climate == TROPIC {
            let mut c = scale_by_size(s.size, 1);
            while c > 0 {
                self.plant_random_tree(s, true);
                c -= 1;
            }
        }
        // SAFETY: Game thread only; no reference to this byte is formed, and no
        // service runs between the read and the write.
        let wrapped = unsafe {
            let (next, wrapped) = decrement(TREE_COUNTER, scale_by_size(s.size, 1));
            TREE_COUNTER = next;
            wrapped
        };
        if !wrapped || s.extra == ETP_SPREAD_RAINFOREST {
            return;
        }
        self.plant_random_tree(s, false);
    }

    /// `CmdPlantTree`.
    fn plant_command(
        &self,
        ctx: *mut Command,
        tile: u32,
        start: u32,
        tree: u8,
        diagonal: bool,
        s: &CommandSettings,
        flags: u16,
        mut clear: impl FnMut(u32) -> LandscapeClear,
    ) -> CommandResult {
        const ERROR: CommandResult = CommandResult {
            cost: 0,
            status: 1,
            message: 0,
        };
        let mut message = 0;
        let mut cost: i64 = 0;
        if start >= s.size {
            return ERROR;
        }
        let (base, count) = match s.climate {
            0 => (0, 12),
            ARCTIC => (12, 8),
            TROPIC => (TREE_RAINFOREST, 12),
            _ => (TREE_TOYLAND, 9),
        };
        if tree != TREE_INVALID && tree.wrapping_sub(base) >= count {
            return ERROR;
        }

        let mut limit = (self.t.command_begin)(ctx, tile, start, diagonal);
        let mut current = (self.t.command_tile)(ctx);
        while current != INVALID_TILE {
            let p = self.plant_observation(current);
            'tile: {
                match p.tile_type {
                    MP_TREES => {
                        if (self.t.tree_count)(current) == 4 {
                            message = 1;
                            break 'tile;
                        }
                        limit -= 1;
                        if limit < 1 {
                            message = 2;
                        } else {
                            if s.execute {
                                (self.t.add_count)(current, 1);
                                self.dirty(current);
                                (self.t.command_debit)(ctx);
                            }
                            cost = cost.saturating_add(s.build_price.saturating_mul(2));
                        }
                    }
                    MP_WATER | MP_CLEAR => {
                        if p.tile_type == MP_WATER && !p.coast {
                            message = 3;
                            break 'tile;
                        }
                        if p.bridge {
                            message = 4;
                            break 'tile;
                        }
                        if s.climate == TROPIC
                            && tree != TREE_INVALID
                            && ((tree == TREE_CACTUS && p.zone != ZONE_DESERT)
                                || (is_rainforest_tree(tree)
                                    && p.zone != ZONE_RAINFOREST
                                    && !s.editor)
                                || ((TREE_SUB_TROPICAL..TREE_TOYLAND).contains(&tree)
                                    && p.zone != ZONE_NORMAL))
                        {
                            message = 5;
                            break 'tile;
                        }
                        limit -= 1;
                        if limit < 1 {
                            message = 2;
                        } else {
                            if p.tile_type == MP_CLEAR
                                && (p.ground == CLEAR_FIELDS || p.ground == CLEAR_ROCKS)
                            {
                                let nested = clear(current);
                                if nested.failed {
                                    return CommandResult {
                                        cost: 0,
                                        status: 2,
                                        message: 0,
                                    };
                                }
                                cost = cost.saturating_add(nested.cost);
                            }
                            if !s.editor && s.company_valid {
                                (self.t.town_rating)(flags, current, true);
                            }
                            if s.execute {
                                let mut tree = tree;
                                let seed = if tree == TREE_INVALID {
                                    Some(self.random() >> 24)
                                } else {
                                    None
                                };
                                // The nested clear changed the tile; read it again.
                                let p = self.plant_observation(current);
                                if let Some(seed) = seed {
                                    tree = random_tree_type(s.climate, p.zone, seed);
                                    if tree == TREE_INVALID {
                                        tree = TREE_CACTUS;
                                    }
                                }
                                let growth = if s.editor { GROWN } else { GROWING1 };
                                self.plant(current, p, tree, 0, growth);
                                self.dirty(current);
                                (self.t.command_debit)(ctx);
                                if s.editor && is_rainforest_tree(tree) {
                                    (self.s.set_tropic_zone)(current, ZONE_RAINFOREST);
                                }
                            }
                            cost = cost.saturating_add(s.build_price);
                        }
                    }
                    _ => message = 4,
                }
                if limit < 0 {
                    return CommandResult {
                        cost,
                        status: 0,
                        message,
                    };
                }
            }
            current = (self.t.command_next)(ctx);
        }
        CommandResult {
            cost,
            status: 0,
            message,
        }
    }
}

/// Build the entry view of the C++ service tables.
/// # Safety
/// Both pointers address live immutable tables for the whole entry.
unsafe fn trees<'a>(services: *const TreeServices, shared: *const Services) -> Trees<'a> {
    // SAFETY: This function's contract keeps both tables live and immutable for the entry.
    unsafe {
        Trees {
            t: &*services,
            s: &*shared,
            map: None,
        }
    }
}

/// Scheduler-only entry: allocation metadata follows the scheduler's no-reset/no-yield proof.
/// Each scalar load ends before ambient/flooding reentry; observations reload afterwards.
pub(crate) unsafe fn tile_loop(
    tile: u32,
    settings: LoopSettings,
    call: *mut crate::ground::Call,
    services: *const TreeServices,
    shared: *const Services,
    map: &crate::services::TileTypeAccess,
) {
    let mut owner = unsafe { trees(services, shared) };
    owner.map = Some(map);
    owner.tile_loop(tile, &settings, call);
}

/// `OnTick_Trees`.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
pub(crate) unsafe fn tick(
    settings: TickSettings,
    services: *const TreeServices,
    shared: *const Services,
) {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    unsafe { trees(services, shared) }.tick(&settings);
}

/// `GenerateTrees`; true when a progress service reported world-generation abort.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_generate(
    settings: GenerateSettings,
    services: *const TreeServices,
    shared: *const Services,
) -> bool {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    unsafe { trees(services, shared) }.generate(&settings)
}

/// `PlaceTreesRandomly`; true when a progress service reported world-generation abort.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_place_randomly(
    settings: GenerateSettings,
    services: *const TreeServices,
    shared: *const Services,
) -> bool {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    unsafe { trees(services, shared) }.place_randomly(&settings)
}

/// `PlaceTree` on a tile suitable for trees.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_place(
    tile: u32,
    r: u32,
    keep_density: bool,
    climate: u8,
    services: *const TreeServices,
    shared: *const Services,
) {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    let trees = unsafe { trees(services, shared) };
    let p = trees.plant_observation(tile);
    trees.place(tile, p, r, keep_density, climate);
}

/// `PlantTreesOnTile` on a tile suitable for trees.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_plant(
    tile: u32,
    tree: u8,
    count: u32,
    growth: u8,
    services: *const TreeServices,
    shared: *const Services,
) {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    let trees = unsafe { trees(services, shared) };
    let p = trees.plant_observation(tile);
    trees.plant(tile, p, tree, count, growth);
}

/// `CanPlantTreesOnTile`.
/// # Safety
/// The table is live and immutable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_can_plant(
    tile: u32,
    allow_desert: bool,
    services: *const TreeServices,
) -> bool {
    // SAFETY: The contract above keeps the table live and immutable for the call.
    (unsafe { &*services }.plant_observation)(tile).suitable(allow_desert)
}

/// `CmdPlantTree`. The context is passed back to its command services only.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
/// `ctx` and the independent nested command result live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_cmd_plant(
    ctx: *mut Command,
    tile: u32,
    start: u32,
    tree: u8,
    diagonal: bool,
    settings: CommandSettings,
    services: *const TreeServices,
    shared: *const Services,
    nested: *mut crate::services::CommandCost,
    commands: &crate::services::CommandServices,
    flags: u16,
) -> CommandResult {
    // SAFETY: The contract above supplies the tables. The existing native nested result slot
    // keeps the complete error returned by a failed clear for status 2.
    unsafe { trees(services, shared) }.plant_command(
        ctx,
        tile,
        start,
        tree,
        diagonal,
        &settings,
        flags,
        |current| {
            let table = unsafe { &*(commands.landscape_commands)() };
            let costs = unsafe { &*table.costs };
            crate::landscape_cmd::clear_command(commands, nested, current, flags);
            LandscapeClear {
                cost: (costs.money)(nested),
                failed: !(costs.success)(nested),
            }
        },
    )
}

/// `ClearTile_Trees`; returns the saturated cost.
/// # Safety
/// As for `openttd_rust_trees_cmd_plant`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_clear_tile(
    tile: u32,
    flags: u16,
    price: i64,
    company_valid: bool,
    services: *const TreeServices,
    shared: *const Services,
) -> i64 {
    // SAFETY: the native caller supplies immutable canonical tables.
    unsafe { clear_tile_price(&*services, &*shared, tile, flags, company_valid, price) }
}
/// The existing tree clear callee, also used by direct landscape dispatch.
pub(crate) fn clear_tile(t: &TreeServices, s: &Services, tile: u32, flags: u16) -> i64 {
    clear_tile_price(
        t,
        s,
        tile,
        flags,
        (s.company_valid)((s.current_company)()),
        crate::company::price(crate::economy_type::PR_CLEAR_TREES),
    )
}
fn clear_tile_price(
    t: &TreeServices,
    s: &Services,
    tile: u32,
    flags: u16,
    company_valid: bool,
    price: i64,
) -> i64 {
    if company_valid {
        (t.town_rating)(flags, tile, false);
    }
    let tree = (t.clear_observation)(tile);
    let mut count = tree.count;
    if is_rainforest_tree(tree.species) {
        count = count.wrapping_mul(4);
    }
    if flags & crate::command_type::DoCommandFlag::EXECUTE != 0 {
        crate::ground::clear_square(tile, s);
    }
    price.saturating_mul(i64::from(count))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTreeServices" ["probe"] => crate::trees::TreeServices {
        plant_observation,
        tree_observation,
        tree_count,
        clear_observation,
        snow_line,
        sin,
        cos,
        make_tree,
        set_ground_density,
        add_count,
        add_growth,
        set_growth,
        make_clear,
        progress,
        progress_total,
        town_rating,
        command_begin,
        command_tile,
        command_next,
        command_debit,
    };
    "OpenTTDTreePlantObservation" ["probe"] => PlantObservation {
        tile_type,
        bridge,
        ground,
        density,
        snow,
        coast,
        zone,
    };
    "OpenTTDTreeObservation" ["probe"] => TreeObservation {
        ground,
        density,
        species,
        count,
        growth,
        zone,
    };
    "OpenTTDTreeClearObservation" ["probe"] => ClearObservation {
        count,
        species,
    };
    "OpenTTDTreeLoopSettings" ["probe"] => LoopSettings {
        tick_counter,
        size_x,
        climate,
        extra,
        ambient,
        ambient_callback,
    };
    "OpenTTDTreeTickSettings" ["probe"] => TickSettings {
        tick_counter,
        size,
        climate,
        extra,
    };
    "OpenTTDTreeGenerateSettings" ["probe"] => GenerateSettings {
        size,
        size_x,
        size_y,
        climate,
        placer,
        height_limit,
        editor,
        freeform_edges,
    };
    "OpenTTDTreeCommandSettings" ["probe"] => CommandSettings {
        build_price,
        size,
        climate,
        editor,
        execute,
        company_valid,
    };
    "OpenTTDTreeCommandResult" ["probe"] => CommandResult {
        cost,
        status,
        message,
    };

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::ffi::c_void;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Draw,
        Progress,
        Total(u32),
        TileZ,
        Snow,
        Plant,
        Tree,
        Count,
        ClearTree,
        Write(&'static str),
        Dirty,
        Zone(u8),
        Water,
        Sound(u16),
        Rating(bool),
        Begin(u32, u32, bool),
        Next,
        Debit,
        Clear,
    }

    #[derive(Default)]
    struct World {
        plant: PlantObservation,
        tree: TreeObservation,
        log: Vec<Event>,
        random: u32,
        abort_at: Option<usize>,
        progress: usize,
        limit: i32,
        length: u32,
        iterator: u32,
        clear_failed: bool,
    }
    thread_local! {
        static WORLD: RefCell<World> = RefCell::new(World::default());
    }
    fn log(event: Event) {
        WORLD.with_borrow_mut(|w| w.log.push(event));
    }
    fn reset(world: World) {
        WORLD.with_borrow_mut(|w| *w = world);
    }
    fn events() -> Vec<Event> {
        WORLD.with_borrow(|w| w.log.clone())
    }

    extern "C" fn random(_: *mut c_void) -> u32 {
        log(Event::Draw);
        WORLD.with_borrow(|w| w.random)
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    extern "C" fn tile_z(_: u32) -> i32 {
        log(Event::TileZ);
        8
    }
    extern "C" fn dirty(_: u32) {
        log(Event::Dirty);
    }
    extern "C" fn set_zone(_: u32, zone: u8) {
        log(Event::Zone(zone));
        WORLD.with_borrow_mut(|w| w.plant.zone = zone);
    }
    fn shared() -> Services {
        Services {
            tile_z,
            mark_dirty: dirty,
            set_tropic_zone: set_zone,
            make_shore,
            make_snow,
            water_services,
            tile_type: water_type,
            tree_ground: water,
            play_sound: sound,
            clear_square_services: crate::services::ClearSquareServices {
                docking: clear_flag,
                generating_world: clear_generation,
            },
            ..crate::services::fixture(std::ptr::null_mut(), random, industry)
        }
    }

    extern "C" fn plant_observation(_: u32) -> PlantObservation {
        log(Event::Plant);
        WORLD.with_borrow(|w| w.plant)
    }
    extern "C" fn tree_observation(_: u32) -> TreeObservation {
        log(Event::Tree);
        WORLD.with_borrow(|w| w.tree)
    }
    extern "C" fn tree_count(_: u32) -> u32 {
        log(Event::Count);
        WORLD.with_borrow(|w| u32::from(w.tree.count))
    }
    extern "C" fn clear_observation(_: u32) -> ClearObservation {
        log(Event::ClearTree);
        WORLD.with_borrow(|w| ClearObservation {
            count: u32::from(w.tree.count),
            species: w.tree.species,
        })
    }
    extern "C" fn snow_line() -> u8 {
        log(Event::Snow);
        4
    }
    extern "C" fn trig(value: f32) -> f32 {
        value
    }
    extern "C" fn make_tree(_: u32, tree: u8, count: u32, growth: u8, ground: u8, density: u32) {
        log(Event::Write("make_tree"));
        WORLD.with_borrow_mut(|w| {
            w.tree = TreeObservation {
                ground,
                density: density as u8,
                species: tree,
                count: count as u8 + 1,
                growth,
                zone: w.plant.zone,
            };
        });
    }
    extern "C" fn set_ground_density(_: u32, ground: u8, density: u32) {
        log(Event::Write("ground"));
        WORLD.with_borrow_mut(|w| {
            w.tree.ground = ground;
            w.tree.density = density as u8;
        });
    }
    extern "C" fn add_count(_: u32, _: i32) {
        log(Event::Write("count"));
    }
    extern "C" fn add_growth(_: u32, value: i32) {
        log(Event::Write("add_growth"));
        WORLD.with_borrow_mut(|w| w.tree.growth = (i32::from(w.tree.growth) + value) as u8);
    }
    extern "C" fn set_growth(_: u32, growth: u8) {
        log(Event::Write("set_growth"));
        WORLD.with_borrow_mut(|w| w.tree.growth = growth);
    }
    extern "C" fn make_clear(_: u32, _: u8, _: u32) {
        log(Event::Write("clear"));
    }
    extern "C" fn make_shore(_: u32) {
        log(Event::Write("shore"));
    }
    extern "C" fn make_snow(_: u32, _: u32) {
        log(Event::Write("snow"));
    }
    extern "C" fn water(_: u32) -> crate::services::TreeGround {
        log(Event::Water);
        crate::services::TreeGround(0)
    }
    extern "C" fn water_type(_: u32) -> u8 {
        MP_TREES
    }
    extern "C" fn water_services() -> *const crate::water::WaterServices {
        thread_local! { static WATER: crate::water::WaterServices = {
            let mut water = crate::water::fixture();
            water.clear = &raw const CLEAR;
            water
        }; }
        WATER.with(std::ptr::from_ref)
    }
    extern "C" fn sound(_: u32, id: u16) {
        log(Event::Sound(id));
    }
    extern "C" fn progress() -> bool {
        log(Event::Progress);
        WORLD.with_borrow_mut(|w| {
            w.progress += 1;
            w.abort_at == Some(w.progress)
        })
    }
    extern "C" fn progress_total(total: u32) -> bool {
        log(Event::Total(total));
        WORLD.with_borrow_mut(|w| {
            w.progress += 1;
            w.abort_at == Some(w.progress)
        })
    }
    extern "C" fn clear_flag(_: u32) -> bool {
        false
    }
    extern "C" fn clear_generation() -> bool {
        false
    }
    extern "C" fn unused_clear1<A, R>(_: A) -> R {
        panic!("unused clear leaf");
    }
    extern "C" fn unused_clear2<A, B, R>(_: A, _: B) -> R {
        panic!("unused clear leaf");
    }
    extern "C" fn unused_clear3<A, B, C, R>(_: A, _: B, _: C) -> R {
        panic!("unused clear leaf");
    }
    static CLEAR: crate::ground::ClearServices = crate::ground::ClearServices {
        field_tile: unused_clear1,
        density: unused_clear1,
        counter: unused_clear1,
        snow: unused_clear1,
        surface: unused_clear1,
        set_ground: unused_clear3,
        set_density: unused_clear2,
        set_counter: unused_clear2,
        clear_snow: unused_clear1,
        make_clear: clear_square,
        fences: unused_clear1,
        field: unused_clear1,
        set_field: unused_clear2,
        water_class: unused_clear1,
    };
    extern "C" fn clear_square(_: u32, _: crate::services::ClearGroundType, _: u32) {
        log(Event::Write("clear_square"));
    }
    extern "C" fn rating(_: u16, _: u32, up: bool) {
        log(Event::Rating(up));
    }
    extern "C" fn begin(_: *mut Command, end: u32, start: u32, diagonal: bool) -> i32 {
        log(Event::Begin(end, start, diagonal));
        WORLD.with_borrow(|w| w.limit)
    }
    extern "C" fn command_tile(_: *mut Command) -> u32 {
        WORLD.with_borrow(|w| {
            if w.iterator < w.length {
                1
            } else {
                INVALID_TILE
            }
        })
    }
    extern "C" fn command_next(ctx: *mut Command) -> u32 {
        log(Event::Next);
        WORLD.with_borrow_mut(|w| w.iterator += 1);
        command_tile(ctx)
    }
    extern "C" fn debit(_: *mut Command) {
        log(Event::Debit);
    }
    fn landscape_clear(_: *mut Command, _: u32) -> LandscapeClear {
        log(Event::Clear);
        WORLD.with_borrow_mut(|w| {
            w.plant.ground = CLEAR_GRASS;
            LandscapeClear {
                cost: 7,
                failed: w.clear_failed,
            }
        })
    }
    const SERVICES: TreeServices = TreeServices {
        plant_observation,
        tree_observation,
        tree_count,
        clear_observation,
        snow_line,
        sin: trig,
        cos: trig,
        make_tree,
        set_ground_density,
        add_count,
        add_growth,
        set_growth,
        make_clear,
        progress,
        progress_total,
        town_rating: rating,
        command_begin: begin,
        command_tile,
        command_next,
        command_debit: debit,
    };
    fn with<R>(f: impl FnOnce(&Trees) -> R) -> R {
        let shared = shared();
        f(&Trees {
            t: &SERVICES,
            s: &shared,
            map: None,
        })
    }
    const CLEAR_TILE: PlantObservation = PlantObservation {
        tile_type: MP_CLEAR,
        bridge: false,
        ground: CLEAR_GRASS,
        density: 3,
        snow: false,
        coast: false,
        zone: ZONE_NORMAL,
    };
    const GENERATE: GenerateSettings = GenerateSettings {
        size: 4096,
        size_x: 64,
        size_y: 64,
        climate: 0,
        placer: TP_ORIGINAL,
        height_limit: 15,
        editor: false,
        freeform_edges: true,
    };
    const COMMAND: CommandSettings = CommandSettings {
        build_price: 15,
        size: 4096,
        climate: 0,
        editor: false,
        execute: true,
        company_valid: true,
    };

    #[test]
    fn counter_full_byte_and_scale_domain() {
        for scale in 1..=256 {
            for old in 0..=u8::MAX {
                let (next, wrapped) = decrement(old, scale);
                assert_eq!(next, (u32::from(old).wrapping_sub(scale) & 255) as u8);
                assert_eq!(wrapped, u32::from(old) < scale);
            }
        }
    }

    #[test]
    fn scale_and_wrap_follow_map_geometry() {
        assert_eq!(scale_by_size(4096, 1000), 63);
        assert_eq!(scale_by_size(65536, 1), 1);
        assert_eq!(scale_by_size(4096 * 4096, 1), 256);
        assert_eq!(tile_add_wrap(&GENERATE, 65, -1, 0), INVALID_TILE);
        assert_eq!(tile_add_wrap(&GENERATE, 65, 1, 1), 130);
        assert_eq!(tile_add_wrap(&GENERATE, 65, 62, 0), INVALID_TILE);
        let edges = GenerateSettings {
            freeform_edges: false,
            ..GENERATE
        };
        assert_eq!(tile_add_wrap(&edges, 65, -1, 0), 64);
    }

    // Headless semantic scenarios cannot click Abort during map generation. The
    // aborting progress call is the last service: no draw, read or write follows it.
    #[test]
    fn generation_abort_returns_at_the_progress_call() {
        reset(World {
            abort_at: Some(1),
            ..World::default()
        });
        assert!(with(|t| t.place_randomly(&GENERATE)));
        assert_eq!(events(), [Event::Draw, Event::Progress]);

        reset(World {
            abort_at: Some(1),
            ..World::default()
        });
        assert!(with(|t| t.generate(&GENERATE)));
        assert_eq!(events(), [Event::Draw, Event::Total(6 * 63 + 2 * 1000)]);

        reset(World {
            abort_at: Some(2),
            ..World::default()
        });
        assert!(with(|t| t.generate(&GENERATE)));
        let mut expected = vec![Event::Draw, Event::Total(6 * 63 + 2 * 1000)];
        expected.extend([Event::Draw; 5]); // Centre and four phases.
        expected.push(Event::Progress);
        assert_eq!(events(), expected);

        reset(World {
            abort_at: Some(3),
            plant: PlantObservation {
                tile_type: MP_TREES,
                ..CLEAR_TILE
            },
            ..World::default()
        });
        assert!(with(|t| t.place_randomly(&GENERATE)));
        let attempt = [Event::Draw, Event::Progress, Event::Plant];
        let mut expected = [attempt, attempt].concat();
        expected.extend([Event::Draw, Event::Progress]);
        assert_eq!(events(), expected);
    }

    #[test]
    fn quiet_temperate_tile_reads_once_and_never_reads_height() {
        reset(World {
            tree: TreeObservation {
                growth: GROWN,
                count: 1,
                density: 3,
                ..TreeObservation::default()
            },
            ..World::default()
        });
        let s = LoopSettings {
            tick_counter: 0,
            size_x: 64,
            climate: 0,
            extra: ETP_SPREAD_ALL,
            ambient: true,
            ambient_callback: false,
        };
        with(|t| t.tile_loop(0, &s, std::ptr::null_mut()));
        assert_eq!(events(), [Event::Tree]);
    }

    #[test]
    fn water_rereads_the_tile_without_normalizing_bitpattern_seven() {
        let s = LoopSettings {
            tick_counter: 2304, // Tile 2: cycle 31.
            size_x: 64,
            climate: 0,
            extra: ETP_NO_SPREAD,
            ambient: false,
            ambient_callback: false,
        };
        let tree = TreeObservation {
            ground: GROUND_GRASS,
            density: 3,
            count: 1,
            growth: 7,
            ..TreeObservation::default()
        };
        reset(World {
            tree,
            ..World::default()
        });
        with(|t| t.tile_loop(2, &s, std::ptr::null_mut()));
        assert_eq!(
            events(),
            [
                Event::Tree,
                Event::Tree,
                Event::Tree,
                Event::Tree,
                Event::Write("add_growth"),
                Event::Dirty
            ]
        );
        reset(World {
            tree: TreeObservation {
                ground: GROUND_SHORE,
                ..tree
            },
            ..World::default()
        });
        with(|t| t.tile_loop(2, &s, std::ptr::null_mut()));
        assert_eq!(
            events(),
            [
                Event::Tree,
                Event::Water,
                Event::Tree,
                Event::Tree,
                Event::Write("add_growth"),
                Event::Dirty
            ]
        );
    }

    #[test]
    fn arctic_loop_reads_height_and_snow_line_once() {
        reset(World {
            tree: TreeObservation {
                ground: GROUND_SNOW_DESERT,
                density: 3,
                growth: GROWING1,
                count: 1,
                ..TreeObservation::default()
            },
            random: 0x8000_0000,
            ..World::default()
        });
        let s = LoopSettings {
            tick_counter: 0,
            size_x: 64,
            climate: ARCTIC,
            extra: ETP_SPREAD_ALL,
            ambient: true,
            ambient_callback: false,
        };
        with(|t| t.tile_loop(0, &s, std::ptr::null_mut()));
        assert_eq!(
            events(),
            [
                Event::Tree,
                Event::TileZ,
                Event::Snow,
                Event::Tree,
                Event::Tree,
                Event::Draw,
                Event::Sound(ARCTIC_SNOW_2)
            ]
        );
    }

    // Script APIs cannot request explicit tree types, diagonal rectangles, or editor
    // planting. These tests cover those source branches and full Money bounds;
    // end-to-end command evidence remains the unchanged reference harness.
    #[test]
    fn explicit_editor_planting_and_diagonal_iterator_request() {
        reset(World {
            plant: PlantObservation {
                zone: ZONE_NORMAL,
                ..CLEAR_TILE
            },
            limit: i32::MAX,
            length: 1,
            ..World::default()
        });
        let s = CommandSettings {
            climate: TROPIC,
            editor: true,
            ..COMMAND
        };
        let result = with(|t| {
            t.plant_command(std::ptr::null_mut(), 2, 1, 20, true, &s, 0, |tile| {
                landscape_clear(std::ptr::null_mut(), tile)
            })
        });
        assert_eq!((result.status, result.message, result.cost), (0, 0, 15));
        assert_eq!(
            events(),
            [
                Event::Begin(2, 1, true),
                Event::Plant,
                Event::Plant,
                Event::Write("make_tree"),
                Event::Dirty,
                Event::Debit,
                Event::Zone(ZONE_RAINFOREST),
                Event::Next
            ]
        );
        assert_eq!(WORLD.with_borrow(|w| w.tree.growth), GROWN);
    }

    #[test]
    fn explicit_tree_type_checks_and_wrong_tropical_zone() {
        reset(World {
            plant: CLEAR_TILE,
            limit: i32::MAX,
            length: 1,
            ..World::default()
        });
        let result = with(|t| {
            t.plant_command(std::ptr::null_mut(), 2, 1, 20, false, &COMMAND, 0, |tile| {
                landscape_clear(std::ptr::null_mut(), tile)
            })
        });
        assert_eq!(result.status, 1); // Rainforest is outside the temperate range.
        let result = with(|t| {
            t.plant_command(
                std::ptr::null_mut(),
                2,
                4096,
                255,
                false,
                &COMMAND,
                0,
                |tile| landscape_clear(std::ptr::null_mut(), tile),
            )
        });
        assert_eq!(result.status, 1); // Start tile outside the map.
        assert_eq!(events(), []);
        let s = CommandSettings {
            climate: TROPIC,
            ..COMMAND
        };
        let result = with(|t| {
            t.plant_command(std::ptr::null_mut(), 2, 1, 20, false, &s, 0, |tile| {
                landscape_clear(std::ptr::null_mut(), tile)
            })
        });
        assert_eq!((result.status, result.cost, result.message), (0, 0, 5));
    }

    #[test]
    fn exhausted_limit_checks_zero_and_negative_separately() {
        reset(World {
            plant: CLEAR_TILE,
            limit: 1,
            length: 5,
            ..World::default()
        });
        let result = with(|t| {
            t.plant_command(
                std::ptr::null_mut(),
                2,
                1,
                255,
                false,
                &COMMAND,
                0,
                |tile| landscape_clear(std::ptr::null_mut(), tile),
            )
        });
        assert_eq!((result.status, result.cost, result.message), (0, 0, 2));
        assert_eq!(
            events(),
            [
                Event::Begin(2, 1, false),
                Event::Plant,
                Event::Next,
                Event::Plant
            ]
        );
    }

    #[test]
    fn nested_clear_failure_and_fields_replant_from_a_fresh_record() {
        reset(World {
            plant: PlantObservation {
                ground: CLEAR_FIELDS,
                ..CLEAR_TILE
            },
            limit: i32::MAX,
            length: 1,
            clear_failed: true,
            ..World::default()
        });
        let result = with(|t| {
            t.plant_command(std::ptr::null_mut(), 2, 1, 3, false, &COMMAND, 0, |tile| {
                landscape_clear(std::ptr::null_mut(), tile)
            })
        });
        assert_eq!(result.status, 2);
        reset(World {
            plant: PlantObservation {
                ground: CLEAR_FIELDS,
                ..CLEAR_TILE
            },
            limit: i32::MAX,
            length: 1,
            ..World::default()
        });
        let result = with(|t| {
            t.plant_command(
                std::ptr::null_mut(),
                2,
                1,
                255,
                false,
                &COMMAND,
                0,
                |tile| landscape_clear(std::ptr::null_mut(), tile),
            )
        });
        assert_eq!((result.status, result.cost), (0, 22));
        assert_eq!(
            events(),
            [
                Event::Begin(2, 1, false),
                Event::Plant,
                Event::Clear,
                Event::Rating(true),
                Event::Draw,
                Event::Plant,
                Event::Write("make_tree"),
                Event::Dirty,
                Event::Debit,
                Event::Next
            ]
        );
        // The fresh record saw the cleared grass ground.
        assert_eq!(WORLD.with_borrow(|w| w.tree.ground), GROUND_GRASS);
    }

    #[test]
    fn money_extremes_keep_original_saturation_order() {
        for price in [i64::MIN, i64::MAX] {
            reset(World {
                plant: PlantObservation {
                    tile_type: MP_TREES,
                    ..CLEAR_TILE
                },
                tree: TreeObservation {
                    count: 1,
                    species: 20,
                    ..TreeObservation::default()
                },
                limit: i32::MAX,
                length: 3,
                ..World::default()
            });
            let s = CommandSettings {
                build_price: price,
                execute: false,
                ..COMMAND
            };
            let result = with(|t| {
                t.plant_command(std::ptr::null_mut(), 2, 1, 255, false, &s, 0, |tile| {
                    landscape_clear(std::ptr::null_mut(), tile)
                })
            });
            assert_eq!(result.cost, price);
            assert_eq!(events().iter().filter(|e| **e == Event::Count).count(), 3);
            assert!(!events().contains(&Event::Tree));
            WORLD.with_borrow_mut(|w| w.tree.count = 4);
            // SAFETY: The test table is a live constant.
            let shared = shared();
            let cost = unsafe {
                openttd_rust_trees_clear_tile(1, 1, price, true, &SERVICES, &raw const shared)
            };
            assert_eq!(cost, price);
        }
        let log = events();
        assert_eq!(
            &log[log.len() - 4..],
            [
                Event::Rating(false),
                Event::ClearTree,
                Event::Write("clear_square"),
                Event::Dirty
            ]
        );
    }
}

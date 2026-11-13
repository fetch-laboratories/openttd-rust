/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Handling of tree tiles (`tree_cmd.cpp`). Tiles stay in the native map arrays, which these
//! functions read in place through a `World` view; the `tree_map.h` writers stay native leaves.
//! Flooding, the `NewGRF` ambient callback and the nested landscape clear may reenter tree
//! entries but cannot reset or load the map, so a view lives for its entry. A world-generation
//! progress update yields the game-state mutex: generation builds a new view after each one. A
//! generation abort is the one C++ exception: its progress service reports it and the entry
//! returns. Settings are read through the catalog's process-wide addresses, one field-sized
//! load at the original's observation point.
#![allow(
    unsafe_code,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    reason = "the original's coordinate and random names; CmdPlantTree and TileLoop_Trees are one function each"
)]

use crate::bitmath_func::gb;
use crate::clear_map::{ClearGround, get_clear_density, get_clear_ground, is_clear_ground};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::economy_type::{ExpensesType, PR_BUILD_TREES, PR_CLEAR_TREES};
use crate::geometry_type::Point;
use crate::landscape_type::LandscapeType;
use crate::map_func::{Map, TileTypeAccess, World};
use crate::math::{delta, is_inside_bs, is_inside_mm};
use crate::openttd::GameMode;
use crate::random_func::chance16_i;
use crate::services::{CommandCost, CommandServices, Cost, Services, TownHandle};
use crate::settings_type::TreePlacer;
use crate::sound_type::SoundFx;
use crate::strings_type::StringID;
use crate::tile_type::{TileIndex, TileType, TropicZone};
use crate::tree_land::{TREE_BASE_BY_LANDSCAPE, TREE_COUNT_BY_LANDSCAPE};
use crate::tree_map::{
    TREE_COUNT_RAINFOREST, TREE_COUNT_SUB_ARCTIC, TREE_COUNT_SUB_TROPICAL, TREE_COUNT_TEMPERATE,
    TREE_COUNT_TOYLAND, TreeGround, TreeGrowthStage, TreeType, get_tree_count, get_tree_density,
    get_tree_ground, get_tree_growth, get_tree_type,
};

/// Where to place trees while in-game?
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraTreePlacement(pub u8);
impl ExtraTreePlacement {
    /// Grow trees on tiles that have them but don't spread to new ones
    pub const ETP_NO_SPREAD: Self = Self(0);
    /// Grow trees on tiles that have them, only spread to new ones in rainforests
    pub const ETP_SPREAD_RAINFOREST: Self = Self(1);
    /// Grow trees and spread them without restrictions
    pub const ETP_SPREAD_ALL: Self = Self(2);
    /// Don't grow trees and don't spread them at all
    pub const ETP_NO_GROWTH_NO_SPREAD: Self = Self(3);
}

/// Determines when to consider building more trees.
// Process-wide, initialized without runtime construction. The game-state mutex serializes
// access; save/load dereferences its address only when no Rust call is active, and no Rust
// reference to it is formed.
#[allow(non_upper_case_globals, reason = "the original's name")]
static mut _trees_tick_ctr: u8 = 0;

/// Default number of attempts for placing trees.
const DEFAULT_TREE_STEPS: u16 = 1000;
/// Default number of attempts for placing extra trees at rainforest in tropic.
const DEFAULT_RAINFOREST_TREE_STEPS: u16 = 15000;
/// Game editor tree generation divisor factor.
const EDITOR_TREE_DIV: u16 = 5;

/// Stable address of `_trees_tick_ctr` for the unchanged DATE and legacy TTD/TTO save
/// descriptors. `LoadCheck` does not access this byte.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tree_counter() -> *mut u8 {
    &raw mut _trees_tick_ctr
}

/// `TileLoop_Trees` settings of one tile-loop batch. `ambient` is the client sound setting;
/// `ambient_callback` is the `NewGRF` feature gate of `AmbientSoundEffect`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoopSettings {
    pub tick_counter: u64,
    pub size_x: u32,
    pub climate: LandscapeType,
    pub extra: ExtraTreePlacement,
    pub ambient: bool,
    pub ambient_callback: bool,
}

/// The error messages of `CmdPlantTree`, filled once by C++.
#[repr(C)]
pub struct Strings {
    /// `STR_ERROR_TREE_ALREADY_HERE`.
    pub already_here: u32,
    /// `STR_ERROR_CAN_T_BUILD_ON_WATER`.
    pub on_water: u32,
    /// `STR_ERROR_TREE_PLANT_LIMIT_REACHED`.
    pub plant_limit: u32,
    /// `STR_ERROR_SITE_UNSUITABLE`.
    pub site_unsuitable: u32,
    /// `STR_ERROR_TREE_WRONG_TERRAIN_FOR_TREE_TYPE`.
    pub wrong_terrain: u32,
}

/// Tree services: the `tree_map.h` and `clear_map.h` writers of the same names, the native
/// `sinf` and `cosf` that keep grove shapes identical across platforms, and the generation
/// progress updates. `progress` and `progress_total` return true when world generation was
/// aborted; C++ then holds the exception and rethrows it after the entry returns.
#[repr(C)]
pub struct TreeServices {
    pub add_count: extern "C" fn(u32, i32),
    pub add_growth: extern "C" fn(u32, i32),
    /// The magic bulldozer cheat and town indices of `ChangeTownRating`.
    pub authority: *const crate::town::AuthorityServices,
    pub cos: extern "C" fn(f32) -> f32,
    pub make_clear: extern "C" fn(u32, u8, u32),
    pub make_tree: extern "C" fn(u32, u8, u32, u8, u8, u32),
    pub progress: extern "C" fn() -> bool,
    pub progress_total: extern "C" fn(u32) -> bool,
    pub set_ground_density: extern "C" fn(u32, u8, u32),
    pub set_growth: extern "C" fn(u32, u8),
    pub sin: extern "C" fn(f32) -> f32,
    pub strings: Strings,
    /// Town leaves of `ChangeTownRating`.
    pub town: *const crate::town::Leaves,
}

crate::abi::shared_records! {
    fn abi_record;
    /// The parameters of `CmdPlantTree`, in the original's order; tiles are base `TileIndex`
    /// values.
    "OpenTTDPlantTreeArgs" ["probe"] => pub struct PlantTreeArgs {
        /// End tile of area-drag.
        pub tile: u32,
        /// Start tile of area-drag of tree plantation.
        pub start_tile: u32,
        /// Tree type, `TREE_INVALID` means random.
        pub tree_to_plant: u8,
        /// Whether to use the Orthogonal (false) or Diagonal (true) iterator.
        pub diagonal: bool,
    }
}

/// The immutable service tables of one entry. They are C++ statics, never world storage, so
/// holding them across reentrant services is sound.
pub(crate) struct Trees<'a> {
    t: &'a TreeServices,
    s: &'a Services,
}
impl Trees<'_> {
    /// # Safety
    /// Both pointers address live immutable tables for the whole entry.
    pub(crate) unsafe fn new(services: *const TreeServices, shared: *const Services) -> Self {
        // SAFETY: This function's contract.
        unsafe {
            Self {
                t: &*services,
                s: &*shared,
            }
        }
    }
    /// `MakeTree`: make a tree-tile.
    ///
    /// This functions change the tile to a tile with trees and all information which belongs
    /// to it. `count` is the number of trees, `density` the density (not the number of trees).
    fn make_tree(
        &self,
        t: TileIndex,
        r#type: TreeType,
        count: u32,
        growth: TreeGrowthStage,
        ground: TreeGround,
        density: u32,
    ) {
        (self.t.make_tree)(t.0, r#type.0, count, growth.0, ground.0, density);
    }
    /// `SetTreeGroundDensity`: set the density and ground type of a tile with trees.
    ///
    /// This functions saves the ground type and the density which belongs to it for a given
    /// tile.
    fn set_tree_ground_density(&self, t: TileIndex, g: TreeGround, d: u32) {
        (self.t.set_ground_density)(t.0, g.0, d);
    }
    /// `AddTreeCount`: add a amount to the tree-count value of a tile with trees.
    ///
    /// This function add a value to the tree-count value of a tile. This value may be negative
    /// to reduce the tree-counter. If the resulting value reach 0 it doesn't get converted to a
    /// "normal" tile.
    fn add_tree_count(&self, t: TileIndex, c: i32) {
        (self.t.add_count)(t.0, c);
    }
    /// `AddTreeGrowth`: add a value to the tree growth stage.
    ///
    /// This function adds a value to the tree grow stage of a tile.
    fn add_tree_growth(&self, t: TileIndex, a: i32) {
        (self.t.add_growth)(t.0, a);
    }
    /// `SetTreeGrowth`: sets the tree growth stage of a tile.
    ///
    /// This function sets the tree growth stage of a tile directly with the given value.
    fn set_tree_growth(&self, t: TileIndex, g: TreeGrowthStage) {
        (self.t.set_growth)(t.0, g.0);
    }
    /// `MakeClear`.
    fn make_clear(&self, t: TileIndex, g: ClearGround, density: u32) {
        (self.t.make_clear)(t.0, g.0, density);
    }
    /// `MarkTileDirtyByTile`.
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.s.mark_dirty)(tile.0);
    }
    /// `ChangeTownRating(t, add, max, flags)` for `_current_company`; `t` is not null.
    fn change_town_rating(&self, t: *const TownHandle, add: i32, max: i32, flags: DoCommandFlags) {
        // SAFETY: The tables are live for the entry and the town is live; the rating owner ends
        // its access before its native window invalidation.
        unsafe {
            let authority = &*self.t.authority;
            crate::town::change_town_rating(
                &*self.t.town,
                self.s,
                crate::town::RatedTown {
                    state: (self.s.town_owner)(t),
                    id: crate::town_type::TownId((authority.town_index)(t)),
                    shell: t,
                },
                crate::town::RatingChange {
                    company: CompanyID((self.s.current_company)()),
                    add,
                    maximum: max,
                    flags,
                    bulldozer: (authority.bulldozer)(),
                },
            );
        }
    }
}

/// `_settings_game.game_creation.landscape`.
fn landscape(s: &Services) -> LandscapeType {
    // SAFETY: The module's settings contract.
    LandscapeType(unsafe { s.settings().climate.read() })
}

/// `_game_mode == GM_EDITOR`.
fn editor() -> bool {
    crate::session::mode() == GameMode::GM_EDITOR
}

/// `NOT_REACHED()`, through the native fatal error.
fn not_reached() -> ! {
    crate::services_assert::not_reached()
}

/// Tests if a tile can be converted to `MP_TREES`.
/// This is true for clear ground without farms or rocks.
fn can_plant_trees_on_tile(w: &World, tile: TileIndex, allow_desert: bool) -> bool {
    match w.kind(tile) {
        TileType::MP_WATER => {
            !w.bridge_above(tile.0)
                && w.asserted(|m| crate::water_map::is_coast(m, tile.0))
                && !crate::slope_func::is_slope_with_one_corner_raised(w.tile_slope(tile))
        }
        TileType::MP_CLEAR => {
            !w.bridge_above(tile.0)
                && !w.asserted(|m| is_clear_ground(m, tile, ClearGround::CLEAR_FIELDS))
                && !w.asserted(|m| is_clear_ground(m, tile, ClearGround::CLEAR_ROCKS))
                && (allow_desert
                    || !w.asserted(|m| is_clear_ground(m, tile, ClearGround::CLEAR_DESERT)))
        }
        _ => false,
    }
}

/// Creates a tree tile.
/// Ground type and density is preserved.
/// `count` is the number of trees (minus 1).
fn plant_trees_on_tile(
    c: &Trees,
    w: &World,
    tile: TileIndex,
    treetype: TreeType,
    count: u32,
    growth: TreeGrowthStage,
) {
    w.assert_with(|| treetype != TreeType::TREE_INVALID);
    w.assert_with(|| can_plant_trees_on_tile(w, tile, true));

    let ground;
    let mut density = 3;

    match w.kind(tile) {
        TileType::MP_WATER => {
            ground = TreeGround::TREE_GROUND_SHORE;
            // SAFETY: Live immutable tables; flooding state writes cannot reset the map.
            unsafe {
                crate::water::clear_neighbour_non_flooding_states(
                    tile,
                    w.dimensions.size_x(),
                    w.size_y(),
                    (c.s.water_services)(),
                    c.s,
                );
            }
        }
        TileType::MP_CLEAR => {
            let clearground = w.asserted(|m| get_clear_ground(m, tile));
            if w.asserted(|m| crate::clear_map::is_snow_tile(m, tile)) {
                ground = if clearground == ClearGround::CLEAR_ROUGH {
                    TreeGround::TREE_GROUND_ROUGH_SNOW
                } else {
                    TreeGround::TREE_GROUND_SNOW_DESERT
                };
            } else {
                ground = match clearground {
                    ClearGround::CLEAR_GRASS => TreeGround::TREE_GROUND_GRASS,
                    ClearGround::CLEAR_ROUGH => TreeGround::TREE_GROUND_ROUGH,
                    _ => TreeGround::TREE_GROUND_SNOW_DESERT,
                };
            }
            if clearground != ClearGround::CLEAR_ROUGH {
                density = u32::from(w.asserted(|m| get_clear_density(m, tile)));
            }
        }
        _ => not_reached(),
    }

    c.make_tree(tile, treetype, count, growth, ground, density);
}

/// Get a random `TreeType` for the given tile based on a given seed.
///
/// This function returns a random `TreeType` which can be placed on the given tile.
/// The seed for randomness must be less or equal 256, use `GB` on the value of `Random()`
/// to get such a value.
#[allow(
    clippy::cast_possible_truncation,
    reason = "static_cast<TreeType> of a uint"
)]
fn get_random_tree_type(w: &World, tile: TileIndex, seed: u32) -> TreeType {
    let tree = |count: u32, base: TreeType| {
        TreeType(
            seed.wrapping_mul(count)
                .wrapping_div(256)
                .wrapping_add(u32::from(base.0)) as u8,
        )
    };
    match landscape(w.shared) {
        LandscapeType::Temperate => tree(TREE_COUNT_TEMPERATE, TreeType::TREE_TEMPERATE),
        LandscapeType::Arctic => tree(TREE_COUNT_SUB_ARCTIC, TreeType::TREE_SUB_ARCTIC),
        LandscapeType::Tropic => match w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile)) {
            TropicZone::TROPICZONE_NORMAL => {
                tree(TREE_COUNT_SUB_TROPICAL, TreeType::TREE_SUB_TROPICAL)
            }
            TropicZone::TROPICZONE_DESERT => {
                if seed > 12 {
                    TreeType::TREE_INVALID
                } else {
                    TreeType::TREE_CACTUS
                }
            }
            _ => tree(TREE_COUNT_RAINFOREST, TreeType::TREE_RAINFOREST),
        },
        _ => tree(TREE_COUNT_TOYLAND, TreeType::TREE_TOYLAND),
    }
}

/// Make a random tree tile of the given tile.
///
/// Create a new tree-tile for the given tile. `r`, a `Random()` value, is used for
/// randomness like type and number of trees. `keep_density` keeps the existing ground density
/// of the tile.
#[allow(
    clippy::cast_possible_truncation,
    reason = "GB of a uint into a uint8_t field"
)]
pub(crate) fn place_tree(c: &Trees, w: &World, tile: TileIndex, r: u32, keep_density: bool) {
    let tree = get_random_tree_type(w, tile, gb(r, 24, 8));

    if tree != TreeType::TREE_INVALID {
        plant_trees_on_tile(
            c,
            w,
            tile,
            tree,
            gb(r, 22, 2),
            TreeGrowthStage((gb(r, 16, 3) as u8).min(6)),
        );
        c.mark_tile_dirty_by_tile(tile);

        // Maybe keep the existing ground density.
        if keep_density {
            return;
        }

        // Rerandomize ground, if neither snow nor shore
        let ground = w.asserted(|m| get_tree_ground(m, tile));
        if ground != TreeGround::TREE_GROUND_SNOW_DESERT
            && ground != TreeGround::TREE_GROUND_ROUGH_SNOW
            && ground != TreeGround::TREE_GROUND_SHORE
        {
            c.set_tree_ground_density(tile, TreeGround(gb(r, 28, 1) as u8), 3);
        }
    }
}

/// One harmonic of a star-shaped polygon.
struct BlobHarmonic {
    amplitude: i32,
    phase: f32,
    frequency: i32,
}

/// Creates a star-shaped polygon originating from (0, 0) as defined by the given harmonics.
/// The shape is placed into a pre-allocated span so the caller controls allocation.
/// `radius` is the maximum radius of the polygon. May be smaller, but will not be larger.
#[allow(
    clippy::cast_possible_truncation,
    reason = "CreateStarShapedPolygon's size_t, int and float conversions"
)]
fn create_star_shaped_polygon(
    c: &Trees,
    radius: i32,
    harmonics: &[BlobHarmonic],
    shape: &mut [Point],
) {
    let mut theta = 0.0_f32;
    let step = (std::f64::consts::PI * 2.0 / f64::from(shape.len() as u32)) as f32;

    // Divide a circle into a number of equally spaced divisions.
    for vertex in shape {
        // Add up the values of each harmonic at this segment.
        let deviation = harmonics.iter().fold(0.0_f32, |d, harmonic| {
            d + (c.t.sin)((theta + harmonic.phase) * f64::from(harmonic.frequency) as f32)
                * f64::from(harmonic.amplitude) as f32
        });

        // Smooth out changes.
        let adjusted_radius = (f64::from(radius) as f32 / 2.0) + (deviation / 2.0);

        // Add to the final polygon.
        vertex.x = ((c.t.cos)(theta) * adjusted_radius) as i32;
        vertex.y = ((c.t.sin)(theta) * adjusted_radius) as i32;

        // Proceed to the next segment.
        theta += step;
    }
}

/// Creates a random star-shaped polygon originating from (0, 0).
/// The shape is placed into a pre-allocated span so the caller controls allocation.
/// `radius` is the maximum radius of the blob. May be smaller, but will not be larger.
#[allow(
    clippy::cast_possible_truncation,
    reason = "PHASE_DIVISOR's double and Random()'s uint32_t converted to float"
)]
fn create_random_star_shaped_polygon(c: &Trees, radius: i32, shape: &mut [Point]) {
    // Valid values for the phase of blob harmonics are between 0 and Tau. we can get a value in
    // the correct range from Random() by dividing the maximum possible value by the desired
    // maximum, and then dividing the random value by the result.
    const PHASE_DIVISOR: f32 = (i32::MAX as f64 / std::f64::consts::PI * 2.0) as f32;

    // These values are ones found in testing that result in suitable-looking polygons that did
    // not self-intersect and fit within a square of radius * radius dimensions.
    let harmonics = [
        BlobHarmonic {
            amplitude: radius / 2,
            phase: f64::from(c.s.random()) as f32 / PHASE_DIVISOR,
            frequency: 1,
        },
        BlobHarmonic {
            amplitude: radius / 4,
            phase: f64::from(c.s.random()) as f32 / PHASE_DIVISOR,
            frequency: 2,
        },
        BlobHarmonic {
            amplitude: radius / 8,
            phase: f64::from(c.s.random()) as f32 / PHASE_DIVISOR,
            frequency: 3,
        },
        BlobHarmonic {
            amplitude: radius / 16,
            phase: f64::from(c.s.random()) as f32 / PHASE_DIVISOR,
            frequency: 4,
        },
    ];

    create_star_shaped_polygon(c, radius, &harmonics, shape);
}

/// Returns true if the given coordinates (relative to the centre of the shape) lie within a
/// triangle.
fn is_point_in_triangle(x: i32, y: i32, v1: Point, v2: Point, v3: Point) -> bool {
    let s = ((v1.x.wrapping_sub(v3.x)).wrapping_mul(y.wrapping_sub(v3.y)))
        .wrapping_sub((v1.y.wrapping_sub(v3.y)).wrapping_mul(x.wrapping_sub(v3.x)));
    let t = ((v2.x.wrapping_sub(v1.x)).wrapping_mul(y.wrapping_sub(v1.y)))
        .wrapping_sub((v2.y.wrapping_sub(v1.y)).wrapping_mul(x.wrapping_sub(v1.x)));

    if (s < 0) != (t < 0) && s != 0 && t != 0 {
        return false;
    }

    let d = (v3.x.wrapping_sub(v2.x))
        .wrapping_mul(y.wrapping_sub(v2.y))
        .wrapping_sub((v3.y.wrapping_sub(v2.y)).wrapping_mul(x.wrapping_sub(v2.x)));
    (d < 0) == (s.wrapping_add(t) <= 0)
}

/// Returns true if the given coordinates (relative to the centre of the shape) lie within a
/// star shaped polygon.
/// Breaks the polygon into a series of triangles around the centre point (0, 0) and then tests
/// the coordinates against each triangle until a match is found (or not).
fn is_point_in_star_shaped_polygon(x: i32, y: i32, shape: &[Point]) -> bool {
    let mut it = 0;
    while it != shape.len() {
        // The iterator advances inside the body.
        let v1 = shape[it];
        it += 1;
        let v2 = if it == shape.len() {
            shape[0]
        } else {
            shape[it]
        };

        if is_point_in_triangle(x, y, v1, v2, Point { x: 0, y: 0 }) {
            return true;
        }
    }

    false
}

/// `TileAddWrap`, with `construction.freeform_edges` read at the call.
fn tile_add_wrap(w: &World, tile: TileIndex, x: i32, y: i32) -> TileIndex {
    // SAFETY: The module's settings contract.
    let freeform = unsafe { w.shared.settings().freeform_edges.read() };
    w.dimensions.tile_add_wrap(tile, x, y, w.size_y(), freeform)
}

/// Creates a number of tree groups.
/// The number of trees in each group depends on how many trees are actually placed around the
/// given tile. Returns true on world-generation abort.
fn place_tree_groups(c: &Trees, num_groups: u32) -> bool {
    /// How many segments make up the tree group.
    const GROVE_SEGMENTS: usize = 16;
    /// Maximum radius of tree groups.
    const GROVE_RADIUS: u32 = 16;

    // Shape in which trees may be contained. Array is here to reduce allocations.
    let mut grove = [Point::default(); GROVE_SEGMENTS];

    let mut num_groups = num_groups;
    loop {
        let w = World::new(c.s);
        let center_tile = w.dimensions.random_tile_seed(w.size_y(), c.s.random());

        create_random_star_shaped_polygon(c, GROVE_RADIUS.cast_signed(), &mut grove);

        for _ in 0..DEFAULT_TREE_STEPS {
            if (c.t.progress)() {
                return true;
            }
            let w = World::new(c.s);

            let r = c.s.random();
            let x = gb(r, 0, 5).wrapping_sub(GROVE_RADIUS).cast_signed();
            let y = gb(r, 8, 5).wrapping_sub(GROVE_RADIUS).cast_signed();
            let cur_tile = tile_add_wrap(&w, center_tile, x, y);

            if cur_tile == TileIndex::INVALID {
                continue;
            }
            if !can_plant_trees_on_tile(&w, cur_tile, true) {
                continue;
            }
            if !is_point_in_star_shaped_polygon(x, y, &grove) {
                continue;
            }

            place_tree(c, &w, cur_tile, r, false);
        }

        num_groups = num_groups.wrapping_sub(1);
        if num_groups == 0 {
            return false;
        }
    }
}

/// `GetTileZ`.
fn get_tile_z(w: &World, tile: TileIndex) -> i32 {
    let (x, y) = w.size();
    w.asserted(|m| crate::tile_map::get_tile_z(m, tile, x, y))
}

/// Place a tree at the same height as an existing tree.
///
/// Add a new tree around the given tile which is at the same height or at some offset (2
/// units) of `height`, the height like the one from the tile.
fn place_tree_at_same_height(c: &Trees, w: &World, tile: TileIndex, height: i32) {
    for _ in 0..DEFAULT_TREE_STEPS {
        let r = c.s.random();
        let x = gb(r, 0, 5).wrapping_sub(16).cast_signed();
        let y = gb(r, 8, 5).wrapping_sub(16).cast_signed();
        let cur_tile = tile_add_wrap(w, tile, x, y);
        if cur_tile == TileIndex::INVALID {
            continue;
        }

        // Keep in range of the existing tree
        if x.abs() + y.abs() > 16 {
            continue;
        }

        // Clear tile, no farm-tiles or rocks
        if !can_plant_trees_on_tile(w, cur_tile, true) {
            continue;
        }

        // Not too much height difference
        if delta(get_tile_z(w, cur_tile), height) > 2 {
            continue;
        }

        // Place one tree and quit
        place_tree(c, w, cur_tile, r, false);
        break;
    }
}

/// Place some trees randomly.
///
/// This function just place some trees randomly on the map. Returns true on world-generation
/// abort.
pub(crate) fn place_trees_randomly(c: &Trees) -> bool {
    let s = c.s;
    // SAFETY: The module's settings contract.
    let max_height: u8 = unsafe { s.settings().height_limit.read() };

    let w = World::new(s);
    let mut i = w
        .dimensions
        .scale_by_size(w.size_y(), u32::from(DEFAULT_TREE_STEPS))
        .cast_signed();
    if editor() {
        i /= i32::from(EDITOR_TREE_DIV);
    }
    loop {
        'attempt: {
            let w = World::new(s);
            let r = s.random();
            let tile = w.dimensions.random_tile_seed(w.size_y(), r);

            if (c.t.progress)() {
                return true;
            }
            let w = World::new(s);

            if can_plant_trees_on_tile(&w, tile, true) {
                place_tree(c, &w, tile, r, false);
                // SAFETY: The module's settings contract.
                if TreePlacer(unsafe { s.settings().tree_placer.read() }) != TreePlacer::TP_IMPROVED
                {
                    break 'attempt;
                }

                // Place a number of trees based on the tile height.
                //  This gives a cool effect of multiple trees close together.
                //  It is almost real life ;)
                let ht = get_tile_z(&w, tile);
                // The higher we get, the more trees we plant
                let mut j = get_tile_z(&w, tile) * 2;
                // Above snowline more trees!
                if landscape(s) == LandscapeType::Arctic && ht > i32::from(crate::snow_line::get(s))
                {
                    j *= 3;
                }
                // Scale generation by maximum map height.
                if u32::from(max_height) > crate::genworld::MAP_HEIGHT_LIMIT_ORIGINAL {
                    j = (j
                        .cast_unsigned()
                        .wrapping_mul(crate::genworld::MAP_HEIGHT_LIMIT_ORIGINAL)
                        / u32::from(max_height))
                    .cast_signed();
                }
                while j != 0 {
                    j -= 1;
                    place_tree_at_same_height(c, &w, tile, ht);
                }
            }
        }
        i -= 1;
        if i == 0 {
            break;
        }
    }

    // place extra trees at rainforest area
    if landscape(s) == LandscapeType::Tropic {
        let w = World::new(s);
        i = w
            .dimensions
            .scale_by_size(w.size_y(), u32::from(DEFAULT_RAINFOREST_TREE_STEPS))
            .cast_signed();
        if editor() {
            i /= i32::from(EDITOR_TREE_DIV);
        }

        loop {
            let w = World::new(s);
            let r = s.random();
            let tile = w.dimensions.random_tile_seed(w.size_y(), r);

            if (c.t.progress)() {
                return true;
            }
            let w = World::new(s);

            if w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile))
                == TropicZone::TROPICZONE_RAINFOREST
                && can_plant_trees_on_tile(&w, tile, false)
            {
                place_tree(c, &w, tile, r, false);
            }
            i -= 1;
            if i == 0 {
                break;
            }
        }
    }
    false
}

/// Place some trees in a radius around a tile.
/// The trees are placed in an quasi-normal distribution around the indicated tile, meaning that
/// while the radius does define a square, the distribution inside the square will be roughly
/// circular. This function uses the interactive RNG and must only be used in editor and map
/// generation. `treetype` must be a valid tree type for the climate; `radius` is the maximum
/// distance (on each axis) from tile to place trees, `count` the maximum number of trees to
/// place, and `set_zone` whether to create a rainforest zone when placing rainforest trees.
/// Returns the number of trees actually placed.
pub(crate) fn place_tree_group_around_tile(
    c: &Trees,
    random: &crate::genworld::WorldServices,
    tile: TileIndex,
    treetype: TreeType,
    radius: u32,
    count: u32,
    set_zone: bool,
) -> u32 {
    let w = World::new(c.s);
    w.assert_with(editor); // Due to InteractiveRandom being used in this function
    w.assert_with(|| {
        u32::from(treetype.0) < u32::from(TreeType::TREE_TOYLAND.0) + TREE_COUNT_TOYLAND
    });
    let allow_desert = treetype == TreeType::TREE_CACTUS;
    let mut planted = 0_u32;

    for _ in 0..count {
        // Simple quasi-normal distribution with range [-radius; radius)
        let mkcoord = || {
            let rand = (random.interactive_random)();
            let dist = gb(rand, 0, 8) + gb(rand, 8, 8) + gb(rand, 16, 8) + gb(rand, 24, 8);
            let scu = dist.wrapping_mul(radius) / 512;
            scu.wrapping_sub(radius).cast_signed()
        };
        let xofs = mkcoord();
        let yofs = mkcoord();
        let tile_to_plant = tile_add_wrap(&w, tile, xofs, yofs);
        if tile_to_plant != TileIndex::INVALID {
            if w.kind(tile_to_plant) == TileType::MP_TREES
                && w.asserted(|m| get_tree_count(m, tile_to_plant)) < 4
            {
                c.add_tree_count(tile_to_plant, 1);
                c.set_tree_growth(tile_to_plant, TreeGrowthStage::Growing1);
                c.mark_tile_dirty_by_tile(tile_to_plant);
                planted += 1;
            } else if can_plant_trees_on_tile(&w, tile_to_plant, allow_desert) {
                plant_trees_on_tile(c, &w, tile_to_plant, treetype, 0, TreeGrowthStage::Grown);
                c.mark_tile_dirty_by_tile(tile_to_plant);
                planted += 1;
            }
        }
    }

    if set_zone
        && is_inside_mm(
            usize::from(treetype.0),
            usize::from(TreeType::TREE_RAINFOREST.0),
            usize::from(TreeType::TREE_CACTUS.0),
        )
    {
        let map = w.dimensions;
        // The native TileArea(tile) constructor starts with zero extents.
        let mut area = crate::tilearea::TileArea {
            tile: tile.0,
            w: 0,
            h: 0,
        };
        area.expand(radius.cast_signed(), map, w.size_y());
        for t in area.tiles(map) {
            let t = TileIndex(t);
            if w.kind(t) != TileType::MP_VOID
                && map.distance_square(tile, t) < radius.wrapping_mul(radius)
            {
                (c.s.set_tropic_zone)(t.0, TropicZone::TROPICZONE_RAINFOREST.0);
            }
        }
    }

    planted
}

/// Place new trees.
///
/// This function takes care of the selected tree placer algorithm and place randomly the trees
/// for a new game. Returns true on world-generation abort.
pub(crate) fn generate_trees(c: &Trees) -> bool {
    let s = c.s;
    // SAFETY: The module's settings contract.
    let tree_placer = || TreePlacer(unsafe { s.settings().tree_placer.read() });

    if tree_placer() == TreePlacer::TP_NONE {
        return false;
    }

    let mut i: u32 = match tree_placer() {
        TreePlacer::TP_ORIGINAL => {
            if landscape(s) == LandscapeType::Arctic {
                15
            } else {
                6
            }
        }
        TreePlacer::TP_IMPROVED => {
            if landscape(s) == LandscapeType::Arctic {
                4
            } else {
                2
            }
        }
        _ => not_reached(),
    };

    let w = World::new(s);
    let scale_by_size = |n: u32| w.dimensions.scale_by_size(w.size_y(), n);
    let mut total = scale_by_size(u32::from(DEFAULT_TREE_STEPS));
    if landscape(s) == LandscapeType::Tropic {
        total = total.wrapping_add(scale_by_size(u32::from(DEFAULT_RAINFOREST_TREE_STEPS)));
    }
    total = total.wrapping_mul(i);
    let num_groups = if landscape(s) == LandscapeType::Toyland {
        0
    } else {
        scale_by_size(gb(s.random(), 0, 5) + 25)
    };
    total = total.wrapping_add(num_groups.wrapping_mul(u32::from(DEFAULT_TREE_STEPS)));
    if (c.t.progress_total)(total) {
        return true;
    }

    if num_groups != 0 && place_tree_groups(c, num_groups) {
        return true;
    }

    while i != 0 {
        if place_trees_randomly(c) {
            return true;
        }
        i -= 1;
    }
    false
}

/// Plant a tree. `args.tile` is the end tile of area-drag; `args.tree_to_plant`
/// `TREE_INVALID` means random. Writes the cost of this operation or an error to `out`.
fn cmd_plant_tree(
    c: &Trees,
    commands: &CommandServices,
    flags: DoCommandFlags,
    args: &PlantTreeArgs,
    out: Cost<'_>,
) {
    let s = c.s;
    let strings = &c.t.strings;
    let mut msg = StringID::INVALID_STRING_ID;
    // CommandCost cost(EXPENSES_OTHER): `AddCost` saturates.
    let mut cost: i64 = 0;
    let w = World::new(s);

    if args.start_tile >= w.types.count() {
        out.error(StringID::INVALID_STRING_ID.0);
        return;
    }
    // Check the tree type within the current climate
    let climate = usize::from(landscape(s).0);
    if TreeType(args.tree_to_plant) != TreeType::TREE_INVALID
        && !is_inside_bs(
            usize::from(args.tree_to_plant),
            usize::from(TREE_BASE_BY_LANDSCAPE[climate]),
            usize::from(TREE_COUNT_BY_LANDSCAPE[climate]),
        )
    {
        out.error(StringID::INVALID_STRING_ID.0);
        return;
    }

    let company = if editor() {
        std::ptr::null_mut()
    } else {
        (commands.finance.company_finances)((commands.finance.current_company)())
    };
    let mut limit = if company.is_null() {
        i32::MAX
    } else {
        // SAFETY: A valid company's finances; one field-sized load.
        gb(
            unsafe { std::ptr::addr_of!((*company).tree).read() },
            16,
            16,
        )
        .cast_signed()
    };
    // `c->tree_limit -= 1 << 16`.
    let debit = || {
        if !company.is_null() {
            // SAFETY: As above; one field-sized load and store with no call between.
            unsafe {
                let field = std::ptr::addr_of_mut!((*company).tree);
                field.write(field.read().wrapping_sub(1 << 16));
            }
        }
    };

    let check = |ok| w.check(ok);
    let mut iter = crate::tilearea::TileIterator::create(
        w.dimensions,
        w.types.count(),
        args.tile,
        args.start_tile,
        args.diagonal,
        check,
    );
    'tiles: while iter.tile() != TileIndex::INVALID.0 {
        let current_tile = TileIndex(iter.tile());
        'next: {
            match w.kind(current_tile) {
                TileType::MP_TREES => {
                    // no more space for trees?
                    if w.asserted(|m| get_tree_count(m, current_tile)) == 4 {
                        msg = StringID(strings.already_here);
                        break 'next;
                    }

                    // Test tree limit.
                    limit -= 1;
                    if limit < 1 {
                        msg = StringID(strings.plant_limit);
                    } else {
                        if flags.test(DoCommandFlag::Execute) {
                            c.add_tree_count(current_tile, 1);
                            c.mark_tile_dirty_by_tile(current_tile);
                            debit();
                        }
                        // 2x as expensive to add more trees to an existing tile
                        cost = cost.saturating_add(
                            crate::company::price(PR_BUILD_TREES).saturating_mul(2),
                        );
                    }
                }

                kind @ (TileType::MP_WATER | TileType::MP_CLEAR) => {
                    if kind == TileType::MP_WATER
                        && (!w.asserted(|m| crate::water_map::is_coast(m, current_tile.0))
                            || crate::slope_func::is_slope_with_one_corner_raised(
                                w.tile_slope(current_tile),
                            ))
                    {
                        msg = StringID(strings.on_water);
                        break 'next;
                    }

                    if w.bridge_above(current_tile.0) {
                        msg = StringID(strings.site_unsuitable);
                        break 'next;
                    }

                    let mut treetype = TreeType(args.tree_to_plant);
                    let zone = || w.asserted(|m| crate::tile_map::get_tropic_zone(m, current_tile));
                    // Be a bit picky about which trees go where.
                    if landscape(s) == LandscapeType::Tropic
                        && treetype != TreeType::TREE_INVALID
                        && (
                            // No cacti outside the desert
                            (treetype == TreeType::TREE_CACTUS
                                && zone() != TropicZone::TROPICZONE_DESERT)
                                // No rainforest trees outside the rainforest, except in the editor mode where it makes those tiles rainforest tile
                                || (is_inside_mm(
                                    usize::from(treetype.0),
                                    usize::from(TreeType::TREE_RAINFOREST.0),
                                    usize::from(TreeType::TREE_CACTUS.0),
                                ) && zone() != TropicZone::TROPICZONE_RAINFOREST
                                    && !editor())
                                // And no subtropical trees in the desert/rainforest
                                || (is_inside_mm(
                                    usize::from(treetype.0),
                                    usize::from(TreeType::TREE_SUB_TROPICAL.0),
                                    usize::from(TreeType::TREE_TOYLAND.0),
                                ) && zone() != TropicZone::TROPICZONE_NORMAL)
                        )
                    {
                        msg = StringID(strings.wrong_terrain);
                        break 'next;
                    }

                    // Test tree limit.
                    limit -= 1;
                    if limit < 1 {
                        msg = StringID(strings.plant_limit);
                    } else {
                        if w.kind(current_tile) == TileType::MP_CLEAR {
                            // Remove fields or rocks. Note that the ground will get barrened
                            match w.asserted(|m| get_clear_ground(m, current_tile)) {
                                ClearGround::CLEAR_FIELDS | ClearGround::CLEAR_ROCKS => {
                                    crate::landscape_cmd::clear_command(
                                        commands,
                                        out.0,
                                        current_tile.0,
                                        flags.0,
                                    );
                                    // A failed clear is the command's result.
                                    if !out.ok() {
                                        return;
                                    }
                                    cost = cost.saturating_add(out.money());
                                }
                                _ => {}
                            }
                        }

                        if !editor() && (s.company_valid)((s.current_company)()) {
                            let t = crate::town::spatial::closest_town_from_tile(
                                s,
                                current_tile,
                                u32::from(crate::town::authority_distance(s)),
                            );
                            if !t.is_null() {
                                c.change_town_rating(
                                    t,
                                    crate::town_type::RATING_TREE_UP_STEP,
                                    crate::town_type::RATING_TREE_MAXIMUM,
                                    flags,
                                );
                            }
                        }

                        if flags.test(DoCommandFlag::Execute) {
                            if treetype == TreeType::TREE_INVALID {
                                treetype =
                                    get_random_tree_type(&w, current_tile, gb(s.random(), 24, 8));
                                if treetype == TreeType::TREE_INVALID {
                                    treetype = TreeType::TREE_CACTUS;
                                }
                            }

                            // Plant full grown trees in scenario editor
                            plant_trees_on_tile(
                                c,
                                &w,
                                current_tile,
                                treetype,
                                0,
                                if editor() {
                                    TreeGrowthStage::Grown
                                } else {
                                    TreeGrowthStage::Growing1
                                },
                            );
                            c.mark_tile_dirty_by_tile(current_tile);
                            debit();

                            // When planting rainforest-trees, set tropiczone to rainforest in editor.
                            if editor()
                                && is_inside_mm(
                                    usize::from(treetype.0),
                                    usize::from(TreeType::TREE_RAINFOREST.0),
                                    usize::from(TreeType::TREE_CACTUS.0),
                                )
                            {
                                (s.set_tropic_zone)(
                                    current_tile.0,
                                    TropicZone::TROPICZONE_RAINFOREST.0,
                                );
                            }
                        }
                        cost = cost.saturating_add(crate::company::price(PR_BUILD_TREES));
                    }
                }

                _ => msg = StringID(strings.site_unsuitable),
            }

            // Tree limit used up? No need to check more.
            if limit < 0 {
                break 'tiles;
            }
        }
        iter.advance(w.dimensions, w.types.count(), check);
    }

    if cost == 0 {
        out.error(msg.0);
    } else {
        (out.1.expense_money)(out.0, ExpensesType::EXPENSES_OTHER.0, cost);
    }
}

/// `ClearTile_Trees`; returns the cost, of `EXPENSES_CONSTRUCTION`.
pub(crate) fn clear_tile_trees(c: &Trees, tile: TileIndex, flags: DoCommandFlags) -> i64 {
    let s = c.s;

    if (s.company_valid)((s.current_company)()) {
        let t = crate::town::spatial::closest_town_from_tile(
            s,
            tile,
            u32::from(crate::town::authority_distance(s)),
        );
        if !t.is_null() {
            c.change_town_rating(
                t,
                crate::town_type::RATING_TREE_DOWN_STEP,
                crate::town_type::RATING_TREE_MINIMUM,
                flags,
            );
        }
    }

    let w = World::new(s);
    let mut num = u32::from(w.asserted(|m| get_tree_count(m, tile)));
    if is_inside_mm(
        usize::from(w.asserted(|m| get_tree_type(m, tile)).0),
        usize::from(TreeType::TREE_RAINFOREST.0),
        usize::from(TreeType::TREE_CACTUS.0),
    ) {
        num *= 4;
    }

    if flags.test(DoCommandFlag::Execute) {
        crate::ground::clear_square(tile.0, s);
    }

    i64::from(num).saturating_mul(crate::company::price(PR_CLEAR_TREES))
}

/// `TileLoopTreesDesert`.
fn tile_loop_trees_desert(c: &Trees, w: &World, tile: TileIndex, settings: &LoopSettings) {
    match w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile)) {
        TropicZone::TROPICZONE_DESERT => {
            if w.asserted(|m| get_tree_ground(m, tile)) != TreeGround::TREE_GROUND_SNOW_DESERT {
                c.set_tree_ground_density(tile, TreeGround::TREE_GROUND_SNOW_DESERT, 3);
                c.mark_tile_dirty_by_tile(tile);
            }
        }

        TropicZone::TROPICZONE_RAINFOREST => {
            const FOREST_SOUNDS: [SoundFx; 4] = [
                SoundFx::SND_42_RAINFOREST_1,
                SoundFx::SND_43_RAINFOREST_2,
                SoundFx::SND_44_RAINFOREST_3,
                SoundFx::SND_48_RAINFOREST_4,
            ];
            let r = c.s.random();

            if chance16_i(1, 200, r) && settings.ambient {
                (c.s.play_sound)(tile.0, u16::from(FOREST_SOUNDS[gb(r, 16, 2) as usize].0));
            }
        }

        _ => {}
    }
}

/// `TileLoopTreesAlps`.
#[allow(clippy::cast_sign_loss, reason = "std::min<uint> of a nonnegative int")]
fn tile_loop_trees_alps(c: &Trees, w: &World, tile: TileIndex, settings: &LoopSettings) {
    let k = get_tile_z(w, tile) - i32::from(crate::snow_line::get(c.s)) + 1;

    if k < 0 {
        match w.asserted(|m| get_tree_ground(m, tile)) {
            TreeGround::TREE_GROUND_SNOW_DESERT => {
                c.set_tree_ground_density(tile, TreeGround::TREE_GROUND_GRASS, 3);
            }
            TreeGround::TREE_GROUND_ROUGH_SNOW => {
                c.set_tree_ground_density(tile, TreeGround::TREE_GROUND_ROUGH, 3);
            }
            _ => return,
        }
    } else {
        let density = (k as u32).min(3);

        if w.asserted(|m| get_tree_ground(m, tile)) != TreeGround::TREE_GROUND_SNOW_DESERT
            && w.asserted(|m| get_tree_ground(m, tile)) != TreeGround::TREE_GROUND_ROUGH_SNOW
        {
            let tg = if w.asserted(|m| get_tree_ground(m, tile)) == TreeGround::TREE_GROUND_ROUGH {
                TreeGround::TREE_GROUND_ROUGH_SNOW
            } else {
                TreeGround::TREE_GROUND_SNOW_DESERT
            };
            c.set_tree_ground_density(tile, tg, density);
        } else if u32::from(w.asserted(|m| get_tree_density(m, tile))) != density {
            c.set_tree_ground_density(tile, w.asserted(|m| get_tree_ground(m, tile)), density);
        } else {
            if w.asserted(|m| get_tree_density(m, tile)) == 3 {
                let r = c.s.random();
                if chance16_i(1, 200, r) && settings.ambient {
                    let sound = if r & 0x8000_0000 != 0 {
                        SoundFx::SND_39_ARCTIC_SNOW_2
                    } else {
                        SoundFx::SND_34_ARCTIC_SNOW_1
                    };
                    (c.s.play_sound)(tile.0, u16::from(sound.0));
                }
            }
            return;
        }
    }
    c.mark_tile_dirty_by_tile(tile);
}

/// Check if trees on this tile are allowed to spread.
/// If they are allowed to spread, they are also allowed to die.
fn trees_on_tile_can_spread(w: &World, tile: TileIndex, settings: &LoopSettings) -> bool {
    // Desert and rainforest trees need special handling.
    if settings.climate == LandscapeType::Tropic {
        return match w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile)) {
            // Cacti never spread.
            TropicZone::TROPICZONE_DESERT => false,
            TropicZone::TROPICZONE_RAINFOREST => {
                settings.extra == ExtraTreePlacement::ETP_SPREAD_ALL
                    || settings.extra == ExtraTreePlacement::ETP_SPREAD_RAINFOREST
            }
            _ => settings.extra == ExtraTreePlacement::ETP_SPREAD_ALL,
        };
    }

    settings.extra == ExtraTreePlacement::ETP_SPREAD_ALL
}

/// `TileLoop_Trees`. `types` is the tile-loop batch's map descriptor; `call` is the native
/// lazy-result context of the flooding it may run.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the uint32_t cycle of a uint64_t sum"
)]
fn tile_loop_trees(
    c: &Trees,
    types: &TileTypeAccess,
    tile: TileIndex,
    settings: &LoopSettings,
    call: *mut crate::ground::Call,
) {
    /// How many tile updates happen for one tree update
    const TREE_UPDATE_FREQUENCY: u32 = 16;

    let w = World {
        types: types.without_native_assert(),
        shared: c.s,
        dimensions: Map::from_log_x(settings.size_x.trailing_zeros()),
    };
    let mut tile = tile;

    if w.asserted(|m| get_tree_ground(m, tile)) == TreeGround::TREE_GROUND_SHORE {
        // SAFETY: The native caller owns an independent stack result context; flooding may
        // reenter trees but cannot reset the map, and no owner state is borrowed here.
        unsafe {
            crate::water::water_tile_loop(
                tile.0,
                TileType::MP_TREES.0,
                false,
                settings.size_x,
                w.size_y(),
                call,
                (c.s.water_services)(),
                c.s,
                &w.types,
            );
        }
    } else {
        match settings.climate {
            LandscapeType::Tropic => tile_loop_trees_desert(c, &w, tile, settings),
            LandscapeType::Arctic => tile_loop_trees_alps(c, &w, tile, settings),
            _ => {}
        }
    }

    crate::newgrf_generic::ambient(settings.ambient_callback, tile.0);

    // TimerGameTick::counter is incremented by 256 between each call, so ignore lower 8 bits.
    // Also, we use a simple hash to spread the updates evenly over the map.
    // 11 and 9 are just some co-prime numbers for better spread.
    let cycle = (11 * w.dimensions.tile_x(tile))
        .wrapping_add(9 * w.dimensions.tile_y(tile))
        .wrapping_add((settings.tick_counter >> 8) as u32);

    // Handle growth of grass (under trees/on MP_TREES tiles) at every 8th processings, like
    // it's done for grass on MP_CLEAR tiles.
    if (cycle & 7) == 7 && w.asserted(|m| get_tree_ground(m, tile)) == TreeGround::TREE_GROUND_GRASS
    {
        let density = u32::from(w.asserted(|m| get_tree_density(m, tile)));
        if density < 3 {
            c.set_tree_ground_density(tile, TreeGround::TREE_GROUND_GRASS, density + 1);
            c.mark_tile_dirty_by_tile(tile);
        }
    }

    if settings.extra == ExtraTreePlacement::ETP_NO_GROWTH_NO_SPREAD {
        return;
    }

    if cycle % TREE_UPDATE_FREQUENCY != TREE_UPDATE_FREQUENCY - 1 {
        return;
    }

    match w.asserted(|m| get_tree_growth(m, tile)) {
        // regular sized tree
        TreeGrowthStage::Grown => {
            if settings.climate == LandscapeType::Tropic
                && w.asserted(|m| get_tree_type(m, tile)) != TreeType::TREE_CACTUS
                && w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile))
                    == TropicZone::TROPICZONE_DESERT
            {
                c.add_tree_growth(tile, 1);
            } else {
                let spread = match gb(c.s.random(), 0, 3) {
                    // start destructing
                    0 => {
                        c.add_tree_growth(tile, 1);
                        false
                    }
                    // add a tree
                    1 => {
                        if w.asserted(|m| get_tree_count(m, tile)) < 4
                            && trees_on_tile_can_spread(&w, tile, settings)
                        {
                            c.add_tree_count(tile, 1);
                            c.set_tree_growth(tile, TreeGrowthStage::Growing1);
                            false
                        } else {
                            true
                        }
                    }
                    2 => true,
                    _ => return,
                };
                // add a neighbouring tree
                if spread && trees_on_tile_can_spread(&w, tile, settings) {
                    let treetype = w.asserted(|m| get_tree_type(m, tile));

                    let direction = crate::direction_type::Direction(
                        (c.s.random() % u32::from(crate::direction_type::Direction::DIR_END.0))
                            as u8,
                    );
                    tile = TileIndex(tile.0.wrapping_add_signed(
                        w.dimensions.tile_offs_by_dir(direction, |ok| w.check(ok)),
                    ));

                    if !can_plant_trees_on_tile(&w, tile, false) {
                        return;
                    }

                    // Don't plant trees, if ground was freshly cleared
                    if w.kind(tile) == TileType::MP_CLEAR
                        && w.asserted(|m| get_clear_ground(m, tile)) == ClearGround::CLEAR_GRASS
                        && !w.asserted(|m| crate::clear_map::is_snow_tile(m, tile))
                        && w.asserted(|m| get_clear_density(m, tile)) != 3
                    {
                        return;
                    }

                    plant_trees_on_tile(c, &w, tile, treetype, 0, TreeGrowthStage::Growing1);
                }
            }
        }

        // final stage of tree destruction
        TreeGrowthStage::Dead => {
            if !trees_on_tile_can_spread(&w, tile, settings) {
                // if trees can't spread just plant a new one to prevent deforestation
                c.set_tree_growth(tile, TreeGrowthStage::Growing1);
            } else if w.asserted(|m| get_tree_count(m, tile)) > 1 {
                // more than one tree, delete it
                c.add_tree_count(tile, -1);
                c.set_tree_growth(tile, TreeGrowthStage::Grown);
            } else {
                // just one tree, change type into MP_CLEAR
                match w.asserted(|m| get_tree_ground(m, tile)) {
                    TreeGround::TREE_GROUND_SHORE => (c.s.make_shore)(tile.0),
                    TreeGround::TREE_GROUND_GRASS => c.make_clear(
                        tile,
                        ClearGround::CLEAR_GRASS,
                        u32::from(w.asserted(|m| get_tree_density(m, tile))),
                    ),
                    TreeGround::TREE_GROUND_ROUGH => {
                        c.make_clear(tile, ClearGround::CLEAR_ROUGH, 3);
                    }
                    TreeGround::TREE_GROUND_ROUGH_SNOW => {
                        let density = u32::from(w.asserted(|m| get_tree_density(m, tile)));
                        c.make_clear(tile, ClearGround::CLEAR_ROUGH, 3);
                        (c.s.make_snow)(tile.0, density);
                    }
                    // snow or desert
                    _ => {
                        if settings.climate == LandscapeType::Tropic {
                            c.make_clear(
                                tile,
                                ClearGround::CLEAR_DESERT,
                                u32::from(w.asserted(|m| get_tree_density(m, tile))),
                            );
                        } else {
                            let density = u32::from(w.asserted(|m| get_tree_density(m, tile)));
                            c.make_clear(tile, ClearGround::CLEAR_GRASS, 3);
                            (c.s.make_snow)(tile.0, density);
                        }
                    }
                }
            }
        }

        _ => c.add_tree_growth(tile, 1),
    }

    c.mark_tile_dirty_by_tile(tile);
}

/// Decrement the tree tick counter.
/// The interval is scaled by map size to allow for the same density regardless of size.
/// Adjustment for map sizes below the standard 256 * 256 are handled earlier.
/// Returns true if the counter was decremented past zero.
#[allow(clippy::cast_possible_truncation, reason = "uint8_t -= uint")]
fn decrement_tree_counter(w: &World) -> bool {
    // Ensure _trees_tick_ctr can be decremented past zero only once for the largest map size:
    // tree_cmd.cpp keeps the original's static_assert.

    // byte underflow
    let counter = &raw mut _trees_tick_ctr;
    // SAFETY: The counter's contract; no call runs between the load and the store.
    unsafe {
        let old_trees_tick_ctr = counter.read();
        counter.write(
            u32::from(old_trees_tick_ctr).wrapping_sub(w.dimensions.scale_by_size(w.size_y(), 1))
                as u8,
        );
        old_trees_tick_ctr <= counter.read()
    }
}

/// Place a random tree on a random tile.
/// If `rainforest` is set the random tile must be in a rainforest zone.
fn plant_random_tree(c: &Trees, w: &World, rainforest: bool) {
    let r = c.s.random();
    let tile = w.dimensions.random_tile_seed(w.size_y(), r);

    if rainforest
        && w.asserted(|m| crate::tile_map::get_tropic_zone(m, tile))
            != TropicZone::TROPICZONE_RAINFOREST
    {
        return;
    }
    if !can_plant_trees_on_tile(w, tile, false) {
        return;
    }

    let tree = get_random_tree_type(w, tile, gb(r, 24, 8));
    if tree == TreeType::TREE_INVALID {
        return;
    }

    plant_trees_on_tile(c, w, tile, tree, 0, TreeGrowthStage::Growing1);
}

/// `OnTick_Trees`.
pub(crate) fn on_tick_trees(c: &Trees) {
    let s = c.s;
    // SAFETY: The module's settings contract.
    let extra_tree_placement =
        || ExtraTreePlacement(unsafe { s.settings().extra_tree_placement.read() });

    // Don't spread trees if that's not allowed
    if extra_tree_placement() == ExtraTreePlacement::ETP_NO_SPREAD
        || extra_tree_placement() == ExtraTreePlacement::ETP_NO_GROWTH_NO_SPREAD
    {
        return;
    }

    let w = World::new(s);
    // Skip some tree ticks for map sizes below 256 * 256. 64 * 64 is 16 times smaller, so
    // this is the maximum number of ticks that are skipped. Number of ticks to skip is
    // inversely proportional to map size, so that is handled to create a mask.
    let skip = w.dimensions.scale_by_size(w.size_y(), 16).cast_signed();
    if skip < 16 && ((s.tick_counter)() & i64::from(16 / skip - 1).cast_unsigned()) != 0 {
        return;
    }

    // place a tree at a random rainforest spot
    if landscape(s) == LandscapeType::Tropic {
        let mut c_ = w.dimensions.scale_by_size(w.size_y(), 1);
        while c_ > 0 {
            plant_random_tree(c, &w, true);
            c_ -= 1;
        }
    }

    if !decrement_tree_counter(&w)
        || extra_tree_placement() == ExtraTreePlacement::ETP_SPREAD_RAINFOREST
    {
        return;
    }

    // place a tree at a random spot
    plant_random_tree(c, &w, false);
}

/// `InitializeTrees`.
pub(crate) fn initialize_trees() {
    // SAFETY: The counter's contract.
    unsafe { (&raw mut _trees_tick_ctr).write(0) };
}

/// Raw adapter for the tile-loop scheduler: `TileLoop_Trees` over base values.
/// # Safety
/// The tables are live and immutable; `map` follows the scheduler's no-reset, no-yield
/// contract for the batch.
pub(crate) unsafe fn tile_loop(
    tile: u32,
    settings: LoopSettings,
    call: *mut crate::ground::Call,
    services: *const TreeServices,
    shared: *const Services,
    map: &TileTypeAccess,
) {
    // SAFETY: The caller's contract.
    let c = unsafe { Trees::new(services, shared) };
    tile_loop_trees(&c, map, TileIndex(tile), &settings, call);
}

/// `OnTick_Trees` from the session tick.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
pub(crate) unsafe fn tick(services: *const TreeServices, shared: *const Services) {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    on_tick_trees(&unsafe { Trees::new(services, shared) });
}

/// `GenerateTrees` from world generation; true when a progress service reported a
/// world-generation abort.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
pub(crate) unsafe fn generate(services: *const TreeServices, shared: *const Services) -> bool {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    generate_trees(&unsafe { Trees::new(services, shared) })
}

/// Raw adapter for the landscape clear dispatch: `ClearTile_Trees` over base values.
pub(crate) fn clear_tile(t: &TreeServices, s: &Services, tile: u32, flags: u16) -> i64 {
    clear_tile_trees(&Trees { t, s }, TileIndex(tile), DoCommandFlags(flags))
}

/// `InitializeTrees`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trees_initialize() {
    initialize_trees();
}

/// `PlaceTreesRandomly`; 1 when a progress service reported a world-generation abort.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_place_randomly_abi(
    services: *const TreeServices,
    shared: *const Services,
) -> u8 {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    u8::from(place_trees_randomly(&unsafe {
        Trees::new(services, shared)
    }))
}

/// `PlaceTree`.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_place(
    tile: u32,
    r: u32,
    keep_density: bool,
    services: *const TreeServices,
    shared: *const Services,
) {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    let c = unsafe { Trees::new(services, shared) };
    place_tree(&c, &World::new(c.s), TileIndex(tile), r, keep_density);
}

/// `PlaceTreeGroupAroundTile`, called by the native scenario-editor brush; a positional shim
/// over the typed entry.
/// # Safety
/// Tables and canonical map remain live under the game-state mutex; no reset or yield.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_place_group(
    tile: u32,
    treetype: u8,
    radius: u32,
    count: u32,
    set_zone: bool,
    services: *const TreeServices,
    shared: *const Services,
    random: &crate::genworld::WorldServices,
) -> u32 {
    // SAFETY: The caller supplies live immutable tables for this serialized entry.
    let c = unsafe { Trees::new(services, shared) };
    place_tree_group_around_tile(
        &c,
        random,
        TileIndex(tile),
        TreeType(treetype),
        radius,
        count,
        set_zone,
    )
}

/// `CmdPlantTree`, writing its result to the native slot `out`.
/// # Safety
/// Tables are live and immutable; game thread only. Services are `noexcept`. `args` and the
/// native result slot live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_trees_cmd_plant(
    flags: u16,
    args: *const PlantTreeArgs,
    out: *mut CommandCost,
    services: *const TreeServices,
    shared: *const Services,
    commands: *const CommandServices,
) {
    // SAFETY: The caller's contract.
    unsafe {
        let commands = &*commands;
        cmd_plant_tree(
            &Trees::new(services, shared),
            commands,
            DoCommandFlags(flags),
            &*args,
            Cost(out, commands.cost),
        );
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTreeServices" ["probe"] => crate::trees::TreeServices {
        add_count,
        add_growth,
        authority,
        cos,
        make_clear,
        make_tree,
        progress,
        progress_total,
        set_ground_density,
        set_growth,
        sin,
        strings,
        town,
    };
    "OpenTTDTreeStrings" ["probe"] => Strings {
        already_here,
        on_water,
        plant_limit,
        site_unsuitable,
        wrong_terrain,
    };
    "OpenTTDTreeLoopSettings" ["probe"] => LoopSettings {
        tick_counter,
        size_x,
        climate,
        extra,
        ambient,
        ambient_callback,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Draw,
        Progress,
        Total(u32),
        Write(&'static str, u32),
        Dirty(u32),
        Zone(u32, u8),
        Sound(u16),
        Assert,
        Error(u32),
        Expense(u8, i64),
        Authority(u32),
    }

    thread_local! {
        static BASE: Cell<*mut BaseTileLayout> = const { Cell::new(std::ptr::null_mut()) };
        static LOG: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
        static RANDOM: Cell<u32> = const { Cell::new(0) };
        static PROGRESS: Cell<usize> = const { Cell::new(0) };
        static ABORT_AT: Cell<usize> = const { Cell::new(0) };
        static PLANTED_GROUND: Cell<Option<u8>> = const { Cell::new(None) };
        static COMPANY: Cell<*mut crate::company::Finances> =
            const { Cell::new(std::ptr::null_mut()) };
        static COMPANY_VALID: Cell<bool> = const { Cell::new(false) };
        static CLIMATE: Cell<u8> = const { Cell::new(0) };
        static PLACER: Cell<u8> = const { Cell::new(1) };
        static EXTRA: Cell<u8> = const { Cell::new(2) };
        static FREEFORM: Cell<bool> = const { Cell::new(true) };
        static HEIGHT_LIMIT: Cell<u8> = const { Cell::new(15) };
        static AUTHORITY: Cell<u8> = const { Cell::new(20) };
        static SNOW_LINE: Cell<u8> = const { Cell::new(4) };
        static TOWN: Cell<*mut crate::town::State> = const { Cell::new(std::ptr::null_mut()) };
        static COMMANDS: Cell<*const CommandServices> = const { Cell::new(std::ptr::null()) };
    }
    fn log(event: Event) {
        LOG.with_borrow_mut(|l| l.push(event));
    }
    fn events() -> Vec<Event> {
        LOG.with_borrow_mut(std::mem::take)
    }

    /// A 64x64 (or larger) map of real tile bytes. The tree and clear writer mocks store
    /// the fields `tree_map.h` and `clear_map.h` store.
    struct TestMap {
        _base: Vec<BaseTileLayout>,
        _extended: Vec<ExtendedTileLayout>,
    }
    fn install(log_x: u32, kind: TileType, assertions: bool) -> TestMap {
        let count = 1_usize << (2 * log_x);
        let mut base: Vec<_> = (0..count)
            .map(|_| BaseTileLayout {
                kind: kind.0 << 4,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let extended: Vec<_> = (0..count)
            .map(|_| ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        BASE.set(base.as_mut_ptr());
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: base.as_ptr(),
                extended: extended.as_ptr(),
                count: u32::try_from(count).unwrap(),
                assertions,
            },
            log_x,
        );
        LOG.with_borrow_mut(Vec::clear);
        PROGRESS.set(0);
        ABORT_AT.set(0);
        RANDOM.set(0);
        PLANTED_GROUND.set(None);
        TestMap {
            _base: base,
            _extended: extended,
        }
    }
    fn edit(tile: TileIndex, f: impl FnOnce(&mut BaseTileLayout)) {
        // SAFETY: The installed test map; no other reference to the tile is live.
        f(unsafe { &mut *BASE.get().add(tile.0 as usize) });
    }
    fn byte(tile: TileIndex, f: impl FnOnce(&BaseTileLayout) -> u8) -> u8 {
        // SAFETY: As `edit`.
        f(unsafe { &*BASE.get().add(tile.0 as usize) })
    }
    fn set_type(b: &mut BaseTileLayout, kind: TileType) {
        b.kind = (kind.0 << 4) | (b.kind & 0x0f);
    }
    fn clear(tile: TileIndex, ground: ClearGround, density: u8) {
        edit(tile, |b| {
            set_type(b, TileType::MP_CLEAR);
            b.m5 = (ground.0 << 2) | density;
            b.m3 = 0;
        });
    }
    fn tree(
        tile: TileIndex,
        r#type: TreeType,
        count: u8,
        growth: TreeGrowthStage,
        ground: TreeGround,
    ) {
        edit(tile, |b| {
            set_type(b, TileType::MP_TREES);
            b.m3 = r#type.0;
            b.m2 = (u16::from(ground.0) << 6) | (3 << 4);
            b.m5 = ((count - 1) << 6) | growth.0;
        });
    }
    fn tree_ground(TileIndex(tile): TileIndex) -> TreeGround {
        TreeGround(byte(TileIndex(tile), |b| {
            u8::try_from((b.m2 >> 6) & 7).unwrap()
        }))
    }
    fn growth(TileIndex(tile): TileIndex) -> TreeGrowthStage {
        TreeGrowthStage(byte(TileIndex(tile), |b| b.m5 & 7))
    }

    extern "C" fn random(_: *mut c_void) -> u32 {
        log(Event::Draw);
        RANDOM.get()
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    extern "C" fn tile_type(tile: u32) -> u8 {
        byte(TileIndex(tile), |b| b.kind >> 4)
    }
    extern "C" fn dirty(tile: u32) {
        log(Event::Dirty(tile));
    }
    extern "C" fn set_zone(tile: u32, zone: u8) {
        log(Event::Zone(tile, zone));
    }
    extern "C" fn make_shore(tile: u32) {
        log(Event::Write("shore", tile));
    }
    extern "C" fn make_snow(tile: u32, _: u32) {
        log(Event::Write("snow", tile));
    }
    extern "C" fn sound(_: u32, id: u16) {
        log(Event::Sound(id));
    }
    extern "C" fn record_assert(ok: bool) {
        if !ok {
            log(Event::Assert);
        }
    }
    extern "C" fn tick_counter() -> u64 {
        0
    }
    extern "C" fn max_xy() -> u32 {
        63
    }
    extern "C" fn current_company() -> u8 {
        0
    }
    extern "C" fn company_valid(_: u8) -> bool {
        COMPANY_VALID.get()
    }
    extern "C" fn company_finances(_: u8) -> *mut crate::company::Finances {
        COMPANY.get()
    }
    extern "C" fn no_flag(_: u32) -> bool {
        false
    }
    extern "C" fn not_generating() -> bool {
        false
    }
    extern "C" fn clear_square(tile: u32, _: crate::services::ClearGroundType, _: u32) {
        log(Event::Write("clear_square", tile));
    }
    extern "C" fn water_services() -> *const crate::water::WaterServices {
        thread_local! { static WATER: crate::water::WaterServices = {
            let mut water = crate::water::fixture();
            water.clear = Box::leak(Box::new(crate::ground::ClearServices {
                make_clear: clear_square,
                ..crate::test_tables::CLEAR_GROUND
            }));
            water
        }; }
        WATER.with(std::ptr::from_ref)
    }
    extern "C" fn town_count() -> usize {
        usize::from(!TOWN.get().is_null())
    }
    extern "C" fn town_next_id(from: u32) -> u32 {
        if from == 0 && !TOWN.get().is_null() {
            0
        } else {
            u32::MAX
        }
    }
    extern "C" fn town_owner_by_id(_: u32) -> *mut crate::town::State {
        TOWN.get()
    }
    extern "C" fn town_by_id(_: u32) -> *mut TownHandle {
        std::ptr::without_provenance_mut(0x40)
    }
    extern "C" fn town_owner(_: *const TownHandle) -> *mut crate::town::State {
        TOWN.get()
    }
    extern "C" fn town_index(_: *const TownHandle) -> u16 {
        0
    }
    extern "C" fn bulldozer() -> bool {
        false
    }
    extern "C" fn dirty_authority(town: u32) {
        log(Event::Authority(town));
    }

    fn settings() -> &'static crate::settings_type::SettingAddresses {
        Box::leak(Box::new(crate::settings_type::SettingAddresses {
            climate: CLIMATE.with(Cell::as_ptr).cast_const(),
            tree_placer: PLACER.with(Cell::as_ptr).cast_const(),
            extra_tree_placement: EXTRA.with(Cell::as_ptr).cast_const(),
            freeform_edges: FREEFORM.with(Cell::as_ptr).cast_const(),
            height_limit: HEIGHT_LIMIT.with(Cell::as_ptr).cast_const(),
            dist_local_authority: AUTHORITY.with(Cell::as_ptr).cast_const(),
            ..crate::settings_type::SettingAddresses::NULL
        }))
    }
    fn shared() -> &'static Services {
        let fixture = crate::services::fixture(std::ptr::null_mut(), random, industry);
        Box::leak(Box::new(Services {
            settings: settings(),
            snow_line_support: crate::snow_line::Support {
                height: SNOW_LINE.with(Cell::as_ptr).cast_const(),
                ..fixture.snow_line_support
            },
            movement: Box::leak(Box::new(crate::movement::MovementServices {
                native_assert: record_assert,
                ..crate::test_tables::MOVEMENT
            })),
            tile_type,
            mark_dirty: dirty,
            set_tropic_zone: set_zone,
            make_shore,
            make_snow,
            play_sound: sound,
            water_services,
            tick_counter,
            map_max_x: max_xy,
            map_max_y: max_xy,
            current_company,
            company_valid,
            town_count,
            town_next_id,
            town_owner_by_id,
            town_by_id,
            town_owner,
            clear_square_services: crate::services::ClearSquareServices {
                docking: no_flag,
                generating_world: not_generating,
            },
            ..fixture
        }))
    }

    extern "C" fn trig(value: f32) -> f32 {
        value
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "MakeTree's uint count and density in bitfields"
    )]
    extern "C" fn make_tree(
        tile: u32,
        r#type: u8,
        count: u32,
        growth: u8,
        ground: u8,
        density: u32,
    ) {
        log(Event::Write("make_tree", tile));
        let ground = PLANTED_GROUND.get().unwrap_or(ground);
        edit(TileIndex(tile), |b| {
            set_type(b, TileType::MP_TREES);
            b.m2 = (u16::from(ground) << 6) | ((density as u16) << 4);
            b.m3 = r#type;
            b.m5 = ((count as u8) << 6) | growth;
        });
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "SetTreeGroundDensity's uint density in m2"
    )]
    extern "C" fn set_ground_density(tile: u32, ground: u8, density: u32) {
        log(Event::Write("ground", tile));
        edit(TileIndex(tile), |b| {
            b.m2 = (b.m2 & !0x1f0) | (u16::from(ground) << 6) | ((density as u16) << 4);
        });
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "AddTreeCount's m5() += c << 6 of an int"
    )]
    extern "C" fn add_count(tile: u32, c: i32) {
        log(Event::Write("count", tile));
        edit(TileIndex(tile), |b| {
            b.m5 = b.m5.wrapping_add(((c as u8) & 3) << 6)
        });
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "AddTreeGrowth's m5() += a of an int"
    )]
    extern "C" fn add_growth(tile: u32, a: i32) {
        log(Event::Write("add_growth", tile));
        edit(TileIndex(tile), |b| {
            b.m5 = (b.m5 & !7) | (b.m5.wrapping_add(a as u8) & 7);
        });
    }
    extern "C" fn set_growth(tile: u32, g: u8) {
        log(Event::Write("set_growth", tile));
        edit(TileIndex(tile), |b| b.m5 = (b.m5 & !7) | g);
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "MakeClear's uint density in m5"
    )]
    extern "C" fn make_clear(tile: u32, g: u8, density: u32) {
        log(Event::Write("clear", tile));
        clear(TileIndex(tile), ClearGround(g), density as u8);
    }
    extern "C" fn progress() -> bool {
        log(Event::Progress);
        PROGRESS.set(PROGRESS.get() + 1);
        PROGRESS.get() == ABORT_AT.get()
    }
    extern "C" fn progress_total(total: u32) -> bool {
        log(Event::Total(total));
        PROGRESS.set(PROGRESS.get() + 1);
        PROGRESS.get() == ABORT_AT.get()
    }
    const STRINGS: Strings = Strings {
        already_here: 101,
        on_water: 103,
        plant_limit: 102,
        site_unsuitable: 104,
        wrong_terrain: 105,
    };
    fn services() -> &'static TreeServices {
        Box::leak(Box::new(TreeServices {
            add_count,
            add_growth,
            authority: Box::leak(Box::new(crate::town::AuthorityServices {
                bulldozer,
                town_index,
                refusal: 0,
            })),
            cos: trig,
            make_clear,
            make_tree,
            progress,
            progress_total,
            set_ground_density,
            set_growth,
            sin: trig,
            strings: STRINGS,
            town: Box::leak(Box::new(crate::town::Leaves {
                dirty_authority,
                ..crate::test_tables::TOWN_LEAVES
            })),
        }))
    }
    fn trees() -> Trees<'static> {
        Trees {
            t: services(),
            s: shared(),
        }
    }

    extern "C" fn error(_: *mut CommandCost, message: u32) {
        log(Event::Error(message));
    }
    extern "C" fn expense_money(_: *mut CommandCost, expense: u8, money: i64) {
        log(Event::Expense(expense, money));
    }
    fn commands(s: &'static Services) -> &'static CommandServices {
        Box::leak(Box::new(CommandServices {
            finance: Box::leak(Box::new(crate::company::FinanceServices {
                current_company,
                company_finances,
                ..crate::test_tables::FINANCE
            })),
            cost: Box::leak(Box::new(crate::services::CommandCostServices {
                error,
                expense_money,
                ..crate::test_tables::COMMAND_COSTS
            })),
            shared: s,
            ..crate::test_tables::COMMANDS
        }))
    }
    /// Run `CmdPlantTree` from `start_tile` to `tile`.
    fn plant(
        c: &Trees<'static>,
        flags: DoCommandFlags,
        tile: TileIndex,
        start_tile: TileIndex,
        tree_to_plant: TreeType,
        diagonal: bool,
    ) {
        let commands = commands(c.s);
        let args = PlantTreeArgs {
            tile: tile.0,
            start_tile: start_tile.0,
            tree_to_plant: tree_to_plant.0,
            diagonal,
        };
        cmd_plant_tree(
            c,
            commands,
            flags,
            &args,
            Cost(std::ptr::null_mut(), commands.cost),
        );
    }
    /// Set `_price[PR_BUILD_TREES]` and `_price[PR_CLEAR_TREES]` for the guarded test.
    fn set_prices(build: i64, clear: i64) {
        let prices = crate::company::openttd_rust_economy_prices();
        // SAFETY: The process-wide price table, serialized by the global test lock.
        unsafe {
            *prices.add(PR_BUILD_TREES) = build;
            *prices.add(PR_CLEAR_TREES) = clear;
        }
    }
    /// Set `_game_mode`, returning the previous mode.
    fn set_mode(mode: GameMode) -> GameMode {
        // SAFETY: The canonical session state, serialized by the global test lock.
        unsafe {
            let state = crate::session::openttd_rust_session_storage();
            std::mem::replace(&mut (*state).mode, mode)
        }
    }
    fn finances(tree_limit: u32, clear_limit: u32) -> Box<crate::company::Finances> {
        // SAFETY: Every field of the finance record is an integer or an array of integers.
        let mut finances: Box<crate::company::Finances> = Box::new(unsafe { std::mem::zeroed() });
        finances.tree = tree_limit;
        finances.clear = clear_limit;
        finances
    }
    fn loop_settings(climate: LandscapeType, extra: ExtraTreePlacement) -> LoopSettings {
        LoopSettings {
            tick_counter: 0,
            size_x: 64,
            climate,
            extra,
            ambient: true,
            ambient_callback: false,
        }
    }

    #[test]
    fn counter_full_byte_and_scale_domain() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let counter = openttd_rust_tree_counter();
        // SAFETY: The process-wide counter, serialized by the global test lock.
        let saved = unsafe { counter.read() };
        // 64x64 scales by 1 and 512x512 by 4.
        for (log_x, scale) in [(6, 1_u32), (9, 4)] {
            let _map = install(log_x, TileType::MP_CLEAR, true);
            let w = World::new(shared());
            for old in 0..=u8::MAX {
                // SAFETY: As above.
                unsafe { counter.write(old) };
                let wrapped = decrement_tree_counter(&w);
                // SAFETY: As above.
                let next = unsafe { counter.read() };
                assert_eq!(u32::from(next), (u32::from(old).wrapping_sub(scale)) & 255);
                assert_eq!(wrapped, u32::from(old) < scale);
            }
        }
        initialize_trees();
        // SAFETY: As above.
        assert_eq!(unsafe { counter.read() }, 0);

        // The game thread and the GUI thread share one counter: spawn and join serialize each
        // access as the native game-state mutex does. Thread-local storage would fail each step.
        // SAFETY: As above.
        unsafe { counter.write(37) };
        let (address, seen) = std::thread::spawn(|| {
            let counter = openttd_rust_tree_counter();
            // SAFETY: As above; the parent does not access the counter until this join.
            let seen = unsafe { counter.read() };
            let _map = install(6, TileType::MP_CLEAR, true);
            assert!(!decrement_tree_counter(&World::new(shared())));
            (counter as usize, seen)
        })
        .join()
        .unwrap();
        assert_eq!((address, seen), (counter as usize, 37));
        // SAFETY: As above.
        assert_eq!(unsafe { counter.read() }, 36);
        std::thread::spawn(initialize_trees).join().unwrap();
        // SAFETY: As above.
        assert_eq!(unsafe { counter.read() }, 0);
        // SAFETY: As above.
        unsafe { counter.write(saved) };
    }

    #[test]
    fn tile_add_wrap_reads_freeform_edges() {
        let _map = install(6, TileType::MP_CLEAR, true);
        let w = World::new(shared());
        assert_eq!(tile_add_wrap(&w, TileIndex(65), -1, 0), TileIndex::INVALID);
        assert_eq!(tile_add_wrap(&w, TileIndex(65), 1, 1), TileIndex(130));
        assert_eq!(tile_add_wrap(&w, TileIndex(65), 62, 0), TileIndex::INVALID);
        FREEFORM.set(false);
        assert_eq!(tile_add_wrap(&w, TileIndex(65), -1, 0), TileIndex(64));
        FREEFORM.set(true);
    }

    #[test]
    fn tree_tables_match_the_original() {
        // table/tree_land.h
        assert_eq!(TREE_BASE_BY_LANDSCAPE, [0, 12, 20, 32]);
        assert_eq!(TREE_COUNT_BY_LANDSCAPE, [12, 8, 12, 9]);
        // tree_map.h
        assert_eq!(
            [
                TREE_COUNT_TEMPERATE,
                TREE_COUNT_SUB_ARCTIC,
                TREE_COUNT_RAINFOREST,
                TREE_COUNT_SUB_TROPICAL,
                TREE_COUNT_TOYLAND
            ],
            [12, 8, 7, 4, 9]
        );
        // town_type.h
        assert_eq!(
            [
                crate::town_type::RATING_TREE_DOWN_STEP,
                crate::town_type::RATING_TREE_MINIMUM,
                crate::town_type::RATING_TREE_UP_STEP,
                crate::town_type::RATING_TREE_MAXIMUM
            ],
            [-35, -1000, 7, 220]
        );
        // IsInsideBS over each landscape's window.
        assert!(is_inside_bs(20, 20, 12) && is_inside_bs(31, 20, 12));
        assert!(!is_inside_bs(19, 20, 12) && !is_inside_bs(32, 20, 12));
    }

    // Headless semantic scenarios cannot click Abort during map generation. The
    // aborting progress call is the last service: no draw, read or write follows it.
    #[test]
    fn generation_abort_returns_at_the_progress_call() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_TREES, true);
        ABORT_AT.set(1);
        assert!(place_trees_randomly(&trees()));
        assert_eq!(events(), [Event::Draw, Event::Progress]);

        PROGRESS.set(0);
        assert!(generate_trees(&trees()));
        assert_eq!(events(), [Event::Draw, Event::Total(6 * 63 + 2 * 1000)]);

        PROGRESS.set(0);
        ABORT_AT.set(2);
        assert!(generate_trees(&trees()));
        let mut expected = vec![Event::Draw, Event::Total(6 * 63 + 2 * 1000)];
        expected.extend([Event::Draw; 5]); // Centre and four phases.
        expected.push(Event::Progress);
        assert_eq!(events(), expected);

        // Tree tiles are unsuitable: each attempt draws and reports progress only.
        PROGRESS.set(0);
        ABORT_AT.set(3);
        assert!(place_trees_randomly(&trees()));
        assert_eq!(events(), [Event::Draw, Event::Progress].repeat(3));
        set_mode(saved);
    }

    #[test]
    fn quiet_temperate_tile_calls_no_service() {
        let _map = install(6, TileType::MP_CLEAR, true);
        tree(
            TileIndex(0),
            TreeType(0),
            1,
            TreeGrowthStage::Grown,
            TreeGround::TREE_GROUND_GRASS,
        );
        let c = trees();
        let s = loop_settings(LandscapeType::Temperate, ExtraTreePlacement::ETP_SPREAD_ALL);
        tile_loop_trees(
            &c,
            &World::new(c.s).types,
            TileIndex(0),
            &s,
            std::ptr::null_mut(),
        );
        assert_eq!(events(), []);
    }

    #[test]
    fn growth_bitpattern_seven_takes_the_default_arm() {
        let _map = install(6, TileType::MP_CLEAR, true);
        tree(
            TileIndex(2),
            TreeType(0),
            1,
            TreeGrowthStage(7),
            TreeGround::TREE_GROUND_GRASS,
        );
        let c = trees();
        let s = LoopSettings {
            tick_counter: 2304, // Tile 2: cycle 31.
            ..loop_settings(LandscapeType::Temperate, ExtraTreePlacement::ETP_NO_SPREAD)
        };
        tile_loop_trees(
            &c,
            &World::new(c.s).types,
            TileIndex(2),
            &s,
            std::ptr::null_mut(),
        );
        assert_eq!(events(), [Event::Write("add_growth", 2), Event::Dirty(2)]);
        assert_eq!(growth(TileIndex(2)), TreeGrowthStage::Growing1);
    }

    #[test]
    fn arctic_loop_uses_configured_snow_line() {
        let _map = install(6, TileType::MP_CLEAR, true);
        for tile in 0..4096 {
            edit(TileIndex(tile), |b| b.height = 8);
        }
        tree(
            TileIndex(65),
            TreeType(12),
            1,
            TreeGrowthStage::Growing1,
            TreeGround::TREE_GROUND_SNOW_DESERT,
        );
        RANDOM.set(0x8000_0000);
        let c = trees();
        let s = loop_settings(LandscapeType::Arctic, ExtraTreePlacement::ETP_SPREAD_ALL);
        // Height 8 above snow line 4 is full density: the snow sound is the only effect.
        tile_loop_trees(
            &c,
            &World::new(c.s).types,
            TileIndex(65),
            &s,
            std::ptr::null_mut(),
        );
        assert_eq!(
            events(),
            [
                Event::Draw,
                Event::Sound(u16::from(SoundFx::SND_39_ARCTIC_SNOW_2.0))
            ]
        );
        // Below the snow line, snow turns back into grass.
        SNOW_LINE.set(10);
        tile_loop_trees(
            &c,
            &World::new(c.s).types,
            TileIndex(65),
            &s,
            std::ptr::null_mut(),
        );
        assert_eq!(events(), [Event::Write("ground", 65), Event::Dirty(65)]);
        assert_eq!(tree_ground(TileIndex(65)), TreeGround::TREE_GROUND_GRASS);
        SNOW_LINE.set(4);
    }

    // Script APIs cannot request explicit tree types, diagonal rectangles, or editor
    // planting. These tests cover those source branches; end-to-end command evidence is
    // the simulation harness.
    #[test]
    fn explicit_editor_planting_and_diagonal_iterator_request() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode::GM_EDITOR);
        set_prices(15, 0);
        let _map = install(6, TileType::MP_VOID, true);
        clear(TileIndex(1), ClearGround::CLEAR_GRASS, 3);
        clear(TileIndex(2), ClearGround::CLEAR_GRASS, 3);
        CLIMATE.set(LandscapeType::Tropic.0);
        // The diagonal from corner 2 to corner 1 starts at 2; the orthogonal area would
        // start at 1.
        plant(
            &trees(),
            DoCommandFlags(DoCommandFlag::EXECUTE),
            TileIndex(2),
            TileIndex(1),
            TreeType(20),
            true,
        );
        let rainforest = TropicZone::TROPICZONE_RAINFOREST.0;
        assert_eq!(
            events(),
            [
                Event::Write("make_tree", 2),
                Event::Dirty(2),
                Event::Zone(2, rainforest),
                Event::Write("make_tree", 1),
                Event::Dirty(1),
                Event::Zone(1, rainforest),
                Event::Expense(ExpensesType::EXPENSES_OTHER.0, 30),
            ]
        );
        // Full grown trees in the scenario editor.
        assert_eq!(growth(TileIndex(1)), TreeGrowthStage::Grown);
        CLIMATE.set(0);
        set_mode(saved);
    }

    #[test]
    fn explicit_tree_type_checks_and_wrong_tropical_zone() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        clear(TileIndex(1), ClearGround::CLEAR_GRASS, 3);
        // Rainforest is outside the temperate range.
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(1),
            TileIndex(1),
            TreeType(20),
            false,
        );
        // Start tile outside the map.
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(2),
            TileIndex(4096),
            TreeType(255),
            false,
        );
        CLIMATE.set(LandscapeType::Tropic.0);
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(1),
            TileIndex(1),
            TreeType(20),
            false,
        );
        assert_eq!(
            events(),
            [
                Event::Error(StringID::INVALID_STRING_ID.0),
                Event::Error(StringID::INVALID_STRING_ID.0),
                Event::Error(STRINGS.wrong_terrain),
            ]
        );
        CLIMATE.set(0);
        set_mode(saved);
    }

    #[test]
    fn non_coast_water_reports_water_before_bridge_and_zone() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        // Plain water under a bridge, in the desert, for a rainforest tree.
        edit(TileIndex(1), |b| {
            b.kind = (TileType::MP_WATER.0 << 4) | 0x04 | TropicZone::TROPICZONE_DESERT.0;
        });
        CLIMATE.set(LandscapeType::Tropic.0);
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(1),
            TileIndex(1),
            TreeType(20),
            false,
        );
        assert_eq!(events(), [Event::Error(STRINGS.on_water)]);
        CLIMATE.set(0);
        set_mode(saved);
    }

    #[test]
    fn exhausted_limit_checks_zero_and_negative_separately() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        let mut company = finances(1 << 16, 0);
        COMPANY.set(&raw mut *company);
        clear(TileIndex(1), ClearGround::CLEAR_GRASS, 3);
        tree(
            TileIndex(2),
            TreeType(0),
            4,
            TreeGrowthStage::Grown,
            TreeGround::TREE_GROUND_GRASS,
        );
        clear(TileIndex(3), ClearGround::CLEAR_GRASS, 3);
        tree(
            TileIndex(4),
            TreeType(0),
            4,
            TreeGrowthStage::Grown,
            TreeGround::TREE_GROUND_GRASS,
        );
        // A limit of zero is reached but not exceeded: tile 2 is still visited.
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(2),
            TileIndex(1),
            TreeType(255),
            false,
        );
        assert_eq!(events(), [Event::Error(STRINGS.already_here)]);
        // A negative limit stops before tile 4.
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(4),
            TileIndex(1),
            TreeType(255),
            false,
        );
        assert_eq!(events(), [Event::Error(STRINGS.plant_limit)]);
        COMPANY.set(std::ptr::null_mut());
        set_mode(saved);
    }

    #[test]
    fn tree_limit_is_debited_per_planted_tile_under_execute() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        set_prices(15, 0);
        let _map = install(6, TileType::MP_VOID, true);
        for tile in 1..=3 {
            clear(TileIndex(tile), ClearGround::CLEAR_GRASS, 3);
        }
        // `--limit < 1` leaves a limit of 3 room for two plantings.
        let mut company = finances(3 << 16, 0);
        COMPANY.set(&raw mut *company);
        plant(
            &trees(),
            DoCommandFlags(0),
            TileIndex(3),
            TileIndex(1),
            TreeType(0),
            false,
        );
        assert_eq!(
            events(),
            [Event::Expense(ExpensesType::EXPENSES_OTHER.0, 30)]
        );
        assert_eq!(company.tree, 3 << 16);
        plant(
            &trees(),
            DoCommandFlags(DoCommandFlag::EXECUTE),
            TileIndex(3),
            TileIndex(1),
            TreeType(0),
            false,
        );
        assert_eq!(
            events(),
            [
                Event::Write("make_tree", 1),
                Event::Dirty(1),
                Event::Write("make_tree", 2),
                Event::Dirty(2),
                Event::Expense(ExpensesType::EXPENSES_OTHER.0, 30),
            ]
        );
        assert_eq!(company.tree, 1 << 16);
        plant(
            &trees(),
            DoCommandFlags(DoCommandFlag::EXECUTE),
            TileIndex(3),
            TileIndex(3),
            TreeType(0),
            false,
        );
        assert_eq!(events(), [Event::Error(STRINGS.plant_limit)]);
        COMPANY.set(std::ptr::null_mut());
        set_mode(saved);
    }

    #[test]
    fn failed_nested_clear_is_the_result_and_debits_nothing() {
        extern "C" fn landscape() -> *const crate::landscape_cmd::LandscapeServices {
            thread_local! { static LANDSCAPE: crate::landscape_cmd::LandscapeServices =
                crate::landscape_cmd::LandscapeServices {
                    shared: shared(),
                    costs: Box::leak(Box::new(crate::services::CommandCostServices {
                        error,
                        expense_money,
                        success,
                        ..crate::test_tables::COMMAND_COSTS
                    })),
                    commands: COMMANDS.get(),
                    strings: crate::landscape_cmd::Strings {
                        clear_limit: 201,
                        ..crate::test_tables::LANDSCAPE.strings
                    },
                    ..crate::test_tables::LANDSCAPE
                };
            }
            LANDSCAPE.with(std::ptr::from_ref)
        }
        extern "C" fn success(_: *const CommandCost) -> bool {
            !LOG.with_borrow(|l| l.iter().any(|e| matches!(e, Event::Error(_))))
        }
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        clear(TileIndex(1), ClearGround::CLEAR_FIELDS, 3);
        // The tree limit allows planting; the clear limit fails the nested clear.
        let mut company = finances(5 << 16, 0);
        COMPANY.set(&raw mut *company);
        let c = trees();
        let commands: &'static CommandServices = Box::leak(Box::new(CommandServices {
            landscape_commands: landscape,
            cost: Box::leak(Box::new(crate::services::CommandCostServices {
                error,
                expense_money,
                success,
                ..crate::test_tables::COMMAND_COSTS
            })),
            ..*commands(c.s)
        }));
        COMMANDS.set(commands);
        let args = PlantTreeArgs {
            tile: 1,
            start_tile: 1,
            tree_to_plant: 255,
            diagonal: false,
        };
        cmd_plant_tree(
            &c,
            commands,
            DoCommandFlags(DoCommandFlag::EXECUTE),
            &args,
            Cost(std::ptr::null_mut(), commands.cost),
        );
        // The clear's own cost reset, then its error: nothing follows it.
        assert_eq!(
            events(),
            [
                Event::Expense(crate::economy_type::EXPENSES_CONSTRUCTION, 0),
                Event::Error(201)
            ]
        );
        assert_eq!(company.tree, 5 << 16);
        COMPANY.set(std::ptr::null_mut());
        set_mode(saved);
    }

    #[test]
    fn town_rating_steps_and_limits_of_planting_and_clearing() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        set_prices(15, 4);
        let _map = install(6, TileType::MP_VOID, true);
        let mut town = Box::new(crate::town::State::default());
        town.simulation.xy = 65;
        TOWN.set(&raw mut *town);
        COMPANY_VALID.set(true);
        let c = trees();
        crate::town::spatial::rebuild_town_kdtree(c.s);
        clear(TileIndex(66), ClearGround::CLEAR_GRASS, 3);
        // RATING_TREE_UP_STEP, up to RATING_TREE_MAXIMUM.
        town.simulation.ratings[0] = 210;
        plant(
            &c,
            DoCommandFlags(0),
            TileIndex(66),
            TileIndex(66),
            TreeType(255),
            false,
        );
        assert_eq!(town.simulation.ratings[0], 217);
        plant(
            &c,
            DoCommandFlags(0),
            TileIndex(66),
            TileIndex(66),
            TreeType(255),
            false,
        );
        assert_eq!(town.simulation.ratings[0], 220);
        // RATING_TREE_DOWN_STEP, down to RATING_TREE_MINIMUM.
        tree(
            TileIndex(67),
            TreeType(20),
            2,
            TreeGrowthStage::Grown,
            TreeGround::TREE_GROUND_GRASS,
        );
        town.simulation.ratings[0] = -960;
        // Two rainforest trees cost four times each.
        assert_eq!(
            clear_tile_trees(&c, TileIndex(67), DoCommandFlags(0)),
            2 * 4 * 4
        );
        assert_eq!(town.simulation.ratings[0], -995);
        clear_tile_trees(&c, TileIndex(67), DoCommandFlags(0));
        assert_eq!(town.simulation.ratings[0], -1000);
        let log = events();
        assert_eq!(log.iter().filter(|e| **e == Event::Authority(0)).count(), 4);
        TOWN.set(std::ptr::null_mut());
        COMPANY_VALID.set(false);
        crate::town::spatial::rebuild_town_kdtree(c.s);
        set_mode(saved);
    }

    #[test]
    fn money_extremes_keep_original_saturation_order() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        for price in [i64::MIN, i64::MAX] {
            set_prices(price, price);
            for tile in 1..=3 {
                tree(
                    TileIndex(tile),
                    TreeType(20),
                    1,
                    TreeGrowthStage::Grown,
                    TreeGround::TREE_GROUND_GRASS,
                );
            }
            plant(
                &trees(),
                DoCommandFlags(0),
                TileIndex(3),
                TileIndex(1),
                TreeType(255),
                false,
            );
            assert_eq!(
                events(),
                [Event::Expense(ExpensesType::EXPENSES_OTHER.0, price)]
            );
            tree(
                TileIndex(1),
                TreeType(20),
                4,
                TreeGrowthStage::Grown,
                TreeGround::TREE_GROUND_GRASS,
            );
            let cost = clear_tile_trees(
                &trees(),
                TileIndex(1),
                DoCommandFlags(DoCommandFlag::EXECUTE),
            );
            assert_eq!(cost, price);
            assert_eq!(events(), [Event::Write("clear_square", 1), Event::Dirty(1)]);
        }
        set_mode(saved);
    }

    #[test]
    fn planting_an_unsuitable_tile_asserts_once_where_the_build_asserts() {
        for assertions in [true, false] {
            let _map = install(6, TileType::MP_VOID, assertions);
            clear(TileIndex(1), ClearGround::CLEAR_FIELDS, 2);
            let c = trees();
            let w = World::new(c.s);
            plant_trees_on_tile(
                &c,
                &w,
                TileIndex(1),
                TreeType(0),
                0,
                TreeGrowthStage::Growing1,
            );
            let mut expected = vec![Event::Write("make_tree", 1)];
            if assertions {
                expected.insert(0, Event::Assert);
            }
            assert_eq!(events(), expected);
            // Fields are neither grass nor rough: snow-or-desert ground keeps the density.
            assert_eq!(
                tree_ground(TileIndex(1)),
                TreeGround::TREE_GROUND_SNOW_DESERT
            );
        }
    }

    /// Run `body` in a child test process whose `NOT_REACHED` exits with status 73, and
    /// check that it does.
    fn takes_native_unreachable(name: &str, body: fn()) {
        const CHILD: &str = "OPENTTD_TREES_UNREACHABLE_CHILD";
        extern "C" fn exit_not_reached() {
            std::process::exit(73);
        }
        if std::env::var(CHILD).ok().as_deref() == Some(name) {
            let shared: &'static Services = Box::leak(Box::new(Services {
                not_reached: exit_not_reached,
                ..*shared()
            }));
            // SAFETY: The leaked table lives for the child process.
            unsafe { crate::services_assert::openttd_rust_assert_bind(record_assert, shared) };
            body();
            std::process::exit(0);
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name])
            .env(CHILD, name)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73), "{name}");
    }

    #[test]
    fn planting_a_tile_of_another_type_is_not_reached() {
        takes_native_unreachable(
            "trees::tests::planting_a_tile_of_another_type_is_not_reached",
            || {
                let _map = install(6, TileType::MP_TREES, true);
                let c = trees();
                let w = World::new(c.s);
                plant_trees_on_tile(
                    &c,
                    &w,
                    TileIndex(1),
                    TreeType(0),
                    0,
                    TreeGrowthStage::Growing1,
                );
            },
        );
    }

    #[test]
    fn an_unknown_tree_placer_is_not_reached() {
        takes_native_unreachable(
            "trees::tests::an_unknown_tree_placer_is_not_reached",
            || {
                let _map = install(6, TileType::MP_TREES, true);
                PLACER.set(3);
                generate_trees(&trees());
            },
        );
    }

    #[test]
    fn place_tree_rerandomizes_from_the_ground_read_back() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let saved = set_mode(GameMode(1));
        let _map = install(6, TileType::MP_VOID, true);
        let c = trees();
        // Seed 0 plants temperate type 0; bit 28 chooses rough ground.
        let r = 1 << 28;
        clear(TileIndex(1), ClearGround::CLEAR_GRASS, 3);
        place_tree(&c, &World::new(c.s), TileIndex(1), r, false);
        assert_eq!(
            events(),
            [
                Event::Write("make_tree", 1),
                Event::Dirty(1),
                Event::Write("ground", 1)
            ]
        );
        assert_eq!(tree_ground(TileIndex(1)), TreeGround::TREE_GROUND_ROUGH);
        // A grass tile whose planted ground reads back as shore keeps it.
        clear(TileIndex(1), ClearGround::CLEAR_GRASS, 3);
        PLANTED_GROUND.set(Some(TreeGround::TREE_GROUND_SHORE.0));
        place_tree(&c, &World::new(c.s), TileIndex(1), r, false);
        assert_eq!(events(), [Event::Write("make_tree", 1), Event::Dirty(1)]);
        set_mode(saved);
    }
}

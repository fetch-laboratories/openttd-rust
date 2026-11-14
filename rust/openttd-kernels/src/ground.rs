/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ground evolution of clear, railway, road and tunnel/bridge tiles: `ClearTile_Clear`,
//! `TileLoop_Clear` and `CheckBuildAbove_Clear` (`clear_cmd.cpp`), `DoClearSquare`
//! (`landscape.cpp`), `TileLoop_Track` (`rail_cmd.cpp`), `TileLoop_Road` (`road_cmd.cpp`) and
//! `TileLoop_TunnelBridge` (`tunnelbridge_cmd.cpp`).
//!
//! Map storage stays native. Bodies read it in place through the map descriptor and write it
//! through the native leaves of the tables below. Every table is immutable and lives for the
//! process; no Rust borrow of map or owner state spans a leaf call, and leaves do not unwind.
#![allow(unsafe_code)]
use crate::clear_map::{
    ClearGround, add_clear_counter, add_clear_density, get_clear_counter, get_clear_density,
    get_clear_ground, get_fence, get_field_type, get_industry_index_of_field, is_clear_ground,
    is_snow_tile, require_field, set_clear_ground_density,
};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::direction_type::{DiagDirection, DiagDirections};
use crate::economy_type::Price;
use crate::house::HouseZone;
use crate::industry_type::IndustryID;
use crate::map_func::World;
use crate::services::{
    Foundation, FoundationSlope, LandscapeType, RailGroundType, RoadBits, RoadType, Roadside,
    Services, Slope, TrackBits, WaterClass,
};
use crate::slope_func::{is_slope_with_three_corners_raised, is_steep_slope};
use crate::tile_type::{TileIndex, TileType, TropicZone};

/// `CheckBuildAbove_Clear`: can always build above clear tiles.
pub(crate) fn check_build_above_clear(out: crate::services::Cost<'_>) {
    /* Can always build above clear tiles. */
    out.zero();
}

/// `ClearTile_Clear`. The native result keeps `AddCost`'s sequential saturation.
pub(crate) fn clear_tile_clear(
    out: crate::services::Cost<'_>,
    tile: TileIndex,
    flags: DoCommandFlags,
    s: &Services,
) {
    const CLEAR_PRICE_TABLE: [Price; 6] = [
        Price::PR_CLEAR_GRASS,
        Price::PR_CLEAR_ROUGH,
        Price::PR_CLEAR_ROCKS,
        Price::PR_CLEAR_FIELDS,
        Price::PR_CLEAR_ROUGH,
        Price::PR_CLEAR_ROUGH,
    ];
    let price = |p: Price| crate::company::price(usize::from(p.0));
    out.set(0);

    let world = World::new(s);
    let ground = world.asserted(|m| get_clear_ground(m, tile));
    let density = world.asserted(|m| get_clear_density(m, tile));
    // `clear_price_table[ground]` reads past the table for the grounds 6 and 7, which no map
    // holds; the checked index stops there instead.
    if world.asserted(|m| is_snow_tile(m, tile)) {
        out.add_money(price(CLEAR_PRICE_TABLE[usize::from(ground.0)]));
        /* Add a little more for removing snow. */
        // Money subtraction saturates before conversion to int64 for std::abs.
        out.add_money(
            price(Price::PR_CLEAR_ROUGH)
                .saturating_sub(price(Price::PR_CLEAR_GRASS))
                .wrapping_abs(),
        );
    } else if ground != ClearGround::CLEAR_GRASS || density != 0 {
        out.add_money(price(CLEAR_PRICE_TABLE[usize::from(ground.0)]));
    }

    if flags.test(DoCommandFlag::Execute) {
        do_clear_square(tile, s);
    }
}

/// # Safety
/// Result storage and immutable services remain live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_clear_tile(
    out: *mut crate::services::CommandCost,
    tile: u32,
    flags: u16,
    costs: &crate::services::CommandCostServices,
    s: &Services,
) {
    clear_tile_clear(
        crate::services::Cost(out, costs),
        TileIndex(tile),
        DoCommandFlags(flags),
        s,
    );
}

/// `DoClearSquare`.
pub(crate) fn do_clear_square(tile: TileIndex, s: &Services) {
    /* If the tile can have animation and we clear it, delete it from the animated tile list. */
    if crate::animated_tile::may_animate(tile.0, s) {
        crate::animated_tile::openttd_rust_animation_delete(tile.0, true);
    }

    let remove = (s.clear_square_services.docking)(tile.0);
    // SAFETY: the native getter supplies immutable process-lifetime service tables.
    let water = unsafe { &*(s.water_services)() };
    // SAFETY: as above.
    let clear = unsafe { &*water.clear };
    (clear.make_clear)(
        tile.0,
        ClearGround::CLEAR_GRASS,
        if crate::genworld::generating() { 3 } else { 0 },
    );
    (s.mark_dirty)(tile.0);
    if remove {
        // SAFETY: the native getter supplies the process-lifetime station table.
        unsafe { crate::station_cmd::remove_docking_tile((s.station_commands)(), tile) };
    }

    // SAFETY: the tables address the current serialized map; no region borrow crosses a leaf.
    unsafe {
        crate::water::clear_neighbour_non_flooding_states(
            tile,
            (s.map_max_x)() + 1,
            (s.map_max_y)() + 1,
            water,
            s,
        );
        crate::water_regions::invalidate_water_region(&mut *(water.water_regions)(), s, tile);
    }
}
/// Raw adapter of [`do_clear_square`] for callers in files no pass has finished.
pub(crate) fn do_clear_square_raw(tile: u32, s: &Services) {
    do_clear_square(TileIndex(tile), s);
}

/// # Safety
/// The table and its native map services stay live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_clear_square(tile: u32, s: *const Services) {
    // SAFETY: the caller's contract.
    do_clear_square(TileIndex(tile), unsafe { &*s });
}

/// Landscape branch witnesses, also hit by the water and station loops.
#[repr(usize)]
pub(crate) enum Witness {
    SnowStart,
    SnowChange,
    SnowClear,
    Desert,
    FieldFence,
    FieldCycle,
    FieldOrphan,
    RailGround,
    Roadside,
    RoadWorkStart,
    RoadWorkEnd,
    RoadRemove,
    TunnelGround,
    FloodSuccess,
    FloodFail,
    DryCoast,
    DryRail,
    DryTree,
    Wakeup,
    HalfRailRemove,
    HalfRail,
    BridgeStart,
    BridgeEnd,
    AirportMember,
    VehicleCrash,
    CrashedSkip,
}
#[inline]
pub(crate) fn hit(branch: Witness) {
    crate::witness::hit(crate::witness::LANDSCAPE + branch as usize);
}

/// Native leaves of `clear_map.h` writes.
#[repr(C)]
pub struct ClearServices {
    /// `SetClearGroundDensity`.
    pub set_ground: extern "C" fn(u32, ClearGround, u32),
    /// `SetClearCounter`.
    pub set_counter: extern "C" fn(u32, u32),
    /// `ClearSnow`.
    pub clear_snow: extern "C" fn(u32),
    /// `MakeClear`.
    pub make_clear: extern "C" fn(u32, ClearGround, u32),
    /// `SetFieldType`.
    pub set_field: extern "C" fn(u32, u32),
}
/// Native railway map leaves.
#[repr(C)]
pub struct RailServices {
    /// `GetTrackBits`.
    pub tracks: extern "C" fn(u32) -> TrackBits,
    /// `GetRailGroundType`.
    pub ground: extern "C" fn(u32) -> RailGroundType,
    /// `SetRailGroundType`.
    pub set_ground: extern "C" fn(u32, RailGroundType),
    /// `IsObjectType(t, OBJECT_OWNED_LAND)`.
    pub owned_land: extern "C" fn(u32) -> bool,
}
/// Native tunnel/bridge map leaves.
#[repr(C)]
pub struct TunnelServices {
    /// `HasTunnelBridgeSnowOrDesert`.
    pub snow_or_desert: extern "C" fn(u32) -> bool,
    /// `SetTunnelBridgeSnowOrDesert`.
    pub set_snow_or_desert: extern "C" fn(u32, bool),
}
#[repr(C)]
pub struct Call {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CostServices {
    pub commands: *const crate::services::CommandServices,
    pub costs: *const crate::services::CommandCostServices,
    // Caller provides the facade's live context; release follows its last result access.
    pub prepare: unsafe extern "C" fn(*mut Call) -> *mut crate::services::CommandCost,
    pub release: unsafe extern "C" fn(*mut Call),
}
/// Native road map leaves and the town table.
#[repr(C)]
pub struct RoadServices {
    /// `IsOnSnowOrDesert`.
    pub snow_or_desert: extern "C" fn(u32) -> bool,
    /// `ToggleSnowOrDesert`.
    pub toggle_snow_or_desert: extern "C" fn(u32),
    /// `GetRoadside`.
    pub roadside: extern "C" fn(u32) -> Roadside,
    /// `SetRoadside`.
    pub set_roadside: extern "C" fn(u32, Roadside),
    /// `GetAllRoadBits`.
    pub all_bits: extern "C" fn(u32) -> RoadBits,
    pub town: *const crate::town::Leaves,
}

/// The tile loop batch's inputs for the ground tile types, built by `tile_loop`'s dispatch.
pub(crate) struct TileLoop<'a> {
    /// The batch's map descriptor.
    pub world: World<'a>,
    /// `Map::SizeY()`.
    pub size_y: u32,
    /// The batch's ground tables and result context.
    pub ground: &'a crate::tile_loop::GroundServices,
    /// `_settings_game.game_creation.landscape`.
    pub landscape: LandscapeType,
    /// `_game_mode == GM_EDITOR`.
    pub game_mode_editor: bool,
    /// `_settings_client.sound.ambient`.
    pub sound_ambient: bool,
    /// The batch's `AmbientSoundEffect` callback gate.
    pub ambient_callback: bool,
    /// `_settings_game.economy.mod_road_rebuild`.
    pub mod_road_rebuild: bool,
}
impl TileLoop<'_> {
    fn s(&self) -> &Services {
        self.world.shared
    }
    fn clear(&self) -> &ClearServices {
        // SAFETY: the batch's table is immutable and lives for the process.
        unsafe { &*self.ground.clear }
    }
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| self.world.check(ok)
    }
    /// `TileIndex + TileOffsByDiagDir(dir)`.
    fn neighbour(&self, tile: TileIndex, dir: DiagDirection) -> TileIndex {
        tile + self
            .world
            .dimensions
            .tile_offs_by_diag_dir(dir, self.check())
    }
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.s().mark_dirty)(tile.0);
    }
    fn get_snow_line(&self) -> i32 {
        i32::from(crate::snow_line::get(self.s()))
    }
    fn get_tropic_zone(&self, tile: TileIndex) -> TropicZone {
        self.world
            .asserted(|m| crate::tile_map::get_tropic_zone(m, tile))
    }
}

/// `UpdateFences`.
#[allow(
    clippy::inline_always,
    reason = "the clear tile loop's measured hot path"
)]
#[inline(always)]
fn update_fences(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    w.assert_with(|| {
        w.kind(tile) == TileType::MP_CLEAR
            && w.asserted(|m| is_clear_ground(m, tile, ClearGround::CLEAR_FIELDS))
    });
    let mut dirty = false;

    for dir in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
        let dir = DiagDirection(dir);
        if w.asserted(|m| {
            Some(get_fence(&require_field(m, tile)?, dir, || {
                crate::services_assert::not_reached()
            }))
        }) != 0
        {
            continue;
        }
        let neighbour = c.neighbour(tile, dir);
        if w.kind(neighbour) == TileType::MP_CLEAR
            && w.asserted(|m| is_clear_ground(m, neighbour, ClearGround::CLEAR_FIELDS))
        {
            continue;
        }
        (c.s().set_fence)(tile.0, dir, 3);
        dirty = true;
    }

    if dirty {
        c.mark_tile_dirty_by_tile(tile);
        hit(Witness::FieldFence);
    }
}

/// `TileLoopClearAlps`: convert to or from snowy tiles.
#[inline]
fn tile_loop_clear_alps(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    let k = w.tile_z(tile) - c.get_snow_line() + 1;

    if !w.asserted(|m| is_snow_tile(m, tile)) {
        /* Below the snow line, do nothing if no snow. */
        /* At or above the snow line, make snow tile if needed. */
        if k >= 0 {
            /* Snow density is started at 0 so that it can gradually reach the required density. */
            (c.s().make_snow)(tile.0, 0);
            hit(Witness::SnowStart);
            c.mark_tile_dirty_by_tile(tile);
        }
        return;
    }

    /* Update snow density. */
    let current_density = u32::from(w.asserted(|m| get_clear_density(m, tile)));
    let req_density = if k < 0 { 0 } else { k.cast_unsigned().min(3) };

    if current_density == req_density {
        /* Density at the required level. */
        if k >= 0 {
            return;
        }
        (c.clear().clear_snow)(tile.0);
        hit(Witness::SnowClear);
    } else {
        let d = if current_density < req_density { 1 } else { -1 };
        w.asserted(|m| add_clear_density(m, tile, d));
        hit(Witness::SnowChange);
    }

    c.mark_tile_dirty_by_tile(tile);
}

/// `NeighbourIsNormal`: tests if at least one surrounding tile is non-desert.
#[inline]
fn neighbour_is_normal(c: &TileLoop<'_>, tile: TileIndex) -> bool {
    let w = &c.world;
    for dir in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
        let t = c.neighbour(tile, DiagDirection(dir));
        if !w.is_valid_tile(t) {
            continue;
        }
        if c.get_tropic_zone(t) != TropicZone::TROPICZONE_DESERT {
            return true;
        }
        if w.asserted(|m| crate::water_map::has_tile_water_class(m, t.0))
            && WaterClass(w.asserted(|m| crate::water_map::get_water_class(m, t.0)))
                == WaterClass::Sea
        {
            return true;
        }
    }
    false
}

/// `TileLoopClearDesert`.
#[inline]
fn tile_loop_clear_desert(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    /* Current desert level - 0 if it is not desert */
    let mut current = 0;
    if w.asserted(|m| is_clear_ground(m, tile, ClearGround::CLEAR_DESERT)) {
        current = u32::from(w.asserted(|m| get_clear_density(m, tile)));
    }

    /* Expected desert level - 0 if it shouldn't be desert */
    let mut expected = 0;
    if c.get_tropic_zone(tile) == TropicZone::TROPICZONE_DESERT {
        expected = if neighbour_is_normal(c, tile) { 1 } else { 3 };
    }

    if current == expected {
        return;
    }

    if expected == 0 {
        w.asserted(|m| set_clear_ground_density(m, tile, ClearGround::CLEAR_GRASS, 3));
    } else {
        /* Transition from clear to desert is not smooth (after clearing desert tile) */
        w.asserted(|m| set_clear_ground_density(m, tile, ClearGround::CLEAR_DESERT, expected));
    }

    c.mark_tile_dirty_by_tile(tile);
    hit(Witness::Desert);
}

/// `TileLoop_Clear`.
#[allow(
    clippy::inline_always,
    reason = "the clear tile loop's measured hot path"
)]
#[inline(always)]
pub(crate) fn tile_loop_clear(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    crate::newgrf_generic::ambient(c.ambient_callback, tile.0);

    match c.landscape {
        LandscapeType::Tropic => tile_loop_clear_desert(c, tile),
        LandscapeType::Arctic => tile_loop_clear_alps(c, tile),
        _ => {}
    }

    if w.asserted(|m| is_snow_tile(m, tile)) {
        return;
    }

    match w.asserted(|m| get_clear_ground(m, tile)) {
        ClearGround::CLEAR_GRASS => {
            if w.asserted(|m| get_clear_density(m, tile)) == 3 {
                return;
            }

            if c.game_mode_editor {
                let random = c.s().random();
                w.asserted(|m| {
                    set_clear_ground_density(
                        m,
                        tile,
                        if crate::bitmath_func::gb(random, 0, 8) > 21 {
                            ClearGround::CLEAR_GRASS
                        } else {
                            ClearGround::CLEAR_ROUGH
                        },
                        3,
                    )
                });
            } else {
                if w.asserted(|m| get_clear_counter(m, tile)) < 7 {
                    w.asserted(|m| add_clear_counter(m, tile, 1));
                    return;
                }
                (c.clear().set_counter)(tile.0, 0);
                w.asserted(|m| add_clear_density(m, tile, 1));
            }
        }

        ClearGround::CLEAR_FIELDS => {
            update_fences(c, tile);

            if c.game_mode_editor {
                return;
            }

            if w.asserted(|m| get_clear_counter(m, tile)) < 7 {
                w.asserted(|m| add_clear_counter(m, tile, 1));
                return;
            }
            (c.clear().set_counter)(tile.0, 0);

            if w.asserted(|m| Some(get_industry_index_of_field(&require_field(m, tile)?)))
                == IndustryID::INVALID
                && w.asserted(|m| Some(get_field_type(&require_field(m, tile)?))) >= 7
            {
                /* This farmfield is no longer farmfield, so make it grass again */
                (c.clear().make_clear)(tile.0, ClearGround::CLEAR_GRASS, 2);
                hit(Witness::FieldOrphan);
            } else {
                let mut field_type = w.asserted(|m| Some(get_field_type(&require_field(m, tile)?)));
                field_type = if field_type < 8 { field_type + 1 } else { 0 };
                hit(Witness::FieldCycle);
                (c.clear().set_field)(tile.0, field_type);
            }
        }

        _ => return,
    }

    c.mark_tile_dirty_by_tile(tile);
}

/// `TileLoop_Track`.
pub(crate) fn tile_loop_track(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    // SAFETY: the batch's table is immutable and lives for the process.
    let r = unsafe { &*c.ground.rail };
    let old_ground = RailGroundType(w.rail_ground(tile.0));

    if old_ground == RailGroundType::HalfTileWater {
        // SAFETY: the batch owns an independent native result context; no result is live
        // before this early return.
        unsafe {
            crate::water::water_tile_loop(
                tile.0,
                TileType::MP_RAILWAY.0,
                false,
                w.dimensions.size_x(),
                c.size_y,
                c.ground.call,
                c.ground.water,
                c.s(),
                &w.types,
            );
        }
        return;
    }

    let new_ground = 'set_ground: {
        match c.landscape {
            LandscapeType::Arctic => {
                let (slope, mut z) = w.tile_slope_z(tile);
                let mut half = false;

                /* for non-flat track, use lower part of track
                 * in other cases, use the highest part with track */
                if w.is_plain_rail(tile.0) {
                    let track = TrackBits(w.tracks(tile.0));
                    let f = crate::movement::get_rail_foundation(slope, track, c.check());

                    match f {
                        Foundation::FOUNDATION_NONE => {
                            /* no foundation - is the track on the upper side of three corners raised tile? */
                            if is_slope_with_three_corners_raised(slope) {
                                z += 1;
                            }
                        }

                        Foundation::FOUNDATION_INCLINED_X | Foundation::FOUNDATION_INCLINED_Y => {
                            /* sloped track - is it on a steep slope? */
                            if is_steep_slope(slope) {
                                z += 1;
                            }
                        }

                        Foundation::FOUNDATION_STEEP_LOWER => {
                            /* only lower part of steep slope */
                            z += 1;
                        }

                        _ => {
                            /* if it is a steep slope, then there is a track on higher part */
                            if is_steep_slope(slope) {
                                z += 1;
                            }
                            z += 1;
                        }
                    }

                    half = crate::math::is_inside_mm(
                        usize::from(f.0),
                        usize::from(Foundation::FOUNDATION_STEEP_BOTH.0),
                        usize::from(Foundation::FOUNDATION_HALFTILE_N.0) + 1,
                    );
                } else {
                    /* is the depot on a non-flat tile? */
                    if slope != Slope::SLOPE_FLAT {
                        z += 1;
                    }
                }

                /* 'z' is now the lowest part of the highest track bit -
                 * for sloped track, it is 'z' of lower part
                 * for two track bits, it is 'z' of higher track bit
                 * For non-continuous foundations (and STEEP_BOTH), 'half' is set */
                if z > c.get_snow_line() {
                    if half && z - c.get_snow_line() == 1 {
                        /* track on non-continuous foundation, lower part is not under snow */
                        break 'set_ground RailGroundType::HalfTileSnow;
                    }
                    break 'set_ground RailGroundType::SnowOrDesert;
                }
            }

            LandscapeType::Tropic => {
                if c.get_tropic_zone(tile) == TropicZone::TROPICZONE_DESERT {
                    break 'set_ground RailGroundType::SnowOrDesert;
                }
            }

            _ => {}
        }

        let mut new_ground = RailGroundType::Grass;

        if w.is_plain_rail(tile.0) && old_ground != RailGroundType::Barren {
            // wait until bottom is green
            /* determine direction of fence */
            let rail = TrackBits(w.tracks(tile.0));

            let owner = w.owner(tile);
            let mut fences = DiagDirections::EMPTY;

            for d in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
                const DIR_TO_TRACKBITS: [TrackBits; 4] = [
                    TrackBits::TRACK_BIT_3WAY_NE,
                    TrackBits::TRACK_BIT_3WAY_SE,
                    TrackBits::TRACK_BIT_3WAY_SW,
                    TrackBits::TRACK_BIT_3WAY_NW,
                ];
                let d = DiagDirection(d);

                /* Track bit on this edge => no fence. */
                if rail & DIR_TO_TRACKBITS[usize::from(d.0)] != TrackBits::TRACK_BIT_NONE {
                    continue;
                }

                let tile2 = c.neighbour(tile, d);

                /* Show fences if it's a house, industry, object, road, tunnelbridge or not owned by us. */
                if !w.is_valid_tile(tile2)
                    || w.kind(tile2) == TileType::MP_HOUSE
                    || w.kind(tile2) == TileType::MP_INDUSTRY
                    || w.kind(tile2) == TileType::MP_ROAD
                    || (w.kind(tile2) == TileType::MP_OBJECT && !(r.owned_land)(tile2.0))
                    || w.kind(tile2) == TileType::MP_TUNNELBRIDGE
                    || w.owner(tile2) != owner
                {
                    fences = fences.with(DiagDirections::from_element(d));
                }
            }

            const NE: DiagDirections = DiagDirections::from_element(DiagDirection::DIAGDIR_NE);
            const SE: DiagDirections = DiagDirections::from_element(DiagDirection::DIAGDIR_SE);
            const SW: DiagDirections = DiagDirections::from_element(DiagDirection::DIAGDIR_SW);
            const NW: DiagDirections = DiagDirections::from_element(DiagDirection::DIAGDIR_NW);
            const NE_SW: DiagDirections = NE.with(SW);
            const SE_NW: DiagDirections = SE.with(NW);
            const NE_SE: DiagDirections = NE.with(SE);
            const NE_NW: DiagDirections = NE.with(NW);
            const SE_SW: DiagDirections = SE.with(SW);
            const SW_NW: DiagDirections = SW.with(NW);
            match fences {
                DiagDirections::EMPTY => {}
                NE => new_ground = RailGroundType::FenceNE,
                SE => new_ground = RailGroundType::FenceSE,
                SW => new_ground = RailGroundType::FenceSW,
                NW => new_ground = RailGroundType::FenceNW,
                NE_SW => new_ground = RailGroundType::FenceNESW,
                SE_NW => new_ground = RailGroundType::FenceSENW,
                NE_SE => new_ground = RailGroundType::FenceVert1,
                NE_NW => new_ground = RailGroundType::FenceHoriz2,
                SE_SW => new_ground = RailGroundType::FenceHoriz1,
                SW_NW => new_ground = RailGroundType::FenceVert2,
                _ => crate::services_assert::not_reached(),
            }
        }
        new_ground
    };

    if old_ground != new_ground {
        (r.set_ground)(tile.0, new_ground);
        hit(Witness::RailGround);
        c.mark_tile_dirty_by_tile(tile);
    }
}

/// `TileLoop_TunnelBridge`.
pub(crate) fn tile_loop_tunnel_bridge(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    // SAFETY: the batch's table is immutable and lives for the process.
    let t = unsafe { &*c.ground.tunnel };
    let snow_or_desert = (t.snow_or_desert)(tile.0);
    match c.landscape {
        LandscapeType::Arctic => {
            /* As long as we do not have a snow density, we want to use the density
             * from the entry edge. For tunnels this is the lowest point for bridges the highest point.
             * (Independent of foundations) */
            let z = if w.bridge(tile.0) {
                w.tile_max_z(tile)
            } else {
                w.tile_z(tile)
            };
            if snow_or_desert != (z > c.get_snow_line()) {
                (t.set_snow_or_desert)(tile.0, !snow_or_desert);
                hit(Witness::TunnelGround);
                c.mark_tile_dirty_by_tile(tile);
            }
        }

        LandscapeType::Tropic => {
            if c.get_tropic_zone(tile) == TropicZone::TROPICZONE_DESERT && !snow_or_desert {
                (t.set_snow_or_desert)(tile.0, true);
                hit(Witness::TunnelGround);
                c.mark_tile_dirty_by_tile(tile);
            }
        }

        _ => {}
    }
}

/// `_town_road_types`: the desired roadside and its pre-type by town radius group.
const TOWN_ROAD_TYPES: [[Roadside; 2]; HouseZone::TownEnd.0 as usize] = [
    [Roadside::Grass, Roadside::Grass],
    [Roadside::Paved, Roadside::Paved],
    [Roadside::Paved, Roadside::Paved],
    [Roadside::Trees, Roadside::Trees],
    [Roadside::StreetLights, Roadside::Paved],
];

/// `_town_road_types_2`: [`TOWN_ROAD_TYPES`] in Toyland.
const TOWN_ROAD_TYPES_2: [[Roadside; 2]; HouseZone::TownEnd.0 as usize] = [
    [Roadside::Grass, Roadside::Grass],
    [Roadside::Paved, Roadside::Paved],
    [Roadside::StreetLights, Roadside::Paved],
    [Roadside::StreetLights, Roadside::Paved],
    [Roadside::StreetLights, Roadside::Paved],
];

/// `EnsureNoVehicleOnGround(tile).Succeeded()`, in the batch's lazy native result.
fn ensure_no_vehicle_on_ground_succeeded(c: &TileLoop<'_>, tile: TileIndex) -> bool {
    // SAFETY: the batch's table is immutable and lives for the process.
    let costs = unsafe { &*c.ground.costs };
    // SAFETY: the batch owns this live native stack context.
    let out = unsafe { (costs.prepare)(c.ground.call) };
    // SAFETY: prepare provides live native result storage; the check has returned before
    // the result is read and released.
    unsafe {
        (*costs.commands).no_vehicle(out, tile.0);
        let succeeded = ((*costs.costs).success)(out);
        (costs.release)(c.ground.call);
        succeeded
    }
}

/// `TileLoop_Road`'s roadside step: adjust road ground type depending on `grp` (`grp` is the
/// distance to the center).
#[inline]
fn adjust_road_ground_type(c: &TileLoop<'_>, tile: TileIndex, grp: HouseZone) {
    let w = &c.world;
    let new_rs = if c.landscape == LandscapeType::Toyland {
        TOWN_ROAD_TYPES_2[usize::from(grp.0)]
    } else {
        TOWN_ROAD_TYPES[usize::from(grp.0)]
    };
    let mut cur_rs = w.asserted(|m| crate::road_map::get_roadside(m, tile.0));

    /* We have our desired type, do nothing */
    if cur_rs == new_rs[0] {
        return;
    }

    /* We have the pre-type of the desired type, switch to the desired type */
    if cur_rs == new_rs[1] {
        cur_rs = new_rs[0];
    /* We have barren land, install the pre-type */
    } else if cur_rs == Roadside::Barren {
        cur_rs = new_rs[1];
    /* We're totally off limits, remove any installation and make barren land */
    } else {
        cur_rs = Roadside::Barren;
    }
    w.asserted(|m| crate::road_map::set_roadside(m, tile, cur_rs));
    c.mark_tile_dirty_by_tile(tile);
    hit(Witness::Roadside);
}

/// `TileLoop_Road`.
#[allow(
    clippy::cast_possible_wrap,
    reason = "`TileX(tile) * TILE_SIZE + 7` as CreateEffectVehicleAbove's int"
)]
pub(crate) fn tile_loop_road(c: &TileLoop<'_>, tile: TileIndex) {
    let w = &c.world;
    let s = c.s();
    // SAFETY: the batch's table is immutable and lives for the process.
    let r = unsafe { &*c.ground.road };
    match c.landscape {
        LandscapeType::Arctic => {
            /* Roads on flat foundations use the snow level of the height they are elevated to. All others use the snow level of their minimum height. */
            let tile_z = if crate::movement::get_foundation_slope(s, tile).0 == Slope::SLOPE_FLAT {
                w.tile_max_z(tile)
            } else {
                w.tile_z(tile)
            };
            if (r.snow_or_desert)(tile.0) != (tile_z > c.get_snow_line()) {
                (r.toggle_snow_or_desert)(tile.0);
                c.mark_tile_dirty_by_tile(tile);
            }
        }

        LandscapeType::Tropic => {
            if c.get_tropic_zone(tile) == TropicZone::TROPICZONE_DESERT
                && !(r.snow_or_desert)(tile.0)
            {
                (r.toggle_snow_or_desert)(tile.0);
                c.mark_tile_dirty_by_tile(tile);
            }
        }

        _ => {}
    }

    if w.road_tile_type(tile.0).0 == crate::road_map::RoadTileType::Depot.0 {
        return;
    }

    let t = crate::town::spatial::closest_town_from_tile(s, tile, u32::MAX);
    if !w.has_road_works(tile.0) {
        let mut grp = HouseZone::TownEdge;

        if !t.is_null() {
            let town = (s.town_owner)(t);
            // SAFETY: the closest town is live; each read is one field-sized load.
            unsafe {
                grp = crate::town::get_town_radius_group(town, tile, w.dimensions);

                /* Show an animation to indicate road work */
                if crate::town::road_build_months(town) != 0
                    && (w
                        .dimensions
                        .distance_manhattan(crate::town::position(town), tile)
                        < 8
                        || grp != HouseZone::TownEdge)
                    && w.asserted(|m| crate::road_map::is_normal_road(m, tile.0))
                    && !crate::bitmath_func::has_at_most_one_bit(u64::from(w.all_road_bits(tile.0)))
                    && crate::movement::get_foundation_slope(s, tile).0 == Slope::SLOPE_FLAT
                    && ensure_no_vehicle_on_ground_succeeded(c, tile)
                    && crate::random_func::chance16(s, 1, 40)
                {
                    w.asserted(|m| crate::road_map::start_road_works(m, tile));
                    hit(Witness::RoadWorkStart);

                    if c.sound_ambient {
                        (s.play_sound)(
                            tile.0,
                            u16::from(crate::sound_type::SoundFx::SND_21_ROAD_WORKS.0),
                        );
                    }
                    crate::effect::openttd_rust_effect_create_above(
                        (w.dimensions.tile_x(tile) * crate::tile_type::TILE_SIZE + 7) as i32,
                        (w.dimensions.tile_y(tile) * crate::tile_type::TILE_SIZE + 7) as i32,
                        0,
                        crate::effectvehicle_func::EV_BULLDOZER,
                    );
                    c.mark_tile_dirty_by_tile(tile);
                    return;
                }
            }
        }

        adjust_road_ground_type(c, tile, grp);
    } else if w.asserted(|m| crate::road_map::increase_road_works_counter(m, tile)) {
        w.asserted(|m| crate::road_map::terminate_road_works(m, tile));
        hit(Witness::RoadWorkEnd);

        if c.mod_road_rebuild {
            /* Generate a nicer town surface */
            let old_rb = RoadBits((s.road_bits)(
                tile.0,
                u8::from(crate::road_type::RTT_ROAD.0),
                false,
            ));
            let new_rb = crate::road_cleanup::clean_up_road_bits(s, tile.0, old_rb);

            if old_rb != new_rb {
                let flags = DoCommandFlags::from_element(DoCommandFlag::Execute)
                    .with(DoCommandFlags::from_element(DoCommandFlag::Auto))
                    .with(DoCommandFlags::from_element(DoCommandFlag::NoWater));
                // SAFETY: the batch owns both native result slots of the command context.
                unsafe {
                    let command = &*c.ground.road_command;
                    crate::road_cmd::openttd_rust_road_remove(
                        command.services,
                        command.cost,
                        command.ret,
                        flags.0,
                        tile.0,
                        (old_rb ^ new_rb).0,
                        u8::from(crate::road_type::RTT_ROAD.0),
                        true,
                    );
                }
                hit(Witness::RoadRemove);

                /* If new_rb is 0, there are now no road pieces left and the tile is no longer a road tile */
                if new_rb == RoadBits::ROAD_NONE {
                    c.mark_tile_dirty_by_tile(tile);
                    return;
                }
            }
        }

        /* Possibly change road type */
        if w.road_owner(tile.0, false) == crate::company_type::OWNER_TOWN {
            // SAFETY: the immutable town table is live; road selection borrows no owner.
            let rt = unsafe { crate::town::get_town_road_type(r.town, s) };
            if rt != w.asserted(|m| crate::road_map::get_road_type_road(m, tile.0)) {
                crate::road_map::set_road_type_road(s, tile.0, rt.0);
            }
        }

        c.mark_tile_dirty_by_tile(tile);
    }
}

/// Actual record-return and enum-argument calls for every default native ABI.
#[repr(C)]
pub struct ABICalls {
    pub foundation: extern "C" fn() -> FoundationSlope,
    pub airport: extern "C" fn() -> crate::water::AirportGround,
    pub enums: extern "C" fn(
        ClearGround,
        RailGroundType,
        Slope,
        TrackBits,
        crate::services::Track,
        Foundation,
        Roadside,
        RoadBits,
        RoadType,
        crate::services::TreeGround,
        WaterClass,
        crate::services::FloodingBehaviour,
        DiagDirection,
        LandscapeType,
        crate::vehicle::VehicleType,
    ) -> bool,
}
/// # Safety
/// A complete immutable fixture table. Returned opaque handles are marker values
/// for this ABI-only call; they are compared, never dereferenced or used by game services.
#[cfg(feature = "native-test-support")]
pub(crate) unsafe fn abi_calls(calls: *const ABICalls) -> u8 {
    // SAFETY: Caller provides actual typed native functions for this fixture.
    let c = unsafe { &*calls };
    let foundation = (c.foundation)();
    let airport = (c.airport)();
    u8::from(
        foundation.z == 123_456
            && foundation.slope.0 == 234
            && airport.station.is_null()
            && airport.tile == 0x1234_5678
            && airport.width == 0xabcd
            && airport.height == 0xdcba
            && (c.enums)(
                ClearGround(240),
                RailGroundType(239),
                Slope(238),
                TrackBits(237),
                crate::services::Track(227),
                Foundation(228),
                Roadside(229),
                RoadBits(230),
                RoadType(231),
                crate::services::TreeGround(232),
                WaterClass(233),
                crate::services::FloodingBehaviour(234),
                DiagDirection(235),
                LandscapeType(236),
                crate::vehicle_type::VehicleType(226),
            ),
    )
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGroundCostServices" ["landscape"] => CostServices {
        commands,
        costs,
        prepare,
        release,
    };
    "OpenTTDRoadGroundServices" ["landscape"] => RoadServices {
        snow_or_desert,
        toggle_snow_or_desert,
        roadside,
        set_roadside,
        all_bits,
        town,
    };
    "OpenTTDClearServices" ["landscape"] => ClearServices {
        set_ground,
        set_counter,
        clear_snow,
        make_clear,
        set_field,
    };
    "OpenTTDRailServices" ["landscape"] => RailServices {
        tracks,
        ground,
        set_ground,
        owned_land,
    };
    "OpenTTDTunnelServices" ["landscape"] => TunnelServices {
        snow_or_desert,
        set_snow_or_desert,
    };
    "OpenTTDGroundABICalls" ["landscape"] => ABICalls {
        foundation,
        airport,
        enums,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use std::cell::Cell;

    thread_local! {
        static FAILED: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED.set(FAILED.get() + 1);
        }
    }
    extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
        0
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }

    /// A 64x64 map of `kind` tiles, installed as this thread's canonical map.
    struct Map {
        tiles: Vec<BaseTileLayout>,
        extended: Vec<ExtendedTileLayout>,
    }
    impl Map {
        fn new(kind: TileType) -> Self {
            let mut tiles: Vec<_> = (0..64 * 64)
                .map(|_| BaseTileLayout {
                    kind: kind.0 << 4,
                    height: 0,
                    m1: 0,
                    m2: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect();
            let extended: Vec<_> = (0..64 * 64)
                .map(|_| ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect();
            crate::map_func::fixture::set(
                TileTypeAllocation {
                    base: tiles.as_mut_ptr(),
                    extended: extended.as_ptr(),
                    count: 64 * 64,
                    assertions: true,
                },
                6,
            );
            Self { tiles, extended }
        }
    }

    /// Shared services over this thread's map whose native assertion records failures.
    fn shared() -> &'static Services {
        let shared: &'static mut Services = Box::leak(Box::new(crate::services::fixture(
            std::ptr::null_mut(),
            random,
            industry,
        )));
        shared.map = crate::map_func::fixture::statics();
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            shared: std::ptr::from_ref(shared),
            native_assert: record_assertion,
            ..crate::test_tables::MOVEMENT
        }));
        shared.movement = std::ptr::from_ref(movement);
        shared
    }

    const NO_TABLES: crate::tile_loop::GroundServices = crate::tile_loop::GroundServices {
        clear: std::ptr::null(),
        rail: std::ptr::null(),
        road: std::ptr::null(),
        road_command: std::ptr::null(),
        tunnel: std::ptr::null(),
        water: std::ptr::null(),
        costs: std::ptr::null(),
        call: std::ptr::null_mut(),
        climate: LandscapeType::Temperate,
        editor: false,
        rebuild: false,
    };

    fn tile_loop(s: &'static Services, landscape: LandscapeType) -> TileLoop<'static> {
        TileLoop {
            world: World::new(s),
            size_y: 64,
            ground: &NO_TABLES,
            landscape,
            game_mode_editor: false,
            sound_ambient: false,
            ambient_callback: false,
            mod_road_rebuild: false,
        }
    }

    /// `UpdateFences` asserts that its tile is a field. On a grass tile the native handler
    /// sees that assertion, then each `GetFence`'s, and the body continues as the release
    /// build does: every fence is already set, so no neighbour is read and nothing is written.
    #[test]
    fn update_fences_on_a_grass_tile_reports_each_failed_assertion() {
        FAILED.set(0);
        let mut map = Map::new(TileType::MP_CLEAR);
        let tile = TileIndex(5 * 64 + 5);
        let t = tile.0 as usize;
        map.tiles[t].m3 = 1 << 5;
        map.tiles[t].m4 = 1 << 5 | 1 << 2;
        map.extended[t].m6 = 1 << 2;
        let c = tile_loop(shared(), LandscapeType::Temperate);
        update_fences(&c, tile);
        assert_eq!(FAILED.get(), 5);
    }

    /// `StartRoadWorks` asserts `!HasRoadWorks(t)` and `TerminateRoadWorks` asserts
    /// `HasRoadWorks(t)`; after the failed assertion each writes as the release build does.
    #[test]
    fn road_works_report_the_failed_roadside_assertions_and_continue() {
        FAILED.set(0);
        let mut map = Map::new(TileType::MP_ROAD);
        let tile = TileIndex(7 * 64 + 7);
        let t = tile.0 as usize;
        let c = tile_loop(shared(), LandscapeType::Temperate);
        let roadside = |map: &Map| Roadside((map.extended[t].m6 >> 3) & 7);

        map.extended[t].m6 = Roadside::GrassRoadWorks.0 << 3;
        c.world
            .asserted(|m| crate::road_map::start_road_works(m, tile));
        assert_eq!(FAILED.get(), 1);
        assert_eq!(roadside(&map), Roadside::PavedRoadWorks);

        map.extended[t].m6 = Roadside::Grass.0 << 3;
        map.extended[t].m7 = 0xf9;
        c.world
            .asserted(|m| crate::road_map::terminate_road_works(m, tile));
        assert_eq!(FAILED.get(), 2);
        assert_eq!(roadside(&map), Roadside::Paved);
        assert_eq!(map.extended[t].m7, 0xf0);
    }

    /// `TileLoop_Road`'s roadside step over `_town_road_types` and `_town_road_types_2`, for
    /// every town radius group and every roadside without road works.
    #[test]
    fn roadside_step_follows_the_town_road_type_tables() {
        use Roadside as R;
        const B: R = R::Barren;
        const G: R = R::Grass;
        const P: R = R::Paved;
        const L: R = R::StreetLights;
        const T: R = R::Trees;
        const CURRENT: [R; 5] = [B, G, P, L, T];
        // The roadside after one step, per radius group, for each `CURRENT` roadside.
        let temperate = [
            [G, G, B, B, B],
            [P, B, P, B, B],
            [P, B, P, B, B],
            [T, B, B, B, T],
            [P, B, L, L, B],
        ];
        let toyland = [
            [G, G, B, B, B],
            [P, B, P, B, B],
            [P, B, L, L, B],
            [P, B, L, L, B],
            [P, B, L, L, B],
        ];
        let mut map = Map::new(TileType::MP_ROAD);
        let tile = TileIndex(9 * 64 + 9);
        let t = tile.0 as usize;
        let s = shared();
        for (landscape, expected) in [
            (LandscapeType::Temperate, temperate),
            (LandscapeType::Toyland, toyland),
        ] {
            let c = tile_loop(s, landscape);
            for (grp, row) in expected.iter().enumerate() {
                for (current, after) in CURRENT.iter().zip(row) {
                    map.extended[t].m6 = current.0 << 3;
                    adjust_road_ground_type(&c, tile, HouseZone(grp as u8));
                    assert_eq!(
                        Roadside((map.extended[t].m6 >> 3) & 7),
                        *after,
                        "{landscape:?} group {grp} from {current:?}"
                    );
                }
            }
        }
    }
}

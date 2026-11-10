/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete waypoint command policy; shared station operations are called directly.
//! Native pointers are opaque, synchronous and serialized by the game-state mutex.
use super::{
    BaseStation, CommandCost, CommandServicesTable, Cost, Ctx, JoiningStation, RoadStopCost,
    SignSearch, TileArea, sign_visit,
};
use crate::base_station_base::StationRectMode;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::direction_type::Axis;
use crate::economy_type::{
    PR_BUILD_WAYPOINT_BUOY, PR_BUILD_WAYPOINT_RAIL, PR_CLEAR_WAYPOINT_BUOY,
    ROAD_STOP_TRACKBIT_FACTOR,
};
use crate::newgrf_roadstop::RoadStopClassID;
use crate::newgrf_station::StationClassID;
use crate::rail_map::RAIL_TILE_NORMAL;
use crate::road_type::{RoadBits, RoadTramType, RoadType};
use crate::slope_type::Slope;
use crate::station_map::StationGfx;
use crate::station_type::{
    MAX_LENGTH_STATION_NAME_CHARS, StationFacilities, StationFacility, StationID, StationType,
};
use crate::strings_type::StringID;
use crate::tile_type::{TileIndex, TileType};
use crate::track_type::TrackBits;
use crate::witness as w;

/// `CmdRenameWaypoint`'s arguments.
#[derive(Clone, Copy)]
struct CmdRenameWaypoint {
    /// Id of waypoint.
    waypoint_id: StationID,
    /// The new name, or an empty string when resetting to the default.
    text: *const u8,
    /// The byte length of `text`.
    length: usize,
}
/// `CmdMoveWaypointName`'s arguments.
#[derive(Clone, Copy)]
struct CmdMoveWaypointName {
    /// Id of waypoint.
    waypoint_id: StationID,
    /// Tile to move the waypoint name to.
    tile: TileIndex,
}
/// `CmdBuildRailWaypoint`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildRailWaypoint {
    /// Northern most tile where waypoint will be built.
    start_tile: TileIndex,
    /// Orientation (Axis).
    axis: Axis,
    /// Width of waypoint.
    width: u8,
    /// Height of waypoint.
    height: u8,
    /// Custom station class.
    spec_class: StationClassID,
    /// Custom station id.
    spec_index: u16,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow waypoints directly adjacent to other waypoints.
    adjacent: bool,
}
/// `CmdBuildRoadWaypoint`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildRoadWaypoint {
    /// Northern most tile where waypoint will be built.
    start_tile: TileIndex,
    /// Orientation (Axis).
    axis: Axis,
    /// Width of waypoint.
    width: u8,
    /// Height of waypoint.
    height: u8,
    /// Custom road stop class.
    spec_class: RoadStopClassID,
    /// Custom road stop id.
    spec_index: u16,
    /// Station ID to join (`NEW_STATION` if build new one).
    station_to_join: StationID,
    /// Allow waypoints directly adjacent to other waypoints.
    adjacent: bool,
}
/// `CmdBuildBuoy`'s arguments.
#[derive(Clone, Copy)]
struct CmdBuildBuoy {
    /// Tile where to place the buoy.
    tile: TileIndex,
}
/// A waypoint command's execution flags and its native result storage.
#[derive(Clone, Copy)]
struct WaypointCall<'a> {
    /// The command's resulting cost or error.
    cost: Cost<'a>,
    /// Scratch storage for the nested checks' results.
    ret: Cost<'a>,
    /// Type of operation.
    flags: DoCommandFlags,
}

impl Ctx<'_> {
    /// `GetAxisForNewRailWaypoint`: the axis for a new rail waypoint. If it is a
    /// valid tile to build a waypoint on it returns a valid `Axis`, otherwise an
    /// invalid one.
    fn get_axis_for_new_rail_waypoint(&self, tile: TileIndex) -> Axis {
        w::hit(w::WAYPOINT_AXIS_RAIL);
        /* The axis for rail waypoints is easy. */
        if crate::station_map::is_rail_waypoint_tile(tile, self.shared) {
            return crate::map_func::World::new(self.shared).get_rail_station_axis(tile);
        }
        /* Non-plain rail type, no valid axis for waypoints. */
        if TileType((self.shared.tile_type)(tile.0)) != TileType::MP_RAILWAY
            || crate::services::RailTileType(
                crate::map_func::World::new(self.shared).rail_tile_type(tile.0),
            )
            .0 != RAIL_TILE_NORMAL
        {
            return Axis::INVALID_AXIS;
        }
        let ground = unsafe { &*self.rail_map().ground };
        match TrackBits((ground.tracks)(tile.0).0) {
            TrackBits::TRACK_BIT_X => Axis::AXIS_X,
            TrackBits::TRACK_BIT_Y => Axis::AXIS_Y,
            _ => Axis::INVALID_AXIS,
        }
    }
    /// `GetAxisForNewRoadWaypoint`: the axis for a new road waypoint. If it is a
    /// valid tile to build a waypoint on it returns a valid `Axis`, otherwise an
    /// invalid one.
    fn get_axis_for_new_road_waypoint(&self, tile: TileIndex) -> Axis {
        w::hit(w::WAYPOINT_AXIS_ROAD);
        /* The axis for existing road waypoints is easy. */
        if crate::station_map::is_road_waypoint_tile(tile, self.shared) {
            return crate::map_func::World::new(self.shared).get_drive_through_stop_axis(tile);
        }
        /* Non-plain road type, no valid axis for waypoints. */
        if !crate::map_func::World::new(self.shared).is_normal_road_tile(tile.0) {
            return Axis::INVALID_AXIS;
        }
        let ground = unsafe { &*self.s.road_ground };
        let bits = RoadBits((ground.all_bits)(tile.0).0);
        if bits & RoadBits::ROAD_Y == RoadBits::ROAD_NONE {
            Axis::AXIS_X
        } else if bits & RoadBits::ROAD_X == RoadBits::ROAD_NONE {
            Axis::AXIS_Y
        } else {
            Axis::INVALID_AXIS
        }
    }
    /// `CmdRenameWaypoint`: rename a waypoint.
    fn cmd_rename_waypoint(&self, call: WaypointCall<'_>, args: CmdRenameWaypoint) {
        w::hit(w::WAYPOINT_RENAME);
        let (out, flags) = (call.cost, call.flags);
        let wp = self.waypoint(args.waypoint_id);
        if wp.is_null() {
            return out.error(StringID::INVALID_STRING_ID.0);
        }
        let owner = CompanyID((self.world().station_owner)(wp));
        if owner != CompanyID::NONE {
            unsafe { self.command.ownership(out.0, owner.0, 0) };
            if !out.ok() {
                return;
            }
        }
        let reset = args.length == 0;
        if !reset {
            if (self.shared.utf8_length)(args.text, args.length) >= MAX_LENGTH_STATION_NAME_CHARS {
                return out.error(StringID::INVALID_STRING_ID.0);
            }
            // `IsUniqueWaypointName`: check whether the name is unique amongst the waypoints.
            for index in 0..(self.world().pool_size)() {
                let candidate = self.waypoint(StationID(index as u16));
                if !candidate.is_null() && (self.s.name_equals)(candidate, args.text, args.length) {
                    return out.error(self.s.strings.name_unique);
                }
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            if reset {
                (self.s.clear_name)(wp);
            } else {
                (self.s.set_name)(wp, args.text, args.length);
            }
            (self.s.virtual_coord)(wp);
        }
        out.zero();
    }
    /// `CmdMoveWaypointName`: move a waypoint name; returns the waypoint ID.
    fn cmd_move_waypoint_name(
        &self,
        call: WaypointCall<'_>,
        args: CmdMoveWaypointName,
    ) -> StationID {
        w::hit(w::WAYPOINT_MOVE_NAME);
        let (out, flags) = (call.cost, call.flags);
        let tile = args.tile;
        let wp = self.waypoint(args.waypoint_id);
        if wp.is_null() {
            out.error(StringID::INVALID_STRING_ID.0);
            return StationID::INVALID;
        }
        let owner = CompanyID((self.world().station_owner)(wp));
        if owner != CompanyID::NONE {
            unsafe { self.command.ownership(out.0, owner.0, 0) };
            if !out.ok() {
                return StationID::INVALID;
            }
        }
        if !self.rect_contains(wp, tile) {
            out.error(self.s.strings.unsuitable);
            return StationID::INVALID;
        }
        /* Check if the tile is the base tile of another station */
        let mut search = SignSearch {
            c: self,
            station: wp,
            tile,
            other: false,
        };
        crate::station::spatial::for_all_stations_radius(self.shared, tile, 0, |st| unsafe {
            sign_visit((&raw mut search).cast(), st);
        });
        if search.other {
            out.error(self.s.strings.unsuitable);
            return StationID::INVALID;
        }
        if flags.test(DoCommandFlag::Execute) {
            crate::station::spatial::station_move_sign(self.shared, wp, tile);
            (self.s.virtual_coord)(wp);
        }
        out.zero();
        args.waypoint_id
    }
}
/// # Safety
/// The immutable native services and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_rail_axis(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    unsafe { Ctx::new(s) }
        .get_axis_for_new_rail_waypoint(TileIndex(tile))
        .0
}
/// # Safety
/// The immutable native services and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_road_axis(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    unsafe { Ctx::new(s) }
        .get_axis_for_new_road_waypoint(TileIndex(tile))
        .0
}
/// # Safety
/// Services, native output, pool and borrowed string bytes remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_rename(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    text: *const u8,
    length: usize,
) {
    let c = unsafe { Ctx::new(s) };
    // Renaming needs no scratch result; `ret` aliases the output.
    c.cmd_rename_waypoint(
        WaypointCall {
            cost: c.c(out),
            ret: c.c(out),
            flags: DoCommandFlags(flags),
        },
        CmdRenameWaypoint {
            waypoint_id: StationID(id),
            text,
            length,
        },
    );
}
/// # Safety
/// Services, native output and pool remain live for the synchronous entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_move_name(
    s: *const CommandServicesTable,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    tile: u32,
) -> u16 {
    let c = unsafe { Ctx::new(s) };
    // Moving the name needs no scratch result; `ret` aliases the output.
    c.cmd_move_waypoint_name(
        WaypointCall {
            cost: c.c(out),
            ret: c.c(out),
            flags: DoCommandFlags(flags),
        },
        CmdMoveWaypointName {
            waypoint_id: StationID(id),
            tile: TileIndex(tile),
        },
    )
    .0
}

/// Opaque caller-owned `RailStationTileLayout` and iterator, never dereferenced here.
#[repr(C)]
pub struct WaypointLayout {
    _private: [u8; 0],
}
impl Ctx<'_> {
    /// `FindDeletedWaypointCloseTo`: find a deleted waypoint close to a tile, with
    /// the given string, previous owner `cid` and road kind.
    fn find_deleted_waypoint_close_to(
        &self,
        tile: TileIndex,
        str: StringID,
        cid: CompanyID,
        is_road: bool,
    ) -> *mut BaseStation {
        let mut best: *mut BaseStation = std::ptr::null_mut();
        let mut thres = 8;
        // Pool ID order is Waypoint::Iterate order. Equal distances never replace
        // the earlier candidate, including deleted waypoints outside the kdtree.
        for id in 0..(self.world().pool_size)() {
            let wp = self.waypoint(StationID(id as u16));
            if wp.is_null()
                || (self.world().in_use)(wp)
                || StringID((self.s.string_id)(wp)) != str
                || CompanyID((self.world().station_owner)(wp)) != cid
                || (self.s.road_waypoint)(wp) != is_road
            {
                continue;
            }
            let xy = TileIndex((self.s.location)(wp));
            let cur_dist = self.map().distance_manhattan(tile.0, xy.0);
            if cur_dist == thres && !best.is_null() {
                w::hit(w::WAYPOINT_TIE);
            }
            if cur_dist < thres {
                thres = cur_dist;
                best = wp;
            }
        }
        if !best.is_null() {
            w::hit(w::WAYPOINT_REUSE);
        }
        best
    }
    /// `IsValidTileForWaypoint`: check whether the given tile is suitable for a
    /// waypoint along `axis`. `waypoint` is the waypoint the check is already
    /// joined to; finding another waypoint to join is an error.
    fn is_valid_tile_for_waypoint(
        &self,
        out: Cost<'_>,
        tile: TileIndex,
        axis: Axis,
        waypoint: &mut StationID,
    ) {
        w::hit(w::WAYPOINT_VALIDATE);
        /* if waypoint is set, then we have special handling to allow building on top of already existing waypoints.
         * so waypoint points to StationID::Invalid() if we can build on any waypoint.
         * Or it points to a waypoint if we're only allowed to build on exactly that waypoint. */
        if TileType((self.shared.tile_type)(tile.0)) == TileType::MP_STATION {
            if !crate::map_func::World::new(self.shared).is_rail_waypoint(tile) {
                return self.auto_clear(out, tile); // get error message
            }
            let wp = StationID((self.shared.station_index)(tile.0));
            if *waypoint == StationID::INVALID {
                *waypoint = wp;
            } else if *waypoint != wp {
                return out.error(self.s.strings.waypoint_adjoins_multiple);
            }
        }
        if self.get_axis_for_new_rail_waypoint(tile) != axis {
            return out.error(self.rail().strings.no_suitable_track);
        }
        let owner = CompanyID((self.shared.tile_owner)(tile.0));
        unsafe { self.command.ownership(out.0, owner.0, 0) };
        if out.ok() {
            unsafe { self.command.no_vehicle(out.0, tile.0) };
        }
        if !out.ok() {
            return;
        }
        let tileh = Slope(crate::map_func::World::new(self.shared).tile_slope(tile.0));
        if tileh != Slope::SLOPE_FLAT
            && (!(self.shared.build_on_slopes)()
                || crate::slope_func::is_steep_slope(tileh.0)
                || tileh.0 & (0x3 << axis.0) == 0
                || tileh.0 & !(0x3 << axis.0) == 0)
        {
            return out.error(self.s.strings.flat_land);
        }
        out.zero();
    }
    /// `wp->delete_ctr = 0`.
    fn reset_delete_counter(&self, st: *mut BaseStation) {
        let owner = (self.world().owner)(st);
        // SAFETY: The station service owner holds the counter; no borrow survives a callback.
        let fields = unsafe { crate::station_service::fields(owner) };
        unsafe { (&raw mut (*fields).delete_ctr).write(0) };
    }
    /// `TileOffsByAxis(OtherAxis(axis))`, as the unsigned pattern of its `TileIndexDiff`.
    fn offs_by_other_axis(&self, axis: Axis) -> u32 {
        self.map()
            .offs_by_axis(crate::direction_func::other_axis(axis.0, |ok| {
                crate::movement::native_assert(self.shared, ok);
            }))
    }
    /// The common checks of `CmdBuildRailWaypoint` and `CmdBuildRoadWaypoint` after
    /// their class checks: the number of parts to build, whether a deleted
    /// waypoint may be reused, and the station to join. `None` is `CMD_ERROR`.
    fn waypoint_parts_and_join(
        &self,
        axis: Axis,
        width: u8,
        height: u8,
        station_to_join: StationID,
    ) -> Option<(u8, bool, StationID)> {
        /* The number of parts to build */
        let count = if axis == Axis::AXIS_X { height } else { width };
        if (if axis == Axis::AXIS_X { width } else { height }) != 1
            || count == 0
            || count > (self.s.spread)()
        {
            return None;
        }
        let reuse = station_to_join != StationID::NEW;
        let mut station_to_join = station_to_join;
        if !reuse {
            station_to_join = StationID::INVALID;
        }
        let distant_join = station_to_join != StationID::INVALID;
        if distant_join && (!(self.s.distant_join)() || self.waypoint(station_to_join).is_null()) {
            return None;
        }
        Some((count, reuse, station_to_join))
    }
    /// `CmdBuildRailWaypoint`: convert existing rail to a waypoint, e.g. build a
    /// waypoint station over a piece of rail.
    fn cmd_build_rail_waypoint(
        &self,
        call: WaypointCall<'_>,
        layout: *mut WaypointLayout,
        args: CmdBuildRailWaypoint,
    ) {
        w::hit(w::WAYPOINT_BUILD_RAIL);
        let (cost, ret, flags) = (call.cost, call.ret, call.flags);
        let (start_tile, axis, width, height) =
            (args.start_tile, args.axis, args.width, args.height);
        let spec_class = args.spec_class;
        if !crate::direction_func::is_valid_axis(axis.0) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the given station class is valid */
        if u32::from(spec_class.0) >= crate::newgrf_class::station::count()
            || !crate::newgrf_station::is_waypoint_class(crate::newgrf_class::station_label(
                spec_class.0,
            ))
            || u32::from(args.spec_index) >= crate::newgrf_class::station::spec_count(spec_class.0)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let Some((count, reuse, station_to_join)) =
            self.waypoint_parts_and_join(axis, width, height, args.station_to_join)
        else {
            return cost.error(StringID::INVALID_STRING_ID.0);
        };
        let mut new_location = TileArea {
            tile: start_tile.0,
            w: u16::from(width),
            h: u16::from(height),
        };
        /* only AddCost for non-existing waypoints */
        cost.set(0);
        for cur_tile in new_location.tiles(self.map()) {
            if !crate::station_map::is_rail_waypoint_tile(TileIndex(cur_tile), self.shared) {
                cost.add_money(crate::company::price(PR_BUILD_WAYPOINT_RAIL));
            }
        }
        /* Make sure the area below consists of clear tiles. (OR tiles belonging to a certain rail station) */
        let mut est = StationID::INVALID;
        let spec = crate::newgrf_class::station::spec(spec_class.0, u32::from(args.spec_index));
        (self.s.waypoint_layout_init)(layout, spec, count);
        /* Check whether the tiles we're building on are valid rail or not. */
        let offset = self.offs_by_other_axis(axis);
        let mut tile = start_tile;
        for i in 0..count {
            let gfx = (self.s.waypoint_layout_gfx)(layout, i == 0);
            self.is_valid_tile_for_waypoint(ret, tile, axis, &mut est);
            if !ret.ok() {
                return cost.take(ret);
            }
            self.is_rail_station_bridge_above_ok(
                ret,
                tile,
                spec,
                StationType::RailWaypoint,
                StationGfx(gfx.wrapping_add(axis.0)),
            );
            if !ret.ok() {
                return cost.take(ret);
            }
            tile = TileIndex(tile.0.wrapping_add(offset));
        }
        let mut wp = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: est,
            station_to_join,
            adjacent: args.adjacent,
            ta: new_location,
            waypoint: true,
            road: false,
            error_message: StringID(self.s.strings.remove_rail_waypoint),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Check if there is an already existing, deleted, waypoint close to us that we can reuse. */
        let center_tile = TileIndex(
            start_tile
                .0
                .wrapping_add(u32::from(count / 2).wrapping_mul(offset)),
        );
        if wp.is_null() && reuse {
            wp = self.find_deleted_waypoint_close_to(
                center_tile,
                StringID(self.s.strings.waypoint_name),
                CompanyID((self.shared.current_company)()),
                false,
            );
        }
        if !wp.is_null() {
            /* Reuse an existing waypoint. */
            if (self.world().station_owner)(wp) != (self.shared.current_company)() {
                return cost.error(self.s.strings.waypoint_close);
            }
            /* Check if we want to expand an already existing waypoint. */
            if TileIndex(self.s.rail_area(wp).tile) != TileIndex::INVALID {
                new_location = self.can_expand_rail_station(ret, wp, new_location);
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
            self.before_add_rect(
                ret,
                wp,
                TileArea {
                    tile: start_tile.0,
                    w: u16::from(width),
                    h: u16::from(height),
                },
                StationRectMode::ADD_TEST,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
        } else if !(self.s.waypoint_available)() {
            /* Check if we can create a new waypoint. */
            return cost.error(self.s.strings.too_many_stations);
        }
        /* Check if we can allocate a custom spec to this waypoint. */
        let specindex = crate::station_association::allocate(self.s.rail_association, spec, wp);
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(start_tile.0);
        } else if !(self.world().in_use)(wp) {
            /* Move existing (recently deleted) waypoint to the new location */
            (self.s.set_location)(wp, start_tile.0);
        }
        let owner = CompanyID((self.shared.tile_owner)(start_tile.0));
        (self.s.set_station_owner)(wp, owner.0);
        self.before_add_rect(
            ret,
            wp,
            TileArea {
                tile: start_tile.0,
                w: u16::from(width),
                h: u16::from(height),
            },
            StationRectMode::ADD_TRY,
        );
        crate::station_association::assign(self.s.rail_association, spec, wp, specindex.value);
        self.reset_delete_counter(wp);
        (self.s.set_facility_bits)(
            wp,
            StationFacilities::from_element(StationFacility::Train).0,
        );
        (self.s.set_build_date)(wp, crate::timer_game_common::calendar_date().0);
        (self.s.set_string_id)(wp, self.s.strings.waypoint_name);
        (self.s.set_train_area)(wp, new_location);
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        (self.s.virtual_coord)(wp);
        let id = (self.links().graphs.station_id)(wp);
        let world = || crate::map_func::World::new(self.shared);
        let mut tile = start_tile;
        for i in 0..count {
            let gfx = (self.s.waypoint_layout_gfx)(layout, i == 0);
            let old_specindex = if world().has_station_tile_rail(tile) {
                (self.s.custom_rail_index)(tile.0) as u8
            } else {
                0
            };
            if !world().has_station_tile_rail(tile) {
                (self.s.station_infrastructure)(owner.0, 1);
            }
            let track = crate::track_func::axis_to_track(crate::services::Axis(axis.0), |ok| {
                crate::movement::native_assert(self.shared, ok);
            })
            .0;
            let reserved = if TileType((self.shared.tile_type)(tile.0)) == TileType::MP_RAILWAY {
                crate::bitmath_func::has_bit((self.rail_map().rail_reservation)(tile.0).0, track)
            } else {
                world().has_station_reservation(tile)
            };
            (self.s.make_rail_waypoint)(
                tile.0,
                owner.0,
                id,
                axis.0,
                gfx,
                crate::services::RailType(world().rail_type(tile.0)),
            );
            (self.s.set_custom_rail_index)(tile.0, specindex.value);
            self.set_rail_station_tile_flags(tile, spec);
            (self.s.set_rail_station_reservation)(tile.0, reserved);
            (self.shared.mark_dirty)(tile.0);
            crate::station_association::deallocate(self.s.rail_association, wp, old_specindex);
            if spec.is_null() {
                (self.s.close_station_inspect)(tile.0);
            }
            (self.rail().layout_changed)(tile.0, track);
            tile = TileIndex(tile.0.wrapping_add(offset));
        }
        (self.command.infrastructure_dirty)(owner.0);
    }
    /// `CmdBuildRoadWaypoint`: build a road waypoint on an existing road.
    fn cmd_build_road_waypoint(&self, call: WaypointCall<'_>, args: CmdBuildRoadWaypoint) {
        w::hit(w::WAYPOINT_BUILD_ROAD);
        let (cost, ret, flags) = (call.cost, call.ret, call.flags);
        let (start_tile, axis, width, height) =
            (args.start_tile, args.axis, args.width, args.height);
        let spec_class = args.spec_class;
        if !crate::direction_func::is_valid_axis(axis.0) {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        /* Check if the given station class is valid */
        if u32::from(spec_class.0) >= crate::newgrf_class::roadstop::count()
            || !crate::newgrf_station::is_waypoint_class(crate::newgrf_class::roadstop_label(
                spec_class.0,
            ))
            || u32::from(args.spec_index) >= crate::newgrf_class::roadstop::spec_count(spec_class.0)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let roadstopspec =
            crate::newgrf_class::roadstop::spec(spec_class.0, u32::from(args.spec_index));
        let Some((count, reuse, station_to_join)) =
            self.waypoint_parts_and_join(axis, width, height, args.station_to_join)
        else {
            return cost.error(StringID::INVALID_STRING_ID.0);
        };
        let roadstop_area = TileArea {
            tile: start_tile.0,
            w: u16::from(width),
            h: u16::from(height),
        };
        /* Total road stop cost. */
        let unit_cost = if roadstopspec.is_null() {
            crate::company::price(usize::from(
                crate::economy_type::Price::PR_BUILD_STATION_TRUCK.0,
            ))
        } else {
            crate::newgrf_roadstop::get_build_cost(
                self.s,
                roadstopspec,
                crate::economy_type::Price::PR_BUILD_STATION_TRUCK,
            )
        };
        let ddir = crate::direction_type::DiagDirection(crate::direction_func::axis_to_diag_dir(
            axis.0,
            |ok| crate::movement::native_assert(self.shared, ok),
        ));
        let mut est = StationID::INVALID;
        self.calculate_road_stop_cost(RoadStopCost {
            cost,
            ret,
            tile_area: roadstop_area,
            flags,
            is_drive_through: true,
            station_type: StationType::RoadWaypoint,
            roadstopspec,
            axis,
            ddir,
            est: &mut est,
            rt: RoadType::INVALID_ROADTYPE,
            unit_cost,
        });
        if !cost.ok() {
            return;
        }
        let mut wp = self.find_joining_base_station(JoiningStation {
            out: ret,
            existing_station: est,
            station_to_join,
            adjacent: args.adjacent,
            ta: roadstop_area,
            waypoint: true,
            road: true,
            error_message: StringID(self.s.strings.remove_road_waypoint),
        });
        if !ret.ok() {
            return cost.take(ret);
        }
        /* Check if there is an already existing, deleted, waypoint close to us that we can reuse. */
        let offset = self.offs_by_other_axis(axis);
        let center_tile = TileIndex(
            start_tile
                .0
                .wrapping_add(u32::from(count / 2).wrapping_mul(offset)),
        );
        if wp.is_null() && reuse {
            wp = self.find_deleted_waypoint_close_to(
                center_tile,
                StringID(self.s.strings.waypoint_name),
                CompanyID((self.shared.current_company)()),
                true,
            );
        }
        if !wp.is_null() {
            /* Reuse an existing waypoint. */
            if !(self.s.road_waypoint)(wp) {
                return cost.error(StringID::INVALID_STRING_ID.0);
            }
            if (self.world().station_owner)(wp) != (self.shared.current_company)() {
                return cost.error(self.s.strings.waypoint_close);
            }
            self.before_add_rect(ret, wp, roadstop_area, StationRectMode::ADD_TEST);
            if !ret.ok() {
                return cost.take(ret);
            }
        } else if !(self.s.waypoint_available)() {
            /* Check if we can create a new waypoint. */
            return cost.error(self.s.strings.too_many_stations);
        }
        /* Check if we can allocate a custom spec to this waypoint. */
        let index = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_allocate(
                roadstopspec,
                wp,
                self.s.roadstop_resolver,
            )
        };
        let specindex = super::SpecIndex {
            value: index as u8,
            valid: index >= 0,
        };
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(start_tile.0);
            (self.s.set_road_waypoint)(wp);
        } else if !(self.world().in_use)(wp) {
            /* Move existing (recently deleted) waypoint to the new location */
            (self.s.set_location)(wp, start_tile.0);
        }
        let owner = CompanyID((self.shared.current_company)());
        (self.s.set_station_owner)(wp, owner.0);
        self.before_add_rect(ret, wp, roadstop_area, StationRectMode::ADD_TRY);
        unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_assign(
                roadstopspec,
                wp,
                specindex.value,
                self.s.roadstop_resolver,
            );
        }
        if !roadstopspec.is_null() {
            /* Include this road stop spec's animation trigger bitmask
             * in the station's cached copy. */
            (self.s.road_triggers)(wp, roadstopspec);
        }
        self.reset_delete_counter(wp);
        let facilities = StationFacilities::from_element(StationFacility::BusStop)
            .with(StationFacilities::from_element(StationFacility::TruckStop));
        (self.s.set_facility_bits)(wp, facilities.0);
        (self.s.set_build_date)(wp, crate::timer_game_common::calendar_date().0);
        (self.s.set_string_id)(wp, self.s.strings.waypoint_name);
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        (self.s.virtual_coord)(wp);
        let id = (self.links().graphs.station_id)(wp);
        let world = || crate::map_func::World::new(self.shared);
        let replacement_spec_index = i32::from(specindex.value);
        /* Check every tile in the area. */
        for cur_tile in roadstop_area.tiles(self.map()) {
            let tile = TileIndex(cur_tile);
            /* Get existing road types and owners before any tile clearing */
            let road_rt = if self.may_have_road(tile) {
                self.get_road_type(tile, RoadTramType::RTT_ROAD)
            } else {
                RoadType::INVALID_ROADTYPE
            };
            let tram_rt = if self.may_have_road(tile) {
                self.get_road_type(tile, RoadTramType::RTT_TRAM)
            } else {
                RoadType::INVALID_ROADTYPE
            };
            let road_owner = if road_rt == RoadType::INVALID_ROADTYPE {
                owner
            } else {
                self.get_road_owner(tile, RoadTramType::RTT_ROAD)
            };
            let tram_owner = if tram_rt == RoadType::INVALID_ROADTYPE {
                owner
            } else {
                self.get_road_owner(tile, RoadTramType::RTT_TRAM)
            };
            if crate::station_map::is_road_waypoint_tile(tile, self.shared) {
                self.remove_road_waypoint_stop(ret, tile, flags, replacement_spec_index);
            }
            self.area_add((self.s.road_waypoint_tile_area)(wp), TileIndex(cur_tile));
            self.before_add_tile(ret, wp, TileIndex(cur_tile), StationRectMode::ADD_TRY);
            /* Update company infrastructure counts. If the current tile is a normal road tile, remove the old
             * bits first. */
            if world().is_normal_road_tile(cur_tile) {
                self.update_company_road_infrastructure(
                    road_rt,
                    road_owner,
                    -(world().road_bits(cur_tile, false).count_ones() as i32),
                );
                self.update_company_road_infrastructure(
                    tram_rt,
                    tram_owner,
                    -(world().road_bits(cur_tile, true).count_ones() as i32),
                );
            }
            self.update_company_road_infrastructure(
                road_rt,
                road_owner,
                ROAD_STOP_TRACKBIT_FACTOR as i32,
            );
            self.update_company_road_infrastructure(
                tram_rt,
                tram_owner,
                ROAD_STOP_TRACKBIT_FACTOR as i32,
            );
            (self.s.make_drive_stop)(
                cur_tile,
                owner.0,
                road_owner.0,
                tram_owner.0,
                id,
                StationType::RoadWaypoint.0,
                road_rt.0,
                tram_rt.0,
                axis.0,
            );
            (self.s.set_custom_road_index)(cur_tile, specindex.value);
            if !roadstopspec.is_null() {
                unsafe {
                    ((*self.s.roadstop_resolver).set_tile_random)(wp, cur_tile, 0);
                }
            }
            (self.s.station_infrastructure)(owner.0, 1);
            (self.shared.mark_dirty)(cur_tile);
        }
        (self.command.infrastructure_dirty)(owner.0);
    }
    /// `CmdBuildBuoy`: build a buoy.
    fn cmd_build_buoy(&self, call: WaypointCall<'_>, args: CmdBuildBuoy) {
        w::hit(w::WAYPOINT_BUILD_BUOY);
        let (cost, ret, flags, tile) = (call.cost, call.ret, call.flags, args.tile);
        if tile.0 == 0 || !(self.shared.has_water_ground)(tile.0) {
            return cost.error(self.s.strings.unsuitable);
        }
        // `IsBuoyBridgeAboveOk`.
        self.default_bridge_above_ok(ret, tile, StationType::Buoy, StationGfx(0));
        if !ret.ok() {
            return cost.take(ret);
        }
        if !(self.s.tile_flat)(tile.0) {
            return cost.error(self.s.strings.unsuitable);
        }
        /* Check if there is an already existing, deleted, waypoint close to us that we can reuse. */
        let mut wp = self.find_deleted_waypoint_close_to(
            tile,
            StringID(self.s.strings.buoy_name),
            CompanyID::NONE,
            false,
        );
        if wp.is_null() && !(self.s.waypoint_available)() {
            return cost.error(self.s.strings.too_many_stations);
        }
        cost.set(crate::company::price(PR_BUILD_WAYPOINT_BUOY));
        if !(self.shared.is_water_tile)(tile.0) {
            crate::landscape_cmd::clear_command(
                self.command,
                ret.0,
                tile.0,
                (flags | DoCommandFlags(DoCommandFlag::AUTO)).0,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
        if !flags.test(DoCommandFlag::Execute) {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(tile.0);
        } else {
            /* Move existing (recently deleted) buoy to the new location */
            (self.s.set_location)(wp, tile.0);
            (self.s.waypoint_view_invalidate)(wp);
        }
        self.before_add_tile(ret, wp, tile, StationRectMode::ADD_TRY);
        (self.s.set_string_id)(wp, self.s.strings.buoy_name);
        (self.s.set_facility_bits)(wp, StationFacilities::from_element(StationFacility::Dock).0);
        (self.s.set_station_owner)(wp, CompanyID::NONE.0);
        (self.s.set_build_date)(wp, crate::timer_game_common::calendar_date().0);
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        (self.s.make_buoy)(
            tile.0,
            (self.links().graphs.station_id)(wp),
            crate::services::WaterClass(
                crate::map_func::World::new(self.shared).water_class(tile.0),
            ),
        );
        self.check_for_docking_tile(tile);
        (self.shared.mark_dirty)(tile.0);
        unsafe {
            crate::water::openttd_rust_clear_neighbour_flooding(
                tile.0,
                self.map().size_x(),
                (self.shared.map_max_y)() + 1,
                (self.shared.water_services)(),
                self.s.shared,
            );
        }
        (self.s.virtual_coord)(wp);
        (self.s.waypoint_view_invalidate)(wp);
    }
    /// `RemoveBuoy`: remove a buoy. The tile is a buoy tile.
    pub(super) fn remove_buoy(&self, cost: Cost<'_>, tile: TileIndex, flags: DoCommandFlags) {
        w::hit(w::WAYPOINT_REMOVE_BUOY);
        /* XXX: strange stuff, allow clearing as invalid company when clearing landscape */
        if !(self.shared.company_valid)((self.shared.current_company)())
            && !flags.test(DoCommandFlag::Bankrupt)
        {
            return cost.error(StringID::INVALID_STRING_ID.0);
        }
        let wp = (self.world().get)((self.shared.station_index)(tile.0));
        if self.has_station_in_use(
            StationID((self.links().graphs.station_id)(wp)),
            false,
            CompanyID((self.shared.current_company)()),
        ) {
            return cost.error(self.s.strings.buoy_in_use);
        }
        if flags.test(DoCommandFlag::Bankrupt) {
            w::hit(w::WAYPOINT_BANKRUPT);
        }
        /* remove the buoy if there is a ship on tile when company goes bankrupt... */
        if !flags.test(DoCommandFlag::Bankrupt) {
            unsafe { self.command.no_vehicle(cost.0, tile.0) };
            if !cost.ok() {
                return;
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            (self.s.reset_facilities)(wp, StationFacilities::from_element(StationFacility::Dock).0);
            (self.s.waypoint_view_invalidate)(wp);
            /* We have to set the water tile's state to the same state as before the
             * buoy was placed. Otherwise one could plant a buoy on a canal edge,
             * remove it and flood the land (if the canal edge is at level 0) */
            unsafe {
                crate::water::openttd_rust_water_restore(
                    tile.0,
                    (self.shared.tile_owner)(tile.0),
                    self.s.shared,
                    (self.shared.water_services)(),
                    self.s.restoration,
                );
            }
            self.after_remove_tile(wp, tile);
            (self.s.virtual_coord)(wp);
            self.reset_delete_counter(wp);
        }
        cost.set(crate::company::price(PR_CLEAR_WAYPOINT_BUOY));
    }
}
/// # Safety
/// All native costs and the layout frame remain live; callbacks do not retain them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_build_rail(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    layout: *mut WaypointLayout,
    flags: u16,
    start: u32,
    axis: u8,
    width: u8,
    height: u8,
    class: u16,
    index: u16,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.cmd_build_rail_waypoint(
        WaypointCall {
            cost: c.c(cost),
            ret: c.c(ret),
            flags: DoCommandFlags(flags),
        },
        layout,
        CmdBuildRailWaypoint {
            start_tile: TileIndex(start),
            axis: Axis(axis),
            width,
            height,
            spec_class: StationClassID(class),
            spec_index: index,
            station_to_join: StationID(target),
            adjacent,
        },
    );
}
/// # Safety
/// Tables, native costs, serialized map/pool and specs remain live for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_build_road(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    start: u32,
    axis: u8,
    width: u8,
    height: u8,
    class: u16,
    index: u16,
    target: u16,
    adjacent: bool,
) {
    let c = unsafe { Ctx::new(s) };
    c.cmd_build_road_waypoint(
        WaypointCall {
            cost: c.c(cost),
            ret: c.c(ret),
            flags: DoCommandFlags(flags),
        },
        CmdBuildRoadWaypoint {
            start_tile: TileIndex(start),
            axis: Axis(axis),
            width,
            height,
            spec_class: RoadStopClassID(class),
            spec_index: index,
            station_to_join: StationID(target),
            adjacent,
        },
    );
}
/// # Safety
/// Tables, native costs and serialized map/pool remain live for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_build_buoy(
    s: *const CommandServicesTable,
    cost: *mut CommandCost,
    ret: *mut CommandCost,
    flags: u16,
    tile: u32,
) {
    let c = unsafe { Ctx::new(s) };
    c.cmd_build_buoy(
        WaypointCall {
            cost: c.c(cost),
            ret: c.c(ret),
            flags: DoCommandFlags(flags),
        },
        CmdBuildBuoy {
            tile: TileIndex(tile),
        },
    );
}

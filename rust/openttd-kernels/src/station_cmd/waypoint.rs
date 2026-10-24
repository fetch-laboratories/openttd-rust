/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete waypoint command policy; shared station operations are called directly.
//! Native pointers are opaque, synchronous and serialized by the game-state mutex.
use super::{
    BaseStation, CommandCost, CommandServicesTable, Cost, Ctx, SignSearch, TileArea, sign_visit,
};
use crate::base_station_base::{ADD_TEST, ADD_TRY};
use crate::command_type::DoCommandFlag;
use crate::company_type::OWNER_NONE;
use crate::direction_type::{AXIS_X, AXIS_Y, INVALID_AXIS};
use crate::economy_type::{
    PR_BUILD_STATION_TRUCK, PR_BUILD_WAYPOINT_BUOY, PR_BUILD_WAYPOINT_RAIL, PR_CLEAR_WAYPOINT_BUOY,
    ROAD_STOP_TRACKBIT_FACTOR,
};
use crate::rail_map::RAIL_TILE_NORMAL;
use crate::road_type::{INVALID_ROADTYPE, ROAD_X, ROAD_Y};
use crate::slope_func::{SLOPE_FLAT, is_steep_slope};
use crate::station_type::{
    BUOY, FACILITY_BUS, FACILITY_DOCK, FACILITY_TRAIN, FACILITY_TRUCK, INVALID_STATION,
    MAX_LENGTH_STATION_NAME_CHARS, NEW_STATION, RAIL_WAYPOINT, ROAD_WAYPOINT,
};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{INVALID_TILE, MP_RAILWAY, MP_STATION};
use crate::track_type::{TRACK_BIT_X, TRACK_BIT_Y};
use crate::witness as w;

impl Ctx<'_> {
    fn waypoint_rail_axis(&self, tile: u32) -> u8 {
        w::hit(w::WAYPOINT_AXIS_RAIL);
        if crate::station_map::is_rail_waypoint_tile(tile, self.shared) {
            return (self.s.rail_axis)(tile).0;
        }
        if (self.shared.tile_type)(tile) != MP_RAILWAY
            || (self.rail_map().rail_tile_type)(tile).0 != RAIL_TILE_NORMAL
        {
            return INVALID_AXIS;
        }
        let ground = unsafe { &*self.rail_map().ground };
        match (ground.tracks)(tile).0 {
            TRACK_BIT_X => AXIS_X,
            TRACK_BIT_Y => AXIS_Y,
            _ => INVALID_AXIS,
        }
    }
    fn waypoint_road_axis(&self, tile: u32) -> u8 {
        w::hit(w::WAYPOINT_AXIS_ROAD);
        if crate::station_map::is_road_waypoint_tile(tile, self.shared) {
            return (self.shared.drive_through_axis)(tile);
        }
        if !(self.shared.is_normal_road)(tile) {
            return INVALID_AXIS;
        }
        let ground = unsafe { &*self.s.road_ground };
        let bits = (ground.all_bits)(tile).0;
        if bits & ROAD_Y == 0 {
            AXIS_X
        } else if bits & ROAD_X == 0 {
            AXIS_Y
        } else {
            INVALID_AXIS
        }
    }
    fn rename_waypoint(&self, out: Cost<'_>, flags: u16, id: u16, text: *const u8, length: usize) {
        w::hit(w::WAYPOINT_RENAME);
        let st = self.waypoint(id);
        if st.is_null() {
            return out.error(INVALID_STRING_ID);
        }
        let owner = (self.world().station_owner)(st);
        if owner != OWNER_NONE {
            unsafe { (self.command.ownership)(out.0, owner, 0) };
            if !out.ok() {
                return;
            }
        }
        if length != 0 {
            if (self.shared.utf8_length)(text, length) >= MAX_LENGTH_STATION_NAME_CHARS {
                return out.error(INVALID_STRING_ID);
            }
            for index in 0..(self.world().pool_size)() {
                let candidate = self.waypoint(index as u16);
                if !candidate.is_null() && (self.s.name_equals)(candidate, text, length) {
                    return out.error(self.s.strings.name_unique);
                }
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            if length == 0 {
                (self.s.clear_name)(st);
            } else {
                (self.s.set_name)(st, text, length);
            }
            (self.s.virtual_coord)(st);
        }
        out.zero();
    }
    fn move_waypoint_name(&self, out: Cost<'_>, flags: u16, id: u16, tile: u32) -> u16 {
        w::hit(w::WAYPOINT_MOVE_NAME);
        let st = self.waypoint(id);
        if st.is_null() {
            out.error(INVALID_STRING_ID);
            return INVALID_STATION;
        }
        let owner = (self.world().station_owner)(st);
        if owner != OWNER_NONE {
            unsafe { (self.command.ownership)(out.0, owner, 0) };
            if !out.ok() {
                return INVALID_STATION;
            }
        }
        if !(self.s.rect_contains)(st, tile) {
            out.error(self.s.strings.unsuitable);
            return INVALID_STATION;
        }
        let mut search = SignSearch {
            c: self,
            station: st,
            tile,
            other: false,
        };
        unsafe { (self.s.radius)(tile, 0, (&raw mut search).cast(), sign_visit) };
        if search.other {
            out.error(self.s.strings.unsuitable);
            return INVALID_STATION;
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            (self.s.move_sign)(st, tile);
            (self.s.virtual_coord)(st);
        }
        out.zero();
        id
    }
}
/// # Safety
/// The immutable native services and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_rail_axis(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    unsafe { Ctx::new(s) }.waypoint_rail_axis(tile)
}
/// # Safety
/// The immutable native services and serialized map remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_waypoint_road_axis(
    s: *const CommandServicesTable,
    tile: u32,
) -> u8 {
    unsafe { Ctx::new(s) }.waypoint_road_axis(tile)
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
    c.rename_waypoint(c.c(out), flags, id, text, length);
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
    c.move_waypoint_name(c.c(out), flags, id, tile)
}

/// Opaque caller-owned `RailStationTileLayout` and iterator, never dereferenced here.
#[repr(C)]
pub struct WaypointLayout {
    _private: [u8; 0],
}
impl Ctx<'_> {
    fn deleted_waypoint(&self, tile: u32, string: u32, owner: u8, road: bool) -> *mut BaseStation {
        let mut best: *mut BaseStation = std::ptr::null_mut();
        let mut threshold = 8;
        // Pool ID order is Waypoint::Iterate order. Equal distances never replace
        // the earlier candidate, including deleted waypoints outside the kdtree.
        for id in 0..(self.world().pool_size)() {
            let wp = self.waypoint(id as u16);
            if wp.is_null()
                || (self.world().in_use)(wp)
                || (self.s.string_id)(wp) != string
                || (self.world().station_owner)(wp) != owner
                || (self.s.road_waypoint)(wp) != road
            {
                continue;
            }
            let xy = (self.s.location)(wp);
            let distance = self.map().distance_manhattan(tile, xy);
            if distance == threshold && !best.is_null() {
                w::hit(w::WAYPOINT_TIE);
            }
            if distance < threshold {
                threshold = distance;
                best = wp;
            }
        }
        if !best.is_null() {
            w::hit(w::WAYPOINT_REUSE);
        }
        best
    }
    fn valid_waypoint_tile(&self, out: Cost<'_>, tile: u32, axis: u8, waypoint: &mut u16) {
        w::hit(w::WAYPOINT_VALIDATE);
        if (self.shared.tile_type)(tile) == MP_STATION {
            if !crate::station_map::is_rail_waypoint(tile, self.shared) {
                return self.auto_clear(out, tile);
            }
            let wp = (self.shared.station_index)(tile);
            if *waypoint == INVALID_STATION {
                *waypoint = wp;
            } else if *waypoint != wp {
                return out.error(self.s.strings.waypoint_adjoins_multiple);
            }
        }
        if self.waypoint_rail_axis(tile) != axis {
            return out.error(self.rail().strings.no_suitable_track);
        }
        let owner = (self.shared.tile_owner)(tile);
        unsafe { (self.command.ownership)(out.0, owner, 0) };
        if out.ok() {
            unsafe { self.command.no_vehicle(out.0, tile) };
        }
        if !out.ok() {
            return;
        }
        let slope = (self.shared.tile_slope)(tile);
        if slope != SLOPE_FLAT
            && (!(self.shared.build_on_slopes)()
                || is_steep_slope(slope)
                || slope & (0x3 << axis) == 0
                || slope & !(0x3 << axis) == 0)
        {
            return out.error(self.s.strings.flat_land);
        }
        out.zero();
    }
    fn reset_delete_counter(&self, st: *mut BaseStation) {
        let owner = (self.world().owner)(st);
        // SAFETY: #125 owns the sole canonical counter; no borrow survives a callback.
        let fields = unsafe { crate::station_service::openttd_rust_station_service_fields(owner) };
        unsafe { (&raw mut (*fields).delete_ctr).write(0) };
    }
    fn build_rail_waypoint(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        layout: *mut WaypointLayout,
        flags: u16,
        start: u32,
        axis: u8,
        width: u8,
        height: u8,
        class: u16,
        index: u16,
        mut target: u16,
        adjacent: bool,
    ) {
        w::hit(w::WAYPOINT_BUILD_RAIL);
        if !crate::direction_func::is_valid_axis(axis) {
            return cost.error(INVALID_STRING_ID);
        }
        if u32::from(class) >= (self.s.rail_class_count)()
            || !(self.s.rail_waypoint_class)(class)
            || u32::from(index) >= (self.s.rail_spec_count)(class)
        {
            return cost.error(INVALID_STRING_ID);
        }
        let count = if axis == AXIS_X { height } else { width };
        if (if axis == AXIS_X { width } else { height }) != 1
            || count == 0
            || count > (self.s.spread)()
        {
            return cost.error(INVALID_STRING_ID);
        }
        let reuse = target != NEW_STATION;
        if !reuse {
            target = INVALID_STATION;
        }
        if target != INVALID_STATION
            && (!(self.s.distant_join)() || self.waypoint(target).is_null())
        {
            return cost.error(INVALID_STRING_ID);
        }
        let mut area = TileArea {
            tile: start,
            w: u16::from(width),
            h: u16::from(height),
        };
        cost.set(0);
        for tile in area.tiles(self.map()) {
            if !crate::station_map::is_rail_waypoint_tile(tile, self.shared) {
                cost.add_money(crate::company::price(PR_BUILD_WAYPOINT_RAIL));
            }
        }
        let mut existing = INVALID_STATION;
        let spec = (self.s.class_rail_spec)(class, index);
        (self.s.waypoint_layout_init)(layout, spec, count);
        let offset = self
            .map()
            .offs_by_axis(crate::direction_func::other_axis(axis, |ok| {
                crate::movement::native_assert(self.shared, ok);
            }));
        let mut tile = start;
        for i in 0..count {
            let gfx = (self.s.waypoint_layout_gfx)(layout, i == 0);
            self.valid_waypoint_tile(ret, tile, axis, &mut existing);
            if !ret.ok() {
                return cost.take(ret);
            }
            self.rail_bridge(ret, tile, spec, RAIL_WAYPOINT, gfx.wrapping_add(axis));
            if !ret.ok() {
                return cost.take(ret);
            }
            tile = tile.wrapping_add(offset);
        }
        let mut wp = self.join(
            ret,
            existing,
            target,
            adjacent,
            area,
            true,
            false,
            self.s.strings.remove_rail_waypoint,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        let center = start.wrapping_add(u32::from(count / 2).wrapping_mul(offset));
        if wp.is_null() && reuse {
            wp = self.deleted_waypoint(
                center,
                self.s.strings.waypoint_name,
                (self.shared.current_company)(),
                false,
            );
        }
        if !wp.is_null() {
            if (self.world().station_owner)(wp) != (self.shared.current_company)() {
                return cost.error(self.s.strings.waypoint_close);
            }
            if unsafe { (self.s.train_area)(wp) }.tile != INVALID_TILE {
                area = self.expand_rail(ret, wp, area);
                if !ret.ok() {
                    return cost.take(ret);
                }
            }
            (self.s.before_add_rect)(
                ret.0,
                wp,
                TileArea {
                    tile: start,
                    w: u16::from(width),
                    h: u16::from(height),
                },
                ADD_TEST,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
        } else if !(self.s.waypoint_available)() {
            return cost.error(self.s.strings.too_many_stations);
        }
        let specindex = (self.s.allocate_rail_spec)(spec, wp);
        if !specindex.valid {
            return cost.error(self.s.strings.too_many_specs);
        }
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(start);
        } else if !(self.world().in_use)(wp) {
            (self.s.set_location)(wp, start);
        }
        let owner = (self.shared.tile_owner)(start);
        (self.s.set_station_owner)(wp, owner);
        (self.s.before_add_rect)(
            ret.0,
            wp,
            TileArea {
                tile: start,
                w: u16::from(width),
                h: u16::from(height),
            },
            ADD_TRY,
        );
        (self.s.assign_rail_spec)(spec, wp, specindex.value);
        self.reset_delete_counter(wp);
        (self.s.set_facility_bits)(wp, FACILITY_TRAIN);
        (self.s.set_build_date)(wp, (self.shared.calendar_date)());
        (self.s.set_string_id)(wp, self.s.strings.waypoint_name);
        (self.s.set_train_area)(wp, area);
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        (self.s.virtual_coord)(wp);
        let id = (self.links().graphs.station_id)(wp);
        let mut tile = start;
        for i in 0..count {
            let gfx = (self.s.waypoint_layout_gfx)(layout, i == 0);
            let old =
                if crate::station_map::has_station_tile_rail(tile, self.shared, self.rail_map()) {
                    (self.s.custom_rail_index)(tile) as u8
                } else {
                    0
                };
            if !crate::station_map::has_station_tile_rail(tile, self.shared, self.rail_map()) {
                (self.s.station_infrastructure)(owner, 1);
            }
            let track = crate::track_func::axis_to_track(crate::services::Axis(axis), |ok| {
                crate::movement::native_assert(self.shared, ok);
            })
            .0;
            let reserved = if (self.shared.tile_type)(tile) == MP_RAILWAY {
                crate::bitmath_func::has_bit((self.rail_map().rail_reservation)(tile).0, track)
            } else {
                (self.s.reserved)(tile)
            };
            (self.s.make_rail_waypoint)(
                tile,
                owner,
                id,
                axis,
                gfx,
                (self.rail_map().rail_type)(tile),
            );
            (self.s.set_custom_rail_index)(tile, specindex.value);
            self.set_rail_flags(tile, spec);
            (self.s.set_rail_station_reservation)(tile, reserved);
            (self.shared.mark_dirty)(tile);
            (self.s.deallocate_rail_spec)(wp, u32::from(old));
            if spec.is_null() {
                (self.s.close_station_inspect)(tile);
            }
            (self.rail().layout_changed)(tile, track);
            tile = tile.wrapping_add(offset);
        }
        (self.command.infrastructure_dirty)(owner);
    }
    fn build_road_waypoint(
        &self,
        cost: Cost<'_>,
        ret: Cost<'_>,
        flags: u16,
        start: u32,
        axis: u8,
        width: u8,
        height: u8,
        class: u16,
        index: u16,
        mut target: u16,
        adjacent: bool,
    ) {
        w::hit(w::WAYPOINT_BUILD_ROAD);
        if !crate::direction_func::is_valid_axis(axis) {
            return cost.error(INVALID_STRING_ID);
        }
        if u32::from(class) >= (self.s.road_class_count)()
            || !(self.s.road_waypoint_class)(class)
            || u32::from(index) >= (self.s.road_spec_count)(class)
        {
            return cost.error(INVALID_STRING_ID);
        }
        let spec = (self.s.class_road_spec)(class, index);
        let count = if axis == AXIS_X { height } else { width };
        if (if axis == AXIS_X { width } else { height }) != 1
            || count == 0
            || count > (self.s.spread)()
        {
            return cost.error(INVALID_STRING_ID);
        }
        let reuse = target != NEW_STATION;
        if !reuse {
            target = INVALID_STATION;
        }
        if target != INVALID_STATION
            && (!(self.s.distant_join)() || self.waypoint(target).is_null())
        {
            return cost.error(INVALID_STRING_ID);
        }
        let area = TileArea {
            tile: start,
            w: u16::from(width),
            h: u16::from(height),
        };
        let unit = if spec.is_null() {
            crate::company::price(PR_BUILD_STATION_TRUCK)
        } else {
            let factor = (self.s.road_spec_build_cost)(spec, PR_BUILD_STATION_TRUCK as u8);
            crate::economy_func::get_price(
                PR_BUILD_STATION_TRUCK as u32,
                u32::from(factor.multiplier),
                factor.shift,
                -4,
            )
        };
        let mut existing = INVALID_STATION;
        self.road_cost(
            cost,
            ret,
            area,
            flags,
            true,
            ROAD_WAYPOINT,
            spec,
            axis,
            crate::direction_func::axis_to_diag_dir(axis, |ok| {
                crate::movement::native_assert(self.shared, ok);
            }),
            &mut existing,
            INVALID_ROADTYPE,
            unit,
        );
        if !cost.ok() {
            return;
        }
        let mut wp = self.join(
            ret,
            existing,
            target,
            adjacent,
            area,
            true,
            true,
            self.s.strings.remove_road_waypoint,
        );
        if !ret.ok() {
            return cost.take(ret);
        }
        let center =
            start.wrapping_add(u32::from(count / 2).wrapping_mul(self.map().offs_by_axis(
                crate::direction_func::other_axis(axis, |ok| {
                    crate::movement::native_assert(self.shared, ok);
                }),
            )));
        if wp.is_null() && reuse {
            wp = self.deleted_waypoint(
                center,
                self.s.strings.waypoint_name,
                (self.shared.current_company)(),
                true,
            );
        }
        if !wp.is_null() {
            if !(self.s.road_waypoint)(wp) {
                return cost.error(INVALID_STRING_ID);
            }
            if (self.world().station_owner)(wp) != (self.shared.current_company)() {
                return cost.error(self.s.strings.waypoint_close);
            }
            (self.s.before_add_rect)(ret.0, wp, area, ADD_TEST);
            if !ret.ok() {
                return cost.take(ret);
            }
        } else if !(self.s.waypoint_available)() {
            return cost.error(self.s.strings.too_many_stations);
        }
        let index = unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_allocate(
                spec,
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
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(start);
            (self.s.set_road_waypoint)(wp);
        } else if !(self.world().in_use)(wp) {
            (self.s.set_location)(wp, start);
        }
        let owner = (self.shared.current_company)();
        (self.s.set_station_owner)(wp, owner);
        (self.s.before_add_rect)(ret.0, wp, area, ADD_TRY);
        unsafe {
            crate::newgrf_roadstop::openttd_rust_roadstop_assign(
                spec,
                wp,
                specindex.value,
                self.s.roadstop_resolver,
            );
        }
        if !spec.is_null() {
            (self.s.road_triggers)(wp, spec);
        }
        self.reset_delete_counter(wp);
        (self.s.set_facility_bits)(wp, FACILITY_BUS | FACILITY_TRUCK);
        (self.s.set_build_date)(wp, (self.shared.calendar_date)());
        (self.s.set_string_id)(wp, self.s.strings.waypoint_name);
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        (self.s.virtual_coord)(wp);
        let id = (self.links().graphs.station_id)(wp);
        for tile in area.tiles(self.map()) {
            let road = if self.may_have_road(tile) {
                self.road_type(tile, false)
            } else {
                INVALID_ROADTYPE
            };
            let tram = if self.may_have_road(tile) {
                self.road_type(tile, true)
            } else {
                INVALID_ROADTYPE
            };
            let road_owner = if road != INVALID_ROADTYPE {
                self.road_owner(tile, false)
            } else {
                owner
            };
            let tram_owner = if tram != INVALID_ROADTYPE {
                self.road_owner(tile, true)
            } else {
                owner
            };
            if crate::station_map::is_road_waypoint_tile(tile, self.shared) {
                self.remove_road_tile(ret, ret, tile, flags, i32::from(specindex.value), true);
            }
            (self.s.road_waypoint_area_add)(wp, tile);
            (self.s.before_add_tile)(ret.0, wp, tile, ADD_TRY);
            let update = |rt, company, count| unsafe {
                crate::road_cmd::update_company_road_infrastructure(
                    (self.shared.road_commands)(),
                    rt,
                    company,
                    count,
                );
            };
            if (self.shared.is_normal_road)(tile) {
                update(
                    road,
                    road_owner,
                    -((self.shared.normal_road_bits)(tile, 0).0.count_ones() as i32),
                );
                update(
                    tram,
                    tram_owner,
                    -((self.shared.normal_road_bits)(tile, 1).0.count_ones() as i32),
                );
            }
            update(road, road_owner, ROAD_STOP_TRACKBIT_FACTOR as i32);
            update(tram, tram_owner, ROAD_STOP_TRACKBIT_FACTOR as i32);
            (self.s.make_drive_stop)(
                tile,
                owner,
                road_owner,
                tram_owner,
                id,
                ROAD_WAYPOINT,
                road,
                tram,
                axis,
            );
            (self.s.set_custom_road_index)(tile, specindex.value);
            if !spec.is_null() {
                (self.s.stop_random_bits)(wp, tile, 0);
            }
            (self.s.station_infrastructure)(owner, 1);
            (self.shared.mark_dirty)(tile);
        }
        (self.command.infrastructure_dirty)(owner);
    }
    fn build_buoy(&self, cost: Cost<'_>, ret: Cost<'_>, flags: u16, tile: u32) {
        w::hit(w::WAYPOINT_BUILD_BUOY);
        if tile == 0 || !(self.shared.has_water_ground)(tile) {
            return cost.error(self.s.strings.unsuitable);
        }
        self.default_bridge_ok(ret, tile, BUOY, 0);
        if !ret.ok() {
            return cost.take(ret);
        }
        if !(self.s.tile_flat)(tile) {
            return cost.error(self.s.strings.unsuitable);
        }
        let mut wp = self.deleted_waypoint(tile, self.s.strings.buoy_name, OWNER_NONE, false);
        if wp.is_null() && !(self.s.waypoint_available)() {
            return cost.error(self.s.strings.too_many_stations);
        }
        cost.set(crate::company::price(PR_BUILD_WAYPOINT_BUOY));
        if !(self.shared.is_water_tile)(tile) {
            crate::landscape_cmd::clear_command(
                self.command,
                ret.0,
                tile,
                flags | DoCommandFlag::AUTO,
            );
            if !ret.ok() {
                return cost.take(ret);
            }
            cost.add_money(ret.money());
        }
        if flags & DoCommandFlag::EXECUTE == 0 {
            return;
        }
        if wp.is_null() {
            wp = (self.s.new_waypoint)(tile);
        } else {
            (self.s.set_location)(wp, tile);
            (self.s.waypoint_view_invalidate)(wp);
        }
        (self.s.before_add_tile)(ret.0, wp, tile, ADD_TRY);
        (self.s.set_string_id)(wp, self.s.strings.buoy_name);
        (self.s.set_facility_bits)(wp, FACILITY_DOCK);
        (self.s.set_station_owner)(wp, OWNER_NONE);
        (self.s.set_build_date)(wp, (self.shared.calendar_date)());
        if (self.s.station_town)(wp).is_null() {
            (self.s.waypoint_default_name)(wp);
        }
        let clear = unsafe { &*self.s.clear };
        (self.s.make_buoy)(
            tile,
            (self.links().graphs.station_id)(wp),
            (clear.water_class)(tile),
        );
        self.check_docking(tile);
        (self.shared.mark_dirty)(tile);
        unsafe {
            crate::water::openttd_rust_clear_neighbour_flooding(
                tile,
                self.map().size_x(),
                (self.shared.map_max_y)() + 1,
                (self.shared.water_services)(),
                self.s.shared,
            );
        }
        (self.s.virtual_coord)(wp);
        (self.s.waypoint_view_invalidate)(wp);
    }
    pub(super) fn remove_buoy(&self, cost: Cost<'_>, tile: u32, flags: u16) {
        w::hit(w::WAYPOINT_REMOVE_BUOY);
        if !(self.shared.company_valid)((self.shared.current_company)())
            && flags & DoCommandFlag::BANKRUPT == 0
        {
            return cost.error(INVALID_STRING_ID);
        }
        let wp = (self.world().get)((self.shared.station_index)(tile));
        if self.in_orders(
            (self.links().graphs.station_id)(wp),
            false,
            (self.shared.current_company)(),
        ) {
            return cost.error(self.s.strings.buoy_in_use);
        }
        if flags & DoCommandFlag::BANKRUPT != 0 {
            w::hit(w::WAYPOINT_BANKRUPT);
        }
        if flags & DoCommandFlag::BANKRUPT == 0 {
            unsafe { self.command.no_vehicle(cost.0, tile) };
            if !cost.ok() {
                return;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            (self.s.reset_facilities)(wp, FACILITY_DOCK);
            (self.s.waypoint_view_invalidate)(wp);
            unsafe {
                crate::water::openttd_rust_water_restore(
                    tile,
                    (self.shared.tile_owner)(tile),
                    self.s.shared,
                    (self.shared.water_services)(),
                    self.s.restoration,
                );
            }
            (self.s.after_remove_tile)(wp, tile);
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
    c.build_rail_waypoint(
        c.c(cost),
        c.c(ret),
        layout,
        flags,
        start,
        axis,
        width,
        height,
        class,
        index,
        target,
        adjacent,
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
    c.build_road_waypoint(
        c.c(cost),
        c.c(ret),
        flags,
        start,
        axis,
        width,
        height,
        class,
        index,
        target,
        adjacent,
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
    c.build_buoy(c.c(cost), c.c(ret), flags, tile);
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete waypoint command policy; shared station operations are called directly.
//! Native pointers are opaque, synchronous and serialized by the game-state mutex.
use super::{CommandCost, CommandServicesTable, Cost, Ctx, SignSearch, sign_visit};
use crate::direction_type::{AXIS_X, AXIS_Y, INVALID_AXIS};
use crate::road_type::{ROAD_X, ROAD_Y};

impl Ctx<'_> {
    fn rail_waypoint(&self, tile: u32) -> bool {
        (self.shared.tile_type)(tile) == 5 && (self.shared.station_type)(tile).0 == 7
    }
    fn road_waypoint_tile(&self, tile: u32) -> bool {
        (self.shared.tile_type)(tile) == 5 && (self.shared.station_type)(tile).0 == 8
    }
    fn waypoint_rail_axis(&self, tile: u32) -> u8 {
        if self.rail_waypoint(tile) {
            return (self.s.rail_axis)(tile).0;
        }
        if (self.shared.tile_type)(tile) != 1 || (self.rail_map().rail_tile_type)(tile).0 != 0 {
            return INVALID_AXIS;
        }
        let ground = unsafe { &*self.rail_map().ground };
        match (ground.tracks)(tile).0 {
            1 => AXIS_X,
            2 => AXIS_Y,
            _ => INVALID_AXIS,
        }
    }
    fn waypoint_road_axis(&self, tile: u32) -> u8 {
        if self.road_waypoint_tile(tile) {
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
        let st = self.waypoint(id);
        if st.is_null() {
            return out.error(0xffff);
        }
        let owner = (self.world().station_owner)(st);
        if owner != 0x10 {
            unsafe { (self.command.ownership)(out.0, owner, 0) };
            if !out.ok() {
                return;
            }
        }
        if length != 0 {
            if (self.shared.utf8_length)(text, length) >= 32 {
                return out.error(0xffff);
            }
            for index in 0..(self.world().pool_size)() {
                let candidate = self.waypoint(index as u16);
                if !candidate.is_null() && (self.s.name_equals)(candidate, text, length) {
                    return out.error(self.s.strings.name_unique);
                }
            }
        }
        if flags & 1 != 0 {
            (self.s.set_name)(st, text, length);
            (self.s.virtual_coord)(st);
        }
        out.zero();
    }
    fn move_waypoint_name(&self, out: Cost<'_>, flags: u16, id: u16, tile: u32) -> u16 {
        let st = self.waypoint(id);
        if st.is_null() {
            out.error(0xffff);
            return 0xffff;
        }
        let owner = (self.world().station_owner)(st);
        if owner != 0x10 {
            unsafe { (self.command.ownership)(out.0, owner, 0) };
            if !out.ok() {
                return 0xffff;
            }
        }
        if !(self.s.rect_contains)(st, tile) {
            out.error(self.s.strings.unsuitable);
            return 0xffff;
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
            return 0xffff;
        }
        if flags & 1 != 0 {
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

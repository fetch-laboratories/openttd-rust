/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete PBS policy over synchronous typed native leaves. No world reference
//! survives a call; map arrays and native shells remain canonical. Panics abort.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::collapsible_match
)]
use crate::services::{RailMapServices, Services};
#[repr(C)]
pub struct PbsServices {
    pub shared: *const Services,
    pub rail: *const RailMapServices,
    pub train: *const crate::train::Leaves,
    pub depot_bits: extern "C" fn(u32) -> u8,
    pub crossing_bits: extern "C" fn(u32) -> u8,
    pub station_bits: extern "C" fn(u32) -> u8,
    pub wormhole_bits: extern "C" fn(u32) -> u8,
    pub set_depot: extern "C" fn(u32, bool) -> (),
    pub set_station: extern "C" fn(u32, bool) -> (),
    pub set_wormhole: extern "C" fn(u32, bool) -> (),
    pub try_track: extern "C" fn(u32, u8) -> bool,
    pub unreserve_track: extern "C" fn(u32, u8) -> (),
    pub station_reserved: extern "C" fn(u32) -> bool,
    pub compatible_station: extern "C" fn(u32, u32) -> bool,
    pub station_axis: extern "C" fn(u32) -> u8,
    pub show_reservation: extern "C" fn() -> bool,
    pub assert_track: extern "C" fn(u32, u8) -> (),
    pub assert_platform: extern "C" fn(u32, u8) -> (),
    pub station: extern "C" fn(u32) -> *mut std::ffi::c_void,
    pub randomisation: extern "C" fn(*mut std::ffi::c_void, u32) -> (),
    pub animation: extern "C" fn(*mut std::ffi::c_void, u32) -> (),
}
#[derive(Clone, Copy)]
pub(crate) struct Pbs<'a>(pub &'a PbsServices);
impl Pbs<'_> {
    fn shared(self) -> &'static Services {
        // SAFETY: tables have process lifetime; they contain operations, no world borrows.
        unsafe { &*self.0.shared }
    }
    fn rail(self) -> &'static RailMapServices {
        // SAFETY: same table contract.
        unsafe { &*self.0.rail }
    }
    pub(crate) fn reserved(self, tile: u32) -> u8 {
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            1 => {
                if (r.is_rail_depot)(tile) {
                    return (self.0.depot_bits)(tile);
                }
                if (s.is_plain_rail)(tile) {
                    return (r.rail_reservation)(tile).0;
                }
            }
            2 => {
                if (s.is_level_crossing)(tile) {
                    return (self.0.crossing_bits)(tile);
                }
            }
            5 => {
                if (r.has_station_rail)(tile) {
                    return (self.0.station_bits)(tile);
                }
            }
            9 => {
                if (s.tunnel_transport)(tile) == 0 {
                    return (self.0.wormhole_bits)(tile);
                }
            }
            _ => {}
        }
        0
    }
    pub(crate) fn platform(self, start: u32, dir: u8, reserved: bool) {
        (self.0.assert_platform)(start, dir);
        let map = crate::map_func::Map {
            log_x: (self.shared().map_log_x)() as u8,
        };
        let diff = map.offs_by_diag_dir(dir);
        let mut tile = start;
        loop {
            (self.0.set_station)(tile, reserved);
            (self.shared().mark_dirty)(tile);
            tile = tile.wrapping_add(diff);
            if !(self.0.compatible_station)(tile, start) {
                break;
            }
        }
    }
    fn dirty(self, tile: u32) {
        if (self.0.show_reservation)() {
            if (self.shared().is_bridge)(tile) {
                (self.shared().mark_bridge_dirty)(tile);
            } else {
                (self.shared().mark_dirty)(tile);
            }
        }
    }
    fn crossing(self, tile: u32, sound: bool) {
        // SAFETY: synchronous integrated owner, static service tables. No owner borrow crosses it.
        unsafe {
            crate::train::openttd_rust_train_update_crossing(
                tile,
                u8::from(sound),
                0,
                self.0.train,
                self.0.shared,
            );
        }
    }
    pub(crate) fn try_reserve(self, tile: u32, track: u8, trigger: bool) -> bool {
        (self.0.assert_track)(tile, track);
        self.dirty(tile);
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            1 => {
                if (s.is_plain_rail)(tile) {
                    return (self.0.try_track)(tile, track);
                }
                if (r.is_rail_depot)(tile) && !(r.depot_reserved)(tile) {
                    (self.0.set_depot)(tile, true);
                    (s.mark_dirty)(tile);
                    return true;
                }
            }
            2 => {
                if (s.is_level_crossing)(tile) && !(s.crossing_reserved)(tile) {
                    (s.set_crossing_reserved)(tile, true);
                    self.crossing(tile, false);
                    return true;
                }
            }
            5 => {
                if (r.has_station_rail)(tile) && !(self.0.station_reserved)(tile) {
                    (self.0.set_station)(tile, true);
                    if trigger {
                        let station = (self.0.station)(tile);
                        (self.0.randomisation)(station, tile);
                        (self.0.animation)(station, tile);
                    }
                    (s.mark_dirty)(tile);
                    return true;
                }
            }
            9 => {
                if (s.tunnel_transport)(tile) == 0 && (self.0.wormhole_bits)(tile) == 0 {
                    (self.0.set_wormhole)(tile, true);
                    return true;
                }
            }
            _ => {}
        }
        false
    }
    pub(crate) fn unreserve(self, tile: u32, track: u8) {
        (self.0.assert_track)(tile, track);
        self.dirty(tile);
        let s = self.shared();
        let r = self.rail();
        match (s.tile_type)(tile) {
            1 => {
                if (r.is_rail_depot)(tile) {
                    (self.0.set_depot)(tile, false);
                    (s.mark_dirty)(tile);
                } else if (s.is_plain_rail)(tile) {
                    (self.0.unreserve_track)(tile, track);
                }
            }
            2 => {
                if (s.is_level_crossing)(tile) {
                    (s.set_crossing_reserved)(tile, false);
                    self.crossing(tile, true);
                }
            }
            5 => {
                if (r.has_station_rail)(tile) {
                    (self.0.set_station)(tile, false);
                    (s.mark_dirty)(tile);
                }
            }
            9 => {
                if (s.tunnel_transport)(tile) == 0 {
                    (self.0.set_wormhole)(tile, false);
                }
            }
            _ => {}
        }
    }
}
/// # Safety
/// The service tables and native world remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_reserved(s: *const PbsServices, tile: u32) -> u8 {
    Pbs(unsafe { &*s }).reserved(tile)
}
/// # Safety
/// Same synchronous service/world contract; platform has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_platform(
    s: *const PbsServices,
    tile: u32,
    dir: u8,
    b: bool,
) {
    Pbs(unsafe { &*s }).platform(tile, dir, b);
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_try_reserve(
    s: *const PbsServices,
    tile: u32,
    track: u8,
    trigger: bool,
) -> bool {
    Pbs(unsafe { &*s }).try_reserve(tile, track, trigger)
}
/// # Safety
/// Same synchronous service/world contract; track has the source preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pbs_unreserve(s: *const PbsServices, tile: u32, track: u8) {
    Pbs(unsafe { &*s }).unreserve(tile, track);
}

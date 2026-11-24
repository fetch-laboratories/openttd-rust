/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared vehicle boundary: one canonical record handle (valid until its shell is
//! deleted), the base-`Vehicle` and `GroundVehicle` service tables and constants. The tables
//! are declared in `src/rust/vehicle_ffi.h` and filled in `src/rust/vehicle_services.cpp`.
//! Callers end every owner borrow before a service that can reenter Rust.
pub(crate) mod maintenance;
pub(crate) mod occupancy;
pub(crate) mod scheduler;
pub(crate) mod visits;

use crate::newgrf_callbacks::CallbackID;
use crate::newgrf_sound::VehicleSoundEvent;
use crate::timer_game_common::DAYS_IN_YEAR;
use crate::timer_game_tick::DAY_TICKS;
use std::ptr::NonNull;

/// Shared Rust-owned record of a C++ `Vehicle` shell.
pub use crate::vehicle_type::OpenTTDVehicle;

/// Native `VehicleType : uint8_t`. A transparent carrier preserves every byte,
/// including sentinels and unnamed values, without Rust enum validity constraints.
pub use crate::vehicle_type::VehicleType;

/// Compatibility import of the original header type.
pub use crate::order_type::OrderType;

/// Compatibility import of the original header type.
pub use crate::engine_type::RailVehicleTypes;

/// Non-null live vehicle handle; `Option<VehicleRef>` crosses as a nullable pointer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VehicleRef(pub NonNull<OpenTTDVehicle>);

// A VehicleRef and its record are live and serialized at every field access.
#[allow(unsafe_code)]
impl VehicleRef {
    /// Native shell identity for legacy opaque contexts.
    pub(crate) fn shell(self) -> *mut std::ffi::c_void {
        crate::raw_field::read_field!(self.0.as_ptr(), shell).cast()
    }

    /// A live, non-null canonical record pointer.
    pub(crate) fn of(record: *mut OpenTTDVehicle) -> Self {
        Self(NonNull::new(record).unwrap())
    }

    /// Convert a nullable canonical record to a legacy native context.
    pub(crate) fn shell_from_record(record: *mut OpenTTDVehicle) -> *mut () {
        NonNull::new(record).map_or(std::ptr::null_mut(), |record| Self(record).shell().cast())
    }

    /// Resolve a legacy native shell under the caller's original lifetime rules.
    ///
    /// # Safety
    /// `shell` is a live non-null vehicle shell for this serialized entry.
    pub(crate) unsafe fn from_shell(
        shell: *mut std::ffi::c_void,
        services: &VehicleServices,
    ) -> Self {
        Self(unsafe { NonNull::new_unchecked((services.record_from_shell)(shell)) })
    }
}

/// A live canonical Engine record captured under the serialized game lifetime.
pub use crate::engine_type::OpenTTDEngine;
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EngineRef(pub NonNull<OpenTTDEngine>);

pub use crate::newgrf_callbacks::CALLBACK_FAILED;
pub use crate::tile_type::MP_STATION;
/// `VEHICLE_LENGTH`.
#[allow(clippy::cast_possible_truncation)] // Native uint constant is 8; retain the u8 compatibility surface.
pub const VEHICLE_LENGTH: u8 = crate::vehicle_type::VEHICLE_LENGTH as u8;
/// `CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS`.
pub const YEAR_TICKS: i64 = DAYS_IN_YEAR as i64 * DAY_TICKS as i64;

/// `GetNewVehiclePos` result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NewPosition {
    pub x: i32,
    pub y: i32,
    pub tile: u32,
}

/// `HandleBreakdown` result, then the vehicle status it leaves.
#[derive(Clone, Copy)]
pub struct Breakdown {
    pub broken: bool,
    pub status: u8,
}

/// Base `Vehicle` operations and writes shared by every vehicle type.
#[repr(C)]
pub struct VehicleServices {
    /// Resolve a legacy opaque native shell to its canonical record.
    pub record_from_shell: extern "C" fn(*mut std::ffi::c_void) -> *mut OpenTTDVehicle,
    pub tile_links: extern "C" fn(VehicleRef) -> *mut occupancy::Links,
    pub map_log_x: extern "C" fn() -> u32,
    pub assertions: bool,
    pub train_handle: extern "C" fn(*mut OpenTTDVehicle) -> crate::train::Handle,
    /// `VehicleEnterDepot`, owned by `visits` (`openttd_rust_visit_enter_depot`).
    pub enter_depot: unsafe extern "C" fn(VehicleRef, *const visits::Services),
    pub pathfinding_result: extern "C" fn(VehicleRef, bool),
    pub new_position: extern "C" fn(VehicleRef) -> NewPosition,
    /// `Vehicle::UpdateViewport(true)`.
    pub base_viewport: extern "C" fn(VehicleRef),
    /// Immutable process-wide vehicle resolver services.
    pub newgrf: extern "C" fn() -> *const crate::newgrf_vehicle::Services,
    /// Direct Rust `GetVehicleProperty(v, property, fallback)` entry.
    pub property: extern "C" fn(*const crate::newgrf_vehicle::Services, VehicleRef, u8, i32) -> i32,
    /// `GetVehicleCallback(callback, 0, 0, v->engine_type, v)`.
    pub callback:
        extern "C" fn(*const crate::newgrf_vehicle::Services, VehicleRef, CallbackID) -> u16,
    /// Native `max_age` store after the Rust engine lifetime decision.
    pub set_max_age: extern "C" fn(VehicleRef, i32),
    pub length_changed: extern "C" fn(VehicleRef),
    /// `PlayVehicleSound(v, event)`.
    pub play_vehicle_sound: extern "C" fn(VehicleRef, VehicleSoundEvent) -> bool,
    pub play_sound: extern "C" fn(VehicleRef, u16),
    pub start_stop_dirty: extern "C" fn(VehicleRef),
    /// `delete v`; the handle and its owner are dead afterwards.
    pub destroy: extern "C" fn(VehicleRef),
    /// Original virtual `SetDestTile`; may reenter the vehicle owner.
    pub set_dest_tile: extern "C" fn(VehicleRef, u32),
    /// Only the current engine, for callers that do not consume its GRF version.
    pub engine: extern "C" fn(VehicleRef) -> EngineRef,
    pub write_profit: extern "C" fn(VehicleRef, i64),
    pub write_economy_service: extern "C" fn(VehicleRef, i32),
    pub write_calendar_service: extern "C" fn(VehicleRef, i32),
    pub write_breakdowns: extern "C" fn(VehicleRef, u8),
    pub write_reliability: extern "C" fn(VehicleRef, u16),
    pub profit: extern "C" fn(VehicleRef) -> i64,
    pub maintenance: *const maintenance::Services,
    /// The static station and depot visit table (`vehicle/visits.rs`), resolved on use.
    pub visits: extern "C" fn() -> *const visits::Services,
    /// The same canonical orders table used by visits/refit.
    pub orders: extern "C" fn() -> *const crate::orders::Leaves,
    /// Strict `Ship::Get`, never a nullable pool lookup.
    pub ship_get: extern "C" fn(u32) -> VehicleRef,
    pub first_sprite: extern "C" fn(VehicleRef) -> u32,
    /// Virtual `UpdateDeltaXY()`.
    pub update_delta: extern "C" fn(VehicleRef),
    /// `Vehicle::CanAllocateItem(count)`.
    pub can_allocate: extern "C" fn(usize) -> bool,
}
// The visit table and its entries are native-supplied static Rust functions.
#[allow(unsafe_code)]
impl VehicleServices {
    /// `Vehicle::SetNext` for legacy native shell contexts.
    ///
    /// # Safety
    /// `shell` and non-null `next` and their linked records remain live.
    pub(crate) unsafe fn set_next_shell(
        &self,
        shell: *mut std::ffi::c_void,
        next: *mut std::ffi::c_void,
    ) {
        let record = (self.record_from_shell)(shell);
        let next = (self.record_from_shell)(next);
        // SAFETY: The caller preserves both resolved records.
        unsafe { crate::vehicle_base::set_next(record, next) };
    }

    /// The static station/depot visit table.
    pub(crate) fn visits(&self) -> &'static visits::Services {
        // SAFETY: the getter returns the immutable process-wide visit table.
        unsafe { &*(self.visits)() }
    }
    /// The immutable canonical orders services, shared by every controller.
    pub(crate) fn orders(&self) -> &'static crate::orders::Leaves {
        // SAFETY: the getter returns the immutable process-wide orders table.
        unsafe { &*(self.orders)() }
    }
    /// `VehicleEnterDepot(v)`.
    pub(crate) fn enter_depot(&self, v: VehicleRef) {
        // SAFETY: a live front vehicle and the static visit table.
        unsafe { (self.enter_depot)(v, (self.visits)()) }
    }
    /// `v->HandleLoading(mode)`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn handle_loading(
        &self,
        v: VehicleRef,
        mode: bool,
        orders: &'static crate::orders::Leaves,
    ) {
        let kind = crate::orders::VisitOrders::new(orders).current(v).kind();
        if matches!(
            kind,
            crate::order_type::OT_LOADING | crate::order_type::OT_DUMMY
        ) {
            visits::handle_loading(v, mode, self.visits());
        }
    }
}

/// `Vehicle::IsWaitingForUnbunching`: check whether a vehicle inside a depot is waiting
/// for unbunching. True if the vehicle must continue waiting, or false if it may try to
/// leave the depot. `is_in_depot` is the vehicle type's `IsInDepot()`.
pub(crate) fn is_waiting_for_unbunching(
    v: VehicleRef,
    orders: &'static crate::orders::Leaves,
    is_in_depot: impl FnOnce() -> bool,
) -> bool {
    crate::services_assert::assert_with(is_in_depot);
    crate::orders::VisitOrders::new(orders).is_waiting_for_unbunching(v)
}

/// `GroundVehicle<T>` members, one C++ template instance per vehicle type.
#[repr(C)]
pub struct GroundServices {
    pub physics: &'static crate::ground_vehicle::Services,
}

/// `GetAdvanceDistance()`.
#[must_use]
pub fn advance_distance(direction: u8) -> i32 {
    if direction & 1 != 0 { 192 } else { 256 }
}

impl VehicleServices {
    /// Visit the live hash chain in source order. The callback may reenter owners
    /// but cannot delete the current shell. No owner reference spans a callback.
    #[allow(unsafe_code)]
    pub(crate) fn for_each_on_tile<F: FnMut(VehicleRef)>(&self, tile: u32, mut visit: F) {
        // SAFETY: Caller preserves the current shell; no index borrow spans visit.
        unsafe {
            let mut it = occupancy::OnTile::new(self, tile);
            while let Some(v) = it.get() {
                visit(v);
                it.advance(self);
            }
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleNewPosition" ["probe"] => NewPosition {
        x,
        y,
        tile,
    };
    "OpenTTDVehicleServices" ["probe"] => VehicleServices {
        record_from_shell,
        tile_links,
        map_log_x,
        assertions,
        train_handle,
        enter_depot,
        pathfinding_result,
        new_position,
        base_viewport,
        newgrf,
        property,
        callback,
        set_max_age,
        length_changed,
        play_vehicle_sound,
        play_sound,
        start_stop_dirty,
        destroy,
        set_dest_tile,
        engine,
        write_profit,
        write_economy_service,
        write_calendar_service,
        write_breakdowns,
        write_reliability,
        profit,
        maintenance,
        visits,
        orders,
        ship_get,
        first_sprite,
        update_delta,
        can_allocate,
    };




    "OpenTTDGroundServices" ["probe"] => GroundServices {
        physics,
    };
    "OrderType" ["maintenance"] => OrderType {
    };
}

/// `Vehicle::GetAdvanceSpeed`, with the source unsigned multiplication.
pub const fn advance_speed(speed: u32) -> u32 {
    speed.wrapping_mul(3) / 4
}

impl VehicleServices {
    /// Live traversal under the same current-shell lifetime contract as the
    /// native iterators. No owner borrow spans a potentially reentrant visitor.
    #[allow(unsafe_code)]
    pub(crate) fn any_on_tile(&self, tile: u32, mut visit: impl FnMut(VehicleRef) -> bool) -> bool {
        unsafe {
            let mut it = occupancy::OnTile::new(self, tile);
            while let Some(v) = it.get() {
                if visit(v) {
                    return true;
                }
                it.advance(self);
            }
        }
        false
    }
    #[allow(unsafe_code)]
    #[inline]
    pub(crate) fn any_near(
        &self,
        x: i32,
        y: i32,
        distance: u32,
        mut visit: impl FnMut(VehicleRef) -> bool,
    ) -> bool {
        unsafe {
            let mut it = occupancy::Near::new(self, x, y, distance);
            while let Some(v) = it.get() {
                if visit(v) {
                    return true;
                }
                it.advance(self);
            }
        }
        false
    }
}

#[allow(unsafe_code)]
impl VehicleServices {
    /// Visit every vehicle near `(x, y)` in hash order, as a `VehiclesNearTileXY` range loop.
    /// The callback may reenter owners but cannot delete the current shell.
    pub(crate) fn for_each_near<F: FnMut(VehicleRef)>(
        &self,
        x: i32,
        y: i32,
        distance: u32,
        mut visit: F,
    ) {
        // SAFETY: Caller preserves the current shell; no index borrow spans visit.
        unsafe {
            let mut it = occupancy::Near::new(self, x, y, distance);
            while let Some(v) = it.get() {
                visit(v);
                it.advance(self);
            }
        }
    }
    pub(crate) fn unknown_callback_result(
        &self,
        engine: EngineRef,
        callback: CallbackID,
        result: u16,
    ) {
        unsafe {
            crate::engine_capabilities::unknown_callback(
                (&*(self.newgrf)()).capabilities,
                engine.0.as_ptr(),
                callback,
                result,
            );
        }
    }

    /// Ship/aircraft running cost; the caller retains its live vehicle shell.
    pub(crate) unsafe fn running_cost(&self, v: VehicleRef) -> i64 {
        let cap = unsafe { (&*(self.newgrf)()).capabilities };
        unsafe {
            crate::engine_capabilities::openttd_rust_engine_vehicle_running_cost(
                cap,
                (self.engine)(v).0.as_ptr(),
                v.0.as_ptr(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direction_type::{DiagDirection, Direction};
    use crate::services::Map;
    use crate::tile_type::TileIndex;
    #[test]
    fn map_offsets_follow_the_original_tables() {
        let map = Map::from_log_x(8);
        assert_eq!(
            map.tile_add_by_diag_dir(
                TileIndex(0x1010),
                DiagDirection::DIAGDIR_NE,
                crate::map_func::native_check
            )
            .0,
            0x100f
        );
        assert_eq!(
            map.tile_add_by_diag_dir(
                TileIndex(0x1010),
                DiagDirection::DIAGDIR_SE,
                crate::map_func::native_check
            )
            .0,
            0x1110
        );
        assert_eq!(
            map.tile_add_by_dir(
                TileIndex(0x1010),
                Direction::DIR_N,
                crate::map_func::native_check
            )
            .0,
            0x0f0f
        );
        assert_eq!(
            map.tile_add_by_dir(
                TileIndex(0x1010),
                Direction::DIR_SW,
                crate::map_func::native_check
            )
            .0,
            0x1011
        );
        assert_eq!(
            map.tile_add_by_dir(
                TileIndex(0),
                Direction::DIR_N,
                crate::map_func::native_check
            )
            .0,
            0_u32.wrapping_sub(257)
        );
        assert_eq!(
            (map.tile_x(TileIndex(0x1234)), map.tile_y(TileIndex(0x1234))),
            (0x34, 0x12)
        );
        assert_eq!(advance_distance(3), 192);
    }
}

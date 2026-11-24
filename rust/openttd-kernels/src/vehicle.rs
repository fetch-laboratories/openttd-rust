/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The shared vehicle boundary: the canonical record and engine handles, the base `Vehicle`
//! and `GroundVehicle` service tables, and `vehicle.cpp`'s `GetNewVehiclePos` and
//! `Vehicle::IsWaitingForUnbunching`. The tables are declared in `src/rust/vehicle_ffi.h`
//! and filled in `src/rust/vehicle_services.cpp`.
//!
//! A handle names a record that stays live until its shell is deleted, and every access to
//! it is serialized by the game-state mutex. Callers end every owner borrow before a service
//! that can reenter Rust.
pub(crate) mod maintenance;
pub(crate) mod occupancy;
pub(crate) mod scheduler;
pub(crate) mod visits;

use crate::gfx_type::SpriteID;
use crate::map_func::Map;
use crate::newgrf_callbacks::CallbackID;
use crate::newgrf_sound::VehicleSoundEvent;
use crate::vehicle_func::GetNewVehiclePosResult;
use std::ptr::NonNull;

// Compatibility names for files no pass has finished; #734's closure removes them.
pub use crate::engine_type::{OpenTTDEngine, RailVehicleTypes};
pub use crate::newgrf_callbacks::CALLBACK_FAILED;
pub use crate::vehicle_type::{OpenTTDVehicle, VehicleType};
/// `VEHICLE_LENGTH`, as the byte its raw users store.
#[allow(clippy::cast_possible_truncation)] // The native uint constant is 8.
pub const VEHICLE_LENGTH: u8 = crate::vehicle_type::VEHICLE_LENGTH as u8;

/// Non-null live vehicle handle; `Option<VehicleRef>` crosses as a nullable pointer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VehicleRef(pub NonNull<OpenTTDVehicle>);

impl VehicleRef {
    /// The record's native shell, the context its untyped services take.
    #[allow(unsafe_code)]
    pub(crate) fn shell(self) -> *mut std::ffi::c_void {
        crate::raw_field::read_field!(self.0.as_ptr(), shell).cast()
    }

    /// The handle of a non-null record.
    pub(crate) fn of(record: *mut OpenTTDVehicle) -> Self {
        // A null record is a broken caller contract, as the native dereference would be.
        Self(NonNull::new(record).unwrap())
    }

    /// The native shell of a record, or null for a null record.
    pub(crate) fn shell_from_record(record: *mut OpenTTDVehicle) -> *mut () {
        NonNull::new(record).map_or(std::ptr::null_mut(), |record| Self(record).shell().cast())
    }

    /// The record of a native shell.
    ///
    /// # Safety
    /// `shell` is a live non-null vehicle shell.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn from_shell(
        shell: *mut std::ffi::c_void,
        services: &VehicleServices,
    ) -> Self {
        Self(unsafe { NonNull::new_unchecked((services.record_from_shell)(shell)) })
    }
}

/// Non-null live engine handle.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EngineRef(pub NonNull<OpenTTDEngine>);

/// Base `Vehicle` operations and writes shared by every vehicle type.
#[repr(C)]
pub struct VehicleServices {
    /// `Vehicle::UpdateViewport(true)`.
    pub base_viewport: extern "C" fn(VehicleRef),
    /// `Vehicle::CanAllocateItem(count)`.
    pub can_allocate: extern "C" fn(usize) -> bool,
    /// `delete v`; the handle and its owner are dead afterwards.
    pub destroy: extern "C" fn(VehicleRef),
    /// Only the current engine, for callers that do not consume its GRF version.
    pub engine: extern "C" fn(VehicleRef) -> EngineRef,
    pub first_sprite: extern "C" fn(VehicleRef) -> u32,
    pub length_changed: extern "C" fn(VehicleRef),
    pub maintenance: *const maintenance::Services,
    /// Immutable process-wide vehicle resolver services.
    pub newgrf: extern "C" fn() -> *const crate::newgrf_vehicle::Services,
    /// The same canonical orders table used by visits/refit.
    pub orders: extern "C" fn() -> *const crate::orders::Leaves,
    pub pathfinding_result: extern "C" fn(VehicleRef, bool),
    /// Virtual `Vehicle::PlayLeaveStationSound()`.
    pub play_leave_station_sound: extern "C" fn(VehicleRef),
    pub play_sound: extern "C" fn(VehicleRef, u16),
    /// `PlayVehicleSound(v, event)`.
    pub play_vehicle_sound: extern "C" fn(VehicleRef, VehicleSoundEvent) -> bool,
    pub profit: extern "C" fn(VehicleRef) -> i64,
    /// The record of a native shell.
    pub record_from_shell: extern "C" fn(*mut std::ffi::c_void) -> *mut OpenTTDVehicle,
    /// `v->cargo_subtype = subtype`.
    pub set_cargo_subtype: extern "C" fn(VehicleRef, u8),
    /// `v->cargo_type = cargo`.
    pub set_cargo_type: extern "C" fn(VehicleRef, u8),
    /// `v->colourmap = colourmap`.
    pub set_colourmap: extern "C" fn(VehicleRef, crate::gfx_type::PaletteID),
    /// Native `max_age` store after the Rust engine lifetime decision.
    pub set_max_age: extern "C" fn(VehicleRef, i32),
    /// Strict `Ship::Get`, never a nullable pool lookup.
    pub ship_get: extern "C" fn(u32) -> VehicleRef,
    /// `ShowNewGrfVehicleError(engine, part1, part2, bug, critical)`.
    pub show_newgrf_vehicle_error: extern "C" fn(u16, u32, u32, crate::newgrf_config::GRFBug, bool),
    pub start_stop_dirty: extern "C" fn(VehicleRef),
    pub tile_links: extern "C" fn(VehicleRef) -> *mut occupancy::Links,
    /// Virtual `UpdateDeltaXY()`.
    pub update_delta: extern "C" fn(VehicleRef),
    /// The static station and depot visit table (`vehicle/visits.rs`), resolved on use.
    pub visits: extern "C" fn() -> *const visits::Services,
    pub write_breakdowns: extern "C" fn(VehicleRef, u8),
    pub write_calendar_service: extern "C" fn(VehicleRef, i32),
    pub write_economy_service: extern "C" fn(VehicleRef, i32),
    /// `sprite_cache.sprite_seq.seq[0].sprite = sprite`.
    pub write_first_sprite: extern "C" fn(VehicleRef, SpriteID),
    pub write_profit: extern "C" fn(VehicleRef, i64),
    pub write_reliability: extern "C" fn(VehicleRef, u16),
    /// `sprite_cache.sprite_seq.Set(sprite)`.
    pub write_sprite_seq: extern "C" fn(VehicleRef, SpriteID),
    /// `spritenum = sprite`.
    pub write_spritenum: extern "C" fn(VehicleRef, u8),
}
/// The immutable process-wide table a getter field returns.
#[allow(unsafe_code)]
fn table<T>(getter: extern "C" fn() -> *const T) -> &'static T {
    // SAFETY: Each getter field returns a table that lives for the process.
    unsafe { &*getter() }
}

impl VehicleServices {
    /// `Vehicle::SetNext` on two native shells.
    ///
    /// # Safety
    /// `shell` and a non-null `next` are live shells.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn set_next_shell(
        &self,
        shell: *mut std::ffi::c_void,
        next: *mut std::ffi::c_void,
    ) {
        let record = (self.record_from_shell)(shell);
        let next = (self.record_from_shell)(next);
        // SAFETY: The caller's live shells name live records.
        unsafe { crate::vehicle_base::set_next(record, next) };
    }

    /// The immutable process-wide vehicle resolver services.
    pub(crate) fn newgrf(&self) -> &'static crate::newgrf_vehicle::Services {
        table(self.newgrf)
    }
    /// The immutable process-wide station and depot visit table.
    pub(crate) fn visits(&self) -> &'static visits::Services {
        table(self.visits)
    }
    /// The immutable process-wide orders services, shared by every controller.
    pub(crate) fn orders(&self) -> &'static crate::orders::Leaves {
        table(self.orders)
    }
    /// `VehicleEnterDepot(v)`.
    pub(crate) fn enter_depot(&self, v: VehicleRef) {
        visits::enter_depot(v, self.visits());
    }
    /// `v->UpdatePositionAndViewport()`: update the position of the vehicle, and update the
    /// viewport. `map` gives the entry's map, which only the position update reads.
    #[allow(unsafe_code)]
    pub(crate) fn update_position_and_viewport(&self, v: VehicleRef, map: impl FnOnce() -> Map) {
        if v.vehicle_type() != VehicleType::VEH_EFFECT {
            // SAFETY: A live vehicle; the tile hash ends before the viewport callback.
            unsafe { occupancy::update_position(self, v, map()) };
        }
        (self.base_viewport)(v);
    }
    /// `v->HandleLoading(mode)`: handle the loading of the vehicle; when not it skips
    /// through dummy orders and does nothing in all other cases. `mode` is whether this is
    /// the non-first call for this vehicle in this tick.
    ///
    /// The `switch` on `current_order.GetType()` is tested here, inline at each controller's
    /// call, because most calls take its `default: return;`; `visits::handle_loading` holds
    /// the `OT_LOADING` and `OT_DUMMY` arms out of line.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub(crate) fn handle_loading(&self, v: VehicleRef, mode: bool) {
        if matches!(
            v.current_order().kind(),
            crate::order_type::OrderType::OT_LOADING | crate::order_type::OrderType::OT_DUMMY
        ) {
            visits::handle_loading(v, mode, self.visits());
        }
    }
    /// `ErrorUnknownCallbackResult` for an engine's callback result.
    #[allow(unsafe_code)]
    pub(crate) fn unknown_callback_result(
        &self,
        engine: EngineRef,
        callback: CallbackID,
        result: u16,
    ) {
        // SAFETY: The resolver table's capabilities and the live engine record.
        unsafe {
            crate::engine_capabilities::unknown_callback(
                self.newgrf().capabilities,
                engine.0.as_ptr(),
                callback,
                result,
            );
        }
    }

    /// `Ship::GetRunningCost` and `Aircraft::GetRunningCost`.
    ///
    /// # Safety
    /// `v` is a live ship or aircraft.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn running_cost(&self, v: VehicleRef) -> i64 {
        // SAFETY: The caller's live vehicle and its current engine.
        unsafe {
            crate::engine_capabilities::get_vehicle_running_cost(
                self.newgrf().capabilities,
                (self.engine)(v).0.as_ptr(),
                v.0.as_ptr(),
            )
        }
    }
}

/// `GetNewVehiclePos`: get position information of a vehicle when moving one pixel in the
/// direction it is facing.
#[inline]
#[must_use]
pub(crate) fn get_new_vehicle_pos(v: VehicleRef, map: Map) -> GetNewVehiclePosResult {
    const DELTA_COORD: [i8; 16] = [
        -1, -1, -1, 0, 1, 1, 1, 0, /* x */
        -1, 0, 1, 1, 1, 0, -1, -1, /* y */
    ];

    let direction = usize::from(v.direction().0);
    let x = v.x_pos().wrapping_add(i32::from(DELTA_COORD[direction]));
    let y = v
        .y_pos()
        .wrapping_add(i32::from(DELTA_COORD[direction + 8]));

    GetNewVehiclePosResult {
        x,
        y,
        old_tile: v.tile(),
        new_tile: map.tile_virt_xy(x.cast_unsigned(), y.cast_unsigned()),
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

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleServices" ["probe"] => VehicleServices {
        base_viewport,
        can_allocate,
        destroy,
        engine,
        first_sprite,
        length_changed,
        maintenance,
        newgrf,
        orders,
        pathfinding_result,
        play_leave_station_sound,
        play_sound,
        play_vehicle_sound,
        profit,
        record_from_shell,
        set_cargo_subtype,
        set_cargo_type,
        set_colourmap,
        set_max_age,
        ship_get,
        show_newgrf_vehicle_error,
        start_stop_dirty,
        tile_links,
        update_delta,
        visits,
        write_breakdowns,
        write_calendar_service,
        write_economy_service,
        write_first_sprite,
        write_profit,
        write_reliability,
        write_sprite_seq,
        write_spritenum,
    };
    "OpenTTDGroundServices" ["probe"] => GroundServices {
        physics,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direction_type::Direction;
    use crate::tile_type::TileIndex;

    /// A fresh record the test leaks.
    fn record(kind: VehicleType) -> VehicleRef {
        VehicleRef::of(crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            kind,
        ))
    }

    /// `GetNewVehiclePos` moves one pixel by `_delta_coord` and hashes the new position
    /// with `TileVirtXY`; `old_tile` is the vehicle's tile, read as stored.
    #[test]
    fn get_new_vehicle_pos_moves_one_pixel_by_the_delta_table() {
        let map = Map::from_log_x(8);
        let v = record(VehicleType::VEH_ROAD);
        let cases = [
            (Direction::DIR_N, 0x2f, 0x4f, 0x0402),
            (Direction::DIR_NE, 0x2f, 0x50, 0x0502),
            (Direction::DIR_E, 0x2f, 0x51, 0x0502),
            (Direction::DIR_SE, 0x30, 0x51, 0x0503),
            (Direction::DIR_S, 0x31, 0x51, 0x0503),
            (Direction::DIR_SW, 0x31, 0x50, 0x0503),
            (Direction::DIR_W, 0x31, 0x4f, 0x0403),
            (Direction::DIR_NW, 0x30, 0x4f, 0x0403),
        ];
        for (direction, x, y, tile) in cases {
            v.set_x_pos(0x30);
            v.set_y_pos(0x50);
            v.set_tile(TileIndex(0x0777));
            v.set_direction(direction);
            assert_eq!(
                get_new_vehicle_pos(v, map),
                GetNewVehiclePosResult {
                    x,
                    y,
                    old_tile: TileIndex(0x0777),
                    new_tile: TileIndex(tile),
                },
                "{direction:?}"
            );
        }
        // A negative x reaches `TileVirtXY` as the unsigned 0xFFFFFFFF.
        v.set_x_pos(0);
        v.set_direction(Direction::DIR_NE);
        let gp = get_new_vehicle_pos(v, map);
        assert_eq!((gp.x, gp.new_tile), (-1, TileIndex(0x1000_04ff)));
    }

    /// `GetAdvanceDistance`: an odd (axial) direction needs 192 progress, an even (corner)
    /// one 256.
    #[test]
    fn get_advance_distance_follows_the_direction_parity() {
        let v = record(VehicleType::VEH_ROAD);
        for (direction, distance) in [
            (Direction::DIR_NE, 192),
            (Direction::DIR_SW, 192),
            (Direction::DIR_N, 256),
            (Direction::DIR_E, 256),
        ] {
            v.set_direction(direction);
            assert_eq!(v.get_advance_distance(), distance, "{direction:?}");
        }
    }
}

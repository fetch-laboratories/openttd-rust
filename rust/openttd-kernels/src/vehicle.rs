/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared vehicle boundary used by every vehicle port: one opaque handle, the
//! base-`Vehicle` and `GroundVehicle` service tables and constants. Shared map
//! operations and arithmetic live in `services.rs`. The tables are defined once in `src/rust/vehicle_ffi.h` and filled
//! with designated initializers in `src/rust/vehicle_services.cpp`.
//!
//! A handle is a live `Vehicle *`, valid until the service that deletes it.
//! Services are synchronous and `noexcept`; none retains a Rust reference, and
//! callers end every owner borrow before calling one that can reenter Rust.
pub(crate) mod maintenance;
pub(crate) mod scheduler;

use std::ptr::NonNull;

/// Opaque C++ `Vehicle`.
#[repr(C)]
pub struct OpenTTDVehicle {
    _private: [u8; 0],
}

/// Native `VehicleType : uint8_t`. A transparent carrier preserves every byte,
/// including sentinels and unnamed values, without Rust enum validity constraints.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VehicleType(pub u8);

impl VehicleType {
    /// Native `VEH_TRAIN`.
    pub const TRAIN: Self = Self(0);
    /// Native `VEH_ROAD`.
    pub const ROAD: Self = Self(1);
    /// Native `VEH_SHIP`.
    pub const SHIP: Self = Self(2);
    /// Native `VEH_AIRCRAFT`.
    pub const AIRCRAFT: Self = Self(3);
}

/// Native `OrderType : uint8_t`, including every unnamed byte pattern.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OrderType(pub u8);

/// Native `RailVehicleTypes : uint8_t`, with the same unrestricted byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RailVehicleTypes(pub u8);

impl RailVehicleTypes {
    /// Native `RAILVEH_WAGON`.
    pub const WAGON: Self = Self(2);
}

/// Non-null live vehicle handle; `Option<VehicleRef>` crosses as a nullable pointer.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VehicleRef(pub NonNull<OpenTTDVehicle>);

/// Opaque immutable C++ engine context captured before a callback.
#[repr(C)]
pub struct OpenTTDEngine {
    _private: [u8; 0],
}
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EngineRef(pub NonNull<OpenTTDEngine>);

/// Only the engine context and GRF version consumed before the length callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EngineForLength {
    pub engine: EngineRef,
    pub grf_version: u8,
}

/// `MP_*` tile types.
pub const MP_ROAD: u8 = 2;
pub const MP_STATION: u8 = 5;
pub const MP_TUNNELBRIDGE: u8 = 9;
/// `OrderType` values.
pub const OT_GOTO_STATION: u8 = 1;
pub const OT_GOTO_DEPOT: u8 = 2;
pub const OT_LOADING: u8 = 3;
pub const OT_LEAVESTATION: u8 = 4;
/// `VehState` bits.
pub const VS_HIDDEN: u8 = 1 << 0;
pub const VS_STOPPED: u8 = 1 << 1;
pub const VS_CRASHED: u8 = 1 << 7;
/// `VehicleEnterTileState` bits.
pub const VETS_ENTERED_WORMHOLE: u8 = 1 << 1;
pub const VETS_CANNOT_ENTER: u8 = 1 << 2;
/// `AM_REALISTIC`.
pub const AM_REALISTIC: u8 = 1;
/// `INVALID_TRACKDIR`.
pub const INVALID_TRACKDIR: u8 = 0xFF;
/// `EngineID::Invalid()`.
pub const INVALID_ENGINE: u16 = 0xFFFF;
/// `INVALID_PRICE`.
pub const INVALID_PRICE: u8 = 0xFF;
/// `PROP_AIRCRAFT_SPEED`.
pub const PROP_AIRCRAFT_SPEED: u8 = 0x0C;
/// `CALLBACK_FAILED`.
pub const CALLBACK_FAILED: u16 = 0xFFFF;
/// `VEHICLE_LENGTH`.
pub const VEHICLE_LENGTH: u8 = 8;
/// `CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS`.
pub const YEAR_TICKS: i64 = 365 * 74;
/// `SND_19_DEPARTURE_OLD_RV_1` and `SND_1A_DEPARTURE_OLD_RV_2`.
pub const SND_DEPARTURE_OLD_RV_1: u16 = 23;
pub const SND_DEPARTURE_OLD_RV_2: u16 = 24;
/// `HVOT_BUS` and `HVOT_TRUCK`.
pub const HVOT_BUS: u8 = 1 << 2;
pub const HVOT_TRUCK: u8 = 1 << 3;

/// `GetNewVehiclePos` result.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NewPosition {
    pub x: i32,
    pub y: i32,
    pub tile: u32,
}

/// `HandleBreakdown` result, then the vehicle status it leaves.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Breakdown {
    pub broken: bool,
    pub status: u8,
}

/// Previous chain tile observed for the follower step.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PreviousTile {
    pub tile: u32,
    pub exists: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct XY {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpeedInputs {
    pub speed: u16,
    pub max_speed: u16,
    pub acceleration: u8,
    pub status: u8,
    pub subspeed: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirectionProgress {
    pub direction: u8,
    pub progress: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TickStatus {
    pub tick: u8,
    pub status: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DayCost {
    pub profit: i64,
    pub running: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AltitudeInputs {
    pub max_speed: u16,
    pub direction: u8,
}

/// Base `Vehicle` operations and writes shared by every vehicle type.
#[repr(C)]
pub struct VehicleServices {
    /// Consecutive `tick_counter`, `running_ticks` and `current_order_time` stores.
    pub write_counters: extern "C" fn(VehicleRef, u8, u8, i32),
    /// `HandleBreakdown()`, then observe `vehstatus`.
    pub handle_breakdown: extern "C" fn(VehicleRef) -> Breakdown,
    pub waiting_for_unbunching: extern "C" fn(VehicleRef) -> bool,
    pub leave_unbunching_depot: extern "C" fn(VehicleRef),
    pub reset_depot_unbunching: extern "C" fn(VehicleRef),
    pub service_in_depot: extern "C" fn(VehicleRef),
    pub needs_automatic_servicing: extern "C" fn(VehicleRef) -> bool,
    pub enter_depot: extern "C" fn(VehicleRef),
    pub pathfinding_result: extern "C" fn(VehicleRef, bool),
    pub new_position: extern "C" fn(VehicleRef) -> NewPosition,
    /// Store x/y, `UpdatePosition()`, then observe `vehstatus`.
    pub move_then_status: extern "C" fn(VehicleRef, i32, i32) -> u8,
    /// `Vehicle::UpdateViewport(true)`.
    pub base_viewport: extern "C" fn(VehicleRef),
    /// `GetVehicleProperty(v, property, fallback)`.
    pub property: extern "C" fn(VehicleRef, u8, i32) -> i32,
    /// `GetVehicleCallback(CBID_VEHICLE_LENGTH, 0, 0, v->engine_type, v)`.
    pub length_callback: extern "C" fn(VehicleRef) -> u16,
    /// `ErrorUnknownCallbackResult(grfid, CBID_VEHICLE_LENGTH, result)`.
    pub unknown_length_result: extern "C" fn(EngineRef, u16),
    pub length_changed: extern "C" fn(VehicleRef),
    /// `PlayVehicleSound(v, VSE_START)`.
    pub play_start_sound: extern "C" fn(VehicleRef) -> bool,
    pub play_sound: extern "C" fn(VehicleRef, u16),
    /// `CreateEffectVehicleRel(v, 4, 4, 8, EV_EXPLOSION_LARGE)`.
    pub large_explosion: extern "C" fn(VehicleRef),
    pub order_free: extern "C" fn(VehicleRef),
    /// `MakeDummy()`, then the start/stop widget dirty.
    pub order_dummy: extern "C" fn(VehicleRef),
    pub start_stop_dirty: extern "C" fn(VehicleRef),
    /// `delete v`; the handle and its owner are dead afterwards.
    pub destroy: extern "C" fn(VehicleRef),
    pub age: extern "C" fn(VehicleRef),
    /// `EconomyAgeVehicle(v)`, then observe `day_counter`.
    pub economy_age: extern "C" fn(VehicleRef) -> u8,
    pub decrease_value: extern "C" fn(VehicleRef),
    pub check_breakdown: extern "C" fn(VehicleRef),
    /// `CheckOrders(v)`, then observe `running_ticks`.
    pub check_orders: extern "C" fn(VehicleRef) -> u8,
    pub write_day: extern "C" fn(VehicleRef, u8),
    pub write_direction: extern "C" fn(VehicleRef, u8),
    pub write_speed: extern "C" fn(VehicleRef, u16),
    /// Shared last visited station read, before station discovery callbacks.
    pub last_station: extern "C" fn(VehicleRef) -> u16,
    pub write_last_station: extern "C" fn(VehicleRef, u16),
    pub write_dest: extern "C" fn(VehicleRef, u32),
    pub write_progress: extern "C" fn(VehicleRef, u8),
    pub write_max_speed: extern "C" fn(VehicleRef, u16),
    pub previous_tile: extern "C" fn(VehicleRef) -> PreviousTile,
    pub detach_last: extern "C" fn(VehicleRef, VehicleRef, VehicleRef),
    pub next: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub engine_id: extern "C" fn(VehicleRef) -> u16,
    pub engine_for_length: extern "C" fn(VehicleRef) -> EngineForLength,
    pub write_tick: extern "C" fn(VehicleRef, u8),
    pub direction: extern "C" fn(VehicleRef) -> u8,
    pub status: extern "C" fn(VehicleRef) -> u8,
    pub tile: extern "C" fn(VehicleRef) -> u32,
    pub progress: extern "C" fn(VehicleRef) -> u8,
    pub dest_tile: extern "C" fn(VehicleRef) -> u32,
    pub order_type: extern "C" fn(VehicleRef) -> u8,
    pub order_destination: extern "C" fn(VehicleRef) -> u16,
    pub order_nonstop: extern "C" fn(VehicleRef) -> u8,
    pub subtype: extern "C" fn(VehicleRef) -> u8,
    pub x: extern "C" fn(VehicleRef) -> i32,
    pub y: extern "C" fn(VehicleRef) -> i32,
    pub z: extern "C" fn(VehicleRef) -> i32,
    pub tick: extern "C" fn(VehicleRef) -> u8,
    pub owner: extern "C" fn(VehicleRef) -> u8,
    pub speed: extern "C" fn(VehicleRef) -> u16,
    pub subspeed: extern "C" fn(VehicleRef) -> u8,
    pub running: extern "C" fn(VehicleRef) -> u8,
    pub order_time: extern "C" fn(VehicleRef) -> i32,
    pub vehicle_type: extern "C" fn(VehicleRef) -> u8,
    pub write_x: extern "C" fn(VehicleRef, i32),
    pub write_y: extern "C" fn(VehicleRef, i32),
    pub write_z: extern "C" fn(VehicleRef, i32),
    pub write_tile: extern "C" fn(VehicleRef, u32),
    pub write_status: extern "C" fn(VehicleRef, u8),
    pub write_subspeed: extern "C" fn(VehicleRef, u8),
    pub write_breakdown_counter: extern "C" fn(VehicleRef, u8),
    pub write_running: extern "C" fn(VehicleRef, u8),
    pub write_order_time: extern "C" fn(VehicleRef, i32),
    pub write_profit: extern "C" fn(VehicleRef, i64),
    pub write_economy_service: extern "C" fn(VehicleRef, i32),
    pub write_calendar_service: extern "C" fn(VehicleRef, i32),
    pub write_breakdowns: extern "C" fn(VehicleRef, u8),
    pub write_reliability: extern "C" fn(VehicleRef, u16),
    pub write_cargo_age: extern "C" fn(VehicleRef, u16),
    pub coordinates: extern "C" fn(VehicleRef) -> Coordinates,
    pub xy: extern "C" fn(VehicleRef) -> XY,
    pub speed_inputs: extern "C" fn(VehicleRef) -> SpeedInputs,
    pub direction_progress: extern "C" fn(VehicleRef) -> DirectionProgress,
    pub tick_status: extern "C" fn(VehicleRef) -> TickStatus,
    pub day_cost: extern "C" fn(VehicleRef) -> DayCost,
    pub make_dummy: extern "C" fn(VehicleRef),
    pub loading: extern "C" fn(VehicleRef, bool),
    pub breakdown: extern "C" fn(VehicleRef) -> u8,
    pub check_orders_only: extern "C" fn(VehicleRef),
    pub write_coordinates: extern "C" fn(VehicleRef, i32, i32, i32),
    pub altitude_inputs: extern "C" fn(VehicleRef) -> AltitudeInputs,
    pub maintenance: *const maintenance::Services,
}

/// Position step result: `UpdateInclination`'s old height, then the new height,
/// speed and track speed limit it leaves.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Moved {
    pub old_z: i32,
    pub z: i32,
    pub cur_speed: u16,
    pub max_track_speed: u16,
}

/// `DoUpdateSpeed` result, then the speed it stored.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Speed {
    pub distance: i32,
    pub cur_speed: u16,
}

/// Post-viewport link; progress is consumed only when this link ends the loop.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ViewportNext {
    pub next: Option<VehicleRef>,
    pub final_progress: u8,
}

/// `SetLastSpeed`, then the status it leaves before viewport updates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LastSpeed {
    pub status: u8,
}

/// `GroundVehicleBase::Crash` victims, then `IsFrontEngine()`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Crash {
    pub victims: u32,
    pub front: bool,
}

/// `GroundVehicle<T>` members, one C++ template instance per vehicle type.
#[repr(C)]
pub struct GroundServices {
    /// Invalidate the chain `NewGRF` cache, then reset total length.
    pub cache_begin: extern "C" fn(VehicleRef),
    /// Store x/y, `UpdatePosition()`, `UpdateInclination(new_tile, delta)`.
    pub move_incline: extern "C" fn(VehicleRef, i32, i32, bool, bool) -> Moved,
    /// `UpdateViewport(force, delta)`.
    pub update_viewport_then_next: extern "C" fn(VehicleRef, VehicleRef) -> ViewportNext,
    /// Store the direction, then `UpdateViewport(true, true)`.
    pub turn: extern "C" fn(VehicleRef, u8),
    pub do_update_speed: extern "C" fn(VehicleRef, u32, i32, i32) -> Speed,
    pub set_last_speed: extern "C" fn(VehicleRef) -> LastSpeed,
    pub acceleration: extern "C" fn(VehicleRef) -> i32,
    pub crash: extern "C" fn(VehicleRef, bool) -> Crash,
    /// `v->First()->CargoChanged()`.
    pub first_cargo_changed: extern "C" fn(VehicleRef),
    /// Set `GVF_SUPPRESS_IMPLICIT_ORDERS`, then `MakeGoToDepot(depot, Service)`.
    pub go_to_depot_service: extern "C" fn(VehicleRef, u16),
    pub write_first_engine: extern "C" fn(VehicleRef, u16),
    /// Store `u` length and `v` total length, then `u->UpdateVisualEffect()`.
    pub write_length: extern "C" fn(VehicleRef, u8, VehicleRef, u16),
    pub cached_length: extern "C" fn(VehicleRef) -> u8,
    pub has_articulated: extern "C" fn(VehicleRef) -> bool,
}

/// `GetAdvanceDistance()`.
#[must_use]
pub fn advance_distance(direction: u8) -> i32 {
    if direction & 1 != 0 { 192 } else { 256 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::Map;
    #[test]
    fn map_offsets_follow_the_original_tables() {
        let map = Map { log_x: 8 };
        assert_eq!(map.add_diag(0x1010, 0), 0x100f);
        assert_eq!(map.add_diag(0x1010, 1), 0x1110);
        assert_eq!(map.add_dir(0x1010, 0), 0x0f0f);
        assert_eq!(map.add_dir(0x1010, 5), 0x1011);
        assert_eq!(map.add_dir(0, 0), 0_u32.wrapping_sub(257));
        assert_eq!((map.tile_x(0x1234), map.tile_y(0x1234)), (0x34, 0x12));
        assert_eq!(advance_distance(3), 192);
    }
}

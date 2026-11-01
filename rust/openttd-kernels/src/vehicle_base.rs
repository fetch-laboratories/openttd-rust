/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehicle_base.h` subtype values, inline predicates over a copied
//! `Vehicle::subtype`, and the shared `VehicleRecord`. `services.cpp` asserts
//! every value.
//!
//! A record lives from its shell's construction until after the base destructor.
//! Both languages access its fields in place under the game-state mutex; Rust
//! holds no reference to a record across a callback.
#![allow(unsafe_code)]

/// `GroundVehicleSubtypeFlags::GVSF_FRONT`.
pub const GVSF_FRONT: u8 = 0;
/// `GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART`.
pub const GVSF_ARTICULATED_PART: u8 = 1;
/// `GroundVehicleSubtypeFlags::GVSF_WAGON`.
pub const GVSF_WAGON: u8 = 2;
/// `GroundVehicleSubtypeFlags::GVSF_ENGINE`.
pub const GVSF_ENGINE: u8 = 3;
/// `GroundVehicleSubtypeFlags::GVSF_FREE_WAGON`.
pub const GVSF_FREE_WAGON: u8 = 4;
/// `GroundVehicleSubtypeFlags::GVSF_MULTIHEADED`.
pub const GVSF_MULTIHEADED: u8 = 5;

/// `HasBit(subtype, flag)`.
const fn has(subtype: u8, flag: u8) -> bool {
    subtype & (1 << flag) != 0
}
/// `Vehicle::IsFrontEngine`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_front_engine(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GVSF_FRONT)
}
/// `Vehicle::IsArticulatedPart`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_articulated_part(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GVSF_ARTICULATED_PART)
}

/// `Vehicle::IsGroundVehicle`.
pub(crate) const fn is_ground_vehicle(kind: u8) -> bool {
    kind == crate::vehicle_type::VEH_TRAIN || kind == crate::vehicle_type::VEH_ROAD
}

/// `VehState::Hidden`.
pub const VS_HIDDEN: u8 = 1 << 0;
/// `VehState::Stopped`.
pub const VS_STOPPED: u8 = 1 << 1;
/// `VehStatus::TrainSlowing`.
pub const VS_TRAIN_SLOWING: u8 = 1 << 4;
/// `VehState::AircraftBroken`.
pub const VS_AIRCRAFT_BROKEN: u8 = 1 << 6;
/// `VehState::Crashed`.
pub const VS_CRASHED: u8 = 1 << 7;

/// `NCVV_POSITION_CONSIST_LENGTH`.
pub const NCVV_POSITION_CONSIST_LENGTH: u8 = 0;
/// `NCVV_POSITION_SAME_ID_LENGTH`.
pub const NCVV_POSITION_SAME_ID_LENGTH: u8 = 1;
/// `NCVV_CONSIST_CARGO_INFORMATION`.
pub const NCVV_CONSIST_CARGO_INFORMATION: u8 = 2;
/// `NCVV_COMPANY_INFORMATION`.
pub const NCVV_COMPANY_INFORMATION: u8 = 3;
/// `NCVV_POSITION_IN_VEHICLE`.
pub const NCVV_POSITION_IN_VEHICLE: u8 = 4;
/// `NCVV_END`.
pub const NCVV_END: u8 = 5;

/// `VESM_NONE`.
pub const VESM_NONE: u8 = 0;
/// `VESM_STEAM`.
pub const VESM_STEAM: u8 = 1;
/// `VESM_DIESEL`.
pub const VESM_DIESEL: u8 = 2;
/// `VESM_ELECTRIC`.
pub const VESM_ELECTRIC: u8 = 3;
/// `VESM_END`.
pub const VESM_END: u8 = 4;

/// `VehState::Unclickable`.
pub const VS_UNCLICKABLE: u8 = 1 << 2;
/// `VehState::Shadow`.
pub const VS_SHADOW: u8 = 1 << 5;

use crate::company_type::CompanyID;
use crate::engine_type::EngineID;
use crate::station_type::StationID;
use crate::tile_type::TileIndex;
use crate::vehicle_type::VehicleType;

/// Opaque native `Vehicle` shell: pool identity, virtual operations and unmoved fields.
#[repr(C)]
pub struct OpenTTDVehicleShell {
    _private: [u8; 0],
}

crate::abi::shared_records! {
    fn abi_record;
    /// Common movement and consist fields of one native shell, read and written
    /// in place by both languages. Widths and defaults are the original members'.
    "OpenTTDVehicle" ["vehicle"] => pub struct VehicleRecord {
        /// Owning native shell; stable while the record lives.
        pub shell: *mut OpenTTDVehicleShell,
        /// `Vehicle::next`.
        pub next: *mut VehicleRecord,
        /// NOSAVE `Vehicle::previous`.
        pub previous: *mut VehicleRecord,
        /// NOSAVE `Vehicle::first`.
        pub first: *mut VehicleRecord,
        pub tile: TileIndex,
        pub dest_tile: TileIndex,
        pub motion_counter: u32,
        pub x_pos: i32,
        pub y_pos: i32,
        pub z_pos: i32,
        pub engine_type: EngineID,
        pub cur_speed: u16,
        pub cargo_cap: u16,
        pub refit_cap: u16,
        pub cargo_age_counter: u16,
        pub last_station_visited: StationID,
        /// `VehicleCache::cached_max_speed`.
        pub cached_max_speed: u16,
        /// `VehicleCache::cached_cargo_age_period`.
        pub cached_cargo_age_period: u16,
        /// `BaseVehicle::type`.
        pub r#type: VehicleType,
        pub subtype: u8,
        pub owner: CompanyID,
        /// Native `Direction`.
        pub direction: u8,
        /// Native `VehStates`.
        pub vehstatus: u8,
        pub subspeed: u8,
        pub acceleration: u8,
        pub progress: u8,
        pub tick_counter: u8,
        pub running_ticks: u8,
        pub day_counter: u8,
        pub breakdown_ctr: u8,
    }
}

/// Allocate `shell`'s record with the original member and constructor defaults;
/// `first` points to the record itself.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_vehicle_record_new(
    shell: *mut OpenTTDVehicleShell,
    kind: VehicleType,
) -> *mut VehicleRecord {
    let record = Box::into_raw(Box::new(VehicleRecord {
        shell,
        next: std::ptr::null_mut(),
        previous: std::ptr::null_mut(),
        first: std::ptr::null_mut(),
        tile: TileIndex(u32::MAX),
        dest_tile: TileIndex(u32::MAX),
        motion_counter: 0,
        x_pos: 0,
        y_pos: 0,
        z_pos: 0,
        engine_type: EngineID(u16::MAX),
        cur_speed: 0,
        cargo_cap: 0,
        refit_cap: 0,
        cargo_age_counter: 1,
        last_station_visited: StationID(u16::MAX),
        cached_max_speed: 0,
        cached_cargo_age_period: 0,
        r#type: kind,
        subtype: 0,
        owner: CompanyID(u8::MAX),
        direction: u8::MAX,
        vehstatus: 0,
        subspeed: 0,
        acceleration: 0,
        progress: 0,
        tick_counter: 0,
        running_ticks: 0,
        day_counter: 0,
        breakdown_ctr: 0,
    }));
    // SAFETY: The allocation above is live and not yet shared.
    unsafe {
        (*record).first = record;
    }
    record
}

/// Release a record after its shell's base destructor.
///
/// # Safety
/// `record` comes from `openttd_rust_vehicle_record_new` and is released once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_record_delete(record: *mut VehicleRecord) {
    // SAFETY: The owning shell transfers the allocation exactly once.
    unsafe { drop(Box::from_raw(record)) }
}

/// `Vehicle::SetNext` after its self-link assertion: rewrite the old suffix's
/// `first`, clear its `previous`, detach the new suffix from its predecessor,
/// then rewrite the new suffix's `first`.
///
/// # Safety
/// `this` and non-null `next` are live records of the serialized game state, and
/// every record reachable through their links is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_set_next(
    this: *mut VehicleRecord,
    next: *mut VehicleRecord,
) {
    // SAFETY: The caller guarantees every linked record is live; raw field
    // accesses retain no reference and no callback runs.
    unsafe {
        let old = (*this).next;
        if !old.is_null() {
            let mut v = old;
            while !v.is_null() {
                (*v).first = old;
                v = (*v).next;
            }
            (*old).previous = std::ptr::null_mut();
        }
        (*this).next = next;
        if !next.is_null() {
            let predecessor = (*next).previous;
            if !predecessor.is_null() {
                (*predecessor).next = std::ptr::null_mut();
            }
            (*next).previous = this;
            let first = (*this).first;
            let mut v = next;
            while !v.is_null() {
                (*v).first = first;
                v = (*v).next;
            }
        }
    }
}

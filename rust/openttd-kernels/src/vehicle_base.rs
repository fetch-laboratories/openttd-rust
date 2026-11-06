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
        /// Stable NOSAVE links for the canonical tile index.
        pub tile_links: crate::vehicle::occupancy::Links,
        /// Stable owner links, initialized by the corresponding native constructors.
        pub ground: *mut crate::ground_vehicle::GroundState,
        pub train: *mut crate::train_state::TrainState,
        pub road: *mut crate::road::OpenTTDRoadState,
        pub aircraft: *mut crate::aircraft::AircraftState,
        pub consist: *mut crate::orders::Consist,
        pub orders: *mut crate::orders::VehicleOrders,
        pub visual: *mut u8,
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
        tile_links: crate::vehicle::occupancy::Links::default(),
        ground: std::ptr::null_mut(),
        train: std::ptr::null_mut(),
        road: std::ptr::null_mut(),
        aircraft: std::ptr::null_mut(),
        consist: std::ptr::null_mut(),
        orders: std::ptr::null_mut(),
        visual: std::ptr::null_mut(),
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
    // SAFETY: The native facade checks the original self-link precondition.
    unsafe { set_next(this, next) };
}

/// Complete `Vehicle::SetNext` after the native self-link assertion.
///
/// # Safety
/// Both records and all reachable links are live in the serialized game state.
pub(crate) unsafe fn set_next(this: *mut VehicleRecord, next: *mut VehicleRecord) {
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

// VehicleRef's live-record contract permits field-sized raw reads between callbacks.
impl crate::vehicle::VehicleRef {
    pub(crate) fn first(self) -> Self {
        Self(std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), first)).unwrap())
    }
    pub(crate) fn last(self) -> Self {
        let mut v = self;
        while let Some(next) = v.next() {
            v = next;
        }
        v
    }
    pub(crate) fn next_vehicle(self) -> Option<Self> {
        let mut next = self.next();
        while let Some(v) = next {
            if !v.is_articulated_part() {
                break;
            }
            next = v.next();
        }
        next
    }
    /// `Train::GetNextUnit`; this record is a train.
    pub(crate) fn next_unit(self) -> Option<Self> {
        let mut next = self.next_vehicle();
        if let Some(v) = next
            && crate::ground_vehicle::is_rear_dualheaded(v.subtype())
        {
            next = v.next_vehicle();
        }
        next
    }
    /// `Vehicle::GetPrevVehicle`.
    pub(crate) fn previous_vehicle(self) -> Option<Self> {
        let mut previous = self.previous();
        while let Some(v) = previous {
            if !v.is_articulated_part() {
                break;
            }
            previous = v.previous();
        }
        previous
    }
    /// `Train::GetPrevUnit`; this record is a train.
    pub(crate) fn previous_unit(self) -> Option<Self> {
        let mut previous = self.previous_vehicle();
        if let Some(v) = previous
            && crate::ground_vehicle::is_rear_dualheaded(v.subtype())
        {
            previous = v.previous_vehicle();
        }
        previous
    }
    /// `Vehicle::IsArticulatedPart`.
    pub(crate) fn is_articulated_part(self) -> bool {
        is_articulated_part(is_ground_vehicle(self.vehicle_type()), self.subtype())
    }
    /// `Vehicle::HasEngineType`.
    pub(crate) fn has_engine_type(self) -> bool {
        match VehicleType(self.vehicle_type()) {
            VehicleType::AIRCRAFT => crate::aircraft::is_normal_aircraft(self.subtype()),
            VehicleType::TRAIN | VehicleType::ROAD | VehicleType::SHIP => true,
            _ => false,
        }
    }
    /// `Vehicle::IsFrontEngine`.
    pub(crate) fn is_front_engine(self) -> bool {
        is_front_engine(is_ground_vehicle(self.vehicle_type()), self.subtype())
    }
    /// `Vehicle::GetLastEnginePart`.
    pub(crate) fn last_engine_part(self) -> Self {
        let mut v = self;
        while let Some(next) = v.next() {
            if !next.is_articulated_part() {
                break;
            }
            v = next;
        }
        v
    }
    /// `Train::other_multiheaded_part`; the record is a live train.
    pub(crate) fn other_multiheaded_part(self) -> Option<Self> {
        let train = crate::raw_field::read_field!(self.0.as_ptr(), train);
        std::ptr::NonNull::new(crate::raw_field::read_field!(train, other_multiheaded_part))
            .map(Self)
    }
    pub(crate) fn ground(self) -> *mut crate::ground_vehicle::GroundState {
        crate::raw_field::read_field!(self.0.as_ptr(), ground)
    }
    pub(crate) fn ground_cache(self) -> *mut crate::ground_vehicle::GroundCache {
        let ground = self.ground();
        // SAFETY: Ground shells bind a live owner before controller entries.
        unsafe { std::ptr::addr_of_mut!((*ground).cache) }
    }
    pub(crate) fn current_order(self) -> crate::orders::Order {
        let orders = crate::raw_field::read_field!(self.0.as_ptr(), orders);
        // SAFETY: Vehicle construction starts the native Order lifetime before publication.
        let order = crate::raw_field::read_field!(orders, current);
        unsafe { order.assume_init() }
    }
    pub(crate) fn order_index(self) -> u8 {
        let consist = crate::raw_field::read_field!(self.0.as_ptr(), consist);
        crate::raw_field::read_field!(consist, real)
    }
    pub(crate) fn current_order_pointer(self) -> *mut crate::orders::Order {
        let orders = crate::raw_field::read_field!(self.0.as_ptr(), orders);
        // SAFETY: The native Order lifetime begins during Vehicle construction.
        unsafe { std::ptr::addr_of_mut!((*orders).current).cast() }
    }
    pub(crate) fn order_time(self) -> i32 {
        let consist = crate::raw_field::read_field!(self.0.as_ptr(), consist);
        crate::raw_field::read_field!(consist, time)
    }
    pub(crate) fn write_order_time(self, value: i32) {
        let consist = crate::raw_field::read_field!(self.0.as_ptr(), consist);
        crate::raw_field::write_field!(consist, time, value);
    }
    pub(crate) fn visual_effect(self) -> u8 {
        let visual = crate::raw_field::read_field!(self.0.as_ptr(), visual);
        // SAFETY: The bound visual owner lives throughout the source access interval.
        unsafe { visual.read() }
    }
    pub(crate) fn next(self) -> Option<Self> {
        std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), next)).map(Self)
    }
    pub(crate) fn previous(self) -> Option<Self> {
        std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), previous)).map(Self)
    }
    pub(crate) fn set_hidden(self, value: bool) {
        self.write_status(if value {
            self.status() | VS_HIDDEN
        } else {
            self.status() & !VS_HIDDEN
        });
    }
    pub(crate) fn cargo_cap(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), cargo_cap)
    }
    pub(crate) fn refit_cap(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), refit_cap)
    }
    pub(crate) fn write_refit_cap(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), refit_cap, value);
    }
    pub(crate) fn breakdown_counter(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), breakdown_ctr)
    }
    pub(crate) fn motion_counter(self) -> u32 {
        crate::raw_field::read_field!(self.0.as_ptr(), motion_counter)
    }
    /// The aircraft owner link; the record is a live aircraft.
    pub(crate) fn aircraft_state(self) -> *mut crate::aircraft::AircraftState {
        crate::raw_field::read_field!(self.0.as_ptr(), aircraft)
    }
    /// `Vehicle::Move(n)` from a nullable record.
    pub(crate) fn move_by(mut v: *mut VehicleRecord, n: i32) -> *mut VehicleRecord {
        let mut i = 0;
        while i != n && !v.is_null() {
            v = if n < 0 {
                i -= 1;
                crate::raw_field::read_field!(v, previous)
            } else {
                i += 1;
                crate::raw_field::read_field!(v, next)
            };
        }
        v
    }
    /// `GroundVehicle::HasArticulatedPart` / `Vehicle::HasArticulatedPart`.
    pub(crate) fn has_articulated_part(self) -> bool {
        self.next().is_some_and(Self::is_articulated_part)
    }
    /// `GroundVehicle::GetFirstEnginePart`.
    pub(crate) fn first_engine_part(self) -> Self {
        let mut v = self;
        while v.is_articulated_part() {
            v = v.previous().unwrap();
        }
        v
    }
    pub(crate) fn write_cargo_cap(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), cargo_cap, value);
    }
    pub(crate) fn engine_id(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), engine_type).0
    }
    pub(crate) fn owner(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), owner).0
    }
    pub(crate) fn subtype(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), subtype)
    }
    pub(crate) fn vehicle_type(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), r#type).0
    }
    pub(crate) fn day(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), day_counter)
    }

    pub(crate) fn direction(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), direction)
    }

    pub(crate) fn speed(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), cur_speed)
    }

    pub(crate) fn last_station(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), last_station_visited).0
    }

    pub(crate) fn progress(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), progress)
    }

    pub(crate) fn dest_tile(self) -> u32 {
        crate::raw_field::read_field!(self.0.as_ptr(), dest_tile).0
    }

    pub(crate) fn status(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), vehstatus)
    }

    pub(crate) fn tile(self) -> u32 {
        crate::raw_field::read_field!(self.0.as_ptr(), tile).0
    }

    pub(crate) fn x(self) -> i32 {
        crate::raw_field::read_field!(self.0.as_ptr(), x_pos)
    }

    pub(crate) fn y(self) -> i32 {
        crate::raw_field::read_field!(self.0.as_ptr(), y_pos)
    }

    pub(crate) fn z(self) -> i32 {
        crate::raw_field::read_field!(self.0.as_ptr(), z_pos)
    }

    pub(crate) fn tick(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), tick_counter)
    }

    pub(crate) fn subspeed(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), subspeed)
    }

    pub(crate) fn running(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), running_ticks)
    }

    pub(crate) fn acceleration(self) -> u8 {
        crate::raw_field::read_field!(self.0.as_ptr(), acceleration)
    }
    pub(crate) fn max_speed(self) -> u16 {
        crate::raw_field::read_field!(self.0.as_ptr(), cached_max_speed)
    }

    pub(crate) fn write_day(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), day_counter, value);
    }

    pub(crate) fn write_direction(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), direction, value);
    }

    pub(crate) fn write_speed(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), cur_speed, value);
    }

    pub(crate) fn write_engine(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), engine_type, EngineID(value));
    }
    pub(crate) fn write_owner(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), owner, CompanyID(value));
    }
    pub(crate) fn write_last_station(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), last_station_visited, StationID(value));
    }

    pub(crate) fn write_progress(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), progress, value);
    }

    pub(crate) fn write_status(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), vehstatus, value);
    }

    pub(crate) fn write_tile(self, value: u32) {
        crate::raw_field::write_field!(self.0.as_ptr(), tile, TileIndex(value));
    }

    pub(crate) fn write_x(self, value: i32) {
        crate::raw_field::write_field!(self.0.as_ptr(), x_pos, value);
    }

    pub(crate) fn write_y(self, value: i32) {
        crate::raw_field::write_field!(self.0.as_ptr(), y_pos, value);
    }

    pub(crate) fn write_z(self, value: i32) {
        crate::raw_field::write_field!(self.0.as_ptr(), z_pos, value);
    }

    pub(crate) fn write_tick(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), tick_counter, value);
    }

    pub(crate) fn write_subspeed(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), subspeed, value);
    }

    pub(crate) fn write_running(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), running_ticks, value);
    }

    pub(crate) fn write_dest(self, value: u32) {
        crate::raw_field::write_field!(self.0.as_ptr(), dest_tile, TileIndex(value));
    }

    pub(crate) fn write_max_speed(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), cached_max_speed, value);
    }

    pub(crate) fn write_breakdown_counter(self, value: u8) {
        crate::raw_field::write_field!(self.0.as_ptr(), breakdown_ctr, value);
    }

    pub(crate) fn write_cargo_age(self, value: u16) {
        crate::raw_field::write_field!(self.0.as_ptr(), cached_cargo_age_period, value);
    }
    pub(crate) fn write_xy(self, x: i32, y: i32) {
        self.write_x(x);
        self.write_y(y);
    }
    pub(crate) fn write_coordinates(self, x: i32, y: i32, z: i32) {
        self.write_xy(x, y);
        self.write_z(z);
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn train_units_skip_articulated_parts_and_one_rear_head() {
        use crate::vehicle::VehicleRef;
        let articulated = 1 << GVSF_ARTICULATED_PART;
        let rear = 1 << GVSF_MULTIHEADED;
        let engine = 1 << GVSF_ENGINE;
        let records: Vec<_> = [
            engine,
            articulated,
            rear,
            0,
            articulated,
            engine,
            rear,
            rear,
        ]
        .into_iter()
        .map(|subtype| {
            let record = openttd_rust_vehicle_record_new(std::ptr::null_mut(), VehicleType::TRAIN);
            // SAFETY: Each record is privately owned through this test.
            unsafe { (*record).subtype = subtype };
            VehicleRef::of(record)
        })
        .collect();
        for pair in records.windows(2) {
            // SAFETY: Both records remain live through traversal and cleanup.
            unsafe { set_next(pair[0].0.as_ptr(), pair[1].0.as_ptr()) };
        }
        let expected = [
            (Some(3), None),
            (Some(3), Some(0)),
            (Some(3), Some(0)),
            (Some(5), Some(0)),
            (Some(5), Some(3)),
            (Some(7), Some(3)),
            (None, Some(5)),
            (None, Some(5)),
        ];
        for (record, (next, previous)) in records.iter().zip(expected) {
            assert_eq!(
                record.next_unit().map(|v| v.0.as_ptr()),
                next.map(|i| records[i].0.as_ptr())
            );
            assert_eq!(
                record.previous_unit().map(|v| v.0.as_ptr()),
                previous.map(|i| records[i].0.as_ptr())
            );
        }
        for record in records {
            // SAFETY: No traversal follows these private records after cleanup.
            unsafe { openttd_rust_vehicle_record_delete(record.0.as_ptr()) };
        }
    }

    #[test]
    fn set_next_detaches_old_suffix_and_new_predecessor() {
        let records: Vec<_> = (0..5)
            .map(|_| openttd_rust_vehicle_record_new(std::ptr::null_mut(), VehicleType::TRAIN))
            .collect();
        let [head, old, tail, other, attached] = records[..] else {
            unreachable!()
        };
        // SAFETY: These private allocations stay live through all chain operations.
        unsafe {
            set_next(head, old);
            set_next(old, tail);
            set_next(other, attached);
            set_next(head, attached);
            assert_eq!((*head).next, attached);
            assert_eq!((*attached).previous, head);
            assert_eq!((*attached).first, head);
            assert!((*other).next.is_null());
            assert!((*old).previous.is_null());
            assert_eq!((*old).first, old);
            assert_eq!((*tail).first, old);
            assert_eq!((*tail).previous, old);

            set_next(head, tail);
            assert!((*old).next.is_null());
            assert_eq!((*tail).previous, head);
            assert_eq!((*tail).first, head);
            assert!((*attached).previous.is_null());
            assert_eq!((*attached).first, attached);

            set_next(head, std::ptr::null_mut());
            assert!((*head).next.is_null());
            assert!((*tail).previous.is_null());
            assert_eq!((*tail).first, tail);
            for record in records {
                openttd_rust_vehicle_record_delete(record);
            }
        }
    }
}

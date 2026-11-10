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

/// Vehicle state bit positions in `Vehicle::vehstatus`, from `vehicle_base.h`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehState(pub u8);

#[allow(non_upper_case_globals)]
impl VehState {
    /// Vehicle is not visible.
    pub const Hidden: Self = Self(0);
    /// Vehicle is stopped by the player.
    pub const Stopped: Self = Self(1);
    /// Vehicle is not clickable by the user (shadow vehicles).
    pub const Unclickable: Self = Self(2);
    /// Use default vehicle palette. See `DoDrawVehicle`.
    #[allow(dead_code)] // Complete original enumerator vocabulary; drawing remains native.
    pub const DefaultPalette: Self = Self(3);
    /// Train is slowing down.
    pub const TrainSlowing: Self = Self(4);
    /// Vehicle is a shadow vehicle.
    pub const Shadow: Self = Self(5);
    /// Aircraft is broken down.
    pub const AircraftBroken: Self = Self(6);
    /// Vehicle is crashed.
    pub const Crashed: Self = Self(7);
}

/// Native `EnumBitSet<VehState, uint8_t>`, retaining every stored bit.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehStates(pub u8);

impl VehStates {
    /// Set containing the named state (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Native uint8_t storage truncates the 64-bit mask.
    pub const fn from_element(element: VehState) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native bit-set removal.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

/// Bit numbers indicating which `NewGRFCache` values are valid.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewGRFCacheValidValues(pub u8);

impl NewGRFCacheValidValues {
    /// Set if the `NewGRF` var 40 currently stored is valid.
    pub const NCVV_POSITION_CONSIST_LENGTH: Self = Self(0);
    /// Set if the `NewGRF` var 41 currently stored is valid.
    pub const NCVV_POSITION_SAME_ID_LENGTH: Self = Self(1);
    /// Set if the `NewGRF` var 42 currently stored is valid.
    pub const NCVV_CONSIST_CARGO_INFORMATION: Self = Self(2);
    /// Set if the `NewGRF` var 43 currently stored is valid.
    pub const NCVV_COMPANY_INFORMATION: Self = Self(3);
    /// Set if the `NewGRF` var 4D currently stored is valid.
    pub const NCVV_POSITION_IN_VEHICLE: Self = Self(4);
    /// End of the bits.
    pub const NCVV_END: Self = Self(5);
}

/// Models for spawning visual effects.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualEffectSpawnModel(pub u8);

impl VisualEffectSpawnModel {
    /// No visual effect.
    pub const VESM_NONE: Self = Self(0);
    /// Steam model.
    pub const VESM_STEAM: Self = Self(1);
    /// Diesel model.
    pub const VESM_DIESEL: Self = Self(2);
    /// Electric model.
    pub const VESM_ELECTRIC: Self = Self(3);
    /// End of the models.
    pub const VESM_END: Self = Self(4);
}

/// Ground vehicle subtype bit positions. Defined here because common helpers
/// need them; use the subtype access functions rather than accessing flags directly.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroundVehicleSubtypeFlags(pub u8);

impl GroundVehicleSubtypeFlags {
    /// Leading engine of a consist.
    pub const GVSF_FRONT: Self = Self(0);
    /// Articulated part of an engine.
    pub const GVSF_ARTICULATED_PART: Self = Self(1);
    /// Wagon (not used for road vehicles).
    pub const GVSF_WAGON: Self = Self(2);
    /// Engine that can be a front engine, but may be behind another engine
    /// (not used for road vehicles).
    pub const GVSF_ENGINE: Self = Self(3);
    /// First in a wagon chain in a depot (not used for road vehicles).
    pub const GVSF_FREE_WAGON: Self = Self(4);
    /// Engine is multiheaded (not used for road vehicles).
    pub const GVSF_MULTIHEADED: Self = Self(5);
}

/// Raw compatibility bit positions for existing subtype consumers.
pub const GVSF_FRONT: u8 = GroundVehicleSubtypeFlags::GVSF_FRONT.0;
/// Raw compatibility bit position.
#[allow(dead_code)] // Retain the raw compatibility name until cleanup closure.
pub const GVSF_ARTICULATED_PART: u8 = GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART.0;
/// Raw compatibility bit position.
pub const GVSF_WAGON: u8 = GroundVehicleSubtypeFlags::GVSF_WAGON.0;
/// Raw compatibility bit position.
pub const GVSF_ENGINE: u8 = GroundVehicleSubtypeFlags::GVSF_ENGINE.0;
/// Raw compatibility bit position.
pub const GVSF_FREE_WAGON: u8 = GroundVehicleSubtypeFlags::GVSF_FREE_WAGON.0;
/// Raw compatibility bit position.
pub const GVSF_MULTIHEADED: u8 = GroundVehicleSubtypeFlags::GVSF_MULTIHEADED.0;

/// `HasBit(subtype, flag)`.
const fn has(subtype: u8, flag: GroundVehicleSubtypeFlags) -> bool {
    subtype & (1 << flag.0) != 0
}
/// `Vehicle::IsFrontEngine`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_front_engine(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GroundVehicleSubtypeFlags::GVSF_FRONT)
}
/// `Vehicle::IsArticulatedPart`; `ground` is `IsGroundVehicle()`.
pub(crate) const fn is_articulated_part(ground: bool, subtype: u8) -> bool {
    ground && has(subtype, GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART)
}

/// `Vehicle::IsGroundVehicle`: true for a train or a road vehicle.
pub(crate) const fn is_ground_vehicle(kind: u8) -> bool {
    let kind = VehicleType(kind);
    kind.0 == VehicleType::VEH_TRAIN.0 || kind.0 == VehicleType::VEH_ROAD.0
}

/// Raw compatibility mask for `VehState::Hidden`.
pub const VS_HIDDEN: u8 = VehStates::from_element(VehState::Hidden).0;
/// Raw compatibility mask for `VehState::Stopped`.
pub const VS_STOPPED: u8 = VehStates::from_element(VehState::Stopped).0;
/// Raw compatibility mask for `VehState::Unclickable`.
pub const VS_UNCLICKABLE: u8 = VehStates::from_element(VehState::Unclickable).0;
/// Raw compatibility mask for `VehState::TrainSlowing`.
pub const VS_TRAIN_SLOWING: u8 = VehStates::from_element(VehState::TrainSlowing).0;
/// Raw compatibility mask for `VehState::Shadow`.
pub const VS_SHADOW: u8 = VehStates::from_element(VehState::Shadow).0;
/// Raw compatibility mask for `VehState::AircraftBroken`.
pub const VS_AIRCRAFT_BROKEN: u8 = VehStates::from_element(VehState::AircraftBroken).0;
/// Raw compatibility mask for `VehState::Crashed`.
pub const VS_CRASHED: u8 = VehStates::from_element(VehState::Crashed).0;

/// Raw compatibility bit position for `NewGRF` var 40.
pub const NCVV_POSITION_CONSIST_LENGTH: u8 = NewGRFCacheValidValues::NCVV_POSITION_CONSIST_LENGTH.0;
/// Raw compatibility bit position for `NewGRF` var 41.
pub const NCVV_POSITION_SAME_ID_LENGTH: u8 = NewGRFCacheValidValues::NCVV_POSITION_SAME_ID_LENGTH.0;
/// Raw compatibility bit position for `NewGRF` var 42.
pub const NCVV_CONSIST_CARGO_INFORMATION: u8 =
    NewGRFCacheValidValues::NCVV_CONSIST_CARGO_INFORMATION.0;
/// Raw compatibility bit position for `NewGRF` var 43.
pub const NCVV_COMPANY_INFORMATION: u8 = NewGRFCacheValidValues::NCVV_COMPANY_INFORMATION.0;
/// Raw compatibility bit position for `NewGRF` var 4D.
pub const NCVV_POSITION_IN_VEHICLE: u8 = NewGRFCacheValidValues::NCVV_POSITION_IN_VEHICLE.0;
/// Raw compatibility end of the cache-valid bits.
pub const NCVV_END: u8 = NewGRFCacheValidValues::NCVV_END.0;

/// Raw compatibility visual-effect model.
pub const VESM_NONE: u8 = VisualEffectSpawnModel::VESM_NONE.0;
/// Raw compatibility visual-effect model.
pub const VESM_STEAM: u8 = VisualEffectSpawnModel::VESM_STEAM.0;
/// Raw compatibility visual-effect model.
pub const VESM_DIESEL: u8 = VisualEffectSpawnModel::VESM_DIESEL.0;
/// Raw compatibility visual-effect model.
pub const VESM_ELECTRIC: u8 = VisualEffectSpawnModel::VESM_ELECTRIC.0;
/// Raw compatibility end of the visual-effect models.
pub const VESM_END: u8 = VisualEffectSpawnModel::VESM_END.0;

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
        /// Current tile index.
        pub tile: TileIndex,
        /// Tile the vehicle is heading for. For airports and train stations it
        /// need not belong to the destination station, but can estimate distance.
        pub dest_tile: TileIndex,
        /// Counter to occasionally play a vehicle sound.
        pub motion_counter: u32,
        pub x_pos: i32,
        pub y_pos: i32,
        pub z_pos: i32,
        /// Type of engine used for this vehicle.
        pub engine_type: EngineID,
        pub cur_speed: u16,
        pub cargo_cap: u16,
        pub refit_cap: u16,
        /// Ticks until cargo is aged next.
        pub cargo_age_counter: u16,
        pub last_station_visited: StationID,
        /// Maximum consist speed: minimum of all vehicles' maximum speeds.
        pub cached_max_speed: u16,
        /// Number of ticks before carried cargo is aged.
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
        ground: std::ptr::null_mut(),
        train: std::ptr::null_mut(),
        road: std::ptr::null_mut(),
        aircraft: std::ptr::null_mut(),
        consist: std::ptr::null_mut(),
        orders: std::ptr::null_mut(),
        visual: std::ptr::null_mut(),
        previous: std::ptr::null_mut(),
        first: std::ptr::null_mut(),
        tile: TileIndex::INVALID,
        dest_tile: TileIndex::INVALID,
        motion_counter: 0,
        x_pos: 0,
        y_pos: 0,
        z_pos: 0,
        engine_type: EngineID::INVALID,
        cur_speed: 0,
        cargo_cap: 0,
        refit_cap: 0,
        cargo_age_counter: 1,
        last_station_visited: StationID::INVALID,
        cached_max_speed: 0,
        cached_cargo_age_period: 0,
        r#type: kind,
        subtype: 0,
        owner: CompanyID::INVALID,
        direction: crate::direction_type::Direction::INVALID_DIR.0,
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
            // We had an old next vehicle. Update the first and previous pointers.
            let mut v = old;
            while !v.is_null() {
                (*v).first = old;
                v = (*v).next;
            }
            (*old).previous = std::ptr::null_mut();
        }
        (*this).next = next;
        if !next.is_null() {
            // A new next vehicle. Update the first and previous pointers.
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

/// `Vehicle::orders`: the native `OrderList` shell named by the live record's canonical order
/// state, read at the caller's observation point.
pub(crate) fn order_list(v: *mut crate::vehicle::OpenTTDVehicle) -> *mut std::ffi::c_void {
    let state = crate::raw_field::read_field!(v, orders);
    crate::raw_field::read_field!(state, orders)
}

// VehicleRef's live-record contract permits field-sized raw reads between callbacks.
impl crate::vehicle::VehicleRef {
    /// `Vehicle::First`: first vehicle of this chain.
    pub(crate) fn first(self) -> Self {
        Self(std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), first)).unwrap())
    }
    /// `Vehicle::Last`: last vehicle of this chain.
    pub(crate) fn last(self) -> Self {
        let mut v = self;
        while let Some(next) = v.next() {
            v = next;
        }
        v
    }
    /// `Vehicle::GetNextVehicle`: next real (non-articulated part) vehicle.
    pub(crate) fn next_vehicle(self) -> Option<Self> {
        let mut next = self.next();
        while let Some(v) = next {
            if !v.is_articulated_part() {
                break;
            }
            next = v.next();
        }
        // The last articulated part of this engine has now been passed.
        next
    }
    /// `Train::GetNextUnit`: next real vehicle, excluding articulated parts and
    /// the rear part of a dualheaded engine; this record is a train.
    pub(crate) fn next_unit(self) -> Option<Self> {
        let mut next = self.next_vehicle();
        if let Some(v) = next
            && crate::ground_vehicle::is_rear_dualheaded(v.subtype())
        {
            next = v.next_vehicle();
        }
        next
    }
    /// `Vehicle::GetPrevVehicle`: previous real (non-articulated part) vehicle.
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
    /// `Train::GetPrevUnit`: previous real vehicle, excluding articulated parts
    /// and the rear part of a dualheaded engine; this record is a train.
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
    /// `Vehicle::HasEngineType`: whether `engine_type` has any meaning.
    pub(crate) fn has_engine_type(self) -> bool {
        match VehicleType(self.vehicle_type()) {
            VehicleType::VEH_AIRCRAFT => crate::aircraft::is_normal_aircraft(self.subtype()),
            VehicleType::VEH_TRAIN | VehicleType::VEH_ROAD | VehicleType::VEH_SHIP => true,
            _ => false,
        }
    }
    /// `Vehicle::IsFrontEngine`.
    pub(crate) fn is_front_engine(self) -> bool {
        is_front_engine(is_ground_vehicle(self.vehicle_type()), self.subtype())
    }
    /// `Vehicle::GetLastEnginePart`: last part of an articulated engine.
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
    /// `Train::SetOtherMultiheadedPart`; both records are live trains.
    pub(crate) fn set_other_multiheaded_part(self, part: Option<Self>) {
        let train = crate::raw_field::read_field!(self.0.as_ptr(), train);
        crate::raw_field::write_field!(
            train,
            other_multiheaded_part,
            part.map_or(std::ptr::null_mut(), |v| v.0.as_ptr())
        );
    }
    pub(crate) fn ground(self) -> *mut crate::ground_vehicle::GroundState {
        crate::raw_field::read_field!(self.0.as_ptr(), ground)
    }
    /// `Vehicle::GetGroundVehicleFlags`: the flags of this train or road vehicle.
    pub(crate) fn ground_vehicle_flags(self) -> u16 {
        crate::services_assert::check(is_ground_vehicle(self.vehicle_type()));
        crate::raw_field::read_field!(self.ground(), flags)
    }
    /// Store through `Vehicle::GetGroundVehicleFlags` of this train or road vehicle.
    pub(crate) fn write_ground_vehicle_flags(self, value: u16) {
        crate::services_assert::check(is_ground_vehicle(self.vehicle_type()));
        crate::raw_field::write_field!(self.ground(), flags, value);
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
    /// `Vehicle::Next`; articulated parts are also counted as vehicles.
    pub(crate) fn next(self) -> Option<Self> {
        std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), next)).map(Self)
    }
    /// `Vehicle::Previous`; articulated parts are also counted as vehicles.
    pub(crate) fn previous(self) -> Option<Self> {
        std::ptr::NonNull::new(crate::raw_field::read_field!(self.0.as_ptr(), previous)).map(Self)
    }
    pub(crate) fn set_hidden(self, value: bool) {
        self.write_status(if value {
            VehStates(self.status())
                .with(VehStates::from_element(VehState::Hidden))
                .0
        } else {
            VehStates(self.status())
                .without(VehStates::from_element(VehState::Hidden))
                .0
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
    /// `Vehicle::Move(n)` from a nullable record: return the vehicle at the
    /// signed offset, or null if the offset is out of bounds.
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
    /// `Vehicle::GetNextArticulatedPart`: get the next part of an articulated engine.
    /// The vehicle is an articulated engine.
    pub(crate) fn get_next_articulated_part(
        self,
        shared: &crate::services::Services,
    ) -> Option<Self> {
        let map = crate::map_func::World::new(shared);
        if map.types.assertions() {
            map.check(self.has_articulated_part());
        }
        self.next()
    }
    /// `Vehicle::GetFirstEnginePart`: first part of an articulated engine.
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
        let articulated = 1 << GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART.0;
        let rear = 1 << GroundVehicleSubtypeFlags::GVSF_MULTIHEADED.0;
        let engine = 1 << GroundVehicleSubtypeFlags::GVSF_ENGINE.0;
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

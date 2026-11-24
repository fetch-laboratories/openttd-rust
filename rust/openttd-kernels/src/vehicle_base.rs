/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `vehicle_base.h`: the vehicle state and subtype vocabulary, the movement
//! distances, the shared `VehicleRecord` with `Vehicle::SetNext`, and the
//! `Vehicle` members read through `VehicleRef`. `services.cpp` asserts every value.
//!
//! A record lives from its shell's construction until after the base destructor.
//! Both languages access its fields in place under the game-state mutex. Every
//! record pointer and `VehicleRef` used here is live and serialized for its
//! access, and Rust holds no reference to a record across a callback.
#![allow(unsafe_code)]

use crate::base_consist::{VehicleFlag, VehicleFlags};
use crate::bitmath_func::has_bit;
use crate::cargo_type::CargoType;
use crate::company_type::{CompanyID, Owner};
use crate::direction_type::Direction;
use crate::engine_type::EngineID;
use crate::group_type::GroupID;
use crate::raw_field::{read_field, write_field};
use crate::station_type::StationID;
use crate::tile_type::TileIndex;
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_economy::EconomyDate;
use crate::vehicle_type::{VehicleID, VehicleType};

/// Logical length of the tile in any `DiagDirection` used in vehicle movement.
pub const TILE_AXIAL_DISTANCE: u32 = 192;
/// Logical length of the tile corner crossing in any non-diagonal direction used
/// in vehicle movement.
pub const TILE_CORNER_DISTANCE: u32 = 128;

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
    #[allow(dead_code)] // Only native drawing reads it.
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
    /// Native `EnumBitSet::Test`: is the named state set?
    #[must_use]
    pub const fn test(self, element: VehState) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
    /// Native `EnumBitSet::Set`: set the named state.
    #[must_use]
    pub const fn set(self, element: VehState) -> Self {
        self.with(Self::from_element(element))
    }
    /// Native `EnumBitSet::Reset`: reset the named state.
    #[must_use]
    pub const fn reset(self, element: VehState) -> Self {
        self.without(Self::from_element(element))
    }
    /// Native `EnumBitSet::Flip`: flip the named state.
    #[must_use]
    pub const fn flip(self, element: VehState) -> Self {
        if self.test(element) {
            self.reset(element)
        } else {
            self.set(element)
        }
    }
    /// Native `BaseBitSet::Any`: is any state of `other` set?
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
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
    /// Engine that can be front engine, but might be placed behind another engine
    /// (not used for road vehicles).
    pub const GVSF_ENGINE: Self = Self(3);
    /// First in a wagon chain (in depot) (not used for road vehicles).
    pub const GVSF_FREE_WAGON: Self = Self(4);
    /// Engine is multiheaded (not used for road vehicles).
    pub const GVSF_MULTIHEADED: Self = Self(5);
}

/// Structure to return information about the closest depot location,
/// and whether it could be found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClosestDepot {
    pub(crate) location: TileIndex,
    /// The `DestinationID` as used for orders.
    pub(crate) destination: crate::order_type::DestinationID,
    pub(crate) reverse: bool,
    pub(crate) found: bool,
}
impl Default for ClosestDepot {
    /// `ClosestDepot()`: not found.
    fn default() -> Self {
        Self {
            location: TileIndex::INVALID,
            destination: crate::order_type::DestinationID(0),
            reverse: false,
            found: false,
        }
    }
}
impl ClosestDepot {
    /// `ClosestDepot(location, destination, reverse)`: found.
    pub(crate) fn new(
        location: TileIndex,
        destination: crate::order_type::DestinationID,
        reverse: bool,
    ) -> Self {
        Self {
            location,
            destination,
            reverse,
            found: true,
        }
    }
}

/// `Vehicle::IsFrontEngine` over a `subtype`; `ground` is `IsGroundVehicle()`.
pub(crate) fn is_front_engine(ground: bool, subtype: u8) -> bool {
    ground && has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_FRONT.0)
}
/// `Vehicle::IsArticulatedPart` over a `subtype`; `ground` is `IsGroundVehicle()`.
pub(crate) fn is_articulated_part(ground: bool, subtype: u8) -> bool {
    ground && has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART.0)
}

/// `Vehicle::IsStoppedInDepot` of a ground vehicle `v`, whose `IsPrimaryVehicle` is
/// `IsFrontEngine` and whose `IsChainInDepot` is `GroundVehicle::IsChainInDepot`: check
/// whether the vehicle is in the depot *and* stopped.
pub(crate) fn is_stopped_in_depot(
    v: crate::vehicle::VehicleRef,
    physics: &crate::ground_vehicle::Services,
) -> bool {
    crate::services_assert::assert_with(|| v == v.first());
    /* Free wagons have no VehState::Stopped state */
    if v.is_front_engine() && !v.vehstatus().test(VehState::Stopped) {
        return false;
    }
    crate::ground_vehicle::is_chain_in_depot(physics, v)
}

/// `Vehicle::IsGroundVehicle`: is this a ground vehicle (train or road vehicle)?
pub(crate) fn is_ground_vehicle(kind: VehicleType) -> bool {
    kind == VehicleType::VEH_TRAIN || kind == VehicleType::VEH_ROAD
}

/// Raw compatibility names for files no pass has finished; #734's closure removes them.
pub const GVSF_FRONT: u8 = GroundVehicleSubtypeFlags::GVSF_FRONT.0;
pub const GVSF_ENGINE: u8 = GroundVehicleSubtypeFlags::GVSF_ENGINE.0;
pub const VS_HIDDEN: u8 = VehStates::from_element(VehState::Hidden).0;
pub const VS_STOPPED: u8 = VehStates::from_element(VehState::Stopped).0;
pub const VS_UNCLICKABLE: u8 = VehStates::from_element(VehState::Unclickable).0;
pub const VS_SHADOW: u8 = VehStates::from_element(VehState::Shadow).0;
pub const VS_CRASHED: u8 = VehStates::from_element(VehState::Crashed).0;
pub const NCVV_POSITION_CONSIST_LENGTH: u8 = NewGRFCacheValidValues::NCVV_POSITION_CONSIST_LENGTH.0;
pub const NCVV_POSITION_SAME_ID_LENGTH: u8 = NewGRFCacheValidValues::NCVV_POSITION_SAME_ID_LENGTH.0;
pub const NCVV_CONSIST_CARGO_INFORMATION: u8 =
    NewGRFCacheValidValues::NCVV_CONSIST_CARGO_INFORMATION.0;
pub const NCVV_COMPANY_INFORMATION: u8 = NewGRFCacheValidValues::NCVV_COMPANY_INFORMATION.0;
pub const NCVV_POSITION_IN_VEHICLE: u8 = NewGRFCacheValidValues::NCVV_POSITION_IN_VEHICLE.0;
pub const NCVV_END: u8 = NewGRFCacheValidValues::NCVV_END.0;
pub const VESM_NONE: u8 = VisualEffectSpawnModel::VESM_NONE.0;
pub const VESM_STEAM: u8 = VisualEffectSpawnModel::VESM_STEAM.0;
pub const VESM_DIESEL: u8 = VisualEffectSpawnModel::VESM_DIESEL.0;
pub const VESM_ELECTRIC: u8 = VisualEffectSpawnModel::VESM_ELECTRIC.0;
pub const VESM_END: u8 = VisualEffectSpawnModel::VESM_END.0;

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
        /// `Vehicle::next`: pointer to the next vehicle in the chain.
        pub next: *mut VehicleRecord,
        /// Stable owner links, initialized by the corresponding native constructors.
        pub ground: *mut crate::ground_vehicle::GroundState,
        pub train: *mut crate::train_state::TrainState,
        pub road: *mut crate::road::OpenTTDRoadState,
        pub aircraft: *mut crate::aircraft::AircraftState,
        pub consist: *mut crate::orders::Consist,
        pub orders: *mut crate::orders::VehicleOrders,
        pub visual: *mut u8,
        /// NOSAVE `Vehicle::previous`: pointer to the previous vehicle in the chain.
        pub previous: *mut VehicleRecord,
        /// NOSAVE `Vehicle::first`: pointer to the first vehicle in the chain.
        pub first: *mut VehicleRecord,
        /// Current tile index.
        pub tile: TileIndex,
        /// Heading for this tile. For airports and train stations this tile does
        /// not necessarily belong to the destination station, but it can be used
        /// for heuristic purposes to estimate the distance.
        pub dest_tile: TileIndex,
        /// Counter to occasionally play a vehicle sound.
        pub motion_counter: u32,
        /// x coordinate.
        pub x_pos: i32,
        /// y coordinate.
        pub y_pos: i32,
        /// z coordinate.
        pub z_pos: i32,
        /// The type of engine used for this vehicle.
        pub engine_type: EngineID,
        /// Current speed.
        pub cur_speed: u16,
        /// Total capacity.
        pub cargo_cap: u16,
        /// Capacity left over from before last refit.
        pub refit_cap: u16,
        /// Ticks till cargo is aged next.
        pub cargo_age_counter: u16,
        /// The last station we stopped at.
        pub last_station_visited: StationID,
        /// `VehicleCache::cached_max_speed`: maximum speed of the consist (minimum
        /// of the max speed of all vehicles in the consist).
        pub cached_max_speed: u16,
        /// `VehicleCache::cached_cargo_age_period`: number of ticks before carried
        /// cargo is aged.
        pub cached_cargo_age_period: u16,
        /// `BaseVehicle::type`: type of vehicle.
        pub r#type: VehicleType,
        /// Subtype (filled with values from `AircraftSubType`, `DisasterSubType`,
        /// `EffectVehicleType` or `GroundVehicleSubtypeFlags`).
        pub subtype: u8,
        /// Which company owns the vehicle?
        pub owner: CompanyID,
        /// Facing.
        pub direction: Direction,
        /// Status.
        pub vehstatus: VehStates,
        /// Fractional speed.
        pub subspeed: u8,
        /// Used by train and aircraft.
        pub acceleration: u8,
        /// The percentage (if divided by 256) this vehicle already crossed the tile unit.
        pub progress: u8,
        /// Increased by one for each tick.
        pub tick_counter: u8,
        /// Number of ticks this vehicle was not stopped this day.
        pub running_ticks: u8,
        /// Increased by one for each day.
        pub day_counter: u8,
        /// Counter for managing breakdown events; see `Vehicle::HandleBreakdown`.
        pub breakdown_ctr: u8,
    }
}

impl VehicleRecord {
    /// Allocate `shell`'s record with the original member and constructor defaults;
    /// `first` points to the record itself.
    pub(crate) fn new(shell: *mut OpenTTDVehicleShell, kind: VehicleType) -> *mut Self {
        let record = Box::into_raw(Box::new(Self {
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
            direction: Direction::INVALID_DIR,
            vehstatus: VehStates(0),
            subspeed: 0,
            acceleration: 0,
            progress: 0,
            tick_counter: 0,
            running_ticks: 0,
            day_counter: 0,
            breakdown_ctr: 0,
        }));
        write_field!(record, first, record);
        record
    }

    /// Release a record after its shell's base destructor.
    ///
    /// # Safety
    /// `record` comes from `VehicleRecord::new` and is released once.
    pub(crate) unsafe fn delete(record: *mut Self) {
        // SAFETY: The owning shell transfers the allocation exactly once.
        unsafe { drop(Box::from_raw(record)) }
    }
}

/// `VehicleRecord::new` for the native shell constructor.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_vehicle_record_new(
    shell: *mut OpenTTDVehicleShell,
    kind: VehicleType,
) -> *mut VehicleRecord {
    VehicleRecord::new(shell, kind)
}

/// `VehicleRecord::delete` for the native shell destructor.
///
/// # Safety
/// `record` comes from `openttd_rust_vehicle_record_new` and is released once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_record_delete(record: *mut VehicleRecord) {
    // SAFETY: The caller transfers the allocation exactly once.
    unsafe { VehicleRecord::delete(record) }
}

/// `Vehicle::SetNext` for the native facade.
///
/// # Safety
/// As `set_next`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_set_next(
    this: *mut VehicleRecord,
    next: *mut VehicleRecord,
) {
    // SAFETY: The caller keeps `set_next`'s contract.
    unsafe { set_next(this, next) };
}

/// `Vehicle::SetNext`: set the next vehicle of this vehicle; a null `next`
/// removes the next vehicle.
///
/// # Safety
/// `this` and non-null `next` are live records, and every record reachable
/// through their links is live.
pub(crate) unsafe fn set_next(this: *mut VehicleRecord, next: *mut VehicleRecord) {
    if this == next {
        crate::services_assert::check(false);
    }

    let old = read_field!(this, next);
    if !old.is_null() {
        // We had an old next vehicle. Update the first and previous pointers.
        let mut v = old;
        while !v.is_null() {
            write_field!(v, first, old);
            v = record_of(crate::vehicle::VehicleRef::of(v).next());
        }
        write_field!(old, previous, std::ptr::null_mut());
    }

    write_field!(this, next, next);

    if !next.is_null() {
        // A new next vehicle. Update the first and previous pointers.
        let predecessor = read_field!(next, previous);
        if !predecessor.is_null() {
            write_field!(predecessor, next, std::ptr::null_mut());
        }
        write_field!(next, previous, this);
        let mut v = next;
        while !v.is_null() {
            write_field!(v, first, read_field!(this, first));
            v = record_of(crate::vehicle::VehicleRef::of(v).next());
        }
    }
}

/// A nullable record pointer from an optional handle, as the original's `Vehicle *`.
fn record_of(v: Option<crate::vehicle::VehicleRef>) -> *mut VehicleRecord {
    v.map_or(std::ptr::null_mut(), |v| v.0.as_ptr())
}

/// `Vehicle::orders`: the native `OrderList` shell named by the live record's canonical order
/// state, read at the caller's observation point.
pub(crate) fn order_list(v: *mut crate::vehicle_type::OpenTTDVehicle) -> *mut std::ffi::c_void {
    let state = crate::raw_field::read_field!(v, orders);
    crate::raw_field::read_field!(state, orders)
}

impl crate::vehicle::VehicleRef {
    /// `Vehicle::First`: get the first vehicle of this vehicle chain.
    pub(crate) fn first(self) -> Self {
        // The original dereferences `this->first` unchecked; every live record has one.
        self.first_link().unwrap()
    }
    /// `Vehicle::First` as stored: the `first` link, `None` where a broken cache holds null.
    pub(crate) fn first_link(self) -> Option<Self> {
        std::ptr::NonNull::new(read_field!(self.0.as_ptr(), first)).map(Self)
    }
    /// `Vehicle::Last`: get the last vehicle of this vehicle chain.
    pub(crate) fn last(self) -> Self {
        let mut v = self;
        while let Some(next) = v.next() {
            v = next;
        }
        v
    }
    /// `Vehicle::Next`: get the next vehicle of this vehicle; articulated parts
    /// are also counted as vehicles.
    pub(crate) fn next(self) -> Option<Self> {
        std::ptr::NonNull::new(read_field!(self.0.as_ptr(), next)).map(Self)
    }
    /// `Vehicle::Next` where the original dereferences the result without a test.
    /// # Safety
    /// The caller's chain invariant guarantees that a part follows this one.
    pub(crate) unsafe fn next_unchecked(self) -> Self {
        // SAFETY: The caller guarantees the link `next` loads is non-null.
        unsafe { self.next().unwrap_unchecked() }
    }
    /// `Vehicle::Previous`: get the previous vehicle of this vehicle; articulated
    /// parts are also counted as vehicles.
    pub(crate) fn previous(self) -> Option<Self> {
        std::ptr::NonNull::new(read_field!(self.0.as_ptr(), previous)).map(Self)
    }
    /// `Vehicle::Move(n)` from a nullable record: get the vehicle at offset `n`
    /// of this vehicle chain, or null if the offset is out of bounds.
    pub(crate) fn move_by(mut v: *mut VehicleRecord, n: i32) -> *mut VehicleRecord {
        if n < 0 {
            let mut i = 0;
            while i != n && !v.is_null() {
                v = record_of(Self::of(v).previous());
                i -= 1;
            }
        } else {
            let mut i = 0;
            while i != n && !v.is_null() {
                v = record_of(Self::of(v).next());
                i += 1;
            }
        }
        v
    }
    /// `Vehicle::IsFrontEngine`: is the vehicle a front engine?
    pub(crate) fn is_front_engine(self) -> bool {
        is_ground_vehicle(self.vehicle_type())
            && has_bit(self.subtype(), GroundVehicleSubtypeFlags::GVSF_FRONT.0)
    }
    /// `Vehicle::IsArticulatedPart`: is the vehicle an articulated part of an engine?
    pub(crate) fn is_articulated_part(self) -> bool {
        is_ground_vehicle(self.vehicle_type())
            && has_bit(
                self.subtype(),
                GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART.0,
            )
    }
    /// `Vehicle::HasArticulatedPart`: does the engine have an articulated part?
    pub(crate) fn has_articulated_part(self) -> bool {
        self.next().is_some_and(Self::is_articulated_part)
    }
    /// `Vehicle::GetNextArticulatedPart`: get the next part of an articulated engine.
    ///
    /// Precondition: the vehicle is an articulated engine.
    pub(crate) fn get_next_articulated_part(self) -> Option<Self> {
        crate::services_assert::assert_with(|| self.has_articulated_part());
        self.next()
    }
    /// `Vehicle::GetFirstEnginePart`: get the first part of an articulated engine.
    pub(crate) fn first_engine_part(self) -> Self {
        let mut v = self;
        while v.is_articulated_part() {
            // The original dereferences `Previous()` unchecked; an articulated part
            // always follows its engine.
            v = v.previous().unwrap();
        }
        v
    }
    /// `Vehicle::GetLastEnginePart`: get the last part of an articulated engine.
    pub(crate) fn last_engine_part(self) -> Self {
        let mut v = self;
        while v.has_articulated_part() {
            // `HasArticulatedPart()` has just found `Next()`.
            v = v.get_next_articulated_part().unwrap();
        }
        v
    }
    /// `Vehicle::GetNextVehicle`: get the next real (non-articulated part) vehicle
    /// in the consist.
    pub(crate) fn next_vehicle(self) -> Option<Self> {
        let mut v = self;
        while v.has_articulated_part() {
            // `HasArticulatedPart()` has just found `Next()`.
            v = v.get_next_articulated_part().unwrap();
        }

        // v now contains the last articulated part in the engine.
        v.next()
    }
    /// `Vehicle::GetPrevVehicle`: get the previous real (non-articulated part)
    /// vehicle in the consist.
    pub(crate) fn previous_vehicle(self) -> Option<Self> {
        let mut v = self.previous();
        while let Some(part) = v
            && part.is_articulated_part()
        {
            v = part.previous();
        }

        v
    }
    /// `Train::GetNextUnit`: get the next real (non-articulated part and non rear
    /// part of dualheaded engine) vehicle in the consist; this record is a train.
    pub(crate) fn next_unit(self) -> Option<Self> {
        let mut v = self.next_vehicle();
        if let Some(u) = v
            && crate::ground_vehicle::is_rear_dualheaded(u.subtype())
        {
            v = u.next_vehicle();
        }

        v
    }
    /// `Train::GetPrevUnit`: get the previous real (non-articulated part and non
    /// rear part of dualheaded engine) vehicle in the consist; this record is a train.
    pub(crate) fn previous_unit(self) -> Option<Self> {
        let mut v = self.previous_vehicle();
        if let Some(u) = v
            && crate::ground_vehicle::is_rear_dualheaded(u.subtype())
        {
            v = u.previous_vehicle();
        }

        v
    }
    /// `Vehicle::HasEngineType`: check whether `engine_type` has any meaning.
    pub(crate) fn has_engine_type(self) -> bool {
        match self.vehicle_type() {
            VehicleType::VEH_AIRCRAFT => crate::aircraft::is_normal_aircraft(self.subtype()),
            VehicleType::VEH_TRAIN | VehicleType::VEH_ROAD | VehicleType::VEH_SHIP => true,
            _ => false,
        }
    }
    /// `Vehicle::GetOldAdvanceSpeed`: determines the effective direction-specific vehicle
    /// movement speed.
    ///
    /// This method belongs to the old vehicle movement method:
    /// A vehicle moves a step every 256 progress units.
    /// The vehicle speed is scaled by 3/4 when moving in X or Y direction due to the longer
    /// distance.
    ///
    /// However, this method is slightly wrong in corners, as the leftover progress is not
    /// scaled correctly when changing movement direction. `GetAdvanceSpeed()` and
    /// `GetAdvanceDistance()` are better wrt. this.
    ///
    /// `speed` is the direction-independent unscaled speed; returns the speed scaled by
    /// movement direction. 256 units are required for each movement step.
    pub(crate) fn get_old_advance_speed(self, speed: u32) -> u32 {
        if self.direction().0 & 1 != 0 {
            speed
        } else {
            speed.wrapping_mul(3) / 4
        }
    }
    /// `Vehicle::GetAdvanceSpeed`: determines the effective vehicle movement speed.
    ///
    /// Together with `GetAdvanceDistance()` this function is a replacement for
    /// `GetOldAdvanceSpeed()`.
    ///
    /// A vehicle progresses independent of it's movement direction. However different amounts
    /// of "progress" are needed for moving a step in a specific direction. That way the
    /// leftover progress does not need any adaption when changing movement direction.
    ///
    /// `speed` is the direction-independent unscaled speed; returns the speed scaled to match
    /// `GetAdvanceDistance()`.
    #[inline]
    pub(crate) const fn get_advance_speed(speed: u32) -> u32 {
        speed.wrapping_mul(3) / 4
    }
    /// `Vehicle::GetAdvanceDistance`: determines the vehicle "progress" needed for moving a
    /// step.
    ///
    /// Together with `GetAdvanceSpeed()` this function is a replacement for
    /// `GetOldAdvanceSpeed()`. Returns the distance to drive for a movement step on the map.
    #[inline]
    pub(crate) fn get_advance_distance(self) -> u32 {
        if self.direction().0 & 1 != 0 {
            TILE_AXIAL_DISTANCE
        } else {
            TILE_CORNER_DISTANCE * 2
        }
    }
    /// `Vehicle::GetGroundVehicleFlags`: access the ground vehicle flags of the vehicle.
    /// The original returns `uint16_t &`: `GroundVehicleFlags` is an enum of bit
    /// numbers with no set type.
    ///
    /// Precondition: the vehicle is a `GroundVehicle`.
    pub(crate) fn ground_vehicle_flags(self) -> u16 {
        crate::services_assert::check(is_ground_vehicle(self.vehicle_type()));
        read_field!(self.ground(), flags)
    }
    /// Store through `Vehicle::GetGroundVehicleFlags`.
    ///
    /// Precondition: the vehicle is a `GroundVehicle`.
    pub(crate) fn set_ground_vehicle_flags(self, value: u16) {
        crate::services_assert::check(is_ground_vehicle(self.vehicle_type()));
        write_field!(self.ground(), flags, value);
    }
    /// `v->vehstatus.Set(VehState::Hidden)` or `.Reset(VehState::Hidden)`.
    pub(crate) fn set_hidden(self, value: bool) {
        let status = self.vehstatus();
        self.set_vehstatus(if value {
            status.set(VehState::Hidden)
        } else {
            status.reset(VehState::Hidden)
        });
    }

    /// The road vehicle owner link; the record is a live road vehicle.
    pub(crate) fn road(self) -> *mut crate::road::State {
        // `OpenTTDRoadState` is the first field of the `#[repr(C)]` road owner.
        read_field!(self.0.as_ptr(), road).cast()
    }
    /// The ground vehicle owner link; the record is a live train or road vehicle.
    pub(crate) fn ground(self) -> *mut crate::ground_vehicle::GroundState {
        read_field!(self.0.as_ptr(), ground)
    }
    /// The ground vehicle cache of a live train or road vehicle.
    pub(crate) fn ground_cache(self) -> *mut crate::ground_vehicle::GroundCache {
        let ground = self.ground();
        // SAFETY: Ground shells bind a live owner before controller entries.
        unsafe { std::ptr::addr_of_mut!((*ground).cache) }
    }
    /// The aircraft owner link; the record is a live aircraft.
    pub(crate) fn aircraft_state(self) -> *mut crate::aircraft::AircraftState {
        read_field!(self.0.as_ptr(), aircraft)
    }
    /// `Train::other_multiheaded_part`; the record is a live train.
    pub(crate) fn other_multiheaded_part(self) -> Option<Self> {
        let train = read_field!(self.0.as_ptr(), train);
        std::ptr::NonNull::new(read_field!(train, other_multiheaded_part)).map(Self)
    }
    /// `Train::SetOtherMultiheadedPart`; both records are live trains.
    pub(crate) fn set_other_multiheaded_part(self, part: Option<Self>) {
        let train = read_field!(self.0.as_ptr(), train);
        write_field!(
            train,
            other_multiheaded_part,
            part.map_or(std::ptr::null_mut(), |v| v.0.as_ptr())
        );
    }

    /// `Vehicle::current_order`: the current order (+ status, like: loading),
    /// read in place.
    pub(crate) fn current_order(self) -> crate::orders::OrderView<'static> {
        // SAFETY: Vehicle construction starts the order's lifetime before publication.
        unsafe { crate::orders::OrderView::of(self.current_order_pointer()) }
    }
    /// `&Vehicle::current_order`, for its writers.
    pub(crate) fn current_order_pointer(self) -> *mut crate::orders::Order {
        let orders = read_field!(self.0.as_ptr(), orders);
        // SAFETY: The native Order lifetime begins during Vehicle construction.
        unsafe { std::ptr::addr_of_mut!((*orders).current).cast() }
    }
    /// `Vehicle::cur_real_order_index`: the index of the current real order
    /// (non-implicit, non-implicit-only).
    pub(crate) fn cur_real_order_index(self) -> crate::order_type::VehicleOrderID {
        let consist = read_field!(self.0.as_ptr(), consist);
        crate::order_type::VehicleOrderID(read_field!(consist, real))
    }
    /// Virtual `Vehicle::GetDisplayMaxSpeed`: the maximum speed in km-ish/h as displayed to
    /// the player, dispatched on the vehicle's type to each override; other types take the
    /// base class's 0.
    pub(crate) fn get_display_max_speed(self) -> i32 {
        use crate::ground_vehicle::{GroundVehicleType, RoadVehicle, Train};
        match self.vehicle_type() {
            VehicleType::VEH_TRAIN => Train::get_display_max_speed(self),
            VehicleType::VEH_ROAD => RoadVehicle::get_display_max_speed(self),
            // `Ship::GetDisplayMaxSpeed`.
            VehicleType::VEH_SHIP => i32::from(self.cached_max_speed()) / 2,
            // `Aircraft::GetDisplayMaxSpeed`.
            VehicleType::VEH_AIRCRAFT => i32::from(self.cached_max_speed()),
            _ => 0,
        }
    }
    /// `Vehicle::cur_implicit_order_index`: the index to the current implicit order.
    pub(crate) fn cur_implicit_order_index(self) -> crate::order_type::VehicleOrderID {
        let consist = read_field!(self.0.as_ptr(), consist);
        crate::order_type::VehicleOrderID(read_field!(consist, implicit))
    }
    /// `Vehicle::lateness_counter`: how many ticks late (or early if negative) this vehicle is.
    pub(crate) fn lateness_counter(self) -> i32 {
        let consist = read_field!(self.0.as_ptr(), consist);
        read_field!(consist, lateness)
    }
    /// `Vehicle::current_order_time`: how many ticks have passed since this order started.
    pub(crate) fn current_order_time(self) -> i32 {
        let consist = read_field!(self.0.as_ptr(), consist);
        read_field!(consist, time)
    }
    pub(crate) fn set_current_order_time(self, value: i32) {
        let consist = read_field!(self.0.as_ptr(), consist);
        write_field!(consist, time, value);
    }
    /// `Vehicle::vcache.cached_vis_effect`: the visual effect to show.
    pub(crate) fn cached_vis_effect(self) -> u8 {
        let visual = read_field!(self.0.as_ptr(), visual);
        // SAFETY: The bound visual owner lives throughout the source access interval.
        unsafe { visual.read() }
    }
    /// `Vehicle::unitnumber`, a field still stored in the C++ shell, read
    /// through the vehicle resolver's field service.
    pub(crate) fn unitnumber(self, shared: &crate::services::Services) -> u16 {
        let vehicles = crate::movement::table(shared).vehicle();
        // SAFETY: The vehicle table returns the static resolver service table.
        unsafe { ((*(vehicles.newgrf)()).unit)(self.0.as_ptr()) }
    }

    // `Vehicle` members still stored in the C++ shell, one C call each. A reader takes the
    // resolved table its slot is in and resolves no table itself.

    /// `Vehicle::age`: age in calendar days.
    #[inline]
    pub(crate) fn age(self, r: &crate::newgrf_vehicle::Services) -> CalendarDate {
        // SAFETY: The resolver slot reads the live record's shell.
        CalendarDate(unsafe { (r.age)(self.0.as_ptr()) })
    }
    /// Store `Vehicle::age`.
    #[inline]
    pub(crate) fn set_age(self, m: &crate::vehicle::maintenance::Services, value: CalendarDate) {
        (m.set_age)(self, value.0);
    }
    /// `Vehicle::max_age`: maximum age.
    #[inline]
    pub(crate) fn max_age(self, r: &crate::newgrf_vehicle::Services) -> CalendarDate {
        // SAFETY: As `age`.
        CalendarDate(unsafe { (r.max_age)(self.0.as_ptr()) })
    }
    /// `Vehicle::breakdown_chance`: current chance of breakdowns.
    #[inline]
    pub(crate) fn breakdown_chance(self, r: &crate::newgrf_vehicle::Services) -> u8 {
        // SAFETY: As `age`.
        unsafe { (r.breakdown_chance)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::breakdown_chance`.
    #[inline]
    pub(crate) fn set_breakdown_chance(self, m: &crate::vehicle::maintenance::Services, value: u8) {
        (m.set_breakdown_chance)(self, value);
    }
    /// `Vehicle::breakdown_delay`: counter for managing breakdown length.
    #[inline]
    pub(crate) fn breakdown_delay(self, r: &crate::newgrf_vehicle::Services) -> u8 {
        // SAFETY: As `age`.
        unsafe { (r.breakdown_delay)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::breakdown_delay`.
    #[inline]
    pub(crate) fn set_breakdown_delay(self, m: &crate::vehicle::maintenance::Services, value: u8) {
        (m.set_breakdown_delay)(self, value);
    }
    /// `Vehicle::breakdowns_since_last_service`: counter for the amount of breakdowns.
    #[inline]
    pub(crate) fn breakdowns_since_last_service(self, r: &crate::newgrf_vehicle::Services) -> u8 {
        // SAFETY: As `age`.
        unsafe { (r.breakdowns)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::breakdowns_since_last_service`.
    #[inline]
    pub(crate) fn set_breakdowns_since_last_service(
        self,
        base: &crate::vehicle::VehicleServices,
        value: u8,
    ) {
        (base.write_breakdowns)(self, value);
    }
    /// `Vehicle::date_of_last_service`: last economy date the vehicle had a service at a depot.
    #[inline]
    pub(crate) fn date_of_last_service(self, r: &crate::newgrf_vehicle::Services) -> EconomyDate {
        // SAFETY: As `age`.
        EconomyDate(unsafe { (r.date_of_last_service)(self.0.as_ptr()) })
    }
    /// Store `Vehicle::date_of_last_service`.
    #[inline]
    pub(crate) fn set_date_of_last_service(
        self,
        base: &crate::vehicle::VehicleServices,
        value: EconomyDate,
    ) {
        (base.write_economy_service)(self, value.0);
    }
    /// Store `Vehicle::date_of_last_service_newgrf`: last calendar date the vehicle had a
    /// service at a depot, unchanged by the date cheat to protect `NewGRF` behaviour.
    #[inline]
    pub(crate) fn set_date_of_last_service_newgrf(
        self,
        base: &crate::vehicle::VehicleServices,
        value: CalendarDate,
    ) {
        (base.write_calendar_service)(self, value.0);
    }
    /// `Vehicle::economy_age`: age in economy days.
    #[inline]
    pub(crate) fn economy_age(self, m: &crate::vehicle::maintenance::Services) -> EconomyDate {
        EconomyDate((m.economy_age)(self))
    }
    /// Store `Vehicle::economy_age`.
    #[inline]
    pub(crate) fn set_economy_age(
        self,
        m: &crate::vehicle::maintenance::Services,
        value: EconomyDate,
    ) {
        (m.set_economy_age)(self, value.0);
    }
    /// `Vehicle::group_id`: index of group pool array item.
    #[inline]
    pub(crate) fn group_id(self, g: &crate::fleet::GroupServices) -> GroupID {
        // SAFETY: The group slot reads the live record's shell.
        GroupID(unsafe { (g.vehicle_group)(self.shell().cast()) })
    }
    /// `Vehicle::index`.
    #[inline]
    pub(crate) fn index(self, r: &crate::newgrf_vehicle::Services) -> VehicleID {
        // SAFETY: As `age`.
        VehicleID(unsafe { (r.id)(self.0.as_ptr()) })
    }
    /// `Vehicle::profit_this_year`: profit this year << 8, low 8 bits are fract.
    #[inline]
    pub(crate) fn profit_this_year(self, base: &crate::vehicle::VehicleServices) -> i64 {
        (base.profit)(self)
    }
    /// Store `Vehicle::profit_this_year`.
    #[inline]
    pub(crate) fn set_profit_this_year(self, base: &crate::vehicle::VehicleServices, value: i64) {
        (base.write_profit)(self, value);
    }
    /// `Vehicle::reliability`: reliability.
    #[inline]
    pub(crate) fn reliability(self, r: &crate::newgrf_vehicle::Services) -> u16 {
        // SAFETY: As `age`.
        unsafe { (r.reliability)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::reliability`.
    #[inline]
    pub(crate) fn set_reliability(self, base: &crate::vehicle::VehicleServices, value: u16) {
        (base.write_reliability)(self, value);
    }
    /// `Vehicle::reliability_spd_dec`: reliability decrease speed.
    #[inline]
    pub(crate) fn reliability_spd_dec(self, r: &crate::newgrf_vehicle::Services) -> u16 {
        // SAFETY: As `age`.
        unsafe { (r.reliability_decay)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::reliability_spd_dec`.
    #[inline]
    pub(crate) fn set_reliability_spd_dec(
        self,
        m: &crate::vehicle::maintenance::Services,
        value: u16,
    ) {
        (m.set_reliability_spd_dec)(self, value);
    }
    /// `Vehicle::value`: value of the vehicle.
    #[inline]
    pub(crate) fn value(self, r: &crate::newgrf_vehicle::Services) -> i64 {
        // SAFETY: As `age`.
        unsafe { (r.value)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::value`.
    #[inline]
    pub(crate) fn set_value(self, m: &crate::vehicle::maintenance::Services, value: i64) {
        (m.set_value)(self, value);
    }
    /// `Vehicle::GetServiceInterval`.
    #[inline]
    pub(crate) fn get_service_interval(self, r: &crate::newgrf_vehicle::Services) -> u16 {
        // SAFETY: As `age`.
        unsafe { (r.service_interval)(self.0.as_ptr()) }
    }
    /// `Vehicle::cargo_type`: type of cargo this vehicle is carrying.
    #[inline]
    pub(crate) fn cargo_type(self, r: &crate::newgrf_vehicle::Services) -> CargoType {
        // SAFETY: As `age`.
        CargoType(unsafe { (r.cargo_type)(self.0.as_ptr()) })
    }
    /// Store `Vehicle::cargo_type`.
    #[inline]
    pub(crate) fn set_cargo_type(self, base: &crate::vehicle::VehicleServices, value: CargoType) {
        (base.set_cargo_type)(self, value.0);
    }
    /// `Vehicle::cargo_subtype`: used for livery refits (`NewGRF` variations).
    #[inline]
    pub(crate) fn cargo_subtype(self, r: &crate::newgrf_vehicle::Services) -> u8 {
        // SAFETY: As `age`.
        unsafe { (r.cargo_subtype)(self.0.as_ptr()) }
    }
    /// Store `Vehicle::cargo_subtype`.
    #[inline]
    pub(crate) fn set_cargo_subtype(self, base: &crate::vehicle::VehicleServices, value: u8) {
        (base.set_cargo_subtype)(self, value);
    }
    /// `Vehicle::last_loading_station`: last station the vehicle has stopped at and could
    /// possibly leave from with any cargo loaded.
    #[inline]
    pub(crate) fn last_loading_station(self, s: &crate::vehicle::visits::Services) -> StationID {
        StationID((s.last_loading_station)(self))
    }
    /// Store `Vehicle::last_loading_station`.
    #[inline]
    pub(crate) fn set_last_loading_station(
        self,
        s: &crate::vehicle::visits::Services,
        value: StationID,
    ) {
        (s.set_last_loading_station)(self, value.0);
    }
    /// `Vehicle::last_loading_tick`: last `TimerGameTick::counter` tick that the vehicle has
    /// stopped at a station and could possibly leave with any cargo loaded.
    #[inline]
    pub(crate) fn last_loading_tick(self, s: &crate::vehicle::visits::Services) -> u64 {
        (s.last_loading_tick)(self)
    }
    /// Store `Vehicle::last_loading_tick`.
    #[inline]
    pub(crate) fn set_last_loading_tick(self, s: &crate::vehicle::visits::Services, value: u64) {
        (s.set_last_loading_tick)(self, value);
    }
    /// Store `Vehicle::trip_occupancy`: occupancy of vehicle of the current trip (updated
    /// after leaving a station).
    #[inline]
    pub(crate) fn set_trip_occupancy(self, s: &crate::vehicle::visits::Services, value: i8) {
        (s.set_trip_occupancy)(self, value);
    }
    /// `Vehicle::vehicle_flags`, read in place from the record's consist state.
    #[inline]
    pub(crate) fn vehicle_flags(self) -> VehicleFlags {
        let consist = read_field!(self.0.as_ptr(), consist);
        VehicleFlags(read_field!(consist, flags))
    }
    /// `Vehicle::ServiceIntervalIsPercent`.
    #[inline]
    pub(crate) fn service_interval_is_percent(self) -> bool {
        self.vehicle_flags()
            .test(VehicleFlag::SERVICE_INTERVAL_IS_PERCENT)
    }

    #[inline]
    pub(crate) fn tile(self) -> TileIndex {
        read_field!(self.0.as_ptr(), tile)
    }
    #[inline]
    pub(crate) fn tile_raw(self) -> u32 {
        self.tile().0
    }
    #[inline]
    pub(crate) fn set_tile(self, value: TileIndex) {
        write_field!(self.0.as_ptr(), tile, value);
    }
    #[inline]
    pub(crate) fn set_tile_raw(self, value: u32) {
        self.set_tile(TileIndex(value));
    }
    #[inline]
    pub(crate) fn dest_tile(self) -> TileIndex {
        read_field!(self.0.as_ptr(), dest_tile)
    }
    #[inline]
    pub(crate) fn dest_tile_raw(self) -> u32 {
        self.dest_tile().0
    }
    #[inline]
    pub(crate) fn set_dest_tile(self, value: TileIndex) {
        write_field!(self.0.as_ptr(), dest_tile, value);
    }
    #[inline]
    pub(crate) fn set_dest_tile_raw(self, value: u32) {
        self.set_dest_tile(TileIndex(value));
    }
    pub(crate) fn motion_counter(self) -> u32 {
        read_field!(self.0.as_ptr(), motion_counter)
    }
    pub(crate) fn x_pos(self) -> i32 {
        read_field!(self.0.as_ptr(), x_pos)
    }
    pub(crate) fn set_x_pos(self, value: i32) {
        write_field!(self.0.as_ptr(), x_pos, value);
    }
    pub(crate) fn y_pos(self) -> i32 {
        read_field!(self.0.as_ptr(), y_pos)
    }
    pub(crate) fn set_y_pos(self, value: i32) {
        write_field!(self.0.as_ptr(), y_pos, value);
    }
    pub(crate) fn z_pos(self) -> i32 {
        read_field!(self.0.as_ptr(), z_pos)
    }
    pub(crate) fn set_z_pos(self, value: i32) {
        write_field!(self.0.as_ptr(), z_pos, value);
    }
    /// Store `x_pos`, then `y_pos`.
    pub(crate) fn set_xy(self, x: i32, y: i32) {
        self.set_x_pos(x);
        self.set_y_pos(y);
    }
    /// Store `x_pos`, `y_pos`, then `z_pos`.
    pub(crate) fn set_coordinates(self, x: i32, y: i32, z: i32) {
        self.set_xy(x, y);
        self.set_z_pos(z);
    }
    #[inline]
    pub(crate) fn engine_type(self) -> EngineID {
        read_field!(self.0.as_ptr(), engine_type)
    }
    #[inline]
    pub(crate) fn engine_type_raw(self) -> u16 {
        self.engine_type().0
    }
    #[inline]
    pub(crate) fn set_engine_type(self, value: EngineID) {
        write_field!(self.0.as_ptr(), engine_type, value);
    }
    #[inline]
    pub(crate) fn set_engine_type_raw(self, value: u16) {
        self.set_engine_type(EngineID(value));
    }
    pub(crate) fn cur_speed(self) -> u16 {
        read_field!(self.0.as_ptr(), cur_speed)
    }
    pub(crate) fn set_cur_speed(self, value: u16) {
        write_field!(self.0.as_ptr(), cur_speed, value);
    }
    pub(crate) fn cargo_cap(self) -> u16 {
        read_field!(self.0.as_ptr(), cargo_cap)
    }
    pub(crate) fn set_cargo_cap(self, value: u16) {
        write_field!(self.0.as_ptr(), cargo_cap, value);
    }
    pub(crate) fn refit_cap(self) -> u16 {
        read_field!(self.0.as_ptr(), refit_cap)
    }
    pub(crate) fn set_refit_cap(self, value: u16) {
        write_field!(self.0.as_ptr(), refit_cap, value);
    }
    #[inline]
    pub(crate) fn last_station_visited(self) -> StationID {
        read_field!(self.0.as_ptr(), last_station_visited)
    }
    #[inline]
    pub(crate) fn last_station_visited_raw(self) -> u16 {
        self.last_station_visited().0
    }
    #[inline]
    pub(crate) fn set_last_station_visited(self, value: StationID) {
        write_field!(self.0.as_ptr(), last_station_visited, value);
    }
    #[inline]
    pub(crate) fn set_last_station_visited_raw(self, value: u16) {
        self.set_last_station_visited(StationID(value));
    }
    pub(crate) fn cached_max_speed(self) -> u16 {
        read_field!(self.0.as_ptr(), cached_max_speed)
    }
    pub(crate) fn set_cached_max_speed(self, value: u16) {
        write_field!(self.0.as_ptr(), cached_max_speed, value);
    }
    pub(crate) fn set_cached_cargo_age_period(self, value: u16) {
        write_field!(self.0.as_ptr(), cached_cargo_age_period, value);
    }
    #[inline]
    pub(crate) fn vehicle_type(self) -> VehicleType {
        read_field!(self.0.as_ptr(), r#type)
    }
    #[inline]
    pub(crate) fn vehicle_type_raw(self) -> u8 {
        self.vehicle_type().0
    }
    pub(crate) fn subtype(self) -> u8 {
        read_field!(self.0.as_ptr(), subtype)
    }
    pub(crate) fn set_subtype(self, value: u8) {
        write_field!(self.0.as_ptr(), subtype, value);
    }
    #[inline]
    pub(crate) fn owner(self) -> Owner {
        read_field!(self.0.as_ptr(), owner)
    }
    #[inline]
    pub(crate) fn owner_raw(self) -> u8 {
        self.owner().0
    }
    #[inline]
    pub(crate) fn set_owner(self, value: Owner) {
        write_field!(self.0.as_ptr(), owner, value);
    }
    #[inline]
    pub(crate) fn set_owner_raw(self, value: u8) {
        self.set_owner(CompanyID(value));
    }
    #[inline]
    pub(crate) fn direction(self) -> Direction {
        read_field!(self.0.as_ptr(), direction)
    }
    #[inline]
    pub(crate) fn direction_raw(self) -> u8 {
        self.direction().0
    }
    #[inline]
    pub(crate) fn set_direction(self, value: Direction) {
        write_field!(self.0.as_ptr(), direction, value);
    }
    #[inline]
    pub(crate) fn set_direction_raw(self, value: u8) {
        self.set_direction(Direction(value));
    }
    #[inline]
    pub(crate) fn vehstatus(self) -> VehStates {
        read_field!(self.0.as_ptr(), vehstatus)
    }
    #[inline]
    pub(crate) fn vehstatus_raw(self) -> u8 {
        self.vehstatus().0
    }
    #[inline]
    pub(crate) fn set_vehstatus(self, value: VehStates) {
        write_field!(self.0.as_ptr(), vehstatus, value);
    }
    #[inline]
    pub(crate) fn set_vehstatus_raw(self, value: u8) {
        self.set_vehstatus(VehStates(value));
    }
    pub(crate) fn subspeed(self) -> u8 {
        read_field!(self.0.as_ptr(), subspeed)
    }
    pub(crate) fn set_subspeed(self, value: u8) {
        write_field!(self.0.as_ptr(), subspeed, value);
    }
    pub(crate) fn acceleration(self) -> u8 {
        read_field!(self.0.as_ptr(), acceleration)
    }
    pub(crate) fn progress(self) -> u8 {
        read_field!(self.0.as_ptr(), progress)
    }
    pub(crate) fn set_progress(self, value: u8) {
        write_field!(self.0.as_ptr(), progress, value);
    }
    pub(crate) fn tick_counter(self) -> u8 {
        read_field!(self.0.as_ptr(), tick_counter)
    }
    pub(crate) fn set_tick_counter(self, value: u8) {
        write_field!(self.0.as_ptr(), tick_counter, value);
    }
    pub(crate) fn running_ticks(self) -> u8 {
        read_field!(self.0.as_ptr(), running_ticks)
    }
    pub(crate) fn set_running_ticks(self, value: u8) {
        write_field!(self.0.as_ptr(), running_ticks, value);
    }
    pub(crate) fn day_counter(self) -> u8 {
        read_field!(self.0.as_ptr(), day_counter)
    }
    pub(crate) fn set_day_counter(self, value: u8) {
        write_field!(self.0.as_ptr(), day_counter, value);
    }
    pub(crate) fn breakdown_ctr(self) -> u8 {
        read_field!(self.0.as_ptr(), breakdown_ctr)
    }
    pub(crate) fn set_breakdown_ctr(self, value: u8) {
        write_field!(self.0.as_ptr(), breakdown_ctr, value);
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;
    use crate::vehicle::VehicleRef;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Failed native assertions seen by `record_failure` in this test process.
    static FAILED: AtomicU32 = AtomicU32::new(0);

    extern "C" fn record_failure(condition: bool) {
        if !condition {
            FAILED.fetch_add(1, Ordering::SeqCst);
        }
    }

    extern "C" fn exit_on_any_call(_: bool) {
        std::process::exit(87);
    }

    /// A private train chain of `subtypes`, linked in order.
    fn chain(subtypes: &[u8]) -> Vec<VehicleRef> {
        let records: Vec<_> = subtypes
            .iter()
            .map(|&subtype| {
                let record = VehicleRecord::new(std::ptr::null_mut(), VehicleType::TRAIN);
                // SAFETY: Each record is privately owned through its test.
                unsafe { (*record).subtype = subtype };
                VehicleRef::of(record)
            })
            .collect();
        for pair in records.windows(2) {
            // SAFETY: Both records remain live through the test.
            unsafe { set_next(pair[0].0.as_ptr(), pair[1].0.as_ptr()) };
        }
        records
    }

    /// `GroundVehicle::SetArticulatedPart` of an empty subtype.
    fn articulated_part() -> u8 {
        let mut subtype = 0;
        crate::bitmath_func::set_bit8(
            &mut subtype,
            GroundVehicleSubtypeFlags::GVSF_ARTICULATED_PART.0,
        );
        subtype
    }

    fn release(records: Vec<VehicleRef>) {
        for record in records {
            // SAFETY: No traversal follows these private records after cleanup.
            unsafe { VehicleRecord::delete(record.0.as_ptr()) };
        }
    }

    /// `GetNextArticulatedPart` asserts `HasArticulatedPart()`. Under this test's
    /// recording policy, which returns from a failed assertion, it then returns
    /// `Next()`.
    #[test]
    fn next_articulated_part_asserts_that_one_follows() {
        if crate::services_assert::isolate_test_check(
            "vehicle_base::record_tests::next_articulated_part_asserts_that_one_follows",
            record_failure,
        ) {
            return;
        }
        let articulated = articulated_part();
        let records = chain(&[0, articulated, 0]);
        let [engine, part, wagon] = records[..] else {
            unreachable!()
        };
        assert_eq!(engine.get_next_articulated_part(), Some(part));
        assert_eq!(FAILED.load(Ordering::SeqCst), 0);
        assert_eq!(part.get_next_articulated_part(), Some(wagon));
        assert_eq!(FAILED.load(Ordering::SeqCst), 1);
        assert_eq!(wagon.get_next_articulated_part(), None);
        assert_eq!(FAILED.load(Ordering::SeqCst), 2);
        release(records);
    }

    /// `SetNext` asserts `this != next` before it links anything: linking a
    /// vehicle to itself reaches the native assertion, which here ends the process.
    #[test]
    fn set_next_asserts_against_a_self_link() {
        if crate::services_assert::isolate_test_exit(
            "vehicle_base::record_tests::set_next_asserts_against_a_self_link",
        ) {
            return;
        }
        let records = chain(&[0]);
        // SAFETY: The record stays live; the failed assertion ends the process.
        unsafe { set_next(records[0].0.as_ptr(), records[0].0.as_ptr()) };
        unreachable!("set_next returned past its failed self-link assertion");
    }

    /// Linking two distinct vehicles makes no assertion call.
    #[test]
    fn set_next_of_distinct_vehicles_makes_no_assertion_call() {
        if crate::services_assert::isolate_test_check(
            "vehicle_base::record_tests::set_next_of_distinct_vehicles_makes_no_assertion_call",
            exit_on_any_call,
        ) {
            return;
        }
        let records = chain(&[0, 0]);
        assert_eq!(records[0].next(), Some(records[1]));
        release(records);
    }

    #[test]
    fn train_units_skip_articulated_parts_and_one_rear_head() {
        use crate::vehicle::VehicleRef;
        let articulated = articulated_part();
        let rear = crate::ground_vehicle::set_multiheaded(0);
        let engine = crate::ground_vehicle::set_engine(0);
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
            let record = VehicleRecord::new(std::ptr::null_mut(), VehicleType::TRAIN);
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
            unsafe { VehicleRecord::delete(record.0.as_ptr()) };
        }
    }

    #[test]
    fn set_next_detaches_old_suffix_and_new_predecessor() {
        let records: Vec<_> = (0..5)
            .map(|_| VehicleRecord::new(std::ptr::null_mut(), VehicleType::TRAIN))
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
                VehicleRecord::delete(record);
            }
        }
    }

    /// `CountVehiclesInChain` counts the given vehicle and each `Next()` after it.
    #[test]
    fn count_vehicles_in_chain_counts_from_the_given_vehicle_to_the_end() {
        let records: Vec<_> = (0..4)
            .map(|_| openttd_rust_vehicle_record_new(std::ptr::null_mut(), VehicleType::VEH_TRAIN))
            .collect();
        for pair in records.windows(2) {
            // SAFETY: The test's fresh records.
            unsafe { (*pair[0]).next = pair[1] };
        }
        for (&record, remaining) in records.iter().zip((1..=4u32).rev()) {
            assert_eq!(
                crate::vehicle_func::count_vehicles_in_chain(VehicleRef::of(record)),
                remaining
            );
        }
        for record in records {
            // SAFETY: Each record came from `openttd_rust_vehicle_record_new`.
            unsafe { openttd_rust_vehicle_record_delete(record) };
        }
    }
}

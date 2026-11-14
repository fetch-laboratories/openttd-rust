/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `GroundVehicle<T, Type>` (`ground_vehicle.cpp`, `ground_vehicle.hpp`): base class and
//! functions for all vehicles that move through ground, with the per-type methods it calls on
//! `Train` (`train.h`, `train_cmd.cpp`) and `RoadVehicle` (`roadveh.h`, `roadveh_cmd.cpp`).
//!
//! The template is [`GroundVehicle<T>`], instantiated for [`Train`] and [`RoadVehicle`]; each
//! entry resolves its [`Owner`] once and selects the instantiation by its vehicle type.
//!
//! Every entry runs under the game-state mutex with live vehicle records and immutable service
//! tables. Field accesses are field-sized raw loads and stores that end before the next service
//! call; services are `noexcept` and form no Rust borrow of a record.
#![allow(unsafe_code)]
use crate::bitmath_func::{clr_bit8, clr_bit16, has_bit, set_bit8, set_bit16};
use crate::cargo_type::CargoType;
use crate::direction_func::{diag_dir_to_axis, dir_to_diag_dir};
use crate::direction_type::{Axis, DiagDirection, Direction};
use crate::engine_type::{EngineID, VehicleAccelerationModel};
use crate::newgrf_properties::{
    PROP_ROADVEH_POWER, PROP_ROADVEH_TRACTIVE_EFFORT, PROP_ROADVEH_WEIGHT, PROP_TRAIN_POWER,
    PROP_TRAIN_TRACTIVE_EFFORT, PROP_TRAIN_WEIGHT,
};
use crate::raw_field::{read_field, write_field};
use crate::services::{RailType, RoadType};
use crate::tile_type::{TILE_SIZE, TILE_UNIT_MASK, TileIndex};
use crate::track_func::track_status_to_track_bits;
use crate::track_type::{TRACK_BIT_DEPOT, TRACK_BIT_WORMHOLE, TRACK_BIT_X, TRACK_BIT_Y};
use crate::transport_type::{TRANSPORT_RAIL, TRANSPORT_ROAD};
use crate::vehicle::{VehicleRef, VehicleServices, VehicleType};
use crate::vehicle_base::{GroundVehicleSubtypeFlags, VehState, VehStates};
use crate::vehicle_type::GROUND_ACCELERATION;
use crate::witness::{self, hit};
use std::marker::PhantomData;

/// `SetFrontEngine`: set front engine state.
pub(crate) fn set_front_engine(mut subtype: u8) -> u8 {
    set_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_FRONT.0)
}
/// `ClearFrontEngine`: remove the front engine state.
pub(crate) fn clear_front_engine(mut subtype: u8) -> u8 {
    clr_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_FRONT.0)
}
/// `SetEngine`: set engine status.
pub(crate) fn set_engine(mut subtype: u8) -> u8 {
    set_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_ENGINE.0)
}
/// `SetFreeWagon`: set a vehicle as a free wagon.
pub(crate) fn set_free_wagon(mut subtype: u8) -> u8 {
    set_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_FREE_WAGON.0)
}
/// `ClearFreeWagon`: clear a vehicle from being a free wagon.
pub(crate) fn clear_free_wagon(mut subtype: u8) -> u8 {
    clr_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_FREE_WAGON.0)
}
/// `SetMultiheaded`: set a vehicle as a multiheaded engine.
pub(crate) fn set_multiheaded(mut subtype: u8) -> u8 {
    set_bit8(&mut subtype, GroundVehicleSubtypeFlags::GVSF_MULTIHEADED.0)
}
/// `IsFreeWagon`: check if the vehicle is a free wagon (got no engine in front of it).
pub(crate) fn is_free_wagon(subtype: u8) -> bool {
    has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_FREE_WAGON.0)
}
/// `IsEngine`: check if a vehicle is an engine (can be first in a consist).
pub(crate) fn is_engine(subtype: u8) -> bool {
    has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_ENGINE.0)
}
/// `IsWagon`: check if a vehicle is a wagon.
pub(crate) fn is_wagon(subtype: u8) -> bool {
    has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_WAGON.0)
}
/// `IsMultiheaded`: check if the vehicle is a multiheaded engine.
pub(crate) fn is_multiheaded(subtype: u8) -> bool {
    has_bit(subtype, GroundVehicleSubtypeFlags::GVSF_MULTIHEADED.0)
}
/// `IsRearDualheaded`: tell if we are dealing with the rear end of a multiheaded engine.
pub(crate) fn is_rear_dualheaded(subtype: u8) -> bool {
    is_multiheaded(subtype) && !is_engine(subtype)
}

crate::abi::shared_records! {
    fn abi_record;
    /// `GroundVehicleCache`: cached, frequently calculated values. All of these values except
    /// `cached_slope_resistance` are set only for the first part of a vehicle. The native shell
    /// aliases this allocation; `first_engine` crosses as the base integer of its `EngineID`.
    #[derive(Default, PartialEq, Eq)]
    "GroundVehicleCache" ["ground"] => pub struct GroundCache {
        /// Total weight of the consist (valid only for the first engine).
        pub cached_weight: u32,
        /// Resistance caused by weight when this vehicle part is at a slope.
        pub cached_slope_resistance: u32,
        /// Maximum tractive effort of consist (valid only for the first engine).
        pub cached_max_te: u32,
        /// Resistance caused by the axles of the vehicle (valid only for the first engine).
        pub cached_axle_resistance: u16,
        /// Maximum consist speed (in internal units) limited by track type (valid only for
        /// the first engine).
        pub cached_max_track_speed: u16,
        /// Total power of the consist (valid only for the first engine).
        pub cached_power: u32,
        /// Air drag coefficient of the vehicle (valid only for the first engine).
        pub cached_air_drag: u32,
        /// Length of the whole vehicle (valid only for the first engine).
        pub cached_total_length: u16,
        /// Cached `EngineID` of the front vehicle. `EngineID::Invalid()` for the front vehicle
        /// itself.
        pub first_engine: u16,
        /// Length of this vehicle in units of 1/`VEHICLE_LENGTH` of normal length. It is cached
        /// because this can be set by a callback.
        pub cached_veh_length: u8,
        /// The last speed we did display, so we only have to redraw when this changes.
        pub last_speed: u16,
    }
    /// One stable owner per native train or road vehicle shell: `gcache` and `gv_flags`.
    #[derive(Default)]
    "OpenTTDGroundState" ["ground"] => pub struct GroundState {
        pub cache: GroundCache,
        /// @see `GroundVehicleFlags`.
        pub flags: u16,
    }
}

/// `GroundVehicleCache::operator<=>` equality, without observing padding bytes.
/// # Safety
/// Both pointers identify live caches for this synchronous comparison.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_cache_equal(
    a: *const GroundCache,
    b: *const GroundCache,
) -> u8 {
    // SAFETY: the caller supplies live caches; this call invokes no callbacks.
    u8::from(unsafe { *a == *b })
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ground_new() -> *mut GroundState {
    Box::into_raw(Box::new(GroundState {
        cache: GroundCache {
            first_engine: EngineID::INVALID.0,
            ..GroundCache::default()
        },
        flags: 0,
    }))
}
/// # Safety
/// The shell transfers its sole live owner after its final native access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_destroy(state: *mut GroundState) {
    // SAFETY: exactly one matching allocation, no aliases remain in use.
    drop(unsafe { Box::from_raw(state) });
}

/// `GroundVehicleFlags`: ground vehicle flags, as bit positions in `gv_flags`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundVehicleFlags(pub u8);
impl GroundVehicleFlags {
    /// Vehicle is currently going uphill. (Cached track information for acceleration)
    pub const GVF_GOINGUP_BIT: Self = Self(0);
    /// Vehicle is currently going downhill. (Cached track information for acceleration)
    pub const GVF_GOINGDOWN_BIT: Self = Self(1);
    /// Disable insertion and removal of automatic orders until the vehicle completes the real
    /// order.
    pub const GVF_SUPPRESS_IMPLICIT_ORDERS: Self = Self(2);
}
/// Base value of `GVF_GOINGUP_BIT`, for callers in files no pass has finished.
pub const GVF_GOINGUP_BIT: u8 = GroundVehicleFlags::GVF_GOINGUP_BIT.0;
/// Base value of `GVF_GOINGDOWN_BIT`, for callers in files no pass has finished.
pub const GVF_GOINGDOWN_BIT: u8 = GroundVehicleFlags::GVF_GOINGDOWN_BIT.0;
/// Base value of `GVF_SUPPRESS_IMPLICIT_ORDERS`, for callers in files no pass has finished.
pub const GVF_SUPPRESS_IMPLICIT_ORDERS: u8 = GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0;

/// `AccelStatus`: what is the status of our acceleration?
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccelStatus(pub u8);
impl AccelStatus {
    /// We want to go faster, if possible of course.
    pub const AS_ACCEL: Self = Self(0);
    /// We want to stop.
    pub const AS_BRAKE: Self = Self(1);
}

/// The Rust owners of one train or road vehicle part, read from its shared record.
#[derive(Clone, Copy)]
pub struct Owner {
    pub state: *mut GroundState,
    pub train: crate::train::Handle,
    pub road: *mut crate::road::State,
    pub kind: VehicleType,
}
impl Owner {
    /// The owners of a live train or road vehicle record: its `gcache`/`gv_flags` owner, and
    /// its train or road vehicle owner by type.
    #[inline]
    pub(crate) fn of(v: VehicleRef) -> Self {
        let kind = VehicleType(v.vehicle_type());
        let record = v.0.as_ptr();
        if kind == VehicleType::VEH_TRAIN {
            Self {
                state: v.ground(),
                train: crate::train::Handle::from_record(record),
                road: std::ptr::null_mut(),
                kind,
            }
        } else {
            Self {
                state: v.ground(),
                train: crate::train::Handle::NONE,
                // `OpenTTDRoadState` is the first field of the `#[repr(C)]` road owner.
                road: read_field!(record, road).cast(),
                kind,
            }
        }
    }
    /// The road shell owns separate stable road and ground allocations.
    pub(crate) fn for_road(state: *mut GroundState, road: *mut crate::road::State) -> Self {
        Self {
            state,
            train: crate::train::Handle::NONE,
            road,
            kind: VehicleType::VEH_ROAD,
        }
    }
    fn flags(self) -> u16 {
        read_field!(self.state, flags)
    }
    fn set_flags(self, flags: u16) {
        write_field!(self.state, flags, flags);
    }
    fn has_flag(self, flag: GroundVehicleFlags) -> bool {
        has_bit(self.flags(), flag.0)
    }
}

/// Immutable process-wide typed services for records still stored in C++.
#[repr(C)]
pub struct Services {
    pub vehicle: &'static VehicleServices,
    pub road: &'static crate::road::Leaves,
    pub assertions: bool,
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    /// `CargoSpec::Get(cargo)->WeightOfNUnits(n)`.
    pub cargo_weight: extern "C" fn(u8, u32) -> u64,
    /// `CargoSpec::Get(cargo)->WeightOfNUnitsInTrain(n)`.
    pub train_cargo_weight: extern "C" fn(u8, u32) -> u64,
    pub powered: extern "C" fn(u64, RailType) -> bool,
    pub rail_type: extern "C" fn(u32) -> RailType,
    pub rail_acceleration: extern "C" fn(RailType) -> u8,
    pub rail_speed: extern "C" fn(RailType) -> u16,
    pub steepness: extern "C" fn(VehicleType) -> u32,
    pub viewport: extern "C" fn(VehicleRef, bool, bool),
    pub base_crash: extern "C" fn(VehicleRef, bool) -> u32,
}
impl Services {
    fn shared(&self) -> &'static crate::services::Services {
        self.capabilities.shared
    }
    /// `assert(condition)`: evaluated only where the native build asserts.
    fn assert(&self, condition: impl FnOnce() -> bool) {
        if self.assertions {
            crate::movement::native_assert(self.shared(), condition());
        }
    }
    fn newgrf(&self) -> &crate::newgrf_vehicle::Services {
        // SAFETY: the vehicle table names the immutable process-lifetime NewGRF table.
        unsafe { &*(self.vehicle.newgrf)() }
    }
    /// `GetVehicleProperty(v, property, orig_value)`.
    fn property(&self, v: VehicleRef, property: u8, orig_value: u16) -> i32 {
        (self.vehicle.property)((self.vehicle.newgrf)(), v, property, i32::from(orig_value))
    }
    /// `GetRailType(tile)`.
    fn get_rail_type(&self, tile: TileIndex) -> RailType {
        RailType(crate::map_func::World::new(self.shared()).rail_type(tile.0))
    }
    /// `v->cargo_type`.
    fn cargo_type(&self, v: VehicleRef) -> CargoType {
        // SAFETY: a live record and the immutable binding; one field read.
        CargoType(unsafe { (self.newgrf().cargo_type)(v.0.as_ptr()) })
    }
    /// `v->cargo.StoredCount()`.
    fn stored_count(&self, v: VehicleRef) -> u32 {
        // SAFETY: the record's live canonical cargo list; no borrow survives this read.
        unsafe {
            crate::cargo_storage::openttd_rust_cargo_list_stored((self.newgrf().cargo_owner)(
                v.0.as_ptr(),
            ))
        }
    }
    /// `v->GetEngine()->DetermineCapacity(v)`.
    fn determine_capacity(&self, v: VehicleRef) -> u32 {
        // SAFETY: live engine and record; the capacity entry borrows neither across a call.
        unsafe {
            crate::engine_capabilities::openttd_rust_engine_capacity(
                self.capabilities,
                (self.vehicle.engine)(v).0.as_ptr(),
                v.0.as_ptr(),
                std::ptr::null_mut(),
            )
        }
    }
    /// `RailVehInfo(this->gcache.first_engine)` or `RoadVehInfo(...)`.
    fn first_engine(&self, o: Owner) -> crate::vehicle::EngineRef {
        crate::vehicle::EngineRef::of((self.capabilities.engine)(read_field!(
            o.state,
            cache.first_engine
        )))
    }
}

/// The methods `GroundVehicle<T, Type>` calls on `T`. Child classes define all of them; the
/// template calls them statically to improve performance.
pub(crate) trait GroundVehicleType {
    /// `Type`, which equals the `TransportType` of its depots.
    const TRANSPORT: u8;
    fn get_power(s: &Services, v: VehicleRef, o: Owner) -> u16;
    fn get_powered_part_power(s: &Services, v: VehicleRef, o: Owner, head: Owner) -> u16;
    fn get_weight(s: &Services, v: VehicleRef, o: Owner) -> u16;
    fn get_max_weight(s: &Services, v: VehicleRef, o: Owner) -> u16;
    fn get_tractive_effort(s: &Services, v: VehicleRef) -> u8;
    fn get_air_drag(s: &Services, v: VehicleRef) -> u8;
    fn get_air_drag_area(v: VehicleRef, o: Owner) -> u8;
    fn get_acceleration_status(v: VehicleRef, o: Owner) -> AccelStatus;
    fn get_current_speed(v: VehicleRef) -> u16;
    fn get_rolling_friction(s: &Services, v: VehicleRef) -> u32;
    fn get_acceleration_type(s: &Services, v: VehicleRef) -> VehicleAccelerationModel;
    fn get_slope_steepness(s: &Services) -> u32;
    fn get_display_max_speed(v: VehicleRef) -> i32;
    fn get_max_track_speed(s: &Services, v: VehicleRef) -> u16;
    fn tile_may_have_sloped_track(
        movement: &crate::movement::CachedTable<'_>,
        v: VehicleRef,
        o: Owner,
    ) -> bool;
    fn has_to_use_get_slope_pixel_z(v: VehicleRef) -> bool;
    fn is_in_depot(o: Owner) -> bool;
}

/// `Train`'s acceleration methods (`train.h`, `train_cmd.cpp`).
pub(crate) struct Train;
impl GroundVehicleType for Train {
    const TRANSPORT: u8 = TRANSPORT_RAIL;
    /// Allows to know the power value that this vehicle will use. Returns the power value from
    /// the engine in HP, or zero if the vehicle is not powered.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`uint16_t power = GetVehicleProperty(...)`"
    )]
    fn get_power(s: &Services, v: VehicleRef, o: Owner) -> u16 {
        /* Power is not added for articulated parts */
        if !v.is_articulated_part()
            && (s.powered)(
                o.train.get_railtypes(),
                s.get_rail_type(TileIndex(v.tile())),
            )
        {
            let engine = (s.vehicle.engine)(v);
            let mut power = s.property(
                v,
                PROP_TRAIN_POWER,
                engine.ground_power(VehicleType::VEH_TRAIN),
            ) as u16;
            /* Halve power for multiheaded parts */
            if is_multiheaded(v.subtype()) {
                power /= 2;
            }
            return power;
        }
        0
    }
    /// Returns a value if this articulated part is powered: the power value from the
    /// articulated part in HP, or zero if it is not powered.
    #[inline]
    fn get_powered_part_power(s: &Services, v: VehicleRef, o: Owner, head: Owner) -> u16 {
        /* For powered wagons the engine defines the type of engine (i.e. railtype) */
        if o.train.get_flags() & crate::train::FLAG_POWERED_WAGON != 0
            && (s.powered)(
                head.train.get_railtypes(),
                s.get_rail_type(TileIndex(v.tile())),
            )
        {
            return s.first_engine(o).wagon_power();
        }
        0
    }
    /// Allows to know the weight value that this vehicle will use, in tonnes.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the uint16_t weight: `WeightOfNUnitsInTrain(...)` and `weight += GetVehicleProperty(...)`"
    )]
    fn get_weight(s: &Services, v: VehicleRef, o: Owner) -> u16 {
        let mut weight = (s.train_cargo_weight)(s.cargo_type(v).0, s.stored_count(v)) as u16;
        /* Vehicle weight is not added for articulated parts. */
        if !v.is_articulated_part() {
            let engine = (s.vehicle.engine)(v);
            weight = weight.wrapping_add(s.property(
                v,
                PROP_TRAIN_WEIGHT,
                engine.ground_weight(VehicleType::VEH_TRAIN),
            ) as u16);
        }
        /* Powered wagons have extra weight added. */
        if o.train.get_flags() & crate::train::FLAG_POWERED_WAGON != 0 {
            weight = weight.wrapping_add(u16::from(s.first_engine(o).wagon_weight()));
        }
        weight
    }
    /// `Train::GetMaxWeight`: calculates the weight value that this vehicle will have when
    /// fully loaded with its current cargo, in tonnes.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the uint16_t weight: `WeightOfNUnitsInTrain(...)` and `weight += GetVehicleProperty(...)`"
    )]
    fn get_max_weight(s: &Services, v: VehicleRef, o: Owner) -> u16 {
        hit(witness::GROUND_FULL_WEIGHT);
        let mut weight = (s.train_cargo_weight)(s.cargo_type(v).0, s.determine_capacity(v)) as u16;
        /* Vehicle weight is not added for articulated parts. */
        if !v.is_articulated_part() {
            let engine = (s.vehicle.engine)(v);
            weight = weight.wrapping_add(s.property(
                v,
                PROP_TRAIN_WEIGHT,
                engine.ground_weight(VehicleType::VEH_TRAIN),
            ) as u16);
        }
        /* Powered wagons have extra weight added. */
        if o.train.get_flags() & crate::train::FLAG_POWERED_WAGON != 0 {
            weight = weight.wrapping_add(u16::from(s.first_engine(o).wagon_weight()));
        }
        weight
    }
    /// Allows to know the tractive effort value that this vehicle will use.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GetTractiveEffort's uint8_t return of GetVehicleProperty"
    )]
    fn get_tractive_effort(s: &Services, v: VehicleRef) -> u8 {
        let engine = (s.vehicle.engine)(v);
        s.property(
            v,
            PROP_TRAIN_TRACTIVE_EFFORT,
            u16::from(engine.ground_te(VehicleType::VEH_TRAIN)),
        ) as u8
    }
    /// Gets the air drag coefficient of this vehicle.
    #[inline]
    fn get_air_drag(s: &Services, v: VehicleRef) -> u8 {
        (s.vehicle.engine)(v).ground_drag(VehicleType::VEH_TRAIN)
    }
    /// Gets the area used for calculating air drag, in m^2.
    #[inline]
    fn get_air_drag_area(v: VehicleRef, o: Owner) -> u8 {
        /* Air drag is higher in tunnels due to the limited cross-section. */
        if o.train.track() == TRACK_BIT_WORMHOLE && VehStates(v.status()).test(VehState::Hidden) {
            28
        } else {
            14
        }
    }
    /// Checks the current acceleration status of this vehicle.
    #[inline]
    fn get_acceleration_status(v: VehicleRef, o: Owner) -> AccelStatus {
        if VehStates(v.status()).test(VehState::Stopped)
            || o.train.get_flags() & (crate::train::FLAG_REVERSING | crate::train::FLAG_STUCK) != 0
        {
            AccelStatus::AS_BRAKE
        } else {
            AccelStatus::AS_ACCEL
        }
    }
    /// Calculates the current speed of this vehicle, in km/h-ish.
    #[inline]
    fn get_current_speed(v: VehicleRef) -> u16 {
        v.speed()
    }
    /// Returns the rolling friction coefficient of this vehicle, in [1e-4].
    #[inline]
    fn get_rolling_friction(_: &Services, v: VehicleRef) -> u32 {
        /* Rolling friction for steel on steel is between 0.1% and 0.2%.
         * The friction coefficient increases with speed in a way that
         * it doubles at 512 km/h, triples at 1024 km/h and so on. */
        15 * (512 + u32::from(Self::get_current_speed(v))) / 512
    }
    /// Allows to know the acceleration type of a vehicle.
    #[inline]
    fn get_acceleration_type(s: &Services, v: VehicleRef) -> VehicleAccelerationModel {
        VehicleAccelerationModel((s.rail_acceleration)(s.get_rail_type(TileIndex(v.tile()))))
    }
    /// Returns the slope steepness used by this vehicle.
    #[inline]
    fn get_slope_steepness(s: &Services) -> u32 {
        (s.steepness)(VehicleType::VEH_TRAIN)
    }
    /// `Train::GetDisplayMaxSpeed`.
    #[inline]
    fn get_display_max_speed(v: VehicleRef) -> i32 {
        i32::from(v.max_speed())
    }
    /// Gets the maximum speed allowed by the track for this vehicle.
    #[inline]
    fn get_max_track_speed(s: &Services, v: VehicleRef) -> u16 {
        (s.rail_speed)(s.get_rail_type(TileIndex(v.tile())))
    }
    /// Checks if the vehicle is at a tile that can be sloped.
    #[inline]
    fn tile_may_have_sloped_track(
        _: &crate::movement::CachedTable<'_>,
        _: VehicleRef,
        o: Owner,
    ) -> bool {
        /* Any track that isn't TRACK_BIT_X or TRACK_BIT_Y cannot be sloped. */
        let track = o.train.track();
        track == TRACK_BIT_X || track == TRACK_BIT_Y
    }
    /// Trains can always use the faster algorithm because they have always the same direction
    /// as the track under them.
    #[inline]
    fn has_to_use_get_slope_pixel_z(_: VehicleRef) -> bool {
        false
    }
    #[inline]
    fn is_in_depot(o: Owner) -> bool {
        o.train.track() == TRACK_BIT_DEPOT
    }
}

/// `RoadVehicle`'s acceleration methods (`roadveh.h`, `roadveh_cmd.cpp`).
pub(crate) struct RoadVehicle;
impl RoadVehicle {
    /// `this->roadtype`.
    fn roadtype(s: &Services, v: VehicleRef) -> RoadType {
        // SAFETY: a live record and the immutable binding; one field read.
        unsafe { (s.newgrf().roadtype)(v.0.as_ptr()) }
    }
}
impl GroundVehicleType for RoadVehicle {
    const TRANSPORT: u8 = TRANSPORT_ROAD;
    /// Allows to know the power value that this vehicle will use. Returns the power value from
    /// the engine in HP, or zero if the vehicle is not powered.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GetPower's uint16_t return of `10 * GetVehicleProperty(...)`"
    )]
    fn get_power(s: &Services, v: VehicleRef, _: Owner) -> u16 {
        /* Power is not added for articulated parts */
        if !v.is_articulated_part() {
            let engine = (s.vehicle.engine)(v);
            /* Road vehicle power is in units of 10 HP. */
            return s
                .property(
                    v,
                    PROP_ROADVEH_POWER,
                    engine.ground_power(VehicleType::VEH_ROAD),
                )
                .wrapping_mul(10) as u16;
        }
        0
    }
    /// Returns zero, because road vehicles don't have powered parts.
    #[inline]
    fn get_powered_part_power(_: &Services, _: VehicleRef, _: Owner, _: Owner) -> u16 {
        0
    }
    /// Allows to know the weight value that this vehicle will use, in tonnes.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the uint16_t weight: `WeightOfNUnits(...)` and `weight += GetVehicleProperty(...) / 4`"
    )]
    fn get_weight(s: &Services, v: VehicleRef, _: Owner) -> u16 {
        let mut weight = (s.cargo_weight)(s.cargo_type(v).0, s.stored_count(v)) as u16;
        /* Vehicle weight is not added for articulated parts. */
        if !v.is_articulated_part() {
            let engine = (s.vehicle.engine)(v);
            /* Road vehicle weight is in units of 1/4 t. */
            weight = weight.wrapping_add(
                (s.property(
                    v,
                    PROP_ROADVEH_WEIGHT,
                    engine.ground_weight(VehicleType::VEH_ROAD),
                ) / 4) as u16,
            );
        }
        weight
    }
    /// `RoadVehicle::GetMaxWeight`: calculates the weight value that this vehicle will have
    /// when fully loaded with its current cargo, in tonnes.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the uint16_t weight: `WeightOfNUnits(...)` and `weight += GetVehicleProperty(...) / 4`"
    )]
    fn get_max_weight(s: &Services, v: VehicleRef, _: Owner) -> u16 {
        hit(witness::GROUND_FULL_WEIGHT);
        let mut weight = (s.cargo_weight)(s.cargo_type(v).0, s.determine_capacity(v)) as u16;
        /* Vehicle weight is not added for articulated parts. */
        if !v.is_articulated_part() {
            let engine = (s.vehicle.engine)(v);
            /* Road vehicle weight is in units of 1/4 t. */
            weight = weight.wrapping_add(
                (s.property(
                    v,
                    PROP_ROADVEH_WEIGHT,
                    engine.ground_weight(VehicleType::VEH_ROAD),
                ) / 4) as u16,
            );
        }
        weight
    }
    /// Allows to know the tractive effort value that this vehicle will use.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GetTractiveEffort's uint8_t return of GetVehicleProperty"
    )]
    fn get_tractive_effort(s: &Services, v: VehicleRef) -> u8 {
        let engine = (s.vehicle.engine)(v);
        /* The tractive effort coefficient is in units of 1/256.  */
        s.property(
            v,
            PROP_ROADVEH_TRACTIVE_EFFORT,
            u16::from(engine.ground_te(VehicleType::VEH_ROAD)),
        ) as u8
    }
    /// Gets the air drag coefficient of this vehicle.
    #[inline]
    fn get_air_drag(s: &Services, v: VehicleRef) -> u8 {
        (s.vehicle.engine)(v).ground_drag(VehicleType::VEH_ROAD)
    }
    /// Gets the area used for calculating air drag, in m^2.
    #[inline]
    fn get_air_drag_area(_: VehicleRef, _: Owner) -> u8 {
        6
    }
    /// Checks the current acceleration status of this vehicle.
    #[inline]
    fn get_acceleration_status(v: VehicleRef, _: Owner) -> AccelStatus {
        if VehStates(v.status()).test(VehState::Stopped) {
            AccelStatus::AS_BRAKE
        } else {
            AccelStatus::AS_ACCEL
        }
    }
    /// Calculates the current speed of this vehicle, in km/h-ish.
    #[inline]
    fn get_current_speed(v: VehicleRef) -> u16 {
        v.speed() / 2
    }
    /// Returns the rolling friction coefficient of this vehicle, in [1e-4].
    #[inline]
    fn get_rolling_friction(s: &Services, v: VehicleRef) -> u32 {
        /* Trams have a slightly greater friction coefficient than trains.
         * The rest of road vehicles have bigger values. */
        let coeff = if crate::road::availability::road_type_is_tram(Self::roadtype(s, v)) {
            40
        } else {
            75
        };
        /* The friction coefficient increases with speed in a way that
         * it doubles at 128 km/h, triples at 256 km/h and so on. */
        coeff * (128 + u32::from(Self::get_current_speed(v))) / 128
    }
    /// Road vehicles always use a normal acceleration method.
    #[inline]
    fn get_acceleration_type(_: &Services, _: VehicleRef) -> VehicleAccelerationModel {
        VehicleAccelerationModel::Normal
    }
    /// Returns the slope steepness used by this vehicle.
    #[inline]
    fn get_slope_steepness(s: &Services) -> u32 {
        (s.steepness)(VehicleType::VEH_ROAD)
    }
    /// `RoadVehicle::GetDisplayMaxSpeed`.
    #[inline]
    fn get_display_max_speed(v: VehicleRef) -> i32 {
        i32::from(v.max_speed()) / 2
    }
    /// Gets the maximum speed allowed by the road type of the tile for this vehicle.
    #[inline]
    fn get_max_track_speed(s: &Services, v: VehicleRef) -> u16 {
        let rtt = crate::road::availability::get_road_tram_type(Self::roadtype(s, v));
        let tile = v.tile();
        let world = crate::map_func::World::new(s.shared());
        let roadtype = world.asserted(|m| crate::road_map::get_road_type(m, tile, rtt));
        (crate::movement::table(s.shared()).road_type_speed)(roadtype)
    }
    /// Checks if the vehicle is at a tile that can be sloped.
    #[inline]
    fn tile_may_have_sloped_track(
        movement: &crate::movement::CachedTable<'_>,
        v: VehicleRef,
        _: Owner,
    ) -> bool {
        let ts = movement.get_road_vehicle_track_status(v, TileIndex(v.tile()));
        let trackbits = track_status_to_track_bits(ts);
        trackbits == TRACK_BIT_X || trackbits == TRACK_BIT_Y
    }
    /// Road vehicles have to use `GetSlopePixelZ()` to compute their height if they are
    /// reversing because in that case, their direction is not parallel with the road. It is
    /// safe to return true even if it is not reversing.
    #[inline]
    fn has_to_use_get_slope_pixel_z(v: VehicleRef) -> bool {
        let rv = v.first();
        let first = Owner::of(rv);
        // SAFETY: the shared slope predicate gets live owners and immutable services.
        unsafe { crate::road::slope_pixel(v, rv, first.road, v.direction()) }
    }
    #[inline]
    fn is_in_depot(o: Owner) -> bool {
        crate::road::is_in_depot(o.road)
    }
}

/// `GroundVehicle<T, Type>`, instantiated for [`Train`] and [`RoadVehicle`].
pub(crate) struct GroundVehicle<T>(PhantomData<T>);
impl<T: GroundVehicleType> GroundVehicle<T> {
    /// Recalculates the cached total power of a vehicle. Should be called when the consist is
    /// changed.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`uint16_t max_speed = v->GetDisplayMaxSpeed()` and the uint8_t air_drag"
    )]
    fn power_changed(s: &Services, v: VehicleRef, o: Owner) {
        s.assert(|| v.first() == v);
        hit(if T::TRANSPORT == TRANSPORT_RAIL {
            witness::GROUND_RAIL_POWER
        } else {
            witness::GROUND_ROAD_POWER
        });

        let mut total_power = 0_u32;
        let mut max_te = 0_u32;
        let mut number_of_parts = 0_u32;
        let mut max_track_speed = v.max_speed(); // Max track speed in internal units.

        let mut part = Some(v);
        while let Some(u) = part {
            let uo = if u == v { o } else { Owner::of(u) };
            let current_power = u32::from(T::get_power(s, u, uo))
                + u32::from(T::get_powered_part_power(s, u, uo, uo));
            total_power = total_power.wrapping_add(current_power);

            /* Only powered parts add tractive effort. */
            if current_power > 0 {
                max_te = max_te.wrapping_add(
                    u32::from(T::get_weight(s, u, uo))
                        .wrapping_mul(u32::from(T::get_tractive_effort(s, u))),
                );
            }
            number_of_parts = number_of_parts.wrapping_add(1);

            /* Get minimum max speed for this track. */
            let track_speed = T::get_max_track_speed(s, u);
            if track_speed > 0 {
                max_track_speed = max_track_speed.min(track_speed);
            }
            part = u.next();
        }

        let air_drag_value = T::get_air_drag(s, v);

        /* If air drag is set to zero (default), the resulting air drag coefficient is dependent
         * on max speed. */
        let air_drag = if air_drag_value == 0 {
            let max_speed = T::get_display_max_speed(v) as u16;
            /* Simplification of the method used in TTDPatch. It uses <= 10 to change more
             * steadily from 128 to 196. */
            if max_speed <= 10 {
                192
            } else {
                (2048 / max_speed).max(1) as u8
            }
        } else {
            /* According to the specs, a value of 0x01 in the air drag property means "no air
             * drag". */
            if air_drag_value == 1 {
                0
            } else {
                air_drag_value
            }
        };

        write_field!(
            o.state,
            cache.cached_air_drag,
            u32::from(air_drag).wrapping_add(
                3_u32
                    .wrapping_mul(u32::from(air_drag))
                    .wrapping_mul(number_of_parts)
                    / 20
            )
        );

        max_te = max_te.wrapping_mul(GROUND_ACCELERATION); // Tractive effort in (tonnes * 1000 * 9.8 =) N.
        max_te /= 256; // Tractive effort is a [0-255] coefficient.
        if read_field!(o.state, cache.cached_power) != total_power
            || read_field!(o.state, cache.cached_max_te) != max_te
        {
            /* Stop the vehicle if it has no power. */
            if total_power == 0 {
                v.write_status(
                    VehStates(v.status())
                        .with(VehStates::from_element(VehState::Stopped))
                        .0,
                );
            }

            write_field!(o.state, cache.cached_power, total_power);
            write_field!(o.state, cache.cached_max_te, max_te);
            // SAFETY: the vehicle table names the immutable process-lifetime maintenance table.
            (unsafe { &*s.vehicle.maintenance }.details_dirty)(v);
            (s.vehicle.start_stop_dirty)(v);
        }

        write_field!(o.state, cache.cached_max_track_speed, max_track_speed);
    }

    /// Recalculates the cached weight of a vehicle and its parts. Should be called each time
    /// the cargo on the consist changes.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the uint16_t `cached_axle_resistance = 10 * weight`"
    )]
    fn cargo_changed(s: &Services, v: VehicleRef, o: Owner) {
        s.assert(|| v.first() == v);
        hit(if T::TRANSPORT == TRANSPORT_RAIL {
            witness::GROUND_RAIL_WEIGHT
        } else {
            witness::GROUND_ROAD_WEIGHT
        });
        let mut weight = 0_u32;

        let mut part = Some(v);
        while let Some(u) = part {
            let uo = if u == v { o } else { Owner::of(u) };
            let current_weight = u32::from(T::get_weight(s, u, uo));
            weight = weight.wrapping_add(current_weight);
            /* Slope steepness is in percent, result in N. */
            write_field!(
                uo.state,
                cache.cached_slope_resistance,
                current_weight
                    .wrapping_mul(T::get_slope_steepness(s))
                    .wrapping_mul(100)
            );
            part = u.next();
        }

        /* Store consist weight in cache. */
        write_field!(o.state, cache.cached_weight, weight.max(1));
        /* Friction in bearings and other mechanical parts is 0.1% of the weight (result in N). */
        write_field!(
            o.state,
            cache.cached_axle_resistance,
            10_u32.wrapping_mul(weight) as u16
        );

        /* Now update vehicle power (tractive effort is dependent on weight). */
        Self::power_changed(s, v, o);
    }

    /// Calculates the acceleration of the vehicle under its current conditions.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "`const int max_te = this->gcache.cached_max_te` and `std::min<int>(max_te, power)`"
    )]
    #[allow(
        clippy::cast_possible_wrap,
        reason = "`const int max_te = this->gcache.cached_max_te`"
    )]
    fn get_acceleration(s: &Services, v: VehicleRef, o: Owner) -> i32 {
        /* Speed is used squared later on, so U16 * U16, and then multiplied by other values. */
        let speed = i64::from(T::get_current_speed(v)); // [km/h-ish]

        /* Weight is stored in tonnes. */
        let mass = i64::from(read_field!(o.state, cache.cached_weight));

        /* Power is stored in HP, we need it in watts.
         * Each vehicle can have U16 power, 128 vehicles, HP -> watt
         * and km/h to m/s conversion below result in a maximum of
         * about 1.1E11, way more than 4.3E9 of int32. */
        let power = i64::from(read_field!(o.state, cache.cached_power)) * 746;

        /* This is constructed from:
         *  - axle resistance:  U16 power * 10 for 128 vehicles.
         *     * 8.3E7
         *  - rolling friction: U16 power * 144 for 128 vehicles.
         *     * 1.2E9
         *  - slope resistance: U16 weight * 100 * 10 (steepness) for 128 vehicles.
         *     * 8.4E9
         *  - air drag: 28 * (U8 drag + 3 * U8 drag * 128 vehicles / 20) * U16 speed * U16 speed
         *     * 6.2E14 before dividing by 1000
         * Sum is 6.3E11, more than 4.3E9 of int32_t, so int64_t is needed.
         */
        let mut resistance = 0_i64;

        let maglev = T::get_acceleration_type(s, v) == VehicleAccelerationModel::Maglev;

        let area = i64::from(T::get_air_drag_area(v, o));
        if !maglev {
            /* Static resistance plus rolling friction. */
            resistance = i64::from(read_field!(o.state, cache.cached_axle_resistance));
            resistance += mass * i64::from(T::get_rolling_friction(s, v));
        }
        /* Air drag; the air drag coefficient is in an arbitrary NewGRF-unit,
         * so we need some magic conversion factor. */
        resistance +=
            area * i64::from(read_field!(o.state, cache.cached_air_drag)) * speed * speed / 1000;

        resistance += Self::get_slope_resistance(v, o);

        /* This value allows to know if the vehicle is accelerating or braking. */
        let mode = T::get_acceleration_status(v, o);
        hit(if mode == AccelStatus::AS_ACCEL {
            witness::GROUND_ACCELERATE
        } else {
            witness::GROUND_BRAKE
        });

        let max_te = read_field!(o.state, cache.cached_max_te) as i32; // [N]

        /* Constructed from power, with need to multiply by 18 and assuming
         * low speed, it needs to be a 64 bit integer too. */
        let mut force;
        if speed > 0 {
            if maglev {
                force = power / 25;
            } else {
                /* Conversion factor from km/h to m/s is 5/18 to get [N] in the end. */
                force = power * 18 / (speed * 5);
                if mode == AccelStatus::AS_ACCEL && force > i64::from(max_te) {
                    force = i64::from(max_te);
                }
            }
        } else {
            /* "Kickoff" acceleration. */
            force = if mode == AccelStatus::AS_ACCEL && !maglev {
                i64::from(max_te.min(power as i32))
            } else {
                power
            };
            force = force.max(mass * 8 + resistance);
        }

        if mode == AccelStatus::AS_ACCEL {
            /* Easy way out when there is no acceleration. */
            if force == resistance {
                return 0;
            }

            /* When we accelerate, make sure we always keep doing that, even when
             * the excess force is more than the mass. Otherwise a vehicle going
             * down hill will never slow down enough, and a vehicle that came up
             * a hill will never speed up enough to (eventually) get back to the
             * same (maximum) speed. */
            let accel = crate::math::clamp_to_i32((force - resistance) / (mass * 4));
            if force < resistance {
                (-1).min(accel)
            } else {
                1.max(accel)
            }
        } else {
            crate::math::clamp_to_i32((-force - resistance).min(-10000) / mass)
        }
    }

    /// Check whether the whole vehicle chain is in the depot.
    fn is_chain_in_depot(s: &Services, this: VehicleRef, o: Owner) -> bool {
        let v = this.first();
        /* Is the front engine stationary in the depot? */
        if !crate::map_func::World::new(s.shared()).depot(v.tile(), T::TRANSPORT) || v.speed() != 0
        {
            return false;
        }

        /* Check whether the rest is also already trying to enter the depot. */
        let mut part = Some(v);
        while let Some(u) = part {
            let uo = if u == this { o } else { Owner::of(u) };
            if !T::is_in_depot(uo) || u.tile() != this.tile() {
                return false;
            }
            part = u.next();
        }

        hit(witness::GROUND_DEPOT);
        true
    }

    /// Calculates the total slope resistance for this vehicle.
    #[inline]
    fn get_slope_resistance(v: VehicleRef, first: Owner) -> i64 {
        let mut incl = 0_i64;

        let mut part = Some(v);
        while let Some(u) = part {
            let o = if u == v { first } else { Owner::of(u) };
            if o.has_flag(GroundVehicleFlags::GVF_GOINGUP_BIT) {
                hit(witness::GROUND_UPHILL);
                incl += i64::from(read_field!(o.state, cache.cached_slope_resistance));
            } else if o.has_flag(GroundVehicleFlags::GVF_GOINGDOWN_BIT) {
                hit(witness::GROUND_DOWNHILL);
                incl -= i64::from(read_field!(o.state, cache.cached_slope_resistance));
            }
            part = u.next();
        }

        incl
    }

    /// Updates vehicle's Z position and inclination. Used when the vehicle entered given tile.
    /// The vehicle has to be at (or near to) a border of the tile, directed towards tile centre.
    #[inline]
    fn update_z_position_and_inclination(
        movement: &crate::movement::CachedTable<'_>,
        v: VehicleRef,
        o: Owner,
    ) {
        v.write_z(movement.get().get_slope_pixel_z(v.x(), v.y(), true));
        let mut flags = o.flags();
        clr_bit16(&mut flags, GroundVehicleFlags::GVF_GOINGUP_BIT.0);
        clr_bit16(&mut flags, GroundVehicleFlags::GVF_GOINGDOWN_BIT.0);
        o.set_flags(flags);

        if T::tile_may_have_sloped_track(movement, v, o) {
            /* To check whether the current tile is sloped, and in which
             * direction it is sloped, we get the 'z' at the center of
             * the tile (middle_z) and the edge of the tile (old_z),
             * which we then can compare. */
            let middle_z = movement.get().get_slope_pixel_z(
                (v.x() & !TILE_UNIT_MASK.cast_signed()) | (TILE_SIZE / 2).cast_signed(),
                (v.y() & !TILE_UNIT_MASK.cast_signed()) | (TILE_SIZE / 2).cast_signed(),
                true,
            );

            if middle_z != v.z() {
                let mut flags = o.flags();
                set_bit16(
                    &mut flags,
                    if middle_z > v.z() {
                        GroundVehicleFlags::GVF_GOINGUP_BIT.0
                    } else {
                        GroundVehicleFlags::GVF_GOINGDOWN_BIT.0
                    },
                );
                o.set_flags(flags);
            }
        }
    }

    /// Updates vehicle's Z position. Inclination can't change in the middle of a tile. The
    /// faster code is used for trains and road vehicles unless they are reversing on a sloped
    /// tile.
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "`int8_t x_pos = this->x_pos` and `int8_t y_pos = this->y_pos`"
    )]
    fn update_z_position(
        s: &Services,
        movement: &crate::movement::CachedTable<'_>,
        v: VehicleRef,
        o: Owner,
    ) {
        /* Vehicle's Z position can change only if it has GVF_GOINGUP_BIT or GVF_GOINGDOWN_BIT set.
         * Furthermore, if this function is called once every time the vehicle's position changes,
         * we know the Z position changes by +/-1 at certain moments - when x_pos, y_pos is odd/even,
         * depending on orientation of the slope and vehicle's direction */

        if o.has_flag(GroundVehicleFlags::GVF_GOINGUP_BIT)
            || o.has_flag(GroundVehicleFlags::GVF_GOINGDOWN_BIT)
        {
            if T::has_to_use_get_slope_pixel_z(v) {
                /* In some cases, we have to use GetSlopePixelZ() */
                v.write_z(movement.get().get_slope_pixel_z(v.x(), v.y(), true));
                return;
            }
            let check = |ok| crate::movement::native_assert(s.shared(), ok);
            /* DirToDiagDir() is a simple right shift */
            let dir = dir_to_diag_dir(Direction(v.direction()), check);
            /* Read variables, so the compiler knows the access doesn't trap */
            let x_pos = v.x() as i8;
            let y_pos = v.y() as i8;
            /* DiagDirToAxis() is a simple mask */
            let mut d = if diag_dir_to_axis(dir, check) == Axis::AXIS_X {
                x_pos
            } else {
                y_pos
            };
            /* We need only the least significant bit */
            d &= 1;
            d ^= i8::from(dir == DiagDirection::DIAGDIR_NW || dir == DiagDirection::DIAGDIR_NE);
            /* Subtraction instead of addition because we are testing for GVF_GOINGUP_BIT.
             * GVF_GOINGUP_BIT is used because it's bit 0, so simple AND can be used,
             * without any shift */
            let delta = if o.has_flag(GroundVehicleFlags::GVF_GOINGUP_BIT) {
                d
            } else {
                -d
            };
            v.write_z(v.z().wrapping_add(i32::from(delta)));
        }

        s.assert(|| v.z() == movement.get().get_slope_pixel_z(v.x(), v.y(), true));
    }

    /// Checks if the vehicle is in a slope and sets the required flags in that case. Returns
    /// the old height of the vehicle.
    #[inline]
    fn update_inclination(
        s: &Services,
        movement: &crate::movement::CachedTable<'_>,
        v: VehicleRef,
        o: Owner,
        new_tile: bool,
        update_delta: bool,
    ) -> i32 {
        let old_z = v.z();

        if new_tile {
            Self::update_z_position_and_inclination(movement, v, o);
        } else {
            Self::update_z_position(s, movement, v, o);
        }

        (s.viewport)(v, true, update_delta);
        old_z
    }
}

/// Run a `GroundVehicle<T>` method on the instantiation for the owner's vehicle type.
macro_rules! dispatch {
    ($owner:expr, $method:ident($($arg:expr),*)) => {
        if $owner.kind == VehicleType::VEH_TRAIN {
            GroundVehicle::<Train>::$method($($arg),*)
        } else {
            GroundVehicle::<RoadVehicle>::$method($($arg),*)
        }
    };
}

/// `GroundVehicle::PowerChanged`.
pub(crate) fn power_changed(s: &Services, v: VehicleRef) {
    let o = Owner::of(v);
    dispatch!(o, power_changed(s, v, o));
}
/// `GroundVehicle::CargoChanged`.
pub(crate) fn cargo_changed(s: &Services, v: VehicleRef) {
    let o = Owner::of(v);
    dispatch!(o, cargo_changed(s, v, o));
}
/// `GroundVehicle::GetAcceleration`.
pub(crate) fn get_acceleration(s: &Services, v: VehicleRef) -> i32 {
    get_acceleration_owned(s, v, Owner::of(v))
}
/// `GroundVehicle::GetAcceleration` with the owner the caller resolved.
#[inline]
pub(crate) fn get_acceleration_owned(s: &Services, v: VehicleRef, o: Owner) -> i32 {
    dispatch!(o, get_acceleration(s, v, o))
}
/// `GroundVehicle::IsChainInDepot`.
pub(crate) fn is_chain_in_depot(s: &Services, v: VehicleRef) -> bool {
    let o = Owner::of(v);
    dispatch!(o, is_chain_in_depot(s, v, o))
}
/// `Train::GetMaxWeight` or `RoadVehicle::GetMaxWeight`.
pub(crate) fn get_max_weight(s: &Services, v: VehicleRef) -> u16 {
    let o = Owner::of(v);
    if o.kind == VehicleType::VEH_TRAIN {
        Train::get_max_weight(s, v, o)
    } else {
        RoadVehicle::get_max_weight(s, v, o)
    }
}

/// `GroundVehicle::Crash`: common code executed for crashed ground vehicles. Returns the
/// number of victims.
pub(crate) fn crash(s: &Services, v: VehicleRef, flooded: bool) -> u32 {
    /* Crashed vehicles aren't going up or down */
    let mut part = Some(v);
    while let Some(u) = part {
        let o = Owner::of(u);
        let mut flags = o.flags();
        clr_bit16(&mut flags, GroundVehicleFlags::GVF_GOINGUP_BIT.0);
        clr_bit16(&mut flags, GroundVehicleFlags::GVF_GOINGDOWN_BIT.0);
        o.set_flags(flags);
        part = u.next();
    }
    hit(if flooded {
        witness::GROUND_FLOOD
    } else {
        witness::GROUND_CRASH
    });
    (s.base_crash)(v, flooded)
}

/// `GroundVehicle::UpdateInclination`.
pub(crate) fn update_inclination(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    new_tile: bool,
    update_delta: bool,
) -> i32 {
    update_inclination_owned(s, movement, v, Owner::of(v), new_tile, update_delta)
}
/// `GroundVehicle::UpdateInclination` with the owner the caller resolved.
#[inline]
pub(crate) fn update_inclination_owned(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    o: Owner,
    new_tile: bool,
    update_delta: bool,
) -> i32 {
    dispatch!(
        o,
        update_inclination(s, movement, v, o, new_tile, update_delta)
    )
}

/// `GroundVehicle::SetLastSpeed`: update the GUI variant of the current speed of the vehicle.
/// Also mark the widget dirty when that is needed, i.e. when the speed of this vehicle has
/// changed.
#[inline]
pub(crate) fn set_last_speed(s: &Services, v: VehicleRef) {
    set_last_speed_owned(s, v, Owner::of(v));
}
/// `GroundVehicle::SetLastSpeed` with the owner the caller resolved.
#[inline]
pub(crate) fn set_last_speed_owned(s: &Services, v: VehicleRef, o: Owner) {
    if v.speed() != read_field!(o.state, cache.last_speed) {
        (s.vehicle.start_stop_dirty)(v);
        write_field!(o.state, cache.last_speed, v.speed());
    }
}

/// `GroundVehicle::DoUpdateSpeed`: update the speed of the vehicle.
///
/// It updates the `cur_speed` and `subspeed` variables depending on the state of the vehicle;
/// in this case the current acceleration, minimum and maximum speeds of the vehicle. It returns
/// the distance that the vehicle can drive this tick. `Vehicle::GetAdvanceDistance()`
/// determines the distance to drive before moving a step on the map.
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "DoUpdateSpeed's (uint8_t)spd, (int)spd, the int to uint assignment of spd and its uint return"
)]
pub(crate) fn do_update_speed(v: VehicleRef, accel: u32, min_speed: i32, max_speed: i32) -> u32 {
    hit(witness::GROUND_SPEED);
    let mut spd = u32::from(v.subspeed()).wrapping_add(accel);
    v.write_subspeed(spd as u8);

    /* When we are going faster than the maximum speed, reduce the speed
     * somewhat gradually. But never lower than the maximum speed. */
    let mut tempmax = max_speed;
    let cur_speed = i32::from(v.speed());
    if cur_speed > max_speed {
        tempmax = (cur_speed - cur_speed / 10 - 1).max(max_speed);
    }

    /* Enforce a maximum and minimum speed. Normally we would use something like
     * Clamp for this, but in this case min_speed might be below the maximum speed
     * threshold for some reason. That makes acceleration fail and assertions
     * happen in Clamp. So make it explicit that min_speed overrules the maximum
     * speed by explicit ordering of min and max. */
    spd = cur_speed
        .wrapping_add((spd as i32) >> 8)
        .min(tempmax)
        .max(min_speed) as u32;
    v.write_speed(spd as u16);

    let mut scaled_spd = crate::vehicle::advance_speed(spd) as i32;

    scaled_spd = scaled_spd.wrapping_add(i32::from(v.progress()));
    v.write_progress(0); // set later in *Handler or *Controller
    scaled_spd as u32
}

/// Export bodies take the immutable service table and a live record.
macro_rules! entry {
    ($export:ident, $function:ident $(, $arg:ident : $ty:ty)* => $ret:ty) => {
        /// # Safety
        /// Services and shell must remain live under the game-state mutex.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $export(s: *const Services, v: VehicleRef, $($arg: $ty),*) -> $ret {
            // SAFETY: immutable native table lives for the synchronous entry.
            $function(unsafe {&*s}, v, $($arg),*)
        }
    };
}
entry!(openttd_rust_ground_power_changed, power_changed => ());
entry!(openttd_rust_ground_cargo_changed, cargo_changed => ());
entry!(openttd_rust_ground_acceleration, get_acceleration => i32);
/// # Safety
/// Services and shell must remain live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_chain_in_depot_abi(
    s: *const Services,
    v: VehicleRef,
) -> u8 {
    // SAFETY: immutable native table lives for the synchronous entry.
    u8::from(is_chain_in_depot(unsafe { &*s }, v))
}
entry!(openttd_rust_ground_crash, crash, flooded: bool => u32);
entry!(openttd_rust_ground_max_weight, get_max_weight => u16);
/// # Safety
/// Services and shell must remain live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_acceleration_type(
    s: *const Services,
    v: VehicleRef,
) -> u8 {
    // SAFETY: immutable native table lives for the synchronous entry.
    Train::get_acceleration_type(unsafe { &*s }, v).0
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGroundPhysicsServices" ["probe"] => crate::ground_vehicle::Services { vehicle, road, assertions, capabilities, cargo_weight, train_cargo_weight, powered, rail_type, rail_acceleration, rail_speed, steepness, viewport, base_crash, };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static FAILED: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED.set(FAILED.get() + 1);
        }
    }
    extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
        0
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    extern "C" fn get_slope_pixel_z_evaluated(_: u32) -> u8 {
        panic!("GetSlopePixelZ was evaluated for the assertion with assertions off")
    }
    extern "C" fn viewport(_: VehicleRef, _: bool, _: bool) {}

    /// Ground services whose native assertion records failures, over `shared`.
    fn services(shared: Box<crate::services::Services>, assertions: bool) -> &'static Services {
        let shared: &'static mut crate::services::Services = Box::leak(shared);
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            shared: std::ptr::from_ref(shared),
            native_assert: record_assertion,
            ..crate::test_tables::MOVEMENT
        }));
        shared.movement = std::ptr::from_ref(movement);
        let capabilities = Box::leak(Box::new(
            crate::engine_capabilities::test_table_with_shared(shared),
        ));
        Box::leak(Box::new(Services {
            assertions,
            capabilities,
            viewport,
            ..crate::test_tables::GROUND_VEHICLE
        }))
    }

    /// A part type with fixed per-type values, so the tests observe only the template.
    struct Fixed;
    impl GroundVehicleType for Fixed {
        const TRANSPORT: u8 = TRANSPORT_ROAD;
        fn get_power(_: &Services, _: VehicleRef, _: Owner) -> u16 {
            0
        }
        fn get_powered_part_power(_: &Services, _: VehicleRef, _: Owner, _: Owner) -> u16 {
            0
        }
        fn get_weight(_: &Services, _: VehicleRef, _: Owner) -> u16 {
            5
        }
        fn get_max_weight(_: &Services, _: VehicleRef, _: Owner) -> u16 {
            unreachable!()
        }
        fn get_tractive_effort(_: &Services, _: VehicleRef) -> u8 {
            unreachable!()
        }
        fn get_air_drag(_: &Services, _: VehicleRef) -> u8 {
            // "No air drag", so the display speed is not read.
            1
        }
        fn get_air_drag_area(_: VehicleRef, _: Owner) -> u8 {
            unreachable!()
        }
        fn get_acceleration_status(_: VehicleRef, _: Owner) -> AccelStatus {
            unreachable!()
        }
        fn get_current_speed(_: VehicleRef) -> u16 {
            unreachable!()
        }
        fn get_rolling_friction(_: &Services, _: VehicleRef) -> u32 {
            unreachable!()
        }
        fn get_acceleration_type(_: &Services, _: VehicleRef) -> VehicleAccelerationModel {
            unreachable!()
        }
        fn get_slope_steepness(_: &Services) -> u32 {
            2
        }
        fn get_display_max_speed(_: VehicleRef) -> i32 {
            unreachable!()
        }
        fn get_max_track_speed(_: &Services, _: VehicleRef) -> u16 {
            0
        }
        fn tile_may_have_sloped_track(
            _: &crate::movement::CachedTable<'_>,
            _: VehicleRef,
            _: Owner,
        ) -> bool {
            unreachable!()
        }
        fn has_to_use_get_slope_pixel_z(_: VehicleRef) -> bool {
            unreachable!()
        }
        fn is_in_depot(_: Owner) -> bool {
            unreachable!()
        }
    }

    /// A road record whose `First()` is another record, with its ground owner.
    struct LaterPart {
        front: *mut crate::vehicle_base::VehicleRecord,
        part: *mut crate::vehicle_base::VehicleRecord,
        ground: Box<GroundState>,
    }
    impl LaterPart {
        fn new() -> Self {
            let front = crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                VehicleType::VEH_ROAD,
            );
            let part = crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                VehicleType::VEH_ROAD,
            );
            let mut ground = Box::<GroundState>::default();
            // SAFETY: both records are live and unshared.
            unsafe {
                (*part).first = front;
                (*part).ground = &raw mut *ground;
                (*part).cached_max_speed = 77;
            }
            Self {
                front,
                part,
                ground,
            }
        }
        fn v(&self) -> VehicleRef {
            VehicleRef::of(self.part)
        }
    }
    impl Drop for LaterPart {
        fn drop(&mut self) {
            // SAFETY: both records came from `openttd_rust_vehicle_record_new`.
            unsafe {
                crate::vehicle_base::openttd_rust_vehicle_record_delete(self.part);
                crate::vehicle_base::openttd_rust_vehicle_record_delete(self.front);
            }
        }
    }

    /// `PowerChanged`'s `assert(this->First() == this)` reports through the native
    /// assertion on a later part, and the body then runs as the release build does.
    #[test]
    fn power_changed_on_a_later_part_reports_the_failed_assertion_and_continues() {
        FAILED.set(0);
        let s = services(
            Box::new(crate::services::fixture(
                std::ptr::null_mut(),
                random,
                industry,
            )),
            true,
        );
        let part = LaterPart::new();
        let v = part.v();
        GroundVehicle::<Fixed>::power_changed(s, v, Owner::of(v));
        assert_eq!(FAILED.get(), 1);
        assert_eq!(part.ground.cache.cached_max_track_speed, 77);
        assert_eq!(part.ground.cache.cached_air_drag, 0);
    }

    /// `CargoChanged` reports its own failed assertion, then `PowerChanged`'s, and writes the
    /// weight caches between them.
    #[test]
    fn cargo_changed_on_a_later_part_reports_both_failed_assertions_and_continues() {
        FAILED.set(0);
        let s = services(
            Box::new(crate::services::fixture(
                std::ptr::null_mut(),
                random,
                industry,
            )),
            true,
        );
        let part = LaterPart::new();
        let v = part.v();
        GroundVehicle::<Fixed>::cargo_changed(s, v, Owner::of(v));
        assert_eq!(FAILED.get(), 2);
        assert_eq!(part.ground.cache.cached_weight, 5);
        assert_eq!(part.ground.cache.cached_slope_resistance, 5 * 2 * 100);
        assert_eq!(part.ground.cache.cached_axle_resistance, 50);
    }

    /// `UpdateZPosition` asserts `z_pos == GetSlopePixelZ(x_pos, y_pos, true)`: a level part
    /// at height 0 over a flat tile at height 3 reports one failed assertion.
    #[test]
    fn update_z_position_reports_a_z_that_differs_from_the_slope() {
        FAILED.set(0);
        let mut tiles: Vec<_> = (0..64 * 64)
            .map(|_| crate::map_func::BaseTileLayout {
                kind: 0,
                height: 3,
                m1: 0,
                m2: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let extended: Vec<_> = (0..64 * 64)
            .map(|_| crate::map_func::ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: tiles.as_mut_ptr(),
                extended: extended.as_ptr(),
                count: 64 * 64,
                assertions: true,
            },
            6,
        );
        let mut shared = crate::services::fixture(std::ptr::null_mut(), random, industry);
        shared.map = crate::map_func::fixture::statics();
        let s = services(Box::new(shared), true);
        let part = LaterPart::new();
        let v = part.v();
        v.write_xy(5 * 16 + 8, 5 * 16 + 8);
        let movement = crate::movement::CachedTable::new(s.shared());
        GroundVehicle::<Fixed>::update_z_position(s, &movement, v, Owner::of(v));
        assert_eq!(FAILED.get(), 1);
        v.write_z(3 * crate::tile_type::TILE_HEIGHT.cast_signed());
        GroundVehicle::<Fixed>::update_z_position(s, &movement, v, Owner::of(v));
        assert_eq!(FAILED.get(), 1);
    }

    /// With assertions compiled out, `UpdateZPosition` does not evaluate `GetSlopePixelZ`
    /// for its assertion: the tile type read it would start fails the test.
    #[test]
    fn update_z_position_without_assertions_does_not_read_the_slope() {
        FAILED.set(0);
        let mut shared = crate::services::fixture(std::ptr::null_mut(), random, industry);
        shared.tile_type = get_slope_pixel_z_evaluated;
        let s = services(Box::new(shared), false);
        let part = LaterPart::new();
        let v = part.v();
        let movement = crate::movement::CachedTable::new(s.shared());
        GroundVehicle::<Fixed>::update_z_position(s, &movement, v, Owner::of(v));
        assert_eq!(FAILED.get(), 0);
    }

    /// `Owner::of` reads the ground, train and road owner links of the shared record, which
    /// the native train and road vehicle constructors set.
    #[test]
    fn owner_of_reads_the_record_links_by_vehicle_type() {
        let mut ground = GroundState::default();
        let train = crate::train_state::openttd_rust_train_state_new();
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            VehicleType::VEH_TRAIN,
        );
        // SAFETY: the record and owners are live and unshared for this test.
        unsafe {
            (*record).ground = &raw mut ground;
            (*record).train = train;
        }
        let o = Owner::of(VehicleRef::of(record));
        assert_eq!(o.kind, VehicleType::VEH_TRAIN);
        assert_eq!(o.state, &raw mut ground);
        assert_eq!((o.train.record, o.train.owner), (record, train));
        assert!(o.road.is_null());

        let mut road = crate::road::State::default();
        let road_record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            VehicleType::VEH_ROAD,
        );
        // SAFETY: as above.
        unsafe {
            (*road_record).ground = &raw mut ground;
            (*road_record).road = &raw mut road.fields;
        }
        let o = Owner::of(VehicleRef::of(road_record));
        assert_eq!(o.kind, VehicleType::VEH_ROAD);
        assert_eq!(o.state, &raw mut ground);
        assert_eq!(o.road, &raw mut road);
        assert!(o.train.record.is_null());
        // SAFETY: each allocation is released once.
        unsafe {
            crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
            crate::vehicle_base::openttd_rust_vehicle_record_delete(road_record);
            crate::train_state::openttd_rust_train_state_destroy(train);
        }
    }
}

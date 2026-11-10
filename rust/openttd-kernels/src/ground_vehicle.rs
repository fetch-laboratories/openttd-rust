/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ground vehicle cache and flags, dynamics and the train and road physics callees. Native
//! shells own one stable allocation.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::struct_field_names
)]
use crate::vehicle_base::{GVSF_ENGINE, GVSF_FREE_WAGON, GVSF_FRONT, GVSF_MULTIHEADED, GVSF_WAGON};

const fn has(subtype: u8, flag: u8) -> bool {
    subtype & (1 << flag) != 0
}
/// `SetFrontEngine`.
pub(crate) const fn set_front_engine(subtype: u8) -> u8 {
    subtype | 1 << GVSF_FRONT
}
/// `ClearFrontEngine`.
pub(crate) const fn clear_front_engine(subtype: u8) -> u8 {
    subtype & !(1 << GVSF_FRONT)
}
/// `SetEngine`.
pub(crate) const fn set_engine(subtype: u8) -> u8 {
    subtype | 1 << GVSF_ENGINE
}
/// `SetFreeWagon`.
pub(crate) const fn set_free_wagon(subtype: u8) -> u8 {
    subtype | 1 << GVSF_FREE_WAGON
}
/// `ClearFreeWagon`.
pub(crate) const fn clear_free_wagon(subtype: u8) -> u8 {
    subtype & !(1 << GVSF_FREE_WAGON)
}
/// `SetMultiheaded`.
pub(crate) const fn set_multiheaded(subtype: u8) -> u8 {
    subtype | 1 << GVSF_MULTIHEADED
}
/// `IsFreeWagon`.
pub(crate) const fn is_free_wagon(subtype: u8) -> bool {
    has(subtype, GVSF_FREE_WAGON)
}
/// `IsEngine`.
pub(crate) const fn is_engine(subtype: u8) -> bool {
    has(subtype, GVSF_ENGINE)
}
/// `IsWagon`.
pub(crate) const fn is_wagon(subtype: u8) -> bool {
    has(subtype, GVSF_WAGON)
}
/// `IsMultiheaded`.
pub(crate) const fn is_multiheaded(subtype: u8) -> bool {
    has(subtype, GVSF_MULTIHEADED)
}
/// `IsRearDualheaded`.
pub(crate) const fn is_rear_dualheaded(subtype: u8) -> bool {
    is_multiheaded(subtype) && !is_engine(subtype)
}

crate::abi::shared_records! {
    fn abi_record;
    /// Canonical cache; native aliases borrow this allocation.
    #[derive(Default, PartialEq, Eq)]
    "GroundVehicleCache" ["ground"] => pub struct GroundCache {
        pub cached_weight: u32,
        pub cached_slope_resistance: u32,
        pub cached_max_te: u32,
        pub cached_axle_resistance: u16,
        pub cached_max_track_speed: u16,
        pub cached_power: u32,
        pub cached_air_drag: u32,
        pub cached_total_length: u16,
        /// Invalid for the front vehicle itself.
        pub first_engine: u16,
        pub cached_veh_length: u8,
        pub last_speed: u16,
    }
    /// One stable owner per native train/road shell.
    #[derive(Default)]
    "OpenTTDGroundState" ["ground"] => pub struct GroundState {
        pub cache: GroundCache,
        pub flags: u16,
    }
}
pub type Cache = GroundCache;
pub type State = GroundState;

/// Compare cache values without observing their padding bytes.
/// # Safety
/// Both pointers identify live caches for this synchronous comparison.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_cache_equal(a: *const Cache, b: *const Cache) -> u8 {
    // SAFETY: the caller supplies live caches; this call invokes no callbacks.
    u8::from(unsafe { *a == *b })
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_ground_new() -> *mut State {
    Box::into_raw(Box::new(State {
        cache: Cache {
            first_engine: u16::MAX,
            ..Cache::default()
        },
        flags: 0,
    }))
}
/// # Safety
/// The shell transfers its sole live owner after its final native access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_destroy(state: *mut State) {
    // SAFETY: exactly one matching allocation, no aliases remain in use.
    drop(unsafe { Box::from_raw(state) });
}

use crate::direction_func::{diag_dir_to_axis_raw, dir_to_diag_dir_raw};
use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW};
use crate::engine_type::{ACCELERATION_MAGLEV, ACCELERATION_NORMAL};
use crate::newgrf_properties::{
    PROP_ROADVEH_POWER, PROP_ROADVEH_TRACTIVE_EFFORT, PROP_ROADVEH_WEIGHT, PROP_TRAIN_POWER,
    PROP_TRAIN_TRACTIVE_EFFORT, PROP_TRAIN_WEIGHT,
};
use crate::services::{RailType, RoadType};
use crate::tile_type::{TILE_SIZE, TILE_UNIT_MASK};
use crate::track_func::track_status_to_track_bits;
use crate::track_type::{TRACK_BIT_WORMHOLE, TRACK_BIT_X, TRACK_BIT_Y};
use crate::vehicle::{VehicleRef, VehicleServices, VehicleType};
use crate::vehicle_base::{VS_HIDDEN, VS_STOPPED, is_articulated_part};
use crate::vehicle_type::GROUND_ACCELERATION;
use crate::witness::{self, hit};

pub const GVF_SUPPRESS_IMPLICIT_ORDERS: u16 = 2;
pub const GVF_GOINGUP_BIT: u16 = 0;
pub const GVF_GOINGDOWN_BIT: u16 = 1;
const SLOPE_FLAGS: u16 = (1 << GVF_GOINGUP_BIT) | (1 << GVF_GOINGDOWN_BIT);
pub const AS_ACCEL: u8 = 0;
pub const AS_BRAKE: u8 = 1;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Owner {
    pub state: *mut State,
    pub train: crate::train::Handle,
    pub road: *mut crate::road::State,
    pub kind: VehicleType,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WeightInput {
    pub cargo: *const std::ffi::c_void,
    pub owner: *mut crate::cargo_storage::List,
}

/// Immutable process-wide typed services. No owner borrow crosses any service;
/// native shells and the canonical allocation are live under the game-state mutex.
#[repr(C)]
pub struct Services {
    pub vehicle: &'static VehicleServices,
    pub road: &'static crate::road::Leaves,
    pub assertions: bool,
    pub resolve: extern "C" fn(VehicleRef) -> Owner,
    pub weight_input: extern "C" fn(VehicleRef) -> WeightInput,
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub cargo_weight: extern "C" fn(*const std::ffi::c_void, u32) -> u64,
    pub train_cargo_weight: extern "C" fn(*const std::ffi::c_void, u32) -> u64,
    pub powered: extern "C" fn(u64, RailType) -> bool,
    pub rail_type: extern "C" fn(u32) -> RailType,
    pub rail_acceleration: extern "C" fn(RailType) -> u8,
    pub rail_speed: extern "C" fn(RailType) -> u16,
    pub road_speed: extern "C" fn(VehicleRef) -> u16,
    pub steepness: extern "C" fn(VehicleType) -> u32,
    pub display_max_speed: extern "C" fn(VehicleRef) -> i32,
    pub viewport: extern "C" fn(VehicleRef, bool, bool),
    pub base_crash: extern "C" fn(VehicleRef, bool) -> u32,
    pub depot_tile: extern "C" fn(u32, u8) -> bool,
}

impl Owner {
    /// The road shell owns separate stable road and ground allocations.
    pub(crate) fn for_road(state: *mut State, road: *mut crate::road::State) -> Self {
        Self {
            state,
            train: crate::train::Handle::NONE,
            road,
            kind: VehicleType::ROAD,
        }
    }
    fn rail(self) -> bool {
        self.kind == VehicleType::TRAIN
    }
    fn powered_wagon(self) -> bool {
        self.rail() && self.train.get_flags() & crate::train::FLAG_POWERED_WAGON != 0
    }
}

fn power_owned(s: &Services, v: VehicleRef, o: Owner) -> u16 {
    let tile = if o.rail() { v.tile() } else { 0 };
    let subtype = v.subtype();
    if is_articulated_part(true, subtype) {
        return 0;
    }
    if o.rail() && !(s.powered)(o.train.get_railtypes(), (s.rail_type)(tile)) {
        return 0;
    }
    let p = (s.vehicle.property)(
        (s.vehicle.newgrf)(),
        v,
        if o.rail() {
            PROP_TRAIN_POWER
        } else {
            PROP_ROADVEH_POWER
        },
        i32::from((s.vehicle.engine)(v).ground_power(VehicleType(v.vehicle_type()))),
    );
    if o.rail() {
        let mut p = p as u16;
        if is_multiheaded(v.subtype()) {
            p /= 2;
        }
        p
    } else {
        p.wrapping_mul(10) as u16
    }
}
fn powered_part_owned(s: &Services, v: VehicleRef, o: Owner, head: VehicleRef) -> u16 {
    if o.powered_wagon() {
        let head = if head == v { o } else { (s.resolve)(head) };
        if (s.powered)(head.train.get_railtypes(), (s.rail_type)((v).tile())) {
            // SAFETY: scalar load ends before the specification lookup.
            return crate::vehicle::EngineRef::of((s.capabilities.engine)(unsafe {
                (*o.state).cache.first_engine
            }))
            .wagon_power();
        }
    }
    0
}
pub(crate) fn weight(s: &Services, v: VehicleRef, full: bool) -> u16 {
    weight_owned(s, v, (s.resolve)(v), full)
}
fn weight_owned(s: &Services, v: VehicleRef, o: Owner, full: bool) -> u16 {
    if full {
        hit(witness::GROUND_FULL_WEIGHT);
    }
    let r = (s.weight_input)(v);
    let count = if full {
        unsafe {
            crate::engine_capabilities::openttd_rust_engine_capacity(
                s.capabilities,
                (s.vehicle.engine)(v).0.as_ptr(),
                v.0.as_ptr(),
                std::ptr::null_mut(),
            )
        }
    } else {
        // SAFETY: live canonical cargo owner; no borrow survives this read.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_stored(r.owner) }
    };
    let mut weight = if o.rail() {
        (s.train_cargo_weight)(r.cargo, count)
    } else {
        (s.cargo_weight)(r.cargo, count)
    } as u16;
    if !is_articulated_part(true, v.subtype()) {
        let w = (s.vehicle.property)(
            (s.vehicle.newgrf)(),
            v,
            if o.rail() {
                PROP_TRAIN_WEIGHT
            } else {
                PROP_ROADVEH_WEIGHT
            },
            i32::from((s.vehicle.engine)(v).ground_weight(VehicleType(v.vehicle_type()))),
        );
        weight = weight.wrapping_add(if o.rail() { w } else { w / 4 } as u16);
    }
    if o.powered_wagon() {
        // SAFETY: no borrow spans the spec leaf.
        weight = weight.wrapping_add(u16::from(
            crate::vehicle::EngineRef::of((s.capabilities.engine)(unsafe {
                (*o.state).cache.first_engine
            }))
            .wagon_weight(),
        ));
    }
    weight
}
fn tractive_effort_owned(s: &Services, v: VehicleRef, o: Owner) -> u8 {
    (s.vehicle.property)(
        (s.vehicle.newgrf)(),
        v,
        if o.rail() {
            PROP_TRAIN_TRACTIVE_EFFORT
        } else {
            PROP_ROADVEH_TRACTIVE_EFFORT
        },
        i32::from((s.vehicle.engine)(v).ground_te(VehicleType(v.vehicle_type()))),
    ) as u8
}
pub(crate) fn air_drag(s: &Services, v: VehicleRef) -> u8 {
    (s.vehicle.engine)(v).ground_drag(VehicleType(v.vehicle_type()))
}
fn air_drag_area_for(o: Owner, status: u8) -> u8 {
    if !o.rail() {
        6
    } else if o.train.track() == TRACK_BIT_WORMHOLE && status & VS_HIDDEN != 0 {
        28
    } else {
        14
    }
}
fn acceleration_status_for(o: Owner, status: u8) -> u8 {
    if status & VS_STOPPED != 0
        || (o.rail()
            && o.train.get_flags() & (crate::train::FLAG_REVERSING | crate::train::FLAG_STUCK) != 0)
    {
        AS_BRAKE
    } else {
        AS_ACCEL
    }
}
fn current_speed_for(o: Owner, speed: u16) -> u16 {
    if o.rail() { speed } else { speed / 2 }
}
fn rolling_friction_for(o: Owner, roadtype: RoadType, speed: u16) -> u32 {
    if o.rail() {
        15 * (512 + u32::from(speed)) / 512
    } else {
        let coeff = if crate::road::road_type_is_tram(roadtype.0) {
            40
        } else {
            75
        };
        coeff * (128 + u32::from(speed)) / 128
    }
}
pub(crate) fn acceleration_type(s: &Services, v: VehicleRef) -> u8 {
    let o = (s.resolve)(v);
    acceleration_type_for(s, o, if o.rail() { (v).tile() } else { 0 })
}
fn acceleration_type_for(s: &Services, o: Owner, tile: u32) -> u8 {
    if o.rail() {
        (s.rail_acceleration)((s.rail_type)(tile))
    } else {
        ACCELERATION_NORMAL
    }
}
fn max_track_speed_owned(s: &Services, v: VehicleRef, o: Owner) -> u16 {
    if o.rail() {
        (s.rail_speed)((s.rail_type)((v).tile()))
    } else {
        (s.road_speed)(v)
    }
}
fn sloped_track_owned(
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    o: Owner,
) -> bool {
    let tracks = if o.rail() {
        o.train.track()
    } else {
        track_status_to_track_bits(movement.road_tracks(v, (v).tile()))
    };
    tracks == TRACK_BIT_X || tracks == TRACK_BIT_Y
}

pub(crate) fn power_changed(s: &Services, v: VehicleRef) {
    power_changed_owned(s, v, (s.resolve)(v));
}
fn power_changed_owned(s: &Services, v: VehicleRef, o: Owner) {
    if s.assertions {
        assert_eq!(v.first(), v);
    }
    hit(if o.rail() {
        witness::GROUND_RAIL_POWER
    } else {
        witness::GROUND_ROAD_POWER
    });
    let mut total_power = 0_u32;
    let mut max_te = 0_u32;
    let mut parts = 0_u32;
    let mut max_track_speed = (v).max_speed();
    let mut part = Some(v);
    while let Some(u) = part {
        let uo = if u == v { o } else { (s.resolve)(u) };
        let current_power =
            u32::from(power_owned(s, u, uo)) + u32::from(powered_part_owned(s, u, uo, u));
        total_power = total_power.wrapping_add(current_power);
        if current_power > 0 {
            max_te = max_te.wrapping_add(
                u32::from(weight_owned(s, u, uo, false))
                    * u32::from(tractive_effort_owned(s, u, uo)),
            );
        }
        parts = parts.wrapping_add(1);
        let speed = max_track_speed_owned(s, u, uo);
        if speed > 0 {
            max_track_speed = max_track_speed.min(speed);
        }
        part = (u).next();
    }
    let drag = air_drag(s, v);
    let drag = if drag == 0 {
        let speed = (s.display_max_speed)(v) as u16;
        if speed <= 10 {
            192
        } else {
            (2048 / speed).max(1) as u8
        }
    } else if drag == 1 {
        0
    } else {
        drag
    };
    // SAFETY: each scalar access ends before any callback; cache remains canonical.
    unsafe {
        (*o.state).cache.cached_air_drag = u32::from(drag)
            .wrapping_add(3_u32.wrapping_mul(u32::from(drag)).wrapping_mul(parts) / 20);
    }
    max_te = max_te.wrapping_mul(GROUND_ACCELERATION) / 256;
    if unsafe {
        (*o.state).cache.cached_power != total_power || (*o.state).cache.cached_max_te != max_te
    } {
        if total_power == 0 {
            let status = (v).status();
            (v).write_status(status | VS_STOPPED);
        }
        unsafe {
            (*o.state).cache.cached_power = total_power;
            (*o.state).cache.cached_max_te = max_te;
        }
        // SAFETY: The canonical vehicle table names the immutable process-lifetime maintenance table.
        (unsafe { &*s.vehicle.maintenance }.details_dirty)(v);
        (s.vehicle.start_stop_dirty)(v);
    }
    unsafe {
        (*o.state).cache.cached_max_track_speed = max_track_speed;
    }
}
pub(crate) fn cargo_changed(s: &Services, v: VehicleRef) {
    if s.assertions {
        assert_eq!(v.first(), v);
    }
    let o = (s.resolve)(v);
    hit(if o.rail() {
        witness::GROUND_RAIL_WEIGHT
    } else {
        witness::GROUND_ROAD_WEIGHT
    });
    let mut total = 0_u32;
    let mut part = Some(v);
    while let Some(u) = part {
        let uo = if u == v { o } else { (s.resolve)(u) };
        let current = u32::from(weight_owned(s, u, uo, false));
        total = total.wrapping_add(current);
        let steepness = (s.steepness)(uo.kind);
        // SAFETY: short canonical scalar write, before advancing the live link.
        unsafe {
            (*uo.state).cache.cached_slope_resistance =
                current.wrapping_mul(steepness).wrapping_mul(100);
        }
        part = (u).next();
    }
    unsafe {
        (*o.state).cache.cached_weight = total.max(1);
        (*o.state).cache.cached_axle_resistance = total.wrapping_mul(10) as u16;
    }
    power_changed_owned(s, v, o);
}
fn slope_resistance(s: &Services, v: VehicleRef, first: Owner) -> i64 {
    let mut incline = 0_i64;
    let mut part = Some(v);
    while let Some(u) = part {
        let o = if u == v { first } else { (s.resolve)(u) };
        let flags = unsafe { (*o.state).flags };
        if flags & (1 << GVF_GOINGUP_BIT) != 0 {
            hit(witness::GROUND_UPHILL);
            incline += i64::from(unsafe { (*o.state).cache.cached_slope_resistance });
        } else if flags & (1 << GVF_GOINGDOWN_BIT) != 0 {
            hit(witness::GROUND_DOWNHILL);
            incline -= i64::from(unsafe { (*o.state).cache.cached_slope_resistance });
        }
        part = (u).next();
    }
    incline
}
fn clamp_int(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
pub(crate) fn acceleration(s: &Services, v: VehicleRef) -> i32 {
    acceleration_owned(s, v, (s.resolve)(v))
}
pub(crate) fn acceleration_owned(s: &Services, v: VehicleRef, o: Owner) -> i32 {
    let tile = if o.rail() { v.tile() } else { 0 };
    let current_speed = v.speed();
    let status = v.status();
    let roadtype = if o.rail() {
        RoadType(0)
    } else {
        // SAFETY: The shared resolver reads the live shell's unmoved road type.
        unsafe { ((*((s.vehicle.newgrf)())).roadtype)(v.0.as_ptr()) }
    };
    let speed = i64::from(current_speed_for(o, current_speed));
    let mass = i64::from(unsafe { (*o.state).cache.cached_weight });
    let power = i64::from(unsafe { (*o.state).cache.cached_power }) * 746;
    let maglev = acceleration_type_for(s, o, tile) == ACCELERATION_MAGLEV;
    let area = i64::from(air_drag_area_for(o, status));
    let mut resistance = if maglev {
        0
    } else {
        i64::from(unsafe { (*o.state).cache.cached_axle_resistance })
            + mass
                * i64::from(rolling_friction_for(
                    o,
                    roadtype,
                    current_speed_for(o, current_speed),
                ))
    };
    resistance +=
        area * i64::from(unsafe { (*o.state).cache.cached_air_drag }) * speed * speed / 1000;
    resistance += slope_resistance(s, v, o);
    let mode = acceleration_status_for(o, status);
    hit(if mode == AS_ACCEL {
        witness::GROUND_ACCELERATE
    } else {
        witness::GROUND_BRAKE
    });
    let max_te = unsafe { (*o.state).cache.cached_max_te } as i32;
    let force = if speed > 0 {
        if maglev {
            power / 25
        } else {
            let force = power * 18 / (speed * 5);
            if mode == AS_ACCEL && force > i64::from(max_te) {
                i64::from(max_te)
            } else {
                force
            }
        }
    } else {
        // `std::min<int>` narrows watts to signed int at kickoff in the source.
        let force = if mode == AS_ACCEL && !maglev {
            i64::from(max_te.min(power as i32))
        } else {
            power
        };
        force.max(mass * 8 + resistance)
    };
    if mode == AS_ACCEL {
        if force == resistance {
            return 0;
        }
        let accel = clamp_int((force - resistance) / (mass * 4));
        if force < resistance {
            (-1).min(accel)
        } else {
            1.max(accel)
        }
    } else {
        clamp_int((-force - resistance).min(-10000) / mass)
    }
}
pub(crate) fn chain_in_depot(s: &Services, v: VehicleRef) -> bool {
    let first = v.first();
    let o = (s.resolve)(v);
    if !(s.depot_tile)((first).tile(), o.kind.0) || (first).speed() != 0 {
        return false;
    }
    let mut part = Some(first);
    while let Some(u) = part {
        if !in_depot(if u == v { o } else { (s.resolve)(u) }) || (u).tile() != (v).tile() {
            return false;
        }
        part = (u).next();
    }
    hit(witness::GROUND_DEPOT);
    true
}
pub(crate) fn crash(s: &Services, v: VehicleRef, flooded: bool) -> u32 {
    let mut part = Some(v);
    while let Some(u) = part {
        let o = (s.resolve)(u);
        unsafe {
            (*o.state).flags &= !SLOPE_FLAGS;
        }
        part = (u).next();
    }
    hit(if flooded {
        witness::GROUND_FLOOD
    } else {
        witness::GROUND_CRASH
    });
    // All flags are cleared before base Crash makes its original random draw.
    (s.base_crash)(v, flooded)
}
fn update_z_inclination(movement: &crate::movement::CachedTable<'_>, v: VehicleRef, o: Owner) {
    let x = v.x();
    let y = v.y();
    let z = movement.get().pixel_z_at(x, y, true);
    (v).write_z(z);
    unsafe {
        (*o.state).flags &= !SLOPE_FLAGS;
    }
    if sloped_track_owned(movement, v, o) {
        let x = v.x();
        let y = v.y();
        let middle = movement.get().pixel_z_at(
            (x & !(TILE_UNIT_MASK as i32)) | (TILE_SIZE / 2) as i32,
            (y & !(TILE_UNIT_MASK as i32)) | (TILE_SIZE / 2) as i32,
            true,
        );
        let z = (v).z();
        if middle != z {
            unsafe {
                (*o.state).flags |= 1
                    << if middle > z {
                        GVF_GOINGUP_BIT
                    } else {
                        GVF_GOINGDOWN_BIT
                    };
            }
        }
    }
}
fn update_z(s: &Services, movement: &crate::movement::CachedTable<'_>, v: VehicleRef, o: Owner) {
    if unsafe { (*o.state).flags } & SLOPE_FLAGS != 0 {
        if !o.rail() && reversing(s, v) {
            let x = v.x();
            let y = v.y();
            let z = movement.get().pixel_z_at(x, y, true);
            (v).write_z(z);
            return;
        }
        let direction = (v).direction();
        if s.assertions {
            assert!(direction < crate::direction_type::DIR_END);
        }
        let dir = dir_to_diag_dir_raw(direction, |ok| {
            if s.assertions {
                assert!(ok);
            }
        });
        let x = v.x();
        let y = v.y();
        let mut d = (if diag_dir_to_axis_raw(dir, |ok| {
            if s.assertions {
                assert!(ok);
            }
        }) == 0
        {
            x as i8
        } else {
            y as i8
        }) & 1;
        d ^= i8::from(dir == DIAGDIR_NW || dir == DIAGDIR_NE);
        let z = (v).z();
        let delta = if unsafe { (*o.state).flags } & (1 << GVF_GOINGUP_BIT) != 0 {
            d
        } else {
            -d
        };
        (v).write_z(z.wrapping_add(i32::from(delta)));
    }
    if s.assertions {
        let z = (v).z();
        let x = v.x();
        let y = v.y();
        assert_eq!(z, movement.get().pixel_z_at(x, y, true));
    }
}
pub(crate) fn inclination(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    new_tile: bool,
    delta: bool,
) -> i32 {
    inclination_owned(s, movement, v, (s.resolve)(v), new_tile, delta)
}
pub(crate) fn inclination_owned(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    o: Owner,
    new_tile: bool,
    delta: bool,
) -> i32 {
    let old_z = (v).z();
    if new_tile {
        update_z_inclination(movement, v, o);
    } else {
        update_z(s, movement, v, o);
    }
    (s.viewport)(v, true, delta);
    old_z
}
#[inline]
pub(crate) fn last_speed(s: &Services, v: VehicleRef) {
    last_speed_owned(s, v, (s.resolve)(v));
}
pub(crate) fn last_speed_owned(s: &Services, v: VehicleRef, o: Owner) {
    if (v).speed() != unsafe { (*o.state).cache.last_speed } {
        (s.vehicle.start_stop_dirty)(v);
        unsafe {
            (*o.state).cache.last_speed = (v).speed();
        }
    }
}
pub(crate) struct SpeedResult {
    pub distance: u32,
    pub stored_speed: u16,
}
#[inline]
pub(crate) fn update_speed(
    v: VehicleRef,
    accel: u32,
    min_speed: i32,
    max_speed: i32,
) -> SpeedResult {
    hit(witness::GROUND_SPEED);
    let current_speed = v.speed();
    let subspeed = v.subspeed();
    let mut spd = u32::from(subspeed).wrapping_add(accel);
    (v).write_subspeed(spd as u8);
    let tempmax = if i32::from(current_speed) > max_speed {
        (i32::from(current_speed) - i32::from(current_speed / 10) - 1).max(max_speed)
    } else {
        max_speed
    };
    spd = (i32::from(current_speed)
        .wrapping_add((spd as i32) >> 8)
        .min(tempmax)
        .max(min_speed)) as u32;
    // Scalar stores retain the source speed write before the progress read/reset.
    let stored_speed = spd as u16;
    (v).write_speed(stored_speed);
    let scaled = crate::vehicle::advance_speed(spd) as i32;
    let scaled = scaled.wrapping_add(i32::from((v).progress()));
    (v).write_progress(0);
    SpeedResult {
        distance: scaled as u32,
        stored_speed,
    }
}

// Every entry receives immutable services and a live serialized native shell.
// Panics/allocations abort; callbacks are noexcept and can reenter only after
// the field-sized access above ends. No Rust borrow spans a callback.
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
entry!(openttd_rust_ground_acceleration, acceleration => i32);
/// # Safety
/// Services and shell must remain live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_ground_chain_in_depot_abi(
    s: *const Services,
    v: VehicleRef,
) -> u8 {
    u8::from(chain_in_depot(unsafe { &*s }, v))
}
entry!(openttd_rust_ground_crash, crash, flooded: bool => u32);
entry!(openttd_rust_ground_weight, weight, full: bool => u16);
entry!(openttd_rust_ground_acceleration_type, acceleration_type => u8);

/// The native position mutation ends before the Rust inclination update.
pub(crate) struct Moved {
    pub old_z: i32,
    pub z: i32,
    pub cur_speed: u16,
    pub max_track_speed: u16,
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn move_incline(
    gs: &crate::vehicle::GroundServices,
    movement: &crate::movement::CachedTable<'_>,
    map: crate::map_func::Map,
    v: VehicleRef,
    o: Owner,
    pos: (i32, i32),
    new_tile: bool,
    delta: bool,
) -> Moved {
    // Road supplies dimensions for its synchronous no-reset/no-yield interval.
    let s = gs.physics;
    (v).write_xy(pos.0, pos.1);
    // SAFETY: Live shell; index writes end before inclination callbacks.
    unsafe {
        crate::vehicle::occupancy::update_position(s.vehicle, v, map);
    }
    let old_z = inclination_owned(s, movement, v, o, new_tile, delta);
    Moved {
        old_z,
        z: (v).z(),
        cur_speed: (v).speed(),
        max_track_speed: unsafe { (*o.state).cache.cached_max_track_speed },
    }
}

fn in_depot(o: Owner) -> bool {
    if o.rail() {
        o.train.track() == crate::track_type::TRACK_BIT_DEPOT
    } else {
        crate::road::is_in_depot(o.road)
    }
}
fn reversing(s: &Services, v: VehicleRef) -> bool {
    let first = v.first();
    let o = (s.resolve)(first);
    let direction = (v).direction();
    // SAFETY: the shared slope predicate gets live owners and immutable services.
    unsafe { crate::road::slope_pixel(v, first, o.road, direction) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGroundWeightInput" ["probe"] => crate::ground_vehicle::WeightInput { cargo, owner, };
    "OpenTTDGroundPhysicsServices" ["probe"] => crate::ground_vehicle::Services { vehicle, road, assertions, resolve, weight_input, capabilities, cargo_weight, train_cargo_weight, powered, rail_type, rail_acceleration, rail_speed, road_speed, steepness, display_max_speed, viewport, base_crash, depot_tile, };
    "OpenTTDGroundOwner" ["probe"] => crate::ground_vehicle::Owner { state, train, road, kind, };
}

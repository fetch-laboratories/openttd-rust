/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical ground cache/flags, dynamics and train/road physics callees.
//! Native shells own one stable allocation. Every raw scalar access ends before
//! property, viewport, crash or other potentially reentrant services.
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

/// Canonical `GroundVehicleCache`; native aliases borrow this allocation.
#[repr(C)]
#[derive(Default)]
pub struct Cache {
    pub cached_weight: u32,
    pub cached_slope_resistance: u32,
    pub cached_max_te: u32,
    pub cached_axle_resistance: u16,
    pub cached_max_track_speed: u16,
    pub cached_power: u32,
    pub cached_air_drag: u32,
    pub cached_total_length: u16,
    pub first_engine: u16,
    pub cached_veh_length: u8,
    pub last_speed: u16,
}
/// One owner per native train/road shell, never relocated or mirrored.
#[repr(C)]
#[derive(Default)]
pub struct State {
    pub cache: Cache,
    pub flags: u16,
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

use crate::direction_func::{diag_dir_to_axis, dir_to_diag_dir};
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
pub struct PowerInput {
    pub tile: u32,
    pub subtype: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WeightInput {
    pub cargo: *const std::ffi::c_void,
    pub owner: *mut crate::cargo_storage::List,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AccelerationInput {
    pub tile: u32,
    pub speed: u16,
    pub status: u8,
    pub roadtype: RoadType,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpeedInput {
    pub speed: u16,
    pub subspeed: u8,
}

/// Immutable process-wide typed services. No owner borrow crosses any service;
/// native shells and the canonical allocation are live under the game-state mutex.
#[repr(C)]
pub struct Services {
    pub vehicle: &'static VehicleServices,
    pub road: &'static crate::road::Leaves,
    pub assertions: bool,
    pub resolve: extern "C" fn(VehicleRef) -> Owner,
    pub first: extern "C" fn(VehicleRef) -> VehicleRef,
    pub power_input: extern "C" fn(VehicleRef) -> PowerInput,
    pub weight_input: extern "C" fn(VehicleRef) -> WeightInput,
    pub acceleration_input: extern "C" fn(VehicleRef) -> AccelerationInput,
    pub speed_input: extern "C" fn(VehicleRef) -> SpeedInput,
    pub capacity: extern "C" fn(crate::vehicle::EngineRef, VehicleRef, *mut u16) -> u32,
    pub cargo_weight: extern "C" fn(*const std::ffi::c_void, u32) -> u64,
    pub train_cargo_weight: extern "C" fn(*const std::ffi::c_void, u32) -> u64,
    pub engine_power: extern "C" fn(VehicleRef) -> u16,
    pub engine_weight: extern "C" fn(VehicleRef) -> u16,
    pub engine_te: extern "C" fn(VehicleRef) -> u8,
    pub engine_drag: extern "C" fn(VehicleRef) -> u8,
    pub wagon_power: extern "C" fn(u16) -> u16,
    pub wagon_weight: extern "C" fn(u16) -> u8,
    pub powered: extern "C" fn(u64, RailType) -> bool,
    pub rail_type: extern "C" fn(u32) -> RailType,
    pub rail_acceleration: extern "C" fn(RailType) -> u8,
    pub rail_speed: extern "C" fn(RailType) -> u16,
    pub road_speed: extern "C" fn(VehicleRef) -> u16,
    pub tram: extern "C" fn(RoadType) -> bool,
    pub steepness: extern "C" fn(VehicleType) -> u32,
    pub display_max_speed: extern "C" fn(VehicleRef) -> i32,
    pub viewport: extern "C" fn(VehicleRef, bool, bool),
    pub details_dirty: extern "C" fn(VehicleRef),
    pub base_crash: extern "C" fn(VehicleRef, bool) -> u32,
    pub depot_tile: extern "C" fn(u32, u8) -> bool,
}

impl Owner {
    fn rail(self) -> bool {
        self.kind == VehicleType::TRAIN
    }
    fn powered_wagon(self) -> bool {
        self.rail() && self.train.get_flags() & crate::train::FLAG_POWERED_WAGON != 0
    }
}

fn power_owned(s: &Services, v: VehicleRef, o: Owner) -> u16 {
    let r = (s.power_input)(v);
    if is_articulated_part(true, r.subtype) {
        return 0;
    }
    if o.rail() && !(s.powered)(o.train.get_railtypes(), (s.rail_type)(r.tile)) {
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
        i32::from((s.engine_power)(v)),
    );
    if o.rail() {
        let mut p = p as u16;
        if is_multiheaded((s.vehicle.subtype)(v)) {
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
        if (s.powered)(
            head.train.get_railtypes(),
            (s.rail_type)((s.vehicle.tile)(v)),
        ) {
            // SAFETY: scalar load ends before the specification lookup.
            return (s.wagon_power)(unsafe { (*o.state).cache.first_engine });
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
        (s.capacity)((s.vehicle.engine)(v), v, std::ptr::null_mut())
    } else {
        // SAFETY: live canonical cargo owner; no borrow survives this read.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_stored(r.owner) }
    };
    let mut weight = if o.rail() {
        (s.train_cargo_weight)(r.cargo, count)
    } else {
        (s.cargo_weight)(r.cargo, count)
    } as u16;
    if !is_articulated_part(true, (s.vehicle.subtype)(v)) {
        let w = (s.vehicle.property)(
            (s.vehicle.newgrf)(),
            v,
            if o.rail() {
                PROP_TRAIN_WEIGHT
            } else {
                PROP_ROADVEH_WEIGHT
            },
            i32::from((s.engine_weight)(v)),
        );
        weight = weight.wrapping_add(if o.rail() { w } else { w / 4 } as u16);
    }
    if o.powered_wagon() {
        // SAFETY: no borrow spans the spec leaf.
        weight = weight.wrapping_add(u16::from((s.wagon_weight)(unsafe {
            (*o.state).cache.first_engine
        })));
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
        i32::from((s.engine_te)(v)),
    ) as u8
}
pub(crate) fn air_drag(s: &Services, v: VehicleRef) -> u8 {
    (s.engine_drag)(v)
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
fn rolling_friction_for(s: &Services, o: Owner, roadtype: RoadType, speed: u16) -> u32 {
    if o.rail() {
        15 * (512 + u32::from(speed)) / 512
    } else {
        let coeff = if (s.tram)(roadtype) { 40 } else { 75 };
        coeff * (128 + u32::from(speed)) / 128
    }
}
pub(crate) fn acceleration_type(s: &Services, v: VehicleRef) -> u8 {
    let o = (s.resolve)(v);
    acceleration_type_for(s, o, if o.rail() { (s.vehicle.tile)(v) } else { 0 })
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
        (s.rail_speed)((s.rail_type)((s.vehicle.tile)(v)))
    } else {
        (s.road_speed)(v)
    }
}
fn sloped_track_owned(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    o: Owner,
) -> bool {
    let tracks = if o.rail() {
        o.train.track()
    } else {
        track_status_to_track_bits(movement.get().road_tracks(v, (s.vehicle.tile)(v)))
    };
    tracks == TRACK_BIT_X || tracks == TRACK_BIT_Y
}

pub(crate) fn power_changed(s: &Services, v: VehicleRef) {
    power_changed_owned(s, v, (s.resolve)(v));
}
fn power_changed_owned(s: &Services, v: VehicleRef, o: Owner) {
    if s.assertions {
        assert!((s.first)(v) == v);
    }
    hit(if o.rail() {
        witness::GROUND_RAIL_POWER
    } else {
        witness::GROUND_ROAD_POWER
    });
    let mut total_power = 0_u32;
    let mut max_te = 0_u32;
    let mut parts = 0_u32;
    let mut max_track_speed = (s.vehicle.max_speed)(v);
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
        part = (s.vehicle.next)(u);
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
            let status = (s.vehicle.status)(v);
            (s.vehicle.write_status)(v, status | VS_STOPPED);
        }
        unsafe {
            (*o.state).cache.cached_power = total_power;
            (*o.state).cache.cached_max_te = max_te;
        }
        (s.details_dirty)(v);
        (s.vehicle.start_stop_dirty)(v);
    }
    unsafe {
        (*o.state).cache.cached_max_track_speed = max_track_speed;
    }
}
pub(crate) fn cargo_changed(s: &Services, v: VehicleRef) {
    if s.assertions {
        assert!((s.first)(v) == v);
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
        part = (s.vehicle.next)(u);
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
        part = (s.vehicle.next)(u);
    }
    incline
}
fn clamp_int(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
pub(crate) fn acceleration(s: &Services, v: VehicleRef) -> i32 {
    let o = (s.resolve)(v);
    let r = (s.acceleration_input)(v);
    let speed = i64::from(current_speed_for(o, r.speed));
    let mass = i64::from(unsafe { (*o.state).cache.cached_weight });
    let power = i64::from(unsafe { (*o.state).cache.cached_power }) * 746;
    let maglev = acceleration_type_for(s, o, r.tile) == ACCELERATION_MAGLEV;
    let area = i64::from(air_drag_area_for(o, r.status));
    let mut resistance = if maglev {
        0
    } else {
        i64::from(unsafe { (*o.state).cache.cached_axle_resistance })
            + mass
                * i64::from(rolling_friction_for(
                    s,
                    o,
                    r.roadtype,
                    current_speed_for(o, r.speed),
                ))
    };
    resistance +=
        area * i64::from(unsafe { (*o.state).cache.cached_air_drag }) * speed * speed / 1000;
    resistance += slope_resistance(s, v, o);
    let mode = acceleration_status_for(o, r.status);
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
    let first = (s.first)(v);
    let o = (s.resolve)(v);
    if !(s.depot_tile)((s.vehicle.tile)(first), o.kind.0) || (s.vehicle.speed)(first) != 0 {
        return false;
    }
    let mut part = Some(first);
    while let Some(u) = part {
        if !in_depot(if u == v { o } else { (s.resolve)(u) })
            || (s.vehicle.tile)(u) != (s.vehicle.tile)(v)
        {
            return false;
        }
        part = (s.vehicle.next)(u);
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
        part = (s.vehicle.next)(u);
    }
    hit(if flooded {
        witness::GROUND_FLOOD
    } else {
        witness::GROUND_CRASH
    });
    // All flags are cleared before base Crash makes its original random draw.
    (s.base_crash)(v, flooded)
}
fn update_z_inclination(
    s: &Services,
    movement: &crate::movement::CachedTable<'_>,
    v: VehicleRef,
    o: Owner,
) {
    let xy = (s.vehicle.xy)(v);
    let z = movement.get().pixel_z_at(xy.x, xy.y, true);
    (s.vehicle.write_z)(v, z);
    unsafe {
        (*o.state).flags &= !SLOPE_FLAGS;
    }
    if sloped_track_owned(s, movement, v, o) {
        let xy = (s.vehicle.xy)(v);
        let middle = movement.get().pixel_z_at(
            (xy.x & !(TILE_UNIT_MASK as i32)) | (TILE_SIZE / 2) as i32,
            (xy.y & !(TILE_UNIT_MASK as i32)) | (TILE_SIZE / 2) as i32,
            true,
        );
        let z = (s.vehicle.z)(v);
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
            let xy = (s.vehicle.xy)(v);
            let z = movement.get().pixel_z_at(xy.x, xy.y, true);
            (s.vehicle.write_z)(v, z);
            return;
        }
        let direction = (s.vehicle.direction)(v);
        if s.assertions {
            assert!(direction < crate::direction_type::DIR_END);
        }
        let dir = dir_to_diag_dir(direction, |ok| {
            if s.assertions {
                assert!(ok);
            }
        });
        let xy = (s.vehicle.xy)(v);
        let mut d = (if diag_dir_to_axis(dir, |ok| {
            if s.assertions {
                assert!(ok);
            }
        }) == 0
        {
            xy.x as i8
        } else {
            xy.y as i8
        }) & 1;
        d ^= i8::from(dir == DIAGDIR_NW || dir == DIAGDIR_NE);
        let z = (s.vehicle.z)(v);
        let delta = if unsafe { (*o.state).flags } & (1 << GVF_GOINGUP_BIT) != 0 {
            d
        } else {
            -d
        };
        (s.vehicle.write_z)(v, z.wrapping_add(i32::from(delta)));
    }
    if s.assertions {
        let z = (s.vehicle.z)(v);
        let xy = (s.vehicle.xy)(v);
        assert_eq!(z, movement.get().pixel_z_at(xy.x, xy.y, true));
    }
}
pub(crate) fn inclination(s: &Services, v: VehicleRef, new_tile: bool, delta: bool) -> i32 {
    inclination_owned(s, v, (s.resolve)(v), new_tile, delta)
}
fn inclination_owned(s: &Services, v: VehicleRef, o: Owner, new_tile: bool, delta: bool) -> i32 {
    // SAFETY: Only the immutable registry is retained for this entry.
    let movement = crate::movement::CachedTable::new(unsafe { &*s.road.shared });
    let old_z = (s.vehicle.z)(v);
    if new_tile {
        update_z_inclination(s, &movement, v, o);
    } else {
        update_z(s, &movement, v, o);
    }
    (s.viewport)(v, true, delta);
    old_z
}
#[inline]
pub(crate) fn last_speed(s: &Services, v: VehicleRef) {
    let o = (s.resolve)(v);
    if (s.vehicle.speed)(v) != unsafe { (*o.state).cache.last_speed } {
        (s.vehicle.start_stop_dirty)(v);
        unsafe {
            (*o.state).cache.last_speed = (s.vehicle.speed)(v);
        }
    }
}
pub(crate) struct SpeedResult {
    pub distance: u32,
    pub stored_speed: u16,
}
#[inline]
pub(crate) fn update_speed(
    s: &Services,
    v: VehicleRef,
    accel: u32,
    min_speed: i32,
    max_speed: i32,
) -> SpeedResult {
    hit(witness::GROUND_SPEED);
    let r = (s.speed_input)(v);
    let mut spd = u32::from(r.subspeed).wrapping_add(accel);
    (s.vehicle.write_subspeed)(v, spd as u8);
    let tempmax = if i32::from(r.speed) > max_speed {
        (i32::from(r.speed) - i32::from(r.speed / 10) - 1).max(max_speed)
    } else {
        max_speed
    };
    spd = (i32::from(r.speed)
        .wrapping_add((spd as i32) >> 8)
        .min(tempmax)
        .max(min_speed)) as u32;
    // Native speed/subspeed stores and progress read/reset are scalar leaves;
    // none reenters or changes cur_speed after this store (vehicle_services.cpp).
    let stored_speed = spd as u16;
    (s.vehicle.write_speed)(v, stored_speed);
    let scaled = crate::vehicle::advance_speed(spd) as i32;
    let scaled = scaled.wrapping_add(i32::from((s.vehicle.progress)(v)));
    (s.vehicle.write_progress)(v, 0);
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
entry!(openttd_rust_ground_chain_in_depot, chain_in_depot => bool);
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
pub(crate) fn move_incline(
    gs: &crate::vehicle::GroundServices,
    map: crate::map_func::Map,
    v: VehicleRef,
    x: i32,
    y: i32,
    new_tile: bool,
    delta: bool,
) -> Moved {
    // Road supplies its synchronous no-reset/no-yield entry dimensions; write_xy
    // is a scalar leaf. They are consumed before any inclination/viewport call.
    let s = gs.physics;
    (s.vehicle.write_xy)(v, x, y);
    // SAFETY: Live shell; index writes end before inclination callbacks.
    unsafe {
        crate::vehicle::occupancy::update_position(s.vehicle, v, map);
    }
    let o = (s.resolve)(v);
    let old_z = inclination_owned(s, v, o, new_tile, delta);
    Moved {
        old_z,
        z: (s.vehicle.z)(v),
        cur_speed: (s.vehicle.speed)(v),
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
    let first = (s.first)(v);
    let o = (s.resolve)(first);
    let direction = (s.vehicle.direction)(v);
    // SAFETY: the shared slope predicate gets live owners and immutable services.
    unsafe { crate::road::openttd_rust_road_slope_pixel(v, first, o.road, direction, s.road) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGroundAccelerationInput" ["probe"] => crate::ground_vehicle::AccelerationInput { tile, speed, status, roadtype, };
    "GroundVehicleCache" ["probe"] => crate::ground_vehicle::Cache {
        cached_weight, cached_slope_resistance, cached_max_te, cached_axle_resistance,
        cached_max_track_speed, cached_power, cached_air_drag, cached_total_length,
        first_engine, cached_veh_length, last_speed,
    };
    "OpenTTDGroundWeightInput" ["probe"] => crate::ground_vehicle::WeightInput { cargo, owner, };
    "OpenTTDGroundSpeedInput" ["probe"] => crate::ground_vehicle::SpeedInput { speed, subspeed, };
    "OpenTTDGroundPhysicsServices" ["probe"] => crate::ground_vehicle::Services { vehicle, road, assertions, resolve, first, power_input, weight_input, acceleration_input, speed_input, capacity, cargo_weight, train_cargo_weight, engine_power, engine_weight, engine_te, engine_drag, wagon_power, wagon_weight, powered, rail_type, rail_acceleration, rail_speed, road_speed, tram, steepness, display_max_speed, viewport, details_dirty, base_crash, depot_tile, };
    "OpenTTDGroundState" ["probe"] => crate::ground_vehicle::State { cache, flags, };
    "OpenTTDGroundOwner" ["probe"] => crate::ground_vehicle::Owner { state, train, road, kind, };
    "OpenTTDGroundPowerInput" ["probe"] => crate::ground_vehicle::PowerInput { tile, subtype, };
}

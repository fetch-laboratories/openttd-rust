/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Vehicle invocation state and the single canonical per-shell `NewGRF` cache.
//! Shells and generic Action2 evaluation remain native. No cache or invocation
//! borrow spans evaluator reentry. All calls are synchronous; panic aborts.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::struct_field_names,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]
use crate::cargo_type::{INVALID_CARGO, NUM_CARGO};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_VEHICLE_BUILD_PROBABILITY, CBID_VEHICLE_MODIFY_PROPERTY,
};
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, VehicleRef, VehicleServices};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
type V = *mut OpenTTDVehicle;
type Group = *const c_void;
#[repr(C)]
#[derive(Default)]
pub struct Cache {
    position_consist_length: u32,
    position_same_id_length: u32,
    consist_cargo_information: u32,
    company_information: u32,
    position_in_vehicle: u32,
    cache_valid: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Scope {
    v: V,
    self_type: u16,
    rotor_in_gui: bool,
}
#[repr(C)]
pub struct State {
    self_scope: Scope,
    parent: Scope,
    relative: Scope,
    cached_relative_count: u8,
    root: Group,
}
#[repr(C)]
pub struct RandomFields {
    bits: u16,
    waiting: u8,
}
#[repr(C)]
pub struct RandomResult {
    used: u32,
    reseed: u32,
}
#[repr(C)]
pub struct CargoQuery {
    cargo: u8,
    subtype: u8,
    engine: *const OpenTTDEngine,
}
#[repr(C)]
pub struct Override {
    group: Group,
    cargo: u8,
    count: usize,
}
#[repr(C)]
pub struct Overrides {
    engine: *const OpenTTDEngine,
    count: usize,
}
#[repr(C)]
pub struct Services {
    vehicle: &'static VehicleServices,
    shared: &'static crate::services::Services,
    cache: unsafe extern "C" fn(V) -> *mut Cache,
    first: unsafe extern "C" fn(V) -> V,
    move_vehicle: unsafe extern "C" fn(V, i32) -> V,
    previous: unsafe extern "C" fn(V) -> V,
    cargo: unsafe extern "C" fn(V) -> CargoQuery,
    can_carry: unsafe extern "C" fn(*const OpenTTDEngine) -> bool,
    cargo_classes: unsafe extern "C" fn(u8) -> u16,
    cargo_bitnum: unsafe extern "C" fn(u8) -> u8,
    cargo_translation: unsafe extern "C" fn(*const c_void, u8) -> u8,
    grf_version: unsafe extern "C" fn(*const c_void) -> u8,
    get_register: unsafe extern "C" fn(*const c_void, u16) -> i32,
    current_company: unsafe extern "C" fn() -> u8,
    valid_company: unsafe extern "C" fn(u8) -> bool,
    livery: unsafe extern "C" fn(u16, u8, u16, V) -> *const c_void,
    company_info: unsafe extern "C" fn(u8, *const c_void) -> u32,
    first_engine: unsafe extern "C" fn(V) -> u16,
    articulated: unsafe extern "C" fn(V) -> bool,
    has_articulated: unsafe extern "C" fn(V) -> bool,
    overrides: unsafe extern "C" fn(u16) -> Overrides,
    override_at: unsafe extern "C" fn(*const OpenTTDEngine, usize) -> Override,
    override_engine: unsafe extern "C" fn(*const OpenTTDEngine, usize, usize) -> u16,
    cached_override: unsafe extern "C" fn(V) -> Group,
    spritegroup: unsafe extern "C" fn(u16, u8) -> Group,
    random_fields: unsafe extern "C" fn(V) -> RandomFields,
    write_random: unsafe extern "C" fn(V, u16),
    write_waiting: unsafe extern "C" fn(V, u8),
    resolve_callback: unsafe extern "C" fn(*mut State, u16, u16, u32, u32, *mut i32, usize) -> u16,
    resolve_random: unsafe extern "C" fn(*mut State, u16, u8) -> RandomResult,
    in_motion: unsafe extern "C" fn(V) -> bool,
    cargo_owner: unsafe extern "C" fn(V) -> *const crate::cargo_storage::List,
    cargo_capacity: unsafe extern "C" fn(V) -> u16,
    real_count: unsafe extern "C" fn(Group, bool) -> u32,
    real_at: unsafe extern "C" fn(Group, bool, u32) -> Group,
    check: unsafe extern "C" fn(bool),
}
fn handle(v: V) -> VehicleRef {
    VehicleRef(std::ptr::NonNull::new(v).unwrap())
}
fn next(s: &Services, v: V) -> V {
    (s.vehicle.next)(handle(v)).map_or(null_mut(), |v| v.0.as_ptr())
}
fn engine(s: &Services, v: V) -> u16 {
    (s.vehicle.engine_id)(handle(v))
}
fn ground(s: &Services, v: V) -> bool {
    crate::vehicle_base::is_ground_vehicle((s.vehicle.vehicle_type)(handle(v)))
}
// SAFETY throughout: the game-state mutex serializes shell/cache access. The
// caller retains live shells and immutable services. Raw cache reads/writes are
// short operations, ending before any native callback or evaluator entry.
unsafe fn livery(s: &Services, id: u16, v: V) -> *const c_void {
    unsafe {
        if v.is_null() {
            let company = (s.current_company)();
            if !(s.valid_company)(company) {
                return null();
            }
            (s.livery)(id, company, crate::vehicle::INVALID_ENGINE, v)
        } else {
            let first_engine = if ground(s, v) {
                (s.first_engine)(v)
            } else {
                crate::vehicle::INVALID_ENGINE
            };
            (s.livery)(engine(s, v), (s.vehicle.owner)(handle(v)), first_engine, v)
        }
    }
}
unsafe fn position(s: &Services, v: V, consecutive: bool) -> u32 {
    unsafe {
        let mut before = 0u8;
        let mut after = 0u8;
        let mut u = (s.first)(v);
        while u != v {
            before = before.wrapping_add(1);
            if consecutive && engine(s, u) != engine(s, v) {
                before = 0;
            }
            u = next(s, u);
        }
        while !next(s, u).is_null() && (!consecutive || engine(s, next(s, u)) == engine(s, v)) {
            after = after.wrapping_add(1);
            u = next(s, u);
        }
        u32::from(before)
            | u32::from(after) << 8
            | (u32::from(before) + u32::from(after) + u32::from(consecutive)) << 16
    }
}
fn first_max(values: &[u8]) -> u8 {
    let mut best = 0;
    for i in 1..values.len() {
        if values[i] > values[best] {
            best = i;
        }
    }
    if values[best] == 0 {
        u8::MAX
    } else {
        best as u8
    }
}
unsafe fn cached_variable(s: &Services, v: V, variable: u8, grf: *const c_void) -> u32 {
    unsafe {
        let (bit, field) = match variable {
            0x40 => (0, 0),
            0x41 => (1, 1),
            0x42 => (2, 2),
            0x43 => (3, 3),
            0x4d => (4, 4),
            _ => unreachable!(),
        };
        let cache = (s.cache)(v);
        if (*cache).cache_valid & (1 << bit) == 0 {
            let value = match variable {
                0x40 | 0x41 => position(s, v, variable == 0x41),
                0x42 => {
                    let mut cargoes = [0u8; NUM_CARGO as usize];
                    let mut classes = 0u8;
                    let mut user_data = 0u8;
                    let mut u = v;
                    while !u.is_null() {
                        if (s.vehicle.vehicle_type)(handle(v)) == 0 {
                            user_data |= (s.vehicle.train_handle)(u).get_user_def_data() as u8;
                        }
                        let c = (s.cargo)(u);
                        if (s.can_carry)(c.engine) {
                            classes |= (s.cargo_classes)(c.cargo) as u8;
                            cargoes[c.cargo as usize] = cargoes[c.cargo as usize].wrapping_add(1);
                        }
                        u = next(s, u);
                    }
                    let cargo = first_max(&cargoes);
                    let mut subtypes = [0u8; 256];
                    u = v;
                    while !u.is_null() {
                        let c = (s.cargo)(u);
                        if c.cargo == cargo && (s.can_carry)(c.engine) {
                            subtypes[c.subtype as usize] =
                                subtypes[c.subtype as usize].wrapping_add(1);
                        }
                        u = next(s, u);
                    }
                    u32::from(classes)
                        | u32::from(cargo) << 8
                        | u32::from(first_max(&subtypes)) << 16
                        | u32::from(user_data) << 24
                }
                0x43 => (s.company_info)((s.vehicle.owner)(handle(v)), livery(s, engine(s, v), v)),
                0x4d => {
                    let mut before = 0u8;
                    let mut after = 0u8;
                    let mut u = v;
                    while (s.articulated)(u) {
                        before = before.wrapping_add(1);
                        u = (s.previous)(u);
                    }
                    u = v;
                    while (s.has_articulated)(u) {
                        after = after.wrapping_add(1);
                        u = next(s, u);
                    }
                    u32::from(before) | u32::from(after) << 8
                }
                _ => unreachable!(),
            };
            // Each field is accessed separately; cache is never borrowed across a service.
            match field {
                0 => (*cache).position_consist_length = value,
                1 => (*cache).position_same_id_length = value,
                2 => (*cache).consist_cargo_information = value,
                3 => (*cache).company_information = value,
                _ => (*cache).position_in_vehicle = value,
            }
            (*cache).cache_valid |= 1 << bit;
        }
        let value = match field {
            0 => (*cache).position_consist_length,
            1 => (*cache).position_same_id_length,
            2 => (*cache).consist_cargo_information,
            3 => (*cache).company_information,
            _ => (*cache).position_in_vehicle,
        };
        if variable != 0x42 {
            return value;
        }
        let cargo = (value >> 8) as u8;
        let bitnum = if cargo == INVALID_CARGO {
            255
        } else if grf.is_null() || (s.grf_version)(grf) < 8 {
            (s.cargo_bitnum)(cargo)
        } else {
            (s.cargo_translation)(grf, cargo)
        };
        (value & 0xffff_00ff) | u32::from(bitnum) << 8
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_vehicle_grf_cache_new() -> *mut Cache {
    Box::into_raw(Box::default())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_grf_cache_delete(cache: *mut Cache) {
    unsafe {
        drop(Box::from_raw(cache));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_grf_invalidate(s: &Services, mut v: V, chain: bool) {
    unsafe {
        loop {
            (*(s.cache)(v)).cache_valid = 0;
            if !chain {
                break;
            }
            v = next(s, v);
            if v.is_null() {
                break;
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_cached_variable(
    s: &Services,
    v: V,
    variable: u8,
    grf: *const c_void,
) -> u32 {
    unsafe { cached_variable(s, v, variable, grf) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_grf_fill(s: &Services, v: V) {
    unsafe {
        for (bit, variable) in [0x40, 0x41, 0x42, 0x43, 0x4d].into_iter().enumerate() {
            if (*(s.cache)(v)).cache_valid & (1 << bit) == 0 {
                cached_variable(s, v, variable, null());
            }
        }
        (s.check)((*(s.cache)(v)).cache_valid == 31);
    }
}
unsafe fn wagon_override(s: &Services, id: u16, cargo: u8, overriding: u16) -> Group {
    unsafe {
        let entries = (s.overrides)(id);
        for i in 0..entries.count {
            let entry = (s.override_at)(entries.engine, i);
            if entry.cargo != cargo && entry.cargo != crate::newgrf_commons::SG_DEFAULT {
                continue;
            }
            for j in 0..entry.count {
                if (s.override_engine)(entries.engine, i, j) == overriding {
                    return entry.group;
                }
            }
        }
        null()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_wagon_override(
    s: &Services,
    id: u16,
    cargo: u8,
    overriding: u16,
) -> Group {
    unsafe { wagon_override(s, id, cargo, overriding) }
}
unsafe fn init(s: &Services, id: u16, v: V, wagon: u8, rotor: bool) -> State {
    unsafe {
        let scope = Scope {
            v,
            self_type: id,
            rotor_in_gui: rotor,
        };
        let mut state = State {
            self_scope: scope,
            parent: Scope {
                v: if v.is_null() { v } else { (s.first)(v) },
                ..scope
            },
            relative: scope,
            cached_relative_count: 0,
            root: null(),
        };
        if wagon == 3 {
            state.root = wagon_override(s, id, crate::newgrf_commons::SG_DEFAULT, id);
        } else {
            if wagon != 0 && !v.is_null() && ground(s, v) {
                (s.check)(engine(s, v) == id);
                state.root = if wagon == 2 && (s.vehicle.vehicle_type)(handle(v)) == 0 {
                    (s.cached_override)(v)
                } else {
                    wagon_override(s, engine(s, v), (s.cargo)(v).cargo, (s.first_engine)(v))
                };
            }
            if state.root.is_null() {
                let cargo = if v.is_null() {
                    crate::newgrf_commons::SG_PURCHASE
                } else {
                    (s.cargo)(v).cargo
                };
                state.root = crate::newgrf_commons::first_sprite_group_of(
                    [cargo, crate::newgrf_commons::SG_DEFAULT],
                    |cargo| (s.spritegroup)(id, cargo),
                );
            }
        }
        state
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_resolver_init(
    s: &Services,
    state: *mut State,
    id: u16,
    v: V,
    wagon: u8,
    rotor: bool,
) {
    unsafe {
        state.write(init(s, id, v, wagon, rotor));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_scope_set(scope: *mut Scope, v: V) {
    unsafe {
        (*scope).v = v;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_scope_random(
    s: &Services,
    scope: *const Scope,
    triggers: bool,
) -> u32 {
    unsafe {
        let v = (*scope).v;
        if v.is_null() {
            return 0;
        }
        let f = (s.random_fields)(v);
        if triggers {
            u32::from(f.waiting)
        } else {
            u32::from(f.bits)
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_resolver_scope(
    s: &Services,
    state: *mut State,
    ro: *const c_void,
    scope: u8,
    relative: u8,
) -> u8 {
    unsafe {
        if scope != 2 {
            return scope;
        }
        let self_v = (*state).self_scope.v;
        let mut count = i32::from(relative & 15);
        if !self_v.is_null() && (relative != (*state).cached_relative_count || count == 0) {
            if count == 0 {
                count = (s.get_register)(ro, 0x100);
            }
            let v = match relative >> 6 {
                0 => self_v,
                1 => {
                    count = count.wrapping_neg();
                    self_v
                }
                2 => (*state).parent.v,
                _ => {
                    let mut first: V = null_mut();
                    let mut u = (s.first)(self_v);
                    while u != self_v {
                        if engine(s, u) != engine(s, self_v) {
                            first = null_mut();
                        } else if first.is_null() {
                            first = u;
                        }
                        u = next(s, u);
                    }
                    if first.is_null() { self_v } else { first }
                }
            };
            // Original initializes cached_relative_count but never updates it.
            (*state).relative.v = (s.move_vehicle)(v, count);
        }
        scope
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_resolve_real(
    s: &Services,
    state: *const State,
    group: Group,
) -> Group {
    unsafe {
        let v = (*state).self_scope.v;
        if v.is_null() {
            if (s.real_count)(group, false) != 0 {
                return (s.real_at)(group, false, 0);
            }
            if (s.real_count)(group, true) != 0 {
                return (s.real_at)(group, true, 0);
            }
            return null();
        }
        let motion = (s.in_motion)(v);
        let total = (s.real_count)(group, motion);
        if total == 0 {
            return null();
        }
        let cargo = crate::cargo_storage::openttd_rust_cargo_list_stored((s.cargo_owner)(v));
        let set = cargo.wrapping_mul(total) / u32::from((s.cargo_capacity)(v).max(1));
        (s.real_at)(group, motion, set.min(total - 1))
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_callback(
    s: &Services,
    callback: u16,
    p1: u32,
    p2: u32,
    id: u16,
    v: V,
    parent: V,
    override_parent: bool,
    regs: *mut i32,
    count: usize,
) -> u16 {
    unsafe {
        let mut state = init(s, id, v, u8::from(!override_parent), false);
        if override_parent {
            state.parent.v = parent;
        }
        // No Rust reference into state is retained while generic Action2 reenters.
        (s.resolve_callback)(&raw mut state, id, callback, p1, p2, regs, count)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_property(
    s: &Services,
    id: u16,
    property: u8,
    fallback: i32,
    v: V,
    signed: bool,
) -> i32 {
    unsafe {
        let result = openttd_rust_vehicle_callback(
            s,
            CBID_VEHICLE_MODIFY_PROPERTY.0,
            u32::from(property),
            0,
            id,
            v,
            null_mut(),
            false,
            null_mut(),
            0,
        );
        if result == CALLBACK_FAILED {
            fallback
        } else if signed {
            i32::from((result << 1) as i16) / 2
        } else {
            i32::from(result)
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_build_probability(
    s: &Services,
    v: V,
    id: u16,
    kind: u8,
) -> i8 {
    unsafe {
        let result = openttd_rust_vehicle_callback(
            s,
            CBID_VEHICLE_BUILD_PROBABILITY.0,
            u32::from(kind),
            0,
            id,
            v,
            null_mut(),
            false,
            null_mut(),
            0,
        );
        if result == CALLBACK_FAILED {
            -1
        } else {
            i8::from(u32::from(result) + s.shared.random_range(100) >= 100)
        }
    }
}
unsafe fn randomise(s: &Services, v: V, trigger: u8, base: u16, first: bool) {
    unsafe {
        (s.check)(!v.is_null());
        let mut state = init(s, engine(s, v), v, 2, false);
        let waiting = (s.random_fields)(v).waiting | (1 << trigger);
        (s.write_waiting)(v, waiting);
        let result = (s.resolve_random)(&raw mut state, engine(s, v), waiting);
        (s.write_waiting)(v, (s.random_fields)(v).waiting & !(result.used as u8));
        let random = s.shared.random() as u16; // Draw even if reseed is zero.
        let bits = (s.random_fields)(v).bits;
        (s.write_random)(
            v,
            (bits & !(result.reseed as u16))
                | ((if first { random } else { base }) & result.reseed as u16),
        );
        match trigger {
            0 => {
                (s.check)(first);
                randomise(s, (s.first)(v), 3, random, false);
            }
            1 => {
                let u = next(s, v);
                if !u.is_null() {
                    randomise(s, u, trigger, 0, true);
                }
            }
            2 => {
                let u = next(s, v);
                if !u.is_null() {
                    randomise(s, u, trigger, if first { random } else { base }, false);
                }
            }
            3 => {
                (s.check)(!first);
                let u = next(s, v);
                if !u.is_null() {
                    randomise(s, u, trigger, base, false);
                }
            }
            _ => {}
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_randomise(s: &Services, v: V, trigger: u8) {
    unsafe {
        openttd_rust_vehicle_grf_invalidate(s, v, true);
        randomise(s, v, trigger, 0, true);
        openttd_rust_vehicle_grf_invalidate(s, v, true);
    }
}
// Native getter resolves the immutable table once; the property/callback table
// entries point directly to these Rust functions, avoiding an owner bounce.
unsafe extern "C" {
    fn openttd_vehicle_resolver_services() -> *const Services;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_shell_property(
    v: V,
    property: u8,
    fallback: i32,
) -> i32 {
    unsafe {
        let s = &*openttd_vehicle_resolver_services();
        openttd_rust_vehicle_property(s, engine(s, v), property, fallback, v, false)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_shell_callback(
    v: V,
    callback: crate::newgrf_callbacks::CallbackID,
) -> u16 {
    unsafe {
        let s = &*openttd_vehicle_resolver_services();
        openttd_rust_vehicle_callback(
            s,
            callback.0,
            0,
            0,
            engine(s, v),
            v,
            null_mut(),
            false,
            null_mut(),
            0,
        )
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleRandomFields" ["vehicle_resolver"] => RandomFields { bits, waiting };
    "OpenTTDVehicleRandomResult" ["vehicle_resolver"] => RandomResult { used, reseed };
    "OpenTTDVehicleCargoQuery" ["vehicle_resolver"] => CargoQuery { cargo, subtype, engine };
    "OpenTTDVehicleOverride" ["vehicle_resolver"] => Override { group, cargo, count };
    "OpenTTDVehicleOverrides" ["vehicle_resolver"] => Overrides { engine, count };
    "OpenTTDVehicleResolverServices" ["vehicle_resolver"] => Services {
        vehicle, shared, cache, first, move_vehicle => move, previous, cargo, can_carry,
        cargo_classes, cargo_bitnum, cargo_translation, grf_version, get_register, current_company,
        valid_company, livery, company_info, first_engine, articulated, has_articulated,
        overrides, override_at, override_engine, cached_override, spritegroup, random_fields,
        write_random, write_waiting, resolve_callback, resolve_random, in_motion, cargo_owner,
        cargo_capacity, real_count, real_at, check
    };
    "NewGRFCache" ["vehicle_resolver"] => Cache { position_consist_length, position_same_id_length, consist_cargo_information, company_information, position_in_vehicle, cache_valid };
    "OpenTTDVehicleScopeState" ["vehicle_resolver"] => Scope { v, self_type, rotor_in_gui };
    "OpenTTDVehicleResolverState" ["vehicle_resolver"] => State { self_scope => self_, parent, relative, cached_relative_count, root };
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Vehicle invocation state and the per-shell `NewGRF` cache. Shells and evaluator stack
//! facades stay native; common Action2 evaluation is in `newgrf_spritegroup.rs`.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::struct_field_names,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::bool_to_int_with_if, // Preserve native movement state branches.
    clippy::invalid_upcast_comparisons, // Signed byte narrowing is intentional.
    clippy::too_many_arguments
)]
use crate::cargo_type::{INVALID_CARGO, NUM_CARGO};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_VEHICLE_BUILD_PROBABILITY, CBID_VEHICLE_MODIFY_PROPERTY,
};
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, VehicleRef, VehicleServices};
use crate::vehicle_base::{
    NCVV_COMPANY_INFORMATION, NCVV_CONSIST_CARGO_INFORMATION, NCVV_END,
    NCVV_POSITION_CONSIST_LENGTH, NCVV_POSITION_IN_VEHICLE, NCVV_POSITION_SAME_ID_LENGTH,
};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
type V = *mut OpenTTDVehicle;
type Group = *const c_void;
/// `BuildProbabilityType::Reversed`.
pub const BUILD_PROBABILITY_REVERSED: u8 = 0;
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
impl Scope {
    /// `VehicleScopeResolver::SetVehicle`.
    fn set_vehicle(&mut self, v: V) {
        self.v = v;
    }
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
pub struct OrderState {
    unload: u8,
    load: u8,
    loading: bool,
}
#[repr(C)]
pub struct RandomResult {
    used: u32,
    reseed: u32,
}
#[repr(C)]
pub struct CargoInfo {
    classes: u16,
    weight: u8,
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
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    cargo_classes: unsafe extern "C" fn(u8) -> u16,
    cargo_bitnum: unsafe extern "C" fn(u8) -> u8,
    cargo_translation: unsafe extern "C" fn(*const c_void, u8) -> u8,
    grf_version: unsafe extern "C" fn(*const c_void) -> u8,
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
    write_override: unsafe extern "C" fn(V, Group),
    spritegroup: unsafe extern "C" fn(u16, u8) -> Group,
    random_bits: unsafe extern "C" fn(V) -> u16,
    waiting: unsafe extern "C" fn(V) -> u8,
    write_random: unsafe extern "C" fn(V, u16),
    write_waiting: unsafe extern "C" fn(V, u8),
    resolve_callback: unsafe extern "C" fn(*mut State, u16, u16, u32, u32, *mut i32, usize) -> u16,
    resolve_random: unsafe extern "C" fn(*mut State, u16, u8) -> RandomResult,
    order: unsafe extern "C" fn(V) -> OrderState,
    cargo_owner: unsafe extern "C" fn(V) -> *const crate::cargo_storage::List,
    cargo_capacity: unsafe extern "C" fn(V) -> u16,
    real_count: unsafe extern "C" fn(Group, bool) -> u32,
    real_at: unsafe extern "C" fn(Group, bool, u32) -> Group,
    vehicle_grfid: unsafe extern "C" fn(V) -> u32,
    engine_by_id: unsafe extern "C" fn(u16) -> *const OpenTTDEngine,
    default_cargo: unsafe extern "C" fn(*const OpenTTDEngine) -> u8,
    engine_flags: unsafe extern "C" fn(*const OpenTTDEngine) -> u8,
    local_id: unsafe extern "C" fn(*const OpenTTDEngine) -> u16,
    cargo_info: unsafe extern "C" fn(u8) -> CargoInfo,
    motion: unsafe extern "C" fn(V) -> u32,
    build_year: unsafe extern "C" fn(V) -> i32,
    last_service: unsafe extern "C" fn(V) -> i32,
    primary: unsafe extern "C" fn(V) -> bool,
    train_services: unsafe extern "C" fn() -> *const crate::train::Leaves,
    road_acceleration: unsafe extern "C" fn() -> u8,
    order_max_speed: unsafe extern "C" fn(V) -> u16,
    id: unsafe extern "C" fn(V) -> u32,
    old_order: unsafe extern "C" fn(V) -> u16,
    num_orders: unsafe extern "C" fn(V) -> u8,
    real_order: unsafe extern "C" fn(V) -> u8,
    load_ticks: unsafe extern "C" fn(V) -> u16,
    service_interval: unsafe extern "C" fn(V) -> u16,
    acceleration: unsafe extern "C" fn(V) -> u8,
    cargo_type: unsafe extern "C" fn(V) -> u8,
    cargo_subtype: unsafe extern "C" fn(V) -> u8,
    age: unsafe extern "C" fn(V) -> i32,
    max_age: unsafe extern "C" fn(V) -> i32,
    unit: unsafe extern "C" fn(V) -> u16,
    sprite: unsafe extern "C" fn(V) -> u8,
    breakdowns: unsafe extern "C" fn(V) -> u8,
    breakdown_counter: unsafe extern "C" fn(V) -> u8,
    breakdown_delay: unsafe extern "C" fn(V) -> u8,
    breakdown_chance: unsafe extern "C" fn(V) -> u8,
    reliability: unsafe extern "C" fn(V) -> u16,
    reliability_decay: unsafe extern "C" fn(V) -> u16,
    value: unsafe extern "C" fn(V) -> i64,
    profit_this: unsafe extern "C" fn(V) -> i64,
    profit_last: unsafe extern "C" fn(V) -> i64,
    vehicle_flags: unsafe extern "C" fn(V) -> u16,
    aircraft: unsafe extern "C" fn(V) -> *mut crate::aircraft::State,
    target_airport: unsafe extern "C" fn(V) -> *const crate::station::Station,
    airport_ttd_type: unsafe extern "C" fn(
        *const crate::station_cmd::AirportSpec,
    ) -> crate::newgrf_airport::TtdpAirportType,
    moving_flags: unsafe extern "C" fn(*const c_void, u8) -> u16,
    roadtype: unsafe extern "C" fn(V) -> crate::services::RoadType,
    tile_roadtype:
        unsafe extern "C" fn(u32, crate::services::RoadTramType) -> crate::services::RoadType,
    tile_railtype: unsafe extern "C" fn(u32) -> crate::services::RailType,
    rail_catenary: unsafe extern "C" fn(crate::services::RailType) -> bool,
    road_catenary: unsafe extern "C" fn(crate::services::RoadType) -> bool,
    reverse_rail: unsafe extern "C" fn(crate::services::RailType, *const c_void) -> u8,
    reverse_road: unsafe extern "C" fn(crate::services::RoadType, *const c_void) -> u8,
    translate_rail: unsafe extern "C" fn(u8, *const c_void) -> crate::services::RailType,
    translate_road: unsafe extern "C" fn(
        crate::services::RoadTramType,
        u8,
        *const c_void,
    ) -> crate::services::RoadType,
    compatible_rail:
        unsafe extern "C" fn(crate::services::RailType, crate::services::RailType) -> bool,
    powered_rail: unsafe extern "C" fn(u64, crate::services::RailType) -> bool,
    powered_single_rail:
        unsafe extern "C" fn(crate::services::RailType, crate::services::RailType) -> bool,
    badge_count: unsafe extern "C" fn(*const c_void) -> usize,
    badge_at: unsafe extern "C" fn(*const c_void, u32) -> u32,
    engine_has_badge: unsafe extern "C" fn(*const OpenTTDEngine, u32) -> bool,
    engine_badge: unsafe extern "C" fn(*const c_void, *const OpenTTDEngine, u32) -> u32,
    rail_badge: unsafe extern "C" fn(*const c_void, crate::services::RailType, u32) -> u32,
    road_badge: unsafe extern "C" fn(*const c_void, crate::services::RoadType, u32) -> u32,
    ship_owner: unsafe extern "C" fn(V) -> *mut crate::ship_control::State,
    unhandled: unsafe extern "C" fn(u8, u8) -> (),
    aircraft_leaves: unsafe extern "C" fn() -> *const crate::aircraft::Leaves,
    ground: unsafe extern "C" fn() -> *const crate::ground_vehicle::Services,
    cargo_services: unsafe extern "C" fn() -> *const crate::cargo_storage::Services,
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
            (s.livery)(id, company, crate::engine_type::INVALID_ENGINE, v)
        } else {
            let first_engine = if ground(s, v) {
                (s.first_engine)(v)
            } else {
                crate::engine_type::INVALID_ENGINE
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
            0x40 => (NCVV_POSITION_CONSIST_LENGTH, 0),
            0x41 => (NCVV_POSITION_SAME_ID_LENGTH, 1),
            0x42 => (NCVV_CONSIST_CARGO_INFORMATION, 2),
            0x43 => (NCVV_COMPANY_INFORMATION, 3),
            0x4d => (NCVV_POSITION_IN_VEHICLE, 4),
            _ => unreachable!(),
        };
        let cache = (s.cache)(v);
        if !crate::bitmath_func::has_bit((*cache).cache_valid, bit) {
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
                        if crate::engine_capabilities::openttd_rust_engine_can_carry(
                            s.capabilities,
                            (s.vehicle.engine)(handle(u)).0.as_ptr(),
                        ) {
                            let cargo_type = (s.cargo_type)(u);
                            classes |= (s.cargo_classes)(cargo_type) as u8;
                            cargoes[cargo_type as usize] =
                                cargoes[cargo_type as usize].wrapping_add(1);
                        }
                        u = next(s, u);
                    }
                    let cargo = first_max(&cargoes);
                    let mut subtypes = [0u8; 256];
                    u = v;
                    while !u.is_null() {
                        let cargo_type = (s.cargo_type)(u);
                        if cargo_type == cargo
                            && crate::engine_capabilities::openttd_rust_engine_can_carry(
                                s.capabilities,
                                (s.vehicle.engine)(handle(u)).0.as_ptr(),
                            )
                        {
                            let subtype = usize::from((s.cargo_subtype)(u));
                            subtypes[subtype] = subtypes[subtype].wrapping_add(1);
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
            let mut valid = u32::from((*cache).cache_valid);
            crate::bitmath_func::set_bit(&mut valid, bit);
            (*cache).cache_valid = valid as u8;
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
pub unsafe extern "C" fn openttd_rust_vehicle_grf_fill(s: &Services, v: V) {
    unsafe {
        for (bit, variable) in [0x40, 0x41, 0x42, 0x43, 0x4d].into_iter().enumerate() {
            if (*(s.cache)(v)).cache_valid & (1 << bit) == 0 {
                cached_variable(s, v, variable, null());
            }
        }
        (s.check)((*(s.cache)(v)).cache_valid == (1 << NCVV_END) - 1);
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
                    wagon_override(s, engine(s, v), (s.cargo_type)(v), (s.first_engine)(v))
                };
            }
            if state.root.is_null() {
                let cargo = if v.is_null() {
                    crate::newgrf_commons::SG_PURCHASE
                } else {
                    (s.cargo_type)(v)
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
        (*scope).set_vehicle(v);
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
        if triggers {
            u32::from((s.waiting)(v))
        } else {
            u32::from((s.random_bits)(v))
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_resolver_scope(
    s: &Services,
    state: *mut State,
    _ro: *const c_void,
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
                count = crate::newgrf_storage::openttd_rust_storage_temp_get(0x100);
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
            let relative_v = (s.move_vehicle)(v, count);
            (*state).relative.set_vehicle(relative_v);
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
        let order = (s.order)((s.first)(v));
        let not_loading = order.unload == crate::order_type::OUFB_NO_UNLOAD
            && order.load == crate::order_type::OLFB_NO_LOAD;
        let motion = !order.loading || not_loading;
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
            state.parent.set_vehicle(parent);
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
        let waiting = (s.waiting)(v) | (1 << trigger);
        (s.write_waiting)(v, waiting);
        let result = (s.resolve_random)(&raw mut state, engine(s, v), waiting);
        (s.write_waiting)(v, (s.waiting)(v) & !(result.used as u8));
        let random = s.shared.random() as u16; // Draw even if reseed is zero.
        let bits = (s.random_bits)(v);
        (s.write_random)(
            v,
            (bits & !(result.reseed as u16))
                | ((if first { random } else { base }) & result.reseed as u16),
        );
        match trigger {
            0 => {
                (s.check)(first);
                randomise(s, (s.first)(v), Trigger::AnyNewCargo as u8, random, false);
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
// Vehicle property/callback table entries point directly to Rust. The caller
// obtains the immutable resolver table once through the native getter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_shell_property(
    s: &Services,
    v: V,
    property: u8,
    fallback: i32,
) -> i32 {
    unsafe { openttd_rust_vehicle_property(s, engine(s, v), property, fallback, v, false) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_shell_callback(
    s: &Services,
    v: V,
    callback: crate::newgrf_callbacks::CallbackID,
) -> u16 {
    unsafe {
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
// Named source compatibility helpers. Opaque shells remain native; these helpers
// read only scalars at the original branches and never retain owner references.
unsafe fn current_max_speed(s: &Services, v: V, kind: u8) -> i32 {
    unsafe {
        match kind {
            0 => crate::train::openttd_rust_train_current_max_speed(
                (s.vehicle.train_handle)(v),
                (s.train_services)(),
                s.shared,
            ),
            1 => {
                let ground = &*(s.ground)();
                let owner = (ground.resolve)(handle(v));
                crate::road::current_max_speed(
                    handle(v),
                    owner.road,
                    (s.road_acceleration)(),
                    ground.road,
                )
            }
            2 => crate::ship_control::current_max_speed(
                (s.vehicle.max_speed)(handle(v)),
                (s.order_max_speed)(v),
            ),
            3 => {
                crate::aircraft::speed_old_units(u32::from((s.vehicle.max_speed)(handle(v)))) as i32
            }
            _ => 0,
        }
    }
}
unsafe fn old_subtype(s: &Services, v: V, kind: u8) -> u32 {
    match kind {
        0 => {
            let subtype = (s.vehicle.subtype)(handle(v));
            if crate::ground_vehicle::is_engine(subtype) {
                0
            } else if crate::ground_vehicle::is_free_wagon(subtype) {
                4
            } else {
                2
            }
        }
        1 | 2 => 0,
        3 | 5 => u32::from((s.vehicle.subtype)(handle(v))),
        4 => u32::from((s.vehicle.subtype)(handle(v))) << 1,
        _ => {
            unsafe { (s.check)(false) };
            unreachable!()
        }
    }
}
const AMS_TTDP_HANGAR: u32 = 0;
const AMS_TTDP_TO_HANGAR: u32 = 1;
const AMS_TTDP_TO_PAD1: u32 = 2;
const AMS_TTDP_TO_PAD2: u32 = 3;
const AMS_TTDP_TO_PAD3: u32 = 4;
const AMS_TTDP_TO_ENTRY_2_AND_3_AND_H: u32 = 6;
const AMS_TTDP_TO_JUNCTION: u32 = 7;
const AMS_TTDP_TO_INWAY: u32 = 9;
const AMS_TTDP_TO_OUTWAY: u32 = 11;
const AMS_TTDP_TAKEOFF: u32 = 13;
const AMS_TTDP_CLIMBING: u32 = 15;
const AMS_TTDP_FLIGHT_APPROACH: u32 = 16;
const AMS_TTDP_FLIGHT_TO_TOWER: u32 = 18;
const AMS_TTDP_FLIGHT_DESCENT: u32 = 21;
const AMS_TTDP_BRAKING: u32 = 22;
const AMS_TTDP_HELI_TAKEOFF_AIRPORT: u32 = 23;
const AMS_TTDP_HELI_LAND_AIRPORT: u32 = 25;
const AMS_TTDP_HELI_TAKEOFF_HELIPORT: u32 = 26;
const AMS_TTDP_HELI_LAND_HELIPORT: u32 = 28;
const AMA_TTDP_IN_HANGAR: u32 = 0;
const AMA_TTDP_ON_PAD1: u32 = 1;
const AMA_TTDP_ON_PAD2: u32 = 2;
const AMA_TTDP_ON_PAD3: u32 = 3;
const AMA_TTDP_LANDING_TO_PAD1: u32 = 7;
const AMA_TTDP_LANDING_TO_PAD2: u32 = 8;
const AMA_TTDP_LANDING_TO_PAD3: u32 = 9;
const AMA_TTDP_PAD1_TO_TAKEOFF: u32 = 13;
const AMA_TTDP_LANDING_TO_HANGAR: u32 = 17;
const AMA_TTDP_IN_FLIGHT: u32 = 18;

unsafe fn aircraft_movement_state(s: &Services, v: V) -> u32 {
    use crate::airport::{
        AMD_BRAKE, AMD_EXACT_POSITION, AMD_HELI_LOWER, AMD_HELI_RAISE, AMD_HOLD, AMD_SLOW_TURN,
        ENDLANDING, ENDTAKEOFF, FLYING, HANGAR, HELIENDLANDING, HELILANDING, HELIPAD1, HELIPAD2,
        HELIPAD3, HELITAKEOFF, LANDING, STARTTAKEOFF, TAKEOFF, TERM1, TERM2, TERM3, TERM4, TERM5,
        TERM6, TERM7, TERM8,
    };
    unsafe {
        let st = (s.target_airport)(v);
        if st.is_null() {
            return AMS_TTDP_FLIGHT_TO_TOWER;
        } // AMS_TTDP_FLIGHT_TO_TOWER
        let leaves = &*(s.aircraft_leaves)();
        let airport = crate::newgrf_airport::station_fta(
            st.cast_mut().cast(),
            crate::newgrf_airport::runtime(),
        );
        let a = (s.aircraft)(v);
        let flags = (s.moving_flags)(airport, (*a).pos);
        let has = |flag: u8| flags & (1u16 << flag) != 0;
        match (*a).state {
            HANGAR => {
                if has(AMD_HELI_LOWER) {
                    AMS_TTDP_HELI_LAND_AIRPORT
                } else if has(AMD_SLOW_TURN) {
                    AMS_TTDP_FLIGHT_TO_TOWER
                } else if has(AMD_EXACT_POSITION) {
                    AMS_TTDP_HANGAR
                } else {
                    AMS_TTDP_TO_HANGAR
                }
            }
            TERM1 => {
                if has(AMD_EXACT_POSITION) {
                    AMS_TTDP_TO_PAD1
                } else {
                    AMS_TTDP_TO_JUNCTION
                }
            }
            TERM2 => {
                if has(AMD_EXACT_POSITION) {
                    AMS_TTDP_TO_PAD2
                } else {
                    AMS_TTDP_TO_ENTRY_2_AND_3_AND_H
                }
            }
            TERM3 | TERM4 | TERM5 | TERM6 | TERM7 | TERM8 => {
                if has(AMD_EXACT_POSITION) {
                    AMS_TTDP_TO_PAD3
                } else {
                    AMS_TTDP_TO_ENTRY_2_AND_3_AND_H
                }
            }
            HELIPAD1 | HELIPAD2 | HELIPAD3 => {
                if has(AMD_HELI_LOWER) {
                    AMS_TTDP_HELI_LAND_AIRPORT
                } else if has(AMD_SLOW_TURN) {
                    AMS_TTDP_FLIGHT_TO_TOWER
                } else {
                    AMS_TTDP_TO_JUNCTION
                }
            }
            TAKEOFF => AMS_TTDP_TO_OUTWAY,
            STARTTAKEOFF => AMS_TTDP_TAKEOFF,
            ENDTAKEOFF => AMS_TTDP_CLIMBING,
            HELITAKEOFF => {
                if (leaves.airport_delta_z)(airport) == 0 {
                    if has(AMD_HELI_RAISE) {
                        AMS_TTDP_HELI_TAKEOFF_AIRPORT
                    } else {
                        AMS_TTDP_TO_JUNCTION
                    }
                } else {
                    AMS_TTDP_HELI_TAKEOFF_HELIPORT
                }
            }
            FLYING => {
                if has(AMD_HOLD) {
                    AMS_TTDP_FLIGHT_APPROACH
                } else {
                    AMS_TTDP_FLIGHT_TO_TOWER
                }
            }
            LANDING => AMS_TTDP_FLIGHT_DESCENT,
            ENDLANDING => {
                if has(AMD_BRAKE) {
                    AMS_TTDP_BRAKING
                } else {
                    AMS_TTDP_TO_INWAY
                }
            }
            HELILANDING | HELIENDLANDING => {
                if has(AMD_HELI_LOWER) {
                    if (leaves.airport_delta_z)(airport) == 0 {
                        AMS_TTDP_HELI_LAND_AIRPORT
                    } else {
                        AMS_TTDP_HELI_LAND_HELIPORT
                    }
                } else {
                    AMS_TTDP_FLIGHT_TO_TOWER
                }
            }
            _ => AMS_TTDP_HANGAR,
        }
    }
}
unsafe fn aircraft_movement_action(s: &Services, v: V) -> u32 {
    use crate::airport::{
        ENDLANDING, ENDTAKEOFF, FLYING, HANGAR, HELIENDLANDING, HELILANDING, HELIPAD1, HELIPAD2,
        HELIPAD3, HELITAKEOFF, LANDING, STARTTAKEOFF, TAKEOFF, TERM1, TERM2, TERM3, TERM4, TERM5,
        TERM6, TERM7, TERM8,
    };
    unsafe {
        let a = (s.aircraft)(v);
        match (*a).state {
            HANGAR => {
                if (s.vehicle.speed)(handle(v)) > 0 {
                    AMA_TTDP_LANDING_TO_HANGAR
                } else {
                    AMA_TTDP_IN_HANGAR
                }
            }
            TERM1 | HELIPAD1 => {
                if (s.vehicle.order_type)(handle(v)) == crate::vehicle::OT_LOADING {
                    AMA_TTDP_ON_PAD1
                } else {
                    AMA_TTDP_LANDING_TO_PAD1
                }
            }
            TERM2 | HELIPAD2 => {
                if (s.vehicle.order_type)(handle(v)) == crate::vehicle::OT_LOADING {
                    AMA_TTDP_ON_PAD2
                } else {
                    AMA_TTDP_LANDING_TO_PAD2
                }
            }
            TERM3 | TERM4 | TERM5 | TERM6 | TERM7 | TERM8 | HELIPAD3 => {
                if (s.vehicle.order_type)(handle(v)) == crate::vehicle::OT_LOADING {
                    AMA_TTDP_ON_PAD3
                } else {
                    AMA_TTDP_LANDING_TO_PAD3
                }
            }
            TAKEOFF | STARTTAKEOFF | ENDTAKEOFF | HELITAKEOFF => AMA_TTDP_PAD1_TO_TAKEOFF,
            FLYING => AMA_TTDP_IN_FLIGHT,
            LANDING | ENDLANDING | HELILANDING | HELIENDLANDING => {
                if (s.vehicle.order_type)(handle(v)) == crate::vehicle::OT_GOTO_DEPOT {
                    AMA_TTDP_LANDING_TO_HANGAR
                } else {
                    AMA_TTDP_LANDING_TO_PAD1
                }
            }
            _ => AMA_TTDP_IN_HANGAR,
        }
    }
}
fn service_date(date: i32) -> u32 {
    date.wrapping_sub(crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR)
        .clamp(0, i32::from(u16::MAX)) as u32
}
fn old_year(year: i32) -> u32 {
    use crate::timer_game_common::{ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR};
    (year.clamp(ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR) - ORIGINAL_BASE_YEAR) as u32
}
unsafe fn cargo_info(s: &Services, cargo: u8, grf: *const c_void) -> u32 {
    unsafe {
        let info = (s.cargo_info)(cargo);
        u32::from(info.classes) << 16
            | u32::from(info.weight) << 8
            | u32::from((s.cargo_translation)(grf, cargo))
    }
}
unsafe fn variable(
    s: &Services,
    v: V,
    rotor: bool,
    ro: *const c_void,
    grf: *const c_void,
    callback: u16,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    use crate::direction_func::{change_dir_diff, dir_difference};
    use crate::direction_type::{DIR_W, DIRDIFF_REVERSE, DIRDIFF_SAME};
    use crate::newgrf_callbacks::{
        CBID_NO_CALLBACK, CBID_RANDOM_TRIGGER, CBID_TRAIN_ALLOW_WAGON_ATTACH,
        CBID_VEHICLE_32DAY_CALLBACK, CBID_VEHICLE_COLOUR_MAPPING, CBID_VEHICLE_SPAWN_VISUAL_EFFECT,
        CBID_VEHICLE_START_STOP_CHECK,
    };
    unsafe {
        let h = handle(v);
        let kind = || (s.vehicle.vehicle_type)(h);
        let check = |ok| (s.check)(ok);
        match variable {
            0x25 => return (s.vehicle_grfid)(v),
            0x40 | 0x41 | 0x42 | 0x43 | 0x4d => return cached_variable(s, v, variable, grf),
            0x44 => {
                if kind() != 3 || !crate::aircraft::is_normal_aircraft((s.vehicle.subtype)(h)) {
                    return u32::MAX;
                }
                let w = next(s, v);
                (s.check)(!w.is_null());
                let altitude = (s.vehicle.z)(h)
                    .wrapping_sub((s.vehicle.z)(handle(w)))
                    .clamp(0, i32::from(u16::MAX));
                let st = (s.target_airport)(v);
                let leaves = &*(s.aircraft_leaves)();
                let airport = if !st.is_null()
                    && (leaves.airport_tile)(st) != crate::tile_type::INVALID_TILE
                {
                    (s.airport_ttd_type)(crate::newgrf_airport::station_spec(
                        st.cast_mut().cast(),
                        crate::newgrf_airport::runtime(),
                    ))
                    .0
                } else {
                    crate::newgrf_airport::ATP_TTDP_LARGE
                };
                return (altitude.min(i32::from(u8::MAX)) as u32) << 8 | u32::from(airport);
            }
            0x45 => {
                if !ground(s, v) {
                    return 0;
                }
                let p = (s.previous)(v);
                let n = next(s, v);
                let direction = (s.vehicle.direction)(h);
                let f = if p.is_null() {
                    DIRDIFF_SAME
                } else {
                    dir_difference((s.vehicle.direction)(handle(p)), direction, check)
                };
                let b = if n.is_null() {
                    DIRDIFF_SAME
                } else {
                    dir_difference(direction, (s.vehicle.direction)(handle(n)), check)
                };
                let t = change_dir_diff(f, b);
                let encode = |d| u32::from(if d > DIRDIFF_REVERSE { d | 8 } else { d });
                return encode(t) << 16 | encode(b) << 8 | encode(f);
            }
            0x46 => return (s.motion)(v),
            0x47 => return cargo_info(s, (s.cargo_type)(v), grf),
            0x48 => return u32::from((s.engine_flags)((s.vehicle.engine)(h).0.as_ptr())),
            0x49 => return (s.build_year)(v) as u32,
            0x4a => {
                let tile = (s.vehicle.tile)(h);
                match kind() {
                    0 => {
                        let rt = (s.tile_railtype)(tile);
                        let types = (s.vehicle.train_handle)(v).get_railtypes();
                        return (u32::from((s.rail_catenary)(rt)) * 0x200)
                            | (u32::from((s.powered_rail)(types, rt)) * 0x100)
                            | u32::from((s.reverse_rail)(rt, grf));
                    }
                    1 => {
                        let rtt = (s.shared.road_tram_type)((s.roadtype)(v).0);
                        let rt = (s.tile_roadtype)(tile, rtt);
                        return (u32::from((s.road_catenary)(rt)) * 0x200)
                            | 0x100
                            | u32::from((s.reverse_road)(rt, grf));
                    }
                    _ => return 0,
                }
            }
            0x4b => return (s.last_service)(v) as u32,
            0x4c => {
                return if (s.primary)(v) {
                    current_max_speed(s, v, kind()) as u32
                } else {
                    0
                };
            }
            0x60 => {
                if kind() != 0 {
                    return u32::from(
                        u32::from((s.local_id)((s.vehicle.engine)(h).0.as_ptr())) == parameter,
                    );
                }
                let mut count = 0u32;
                let mut u = v;
                while !u.is_null() {
                    if u32::from((s.local_id)((s.vehicle.engine)(handle(u)).0.as_ptr()))
                        == parameter
                    {
                        count = count.wrapping_add(1);
                    }
                    u = next(s, u);
                }
                return count;
            }
            0x61 => {
                if ground(s, v)
                    && parameter != 0x61
                    && [
                        CBID_NO_CALLBACK.0,
                        CBID_RANDOM_TRIGGER.0,
                        CBID_TRAIN_ALLOW_WAGON_ATTACH.0,
                        CBID_VEHICLE_START_STOP_CHECK.0,
                        CBID_VEHICLE_32DAY_CALLBACK.0,
                        CBID_VEHICLE_COLOUR_MAPPING.0,
                        CBID_VEHICLE_SPAWN_VISUAL_EFFECT.0,
                    ]
                    .contains(&callback)
                {
                    let u = (s.move_vehicle)(
                        v,
                        crate::newgrf_storage::openttd_rust_storage_temp_get(0x10f),
                    );
                    if u.is_null() {
                        return 0;
                    }
                    if parameter == 0x5f {
                        return u32::from((s.random_bits)(u)) << 8 | u32::from((s.waiting)(u));
                    }
                    return self::variable(
                        s,
                        u,
                        rotor,
                        ro,
                        grf,
                        callback,
                        parameter as u8,
                        crate::newgrf_storage::openttd_rust_storage_temp_get(0x10e) as u32,
                        available,
                    );
                }
            }
            0x62 => {
                if !ground(s, v) {
                    return 0;
                }
                let u = (s.move_vehicle)(v, i32::from(parameter as i8));
                if u.is_null() {
                    return 0;
                }
                let prev = (parameter as i8) < 0;
                let a = (s.vehicle.direction)(h);
                let b = (s.vehicle.direction)(handle(u));
                let mut result = u32::from(if prev {
                    dir_difference(b, a, check)
                } else {
                    dir_difference(a, b, check)
                });
                if result > u32::from(DIRDIFF_REVERSE) {
                    result |= 8;
                }
                if (s.vehicle.status)(handle(u)) & crate::vehicle_base::VS_HIDDEN != 0 {
                    result |= 0x80;
                }
                let a = (s.vehicle.coordinates)(h);
                let b = (s.vehicle.coordinates)(handle(u));
                let difference = |a: i32, b: i32| {
                    (if prev {
                        b.wrapping_sub(a)
                    } else {
                        a.wrapping_sub(b)
                    } as u32)
                        & 0xff
                };
                return result
                    | difference(a.x, b.x) << 8
                    | difference(a.y, b.y) << 16
                    | difference(a.z, b.z) << 24;
            }
            0x63 => match kind() {
                0 => {
                    let param = (s.translate_rail)(parameter as u8, grf);
                    if param.0 == crate::rail_type::INVALID_RAILTYPE {
                        return 0;
                    }
                    let tile = (s.tile_railtype)((s.vehicle.tile)(h));
                    if tile.0 == param.0 {
                        return 15;
                    }
                    return (u32::from((s.powered_single_rail)(param, tile)) * 4)
                        | (u32::from((s.compatible_rail)(param, tile)) * 2)
                        | 1;
                }
                1 => {
                    let rtt = (s.shared.road_tram_type)((s.roadtype)(v).0);
                    let param = (s.translate_road)(rtt, parameter as u8, grf);
                    if param.0 == crate::road_type::INVALID_ROADTYPE {
                        return 0;
                    }
                    let tile = (s.tile_roadtype)((s.vehicle.tile)(h), rtt);
                    if tile.0 == param.0 {
                        return 15;
                    }
                    return if (s.shared.road_powered)(param.0, tile.0) {
                        7
                    } else {
                        1
                    };
                }
                _ => return 0,
            },
            0x64 => {
                if kind() != 0 {
                    return (s.engine_badge)(ro, (s.vehicle.engine)(h).0.as_ptr(), parameter);
                }
                if u64::from(parameter) >= (s.badge_count)(grf) as u64 {
                    return u32::MAX;
                }
                let badge = (s.badge_at)(grf, parameter);
                let mut count = 0u32;
                let mut u = v;
                while !u.is_null() {
                    if (s.engine_has_badge)((s.vehicle.engine)(handle(u)).0.as_ptr(), badge) {
                        count = count.wrapping_add(1);
                    }
                    u = next(s, u);
                }
                return count;
            }
            0x65 => match kind() {
                0 => {
                    return (s.rail_badge)(
                        ro,
                        ((&*(s.ground)()).rail_type)((s.vehicle.tile)(h)),
                        parameter,
                    );
                }
                1 => {
                    let rtt = (s.shared.road_tram_type)((s.roadtype)(v).0);
                    return (s.road_badge)(
                        ro,
                        (s.tile_roadtype)((s.vehicle.tile)(h), rtt),
                        parameter,
                    );
                }
                _ => return u32::MAX,
            },
            0x7a => return (s.engine_badge)(ro, (s.vehicle.engine)(h).0.as_ptr(), parameter),
            0xfe | 0xff => {
                let mut flags = 0u32;
                if kind() == 0 {
                    let t = (s.vehicle.train_handle)(v);
                    let f = t.get_flags();
                    let wagon = f & crate::train::VRF_POWERED_WAGON != 0;
                    let u = if wagon {
                        (s.vehicle.train_handle)((s.first)(v))
                    } else {
                        t
                    };
                    let rt = ((&*(s.ground)()).rail_type)((s.vehicle.tile)(h));
                    let powered = crate::ground_vehicle::is_engine((s.vehicle.subtype)(h)) || wagon;
                    let power = (s.powered_rail)(u.get_railtypes(), rt);
                    if powered {
                        flags |= if power { 1 << 5 } else { 1 << 6 };
                    }
                    if f & crate::train::VRF_REVERSED != 0 {
                        flags |= 1 << 8;
                    }
                }
                let f = (s.vehicle_flags)(v);
                if f & crate::base_consist::VF_CARGO_UNLOADING != 0 {
                    flags |= 1 << 1;
                }
                if f & crate::base_consist::VF_BUILT_AS_PROTOTYPE != 0 {
                    flags |= 1 << 10;
                }
                return if variable == 0xfe { flags } else { flags >> 8 };
            }
            _ => {}
        }
        // Byte slices are computed here from the complete scalar, preserving the
        // source distinction between an unmasked low word and its high bytes.
        let high = |value: u32| {
            if variable & 1 == 0 {
                value
            } else {
                (value >> 8) & 0xff
            }
        };
        match variable {
            0x80 => return u32::from(kind()) + 0x10,
            0x81 => return old_subtype(s, v, kind()),
            0x84 | 0x85 => return high((s.id)(v)),
            0x8a => return u32::from((s.old_order)(v)),
            0x8b => return u32::from((s.vehicle.order_destination)(h)),
            0x8c => return u32::from((s.num_orders)(v)),
            0x8d => return u32::from((s.real_order)(v)),
            0x90 | 0x91 => {
                let ticks = if (s.vehicle.order_type)(h) == crate::vehicle::OT_LOADING {
                    u32::from((s.load_ticks)(v))
                } else {
                    match kind() {
                        0 => (s.vehicle.train_handle)(v).get_wait_counter() as u32,
                        3 => u32::from((*(s.aircraft)(v)).turn_counter),
                        _ => 0,
                    }
                };
                return high(ticks);
            }
            0x92 | 0x93 => return high(service_date((s.last_service)(v))),
            0x94 | 0x95 => return high(u32::from((s.service_interval)(v))),
            0x96 => return u32::from((s.vehicle.last_station)(h)),
            0x97 => return u32::from((s.vehicle.tick)(h)),
            0x98 | 0x99 => {
                let speed = u32::from((s.vehicle.max_speed)(h));
                return high(if kind() == 3 {
                    crate::aircraft::speed_old_units(speed)
                } else {
                    speed
                });
            }
            0x9a | 0x9b => return high((s.vehicle.x)(h) as u32),
            0x9c | 0x9d => return high((s.vehicle.y)(h) as u32),
            0x9e => return (s.vehicle.z)(h) as u32,
            0x9f => {
                return u32::from(if rotor {
                    DIR_W
                } else {
                    (s.vehicle.direction)(h)
                });
            }
            0xa8 | 0xa9 | 0xb3 => return 0,
            0xb2 => return u32::from((s.vehicle.status)(h)),
            0xb4 | 0xb5 => {
                let speed = u32::from((s.vehicle.speed)(h));
                return high(if kind() == 3 { speed * 10 / 128 } else { speed });
            }
            0xb6 => return u32::from((s.vehicle.subspeed)(h)),
            0xb7 => return u32::from((s.acceleration)(v)),
            0xb9 => return u32::from((s.cargo_type)(v)),
            0xba | 0xbb => return high(u32::from((s.cargo_capacity)(v))),
            0xbc | 0xbd => {
                return high(
                    crate::cargo_storage::openttd_rust_cargo_list_stored((s.cargo_owner)(v))
                        .min(u32::from(u16::MAX)),
                );
            }
            0xbe => {
                return u32::from(crate::cargo_storage::openttd_rust_cargo_list_first(
                    (s.cargo_owner)(v),
                    (s.cargo_services)(),
                ));
            }
            0xbf => {
                return crate::cargo_storage::openttd_rust_cargo_list_periods((s.cargo_owner)(v))
                    .min(u32::from(u8::MAX));
            }
            0xc0 | 0xc1 => return high((s.age)(v).clamp(0, i32::from(u16::MAX)) as u32),
            0xc2 | 0xc3 => return high((s.max_age)(v).clamp(0, i32::from(u16::MAX)) as u32),
            0xc4 => return old_year((s.build_year)(v)),
            0xc5 => return u32::from((s.unit)(v)),
            0xc6 | 0xc7 => return high(u32::from((s.local_id)((s.vehicle.engine)(h).0.as_ptr()))),
            0xc8 => {
                let sprite = (s.sprite)(v);
                if kind() != 0 || sprite != crate::vehicle_func::CUSTOM_VEHICLE_SPRITENUM {
                    return u32::from(sprite);
                }
                return u32::from(
                    if (s.vehicle.train_handle)(v).get_flags() & crate::train::VRF_FLIPPED != 0 {
                        crate::vehicle_func::CUSTOM_VEHICLE_SPRITENUM_REVERSED
                    } else {
                        crate::vehicle_func::CUSTOM_VEHICLE_SPRITENUM
                    },
                );
            }
            0xc9 => return u32::from((s.vehicle.day)(h)),
            0xca => return u32::from((s.breakdowns)(v)),
            0xcb => return u32::from((s.breakdown_counter)(v)),
            0xcc => return u32::from((s.breakdown_delay)(v)),
            0xcd => return u32::from((s.breakdown_chance)(v)),
            0xce | 0xcf => return high(u32::from((s.reliability)(v))),
            0xd0 | 0xd1 => return high(u32::from((s.reliability_decay)(v))),
            0xd2..=0xd9 | 0xdc..=0xdf => {
                let value = match variable {
                    0xd2..=0xd5 => (s.profit_this)(v),
                    0xd6..=0xd9 => (s.profit_last)(v),
                    _ => (s.value)(v),
                }
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as u32;
                let shift = u32::from(match variable {
                    0xd2..=0xd5 => variable - 0xd2,
                    0xd6..=0xd9 => variable - 0xd6,
                    _ => variable - 0xdc,
                }) * 8;
                return value >> shift;
            }
            0xda => {
                let n = next(s, v);
                return if n.is_null() {
                    crate::fleet::INVALID_VEHICLE
                } else {
                    (s.id)(n)
                };
            }
            0xf2 => return u32::from((s.cargo_subtype)(v)),
            0xfa => return u32::from((s.random_bits)(v)),
            0xfb => return u32::from((s.waiting)(v)),
            _ => {}
        }
        match kind() {
            0 => match variable {
                0xe2 => return (s.vehicle.train_handle)(v).get_track() as u32,
                0xe6 => {
                    let bits = (s.vehicle.train_handle)(v).get_railtypes();
                    return u32::from(
                        crate::base_bitset_type::get_nth_set_bit(bits, 0)
                            .unwrap_or(crate::rail_type::INVALID_RAILTYPE),
                    );
                }
                0xf3 => {
                    let owner = ((&*(s.ground)()).resolve)(handle(v));
                    return (0x80 + u32::from(crate::vehicle::VEHICLE_LENGTH))
                        .wrapping_sub(u32::from((*owner.state).cache.cached_veh_length));
                }
                0xf4..=0xf7 => {
                    let owner = ((&*(s.ground)()).resolve)(handle(v));
                    return (*owner.state).cache.cached_power >> (u32::from(variable - 0xf4) * 8);
                }
                0xfc | 0xfd => return high((s.id)((s.first)(v))),
                0xff => return 0,
                _ => {}
            },
            1 => match variable {
                0xe2 | 0xe4..=0xe9 => {
                    let owner = ((&*(s.ground)()).resolve)(handle(v));
                    return match variable {
                        0xe2 => u32::from(crate::road::get_state(owner.road)),
                        0xe4 | 0xe5 => high(u32::from(crate::road::blocked_counter(owner.road))),
                        0xe6 => u32::from(crate::road::overtaking(owner.road)),
                        0xe7 => u32::from(crate::road::overtaking_counter(owner.road)),
                        _ => high(u32::from(crate::road::crashed_counter(owner.road))),
                    };
                }
                _ => {}
            },
            2 => {
                if variable == 0xe2 {
                    return u32::from(crate::ship_control::openttd_rust_ship_get_state((s
                        .ship_owner)(
                        v
                    )));
                }
            }
            3 => match variable {
                0xe2 => return aircraft_movement_state(s, v),
                0xe3 => return u32::from((*(s.aircraft)(v)).targetairport),
                0xe6 => return aircraft_movement_action(s, v),
                _ => {}
            },
            _ => {}
        }
        (s.unhandled)(variable, kind());
        *available = false;
        u32::MAX
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_scope_variable(
    s: &Services,
    scope: *const Scope,
    ro: *const c_void,
    grf: *const c_void,
    callback: u16,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    unsafe {
        let v = (*scope).v;
        if !v.is_null() {
            return self::variable(
                s,
                v,
                (*scope).rotor_in_gui,
                ro,
                grf,
                callback,
                variable,
                parameter,
                available,
            );
        }
        match variable {
            0x43 => (s.company_info)(
                (s.current_company)(),
                livery(s, (*scope).self_type, null_mut()),
            ),
            0x46 | 0xf2 => 0,
            0x47 => {
                let cargo = (s.default_cargo)((s.engine_by_id)((*scope).self_type));
                if crate::cargo_type::is_valid_cargo_type(cargo) {
                    cargo_info(s, cargo, grf)
                } else {
                    0xff
                }
            }
            0x48 => u32::from((s.engine_flags)((s.engine_by_id)((*scope).self_type))),
            0x49 => (s.shared.calendar_year)() as u32,
            0x4b => (s.shared.calendar_date)() as u32,
            0x7a => (s.engine_badge)(ro, (s.engine_by_id)((*scope).self_type), parameter),
            0x92 => service_date((s.shared.calendar_date)()),
            0x93 => service_date((s.shared.calendar_date)()) >> 8,
            0xc4 => old_year((s.shared.calendar_year)()),
            0xc6 => u32::from((s.local_id)((s.engine_by_id)((*scope).self_type))),
            0xc7 => u32::from((s.local_id)((s.engine_by_id)((*scope).self_type))) >> 8,
            0xda => crate::fleet::INVALID_VEHICLE,
            _ => {
                *available = false;
                u32::MAX
            }
        }
    }
}

/// Vehicle random trigger domain from `newgrf_engine.h`.
#[derive(Clone, Copy)]
pub enum Trigger {
    NewCargo = 0,
    Depot = 1,
    Empty = 2,
    AnyNewCargo = 3,
    Callback32 = 4,
}
/// `Train::tcache.cached_override = GetWagonOverrideSpriteSet(...)`.
pub(crate) unsafe fn cache_override(vs: &VehicleServices, v: VehicleRef) {
    unsafe {
        let s = &*(vs.newgrf)();
        let shell = v.0.as_ptr();
        let group = wagon_override(
            s,
            (vs.engine_id)(v),
            (s.cargo_type)(shell),
            (s.first_engine)(shell),
        );
        (s.write_override)(shell, group);
    }
}
/// Direct owner entry; the caller preserves the live shell through evaluation.
pub(crate) unsafe fn trigger(vs: &VehicleServices, v: VehicleRef, trigger: Trigger) {
    unsafe { openttd_rust_vehicle_randomise(&*(vs.newgrf)(), v.0.as_ptr(), trigger as u8) }
}
/// Direct callback; `None` preserves purchase/clone-estimate contexts without a shell.
pub(crate) unsafe fn callback(
    vs: &VehicleServices,
    v: Option<VehicleRef>,
    engine: u16,
    callback: u16,
    parameter: u32,
    parent: V,
    override_parent: bool,
    registers: *mut i32,
    count: usize,
) -> u16 {
    unsafe {
        openttd_rust_vehicle_callback(
            &*(vs.newgrf)(),
            callback,
            parameter,
            0,
            engine,
            v.map_or(null_mut(), |v| v.0.as_ptr()),
            parent,
            override_parent,
            registers,
            count,
        )
    }
}
/// Direct signed property entry (unsigned users use the base vehicle table).
pub(crate) unsafe fn signed_property(
    vs: &VehicleServices,
    v: VehicleRef,
    property: u8,
    fallback: i32,
) -> i32 {
    unsafe {
        openttd_rust_vehicle_property(
            &*(vs.newgrf)(),
            (vs.engine_id)(v),
            property,
            fallback,
            v.0.as_ptr(),
            true,
        )
    }
}
/// `TestVehicleBuildProbability(..., Reversed).has_value()` still consumes its RNG.
pub(crate) unsafe fn reverse_probability(vs: &VehicleServices, v: VehicleRef) -> bool {
    unsafe {
        openttd_rust_vehicle_build_probability(
            &*(vs.newgrf)(),
            v.0.as_ptr(),
            (vs.engine_id)(v),
            BUILD_PROBABILITY_REVERSED,
        ) >= 0
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleLoadingOrder" ["vehicle_resolver"] => OrderState { unload, load, loading };
    "OpenTTDVehicleRandomResult" ["vehicle_resolver"] => RandomResult { used, reseed };
    "OpenTTDVehicleCargoInfo" ["vehicle_resolver"] => CargoInfo { classes, weight };
    "OpenTTDVehicleOverride" ["vehicle_resolver"] => Override { group, cargo, count };
    "OpenTTDVehicleOverrides" ["vehicle_resolver"] => Overrides { engine, count };
    "OpenTTDVehicleResolverServices" ["vehicle_resolver"] => Services {
        vehicle, shared, cache, first, move_vehicle => move, previous, capabilities,
        cargo_classes, cargo_bitnum, cargo_translation, grf_version, current_company,
        valid_company, livery, company_info, first_engine, articulated, has_articulated,
        overrides, override_at, override_engine, cached_override, write_override, spritegroup, random_bits, waiting,
        write_random, write_waiting, resolve_callback, resolve_random, order, cargo_owner,
        cargo_capacity, real_count, real_at,
        vehicle_grfid, engine_by_id, default_cargo, engine_flags, local_id, cargo_info, motion, build_year, last_service, primary, train_services, road_acceleration, order_max_speed, id, old_order, num_orders, real_order, load_ticks, service_interval, acceleration, cargo_type, cargo_subtype, age, max_age, unit, sprite, breakdowns, breakdown_counter, breakdown_delay, breakdown_chance, reliability, reliability_decay, value, profit_this, profit_last, vehicle_flags, aircraft, target_airport, airport_ttd_type, moving_flags, roadtype, tile_roadtype, tile_railtype, rail_catenary, road_catenary, reverse_rail, reverse_road, translate_rail, translate_road, compatible_rail, powered_rail, powered_single_rail, badge_count, badge_at, engine_has_badge, engine_badge, rail_badge, road_badge, ship_owner, unhandled, aircraft_leaves, ground, cargo_services, check
    };
    "NewGRFCache" ["vehicle_resolver"] => Cache { position_consist_length, position_same_id_length, consist_cargo_information, company_information, position_in_vehicle, cache_valid };
    "OpenTTDVehicleScopeState" ["vehicle_resolver"] => Scope { v, self_type, rotor_in_gui };
    "OpenTTDVehicleResolverState" ["vehicle_resolver"] => State { self_scope => self_, parent, relative, cached_relative_count, root };
}

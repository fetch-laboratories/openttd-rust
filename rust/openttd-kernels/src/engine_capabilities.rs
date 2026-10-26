/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine cargo capabilities and complete articulated query traversal.
//! Native specs, cargo definitions and pooled shells remain shared. No borrow
//! into an owner survives a callback, TEMP allocation/destruction or cache work.
//! Entries are synchronous, allocate only the original pooled TEMP shell, and abort
//! on panic; native environmental failures terminate in noexcept leaves.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
use crate::bitmath_func::{gb, has_bit, set_bit64};
use crate::cargo_type::{
    ALL_CARGOTYPES, CT_MAIL, CT_PASSENGERS, INVALID_CARGO, NUM_CARGO, is_valid_cargo_type,
};
use crate::engine_type::{INVALID_ENGINE, NO_DEFAULT_CARGO_MULTIPLIER};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_VEHICLE_ARTIC_ENGINE, CBID_VEHICLE_REFIT_CAPACITY, VEHICLE_ARTIC_ENGINE,
    VEHICLE_CARGO_SUFFIX, VEHICLE_REFIT_CAPACITY,
};
use crate::newgrf_properties::{
    PROP_AIRCRAFT_MAIL_CAPACITY, PROP_AIRCRAFT_PASSENGER_CAPACITY, PROP_ROADVEH_CARGO_CAPACITY,
    PROP_SHIP_CARGO_CAPACITY, PROP_TRAIN_CARGO_CAPACITY,
};
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, RailVehicleTypes, VehicleType};
use std::ffi::c_void;
use std::ptr::null_mut;
type E = *const OpenTTDEngine;
type V = *mut OpenTTDVehicle;
/// Source bound includes the front vehicle, so callback indices are 1..99.
pub(crate) const MAX_ARTICULATED_PARTS: u32 = 100;
#[repr(C)]
pub struct Services {
    resolver: extern "C" fn() -> *const crate::newgrf_vehicle::Services,
    engine: extern "C" fn(u16) -> E,
    engine_if_valid: extern "C" fn(u16) -> E,
    kind: extern "C" fn(E) -> VehicleType,
    ground: extern "C" fn(E) -> bool,
    id: extern "C" fn(E) -> u16,
    default_cargo: extern "C" fn(E) -> u8,
    callbacks: extern "C" fn(E) -> u16,
    carrying: extern "C" fn(E) -> crate::engine::Carrying,
    capacity_config: extern "C" fn(E) -> crate::engine::CapacityConfig,
    refit_mask: extern "C" fn(E) -> u64,
    ship_capacity: extern "C" fn(E) -> u16,
    passenger_capacity: extern "C" fn(E) -> u16,
    mail_capacity: extern "C" fn(E) -> u8,
    rail_kind: extern "C" fn(E) -> RailVehicleTypes,
    grf: extern "C" fn(E) -> *const c_void,
    version: extern "C" fn(*const c_void) -> u8,
    new_engine: extern "C" fn(*const c_void, VehicleType, u16) -> u16,
    cargo_classes: extern "C" fn(u8) -> u16,
    multiplier: extern "C" fn(u8) -> u16,
    pool_available: extern "C" fn(usize) -> bool,
    new_temporary: extern "C" fn() -> V,
    destroy_temporary: extern "C" fn(V),
    set_engine: extern "C" fn(V, u16),
    set_owner: extern "C" fn(V, u8),
    current_company: extern "C" fn() -> u8,
    vehicle_engine: extern "C" fn(V) -> u16,
    vehicle_engine_object: extern "C" fn(V) -> E,
    vehicle_cargo: extern "C" fn(V) -> u8,
    vehicle_subtype: extern "C" fn(V) -> u8,
    has_articulated: extern "C" fn(V) -> bool,
    next_articulated: extern "C" fn(V) -> V,
    check: extern "C" fn(bool),
    assertions: bool,
}
unsafe fn callback(s: &Services, cb: u16, p1: u32, engine: u16, v: V) -> u16 {
    unsafe {
        crate::newgrf_vehicle::openttd_rust_vehicle_callback(
            &*(s.resolver)(),
            cb,
            p1,
            0,
            engine,
            v,
            null_mut(),
            false,
            null_mut(),
            0,
        )
    }
}
unsafe fn property(s: &Services, engine: u16, prop: u8, fallback: u32, v: V) -> u32 {
    unsafe {
        crate::newgrf_vehicle::openttd_rust_vehicle_property(
            &*(s.resolver)(),
            engine,
            prop,
            fallback as i32,
            v,
            false,
        ) as u32
    }
}
fn can_carry(info: crate::engine::Carrying) -> bool {
    match info.kind {
        VehicleType::TRAIN | VehicleType::ROAD => {
            if info.ground_capacity == 0 {
                return false;
            }
        }
        VehicleType::SHIP | VehicleType::AIRCRAFT => {}
        _ => unreachable!("cargo engine type"),
    }
    is_valid_cargo_type(info.default_cargo)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_can_carry(s: &Services, e: E) -> bool {
    can_carry((s.carrying)(e))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_capacity(
    s: &Services,
    e: E,
    v: V,
    mail: *mut u16,
) -> u32 {
    unsafe {
        if s.assertions {
            (s.check)(v.is_null() || (s.id)(e) == (s.vehicle_engine)(v));
        }
        if !mail.is_null() {
            mail.write(0);
        }
        let carrying = (s.carrying)(e);
        if !can_carry(carrying) {
            return 0;
        }
        let config = (s.capacity_config)(e);
        let new_multipliers = config.misc & NO_DEFAULT_CARGO_MULTIPLIER != 0;
        let mut default_cargo = carrying.default_cargo;
        let cargo = if v.is_null() {
            default_cargo
        } else {
            (s.vehicle_cargo)(v)
        };
        if !mail.is_null()
            && carrying.kind == VehicleType::AIRCRAFT
            && crate::cargotype::is_cargo_in_class(
                (s.cargo_classes)(cargo),
                crate::cargotype::PASSENGERS,
            )
        {
            mail.write(property(
                s,
                config.id,
                PROP_AIRCRAFT_MAIL_CAPACITY,
                u32::from((s.mail_capacity)(e)),
                v,
            ) as u16);
        }
        if config.callbacks & VEHICLE_REFIT_CAPACITY != 0
            && (new_multipliers
                || default_cargo != cargo
                || (!v.is_null() && (s.vehicle_subtype)(v) != 0))
        {
            let result = callback(s, CBID_VEHICLE_REFIT_CAPACITY.0, 0, config.id, v);
            if result != CALLBACK_FAILED {
                return u32::from(result);
            }
        }
        let mut extra_mail = 0_u32;
        let mut capacity = match carrying.kind {
            VehicleType::TRAIN => {
                let mut capacity = property(
                    s,
                    config.id,
                    PROP_TRAIN_CARGO_CAPACITY,
                    u32::from(carrying.ground_capacity),
                    v,
                );
                if v.is_null() && (s.rail_kind)(e) == RailVehicleTypes::MULTIHEAD {
                    capacity = capacity.wrapping_add(u32::from(carrying.ground_capacity));
                }
                capacity
            }
            VehicleType::ROAD => property(
                s,
                config.id,
                PROP_ROADVEH_CARGO_CAPACITY,
                u32::from(carrying.ground_capacity),
                v,
            ),
            VehicleType::SHIP => property(
                s,
                config.id,
                PROP_SHIP_CARGO_CAPACITY,
                u32::from((s.ship_capacity)(e)),
                v,
            ),
            VehicleType::AIRCRAFT => {
                let capacity = property(
                    s,
                    config.id,
                    PROP_AIRCRAFT_PASSENGER_CAPACITY,
                    u32::from((s.passenger_capacity)(e)),
                    v,
                );
                if !crate::cargotype::is_cargo_in_class(
                    (s.cargo_classes)(cargo),
                    crate::cargotype::PASSENGERS,
                ) {
                    extra_mail = property(
                        s,
                        config.id,
                        PROP_AIRCRAFT_MAIL_CAPACITY,
                        u32::from((s.mail_capacity)(e)),
                        v,
                    );
                }
                if is_valid_cargo_type(crate::cargo_registry::openttd_rust_cargo_by_label(CT_MAIL))
                    && !new_multipliers
                    && cargo == crate::cargo_registry::openttd_rust_cargo_by_label(CT_MAIL)
                {
                    return capacity.wrapping_add(extra_mail);
                }
                default_cargo = crate::cargo_registry::openttd_rust_cargo_by_label(CT_PASSENGERS);
                capacity
            }
            _ => unreachable!("cargo engine type"),
        };
        if !new_multipliers {
            capacity = capacity.wrapping_add(extra_mail);
            extra_mail = 0;
        }
        if new_multipliers || (carrying.kind != VehicleType::SHIP && default_cargo != cargo) {
            let default_multiplier = if new_multipliers {
                0x100_u16
            } else {
                (s.multiplier)(default_cargo)
            };
            let cargo_multiplier = (s.multiplier)(cargo);
            capacity = capacity.wrapping_mul(u32::from(cargo_multiplier));
            if extra_mail > 0
                && is_valid_cargo_type(crate::cargo_registry::openttd_rust_cargo_by_label(CT_MAIL))
            {
                let mail_multiplier = u32::from((s.multiplier)(
                    crate::cargo_registry::openttd_rust_cargo_by_label(CT_MAIL),
                ));
                capacity = capacity.wrapping_add(
                    (u32::from(default_multiplier)
                        .wrapping_mul(extra_mail)
                        .wrapping_mul(u32::from(cargo_multiplier))
                        .wrapping_add(mail_multiplier / 2))
                        / mail_multiplier,
                );
            }
            capacity = capacity.wrapping_add(u32::from(default_multiplier / 2))
                / u32::from(default_multiplier);
        }
        capacity
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_refittable(s: &Services, id: u16) -> bool {
    unsafe {
        let e = (s.engine_if_valid)(id);
        if e.is_null() || !openttd_rust_engine_can_carry(s, e) {
            return false;
        }
        if (s.refit_mask)(e) == 0 {
            return false;
        }
        if (s.callbacks)(e) & VEHICLE_CARGO_SUFFIX != 0 {
            return true;
        }
        let cargo = (s.default_cargo)(e);
        let mut mask = 0;
        set_bit64(&mut mask, cargo);
        is_valid_cargo_type(cargo) && (s.refit_mask)(e) != mask
    }
}
pub(crate) unsafe fn openttd_rust_articulated_next(
    s: &Services,
    index: u32,
    engine: u16,
    v: V,
    mirrored: *mut bool,
) -> u16 {
    unsafe {
        if s.assertions {
            (s.check)(v.is_null() || (s.vehicle_engine)(v) == engine);
        }
        let e = (s.engine)(engine);
        let mut result = callback(s, CBID_VEHICLE_ARTIC_ENGINE.0, index, engine, v);
        if result == CALLBACK_FAILED {
            return INVALID_ENGINE;
        }
        if (s.version)((s.grf)(e)) < 8 {
            result = gb(u32::from(result), 0, 8) as u16;
            if result == 0xff {
                return INVALID_ENGINE;
            }
            if !mirrored.is_null() {
                mirrored.write(has_bit(result, 7));
            }
            result = gb(u32::from(result), 0, 7) as u16;
        } else {
            if result == 0x7fff {
                return INVALID_ENGINE;
            }
            if !mirrored.is_null() {
                mirrored.write(has_bit(result, 14));
            }
            result = gb(u32::from(result), 0, 14) as u16;
        }
        (s.new_engine)((s.grf)(e), (s.kind)(e), result)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_is(s: &Services, engine: u16) -> bool {
    (s.callbacks)((s.engine)(engine)) & VEHICLE_ARTIC_ENGINE != 0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_count(
    s: &Services,
    engine: u16,
    purchase: bool,
) -> u32 {
    unsafe {
        if !openttd_rust_articulated_is(s, engine) || !(s.pool_available)(1) {
            return 0;
        }
        let v = if purchase {
            null_mut()
        } else {
            let v = (s.new_temporary)();
            (s.set_engine)(v, engine);
            (s.set_owner)(v, (s.current_company)());
            v
        };
        let mut i = 1;
        while i < MAX_ARTICULATED_PARTS {
            if openttd_rust_articulated_next(s, i, engine, v, null_mut()) == INVALID_ENGINE {
                break;
            }
            i += 1;
        }
        if !v.is_null() {
            (s.destroy_temporary)(v);
        }
        i - 1
    }
}
/// A live base shell, retained by its fleet/transaction caller.
pub(crate) unsafe fn can_carry_vehicle(s: &Services, v: V) -> bool {
    unsafe { openttd_rust_engine_can_carry(s, (s.vehicle_engine_object)(v)) }
}
/// `Engine::GetDisplayDefaultCapacity` delegates to the capacity owner in purchase context.
pub(crate) unsafe fn display_default_capacity(s: &Services, e: E, mail: *mut u16) -> u32 {
    unsafe { openttd_rust_engine_capacity(s, e, null_mut(), mail) }
}
/// Original helper narrows the display uint result to uint16 before aggregation.
unsafe fn default_capacity(s: &Services, engine: u16) -> (u8, u16) {
    unsafe {
        let e = (s.engine)(engine);
        let cargo = if openttd_rust_engine_can_carry(s, e) {
            (s.default_cargo)(e)
        } else {
            INVALID_CARGO
        };
        (
            cargo,
            if is_valid_cargo_type(cargo) {
                display_default_capacity(s, e, null_mut()) as u16
            } else {
                0
            },
        )
    }
}
pub(crate) unsafe fn available_cargo(s: &Services, engine: u16, initial: bool) -> u64 {
    unsafe {
        let e = (s.engine)(engine);
        if !openttd_rust_engine_can_carry(s, e) {
            return 0;
        }
        let mut cargoes = (s.refit_mask)(e);
        if initial {
            set_bit64(&mut cargoes, (s.default_cargo)(e));
        }
        cargoes
    }
}
/// Complete query traversal; each caller preserves its original early return.
unsafe fn parts(s: &Services, e: E, engine: u16, mut visit: impl FnMut(u16) -> bool) {
    unsafe {
        if !(s.ground)(e) || (s.callbacks)(e) & VEHICLE_ARTIC_ENGINE == 0 {
            return;
        }
        for i in 1..MAX_ARTICULATED_PARTS {
            let part = openttd_rust_articulated_next(s, i, engine, null_mut(), null_mut());
            if part == INVALID_ENGINE || !visit(part) {
                break;
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_capacity(
    s: &Services,
    engine: u16,
    out: *mut u32,
) {
    unsafe {
        let mut capacity = [0_u32; NUM_CARGO as usize];
        let e = (s.engine)(engine);
        let (cargo, cap) = default_capacity(s, engine);
        if is_valid_cargo_type(cargo) {
            capacity[usize::from(cargo)] = u32::from(cap);
        }
        parts(s, e, engine, |part| {
            let (cargo, cap) = default_capacity(s, part);
            if is_valid_cargo_type(cargo) {
                capacity[usize::from(cargo)] =
                    capacity[usize::from(cargo)].wrapping_add(u32::from(cap));
            }
            true
        });
        std::ptr::copy_nonoverlapping(capacity.as_ptr(), out, NUM_CARGO as usize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_cargoes(s: &Services, engine: u16) -> u64 {
    unsafe {
        let mut cargoes = 0;
        let e = (s.engine)(engine);
        let (cargo, cap) = default_capacity(s, engine);
        if is_valid_cargo_type(cargo) && cap > 0 {
            set_bit64(&mut cargoes, cargo);
        }
        parts(s, e, engine, |part| {
            let (cargo, cap) = default_capacity(s, part);
            if is_valid_cargo_type(cargo) && cap > 0 {
                set_bit64(&mut cargoes, cargo);
            }
            true
        });
        cargoes
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_refittable(s: &Services, engine: u16) -> bool {
    unsafe {
        if openttd_rust_engine_refittable(s, engine) {
            return true;
        }
        let e = (s.engine)(engine);
        let mut refittable = false;
        parts(s, e, engine, |part| {
            refittable = openttd_rust_engine_refittable(s, part);
            !refittable
        });
        refittable
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_refit_masks(
    s: &Services,
    engine: u16,
    initial: bool,
    union: *mut u64,
    intersection: *mut u64,
) {
    unsafe {
        let e = (s.engine)(engine);
        let cargoes = available_cargo(s, engine, initial);
        union.write(cargoes);
        intersection.write(if cargoes == 0 {
            ALL_CARGOTYPES
        } else {
            cargoes
        });
        parts(s, e, engine, |part| {
            let cargoes = available_cargo(s, part, initial);
            union.write(union.read() | cargoes);
            if cargoes != 0 {
                intersection.write(intersection.read() & cargoes);
            }
            true
        });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_refit_union(
    s: &Services,
    engine: u16,
    initial: bool,
) -> u64 {
    let mut union = 0;
    let mut intersection = 0;
    unsafe {
        openttd_rust_articulated_refit_masks(
            s,
            engine,
            initial,
            &raw mut union,
            &raw mut intersection,
        );
    };
    union
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_articulated_live_cargoes(
    s: &Services,
    mut v: V,
    mut cargo_out: *mut u8,
) -> u64 {
    unsafe {
        let mut cargoes = 0;
        let mut first = INVALID_CARGO;
        loop {
            let cargo = (s.vehicle_cargo)(v);
            if is_valid_cargo_type(cargo)
                && openttd_rust_engine_can_carry(s, (s.vehicle_engine_object)(v))
            {
                set_bit64(&mut cargoes, cargo);
                if !is_valid_cargo_type(first) {
                    first = (s.vehicle_cargo)(v);
                }
                if first != (s.vehicle_cargo)(v) && !cargo_out.is_null() {
                    cargo_out.write(INVALID_CARGO);
                    cargo_out = null_mut();
                }
            }
            v = if (s.has_articulated)(v) {
                (s.next_articulated)(v)
            } else {
                null_mut()
            };
            if v.is_null() {
                break;
            }
        }
        if !cargo_out.is_null() {
            cargo_out.write(first);
        }
        cargoes
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEngineCapabilitiesServices" ["probe"] => Services {
        resolver, engine, engine_if_valid, kind, ground, id, default_cargo, callbacks, carrying, capacity_config, refit_mask,
        ship_capacity, passenger_capacity, mail_capacity, rail_kind,
        grf, version, new_engine, cargo_classes, multiplier, pool_available,
        new_temporary, destroy_temporary, set_engine, set_owner, current_company, vehicle_engine,
        vehicle_engine_object, vehicle_cargo, vehicle_subtype, has_articulated, next_articulated, check, assertions,
    };
}

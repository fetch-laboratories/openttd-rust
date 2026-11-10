/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Articulated allocation, assembly and purchase-list consistency control.
#![allow(clippy::many_single_char_names)]
use super::refit;
use crate::vehicle::{OpenTTDEngine, VehicleType};
use std::mem::MaybeUninit;
#[repr(C)]
pub struct Services {
    pub refit: &'static refit::Services,
    pub prepare_previous: extern "C" fn(*mut ()) -> *mut (),
    pub new_train: extern "C" fn() -> *mut (),
    pub new_road: extern "C" fn() -> *mut (),
    pub road_length: extern "C" fn(*mut ()),
    pub copy_roadtype: extern "C" fn(*mut (), *mut ()),
    pub set_part: extern "C" fn(*mut (), u8, u8, u16, bool),
    pub mark_train: extern "C" fn(*mut ()),
    pub mark_road: extern "C" fn(*mut ()),
    pub copy_common: extern "C" fn(*mut (), *mut (), u16),
    pub random_bits: extern "C" fn(*mut (), u16),
    pub flip_image: extern "C" fn(*mut ()),
    pub warning: extern "C" fn(*const OpenTTDEngine),
    pub has_articulated: extern "C" fn(*mut ()) -> bool,
    pub next_articulated: extern "C" fn(*mut ()) -> *mut (),
}
impl Services {
    /// Caller retains the live native shell through this synchronous update.
    pub(crate) unsafe fn position(&self, v: *mut ()) {
        unsafe {
            crate::vehicle::occupancy::update(
                self.refit.common.vehicles,
                super::vehicle_ref(v, self.refit.common.vehicles),
                false,
            );
        }
    }
}
/// # Safety
/// A live head and immutable native specification/pool services. Every short
/// train/road owner access ends before allocation, callback or position work.
#[allow(clippy::too_many_lines)]
pub(crate) unsafe fn add(s: &Services, first: *mut ()) {
    let r = s.refit;
    let c = r.common;
    let g = c.base.groups;
    let kind = VehicleType(crate::fleet::record(g, first).vehicle_type());
    if !unsafe {
        crate::engine_capabilities::openttd_rust_articulated_is_abi(
            r.capabilities,
            crate::fleet::record(g, first).engine_id(),
        ) != 0
    } {
        return;
    }
    let mut previous = first;
    for index in 1..crate::engine_capabilities::MAX_ARTICULATED_PARTS {
        let mut flip = MaybeUninit::uninit();
        let engine = unsafe {
            crate::engine_capabilities::openttd_rust_articulated_next(
                r.capabilities,
                index,
                crate::fleet::record(g, first).engine_id(),
                super::vehicle_ref(first, c.vehicles).0.as_ptr(),
                flip.as_mut_ptr(),
            )
        };
        if engine == crate::engine_type::INVALID_ENGINE {
            return;
        }
        if !(s.refit.common.vehicles.can_allocate)(1) {
            return;
        }
        crate::witness::hit(crate::witness::VT_ARTICULATED);
        let cache = (s.prepare_previous)(previous);
        let e = (r.engine)(engine);
        let v = match kind {
            VehicleType::TRAIN => {
                let v = (s.new_train)();
                // SAFETY: New and previous shells are live through assembly.
                unsafe {
                    c.vehicles.set_next_shell(previous.cast(), v.cast());
                    (*(c.vehicles.record_from_shell)(v.cast())).subtype = 0;
                }
                let front = r.train_handle(
                    unsafe { super::vehicle_ref(first, r.common.vehicles) }
                        .0
                        .as_ptr(),
                );
                let train = r.train_handle(
                    unsafe { super::vehicle_ref(v, r.common.vehicles) }
                        .0
                        .as_ptr(),
                );
                train.set_track(u64::from(front.track()));
                train.set_railtypes(front.get_railtypes());
                v
            }
            VehicleType::ROAD => {
                let v = (s.new_road)();
                // SAFETY: New and previous shells are live through assembly.
                unsafe {
                    c.vehicles.set_next_shell(previous.cast(), v.cast());
                    (*(c.vehicles.record_from_shell)(v.cast())).subtype = 0;
                }
                (s.road_length)(cache);
                unsafe {
                    crate::road::set_depot_state((r.road_state)(
                        super::vehicle_ref(v, r.common.vehicles).0.as_ptr(),
                    ));
                };
                (s.copy_roadtype)(v, first);
                v
            }
            _ => unreachable!("articulated vehicles are ground vehicles"),
        };
        let can_carry = unsafe {
            crate::engine_capabilities::openttd_rust_engine_can_carry_abi(r.capabilities, e) != 0
        };
        let specification = crate::vehicle::EngineRef::of(e);
        let image = specification.ground_image(kind);
        let capacity = if can_carry {
            u16::from(specification.ground_capacity(kind))
        } else {
            0
        };
        let cargo = if can_carry {
            specification.default_cargo()
        } else {
            unsafe { (c.transactions.cargo_type)(first) }
        };
        // The road-only assertion remains in its native field-store leaf.
        (s.set_part)(
            v,
            image,
            cargo,
            capacity,
            kind == VehicleType::ROAD && can_carry,
        );
        if kind == VehicleType::TRAIN {
            (s.mark_train)(v);
        } else {
            (s.mark_road)(v);
        }
        (s.copy_common)(v, first, engine);
        (s.random_bits)(v, c.base.shared.random() as u16);
        if unsafe { flip.assume_init() } {
            (s.flip_image)(v);
        }
        if crate::fleet::record(g, v).vehicle_type() == VehicleType::TRAIN.0 {
            let probability = unsafe {
                crate::newgrf_vehicle::openttd_rust_vehicle_build_probability(
                    &*(r.common.vehicles.newgrf)(),
                    super::vehicle_ref(v, c.vehicles).0.as_ptr(),
                    crate::fleet::record(g, v).engine_id(),
                    crate::newgrf_vehicle::BUILD_PROBABILITY_REVERSED,
                )
            };
            if probability >= 0 {
                r.train_handle(
                    unsafe { super::vehicle_ref(v, r.common.vehicles) }
                        .0
                        .as_ptr(),
                )
                .set_flag(crate::train::FLIPPED, probability != 0);
            }
        }
        unsafe {
            s.position(v);
        }
        previous = v;
    }
}
/// # Safety
/// A live freshly built articulated chain; callbacks keep its shells alive.
pub(crate) unsafe fn consistency(s: &Services, mut v: *mut ()) {
    let r = s.refit;
    let c = r.common;
    let e = r.vehicle_engine(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    let mut purchase_union = 0;
    let mut purchase_intersection = 0;
    unsafe {
        crate::engine_capabilities::openttd_rust_articulated_refit_masks(
            r.capabilities,
            crate::fleet::record(c.base.groups, v).engine_id(),
            true,
            &raw mut purchase_union,
            &raw mut purchase_intersection,
        );
    };
    let mut purchase_capacity = [0_u32; 64];
    unsafe {
        crate::engine_capabilities::openttd_rust_articulated_capacity(
            r.capabilities,
            crate::fleet::record(c.base.groups, v).engine_id(),
            purchase_capacity.as_mut_ptr(),
        );
    };
    let mut real_union = 0;
    let mut real_intersection = crate::cargo_type::ALL_CARGOTYPES;
    let mut real_cargo = 0_u64;
    loop {
        let mask = unsafe {
            crate::engine_capabilities::available_cargo(
                r.capabilities,
                crate::fleet::record(c.base.groups, v).engine_id(),
                true,
            )
        };
        real_union |= mask;
        if mask != 0 {
            real_intersection &= mask;
        }
        let cargo = unsafe { (c.transactions.cargo_type)(v) };
        if c.transactions.assertions {
            unsafe { (c.transactions.check)(cargo < c.strings.cargo_count) };
        }
        if unsafe { super::vehicle_ref(v, r.common.vehicles) }.cargo_cap() > 0 {
            real_cargo |= 1_u64.wrapping_shl(u32::from(cargo));
        }
        v = if (s.has_articulated)(v) {
            (s.next_articulated)(v)
        } else {
            std::ptr::null_mut()
        };
        if v.is_null() {
            break;
        }
    }
    let mut more = false;
    while real_cargo != 0 {
        let cargo = real_cargo.trailing_zeros() as usize;
        if purchase_capacity[cargo] == 0 {
            more = true;
            break;
        }
        real_cargo &= real_cargo - 1;
    }
    if real_union != purchase_union || real_intersection != purchase_intersection || more {
        (s.warning)(e);
    }
}
/// # Safety
/// Same synchronous live-chain contract as `add`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_add_articulated(s: &Services, first: *mut ()) {
    unsafe { add(s, first) };
}
/// # Safety
/// Same synchronous live-chain contract as `consistency`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_articulated_consistency(
    s: &Services,
    first: *mut (),
) {
    unsafe { consistency(s, first) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleArticulatedServices" ["probe"] => Services {
        refit, prepare_previous,
        new_train, new_road, road_length, copy_roadtype, set_part, mark_train,
        mark_road, copy_common, random_bits, flip_image, warning,
        has_articulated, next_articulated,
    };
}

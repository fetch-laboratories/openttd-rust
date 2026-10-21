/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Articulated allocation, assembly and purchase-list consistency control.
#![allow(clippy::many_single_char_names)]
use super::refit;
use crate::vehicle::{OpenTTDEngine, VehicleType};
use std::mem::MaybeUninit;
#[repr(C)]
pub struct Part {
    pub image: u8,
    pub capacity: u16,
}
#[repr(C)]
pub struct Probability {
    pub present: bool,
    pub value: bool,
}
#[repr(C)]
pub struct Services {
    pub refit: &'static refit::Services,
    pub default_cargo: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub capacity: extern "C" fn(u16, *mut u32),
    pub pool_available: extern "C" fn(usize) -> bool,
    pub has_callback: extern "C" fn(u16) -> bool,
    pub next_part: extern "C" fn(u32, u16, *mut (), *mut bool) -> u16,
    pub prepare_previous: extern "C" fn(*mut ()) -> *mut (),
    pub new_train: extern "C" fn(*mut ()) -> *mut (),
    pub new_road: extern "C" fn(*mut ()) -> *mut (),
    pub road_length: extern "C" fn(*mut ()),
    pub copy_roadtype: extern "C" fn(*mut (), *mut ()),
    pub train_info: extern "C" fn(*const OpenTTDEngine, bool) -> Part,
    pub road_info: extern "C" fn(*const OpenTTDEngine, bool) -> Part,
    pub set_part: extern "C" fn(*mut (), u8, u8, u16, bool),
    pub mark_train: extern "C" fn(*mut ()),
    pub mark_road: extern "C" fn(*mut ()),
    pub copy_common: extern "C" fn(*mut (), *mut (), u16),
    pub random_bits: extern "C" fn(*mut (), u16),
    pub flip_image: extern "C" fn(*mut ()),
    pub reverse: extern "C" fn(*mut (), u16) -> Probability,
    pub position: extern "C" fn(*mut ()),
    pub available_cargo: extern "C" fn(u16, bool) -> u64,
    pub warning: extern "C" fn(*const OpenTTDEngine),
    pub has_articulated: extern "C" fn(*mut ()) -> bool,
    pub next_articulated: extern "C" fn(*mut ()) -> *mut (),
    pub max_parts: u32,
    pub all_cargo: u64,
}
/// # Safety
/// A live head and immutable native specification/pool services. Every short
/// train/road owner access ends before allocation, callback or position work.
#[allow(clippy::too_many_lines)]
pub(crate) unsafe fn add(s: &Services, first: *mut ()) {
    let r = s.refit;
    let c = r.common;
    let g = c.base.groups;
    let kind = VehicleType(unsafe { (g.vehicle_type)(first) });
    if !(s.has_callback)(unsafe { (g.vehicle_engine)(first) }) {
        return;
    }
    let mut previous = first;
    for index in 1..s.max_parts {
        let mut flip = MaybeUninit::uninit();
        let engine = (s.next_part)(
            index,
            unsafe { (g.vehicle_engine)(first) },
            first,
            flip.as_mut_ptr(),
        );
        if engine == crate::vehicle::INVALID_ENGINE {
            return;
        }
        if !(s.pool_available)(1) {
            return;
        }
        let cache = (s.prepare_previous)(previous);
        let e = (r.engine)(engine);
        let v = match kind {
            VehicleType::TRAIN => {
                let v = (s.new_train)(previous);
                let front = (r.train_handle)(first.cast());
                let train = (r.train_handle)(v.cast());
                train.set_track(u64::from(front.track()));
                train.set_railtypes(front.get_railtypes());
                v
            }
            VehicleType::ROAD => {
                let v = (s.new_road)(previous);
                (s.road_length)(cache);
                unsafe { crate::road::set_depot_state((r.road_state)(v.cast())) };
                (s.copy_roadtype)(v, first);
                v
            }
            _ => unreachable!("articulated vehicles are ground vehicles"),
        };
        let can_carry = (r.can_carry)(e);
        let info = if kind == VehicleType::TRAIN {
            (s.train_info)(e, can_carry)
        } else {
            (s.road_info)(e, can_carry)
        };
        let cargo = if can_carry {
            (s.default_cargo)(e)
        } else {
            unsafe { (c.transactions.cargo_type)(first) }
        };
        // The road-only assertion remains in its native field-store leaf.
        (s.set_part)(
            v,
            info.image,
            cargo,
            info.capacity,
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
        if unsafe { (g.vehicle_type)(v) } == 0 {
            let probability = (s.reverse)(v, unsafe { (g.vehicle_engine)(v) });
            if probability.present {
                (r.train_handle)(v.cast()).set_flag(crate::train::FLIPPED, probability.value);
            }
        }
        (s.position)(v);
        previous = v;
    }
}
/// # Safety
/// A live freshly built articulated chain; callbacks keep its shells alive.
pub(crate) unsafe fn consistency(s: &Services, mut v: *mut ()) {
    let r = s.refit;
    let c = r.common;
    let e = (r.vehicle_engine)(v.cast());
    let mut purchase_union = 0;
    let mut purchase_intersection = 0;
    unsafe {
        (c.transactions.refit_masks)(
            (c.base.groups.vehicle_engine)(v),
            &raw mut purchase_union,
            &raw mut purchase_intersection,
        );
    };
    let mut purchase_capacity = [0_u32; 64];
    (s.capacity)(
        unsafe { (c.base.groups.vehicle_engine)(v) },
        purchase_capacity.as_mut_ptr(),
    );
    let mut real_union = 0;
    let mut real_intersection = s.all_cargo;
    let mut real_cargo = 0_u64;
    loop {
        let mask = (s.available_cargo)(unsafe { (c.base.groups.vehicle_engine)(v) }, true);
        real_union |= mask;
        if mask != 0 {
            real_intersection &= mask;
        }
        let cargo = unsafe { (c.transactions.cargo_type)(v) };
        if c.transactions.assertions {
            unsafe { (c.transactions.check)(cargo < c.strings.cargo_count) };
        }
        if (r.cargo_cap)(v.cast()) > 0 {
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

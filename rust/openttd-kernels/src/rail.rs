/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `rail.h` helpers and complete rail introduction/construction availability.
#![allow(unsafe_code)]
use crate::rail_type::RAILTYPE_END;
use crate::services::Services as Shared;
use crate::timer_game_common::MAX_DATE;
use crate::type_availability::{Services, company_available, services};
use crate::vehicle::{RailVehicleTypes, VehicleType};

pub fn get_all_introduces_rail_types(s: &Services, mut types: u64) -> u64 {
    let mut introduces = 0;
    while types != 0 {
        let rt = crate::base_bitset_type::get_nth_set_bit(types, 0).unwrap_or(0);
        types &= types - 1;
        introduces |= (s.rail_introduces)(rt);
    }
    introduces
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_rail_type(s: &Services, company: u8, rt: u8) -> u8 {
    u8::from(has_rail_type(s, company, rt))
}
/// `HasRailTypeAvail`, retaining the original hidden-mask short circuit.
pub fn has_rail_type(s: &Services, company: u8, rt: u8) -> bool {
    let bit = 1u64.wrapping_shl(u32::from(rt));
    crate::type_registry::rail_hidden() & bit == 0 && (s.company_railtypes)(company) & bit != 0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_any_rail_types(s: &Services, company: u8) -> bool {
    let available = (s.company_railtypes)(company);
    available & !crate::type_registry::rail_hidden() != 0
}
pub fn val_param_rail_type(shared: &Shared, rt: u8) -> bool {
    rt < RAILTYPE_END && has_rail_type(services(shared), (shared.current_company)(), rt)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_val_rail_type(shared: &Shared, rt: u8) -> u8 {
    u8::from(val_param_rail_type(shared, rt))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_date_rail_types(
    s: &Services,
    mut current: u64,
    date: i32,
) -> u64 {
    loop {
        let mut types = current;
        for rt in 0..RAILTYPE_END {
            let info = (s.rail_introduction)(rt);
            if info.label == 0 || !(0..MAX_DATE).contains(&info.date) || info.date > date {
                continue;
            }
            if types & info.required != info.required {
                continue;
            }
            types |= info.introduces;
        }
        if types == current {
            return types;
        }
        current = types;
    }
}
// Whole engine traversal stays in Rust, with ascending IDs and original gates.
fn engine_types(shared: &Shared, s: &Services, company: Option<u8>, introduces: bool) -> u64 {
    let mut types = 0;
    let mut from = 0;
    loop {
        // SAFETY: Native pool leaves resolve one live shell; no borrow crosses calls.
        let e = unsafe { (s.next)(from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind != VehicleType::TRAIN {
            continue;
        }
        let info = unsafe { (s.rail_availability)(e.shell) };
        if !info.climate {
            continue;
        }
        if let Some(company) = company
            && !unsafe { company_available(e, company, shared) }
        {
            continue;
        }
        if info.kind == RailVehicleTypes::WAGON {
            continue;
        }
        let rails = info.railtypes;
        if s.assertions {
            assert_ne!(rails, 0);
        }
        types |= if introduces {
            get_all_introduces_rail_types(s, rails)
        } else {
            rails
        };
    }
    types
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_rail_types(
    shared: &Shared,
    s: &Services,
    company: u8,
    introduces: bool,
) -> u64 {
    let types = engine_types(shared, s, Some(company), introduces);
    if introduces {
        openttd_rust_add_date_rail_types(s, types, crate::timer_game_common::calendar_date().0)
    } else {
        types
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_types(shared: &Shared, s: &Services, introduces: bool) -> u64 {
    let types = engine_types(shared, s, None, introduces);
    if introduces {
        openttd_rust_add_date_rail_types(s, types, MAX_DATE)
    } else {
        types
    }
}

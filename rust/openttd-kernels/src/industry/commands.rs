/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry metadata commands and native shell ownership changes.
//! Lookups resolve once; command checks and writes follow the native source order.
use super::{Industry, Services, ShellHandle};
use crate::services::ByteSpan;

#[repr(C)]
pub struct IndustryCommandServices {
    pub deity: unsafe extern "C" fn() -> bool,
    pub get: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub owner: unsafe extern "C" fn(*mut ShellHandle) -> *mut Industry,
    pub valid_company: unsafe extern "C" fn(u8) -> bool,
    pub invalidate_view: unsafe extern "C" fn(u16),
}
/// # Safety
/// Live synchronous pool services; the owner is resolved only for an executing
/// command and no reference to its fields survives a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_flags_command(
    id: u16,
    execute: bool,
    value: u8,
    services: *const IndustryCommandServices,
) -> bool {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return false;
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() || value & !15 != 0 {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            (*owner).fields.ctlflags = value;
        }
    }
    true
}
/// # Safety
/// Live synchronous services. The original post-assignment news comparison is
/// always false; production multiplier leaves cannot mutate the requested level.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_production_command(
    id: u16,
    execute: bool,
    level: u8,
    services: *const IndustryCommandServices,
    production: *const Services,
) -> bool {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } || !(4..=128).contains(&level) {
        return false;
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            (*owner).fields.ctlflags |= 8;
            (*owner).fields.prod_level = level;
        }
        super::recompute(shell.cast(), owner, unsafe { &*production });
    }
    true
}
/// # Safety
/// Synchronous native pool and company lookup; owner mutation has no reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_exclusivity_command(
    id: u16,
    execute: bool,
    company: u8,
    consumer: bool,
    services: *const IndustryCommandServices,
) -> bool {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return false;
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return false;
    }
    if company != 16 && company != 255 && company != 18 && !unsafe { (s.valid_company)(company) } {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            if consumer {
                (*owner).metadata.exclusive_consumer = company;
            } else {
                (*owner).metadata.exclusive_supplier = company;
            }
        }
    }
    true
}
/// # Safety
/// Borrowed full encoded byte span remains live through this synchronous call.
/// Native UI invalidation runs only after the owner text mutation ends.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_text_command(
    id: u16,
    execute: bool,
    text: ByteSpan,
    services: *const IndustryCommandServices,
) -> bool {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return false;
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            super::openttd_rust_industry_set_text(owner, text.data, text.size);
            (s.invalidate_view)(id);
        }
    }
    true
}
/// # Safety
/// Live owner, with no callbacks or surviving reference to metadata.
pub(crate) unsafe fn openttd_rust_industry_change_owner(owner: *mut Industry, old: u8, new: u8) {
    unsafe {
        if (*owner).metadata.founder == old {
            (*owner).metadata.founder = if new == 255 { 16 } else { new };
        }
        if (*owner).metadata.exclusive_supplier == old {
            (*owner).metadata.exclusive_supplier = new;
        }
        if (*owner).metadata.exclusive_consumer == old {
            (*owner).metadata.exclusive_consumer = new;
        }
    }
}

/// Native-owned complete stack `CommandCost`, never a reconstructed cost snapshot.
#[repr(C)]
pub struct IndustryCost {
    _private: [u8; 0],
}
/// Native stack cost operations consumed by industry policy.
#[repr(C)]
pub struct IndustryCostServices {
    pub zero: unsafe extern "C" fn(*mut IndustryCost),
    pub error: unsafe extern "C" fn(*mut IndustryCost, u32),
    pub expense_money: unsafe extern "C" fn(*mut IndustryCost, u8, i64),
    pub success: unsafe extern "C" fn(*const IndustryCost) -> bool,
}

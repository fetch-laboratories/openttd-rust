/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry metadata commands and native shell ownership changes.
//! Lookups resolve once; command checks and writes follow the native source order.
use super::{Industry, ShellHandle};
use super::{IndustryControlFlag, IndustryControlFlags, PRODLEVEL_MAXIMUM, PRODLEVEL_MINIMUM};
use crate::company_type::{CompanyID, Owner};
use crate::services::ByteSpan;

#[repr(C)]
pub struct IndustryCommandServices {
    pub deity: unsafe extern "C" fn() -> bool,
    pub get: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub owner: unsafe extern "C" fn(*mut ShellHandle) -> *mut Industry,
    pub valid_company: unsafe extern "C" fn(u8) -> bool,
    pub invalidate_view: unsafe extern "C" fn(u16),
}
/// Original `CmdIndustrySetFlags`: set industry control flags.
///
/// # Safety
/// Live synchronous pool services; the owner is resolved only for an executing
/// command and no reference to its fields survives a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_flags_command_abi(
    id: u16,
    execute: bool,
    value: u8,
    services: *const IndustryCommandServices,
) -> u8 {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return u8::from(false);
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() || !IndustryControlFlags(value).is_valid() {
        return u8::from(false);
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            (*owner).fields.ctlflags = value;
        }
    }
    u8::from(true)
}
/// Original `CmdIndustrySetProduction`: set industry production.
///
/// # Safety
/// Live synchronous services. The original post-assignment news comparison is
/// always false; production multiplier leaves cannot mutate the requested level.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_production_command_abi(
    id: u16,
    execute: bool,
    level: u8,
    services: *const IndustryCommandServices,
) -> u8 {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } || !(PRODLEVEL_MINIMUM..=PRODLEVEL_MAXIMUM).contains(&level) {
        return u8::from(false);
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return u8::from(false);
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            (*owner).fields.ctlflags |=
                IndustryControlFlags::from_element(IndustryControlFlag::ExternalProdLevel).0;
            (*owner).fields.prod_level = level;
        }
        super::recompute_production_multipliers(owner);
        // Show news message if requested: `prod_level != ind->prod_level` is false after the
        // assignment above, so the original adds no news item.
    }
    u8::from(true)
}
/// Original `CmdIndustrySetExclusivity`: change exclusive consumer or supplier for the
/// industry. `company` is the company to set, or `INVALID_OWNER` (available to everyone),
/// `OWNER_NONE` (neutral stations only) or `OWNER_DEITY` (no one).
///
/// # Safety
/// Synchronous native pool and company lookup; owner mutation has no reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_exclusivity_command_abi(
    id: u16,
    execute: bool,
    company: u8,
    consumer: bool,
    services: *const IndustryCommandServices,
) -> u8 {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return u8::from(false);
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return u8::from(false);
    }
    let company_id = CompanyID(company);
    if company_id != Owner::NONE
        && company_id != Owner::INVALID
        && company_id != Owner::DEITY
        && !unsafe { (s.valid_company)(company_id.0) }
    {
        return u8::from(false);
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            if consumer {
                (*owner).metadata.exclusive_consumer = company_id.0;
            } else {
                (*owner).metadata.exclusive_supplier = company_id.0;
            }
        }
    }
    u8::from(true)
}
/// Original `CmdIndustrySetText`: change additional industry text.
///
/// # Safety
/// Borrowed full encoded byte span remains live through this synchronous call.
/// Native UI invalidation runs only after the owner text mutation ends.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_text_command_abi(
    id: u16,
    execute: bool,
    text: ByteSpan,
    services: *const IndustryCommandServices,
) -> u8 {
    let s = unsafe { &*services };
    if !unsafe { (s.deity)() } {
        return u8::from(false);
    }
    let shell = unsafe { (s.get)(id) };
    if shell.is_null() {
        return u8::from(false);
    }
    if execute {
        let owner = unsafe { (s.owner)(shell) };
        unsafe {
            super::openttd_rust_industry_set_text(owner, text.data, text.size);
            (s.invalidate_view)(id);
        }
    }
    u8::from(true)
}
/// Original `ChangeTileOwner_Industry`, after its `Industry::GetByTile` lookup.
///
/// # Safety
/// Live owner, with no callbacks or surviving reference to metadata.
pub(crate) unsafe fn change_tile_owner_industry(
    owner: *mut Industry,
    old_owner: Owner,
    new_owner: Owner,
) {
    unsafe {
        // If the founder merges, the industry was created by the merged company
        if CompanyID((*owner).metadata.founder) == old_owner {
            (*owner).metadata.founder = if new_owner == Owner::INVALID {
                Owner::NONE
            } else {
                new_owner
            }
            .0;
        }
        if CompanyID((*owner).metadata.exclusive_supplier) == old_owner {
            (*owner).metadata.exclusive_supplier = new_owner.0;
        }
        if CompanyID((*owner).metadata.exclusive_consumer) == old_owner {
            (*owner).metadata.exclusive_consumer = new_owner.0;
        }
    }
}

/// Native-owned stack `CommandCost`.
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

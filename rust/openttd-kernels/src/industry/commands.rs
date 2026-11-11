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
use crate::industry_type::IndustryID;
use crate::services::ByteSpan;

#[repr(C)]
pub struct IndustryCommandServices {
    pub shared: *const crate::services::Services,
    pub get: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub owner: unsafe extern "C" fn(*mut ShellHandle) -> *mut Industry,
    pub valid_company: unsafe extern "C" fn(u8) -> bool,
    pub invalidate_view: unsafe extern "C" fn(u16),
}
/// `_current_company == OWNER_DEITY`.
fn deity(s: &IndustryCommandServices) -> bool {
    // SAFETY: The shared table is immutable for the process lifetime.
    (unsafe { &*s.shared }.current_company)() == crate::company_type::OWNER_DEITY
}
/// `CmdIndustrySetFlags`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdIndustrySetFlags {
    /// `IndustryID`.
    pub(crate) ind_id: IndustryID,
    /// `IndustryControlFlags`.
    pub(crate) ctlflags: IndustryControlFlags,
}
/// Original `CmdIndustrySetFlags`: set industry control flags. `execute` is
/// `flags.Test(DoCommandFlag::Execute)`; returns false for `CMD_ERROR`.
pub(crate) fn cmd_industry_set_flags(
    execute: bool,
    args: CmdIndustrySetFlags,
    s: &IndustryCommandServices,
) -> bool {
    if !deity(s) {
        return false;
    }
    let ind = unsafe { (s.get)(args.ind_id.0) };
    if ind.is_null() {
        return false;
    }
    if !args.ctlflags.is_valid() {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(ind) };
        unsafe {
            (*owner).fields.ctlflags = args.ctlflags.0;
        }
    }
    true
}
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
    u8::from(cmd_industry_set_flags(
        execute,
        CmdIndustrySetFlags {
            ind_id: IndustryID(id),
            ctlflags: IndustryControlFlags(value),
        },
        unsafe { &*services },
    ))
}
/// `CmdIndustrySetProduction`'s arguments. The news arguments are unused: the original's
/// news comparison follows the assignment and is always false.
#[derive(Clone, Copy)]
pub(crate) struct CmdIndustrySetProduction {
    /// `IndustryID`.
    pub(crate) ind_id: IndustryID,
    /// Production level.
    pub(crate) prod_level: u8,
}
/// Original `CmdIndustrySetProduction`: set industry production. `execute` is
/// `flags.Test(DoCommandFlag::Execute)`; returns false for `CMD_ERROR`.
pub(crate) fn cmd_industry_set_production(
    execute: bool,
    args: CmdIndustrySetProduction,
    s: &IndustryCommandServices,
    production: &super::Services,
) -> bool {
    if !deity(s) {
        return false;
    }
    if args.prod_level < PRODLEVEL_MINIMUM || args.prod_level > PRODLEVEL_MAXIMUM {
        return false;
    }
    let ind = unsafe { (s.get)(args.ind_id.0) };
    if ind.is_null() {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(ind) };
        unsafe {
            (*owner).fields.ctlflags |=
                IndustryControlFlags::from_element(IndustryControlFlag::ExternalProdLevel).0;
            (*owner).fields.prod_level = args.prod_level;
        }
        super::recompute_production_multipliers_checked(owner, production);
        /* Show news message if requested. */
        // `prod_level != ind->prod_level` is false after the assignment above, so the
        // original adds no news item.
    }
    true
}
/// # Safety
/// Live synchronous services and production tables. Production multiplier leaves
/// cannot mutate the requested level.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_set_production_command_abi(
    id: u16,
    execute: bool,
    level: u8,
    services: *const IndustryCommandServices,
    production: *const super::Services,
) -> u8 {
    u8::from(cmd_industry_set_production(
        execute,
        CmdIndustrySetProduction {
            ind_id: IndustryID(id),
            prod_level: level,
        },
        unsafe { &*services },
        unsafe { &*production },
    ))
}
/// `CmdIndustrySetExclusivity`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdIndustrySetExclusivity {
    /// `IndustryID`.
    pub(crate) ind_id: IndustryID,
    /// `CompanyID` to set or `INVALID_OWNER` (available to everyone) or `OWNER_NONE`
    /// (neutral stations only) or `OWNER_DEITY` (no one).
    pub(crate) company_id: Owner,
    /// Set exclusive consumer if true, supplier if false.
    pub(crate) consumer: bool,
}
/// Original `CmdIndustrySetExclusivity`: change exclusive consumer or supplier for the
/// industry. `execute` is `flags.Test(DoCommandFlag::Execute)`; returns false for
/// `CMD_ERROR`.
pub(crate) fn cmd_industry_set_exclusivity(
    execute: bool,
    args: CmdIndustrySetExclusivity,
    s: &IndustryCommandServices,
) -> bool {
    if !deity(s) {
        return false;
    }
    let ind = unsafe { (s.get)(args.ind_id.0) };
    if ind.is_null() {
        return false;
    }
    let company_id = args.company_id;
    if company_id != Owner::NONE
        && company_id != Owner::INVALID
        && company_id != Owner::DEITY
        && !unsafe { (s.valid_company)(company_id.0) }
    {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(ind) };
        unsafe {
            if args.consumer {
                (*owner).metadata.exclusive_consumer = company_id.0;
            } else {
                (*owner).metadata.exclusive_supplier = company_id.0;
            }
        }
    }
    true
}
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
    u8::from(cmd_industry_set_exclusivity(
        execute,
        CmdIndustrySetExclusivity {
            ind_id: IndustryID(id),
            company_id: CompanyID(company),
            consumer,
        },
        unsafe { &*services },
    ))
}
/// `CmdIndustrySetText`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdIndustrySetText {
    /// `IndustryID`.
    pub(crate) ind_id: IndustryID,
    /// Additional industry text, as its full encoded bytes.
    pub(crate) text: ByteSpan,
}
/// Original `CmdIndustrySetText`: change additional industry text. `execute` is
/// `flags.Test(DoCommandFlag::Execute)`; returns false for `CMD_ERROR`.
pub(crate) fn cmd_industry_set_text(
    execute: bool,
    args: CmdIndustrySetText,
    s: &IndustryCommandServices,
) -> bool {
    if !deity(s) {
        return false;
    }
    let ind = unsafe { (s.get)(args.ind_id.0) };
    if ind.is_null() {
        return false;
    }
    if execute {
        let owner = unsafe { (s.owner)(ind) };
        unsafe {
            super::set_text(owner, args.text);
            (s.invalidate_view)(args.ind_id.0);
        }
    }
    true
}
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
    u8::from(cmd_industry_set_text(
        execute,
        CmdIndustrySetText {
            ind_id: IndustryID(id),
            text,
        },
        unsafe { &*services },
    ))
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

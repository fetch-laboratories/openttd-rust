/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `autoreplace_func.h`: a company's engine replacement lookups, over `fleet`'s
//! `EngineReplacement`. The company is a live shell of the serialized game state.

use crate::company::OpenTTDCompany;
use crate::company_identity::Services;
use crate::engine_type::EngineID;
use crate::group_type::GroupID;

/// `EngineReplacementForCompany`: retrieve the engine replacement for the given company and
/// original engine type. `group` is the group related to this replacement;
/// `replace_when_old` is set to true if the replacement should be done when old. Returns the
/// engine type to replace with, or `EngineID::INVALID` if no replacement is in the list.
#[allow(
    unsafe_code,
    reason = "the renewal list and out-value cross as pointers"
)]
pub(crate) fn engine_replacement_for_company(
    companies: &Services,
    c: *const OpenTTDCompany,
    engine: EngineID,
    group: GroupID,
    replace_when_old: Option<&mut bool>,
) -> EngineID {
    let replace_when_old = replace_when_old.map_or(std::ptr::null_mut(), std::ptr::from_mut);
    let head = (companies.renewal_head)(c);
    // SAFETY: `c->RenewalList()`: the live company's renewal owner holds the Rust-owned list
    // head, read in place; the group services and the list are live for the call.
    EngineID(unsafe {
        crate::fleet::engine_replacement(
            companies.groups,
            head.cast::<*mut ()>().read(),
            engine.0,
            group.0,
            replace_when_old,
        )
    })
}

/// `EngineHasReplacementForCompany`: check if a company has a replacement set up for the given
/// engine. `group` is the group related to this replacement.
pub(crate) fn engine_has_replacement_for_company(
    companies: &Services,
    c: *const OpenTTDCompany,
    engine: EngineID,
    group: GroupID,
) -> bool {
    engine_replacement_for_company(companies, c, engine, group, None) != EngineID::INVALID
}

/// `EngineReplacement` over a two-rule renewal list: a group's own rule, the `ALL_GROUP`
/// fallback that a group's replace protection refuses, and `replace_when_old`. Expectations
/// are `autoreplace.cpp`'s.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::{GroupFields, GroupServices, Renew};
    use crate::group::GroupFlag;
    use crate::livery::Livery;
    use crate::test_tables::{COMPANY_IDENTITY, FLEET_GROUPS};
    use std::ptr;

    const OLD: EngineID = EngineID(7);
    const NEW: EngineID = EngineID(9);
    const PLAIN: GroupID = GroupID(5);
    const OTHER: GroupID = GroupID(6);
    const PROTECTED: GroupID = GroupID(8);

    fn group(flags: u8) -> *mut () {
        ptr::from_mut(Box::leak(Box::new(GroupFields {
            owner: 0,
            vehicle_type: 0,
            flags,
            livery: Livery {
                in_use: 0,
                colour1: 0,
                colour2: 0,
            },
            parent: GroupID::INVALID.0,
            number: 0,
        })))
        .cast()
    }
    thread_local! {
        /// `PLAIN`, `OTHER` and `PROTECTED`, the last with replace protection.
        static GROUPS: [*mut (); 3] = [group(0), group(0), group(1 << GroupFlag::ReplaceProtection.0)];
        /// Rule one replaces `OLD` by `NEW` in `PLAIN`, when old by setting; rule two replaces
        /// `OLD` by itself in `ALL_GROUP`, with the setting off.
        static RULES: *mut () = {
            let second = Box::leak(Box::new(Renew {
                from: OLD.0,
                to: OLD.0,
                next: ptr::null_mut(),
                group_id: GroupID::ALL_GROUP.0,
                replace_when_old: false,
            }));
            ptr::from_mut(Box::leak(Box::new(Renew {
                from: OLD.0,
                to: NEW.0,
                next: ptr::from_mut(second).cast(),
                group_id: PLAIN.0,
                replace_when_old: true,
            })))
            .cast()
        };
    }

    extern "C" fn renew_state(renew: *mut ()) -> *mut () {
        renew
    }
    extern "C" fn group_of(id: u16) -> *mut () {
        let index = match GroupID(id) {
            PLAIN => 0,
            OTHER => 1,
            PROTECTED => 2,
            _ => return ptr::null_mut(),
        };
        GROUPS.with(|g| g[index])
    }
    thread_local! {
        /// The company's renewal owner: the Rust-owned slot holding its list head.
        static HEAD: std::cell::Cell<*mut ()> = std::cell::Cell::new(RULES.with(|r| *r));
    }
    extern "C" fn renewal_head(_: *const OpenTTDCompany) -> *mut () {
        HEAD.with(|h| h.as_ptr().cast())
    }

    fn lookup(
        groups: &'static GroupServices,
        engine: EngineID,
        group: GroupID,
    ) -> (EngineID, bool) {
        let mut companies = COMPANY_IDENTITY;
        companies.groups = groups;
        companies.renewal_head = renewal_head;
        let mut when_old = false;
        let to = engine_replacement_for_company(
            &companies,
            ptr::null(),
            engine,
            group,
            Some(&mut when_old),
        );
        assert_eq!(
            engine_has_replacement_for_company(&companies, ptr::null(), engine, group),
            to != EngineID::INVALID
        );
        (to, when_old)
    }

    /// `EngineReplacement` over a two-rule renewal list: a group's own rule, the `ALL_GROUP`
    /// fallback that a group's replace protection refuses, and `replace_when_old`.
    /// Expectations are `autoreplace.cpp`'s.
    #[test]
    fn group_rule_then_all_group_unless_protected() {
        let groups: &'static GroupServices = Box::leak(Box::new(GroupServices {
            group: group_of,
            renew_state,
            ..FLEET_GROUPS
        }));
        // The group's own rule, with the player's setting.
        assert_eq!(lookup(groups, OLD, PLAIN), (NEW, true));
        // No rule of its own: the `ALL_GROUP` rule, same model so only when old.
        assert_eq!(lookup(groups, OLD, OTHER), (OLD, true));
        assert_eq!(lookup(groups, OLD, GroupID::DEFAULT_GROUP), (OLD, true));
        // Replace protection refuses the fallback: not replacing.
        assert_eq!(lookup(groups, OLD, PROTECTED), (EngineID::INVALID, false));
        // A group that is not valid takes no fallback either.
        assert_eq!(lookup(groups, OLD, GroupID(40)), (EngineID::INVALID, false));
        // No rule for the engine at all.
        assert_eq!(lookup(groups, NEW, PLAIN), (EngineID::INVALID, false));
    }
}

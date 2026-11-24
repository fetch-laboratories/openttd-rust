/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `autoreplace_func.h`: a company's engine replacement lookups, over `fleet`'s
//! `EngineReplacement`. The company is a live shell of the serialized game state.
#![allow(unsafe_code)]

use crate::company::OpenTTDCompany;
use crate::company_identity::Services;
use crate::engine_type::EngineID;
use crate::group_type::GroupID;

/// `EngineReplacementForCompany`: retrieve the engine replacement for the given company and
/// original engine type. `group` is the group related to this replacement;
/// `replace_when_old` is set to true if the replacement should be done when old. Returns the
/// engine type to replace with, or `EngineID::INVALID` if no replacement is in the list.
pub(crate) fn engine_replacement_for_company(
    companies: &Services,
    c: *const OpenTTDCompany,
    engine: EngineID,
    group: GroupID,
    replace_when_old: Option<&mut bool>,
) -> EngineID {
    let replace_when_old = replace_when_old.map_or(std::ptr::null_mut(), std::ptr::from_mut);
    // SAFETY: The company's live renewal list and the immutable group services.
    EngineID(unsafe {
        crate::fleet::engine_replacement(
            companies.groups,
            (companies.renewal_list)(c),
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
    use std::cell::UnsafeCell;

    const OLD: EngineID = EngineID(7);
    const NEW: EngineID = EngineID(9);
    const PLAIN: GroupID = GroupID(5);
    const OTHER: GroupID = GroupID(6);
    const PROTECTED: GroupID = GroupID(8);

    struct Fixture {
        rules: [UnsafeCell<Renew>; 2],
        groups: [UnsafeCell<GroupFields>; 3],
    }
    // SAFETY: The tests read the fixture through the services on one thread.
    unsafe impl Sync for Fixture {}

    const fn group(flags: u8) -> UnsafeCell<GroupFields> {
        UnsafeCell::new(GroupFields {
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
        })
    }

    /// Rule one replaces `OLD` by `NEW` in `PLAIN`, when old by setting; rule two
    /// replaces `OLD` by itself in `ALL_GROUP`, with the setting off.
    static FIXTURE: Fixture = Fixture {
        rules: [
            UnsafeCell::new(Renew {
                from: OLD.0,
                to: NEW.0,
                next: std::ptr::null_mut(),
                group_id: PLAIN.0,
                replace_when_old: true,
            }),
            UnsafeCell::new(Renew {
                from: OLD.0,
                to: OLD.0,
                next: std::ptr::null_mut(),
                group_id: GroupID::ALL_GROUP.0,
                replace_when_old: false,
            }),
        ],
        groups: [
            group(0),
            group(0),
            group(1 << GroupFlag::ReplaceProtection.0),
        ],
    };

    unsafe extern "C" fn renew_state(renew: *mut ()) -> *mut () {
        renew
    }
    unsafe extern "C" fn group_of(id: u16) -> *mut () {
        let index = match GroupID(id) {
            PLAIN => 0,
            OTHER => 1,
            PROTECTED => 2,
            _ => return std::ptr::null_mut(),
        };
        FIXTURE.groups[index].get().cast()
    }
    extern "C" fn renewal_list(_: *const OpenTTDCompany) -> *mut () {
        let [first, second] = &FIXTURE.rules;
        // SAFETY: The fixture's rules, linked before each lookup.
        unsafe { (*first.get()).next = second.get().cast() };
        first.get().cast()
    }

    fn lookup(
        groups: &'static GroupServices,
        engine: EngineID,
        group: GroupID,
    ) -> (EngineID, bool) {
        let mut companies = COMPANY_IDENTITY;
        companies.groups = groups;
        companies.renewal_list = renewal_list;
        let mut when_old = false;
        let to = engine_replacement_for_company(
            &companies,
            std::ptr::null(),
            engine,
            group,
            Some(&mut when_old),
        );
        assert_eq!(
            engine_has_replacement_for_company(&companies, std::ptr::null(), engine, group),
            to != EngineID::INVALID
        );
        (to, when_old)
    }

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

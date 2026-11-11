/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Sign command evidence, separate from the production map callers.

use super::{Services, can_edit};
use crate::company_type::{CompanyID, Owner};
use crate::signs_type::Sign;

#[test]
fn ordinary_company_sign_is_editable_without_actor_or_mode_reads() {
    // Original 14ec60f248547d4d062a1160f0fc26d742319888, signs.cpp:
    // CompanyCanEditSign only inspects actor/mode for an OWNER_DEITY sign.
    // This is the ordinary company-owned branch of the existing sign fixture;
    // complete command, record and RNG capture remains pending. No fixture retires.
    extern "C" fn owner(_: *const Sign) -> Owner {
        CompanyID(0)
    }
    let services = Services {
        owner,
        ..crate::test_tables::SIGN_COMMANDS
    };
    // The owner callback supplies the sole read; no native record is dereferenced.
    // All other services abort on use, including the current-company callback.
    assert!(can_edit(&services, std::ptr::null()));
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Sign command evidence, separate from the production map callers.

use super::{Services, can_edit};
use crate::company_type::{CompanyID, Owner};
use crate::signs_type::Sign;

#[test]
fn ordinary_company_sign_is_editable_without_actor_or_mode_reads() {
    // Original 14ec60f248547d4d062a1160f0fc26d742319888, signs.cpp:
    // CompanyCanEditSign only inspects actor/mode for an OWNER_DEITY sign.
    // The native scenario retains this branch with real sign records and commands.
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

use crate::services::{ByteSpan, CommandCost, Cost};
use crate::signs_type::SignID;
use crate::strings_type::INVALID_STRING_ID;
use std::fmt::Write as _;
use std::sync::Mutex;

static ERROR: Mutex<Option<u32>> = Mutex::new(None);
extern "C" fn error(_: *mut CommandCost, message: u32) {
    *ERROR.lock().unwrap() = Some(message);
}
#[cfg(test)]
extern "C" fn available() -> bool {
    true
}
#[cfg(test)]
extern "C" fn full() -> bool {
    false
}
extern "C" fn ascii_length(data: *const u8, size: usize) -> usize {
    // These transferred inputs are exactly the original ASCII Alpha and Beta.
    // Actual multibyte 31/32-character limits stay on the native UTF-8 route.
    let text = unsafe { ByteSpan { data, size }.bytes() };
    assert!(matches!(text, b"Alpha" | b"Beta"));
    text.len()
}
extern "C" fn sign(id: SignID) -> *mut Sign {
    assert!(matches!(id.0, 0 | 2));
    // Only owner receives this opaque identity; no Rust reference or native field read.
    std::ptr::without_provenance_mut(usize::from(id.0) + 1)
}
extern "C" fn ordinary_owner(_: *const Sign) -> Owner {
    CompanyID(0)
}
fn span(text: &str) -> ByteSpan {
    ByteSpan {
        data: text.as_ptr(),
        size: text.len(),
    }
}
fn result(id: Option<SignID>) -> String {
    let error = ERROR.lock().unwrap().take();
    let mut output = format!("{} {}", error.is_none(), error.unwrap_or(INVALID_STRING_ID));
    if let Some(id) = id {
        write!(output, " {}", id.0).unwrap();
    }
    output
}

#[test]
fn sign_test_mode_matches_original_without_mutations() {
    const COSTS: crate::services::CommandCostServices = crate::services::CommandCostServices {
        error,
        ..crate::test_tables::COMMAND_COSTS
    };
    const SHARED: crate::services::Services = crate::services::Services {
        utf8_length: ascii_length,
        ..crate::test_tables::SHARED
    };
    let services = Services {
        shared: &SHARED,
        costs: &COSTS,
        sign,
        owner: ordinary_owner,
        can_allocate: available,
        ..crate::test_tables::SIGN_COMMANDS
    };
    *ERROR.lock().unwrap() = None;
    let out = Cost(std::ptr::null_mut(), &COSTS);
    let mut actual = Vec::new();
    let placed = super::place(&services, out, 0, 3860, span("Alpha"));
    actual.push(("place-test".into(), result(Some(placed))));
    super::rename(&services, out, 0, SignID(0), span("Beta"));
    actual.push(("rename-other-test".into(), result(None)));
    super::relocate(&services, out, 0, SignID(2), 1022);
    actual.push(("move-other-test".into(), result(None)));
    super::rename(&services, out, 0, SignID(2), span(""));
    actual.push(("delete-test".into(), result(None)));
    let full_services = Services {
        // Exhaustion must refuse before even the native length read. All reads
        // and mutations except allocation/cost therefore use aborting defaults.
        shared: &crate::test_tables::SHARED,
        costs: &COSTS,
        can_allocate: full,
        too_many_signs: 4258,
        ..crate::test_tables::SIGN_COMMANDS
    };
    let placed = super::place(&full_services, out, 0, 3860, span("Full"));
    actual.push(("place-full-test".into(), result(Some(placed))));
    let oracle = crate::oracle::parse(include_str!("test-mode-oracle.txt"));
    assert_eq!(crate::oracle::check(&oracle, "test-mode", actual), 5);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Component-local replacements for the selected objectives fixture observations.
//! Expectations below come from the unchanged original, never from candidate output.

use super::valid_goal_destination;
use crate::company_type::CompanyID;
use crate::goal_type::GT_NONE;

#[test]
fn goal_none_destination_matches_original_matrix_without_callbacks() {
    // Original 14ec60f248547d4d062a1160f0fc26d742319888, src/goal.cpp,
    // Goal::IsValidGoalDestination's GT_NONE branch accepts exactly dest == 0.
    // These are the existing objectives-console.cpp kind-0 destination inputs:
    // valid (0), UINT32_MAX and valid + 0x10000, in their original order.
    // The complete destination command matrix remains on its native dispatch route.
    let services = crate::test_tables::OBJECTIVES;
    for (destination, expected) in [(0, true), (u32::MAX, false), (0x1_0000, false)] {
        // SAFETY: This source branch inspects no native record or service. Every
        // callback is an aborting default, so any unexpected observation fails.
        let actual =
            unsafe { valid_goal_destination(&services, CompanyID::INVALID, GT_NONE, destination) };
        assert_eq!(actual, expected, "destination {destination}");
    }
}

#[test]
fn goal_question_matrix_matches_original_without_side_effects() {
    extern "C" fn deity() -> u8 {
        CompanyID::DEITY.0
    }
    // This Rust-only mock supplies the original nonempty text observation.
    #[cfg(test)]
    extern "C" fn text_empty(_: *const super::EncodedString) -> bool {
        false
    }
    let shared = crate::services::Services {
        current_company: deity,
        ..crate::test_tables::SHARED
    };
    let services = super::Services {
        shared: &raw const shared,
        text_empty,
        ..crate::test_tables::OBJECTIVES
    };
    let oracle = crate::oracle::parse(include_str!("question-oracle.txt"));
    let mut actual = Vec::new();
    // Keep the original fixture's kind/button order, global company and test flags.
    // The opaque token is read only by text_empty. Native EncodedString allocation,
    // CommandCost encoding and the real client/window/event routes remain in the scenario.
    let token = 0_u8;
    for kind in 0..=4 {
        for buttons in [0, 1, 7, 15, u32::MAX] {
            // SAFETY: Only text_empty can receive the live opaque token. All other
            // callbacks abort, so test mode must not publish a question or an event.
            let result = unsafe {
                super::openttd_rust_goal_question_abi(
                    &services,
                    0,
                    42,
                    u32::from(CompanyID::INVALID.0),
                    false,
                    buttons,
                    kind,
                    (&raw const token).cast(),
                )
            };
            actual.push((format!("kind={kind},buttons={buttons}"), result.to_string()));
        }
    }
    assert_eq!(crate::oracle::check(&oracle, "question", actual), 25);
}

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
    // This partial checkpoint replaces no fixture; complete command/result/event
    // capture and production-entry tests are still required for that transfer.
    let services = crate::test_tables::OBJECTIVES;
    for (destination, expected) in [(0, true), (u32::MAX, false), (0x1_0000, false)] {
        // SAFETY: This source branch inspects no native record or service. Every
        // callback is an aborting default, so any unexpected observation fails.
        let actual =
            unsafe { valid_goal_destination(&services, CompanyID::INVALID, GT_NONE, destination) };
        assert_eq!(actual, expected, "destination {destination}");
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared helpers of `command_func.h`.
use crate::command_type::{CommandFlag, DoCommandFlag};
pub(crate) fn command_flags_to_dc_flags(flags: u16) -> u16 {
    let mut result = 0;
    if flags & CommandFlag::NO_WATER != 0 {
        result |= DoCommandFlag::NO_WATER;
    }
    if flags & CommandFlag::AUTO != 0 {
        result |= DoCommandFlag::AUTO;
    }
    if flags & CommandFlag::ALL_TILES != 0 {
        result |= DoCommandFlag::ALL_TILES;
    }
    result
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Consumed `direction_func.h` inline helpers.
/// `ReverseDiagDir`; callers supply a native valid diagonal direction.
pub fn reverse_diag_dir(direction: u8) -> u8 {
    direction ^ 2
}

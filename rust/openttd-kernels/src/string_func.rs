/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared helpers from `string_func.h`.
/// `IsPrintable`, including surrogate and sprite codepoints.
pub fn is_printable(codepoint: u32) -> bool {
    codepoint >= 0x20 && !(0xe000..0xe200).contains(&codepoint)
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to errors (`error.h`).
//!
//! `ShowErrorMessage` runs natively (`error_gui.cpp`) until the error GUI is ported, through
//! the news table of `news_func.rs`, which is bound once for the process.

use crate::company_type::CompanyID;
use crate::services::ByteSpan;
use crate::strings_type::EncodedString;

/// Message severity/type.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarningLevel(pub u8);
impl WarningLevel {
    /// Used for DoCommand-like (and some non-fatal AI GUI) errors/information.
    pub const WL_INFO: Self = Self(0);
    /// Other information.
    pub const WL_WARNING: Self = Self(1);
    /// Errors (eg. saving/loading failed).
    pub const WL_ERROR: Self = Self(2);
    /// Critical errors, the MessageBox is shown in all cases.
    pub const WL_CRITICAL: Self = Self(3);
}

/// The bytes of an encoded string, borrowed for a synchronous call.
fn span(string: &EncodedString) -> ByteSpan {
    let bytes = string.as_bytes();
    ByteSpan {
        data: bytes.as_ptr(),
        size: bytes.len(),
    }
}

/// `ShowErrorMessage`: display an error message in a window.
///
/// - `summary_msg`: general error message showed in first line. Must be valid.
/// - `detailed_msg`: detailed error message showed in second line. Can be empty.
/// - `wl`: message severity.
/// - `x`: world X position (`TileVirtX`) of the error location. Set both x and y to 0 to
///   just center the message when there is no related error tile.
/// - `y`: world Y position (`TileVirtY`) of the error location. Set both x and y to 0 to
///   just center the message when there is no related error tile.
/// - `extra_msg`: extra error message shown in third line. Can be empty.
/// - `company`: company belonging to the face being shown; `CompanyID::INVALID` if no face.
pub(crate) fn show_error_message(
    summary_msg: EncodedString,
    detailed_msg: EncodedString,
    wl: WarningLevel,
    x: i32,
    y: i32,
    extra_msg: EncodedString,
    company: CompanyID,
) {
    (crate::news_func::services().show_error_message)(
        span(&summary_msg),
        span(&detailed_msg),
        wl.0,
        x,
        y,
        span(&extra_msg),
        company.0,
    );
}

#[cfg(test)]
mod tests {
    use super::WarningLevel;
    use crate::company_type::CompanyID;
    use crate::news_func::testing::{Effect, encoded, take};
    use crate::strings_func::get_encoded_string;
    use crate::strings_type::{EncodedString, StringID};

    /// Each of the seven arguments reaches the native call unchanged and in its position.
    #[test]
    fn show_error_message_passes_each_argument_in_order() {
        take();
        super::show_error_message(
            get_encoded_string(StringID(1), &[]),
            get_encoded_string(StringID(2), &[]),
            WarningLevel::WL_WARNING,
            -5,
            7,
            get_encoded_string(StringID(3), &[]),
            CompanyID(4),
        );
        super::show_error_message(
            get_encoded_string(StringID(8), &[]),
            EncodedString::default(),
            WarningLevel::WL_CRITICAL,
            0,
            0,
            EncodedString::default(),
            CompanyID::INVALID,
        );
        assert_eq!(
            take(),
            [
                Effect::ShowErrorMessage {
                    summary_msg: encoded(1, &[]),
                    detailed_msg: encoded(2, &[]),
                    wl: 1,
                    x: -5,
                    y: 7,
                    extra_msg: encoded(3, &[]),
                    company: 4,
                },
                Effect::ShowErrorMessage {
                    summary_msg: encoded(8, &[]),
                    detailed_msg: Vec::new(),
                    wl: 3,
                    x: 0,
                    y: 0,
                    extra_msg: Vec::new(),
                    company: 0xFF,
                },
            ]
        );
    }
}

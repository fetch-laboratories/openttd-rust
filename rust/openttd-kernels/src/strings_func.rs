/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to I/O of strings (`strings_func.h`).
//!
//! Encoding runs in Rust; `GetString` formats natively until the strings port, through the
//! news table of `news_func.rs`.

use crate::encoded::{self, Descriptor};
use crate::news_func::services;
use crate::strings_type::{EncodedString, StringID, StringParameterData};
use std::ffi::c_void;

/// `GetEncodedStringWithArgs`: encode a string with its parameters into an encoded string.
/// The encoded string can be stored and decoded later without requiring parameters to be
/// stored separately.
///
/// The original's `WITH_ASSERT` check that a string parameter is not itself encoded runs
/// natively, here, before any later effect of the caller.
pub(crate) fn get_encoded_string(
    id: StringID,
    params: &[StringParameterData<'_>],
) -> EncodedString {
    let s = services();
    let result = encoded::get_encoded_string_with_args(id, params, s.string_assertions != 0);
    let view = result.view();
    if view.assertion_kind != 0 {
        (s.assert_encoded)(view.assertion_codepoint, view.assertion_length);
    }
    EncodedString::from_encoded(result.into_bytes())
}

#[allow(
    unsafe_code,
    reason = "the sink reads the native string's bytes during its call"
)]
extern "C" fn append(target: *mut c_void, bytes: *const u8, length: usize) {
    // SAFETY: `get_string` passes its `Vec` and the native string's live bytes.
    unsafe {
        let target = &mut *target.cast::<Vec<u8>>();
        if length != 0 {
            target.extend_from_slice(std::slice::from_raw_parts(bytes, length));
        }
    }
}

/// Storage for one native `StringParameter`, which `GetString`'s native service constructs in
/// place and destroys before it returns; `services.cpp` checks that one fits.
#[repr(C, align(8))]
pub struct StringParameterSlot {
    bytes: [std::mem::MaybeUninit<u8>; 64],
}
impl StringParameterSlot {
    const UNINIT: Self = Self {
        bytes: [std::mem::MaybeUninit::uninit(); 64],
    };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStringParameterSlot" ["news"] => StringParameterSlot {
        bytes,
    };
}

/// `GetString(string, args...)`: get a parsed string with most special stringcodes replaced
/// by the string parameters.
pub(crate) fn get_string<const N: usize>(
    string: StringID,
    args: [StringParameterData<'_>; N],
) -> Vec<u8> {
    let descriptors = args.map(descriptor);
    // The native parameters live on this stack, as `MakeParameters` builds its array.
    let mut slots = [const { StringParameterSlot::UNINIT }; N];
    let mut result = Vec::new();
    (services().get_string)(
        string.0,
        descriptors.as_ptr(),
        slots.as_mut_ptr(),
        N,
        append,
        (&raw mut result).cast(),
    );
    result
}

/// The tagged descriptor of one parameter, borrowed for a synchronous call.
fn descriptor(parameter: StringParameterData<'_>) -> Descriptor {
    match parameter {
        StringParameterData::Empty => Descriptor {
            value: 0,
            bytes: std::ptr::null(),
            length: 0,
            kind: 0,
        },
        StringParameterData::Integer(value) => Descriptor {
            value,
            bytes: std::ptr::null(),
            length: 0,
            kind: 1,
        },
        StringParameterData::String(bytes) => Descriptor {
            value: 0,
            bytes: bytes.as_ptr(),
            length: bytes.len(),
            kind: 2,
        },
    }
}

/// Sink a native string into `target` (see [`crate::news_func::Sink`]).
pub(crate) fn sink_into(target: &mut Vec<u8>) -> (crate::news_func::Sink, *mut c_void) {
    (append, (&raw mut *target).cast())
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Native-locale byte strings; C++ retains owners and native length comparison.

use std::ffi::c_int;

#[allow(unsafe_code)] // Native C runtime bindings, called only by the mapping wrappers.
unsafe extern "C" {
    fn toupper(value: c_int) -> c_int;
    fn tolower(value: c_int) -> c_int;
}

#[allow(unsafe_code)] // Deliberately retains the original native toupper(char) call.
fn upper(byte: u8, signed_char: bool) -> c_int {
    let promoted = if signed_char {
        c_int::from(i8::from_ne_bytes([byte]))
    } else {
        c_int::from(byte)
    };
    // SAFETY: Same native runtime and char promotion as the C++ call. Negative
    // values other than EOF retain historical host behavior outside portable
    // C's specified domain; this port does not make that behavior defined.
    unsafe { toupper(promoted) }
}

#[allow(unsafe_code)] // All u8 values are in tolower's specified domain.
fn lower(byte: u8) -> u8 {
    // SAFETY: Every input is representable as unsigned char, matching C++.
    let mapped = unsafe { tolower(c_int::from(byte)) };
    // C++ narrows the result to char; retain the low byte rather than validate it.
    mapped.to_le_bytes()[0]
}

fn compare_prefix(left: &[u8], right: &[u8], signed_char: bool) -> i32 {
    for (&left, &right) in left.iter().zip(right) {
        if upper(left, signed_char) < upper(right, signed_char) {
            return -1;
        }
        if upper(left, signed_char) > upper(right, signed_char) {
            return 1;
        }
    }
    0
}

fn case_operation(
    left: &[u8],
    right: &[u8],
    mode: u8,
    signed_char: bool,
    length_order: i32,
) -> i32 {
    match mode {
        0 => {
            let result = compare_prefix(left, right, signed_char);
            if result == 0 { length_order } else { result }
        }
        1 => i32::from(left.len() == right.len() && compare_prefix(left, right, signed_char) == 0),
        2 => i32::from(
            left.len() >= right.len()
                && compare_prefix(&left[..right.len()], right, signed_char) == 0,
        ),
        3 => i32::from(
            left.len() >= right.len()
                && compare_prefix(&left[left.len() - right.len()..], right, signed_char) == 0,
        ),
        4 => {
            if right.is_empty() {
                return 1;
            }
            if right.len() > left.len() {
                return 0;
            }
            for start in 0..=left.len() - right.len() {
                if upper(left[start], signed_char) == upper(right[0], signed_char)
                    && compare_prefix(&left[start..start + right.len()], right, signed_char) == 0
                {
                    return 1;
                }
            }
            0
        }
        _ => unreachable!("invalid case operation"),
    }
}

/// Offsets for the C++ source substring; zero length selects a default null view.
#[repr(C)]
pub struct TrimResult {
    /// First byte retained; zero when all bytes are trimmed.
    pub offset: usize,
    /// Retained length; zero means all trimmed or empty input.
    pub length: usize,
}

fn trim(bytes: &[u8], set: &[u8]) -> TrimResult {
    let Some(first) = bytes.iter().position(|byte| !set.contains(byte)) else {
        return TrimResult {
            offset: 0,
            length: 0,
        };
    };
    let last = bytes.iter().rposition(|byte| !set.contains(byte)).unwrap();
    TrimResult {
        offset: first,
        length: last - first + 1,
    }
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte + 10 - b'A'),
        b'a'..=b'f' => Some(byte + 10 - b'a'),
        _ => None,
    }
}

#[allow(unsafe_code)] // Shared span creation, preserving null for empty inputs.
unsafe fn borrowed<'a>(data: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        &[]
    } else {
        // SAFETY: Each exported caller documents this live immutable span.
        unsafe { std::slice::from_raw_parts(data, length) }
    }
}

/// Compare/equal/prefix/suffix/contains using native toupper(char).
///
/// # Safety
/// Spans follow `byte_strings_ffi.h`: nonempty spans are readable for the call, empty spans
/// permit null and read-only spans may overlap. The native C locale must not change
/// concurrently.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_case(
    left: *const u8,
    left_length: usize,
    right: *const u8,
    right_length: usize,
    mode: u8,
    signed_char: u8,
    length_order: i32,
) -> i32 {
    // SAFETY: Both caller-provided spans satisfy the documented shared borrow.
    let (left, right) = unsafe { (borrowed(left, left_length), borrowed(right, right_length)) };
    case_operation(left, right, mode, signed_char != 0, length_order)
}

/// Lowercase [offset,length), returning one if a byte changed.
///
/// # Safety
/// Nonempty data is initialized writable bytes in one allocation of length <=
/// `isize::MAX`, exclusively accessible during this call; empty permits null.
/// offset <= length. Native C locale must not change concurrently. No pointer is
/// retained, ownership is unchanged, and panic aborts without unwinding.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_lower(
    data: *mut u8,
    length: usize,
    offset: usize,
) -> u8 {
    let bytes = if length == 0 {
        &mut []
    } else {
        // SAFETY: Caller provides the exclusive initialized writable allocation.
        unsafe { std::slice::from_raw_parts_mut(data, length) }
    };
    let mut changed = false;
    for byte in &mut bytes[offset..] {
        let mapped = lower(*byte);
        changed |= mapped != *byte;
        *byte = mapped;
    }
    u8::from(changed)
}

/// Encode uppercase hex into caller-owned storage.
///
/// # Safety
/// The readable input and exclusive writable output are disjoint live initialized
/// allocations, each length <= `isize::MAX`; output length is exactly 2 * length.
/// Empty spans may be null. No pointer is retained or ownership transferred.
/// Panic aborts and this ABI never unwinds.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_hex_encode(
    data: *const u8,
    length: usize,
    output: *mut u8,
) {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    // SAFETY: Caller provides disjoint input and twice-sized initialized output.
    let bytes = unsafe { borrowed(data, length) };
    let output = if length == 0 {
        &mut []
    } else {
        // SAFETY: Exactly twice the input length, exclusive and disjoint.
        unsafe { std::slice::from_raw_parts_mut(output, length * 2) }
    };
    for (&byte, pair) in bytes.iter().zip(output.as_chunks_mut::<2>().0) {
        pair[0] = DIGITS[usize::from(byte >> 4)];
        pair[1] = DIGITS[usize::from(byte & 15)];
    }
}

/// Decode hex sequentially, preserving partial writes and legal overlapping views.
///
/// # Safety
/// Nonempty hex is initialized readable bytes; output is writable and may be uninitialized
/// except bytes also read through overlapping input. Both nibbles are read before each write,
/// and later reads see earlier writes, as in C++. Empty spans permit null.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_hex_decode(
    hex: *const u8,
    length: usize,
    output: *mut u8,
    output_length: usize,
) -> u8 {
    if output_length != length / 2 || !length.is_multiple_of(2) {
        return 0;
    }
    for index in 0..output_length {
        // SAFETY: Both positions are inside the readable caller span. These raw
        // reads finish before writing, even when input/output allocations overlap.
        let (hi, lo) = unsafe {
            (
                nibble(hex.add(index * 2).read()),
                nibble(hex.add(index * 2 + 1).read()),
            )
        };
        let (Some(hi), Some(lo)) = (hi, lo) else {
            return 0;
        };
        // SAFETY: This position is within the writable span; no references alias it.
        unsafe { output.add(index).write((hi << 4) | lo) };
    }
    1
}

/// Return byte-set trim offsets without allocating or retaining views.
///
/// # Safety
/// Both spans have the readable shared-span contract of `openttd_rust_bytes_case`,
/// allowing read-only overlap and null for empty. No pointer is retained; panic
/// aborts and this ABI never unwinds.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_trim(
    data: *const u8,
    length: usize,
    set: *const u8,
    set_length: usize,
) -> TrimResult {
    // SAFETY: Caller supplies two immutable live spans for this call.
    let (bytes, set) = unsafe { (borrowed(data, length), borrowed(set, set_length)) };
    trim(bytes, set)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustByteTrim" ["probe"] => TrimResult {
        offset,
        length,
    };
}

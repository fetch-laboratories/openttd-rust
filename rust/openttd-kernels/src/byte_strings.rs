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

/// `CaseInsensitiveCharTraits::compare`: compare exactly the paired bytes.
fn case_insensitive_char_traits_compare(left: &[u8], right: &[u8], signed_char: bool) -> i32 {
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

/// ABI selectors used by the five native case-insensitive string facades.
const CASE_COMPARE: u8 = 0;
const CASE_EQUALS: u8 = 1;
const CASE_STARTS_WITH: u8 = 2;
const CASE_ENDS_WITH: u8 = 3;
const CASE_CONTAINS: u8 = 4;

/// `StrCompareIgnoreCase`; length ordering remains the native library's result.
fn str_compare_ignore_case(left: &[u8], right: &[u8], signed_char: bool, length_order: i32) -> i32 {
    let result = case_insensitive_char_traits_compare(left, right, signed_char);
    if result == 0 { length_order } else { result }
}
/// `StrEqualsIgnoreCase`: equal lengths and case-insensitive byte equality.
fn str_equals_ignore_case(left: &[u8], right: &[u8], signed_char: bool) -> bool {
    left.len() == right.len() && case_insensitive_char_traits_compare(left, right, signed_char) == 0
}
/// `StrStartsWithIgnoreCase`: compare the prefix only if it fits.
fn str_starts_with_ignore_case(left: &[u8], right: &[u8], signed_char: bool) -> bool {
    left.len() >= right.len() && str_equals_ignore_case(&left[..right.len()], right, signed_char)
}
/// `StrEndsWithIgnoreCase`: compare the suffix only if it fits.
fn str_ends_with_ignore_case(left: &[u8], right: &[u8], signed_char: bool) -> bool {
    left.len() >= right.len()
        && str_equals_ignore_case(&left[left.len() - right.len()..], right, signed_char)
}
/// `StrContainsIgnoreCase`: the native case-insensitive view's substring search.
fn str_contains_ignore_case(left: &[u8], right: &[u8], signed_char: bool) -> bool {
    if right.is_empty() {
        return true;
    }
    if right.len() > left.len() {
        return false;
    }
    for start in 0..=left.len() - right.len() {
        if upper(left[start], signed_char) == upper(right[0], signed_char)
            && case_insensitive_char_traits_compare(
                &left[start..start + right.len()],
                right,
                signed_char,
            ) == 0
        {
            return true;
        }
    }
    false
}

fn case_operation(
    left: &[u8],
    right: &[u8],
    mode: u8,
    signed_char: bool,
    length_order: i32,
) -> i32 {
    match mode {
        CASE_COMPARE => str_compare_ignore_case(left, right, signed_char, length_order),
        CASE_EQUALS => i32::from(str_equals_ignore_case(left, right, signed_char)),
        CASE_STARTS_WITH => i32::from(str_starts_with_ignore_case(left, right, signed_char)),
        CASE_ENDS_WITH => i32::from(str_ends_with_ignore_case(left, right, signed_char)),
        CASE_CONTAINS => i32::from(str_contains_ignore_case(left, right, signed_char)),
        _ => unreachable!("invalid case operation"),
    }
}

/// `strtolower`: lowercase bytes from the original offset, reporting any change.
fn strtolower(bytes: &mut [u8], offset: usize) -> bool {
    let mut changed = false;
    for byte in &mut bytes[offset..] {
        let mapped = lower(*byte);
        changed |= mapped != *byte;
        *byte = mapped;
    }
    changed
}

/// `FormatArrayAsHex`: write a continuous uppercase hex string into its native
/// pre-sized storage. The input and output are disjoint.
pub(crate) fn format_array_as_hex(bytes: &[u8], output: &mut [u8]) {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    for (&byte, pair) in bytes.iter().zip(output.as_chunks_mut::<2>().0) {
        pair[0] = DIGITS[usize::from(byte >> 4)];
        pair[1] = DIGITS[usize::from(byte & 15)];
    }
}

/// `ConvertHexToBytes`: the hex string must have exactly twice the output
/// length. Read each pair before writing, preserving partial writes on failure.
/// Call-local byte accessors retain defined overlapping-span behavior without
/// creating overlapping Rust references or allocating temporary storage.
fn convert_hex_to_bytes(
    length: usize,
    output_length: usize,
    mut read: impl FnMut(usize) -> u8,
    mut write: impl FnMut(usize, u8),
) -> bool {
    if output_length != length / 2 {
        return false;
    }
    /* Hex-string lengths are always divisible by 2. */
    if !length.is_multiple_of(2) {
        return false;
    }
    for index in 0..output_length {
        let hi = convert_hex_nibble_to_byte(read(index * 2));
        let lo = convert_hex_nibble_to_byte(read(index * 2 + 1));
        let (Some(hi), Some(lo)) = (hi, lo) else {
            return false;
        };
        write(index, (hi << 4) | lo);
    }
    true
}

/// Offsets for the C++ source substring; zero length selects a default null view.
#[repr(C)]
pub struct TrimResult {
    /// First byte retained; zero when all bytes are trimmed.
    pub offset: usize,
    /// Retained length; zero means all trimmed or empty input.
    pub length: usize,
}

/// `StrTrimView`: return the retained substring or a default null view.
fn str_trim_view(bytes: &[u8], set: &[u8]) -> TrimResult {
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

/// `ConvertHexNibbleToByte`: a hex digit, or no value for the original -1.
fn convert_hex_nibble_to_byte(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte + 10 - b'A'),
        b'a'..=b'f' => Some(byte + 10 - b'a'),
        _ => None,
    }
}

#[allow(unsafe_code)] // Shared span creation, preserving null for empty inputs.
pub(crate) unsafe fn borrowed<'a>(data: *const u8, length: usize) -> &'a [u8] {
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
    u8::from(strtolower(bytes, offset))
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
    // SAFETY: Caller provides disjoint input and twice-sized initialized output.
    let bytes = unsafe { borrowed(data, length) };
    let output = if length == 0 {
        &mut []
    } else {
        // SAFETY: Exactly twice the input length, exclusive and disjoint.
        unsafe { std::slice::from_raw_parts_mut(output, length * 2) }
    };
    format_array_as_hex(bytes, output);
}

/// Decode hex sequentially, preserving partial writes and legal overlapping views.
///
/// # Safety
/// Nonempty hex is initialized readable bytes; output is writable and may be uninitialized
/// except bytes also read through overlapping input. Both nibbles are read before each write,
/// and later reads see earlier writes, as in C++. Empty spans permit null; spans follow
/// `byte_strings_ffi.h`'s live-allocation, extent and exclusive-access contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_bytes_hex_decode(
    hex: *const u8,
    length: usize,
    output: *mut u8,
    output_length: usize,
) -> u8 {
    u8::from(convert_hex_to_bytes(
        length,
        output_length,
        // SAFETY: The core reads only the validated caller input extent.
        |index| unsafe { hex.add(index).read() },
        // SAFETY: Each output index is within the live writable extent. All
        // reads of its input pair finish first, including for overlapping views.
        |index, byte| unsafe { output.add(index).write(byte) },
    ))
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
    str_trim_view(bytes, set)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustByteTrim" ["probe"] => TrimResult {
        offset,
        length,
    };
}

#[cfg(all(test, target_os = "linux", target_env = "gnu"))]
mod comparison_tests {
    //! Original-only `byte-strings-comparison.cpp` observations (#736 U) through the
    //! production exports, under each captured glibc locale; see the oracle header.
    use super::{
        openttd_rust_bytes_case, openttd_rust_bytes_hex_decode, openttd_rust_bytes_hex_encode,
        openttd_rust_bytes_lower, openttd_rust_bytes_trim,
    };
    use crate::oracle;
    use std::collections::HashSet;
    use std::ffi::{CStr, c_char, c_int};
    use std::fmt::Write as _;

    const ORACLE: &str = include_str!("byte_strings/comparison-oracle.txt");
    const LC_ALL: c_int = 6;

    fn token(value: &[u8]) -> String {
        if value.is_empty() {
            return "-".to_owned();
        }
        value.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    /// `byte-strings-comparison.py::corpus` at 305ac20c28, then `dict.fromkeys`.
    fn corpus() -> Vec<String> {
        let mut out = vec!["null -".to_owned()];
        case_records(&mut out);
        hex_records(&mut out);
        trim_records(&mut out);
        let mut seen = HashSet::new();
        out.retain(|record| seen.insert(record.clone()));
        out
    }

    fn case_records(out: &mut Vec<String>) {
        let values: [&[u8]; 18] = [
            b"",
            b"a",
            b"A",
            b"aa",
            b"aA",
            b"b",
            b"ababa",
            b"aba",
            b"abab",
            b"BAB",
            b"absent",
            b"abc",
            b"c",
            b"\x00",
            b"a\x00B",
            b"A\x00b",
            b"\x00b",
            b"x\x00a\x00Bz",
        ];
        for left in values {
            for right in values {
                out.push(format!("pair {} {}", token(left), token(right)));
            }
        }
        for byte in 0..=u8::MAX {
            let value = [byte];
            for right in [b"\x00", b"A", b"\x7f", b"\xff", &value] {
                out.push(format!("pair {} {}", token(&value), token(right)));
            }
            for offset in 0..4 {
                out.push(format!("lower {} {offset}", token(&[b'A', byte, b'Z'])));
            }
            out.push(format!("encode {}", token(&value)));
            for pair in [[b'1', byte], [byte, b'1']] {
                out.push(format!("decode {} 1", token(&pair)));
            }
        }
        let all: Vec<u8> = (0..=u8::MAX).collect();
        for value in [&b""[..], b"lower", b"UPPER", b"Aa\x00Z", &all] {
            out.push(format!("encode {}", token(value)));
            let mut offsets = vec![0, value.len() / 2, value.len()];
            offsets.dedup();
            for offset in offsets {
                out.push(format!("lower {} {offset}", token(value)));
            }
        }
    }

    fn hex_records(out: &mut Vec<String>) {
        let decodes: [&[u8]; 14] = [
            b"",
            b"1",
            b"12",
            b"123",
            b"1234",
            b"12g4",
            b"123g",
            b"g234",
            b"1g34",
            b"123456789abcdef0",
            b"123456789ABCDEF0",
            b"123456g8",
            b"12\x0034",
            b"12\xff4",
        ];
        for value in decodes {
            for length in 0..6 {
                out.push(format!("decode {} {length}", token(value)));
            }
        }
        let all: Vec<u8> = (0..=u8::MAX).collect();
        out.push(format!("decode {} 256", token(to_hex(&all).as_bytes())));
        for value in [
            &b"12345678"[..],
            b"1234g678",
            b"1234567g",
            b"g2345678",
            b"1234567",
        ] {
            let backing = [&b"\xa5\xa5"[..], value, &[0xa5; 12]].concat();
            for offset in 0..11 {
                for length in [value.len() / 2, value.len() / 2 + 1] {
                    let record = format!(
                        "overlap {} 2 {} {offset} {length}",
                        token(&backing),
                        value.len()
                    );
                    out.push(record);
                }
            }
        }
    }

    fn trim_records(out: &mut Vec<String>) {
        let all: Vec<u8> = (0..=u8::MAX).collect();
        let trims: [&[u8]; 12] = [
            b"",
            b" ",
            b" \t\x0b\x0c\r ",
            b"\n",
            b" \n ",
            b"  a  b  ",
            b"\x00",
            b"\x00a\x00",
            b"\xff\x00a\xff",
            b"\xff\xff",
            b"aaaa",
            b"xabxx",
        ];
        let sets: [&[u8]; 8] = [
            b"",
            b" \t\x0b\x0c\r",
            b"\x00",
            b"\xff",
            b"\xff\x00",
            b"a",
            b"x",
            &all,
        ];
        for value in trims {
            out.push(format!("inplace {}", token(value)));
            for set in sets {
                out.push(format!("trim {} {}", token(value), token(set)));
            }
        }
    }

    fn from_hex(token: &str) -> Vec<u8> {
        if token == "-" {
            return Vec::new();
        }
        (0..token.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&token[i..i + 2], 16).unwrap())
            .collect()
    }

    /// The probe's uppercase `ToHex`.
    fn to_hex(value: &[u8]) -> String {
        if value.is_empty() {
            return "-".to_owned();
        }
        value.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02X}");
            s
        })
    }

    /// The test's only unsafe calls: libc locale switching and the production exports over
    /// this test's live buffers.
    struct Native;

    #[allow(unsafe_code)]
    impl Native {
        /// Candidate `RustByteCase`: C++ computes the libstdc++ `string_view::compare` length
        /// order (difference clamped to int) for mode 0; Rust maps and compares the bytes.
        fn case(left: &[u8], right: &[u8], mode: u8, signed: bool) -> i32 {
            let length_order = if mode != 0 {
                0
            } else if left.len() >= right.len() {
                i32::try_from(left.len() - right.len()).unwrap_or(i32::MAX)
            } else {
                i32::try_from(right.len() - left.len()).map_or(i32::MIN, |d| -d)
            };
            // SAFETY: Live immutable spans for the call.
            unsafe {
                openttd_rust_bytes_case(
                    left.as_ptr(),
                    left.len(),
                    right.as_ptr(),
                    right.len(),
                    mode,
                    u8::from(signed),
                    length_order,
                )
            }
        }

        fn execute(record: &str, signed: bool) -> String {
            let fields: Vec<&str> = record.split(' ').collect();
            let mut left = from_hex(fields[1]);
            match fields[0] {
                "pair" => {
                    let right = from_hex(fields[2]);
                    let r: Vec<i32> = (0..5)
                        .map(|mode| Self::case(&left, &right, mode, signed))
                        .collect();
                    // No ICU in the probe build: StrNaturalContainsIgnoreCase is StrContainsIgnoreCase.
                    format!("{} {} {} {} {} {}", r[0], r[1], r[2], r[3], r[4], r[4])
                }
                "lower" => {
                    let offset = fields[2].parse().unwrap();
                    // SAFETY: Exclusive initialized span; offset <= length.
                    let changed =
                        unsafe { openttd_rust_bytes_lower(left.as_mut_ptr(), left.len(), offset) };
                    format!("{changed} {}", to_hex(&left))
                }
                "encode" => {
                    let mut text = vec![0; left.len() * 2];
                    // SAFETY: Disjoint input and twice-sized output.
                    unsafe {
                        openttd_rust_bytes_hex_encode(left.as_ptr(), left.len(), text.as_mut_ptr());
                    }
                    to_hex(&text)
                }
                "decode" => {
                    let size: usize = fields[2].parse().unwrap();
                    let mut backing = vec![0xa5_u8; size + 2];
                    // SAFETY: Readable hex; the writable span lies strictly inside the backing.
                    let result = unsafe {
                        openttd_rust_bytes_hex_decode(
                            left.as_ptr(),
                            left.len(),
                            backing.as_mut_ptr().add(1),
                            size,
                        )
                    };
                    format!("{result} {}", to_hex(&backing))
                }
                "overlap" => {
                    let n: Vec<usize> = fields[2..6].iter().map(|f| f.parse().unwrap()).collect();
                    let base = left.as_mut_ptr();
                    // SAFETY: Overlapping caller-valid views over one live initialized allocation.
                    let result = unsafe {
                        openttd_rust_bytes_hex_decode(base.add(n[0]), n[1], base.add(n[2]), n[3])
                    };
                    format!("{result} {}", to_hex(&left))
                }
                "trim" => {
                    let set = from_hex(fields[2]);
                    // A "-" token is the default null view.
                    let data = if fields[1] == "-" {
                        std::ptr::null()
                    } else {
                        left.as_ptr()
                    };
                    // SAFETY: Two immutable spans (null only when empty).
                    let r = unsafe {
                        openttd_rust_bytes_trim(data, left.len(), set.as_ptr(), set.len())
                    };
                    if r.length == 0 {
                        "1 - 0 -".to_owned()
                    } else {
                        format!(
                            "0 {} {} {}",
                            r.offset,
                            r.length,
                            to_hex(&left[r.offset..r.offset + r.length])
                        )
                    }
                }
                "inplace" => {
                    // StringConsumer::WHITESPACE_NO_NEWLINE.
                    let set = b"\t\x0b\x0c\r ";
                    // SAFETY: Two immutable spans.
                    let r = unsafe {
                        openttd_rust_bytes_trim(left.as_ptr(), left.len(), set.as_ptr(), set.len())
                    };
                    let kept = if r.length == 0 {
                        &[][..]
                    } else {
                        &left[r.offset..r.offset + r.length]
                    };
                    to_hex(kept)
                }
                "null" => {
                    let null = std::ptr::null();
                    // SAFETY: Null empty spans are permitted by every export.
                    let (cmp, flags, decoded, trim) = unsafe {
                        (
                            openttd_rust_bytes_case(null, 0, null, 0, 0, u8::from(signed), 0),
                            [1, 2, 3, 4].map(|mode| {
                                openttd_rust_bytes_case(null, 0, null, 0, mode, u8::from(signed), 0)
                            }),
                            openttd_rust_bytes_hex_decode(null, 0, std::ptr::null_mut(), 0),
                            openttd_rust_bytes_trim(null, 0, null, 0),
                        )
                    };
                    format!(
                        "{cmp} {} {} {} {} {decoded} {}",
                        flags[0],
                        flags[1],
                        flags[2],
                        flags[3],
                        u8::from(trim.length == 0)
                    )
                }
                other => panic!("unknown record {other}"),
            }
        }

        fn set_locale(name: &CStr) -> bool {
            unsafe extern "C" {
                fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
            }
            // SAFETY: NUL-terminated name; this module's only test switches the C locale.
            !unsafe { setlocale(LC_ALL, name.as_ptr()) }.is_null()
        }
    }

    #[test]
    fn byte_strings_match_original_in_each_locale_and_char_signedness() {
        let o = oracle::parse(ORACLE);
        let records = corpus();
        assert_eq!(records.len(), 3699);
        for locale in [c"C", c"POSIX", c"C.utf8", c"en_US.utf8"] {
            if !Native::set_locale(locale) {
                assert!(
                    locale != c"C" && locale != c"POSIX",
                    "required locale {locale:?} missing"
                );
                eprintln!("byte-strings oracle: locale {locale:?} unavailable; skipped");
                continue;
            }
            let actual = records.iter().map(|record| {
                let (signed, unsigned) = (
                    Native::execute(record, true),
                    Native::execute(record, false),
                );
                let output = if signed == unsigned {
                    signed
                } else {
                    format!("{signed} ~ {unsigned}")
                };
                (format!("{locale:?} {record}"), output)
            });
            oracle::check(&o, "records", actual.collect::<Vec<_>>());
        }
        assert!(Native::set_locale(c"C"));
    }
}

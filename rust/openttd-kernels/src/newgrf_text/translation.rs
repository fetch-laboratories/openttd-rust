/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `TranslateTTDPatchCodes` and `UnmappedChoiceList`. Text is bytes, including
//! surrogate UTF-8 and embedded NULs. Native services borrow only for their call.
use super::Services;
use crate::control_codes::*;
use crate::debug::debug;
use crate::{consumer, string_func, utf8};
use std::collections::BTreeMap;
use std::ffi::c_void;

fn put(dest: &mut Vec<u8>, value: u32) {
    let encoded = utf8::encode(value);
    dest.extend_from_slice(&encoded.bytes[..encoded.length]);
}
struct Consumer<'a> {
    bytes: &'a [u8],
}
impl Consumer<'_> {
    fn read(&mut self, width: u8) -> u32 {
        let value = consumer::little_endian(self.bytes, width).value_bits as u32;
        self.skip(usize::from(width));
        value
    }
    fn skip(&mut self, count: usize) {
        let bounds = consumer::bound(self.bytes.len(), 0, count);
        if bounds.shortfall != 0 {
            debug!(
                misc,
                0, "Source buffer too short: {} > {}", count, bounds.length
            );
        }
        self.bytes = &self.bytes[bounds.position..];
    }
}
struct ChoiceList {
    code: u32,
    offset: u32,
    strings: BTreeMap<i32, Vec<u8>>,
}
impl ChoiceList {
    fn flush(&mut self, s: &Services, lm: *const c_void, result: &mut Vec<u8>) {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.strings.entry(0) {
            s.message("choice list misses default value");
            entry.insert(Vec::new());
        }
        if lm.is_null() {
            result.extend_from_slice(&self.strings[&0]);
            return;
        }
        put(result, self.code);
        if self.code == SCC_SWITCH_CASE {
            let cases = unsafe { (s.num_cases)() };
            let count = (0..cases)
                .filter(|&i| {
                    let index = unsafe { (s.reverse_mapping)(lm, i32::from(i), false) };
                    self.strings.contains_key(&index)
                })
                .count();
            result.push(count as u8);
            let add_case = |dest: &mut Vec<u8>, text: &[u8]| {
                let len = text.len().min(usize::from(u16::MAX));
                // The original writes little endian despite its big-endian comment.
                dest.extend_from_slice(&(len as u16).to_le_bytes());
                dest.extend_from_slice(&text[..len]);
            };
            for i in 0..cases {
                let index = unsafe { (s.reverse_mapping)(lm, i32::from(i), false) };
                if let Some(text) = self.strings.get(&index) {
                    result.push(i + 1);
                    add_case(result, text);
                }
            }
            add_case(result, &self.strings[&0]);
        } else {
            if self.code == SCC_PLURAL_LIST {
                result.push(unsafe { (s.plural_form)(lm) } as u8);
            }
            result.push(self.offset.wrapping_sub(0x80) as u8);
            let count = if self.code == SCC_GENDER_LIST {
                unsafe { (s.num_genders)() }
            } else {
                s.max_plural_forms
            };
            let index = |i: u8| {
                let index = if self.code == SCC_GENDER_LIST {
                    unsafe { (s.reverse_mapping)(lm, i32::from(i), true) }
                } else {
                    i32::from(i) + 1
                };
                if self.strings.contains_key(&index) {
                    index
                } else {
                    0
                }
            };
            result.push(count);
            for i in 0..count {
                let len = self.strings[&index(i)].len();
                if len > usize::from(u8::MAX) {
                    s.message("choice list string is too long");
                }
                result.push(len.min(usize::from(u8::MAX)) as u8);
            }
            for i in 0..count {
                let text = &self.strings[&index(i)];
                result.extend_from_slice(&text[..text.len().min(usize::from(u8::MAX))]);
            }
        }
    }
}
#[derive(Clone, Copy)]
enum Destination {
    Main,
    Case(i32),
    Plural(i32),
}
fn dest<'a>(
    main: &'a mut Vec<u8>,
    case: &'a mut Option<ChoiceList>,
    plural: &'a mut Option<ChoiceList>,
    target: Destination,
) -> &'a mut Vec<u8> {
    match target {
        Destination::Main => main,
        Destination::Case(index) => case.as_mut().unwrap().strings.get_mut(&index).unwrap(),
        Destination::Plural(index) => plural.as_mut().unwrap().strings.get_mut(&index).unwrap(),
    }
}

pub(crate) fn translate(
    s: &Services,
    grfid: u32,
    language: u8,
    allow_newlines: bool,
    text: &[u8],
    byte80: u32,
) -> Vec<u8> {
    let mut input = Consumer { bytes: text };
    let identifier = utf8::decode(input.bytes);
    let unicode = identifier.length != 0 && identifier.codepoint == 0xde;
    if unicode {
        input.skip(identifier.length);
    }
    let (mut plural, mut case): (Option<ChoiceList>, Option<ChoiceList>) = (None, None);
    let mut case_destination = None;
    let mut target = Destination::Main;
    let mut output = Vec::new();
    while !input.bytes.is_empty() {
        let decoded = utf8::decode(input.bytes);
        let mut c = if unicode && decoded.length != 0 {
            input.skip(decoded.length);
            if (0xe000..=0xe0ff).contains(&decoded.codepoint) {
                decoded.codepoint - 0xe000
            } else if decoded.codepoint >= 0x20 {
                put(
                    dest(&mut output, &mut case, &mut plural, target),
                    if string_func::is_printable(decoded.codepoint) {
                        decoded.codepoint
                    } else {
                        u32::from(b'?')
                    },
                );
                continue;
            } else {
                decoded.codepoint
            }
        } else {
            input.read(1)
        };
        if c == 0 {
            break;
        }
        let mut codepoint = None;
        match c {
            0x01 => {
                input.skip(1);
                codepoint = Some(0x20);
            }
            0x0a => (),
            0x0d => {
                if allow_newlines {
                    codepoint = Some(0x0a);
                } else {
                    s.message("Detected newline in string that does not allow one");
                }
            }
            0x0e => codepoint = Some(SCC_TINYFONT),
            0x0f => codepoint = Some(SCC_BIGFONT),
            0x1f => {
                input.skip(1);
                input.skip(1);
                codepoint = Some(0x20);
            }
            0x7b..=0x7f => codepoint = Some(SCC_NEWGRF_PRINT_DWORD_SIGNED + c - 0x7b),
            0x80 => codepoint = Some(byte80),
            0x81 => {
                let string = input.read(2);
                put(
                    dest(&mut output, &mut case, &mut plural, target),
                    SCC_NEWGRF_STRINL,
                );
                codepoint = Some(unsafe { super::openttd_rust_grf_text_map(s, grfid, string) });
            }
            0x82..=0x84 => codepoint = Some(SCC_NEWGRF_PRINT_WORD_DATE_LONG + c - 0x82),
            0x85 => codepoint = Some(SCC_NEWGRF_DISCARD_WORD),
            0x86 => codepoint = Some(SCC_NEWGRF_ROTATE_TOP_4_WORDS),
            0x87 => codepoint = Some(SCC_NEWGRF_PRINT_WORD_VOLUME_LONG),
            0x88..=0x98 => {
                codepoint = Some(
                    [
                        SCC_BLUE,
                        SCC_SILVER,
                        SCC_GOLD,
                        SCC_RED,
                        SCC_PURPLE,
                        SCC_LTBROWN,
                        SCC_ORANGE,
                        SCC_GREEN,
                        SCC_YELLOW,
                        SCC_DKGREEN,
                        SCC_CREAM,
                        SCC_BROWN,
                        SCC_WHITE,
                        SCC_LTBLUE,
                        SCC_GRAY,
                        SCC_DKBLUE,
                        SCC_BLACK,
                    ][(c - 0x88) as usize],
                );
            }
            0x9a => {
                let code = input.read(1);
                match code {
                    0x00 => break,
                    0x01 => codepoint = Some(SCC_NEWGRF_PRINT_QWORD_CURRENCY),
                    0x03 => {
                        let value = input.read(2);
                        put(
                            dest(&mut output, &mut case, &mut plural, target),
                            SCC_NEWGRF_PUSH_WORD,
                        );
                        codepoint = Some(value);
                    }
                    0x06..=0x08 => {
                        codepoint = Some(
                            [
                                SCC_NEWGRF_PRINT_BYTE_HEX,
                                SCC_NEWGRF_PRINT_WORD_HEX,
                                SCC_NEWGRF_PRINT_DWORD_HEX,
                            ][(code - 0x06) as usize],
                        );
                    }
                    0x0b => codepoint = Some(SCC_NEWGRF_PRINT_QWORD_HEX),
                    0x0c => codepoint = Some(SCC_NEWGRF_PRINT_WORD_STATION_NAME),
                    0x0d => codepoint = Some(SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG),
                    0x0e | 0x0f => {
                        let lm = unsafe { (s.language_map)(grfid, language) };
                        let index = input.read(1);
                        let mapped = if lm.is_null() {
                            -1
                        } else {
                            unsafe { (s.mapping)(lm, i32::try_from(index).unwrap(), code == 0x0e) }
                        };
                        if mapped >= 0 {
                            put(
                                dest(&mut output, &mut case, &mut plural, target),
                                if code == 0x0e {
                                    SCC_GENDER_INDEX
                                } else {
                                    SCC_SET_CASE
                                },
                            );
                            codepoint =
                                Some((if code == 0x0e { mapped } else { mapped + 1 }) as u32);
                        }
                    }
                    0x10 | 0x11 => {
                        if plural.is_none() && case.is_none() {
                            if code == 0x10 {
                                input.skip(1);
                            }
                            s.message(if code == 0x10 {
                                "choice list next marker found when not expected"
                            } else {
                                "choice list default marker found when not expected"
                            });
                        } else {
                            let is_plural = plural.is_some();
                            let mapping = if is_plural { &mut plural } else { &mut case };
                            let index = if code == 0x10 {
                                i32::try_from(input.read(1)).unwrap()
                            } else {
                                0
                            };
                            if let std::collections::btree_map::Entry::Vacant(entry) =
                                mapping.as_mut().unwrap().strings.entry(index)
                            {
                                entry.insert(Vec::new());
                                target = if is_plural {
                                    Destination::Plural(index)
                                } else {
                                    case_destination = Some(index);
                                    Destination::Case(index)
                                };
                            } else {
                                s.message("duplicate choice list string, ignoring");
                            }
                        }
                    }
                    0x12 => {
                        if plural.is_none() && case.is_none() {
                            s.message("choice list end marker found when not expected");
                        } else {
                            let is_plural = plural.is_some();
                            let mapping = if is_plural { &mut plural } else { &mut case };
                            let mut mapping = mapping.take().unwrap();
                            let lm = unsafe { (s.language_map)(grfid, language) };
                            target = if is_plural {
                                case_destination.map_or(Destination::Main, Destination::Case)
                            } else {
                                Destination::Main
                            };
                            if !is_plural {
                                case_destination = None;
                            }
                            mapping.flush(s, lm, dest(&mut output, &mut case, &mut plural, target));
                        }
                    }
                    0x13..=0x15 => {
                        let has_plural = plural.is_some();
                        let mapping = if code == 0x14 { &mut case } else { &mut plural };
                        if mapping.is_some() || has_plural {
                            s.message(
                                "choice lists can't be stacked, it's going to get messy now...",
                            );
                            if code != 0x14 {
                                input.skip(1);
                            }
                        } else {
                            *mapping = Some(ChoiceList {
                                code: [SCC_GENDER_LIST, SCC_SWITCH_CASE, SCC_PLURAL_LIST]
                                    [(code - 0x13) as usize],
                                offset: if code == 0x14 { 0 } else { input.read(1) },
                                strings: BTreeMap::new(),
                            });
                        }
                    }
                    0x16..=0x1e => codepoint = Some(SCC_NEWGRF_PRINT_DWORD_DATE_LONG + code - 0x16),
                    0x1f => codepoint = Some(SCC_PUSH_COLOUR),
                    0x20 => codepoint = Some(SCC_POP_COLOUR),
                    0x21 => codepoint = Some(SCC_NEWGRF_PRINT_DWORD_FORCE),
                    _ => s.message("missing handler for extended format code"),
                }
            }
            0x9b => codepoint = Some(SCC_TOWN),
            0x9c => codepoint = Some(SCC_CITY),
            0x9e => codepoint = Some(0x20ac),
            0x9f => codepoint = Some(0x0178),
            0xa0 => codepoint = Some(SCC_UP_ARROW),
            0xaa => codepoint = Some(SCC_DOWN_ARROW),
            0xac => codepoint = Some(SCC_CHECKMARK),
            0xad => codepoint = Some(SCC_CROSS),
            0xaf => codepoint = Some(SCC_RIGHT_ARROW),
            0xb4..=0xb9 => {
                codepoint = Some(
                    [
                        SCC_TRAIN,
                        SCC_LORRY,
                        SCC_BUS,
                        SCC_PLANE,
                        SCC_SHIP,
                        SCC_SUPERSCRIPT_M1,
                    ][(c - 0xb4) as usize],
                );
            }
            0xbc => codepoint = Some(SCC_SMALL_UP_ARROW),
            0xbd => codepoint = Some(SCC_SMALL_DOWN_ARROW),
            _ => {
                if !string_func::is_printable(c) {
                    c = u32::from(b'?');
                }
                codepoint = Some(c);
            }
        }
        if let Some(codepoint) = codepoint {
            put(dest(&mut output, &mut case, &mut plural, target), codepoint);
        }
    }
    if plural.is_some() || case.is_some() {
        s.message("choice list was incomplete, the whole list is ignored");
    }
    output
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    /// `GrfMsg` severities and texts recorded on this thread since the last call.
    fn messages() -> Vec<(i32, Vec<u8>)> {
        crate::debug::tests::take()
            .into_iter()
            .map(|(_, level, text)| (level, text))
            .collect()
    }
    unsafe extern "C" fn language_map(_: u32, language: u8) -> *const c_void {
        if language == 1 {
            std::ptr::dangling()
        } else {
            std::ptr::null()
        }
    }
    unsafe extern "C" fn mapping(_: *const c_void, index: i32, gender: bool) -> i32 {
        match (index, gender) {
            (7, true) => 0,
            (8, false) => 1,
            _ => -1,
        }
    }
    unsafe extern "C" fn reverse_mapping(_: *const c_void, index: i32, gender: bool) -> i32 {
        match (index, gender) {
            (0, true) => 7,
            (1, true) => 6,
            (0, false) => 8,
            (1, false) => 9,
            _ => -1,
        }
    }
    unsafe extern "C" fn two() -> u8 {
        2
    }
    unsafe extern "C" fn plural(_: *const c_void) -> i32 {
        3
    }
    unsafe extern "C" fn legacy_string(string: u32) -> u32 {
        0x20000 + string
    }
    pub(crate) fn services() -> Services {
        Services {
            empty: 1,
            language_map,
            legacy_string,
            mapping,
            max_plural_forms: 2,
            num_cases: two,
            num_genders: two,
            plural_form: plural,
            reverse_mapping,
            undefined: 2,
        }
    }
    #[test]
    fn byte_unicode_and_choice_semantics() {
        let services = services();
        let cases: &[(&[u8], u8, bool, &[u8])] = &[
            (b"a\x00ignored", 1, true, b"a"),
            (b"\x81\x01", 1, false, b"\xee\x81\xb4\xf0\xa0\x80\x80"),
            (b"\x9a\x13\x80\x9a\x11D\x9a\x11ignored\x9a\x12", 99, false, b"Dignored"),
            (b"\x9a\x13\x80\x9a\x10\x07M\x9a\x12", 99, false, b""),
            (b"\x01X\x1fYZ\x0a\x0d\x0e\x0f\x80\x9e\x9f", 1, true, b"  \n\xee\x80\x85\xee\x80\x86\x7f\xe2\x82\xac\xc5\xb8"),
            (b"\xc3\x9e\xe2\x82\xac\xee\x82\x80\xff\xed\xa0\x80", 1, false, b"\xe2\x82\xac\x7f\xc3\xbf\xed\xa0\x80"),
            (b"\xc3\x9e\xee\x87\x80\x9e", 1, false, b"?\xe2\x82\xac"),
            (b"before\x9a\x00after", 1, false, b"before"),
            (b"\x9a\x03\x00\x00", 1, false, b"\xee\x81\xb1\x00"),
            (b"\x9a\x0e\x07\x9a\x0f\x08\x9a\x0e\x04", 1, false, b"\xee\x80\xbe\x00\xee\x81\x80\x02"),
            (b"\x9a\x13\x80\x9a\x11D\x9a\x10\x07M\x9a\x12", 1, false, b"\xee\x80\xbd\x00\x02\x01\x01MD"),
            (b"\x9a\x13\x80\x9a\x11D\x9a\x10\x07M\x9a\x12", 99, false, b"D"),
            (b"\x9a\x14\x9a\x11D\x9a\x10\x08C\x9a\x12", 1, false, b"\xee\x81\x81\x01\x01\x01\x00C\x01\x00D"),
            (b"\x9a\x15\x82\x9a\x11D\x9a\x10\x01A\x9a\x12", 1, false, b"\xee\x80\xbc\x03\x02\x02\x01\x01AD"),
            (b"\x9a\x14\x9a\x11D\x9a\x10\x08C\x9a\x15\x80\x9a\x11p0\x9a\x10\x01p1\x9a\x12\x9a\x12", 1, false, b"\xee\x81\x81\x01\x01\x0d\x00C\xee\x80\xbc\x03\x00\x02\x02\x02p1p0\x01\x00D"),
            (b"out\x9a\x14\x9a\x11lost", 1, false, b"out"),
        ];
        for &(input, language, newlines, expected) in cases {
            assert_eq!(
                translate(&services, 123, language, newlines, input, 0x7f),
                expected,
                "input {input:?}"
            );
        }
        let mut saturated = b"\x9a\x13\x80\x9a\x11".to_vec();
        saturated.extend_from_slice(&[b'D'; 256]);
        saturated.extend_from_slice(b"\x9a\x12");
        let mut expected = b"\xee\x80\xbd\x00\x02\xff\xff".to_vec();
        expected.extend_from_slice(&[b'D'; 510]);
        assert_eq!(translate(&services, 123, 1, false, &saturated, 0), expected);
        messages();
        assert_eq!(
            translate(
                &services,
                123,
                1,
                false,
                b"a\x0d\x9a\x10\x07\x9a\x12\x9a\x22\x01",
                0
            ),
            b"a "
        );
        assert_eq!(
            messages(),
            [
                (
                    1,
                    b"Detected newline in string that does not allow one".to_vec()
                ),
                (
                    1,
                    b"choice list next marker found when not expected".to_vec()
                ),
                (
                    1,
                    b"choice list end marker found when not expected".to_vec()
                ),
                (1, b"missing handler for extended format code".to_vec()),
                (0, b"Source buffer too short: 1 > 0".to_vec()),
            ]
        );
    }
}

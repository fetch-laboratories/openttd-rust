/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `StringConsumer` borrowed offsets, partial binary reads, separator decisions and byte
//! sets (#736).
//!
//! Moved from `src/tests/string_consumer_bytes.cpp`, whose expectations were fork-written
//! literals. The pinned original's `StringConsumer` ran the same four cases, and its
//! observations are frozen in `cursor-oracle.txt` (`python3 tools/oracle-capture.py
//! string_consumer`). [`Consumer`] is the candidate C++ glue over the production helpers
//! behind `openttd_rust_consumer_*`; the operations below add the glue's conditional,
//! separator and byte-set reads line for line. A borrowed view is observed as its offset
//! in the case's source and its length; the default-constructed view has no data pointer,
//! and the glue's `substr` of it stays null.
use super::tests::Consumer;
use super::{FIND_STRING, SeparatorUsage, character, find, prefix, until};
use crate::oracle;

const ORACLE: &str = include_str!("cursor-oracle.txt");

/// The candidate glue's reads that `tests::Consumer` does not carry.
impl<'a> Consumer<'a> {
    fn left_data(&self) -> &'a [u8] {
        &self.src[self.position..]
    }

    /// `PeekIf` through `MatchPrefix`.
    fn peek_if(&self, str: &[u8]) -> bool {
        prefix(self.left_data(), str).matched != 0
    }

    /// `ReadIf`: advance by the matched length.
    fn read_if(&mut self, str: &[u8]) -> bool {
        let result = prefix(self.left_data(), str);
        self.position += result.length;
        result.matched != 0
    }

    /// `SkipIf`.
    fn skip_if(&mut self, str: &[u8]) {
        self.position += prefix(self.left_data(), str).length;
    }

    /// `SkipAll`.
    fn skip_all(&mut self) {
        self.position = self.src.len();
    }

    /// `PeekUntil` through `SeparatorResult`.
    fn peek_until(&self, str: &[u8], sep: SeparatorUsage) -> &'a [u8] {
        let lengths = until(self.left_data(), str, sep);
        &self.left_data()[..lengths.result_length]
    }

    /// `ReadUntil`.
    fn read_until(&mut self, str: &[u8], sep: SeparatorUsage) -> &'a [u8] {
        let lengths = until(self.left_data(), str, sep);
        let result = &self.left_data()[..lengths.result_length];
        self.position += lengths.consumed_length;
        result
    }

    /// `SkipUntil`.
    fn skip_until(&mut self, str: &[u8], sep: SeparatorUsage) {
        self.position += until(self.left_data(), str, sep).consumed_length;
    }

    /// `Find` through `FindBytes` mode 0.
    fn find(&self, str: &[u8]) -> usize {
        find(self.left_data(), str, FIND_STRING)
    }

    /// `PeekCharIfIn` (`member`) or `PeekCharIfNotIn` through `MatchCharacter`.
    fn peek_char_if(&self, chars: &[u8], member: bool) -> Option<u64> {
        let result = character(self.left_data(), chars, member);
        (result.length != 0).then_some(result.value_bits)
    }

    /// `ReadCharIfIn` or `ReadCharIfNotIn`: skip one byte on a match.
    fn read_char_if(&mut self, chars: &[u8], member: bool) -> Option<u64> {
        let result = self.peek_char_if(chars, member);
        if result.is_some() {
            self.skip(1);
        }
        result
    }

    /// `SkipCharIfIn` or `SkipCharIfNotIn`.
    fn skip_char_if(&mut self, chars: &[u8], member: bool) {
        if self.peek_char_if(chars, member).is_some() {
            self.skip(1);
        }
    }
}

/// One case's observations in check order, each named by its operation.
#[derive(Default)]
struct Rows(Vec<(String, String)>);

impl Rows {
    fn row(&mut self, name: &str, value: String) {
        let check = self.0.len();
        self.0.push((format!("check {check} {name}"), value));
    }

    fn flag(&mut self, name: &str, value: bool) {
        self.row(name, u8::from(value).to_string());
    }

    fn count(&mut self, name: &str, value: usize) {
        let text = if value == usize::MAX {
            "npos".into()
        } else {
            value.to_string()
        };
        self.row(name, text);
    }

    /// A view borrowed from `base` as its offset and length; `None` is the null source.
    fn view(&mut self, name: &str, base: Option<&[u8]>, view: &[u8]) {
        let text = match base {
            // Every view the glue returns is a subslice of its source.
            Some(base) => format!(
                "{} {}",
                view.as_ptr().addr() - base.as_ptr().addr(),
                view.len()
            ),
            None => format!("null {}", view.len()),
        };
        self.row(name, text);
    }

    /// An optional value as the capture's `int64_t`, after the C++ result type's cast.
    fn value(&mut self, name: &str, value: Option<i64>) {
        self.row(
            name,
            value.map_or_else(|| "-".into(), |value| value.to_string()),
        );
    }
}

fn prefix_case() -> Rows {
    let mut r = Rows::default();
    let data: &[u8] = b"X\xFF\0Z";
    let mut consumer = Consumer::new(data);
    consumer.skip(1);
    r.view("Peek(0)", Some(data), consumer.peek(0));
    let read = consumer.read(0);
    r.view("Read(0)", Some(data), read);
    r.flag("PeekIf()", consumer.peek_if(b""));
    let matched = consumer.read_if(b"");
    r.flag("ReadIf()", matched);
    consumer.skip_if(b"");
    r.count("GetBytesRead", consumer.position);
    r.view(
        "Peek(SIZE_MAX - 1)",
        Some(data),
        consumer.peek(usize::MAX - 1),
    );
    consumer.skip_all();
    r.flag("PeekIf()", consumer.peek_if(b""));
    let matched = consumer.read_if(b"");
    r.flag("ReadIf()", matched);
    let matched = consumer.read_if(b"Z");
    r.flag("ReadIf(Z)", matched);
    r.count("GetBytesRead", consumer.position);
    r.view("Peek(0)", Some(data), consumer.peek(0));
    let read = consumer.read(usize::MAX);
    r.view("Read(npos)", Some(data), read);
    let mut empty = Consumer::new(&[]);
    let matched = empty.read_if(b"");
    r.flag("empty ReadIf()", matched);
    r.view("empty Peek(0)", None, empty.peek(0));
    let read = empty.read(0);
    r.view("empty Read(0)", None, read);
    r
}

fn binary_case() -> Rows {
    let mut r = Rows::default();
    let mut consumer = Consumer::new(b"X\x01\x02\x03");
    consumer.skip(1);
    let value = consumer.try_read_binary(4);
    r.value("TryReadUint32LE", value.map(|bits| i64::from(bits as u32)));
    let value = consumer.try_read_binary(4);
    r.value(
        "TryReadSint32LE",
        value.map(|bits| i64::from(bits as u32 as i32)),
    );
    let value = consumer.try_read_binary(8);
    r.value("TryReadUint64LE", value.map(|bits| bits as i64));
    let value = consumer.try_read_binary(8);
    r.value("TryReadSint64LE", value.map(|bits| bits as i64));
    r.count("GetBytesRead", consumer.position);
    let value = consumer.try_read_binary(2);
    r.value("TryReadUint16LE", value.map(|bits| i64::from(bits as u16)));
    r.count("GetBytesRead", consumer.position);
    let value = consumer.try_read_binary(2);
    r.value("TryReadUint16LE", value.map(|bits| i64::from(bits as u16)));
    let value = consumer.try_read_binary(2);
    r.value(
        "TryReadSint16LE",
        value.map(|bits| i64::from(bits as u16 as i16)),
    );
    r.count("GetBytesRead", consumer.position);
    let value = consumer.read_binary(2, 42);
    r.row("ReadUint16LE(42)", (value as u16).to_string());
    r.count("GetBytesRead", consumer.position);
    let value = consumer.try_read_binary(1);
    r.value("TryReadUint8", value.map(|bits| i64::from(bits as u8)));
    let value = consumer.try_read_binary(1);
    r.value(
        "TryReadSint8",
        value.map(|bits| i64::from(bits as u8 as i8)),
    );
    let value = consumer.try_read_binary(1);
    r.value("TryReadChar", value.map(|bits| i64::from(bits as u8 as i8)));
    r.count("GetBytesRead", consumer.position);
    r
}

fn separator_case() -> Rows {
    let mut r = Rows::default();
    let data: &[u8] = b"xxabc<><>tail";
    // Every value of the enum's representable range, named or not.
    for policy in 0..8 {
        let sep = SeparatorUsage(policy);
        let (mut read, mut skip) = (Consumer::new(data), Consumer::new(data));
        read.skip(2);
        skip.skip(2);
        r.view(
            &format!("{policy} PeekUntil"),
            Some(data),
            read.peek_until(b"<>", sep),
        );
        r.count(&format!("{policy} GetBytesRead"), read.position);
        let result = read.read_until(b"<>", sep);
        r.view(&format!("{policy} ReadUntil"), Some(data), result);
        r.count(&format!("{policy} GetBytesRead"), read.position);
        skip.skip_until(b"<>", sep);
        r.count(&format!("{policy} SkipUntil GetBytesRead"), skip.position);
    }
    let overlapping_data: &[u8] = b"abababaZ";
    let mut overlapping = Consumer::new(overlapping_data);
    let result = overlapping.read_until(b"aba", SeparatorUsage::READ_ALL_SEPARATORS);
    r.view("overlapping ReadUntil", Some(overlapping_data), result);
    r.view(
        "overlapping GetLeftData",
        Some(overlapping_data),
        overlapping.left_data(),
    );
    let mut missing = Consumer::new(data);
    missing.skip(2);
    let result = missing.read_until(b"missing", SeparatorUsage::SKIP_ALL_SEPARATORS);
    r.view("missing ReadUntil", Some(data), result);
    r.count("missing GetBytesRead", missing.position);
    let result = missing.read_until(b"<>", SeparatorUsage::READ_ALL_SEPARATORS);
    r.view("missing ReadUntil at end", Some(data), result);
    r
}

fn byteset_case() -> Rows {
    let mut r = Rows::default();
    let data: &[u8] = b"X\xFF\0ab";
    let char_value = |value: Option<u64>| value.map(|bits| i64::from(bits as u8 as i8));
    let mut consumer = Consumer::new(data);
    consumer.skip(1);
    // The pattern overlaps the read-only input.
    r.count("Find", consumer.find(&data[1..3]));
    let matched = consumer.read_if(&data[1..2]);
    r.flag("ReadIf", matched);
    r.count("GetBytesRead", consumer.position);
    r.value(
        "PeekCharIfNotIn(NUL)",
        char_value(consumer.peek_char_if(b"\0", false)),
    );
    let value = consumer.read_char_if(b"\0", false);
    r.value("ReadCharIfNotIn(NUL)", char_value(value));
    consumer.skip_char_if(b"\0", false);
    r.count("GetBytesRead", consumer.position);
    let value = consumer.read_char_if(b"\0", true);
    r.value("ReadCharIfIn(NUL)", char_value(value));
    r.value(
        "PeekCharIfNotIn(x)",
        char_value(consumer.peek_char_if(b"x", false)),
    );
    let value = consumer.read_char_if(b"x", false);
    r.value("ReadCharIfNotIn(x)", char_value(value));
    consumer.skip_char_if(b"x", false);
    r.count("GetBytesRead", consumer.position);
    let value = consumer.read_char_if(b"x", false);
    r.value("ReadCharIfNotIn(x)", char_value(value));
    let value = consumer.read_char_if(b"x", true);
    r.value("ReadCharIfIn(x)", char_value(value));
    r.count("GetBytesRead", consumer.position);
    r
}

#[test]
fn consumer_cursor_cases_match_original() {
    let sections = oracle::parse(ORACLE);
    for (name, case, checks) in [
        ("prefix", prefix_case as fn() -> Rows, 15),
        ("binary", binary_case, 16),
        ("separator", separator_case, 45),
        ("byteset", byteset_case, 13),
    ] {
        assert_eq!(oracle::check(&sections, name, case().0), checks, "{name}");
    }
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `Debug` from `debug.h` and `GrfMsg` from `newgrf.h`.
//!
//! Each macro reads the native `_debug_<category>_level` in place and formats only when
//! the level passes, so arguments are evaluated where the original evaluates them. Rust
//! `format_args!` matches `fmt` for the specifiers in use on unsigned integers, `bool` and
//! ASCII text. Native bytes, such as file names and `{:c}` arguments, go through [`Bytes`]
//! and [`Char`], which keep every byte. The bound table lives for the process; level
//! loads race with console writes exactly as the native macro's do.
#![allow(unsafe_code)]
use std::ffi::c_int;
use std::fmt::{self, Write as _};
use std::sync::atomic::{AtomicPtr, Ordering};

/// Native debug levels and the two print leaves.
#[repr(C)]
pub struct Services {
    pub console: *const c_int,
    pub desync: *const c_int,
    pub driver: *const c_int,
    pub fontcache: *const c_int,
    pub gamelog: *const c_int,
    pub grf: *const c_int,
    /// `GrfMsgI`, which adds the `_cur_gps` file and line prefix.
    pub grf_msg: extern "C" fn(c_int, *const u8, usize),
    pub map: *const c_int,
    pub misc: *const c_int,
    pub net: *const c_int,
    pub oldloader: *const c_int,
    /// `DebugPrint(category, level, message)`.
    pub print: extern "C" fn(*const u8, usize, c_int, *const u8, usize),
    pub script: *const c_int,
    pub sl: *const c_int,
    pub sprite: *const c_int,
    pub yapf: *const c_int,
}
// SAFETY: The levels are process-lifetime native globals; the leaves are thread-safe as
// the native `DebugPrint` is.
unsafe impl Sync for Services {}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDebugServices" ["debug"] => Services { console, desync, driver, fontcache, gamelog, grf, grf_msg, map, misc, net, oldloader, print, script, sl, sprite, yapf };
}

/// Levels and sinks before the native table binds: nothing above level 0 passes and
/// output is discarded. Only Rust unit tests that never bind run against it.
static UNBOUND_LEVEL: c_int = 0;
extern "C" fn discard_grf_msg(_: c_int, _: *const u8, _: usize) {}
extern "C" fn discard_print(_: *const u8, _: usize, _: c_int, _: *const u8, _: usize) {}
static UNBOUND: Services = Services {
    console: &raw const UNBOUND_LEVEL,
    desync: &raw const UNBOUND_LEVEL,
    driver: &raw const UNBOUND_LEVEL,
    fontcache: &raw const UNBOUND_LEVEL,
    gamelog: &raw const UNBOUND_LEVEL,
    grf: &raw const UNBOUND_LEVEL,
    grf_msg: discard_grf_msg,
    map: &raw const UNBOUND_LEVEL,
    misc: &raw const UNBOUND_LEVEL,
    net: &raw const UNBOUND_LEVEL,
    oldloader: &raw const UNBOUND_LEVEL,
    print: discard_print,
    script: &raw const UNBOUND_LEVEL,
    sl: &raw const UNBOUND_LEVEL,
    sprite: &raw const UNBOUND_LEVEL,
    yapf: &raw const UNBOUND_LEVEL,
};
static SERVICES: AtomicPtr<Services> = AtomicPtr::new((&raw const UNBOUND).cast_mut());

/// Bind the native table during static initialization.
///
/// # Safety
/// `services` and every level it points to live for the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_debug_bind(services: *const Services) {
    SERVICES.store(services.cast_mut(), Ordering::Release);
}

fn services() -> &'static Services {
    // SAFETY: Either `UNBOUND` or a process-lifetime table from `openttd_rust_debug_bind`.
    unsafe { &*SERVICES.load(Ordering::Acquire) }
}

/// Current `_debug_<category>_level`, loaded at the macro's observation point.
#[inline]
pub(crate) fn level(category: fn(&Services) -> *const c_int) -> c_int {
    // SAFETY: Bound level addresses are native globals that live for the process.
    unsafe { *category(services()) }
}

/// Bytes at or above 0x80 from [`Bytes`] and [`Char`] travel through `fmt` as these
/// private-use scalars and leave the sink as the original byte. Format strings and other
/// arguments are ASCII, so they never contain them.
const ESCAPE: u32 = 0xF700;

fn escape(byte: u8) -> char {
    if byte < 0x80 {
        char::from(byte)
    } else {
        char::from_u32(ESCAPE + u32::from(byte)).unwrap_or(char::REPLACEMENT_CHARACTER)
    }
}

/// Native bytes formatted with `{}`, kept as they are, such as `std::string` file names.
pub(crate) struct Bytes<'a>(pub &'a [u8]);
impl fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|&byte| f.write_char(escape(byte)))
    }
}

/// An integer formatted with `fmt`'s `{:c}`: its low byte, as `char` conversion keeps it.
pub(crate) struct Char(pub u8);
impl fmt::Display for Char {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char(escape(self.0))
    }
}

struct Sink(Vec<u8>);
impl fmt::Write for Sink {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match u32::from(c).checked_sub(ESCAPE) {
                Some(byte @ 0x80..=0xFF) => self.0.push(byte.to_le_bytes()[0]),
                _ => self
                    .0
                    .extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
            }
        }
        Ok(())
    }
}

/// The message bytes `fmt::format` would produce.
pub(crate) fn format(args: fmt::Arguments<'_>) -> Vec<u8> {
    let mut sink = Sink(Vec::new());
    // Neither the sink nor any argument's `Display` reports an error.
    let _ = sink.write_fmt(args);
    sink.0
}

/// `DebugPrint(category, level, fmt::format(...))`.
pub(crate) fn print(category: &'static str, level: c_int, args: fmt::Arguments<'_>) {
    let message = format(args);
    (services().print)(
        category.as_ptr(),
        category.len(),
        level,
        message.as_ptr(),
        message.len(),
    );
}

/// `GrfMsgI(severity, fmt::format(...))`.
pub(crate) fn grf_msg_print(severity: c_int, args: fmt::Arguments<'_>) {
    let message = format(args);
    (services().grf_msg)(severity, message.as_ptr(), message.len());
}

/// `Debug(category, level, format, ...)`: level 0 always prints.
macro_rules! debug {
    ($category:ident, $level:expr, $($arg:tt)+) => {{
        let level: ::std::ffi::c_int = $level;
        if level == 0 || $crate::debug::level(|s| s.$category) >= level {
            $crate::debug::print(stringify!($category), level, format_args!($($arg)+));
        }
    }};
}
pub(crate) use debug;

/// `GrfMsg(severity, format, ...)`: gated on `_debug_grf_level` like `Debug(grf, ...)`.
macro_rules! grf_msg {
    ($severity:expr, $($arg:tt)+) => {{
        let severity: ::std::ffi::c_int = $severity;
        if severity == 0 || $crate::debug::level(|s| s.grf) >= severity {
            $crate::debug::grf_msg_print(severity, format_args!($($arg)+));
        }
    }};
}
pub(crate) use grf_msg;

#[cfg(test)]
mod tests {
    use super::{Bytes, Char, Services, format};
    use std::cell::RefCell;
    use std::ffi::c_int;
    use std::sync::Once;

    thread_local! {
        static LINES: RefCell<Vec<(String, c_int, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn record(
        category: *const u8,
        len: usize,
        level: c_int,
        data: *const u8,
        size: usize,
    ) {
        let category = unsafe { std::slice::from_raw_parts(category, len) };
        let message = unsafe { std::slice::from_raw_parts(data, size) }.to_vec();
        LINES.with(|l| {
            l.borrow_mut().push((
                String::from_utf8(category.to_vec()).unwrap(),
                level,
                message,
            ));
        });
    }
    extern "C" fn record_grf(severity: c_int, data: *const u8, size: usize) {
        record(b"GrfMsgI".as_ptr(), 7, severity, data, size);
    }
    /// Fixed levels shared by every test thread: grf 5, misc 0, net 9, others 1.
    static GRF: c_int = 5;
    static MISC: c_int = 0;
    static NET: c_int = 9;
    static OTHER: c_int = 1;
    static TABLE: Services = Services {
        console: &raw const OTHER,
        desync: &raw const OTHER,
        driver: &raw const OTHER,
        fontcache: &raw const OTHER,
        gamelog: &raw const OTHER,
        grf: &raw const GRF,
        grf_msg: record_grf,
        map: &raw const OTHER,
        misc: &raw const MISC,
        net: &raw const NET,
        oldloader: &raw const OTHER,
        print: record,
        script: &raw const OTHER,
        sl: &raw const OTHER,
        sprite: &raw const OTHER,
        yapf: &raw const OTHER,
    };
    fn take() -> Vec<(String, c_int, Vec<u8>)> {
        static BIND: Once = Once::new();
        BIND.call_once(|| unsafe { super::openttd_rust_debug_bind(&raw const TABLE) });
        LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
    }

    #[test]
    fn level_gate_matches_debug_and_grf_msg_macros() {
        take();
        let mut evaluated = 0;
        let mut arg = || {
            evaluated += 1;
            evaluated
        };
        super::debug!(misc, 0, "always {}", arg());
        super::debug!(misc, 1, "hidden {}", arg());
        super::debug!(grf, 5, "at level");
        super::debug!(grf, 6, "above level {}", arg());
        super::debug!(net, 9, "net");
        super::grf_msg!(0, "grf zero");
        super::grf_msg!(5, "grf five {}", arg());
        super::grf_msg!(6, "grf six {}", arg());
        // Arguments of suppressed messages are not evaluated, as in the native macros.
        assert_eq!(evaluated, 2);
        let line = |category: &str, level: c_int, text: &str| {
            (category.to_owned(), level, text.as_bytes().to_vec())
        };
        assert_eq!(
            take(),
            [
                line("misc", 0, "always 1"),
                line("grf", 5, "at level"),
                line("net", 9, "net"),
                line("GrfMsgI", 0, "grf zero"),
                line("GrfMsgI", 5, "grf five 2"),
            ]
        );
    }

    #[test]
    fn specifiers_match_fmt() {
        // Expected text is `fmt::format` of the same values.
        let cases: [(Vec<u8>, &[u8]); 14] = [
            (format(format_args!("{}", 0u8)), b"0"),
            (format(format_args!("{}", u32::MAX)), b"4294967295"),
            (format(format_args!("{}", -5i32)), b"-5"),
            (format(format_args!("{}", true)), b"true"),
            (format(format_args!("{:X}", 0xABu8)), b"AB"),
            (format(format_args!("{:x}", 0xABCDu32)), b"abcd"),
            (format(format_args!("{:02X}", 0x7u8)), b"07"),
            (format(format_args!("{:02X}", 0x1234u16)), b"1234"),
            (format(format_args!("{:04X}", 0xBEu32)), b"00BE"),
            (
                format(format_args!("{:08X}", 0x0102_0304u32.swap_bytes())),
                b"04030201",
            ),
            (format(format_args!("{:08x}", 0xDEADu32)), b"0000dead"),
            (
                format(format_args!(
                    "{}{}{}{}",
                    Char(b'A'),
                    Char(0),
                    Char(0xE9),
                    Char(0x7F)
                )),
                b"A\0\xE9\x7F",
            ),
            (
                format(format_args!("'{}'", Bytes(b"caf\xC3\xA9 \xFF\xEF\x9C\x80"))),
                b"'caf\xC3\xA9 \xFF\xEF\x9C\x80'",
            ),
            (format(format_args!("{} {}", "text", Bytes(b""))), b"text "),
        ];
        for (actual, expected) in cases {
            assert_eq!(actual, expected);
        }
    }
}

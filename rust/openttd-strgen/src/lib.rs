/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Ports of the string generator (`src/strgen`). For now this holds only the master file's
//! string-ID assignment, which `openttd-kernels`' build script runs to define `table::strings`
//! while C++ strgen still writes `table/strings.h`; the binary holds the two to the same values.

pub mod strgen_base;
pub mod table_strings;

/// Seeded negative control for #1013 (never merged): two idle threads, then a
/// real SIGSEGV fault, so CI shows every thread's stack. A signal sent with
/// `kill` would not do: Rust's handler returns from a SIGSEGV that is no fault.
#[cfg(test)]
mod crash_control {
    #[test]
    #[allow(unsafe_code)] // The seeded fault itself.
    fn segfault_with_threads() {
        let idle = || std::thread::sleep(std::time::Duration::from_secs(60));
        let _first = std::thread::Builder::new()
            .name("idle-one".into())
            .spawn(idle);
        let _second = std::thread::Builder::new()
            .name("idle-two".into())
            .spawn(idle);
        std::thread::sleep(std::time::Duration::from_millis(200));
        // SAFETY: none; this write faults on purpose.
        unsafe { std::ptr::null_mut::<u8>().write_volatile(1) };
    }
}

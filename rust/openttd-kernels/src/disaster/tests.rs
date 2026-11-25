/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
#![allow(unsafe_code)]

/// `_disaster_delay` is one process-wide counter: a serialized worker thread sees and
/// writes the same storage as the thread that started it.
#[test]
fn disaster_delay_is_process_wide_across_serialized_threads() {
    let address = super::openttd_rust_disaster_delay() as usize;
    // SAFETY: The worker runs after this access and is joined before the next.
    unsafe {
        (address as *mut u16).write(41);
    }
    std::thread::spawn(move || {
        let worker = super::openttd_rust_disaster_delay();
        assert_eq!(worker as usize, address);
        // SAFETY: As above.
        unsafe {
            assert_eq!(worker.read(), 41);
            worker.write(73);
        }
    })
    .join()
    .unwrap();
    // SAFETY: As above.
    unsafe {
        assert_eq!((address as *const u16).read(), 73);
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Native `network/core/os_abstraction.h` socket handle widths and invalid value.
/// Windows uses the pointer-sized unsigned Winsock handle.
#[cfg(windows)]
pub type SocketHandle = usize;
/// Unix and Emscripten use an int file descriptor.
#[cfg(not(windows))]
pub type SocketHandle = i32;
/// `INVALID_SOCKET`, retaining all source bits at the platform's native width.
pub const INVALID_SOCKET: SocketHandle = !0;

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Opaque native station handles. Native code performs inheritance conversions, preserving
//! the C++ pointer adjustment; Rust never dereferences a handle. Handles stay live until their
//! owner is retired.

/// A native `Station`, distinct from the shared station/waypoint pool base.
#[repr(C)]
pub struct Station {
    _private: [u8; 0],
}

/// Opaque native entry-frame iterator storage; never read or retained by Rust.
#[repr(C)]
pub struct NearbyIndustryCursor {
    _private: [u8; 0],
}

/// A native `BaseStation`, which may also be a waypoint.
#[repr(C)]
pub struct BaseStation {
    _private: [u8; 0],
}

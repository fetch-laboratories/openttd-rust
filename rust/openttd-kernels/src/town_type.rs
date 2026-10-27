/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared town-rating constants from `town_type.h`.
/// Rating increase for improving a town bridge.
pub const RATING_TUNNEL_BRIDGE_UP_STEP: i32 = 50;
/// Rating decrease for removing a town tunnel or bridge.
pub const RATING_TUNNEL_BRIDGE_DOWN_STEP: i32 = -250;
/// Minimum rating after removing a town tunnel or bridge.
pub const RATING_TUNNEL_BRIDGE_MINIMUM: i32 = 0;
/// Maximum town rating.
pub const RATING_MAXIMUM: i32 = 1000;
/// Minimum town rating.
pub const RATING_MINIMUM: i32 = -1000;
/// Rating increase for a successful bribe.
pub const RATING_BRIBE_UP_STEP: i32 = 200;
/// Maximum rating a bribe can reach.
pub const RATING_BRIBE_MAXIMUM: i32 = 800;
/// Rating after a failed bribe.
pub const RATING_BRIBE_DOWN_TO: i32 = -50;

/// Native `TownID`; all uint16 values remain representable.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TownId(pub u16);

/// Opaque native `Town`; valid only while its native pool shell lives.
#[repr(C)]
pub struct TownHandle {
    _private: [u8; 0],
}

impl TownId {
    /// Native `TownID::Invalid()`.
    pub const INVALID: Self = Self(u16::MAX);
}

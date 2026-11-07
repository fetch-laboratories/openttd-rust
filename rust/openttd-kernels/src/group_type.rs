/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `group_type.h`.

/// Native `GroupID`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupID(pub u16);

impl GroupID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

impl GroupID {
    /// Sentinel for a to-be-created group.
    pub const NEW_GROUP: Self = Self(0xFFFC);
    /// All vehicles are in this group.
    pub const ALL_GROUP: Self = Self(0xFFFD);
    /// Ungrouped vehicles are in this group.
    pub const DEFAULT_GROUP: Self = Self(0xFFFE);
}
/// Maximum group name length, including the terminating character.
pub const MAX_LENGTH_GROUP_NAME_CHARS: usize = 32;

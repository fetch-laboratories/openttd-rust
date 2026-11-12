/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `newgrf_object.h`.

/// Native `ObjectFlag`: various object behaviours, retaining every underlying `u8`
/// value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectFlag(pub u8);

#[allow(non_upper_case_globals, reason = "ObjectFlag enumerator spelling")]
impl ObjectFlag {
    /// Object can only be constructed in the scenario editor.
    pub const OnlyInScenedit: Self = Self(0);
    /// Object can not be removed.
    pub const CannotRemove: Self = Self(1);
    /// Object get automatically removed (like "owned land").
    pub const Autoremove: Self = Self(2);
    /// Object can be built on water (not required).
    pub const BuiltOnWater: Self = Self(3);
    /// When object is cleared a positive income is generated instead of a cost.
    pub const ClearIncome: Self = Self(4);
    /// Do not display foundations when on a slope.
    pub const HasNoFoundation: Self = Self(5);
    /// Object has animated tiles.
    pub const Animation: Self = Self(6);
    /// Object can only be built in game.
    pub const OnlyInGame: Self = Self(7);
    /// Object wants 2CC colour mapping.
    pub const Uses2CC: Self = Self(8);
    /// Object can not be on land, implicitly sets `ObjectFlag::BuiltOnWater`.
    pub const NotOnLand: Self = Self(9);
    /// Object wants to be drawn on water.
    pub const DrawWater: Self = Self(10);
    /// Object can built under a bridge.
    pub const AllowUnderBridge: Self = Self(11);
    /// Object wants random bits in "next animation frame" callback.
    pub const AnimRandomBits: Self = Self(12);
    /// Object count is roughly scaled by water amount at edges.
    pub const ScaleByWater: Self = Self(13);
}

/// Native `ObjectFlags`, an `EnumBitSet<ObjectFlag, uint16_t>` retaining every
/// underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectFlags(pub u16);

impl ObjectFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: ObjectFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
}

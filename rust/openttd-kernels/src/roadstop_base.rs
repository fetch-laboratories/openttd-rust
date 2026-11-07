/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Road-stop status vocabulary from `roadstop_base.h`.

/// Native `RoadStop::RoadStopStatusFlag`, a bit position in the status byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadStopStatusFlag(pub u8);

#[allow(non_upper_case_globals)]
impl RoadStopStatusFlag {
    /// Non-zero when bay 0 is free.
    pub const Bay0Free: Self = Self(0);
    /// Non-zero when bay 1 is free.
    pub const Bay1Free: Self = Self(1);
    /// The entries on this stop are primary, i.e. the ones to delete.
    pub const BaseEntry: Self = Self(6);
    /// Non-zero when the road-stop entry is busy.
    pub const EntryBusy: Self = Self(7);
}

/// Native `RoadStop::RoadStopStatusFlags`; retains all eight status bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadStopStatusFlags(pub u8);

impl RoadStopStatusFlags {
    /// A set containing one status bit position.
    #[must_use]
    pub const fn from_element(element: RoadStopStatusFlag) -> Self {
        Self(1_u8 << element.0)
    }

    /// Native `EnumBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: RoadStopStatusFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }

    /// Native `EnumBitSet::Any`.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Set the named flag, preserving every other status bit.
    #[must_use]
    pub const fn with(self, element: RoadStopStatusFlag) -> Self {
        Self(self.0 | Self::from_element(element).0)
    }

    /// Reset the named flag, preserving every other status bit.
    #[must_use]
    pub const fn without(self, element: RoadStopStatusFlag) -> Self {
        Self(self.0 & !Self::from_element(element).0)
    }
}

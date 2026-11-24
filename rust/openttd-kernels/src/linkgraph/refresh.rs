/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Vocabulary of `linkgraph/refresh.h`. The `LinkRefresher` bodies are in `vehicle/visits.rs`.

/// `LinkRefresher::RefreshFlag`: various flags about properties of the last examined link
/// that might have an influence on the next one.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefreshFlag(pub u8);

#[allow(non_upper_case_globals, reason = "RefreshFlag enumerator spelling")]
impl RefreshFlag {
    /// There was a conditional jump. Try to use the given next order when looking for a new one.
    pub const UseNext: Self = Self(0);
    /// Consist could leave the last stop where it could interact with cargo carrying cargo
    /// (i.e. not an "unload all" + "no loading" order).
    pub const HasCargo: Self = Self(1);
    /// Consist was refit since the last stop where it could interact with cargo.
    pub const WasRefit: Self = Self(2);
    /// Consist had a chance to load since the last refit and the refit capacities can be reset.
    pub const ResetRefit: Self = Self(3);
    /// Currently doing an autorefit loop. Ignore the first autorefit order.
    pub const InAutorefit: Self = Self(4);
}

/// `LinkRefresher::RefreshFlags`: `EnumBitSet<RefreshFlag, uint8_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefreshFlags(pub u8);

impl From<RefreshFlag> for RefreshFlags {
    /// The set containing only `element`.
    fn from(element: RefreshFlag) -> Self {
        Self(1 << element.0)
    }
}

impl RefreshFlags {
    /// `BaseBitSet::Test`: whether `element` is set.
    #[must_use]
    pub fn test(self, element: RefreshFlag) -> bool {
        self.0 & Self::from(element).0 != 0
    }
    /// `BaseBitSet::Set`: this set with `flags` added, an element or a set.
    #[must_use]
    pub fn set(self, flags: impl Into<Self>) -> Self {
        Self(self.0 | flags.into().0)
    }
    /// `BaseBitSet::Reset`: this set with `flags` removed, an element or a set.
    #[must_use]
    pub fn reset(self, flags: impl Into<Self>) -> Self {
        Self(self.0 & !flags.into().0)
    }
}

#[cfg(test)]
mod tests {
    use super::{RefreshFlag, RefreshFlags};

    /// The enumerators are bit positions 0 to 4, as `refresh.h` declares them in order.
    #[test]
    fn flags_set_reset_and_test_the_header_bits() {
        let flags = RefreshFlags::default()
            .set(RefreshFlag::UseNext)
            .set(RefreshFlag::InAutorefit);
        assert_eq!(flags.0, 0b1_0001);
        assert!(flags.test(RefreshFlag::UseNext));
        assert!(!flags.test(RefreshFlag::HasCargo));
        let flags = flags
            .set(RefreshFlag::HasCargo)
            .set(RefreshFlag::WasRefit)
            .set(RefreshFlag::ResetRefit);
        assert_eq!(flags.0, 0b1_1111);
        let pair = RefreshFlags::from(RefreshFlag::ResetRefit).set(RefreshFlag::WasRefit);
        assert_eq!(pair.0, 0b0_1100);
        let flags = flags.reset(pair);
        assert_eq!(flags.0, 0b1_0011);
        assert!(!flags.test(RefreshFlag::WasRefit));
        assert!(!flags.test(RefreshFlag::ResetRefit));
        assert_eq!(flags.reset(RefreshFlag::UseNext).0, 0b1_0010);
    }
}

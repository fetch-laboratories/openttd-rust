/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants from `rail_type.h`.
/// `INVALID_RAILTYPE`.
pub const INVALID_RAILTYPE: u8 = RailType::INVALID_RAILTYPE.0;

/// Native standard railtype identities.
pub const RAILTYPE_RAIL: u8 = RailType::RAILTYPE_RAIL.0;
/// `RAILTYPE_ELECTRIC`.
pub const RAILTYPE_ELECTRIC: u8 = RailType::RAILTYPE_ELECTRIC.0;
/// `RAILTYPE_MONO`.
pub const RAILTYPE_MONO: u8 = RailType::RAILTYPE_MONO.0;
/// `RAILTYPE_MAGLEV`.
pub const RAILTYPE_MAGLEV: u8 = RailType::RAILTYPE_MAGLEV.0;

/// `RAILTYPE_END`.
pub const RAILTYPE_END: u8 = RailType::RAILTYPE_END.0;

/// `RAILTYPE_BEGIN`.
pub const RAILTYPE_BEGIN: u8 = RailType::RAILTYPE_BEGIN.0;

/// Native standard railtype labels.
pub const RAILTYPE_LABEL_RAIL: u32 = RailTypeLabel::RAILTYPE_LABEL_RAIL.0;
/// `RAILTYPE_LABEL_ELECTRIC`.
pub const RAILTYPE_LABEL_ELECTRIC: u32 = RailTypeLabel::RAILTYPE_LABEL_ELECTRIC.0;
/// `RAILTYPE_LABEL_MONO`.
pub const RAILTYPE_LABEL_MONO: u32 = RailTypeLabel::RAILTYPE_LABEL_MONO.0;
/// `RAILTYPE_LABEL_MAGLEV`.
pub const RAILTYPE_LABEL_MAGLEV: u32 = RailTypeLabel::RAILTYPE_LABEL_MAGLEV.0;

/// `INVALID_RAILTYPES`, the original all-types sentinel for follower initialization.
pub const INVALID_RAILTYPES: u64 = RailTypes::INVALID_RAILTYPES.0;

/// Native `RailType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailType(pub u8);

impl RailType {
    /// Used for iterations
    pub const RAILTYPE_BEGIN: Self = Self(0);
    /// Standard non-electric rails
    pub const RAILTYPE_RAIL: Self = Self(0);
    /// Electric rails
    pub const RAILTYPE_ELECTRIC: Self = Self(1);
    /// Monorail
    pub const RAILTYPE_MONO: Self = Self(2);
    /// Maglev
    pub const RAILTYPE_MAGLEV: Self = Self(3);
    /// Used for iterations
    pub const RAILTYPE_END: Self = Self(64);
    /// Flag for invalid railtype
    pub const INVALID_RAILTYPE: Self = Self(255);
}

/// Native `RailTypes`, retaining every underlying `u64` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTypes(pub u64);

impl RailTypes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    pub const fn from_element(element: RailType) -> Self {
        Self(1_u64 << element.0)
    }
}

impl RailTypes {
    /// Native `BaseBitSet::Any`: true iff any of the values are set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }
    /// Native `BaseBitSet::Test`, with an element rather than a set mask. The original's
    /// `1ULL << value` leaves a count of 64 or more, such as `INVALID_RAILTYPE`'s, to the
    /// processor, which masks it to six bits; the wrapping shift does the same.
    #[must_use]
    pub const fn test(self, element: RailType) -> bool {
        self.0 & 1_u64.wrapping_shl(element.0 as u32) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Native `RailTypeLabel`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTypeLabel(pub u32);

impl RailTypes {
    /// All-types sentinel.
    pub const INVALID_RAILTYPES: Self = Self(u64::MAX);
}

crate::base_bitset_type::impl_bitset_ops!(RailTypes);

impl RailTypeLabel {
    /// Standard railway label.
    pub const RAILTYPE_LABEL_RAIL: Self = Self(u32::from_be_bytes(*b"RAIL"));
    /// Electrified railway label.
    pub const RAILTYPE_LABEL_ELECTRIC: Self = Self(u32::from_be_bytes(*b"ELRL"));
    /// Monorail label.
    pub const RAILTYPE_LABEL_MONO: Self = Self(u32::from_be_bytes(*b"MONO"));
    /// Maglev label.
    pub const RAILTYPE_LABEL_MAGLEV: Self = Self(u32::from_be_bytes(*b"MGLV"));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BaseBitSet::Test` with `INVALID_RAILTYPE` (255) tests bit 63, as the original's
    /// shift does on x86-64, where `GetTileRailType` hands it to `HasPowerOnRail` on a tile
    /// with no rail; a checked shift would end the game instead.
    #[test]
    fn test_masks_an_out_of_range_rail_type_as_the_processor_does() {
        let invalid = RailType(INVALID_RAILTYPE);
        assert!(!RailTypes(0).test(invalid));
        assert!(!RailTypes(u64::MAX >> 1).test(invalid));
        assert!(RailTypes(1 << 63).test(invalid));
        assert!(RailTypes(1 << 3).test(RailType(3)));
    }
}

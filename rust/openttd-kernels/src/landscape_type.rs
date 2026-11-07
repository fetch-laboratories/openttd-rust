/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `landscape_type.h` climate identifiers.
/// `LandscapeType::Arctic`.
pub const ARCTIC: u8 = LandscapeType::Arctic.0;
/// `LandscapeType::Tropic`.
pub const TROPIC: u8 = LandscapeType::Tropic.0;

/// Number of original climates.
pub const NUM_LANDSCAPE: u8 = 4;

/// `LandscapeType::Toyland`.
pub const TOYLAND: u8 = LandscapeType::Toyland.0;

/// Native `LandscapeType : uint8_t`; every underlying byte remains valid.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LandscapeType(pub u8);

impl LandscapeType {
    /// Native `LandscapeType::Toyland`.
    pub const TOYLAND: Self = Self::Toyland;
}

#[allow(non_upper_case_globals)]
impl LandscapeType {
    /// Native `Temperate`.
    pub const Temperate: Self = Self(0);
    /// Native `Arctic`.
    pub const Arctic: Self = Self(1);
    /// Native `Tropic`.
    pub const Tropic: Self = Self(2);
    /// Native `Toyland`.
    pub const Toyland: Self = Self(3);
}

/// Native `LandscapeTypes`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LandscapeTypes(pub u8);

impl LandscapeTypes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: LandscapeType) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl LandscapeTypes {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: LandscapeType) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(LandscapeTypes);

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
pub const RAILTYPE_LABEL_RAIL: u32 = u32::from_be_bytes(*b"RAIL");
/// `RAILTYPE_LABEL_ELECTRIC`.
pub const RAILTYPE_LABEL_ELECTRIC: u32 = u32::from_be_bytes(*b"ELRL");
/// `RAILTYPE_LABEL_MONO`.
pub const RAILTYPE_LABEL_MONO: u32 = u32::from_be_bytes(*b"MONO");
/// `RAILTYPE_LABEL_MAGLEV`.
pub const RAILTYPE_LABEL_MAGLEV: u32 = u32::from_be_bytes(*b"MGLV");

/// `INVALID_RAILTYPES`, the original all-types sentinel for follower initialization.
pub const INVALID_RAILTYPES: u64 = u64::MAX;

/// Native `RailType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RailType(pub u8);

#[allow(non_upper_case_globals)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTypes(pub u64);

impl RailTypes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Set containing the named element (the element is a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: RailType) -> Self {
        Self(1_u64 << element.0)
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants of `industry_type.h`.
/// Native `NUM_INDUSTRYTYPES`.
pub const NUM_INDUSTRYTYPES: u8 = IndustryType::NUM_INDUSTRYTYPES.0;
/// Native `IT_INVALID`.
pub const IT_INVALID: u8 = IndustryType::IT_INVALID.0;
/// Native `NEW_INDUSTRYTILEOFFSET`.
pub const NEW_INDUSTRYTILEOFFSET: u16 = IndustryGfx::NEW_INDUSTRYTILEOFFSET.0;

/// Native `NEW_INDUSTRYOFFSET`.
pub const NEW_INDUSTRYOFFSET: u16 = IndustryType::NEW_INDUSTRYOFFSET.0 as u16;
/// Native `NUM_INDUSTRYTILES`.
pub const NUM_INDUSTRYTILES: u16 = IndustryGfx::NUM_INDUSTRYTILES.0;

/// Native `INDUSTRY_NUM_INPUTS`.
pub const INDUSTRY_NUM_INPUTS: usize = 16;
/// Native `INDUSTRY_NUM_OUTPUTS`.
pub const INDUSTRY_NUM_OUTPUTS: usize = 16;
/// Native `INDUSTRY_ORIGINAL_NUM_INPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_INPUTS: usize = 3;
/// Native `INDUSTRY_ORIGINAL_NUM_OUTPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_OUTPUTS: usize = 2;

/// `IndustryBehaviour::AirplaneAttacks`.
pub use crate::industrytype::AIRPLANE_ATTACKS;
/// `IndustryBehaviour::CanSubsidence`.
pub use crate::industrytype::CAN_SUBSIDENCE;
/// `IndustryBehaviour::ChopperAttacks`.
pub use crate::industrytype::CHOPPER_ATTACKS;
/// Native `IndustryID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryID(pub u16);
impl IndustryID {
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(u16::MAX);
}

/// Opaque native `Industry`; valid only while its native pool shell lives.
#[repr(C)]
pub struct Industry {
    _private: [u8; 0],
}

impl IndustryID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

/// Native `IndustryType` (`industry_type.h`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryType(pub u8);
impl IndustryType {
    /// Maximum per `NewGRF`; bit 7 has special meaning in variables/callbacks.
    pub const NUM_INDUSTRYTYPES_PER_GRF: Self = Self(128);
    /// Original number of industry types.
    pub const NEW_INDUSTRYOFFSET: Self = Self(37);
    /// Total types; leaves room for `IT_INVALID`, `IT_AI_UNKNOWN`, `IT_AI_TOWN`, etc.
    pub const NUM_INDUSTRYTYPES: Self = Self(240);
    /// Invalid industry type.
    pub const IT_INVALID: Self = Self(0xff);
}

/// Native `IndustryGfx` (`industry_type.h`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryGfx(pub u16);
impl IndustryGfx {
    /// Limited to 255 so Action3 can later extend this with an extended byte.
    pub const NUM_INDUSTRYTILES_PER_GRF: Self = Self(255);
    /// Marks an industry tile as having no animation.
    pub const INDUSTRYTILE_NOANIM: Self = Self(0xff);
    /// Original number of industry tiles.
    pub const NEW_INDUSTRYTILEOFFSET: Self = Self(175);
    /// Total number of industry tiles, new and old.
    pub const NUM_INDUSTRYTILES: Self = Self(512);
    /// One above the tile count is invalid.
    pub const INVALID_INDUSTRYTILE: Self = Self::NUM_INDUSTRYTILES;
}
/// Final stage of industry construction.
pub const INDUSTRY_COMPLETED: i32 = 3;

/// Native `industry_type.h::IndustryRandomTrigger`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryRandomTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryRandomTrigger {
    /// The tile of the industry has been triggered during the tileloop.
    pub const TileLoop: Self = Self(0);
    /// The industry has been triggered via its tick.
    pub const IndustryTick: Self = Self(1);
    /// Cargo has been delivered.
    pub const CargoReceived: Self = Self(2);
}

/// Native `IndustryRandomTriggers`; elements are `IndustryRandomTrigger` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryRandomTriggers(pub u8);
impl IndustryRandomTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryRandomTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit set containing one element.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "EnumBitSet<IndustryRandomTrigger, uint8_t> stores the element bit in uint8_t"
    )]
    pub const fn from_element(element: IndustryRandomTrigger) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(IndustryRandomTriggers);

/// Native `industry_type.h::IndustryAnimationTrigger`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryAnimationTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryAnimationTrigger {
    /// Trigger whenever the construction stage changes.
    pub const ConstructionStageChanged: Self = Self(0);
    /// Trigger in the periodic tile loop.
    pub const TileLoop: Self = Self(1);
    /// Trigger every tick.
    pub const IndustryTick: Self = Self(2);
    /// Trigger when cargo is received .
    pub const CargoReceived: Self = Self(3);
    /// Trigger when cargo is distributed.
    pub const CargoDistributed: Self = Self(4);
}

/// Native `IndustryAnimationTriggers`; elements are `IndustryAnimationTrigger` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryAnimationTriggers(pub u8);
impl IndustryAnimationTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, element: IndustryAnimationTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
}

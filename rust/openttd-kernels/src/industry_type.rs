/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants of `industry_type.h`.
/// Native `NUM_INDUSTRYTYPES`.
pub const NUM_INDUSTRYTYPES: u8 = 240;
/// Native `INVALID_INDUSTRY`.
pub const INVALID_INDUSTRY: u16 = IndustryID::INVALID.0;
/// Native `IT_INVALID`.
pub const IT_INVALID: u8 = 0xff;
/// Native `NEW_INDUSTRYTILEOFFSET`.
pub const NEW_INDUSTRYTILEOFFSET: u16 = 175;
/// Native `INVALID_INDUSTRYTILE`.
pub const INVALID_INDUSTRYTILE: u16 = 512;

/// Masks from `industrytype.h::IndustryBehaviour`.
pub const BUILT_ON_WATER: u32 = 1 << 2;
/// Native `PROD_MULTI_HANDLING`.
pub const PROD_MULTI_HANDLING: u32 = 1 << 14;

/// Native `NEW_INDUSTRYOFFSET`.
pub const NEW_INDUSTRYOFFSET: u16 = 37;
/// Native `NUM_INDUSTRYTILES`.
pub const NUM_INDUSTRYTILES: u16 = 512;

/// Native `NUM_INDUSTRYTYPES_PER_GRF`.
pub const NUM_INDUSTRYTYPES_PER_GRF: u8 = 128;
/// Native `NUM_INDUSTRYTILES_PER_GRF`.
pub const NUM_INDUSTRYTILES_PER_GRF: u16 = 255;
/// Native `INDUSTRY_NUM_INPUTS`.
pub const INDUSTRY_NUM_INPUTS: usize = 16;
/// Native `INDUSTRY_NUM_OUTPUTS`.
pub const INDUSTRY_NUM_OUTPUTS: usize = 16;
/// Native `INDUSTRY_ORIGINAL_NUM_INPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_INPUTS: usize = 3;
/// Native `INDUSTRY_ORIGINAL_NUM_OUTPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_OUTPUTS: usize = 2;

/// `IndustryBehaviour::AirplaneAttacks`.
pub const AIRPLANE_ATTACKS: u32 = 1 << 11;
/// `IndustryBehaviour::ChopperAttacks`.
pub const CHOPPER_ATTACKS: u32 = 1 << 12;
/// `IndustryBehaviour::CanSubsidence`.
pub const CAN_SUBSIDENCE: u32 = 1 << 13;
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

/// `IndustryRandomTrigger::CargoReceived`.
pub const RANDOM_TRIGGER_CARGO_RECEIVED: u8 = 2;
/// `IndustryAnimationTrigger::CargoReceived`.
pub const ANIMATION_TRIGGER_CARGO_RECEIVED: u8 = 3;

/// `IndustryLifeType::Extractive` mask.
pub const LIFE_EXTRACTIVE: u8 = 1;

/// `IndustryLifeType::Organic` mask.
pub const LIFE_ORGANIC: u8 = 2;
/// `IndustryLifeType::Processing` mask.
pub const LIFE_PROCESSING: u8 = 4;
/// `IndustryBehaviour::CutTrees` mask.
pub const CUT_TREES: u32 = 1 << 1;

impl IndustryID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants of `industry_type.h`.
pub const NUM_INDUSTRYTYPES: u8 = 240;
pub const INVALID_INDUSTRY: u16 = 0xffff;
pub const IT_INVALID: u8 = 0xff;
pub const NEW_INDUSTRYTILEOFFSET: u16 = 175;
pub const INVALID_INDUSTRYTILE: u16 = 512;

/// Masks from `industrytype.h::IndustryBehaviour`.
pub const BUILT_ON_WATER: u32 = 1 << 2;
pub const PROD_MULTI_HANDLING: u32 = 1 << 14;

pub const NEW_INDUSTRYOFFSET: u16 = 37;
pub const NUM_INDUSTRYTILES: u16 = 512;

pub const NUM_INDUSTRYTYPES_PER_GRF: u8 = 128;
pub const NUM_INDUSTRYTILES_PER_GRF: u16 = 255;
pub const INDUSTRY_NUM_INPUTS: usize = 16;
pub const INDUSTRY_NUM_OUTPUTS: usize = 16;
pub const INDUSTRY_ORIGINAL_NUM_INPUTS: usize = 3;
pub const INDUSTRY_ORIGINAL_NUM_OUTPUTS: usize = 2;

/// `IndustryBehaviour::AirplaneAttacks`.
pub const AIRPLANE_ATTACKS: u32 = 1 << 11;
/// `IndustryBehaviour::ChopperAttacks`.
pub const CHOPPER_ATTACKS: u32 = 1 << 12;
/// `IndustryBehaviour::CanSubsidence`.
pub const CAN_SUBSIDENCE: u32 = 1 << 13;

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

/// `INDUSTRY_NUM_INPUTS`.
pub const INDUSTRY_NUM_INPUTS: usize = 16;
/// `INDUSTRY_NUM_OUTPUTS`.
pub const INDUSTRY_NUM_OUTPUTS: usize = 16;
/// `INDUSTRY_ORIGINAL_NUM_INPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_INPUTS: u8 = 3;
/// `INDUSTRY_ORIGINAL_NUM_OUTPUTS`.
pub const INDUSTRY_ORIGINAL_NUM_OUTPUTS: u8 = 2;

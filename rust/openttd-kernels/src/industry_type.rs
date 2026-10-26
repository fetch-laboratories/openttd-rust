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

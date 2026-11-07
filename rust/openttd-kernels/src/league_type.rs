/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `league_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeagueTableID(pub u8);
impl LeagueTableID {
    /// Native `INVALID`.
    pub const INVALID: Self = Self(u8::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeagueTableElementID(pub u16);
impl LeagueTableElementID {
    /// Native `INVALID`.
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct LeagueTableElement {
    _private: [u8; 0],
}
/// Native `LT_NONE`.
pub const LT_NONE: u8 = 0;
/// Native `LT_TILE`.
pub const LT_TILE: u8 = 1;
/// Native `LT_INDUSTRY`.
pub const LT_INDUSTRY: u8 = 2;
/// Native `LT_TOWN`.
pub const LT_TOWN: u8 = 3;
/// Native `LT_COMPANY`.
pub const LT_COMPANY: u8 = 4;
/// Native `LT_STORY_PAGE`.
pub const LT_STORY_PAGE: u8 = 5;

impl LeagueTableID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(255);
}

impl LeagueTableElementID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

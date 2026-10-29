/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `league_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeagueTableID(pub u8);
impl LeagueTableID {
    pub const INVALID: Self = Self(u8::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeagueTableElementID(pub u16);
impl LeagueTableElementID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct LeagueTableElement {
    _private: [u8; 0],
}
pub const LT_NONE: u8 = 0;
pub const LT_TILE: u8 = 1;
pub const LT_INDUSTRY: u8 = 2;
pub const LT_TOWN: u8 = 3;
pub const LT_COMPANY: u8 = 4;
pub const LT_STORY_PAGE: u8 = 5;

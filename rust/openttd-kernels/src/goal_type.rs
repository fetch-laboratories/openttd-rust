/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `goal_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GoalID(pub u16);
impl GoalID {
    /// Native `INVALID`.
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct Goal {
    _private: [u8; 0],
}
/// Native `GT_NONE`.
pub const GT_NONE: u8 = 0;
/// Native `GT_TILE`.
pub const GT_TILE: u8 = 1;
/// Native `GT_INDUSTRY`.
pub const GT_INDUSTRY: u8 = 2;
/// Native `GT_TOWN`.
pub const GT_TOWN: u8 = 3;
/// Native `GT_COMPANY`.
pub const GT_COMPANY: u8 = 4;
/// Native `GT_STORY_PAGE`.
pub const GT_STORY_PAGE: u8 = 5;
/// Native `GOAL_QUESTION_BUTTON_COUNT`.
pub const GOAL_QUESTION_BUTTON_COUNT: u8 = 18;
/// Native `GQT_QUESTION`.
pub const GQT_QUESTION: u8 = 0;
/// Native `GQT_END`.
pub const GQT_END: u8 = 4;

impl GoalID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

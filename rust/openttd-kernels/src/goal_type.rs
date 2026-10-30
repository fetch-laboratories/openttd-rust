/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `goal_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalID(pub u16);
impl GoalID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct Goal {
    _private: [u8; 0],
}
pub const GT_NONE: u8 = 0;
pub const GT_TILE: u8 = 1;
pub const GT_INDUSTRY: u8 = 2;
pub const GT_TOWN: u8 = 3;
pub const GT_COMPANY: u8 = 4;
pub const GT_STORY_PAGE: u8 = 5;
pub const GOAL_QUESTION_BUTTON_COUNT: u8 = 18;
pub const GQT_QUESTION: u8 = 0;
pub const GQT_END: u8 = 4;

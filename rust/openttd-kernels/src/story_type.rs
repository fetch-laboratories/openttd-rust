/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `story_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryPageID(pub u16);
impl StoryPageID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryPageElementID(pub u16);
impl StoryPageElementID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct StoryPage {
    _private: [u8; 0],
}
#[repr(C)]
pub struct StoryPageElement {
    _private: [u8; 0],
}

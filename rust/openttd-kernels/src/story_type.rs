/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `story_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoryPageID(pub u16);
impl StoryPageID {
    /// Native `INVALID`.
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoryPageElementID(pub u16);
impl StoryPageElementID {
    /// Native `INVALID`.
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

impl StoryPageID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

impl StoryPageElementID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
}

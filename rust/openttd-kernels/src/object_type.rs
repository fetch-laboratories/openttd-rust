/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Consumed native header constants, asserted in services.cpp.
pub const OBJECT_STATUE: u16 = 2;

/// Native `ObjectID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectID(pub u32);

impl ObjectID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(16711680);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(4294967295);
}

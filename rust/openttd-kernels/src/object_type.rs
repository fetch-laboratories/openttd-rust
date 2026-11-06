/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Consumed native header constants, asserted in services.cpp.
pub const OBJECT_STATUE: u16 = 2;

/// Object instance index; spec IDs remain the original u16 `ObjectType`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectID(
    /// Full-width native pool index.
    pub u32,
);
impl ObjectID {
    /// Original pool end sentinel.
    pub const END: usize = 0x00FF_0000;
}
impl crate::pool::PoolId for ObjectID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
/// Native shell retained by drawing, script and chunk frames.
#[repr(C)]
pub struct ObjectShell {
    _private: [u8; 0],
}

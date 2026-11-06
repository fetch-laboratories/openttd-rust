/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `depot_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepotID(pub u16);
/// Opaque native `Depot`; valid only while its native pool shell lives.
#[repr(C)]
pub struct Depot {
    _private: [u8; 0],
}
pub const MAX_LENGTH_DEPOT_NAME_CHARS: usize = 32;

impl DepotID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(64000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(65535);
}

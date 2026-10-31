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

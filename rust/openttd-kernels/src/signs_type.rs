/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `signs_type.h`.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignID(pub u16);
impl SignID {
    pub const INVALID: Self = Self(u16::MAX);
}
/// Opaque native `Sign`; valid only while its native pool shell lives.
#[repr(C)]
pub struct Sign {
    _private: [u8; 0],
}
pub const MAX_LENGTH_SIGN_NAME_CHARS: usize = 32;

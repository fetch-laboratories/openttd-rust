/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `newgrf_internal.h` property parser results.
/// `CIR_SUCCESS`.
pub const CIR_SUCCESS: i32 = ChangeInfoResult::CIR_SUCCESS.0 as i32;
/// `CIR_UNKNOWN`.
pub const CIR_UNKNOWN: i32 = ChangeInfoResult::CIR_UNKNOWN.0 as i32;
/// `CIR_INVALID_ID`.
pub const CIR_INVALID_ID: i32 = ChangeInfoResult::CIR_INVALID_ID.0 as i32;

/// `MAX_SPRITEGROUP`.
pub const MAX_SPRITEGROUP: u32 = u8::MAX as u32;

/// Native `ChangeInfoResult : uint8_t`, retaining the full raw domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChangeInfoResult(pub u8);

impl ChangeInfoResult {
    /// Variable was parsed and read.
    pub const CIR_SUCCESS: Self = Self(0);
    /// GRF was disabled due to error.
    pub const CIR_DISABLED: Self = Self(1);
    /// Variable was parsed but unread.
    pub const CIR_UNHANDLED: Self = Self(2);
    /// Variable is unknown.
    pub const CIR_UNKNOWN: Self = Self(3);
    /// Attempt to modify an invalid ID.
    pub const CIR_INVALID_ID: Self = Self(4);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_config.h`: functions to find and configure `NewGRFs`.
use crate::base_bitset_type;

/// `GRFConfigFlag`: GRF config bit flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GRFConfigFlag(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "the enumerator names of the original GRFConfigFlag (newgrf_config.h)"
)]
impl GRFConfigFlag {
    /// GRF file is an openttd-internal system grf
    pub const System: Self = Self(0);
    /// GRF file is unsafe for static usage
    pub const Unsafe: Self = Self(1);
    /// GRF file is used statically (can be used in any MP game)
    pub const Static: Self = Self(2);
    /// GRF file is processed up to `GLS_INIT`
    pub const InitOnly: Self = Self(5);
    /// GRF file passed `GLS_RESERVE` stage
    pub const Reserved: Self = Self(6);
    /// GRF is unusable with this version of OpenTTD
    pub const Invalid: Self = Self(7);
}

/// `GRFConfigFlags`, `EnumBitSet<GRFConfigFlag, uint8_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GRFConfigFlags(pub u8);
impl GRFConfigFlags {
    /// `BaseBitSet::Test`: whether the flag is set.
    #[must_use]
    pub const fn test(self, flag: GRFConfigFlag) -> bool {
        base_bitset_type::test(self.0 as u64, flag.0)
    }
}

/// `GRFStatus`: status of GRF.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GRFStatus(pub u8);
impl GRFStatus {
    /// The status of this grf file is unknown
    pub const GCS_UNKNOWN: Self = Self(0);
    /// GRF file is disabled
    pub const GCS_DISABLED: Self = Self(1);
    /// GRF file was not found in the local cache
    pub const GCS_NOT_FOUND: Self = Self(2);
    /// GRF file has been initialised
    pub const GCS_INITIALISED: Self = Self(3);
    /// GRF file has been activated
    pub const GCS_ACTIVATED: Self = Self(4);
}

/// `GRFPalette`: information about the palette of a GRF.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GRFPalette(pub u8);
impl GRFPalette {
    /// Bitmask to get only the use palette use states.
    pub const GRFP_USE_MASK: Self = Self(0x1);
}

// Raw values of the typed constants for `newgrf_loading.rs`, which no pass has finished; they
// go with its pass (#734's residue).

/// `GRFStatus::GCS_UNKNOWN`.
pub const GCS_UNKNOWN: u8 = GRFStatus::GCS_UNKNOWN.0;
/// `GRFStatus::GCS_DISABLED`.
pub const GCS_DISABLED: u8 = GRFStatus::GCS_DISABLED.0;
/// `GRFStatus::GCS_NOT_FOUND`.
pub const GCS_NOT_FOUND: u8 = GRFStatus::GCS_NOT_FOUND.0;
/// `GRFStatus::GCS_INITIALISED`.
pub const GCS_INITIALISED: u8 = GRFStatus::GCS_INITIALISED.0;
/// `GRFStatus::GCS_ACTIVATED`.
pub const GCS_ACTIVATED: u8 = GRFStatus::GCS_ACTIVATED.0;
/// `GRFConfigFlag::System`.
pub const SYSTEM: u8 = GRFConfigFlag::System.0;
/// `GRFConfigFlag::Unsafe`.
pub const UNSAFE: u8 = GRFConfigFlag::Unsafe.0;
/// `GRFConfigFlag::Static`.
pub const STATIC: u8 = GRFConfigFlag::Static.0;
/// `GRFConfigFlag::InitOnly`.
pub const INIT_ONLY: u8 = GRFConfigFlag::InitOnly.0;
/// `GRFConfigFlag::Reserved`.
pub const RESERVED: u8 = GRFConfigFlag::Reserved.0;
/// `GRFConfigFlag::Invalid`.
pub const INVALID: u8 = GRFConfigFlag::Invalid.0;
/// `GRFPalette::GRFP_USE_MASK`.
pub const GRFP_USE_MASK: u8 = GRFPalette::GRFP_USE_MASK.0;

/// `GRFIdentifier`: basic data to distinguish a GRF. Used in the server list window
pub(crate) struct GRFIdentifier {
    /// GRF ID (defined by Action 0x08)
    pub grfid: u32,
    /// MD5 checksum of file to distinguish files with the same GRF ID (eg. newer version of GRF)
    pub md5sum: crate::md5::MD5Hash,
}

/// `GRFBug`: encountered GRF bugs.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GRFBug(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "the enumerator names of the original GRFBug (newgrf_config.h)"
)]
impl GRFBug {
    /// Length of rail vehicle changes when not inside a depot
    pub const VehLength: Self = Self(0);
    /// Articulated vehicles carry different cargoes resp. are differently refittable than
    /// specified in purchase list
    pub const VehRefit: Self = Self(1);
    /// Powered wagon changed poweredness state when not inside a depot
    pub const VehPoweredWagon: Self = Self(2);
    /// A callback returned an unknown/invalid result
    pub const UnknownCbResult: Self = Self(3);
    /// Capacity of vehicle changes when not refitting or arranging
    pub const VehCapacity: Self = Self(4);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared identities from `newgrf_commons.h`.
use crate::cargo_type::NUM_CARGO;
/// `CargoGRFFileProps::SG_DEFAULT`.
pub const SG_DEFAULT: u8 = NUM_CARGO;
/// `CargoGRFFileProps::SG_PURCHASE`.
pub const SG_PURCHASE: u8 = NUM_CARGO + 1;
/// `CargoGRFFileProps::SG_DEFAULT_NA`.
pub const SG_DEFAULT_NA: u8 = NUM_CARGO + 2;

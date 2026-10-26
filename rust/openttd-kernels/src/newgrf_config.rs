/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Loading constants from `newgrf_config.h`.
#![allow(missing_docs)]
pub const GCS_UNKNOWN: u8 = 0;
pub const GCS_DISABLED: u8 = 1;
pub const GCS_NOT_FOUND: u8 = 2;
pub const GCS_INITIALISED: u8 = 3;
pub const GCS_ACTIVATED: u8 = 4;
pub const SYSTEM: u8 = 0;
pub const UNSAFE: u8 = 1;
pub const STATIC: u8 = 2;
pub const INIT_ONLY: u8 = 5;
pub const RESERVED: u8 = 6;
pub const INVALID: u8 = 7;

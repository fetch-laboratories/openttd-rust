/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants from `rail_type.h`.
/// `INVALID_RAILTYPE`.
pub const INVALID_RAILTYPE: u8 = 255;

/// Native standard railtype identities.
pub const RAILTYPE_RAIL: u8 = 0;
/// `RAILTYPE_ELECTRIC`.
pub const RAILTYPE_ELECTRIC: u8 = 1;
/// `RAILTYPE_MONO`.
pub const RAILTYPE_MONO: u8 = 2;
/// `RAILTYPE_MAGLEV`.
pub const RAILTYPE_MAGLEV: u8 = 3;

/// `RAILTYPE_END`.
pub const RAILTYPE_END: u8 = 64;

/// `RAILTYPE_BEGIN`.
pub const RAILTYPE_BEGIN: u8 = 0;

/// Native standard railtype labels.
pub const RAILTYPE_LABEL_RAIL: u32 = u32::from_be_bytes(*b"RAIL");
/// `RAILTYPE_LABEL_ELECTRIC`.
pub const RAILTYPE_LABEL_ELECTRIC: u32 = u32::from_be_bytes(*b"ELRL");
/// `RAILTYPE_LABEL_MONO`.
pub const RAILTYPE_LABEL_MONO: u32 = u32::from_be_bytes(*b"MONO");
/// `RAILTYPE_LABEL_MAGLEV`.
pub const RAILTYPE_LABEL_MAGLEV: u32 = u32::from_be_bytes(*b"MGLV");

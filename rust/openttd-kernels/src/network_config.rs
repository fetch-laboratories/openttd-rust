/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Values from `network/core/config.h`.
/// `NETWORK_MAX_GRF_COUNT`.
pub const NETWORK_MAX_GRF_COUNT: u32 = 255;

/// Revision string capacity, including its NUL.
pub const NETWORK_REVISION_LENGTH: usize = 33;
/// Client name capacity, including its NUL.
pub const NETWORK_CLIENT_NAME_LENGTH: usize = 25;

/// Chat string capacity, including its NUL.
pub const NETWORK_CHAT_LENGTH: usize = 900;

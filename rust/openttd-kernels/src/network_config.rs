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
/// The maximum length of a content's filename, in bytes including '\0'.
pub const NETWORK_CONTENT_FILENAME_LENGTH: usize = 48;
/// The maximum length of a content's name, in bytes including '\0'.
pub const NETWORK_CONTENT_NAME_LENGTH: usize = 32;
/// The maximum length of a content's version, in bytes including '\0'.
pub const NETWORK_CONTENT_VERSION_LENGTH: usize = 16;
/// The maximum length of a content's url, in bytes including '\0'.
pub const NETWORK_CONTENT_URL_LENGTH: usize = 96;
/// The maximum length of a content's description, in bytes including '\0'.
pub const NETWORK_CONTENT_DESC_LENGTH: usize = 512;
/// The maximum length of a content's tag, in bytes including '\0'.
pub const NETWORK_CONTENT_TAG_LENGTH: usize = 32;

/// Number of bytes in a single packet for backward compatibility (`COMPAT_MTU`).
pub const COMPAT_MTU: usize = 1460;
/* Technically a TCP packet could be 64 KiB, but the high bit is reserved for
 * significantly larger packets in the future, using a strategy like UTF-8.
 *
 * Packets up to 32 KiB have the high bit clear:
 * 00000000 00000000 0bbbbbbb aaaaaaaa -> aaaaaaaa 0bbbbbbb
 * Send_uint16(GB(size, 0, 15))
 *
 * Up to 1 GiB, the first uint16_t has a high bit set to request the next uint16_t:
 * 00dddddd cccccccc bbbbbbbb aaaaaaaa -> cccccccc 10dddddd aaaaaaaa bbbbbbbb
 * Send_uint16(GB(size, 16, 14) | 0b10 << 14)
 * Send_uint16(GB(size, 0, 16))
 */
/// Number of bytes in a single TCP packet (`TCP_MTU`).
pub const TCP_MTU: usize = 32767;

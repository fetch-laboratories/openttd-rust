/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared digest width from `3rdparty/md5/md5.h`.
/// `MD5_HASH_BYTES`.
pub(crate) const MD5_HASH_BYTES: usize = 16;

/// `MD5Hash`: the bytes of an MD5 checksum.
pub(crate) type MD5Hash = [u8; MD5_HASH_BYTES];

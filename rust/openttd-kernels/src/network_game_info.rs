/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Packet serialization helpers of `network/core/network_game_info.h`.

use crate::packet::State as Packet;

/// `SerializeGRFIdentifier`: serializes the `GRFIdentifier` (GRF ID and MD5 checksum) to the
/// packet. `p` is the packet to write the data to; `grfid` and `md5sum` are the `GRFIdentifier` to
/// serialize.
pub(crate) fn serialize_grf_identifier(
    p: &mut Packet,
    grfid: u32,
    md5sum: &[u8],
    check: extern "C" fn(bool),
) {
    p.send_uint32(grfid, check);
    p.send_bytes(md5sum);
}

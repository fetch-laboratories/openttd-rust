/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Packet serialization helpers of `network/core/network_game_info.h`.

use crate::newgrf_config::GRFIdentifier;
use crate::packet::State as Packet;

/// `SerializeGRFIdentifier`: serializes the `GRFIdentifier` (GRF ID and MD5 checksum) to the
/// packet. `p` is the packet to write the data to and `grf` the `GRFIdentifier` to serialize.
pub(crate) fn serialize_grf_identifier(
    p: &mut Packet,
    grf: &GRFIdentifier,
    check: extern "C" fn(bool),
) {
    p.send_uint32(grf.grfid, check);
    p.send_bytes(&grf.md5sum);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Packet serialization helpers of `network/core/network_game_info.h`.
#![allow(unsafe_code)]

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

/// `DeserializeGRFIdentifier`: deserializes the `GRFIdentifier` (GRF ID and MD5 checksum) from
/// the packet `p`. The returned identifier starts zeroed, as the original's local one is.
///
/// # Safety
/// `p` is a live packet whose services are `services`.
pub(crate) unsafe fn deserialize_grf_identifier(
    p: *mut Packet,
    services: &crate::packet::Services,
) -> GRFIdentifier {
    let mut grf = GRFIdentifier {
        grfid: 0,
        md5sum: [0; crate::md5::MD5_HASH_BYTES],
    };
    grf.grfid = unsafe { crate::packet::recv_uint32(p, services) };
    unsafe { (*p).recv_bytes(&mut grf.md5sum) };
    grf
}

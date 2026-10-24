/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants shared from `newgrf_airport.h`.
/// `TTDPatch` large airport classification.
pub const ATP_TTDP_LARGE: u8 = 1;

/// Native airport classification, returned without widening its enum ABI.
#[repr(transparent)]
pub struct TtdpAirportType(pub u8);

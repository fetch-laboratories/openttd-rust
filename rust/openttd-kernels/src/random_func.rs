/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `core/random_func.hpp` shared arithmetic.
/// `ScaleToLimit`, without changing the caller's RNG observation point.
pub fn scale_to_limit(value: u32, limit: u32) -> u32 {
    ((u64::from(value) * u64::from(limit)) >> 32) as u32
}
/// `Chance16I`: uint32 multiplication and addition after the uint16 truncation.
#[must_use]
pub fn chance16_i(a: u32, b: u32, r: u32) -> bool {
    (((r & 65535).wrapping_mul(b).wrapping_add(b / 2)) >> 16) < a
}

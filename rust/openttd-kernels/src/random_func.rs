/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `core/random_func.hpp` shared arithmetic.
/// `ScaleToLimit`: scale a `uint32_t` number to be within the range `[0, limit)`, without
/// changing the caller's RNG observation point.
pub fn scale_to_limit(value: u32, limit: u32) -> u32 {
    ((u64::from(value) * u64::from(limit)) >> 32) as u32
}
/// `Chance16I`: checks if a given randomize-number is below a given probability. This
/// function is used to check if the given probability by the fraction of (a/b) is greater
/// than low 16 bits of the given randomize-number `r`:
/// uint32 multiplication and addition after the uint16 truncation.
#[must_use]
pub fn chance16_i(a: u32, b: u32, r: u32) -> bool {
    (((r & 65535).wrapping_mul(b).wrapping_add(b / 2)) >> 16) < a
}

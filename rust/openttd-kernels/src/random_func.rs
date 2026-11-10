/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `core/random_func.hpp` shared arithmetic.
/// `ScaleToLimit`: scale a `uint32_t` number to be within the range `[0, limit)`, without
/// changing the caller's RNG observation point.
pub fn scale_to_limit(value: u32, limit: u32) -> u32 {
    ((u64::from(value) * u64::from(limit)) >> 32) as u32
}
/// `Chance16I`: checks if a given randomize-number is below a given probability.
///
/// This function is used to check if the given probability by the fraction of (a/b) is
/// greater than low 16 bits of the given randomize-number `r`.
///
/// Do not use this function twice on the same random 16 bits as it will yield the same
/// result. One can use a random number for two calls to `Chance16I`, where one call sends
/// the low 16 bits and the other the high 16 bits.
///
/// `b` is the denominator of the fraction and must of course not be zero. Returns true if
/// the probability given by `r` is less or equal to (a/b). Computed as uint32
/// multiplication and addition after the uint16 truncation.
#[must_use]
pub fn chance16_i(a: u32, b: u32, r: u32) -> bool {
    (((r & 65535).wrapping_mul(b).wrapping_add(b / 2)) >> 16) < a
}

/// `Chance16`: flips a coin with a given probability. This function returns true with
/// (a/b) probability, drawing one value from the shared random stream.
#[must_use]
pub fn chance16(shared: &crate::services::Services, a: u32, b: u32) -> bool {
    chance16_i(a, b, shared.random())
}

/// `Randomizer`: a local pseudo random number generator, independent of the shared
/// random streams.
#[derive(Clone, Copy, Default)]
pub struct Randomizer {
    /// The state of the randomizer.
    pub state: [u32; 2],
}
impl Randomizer {
    /// `Randomizer::Next`: generate the next pseudo random number.
    pub fn next(&mut self) -> u32 {
        let s = self.state[0];
        let t = self.state[1];

        self.state[0] = s
            .wrapping_add((t ^ 0x1234_567F).rotate_right(7))
            .wrapping_add(1);
        self.state[1] = s.rotate_right(3).wrapping_sub(1);
        self.state[1]
    }
    /// `Randomizer::SetSeed`: (re)set the state of the random number generator.
    pub fn set_seed(&mut self, seed: u32) {
        self.state[0] = seed;
        self.state[1] = seed;
    }
}

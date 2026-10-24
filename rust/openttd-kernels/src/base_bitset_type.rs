/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `BaseBitSet` operations from `core/base_bitset_type.hpp`.
/// `BaseBitSet::GetNthSetBit`, retaining ascending bit order and empty results.
pub fn get_nth_set_bit(mut bits: u64, mut n: u32) -> Option<u8> {
    while bits != 0 {
        let bit = bits.trailing_zeros() as u8;
        if n == 0 {
            return Some(bit);
        }
        n -= 1;
        bits &= bits - 1;
    }
    None
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `BaseBitSet` operations from `core/base_bitset_type.hpp`.
/// `BaseBitSet::GetNthSetBit`, retaining ascending bit order and empty results.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // A nonzero u64 has a first bit in 0..64.
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

/// Bitwise operators shared by native bit-mask enums and unmasked `EnumBitSet`s.
/// Each wrapper keeps its original storage width, including unnamed high bits.
macro_rules! impl_bitset_ops {
    ($name:ty) => {
        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }
        impl std::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }
        impl std::ops::BitXor for $name {
            type Output = Self;
            fn bitxor(self, other: Self) -> Self {
                Self(self.0 ^ other.0)
            }
        }
        impl std::ops::Not for $name {
            type Output = Self;
            fn not(self) -> Self {
                Self(!self.0)
            }
        }
        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0;
            }
        }
        impl std::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, other: Self) {
                self.0 &= other.0;
            }
        }
        impl std::ops::BitXorAssign for $name {
            fn bitxor_assign(&mut self, other: Self) {
                self.0 ^= other.0;
            }
        }
    };
}
pub(crate) use impl_bitset_ops;

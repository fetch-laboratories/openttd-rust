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

/// `BaseBitSet::Test`: `(data & (1ULL << value)) != 0`, shifting at the original 64-bit
/// width so that an element beyond a narrower storage tests false. A count of 64 or more,
/// such as an invalid type's, is left to the processor, which masks it to six bits; the
/// wrapping shift does the same.
#[must_use]
pub const fn test(data: u64, value: u8) -> bool {
    data & 1_u64.wrapping_shl(value as u32) != 0
}

/// `BaseBitSet::Set(Tvalue_type value)`: `data | (1ULL << value)`, shifting at the original
/// 64-bit width as [`test`] does. The caller stores the result to its set's `Tstorage`, which
/// drops an element beyond that width as the original's assignment does.
#[must_use]
pub const fn set(data: u64, value: u8) -> u64 {
    data | 1_u64.wrapping_shl(value as u32)
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

#[cfg(test)]
mod tests {
    /// Pinned `core/base_bitset_type.hpp:141-144` shifts `1ULL`: a byte set with every
    /// storage bit set tests its eight elements true and elements 8 to 63 false.
    #[test]
    fn byte_sets_test_at_the_original_width() {
        use crate::road::{RoadTypeFlag, RoadTypeFlags};
        use crate::tile_cmd::{VehicleEnterTileState, VehicleEnterTileStates};
        for element in 0..64_u8 {
            let expected = element < 8;
            assert_eq!(super::test(0xff, element), expected, "element {element}");
            assert_eq!(
                RoadTypeFlags(0xff).test(RoadTypeFlag(element)),
                expected,
                "RoadTypeFlags element {element}"
            );
            assert_eq!(
                VehicleEnterTileStates(0xff).test(VehicleEnterTileState(element)),
                expected,
                "VehicleEnterTileStates element {element}"
            );
        }
    }

    /// Pinned `core/base_bitset_type.hpp:51-55` shifts `1ULL` and stores to `Tstorage`: a
    /// one-element `u8` set (`RoadStopCallbackMasks`) and `u16` set (`VehicleCallbackMasks`)
    /// hold the element's bit below their width and are empty from it up to 63.
    #[test]
    fn single_element_sets_store_at_the_original_width() {
        use crate::newgrf_callbacks::{
            RoadStopCallbackMask, RoadStopCallbackMasks, VehicleCallbackMask, VehicleCallbackMasks,
        };
        for element in 0..64_u8 {
            let byte = RoadStopCallbackMasks::from_element(RoadStopCallbackMask(element));
            let expected_byte = if element < 8 { 1_u8 << element } else { 0 };
            assert_eq!(byte.0, expected_byte, "u8 set element {element}");
            let word = VehicleCallbackMasks::from_element(VehicleCallbackMask(element));
            let expected_word = if element < 16 { 1_u16 << element } else { 0 };
            assert_eq!(word.0, expected_word, "u16 set element {element}");
            assert_eq!(
                super::set(0, element),
                1_u64 << element,
                "u64 element {element}"
            );
        }
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared string-validation flags from `string_type.h`.
/// Native `StringValidationSetting`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringValidationSetting(pub u8);

#[allow(non_upper_case_globals)]
impl StringValidationSetting {
    /// Replace the unknown/bad bits with question marks.
    pub const ReplaceWithQuestionMark: Self = Self(0);
    /// Allow newlines; replaces '\r\n' with '\n' during processing.
    pub const AllowNewline: Self = Self(1);
    /// Allow the special control codes.
    pub const AllowControlCode: Self = Self(2);
    /// Native `ReplaceTabCrNlWithSpace`.
    pub const ReplaceTabCrNlWithSpace: Self = Self(3);
}

/// Native `StringValidationSettings`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringValidationSettings(pub u8);

impl StringValidationSettings {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: StringValidationSetting) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl StringValidationSettings {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: StringValidationSetting) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(StringValidationSettings);

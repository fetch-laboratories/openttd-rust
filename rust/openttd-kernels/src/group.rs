/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared definitions from `group.h`.

/// Native `GroupFlag`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupFlag(pub u8);

impl GroupFlag {
    /// If set, the global autoreplace has no effect on the group
    #[allow(non_upper_case_globals)]
    pub const ReplaceProtection: Self = Self(0);
    /// If set, autoreplace will perform wagon removal on vehicles in this group.
    #[allow(non_upper_case_globals)]
    pub const ReplaceWagonRemoval: Self = Self(1);
}

/// Native `GroupFlags`, retaining every underlying `u8` mask.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupFlags(pub u8);

impl GroupFlags {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: GroupFlag) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native `BaseBitSet::Set` of one element.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "`EnumBitSet<GroupFlag, uint8_t>` stores the element's bit in its `uint8_t`."
    )]
    pub fn set(&mut self, element: GroupFlag) {
        self.0 |= (1_u64 << element.0) as u8;
    }
    /// Native `BaseBitSet::Reset` of one element.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "`EnumBitSet<GroupFlag, uint8_t>` stores the element's bit in its `uint8_t`."
    )]
    pub fn reset(&mut self, element: GroupFlag) {
        self.0 &= !((1_u64 << element.0) as u8);
    }
}

crate::base_bitset_type::impl_bitset_ops!(GroupFlags);

/// `IsDefaultGroupID`: checks if a `GroupID` stands for default vehicles of a company.
pub(crate) fn is_default_group_id(index: crate::group_type::GroupID) -> bool {
    index == crate::group_type::GroupID::DEFAULT_GROUP
}

/// `IsAllGroupID`: checks if a `GroupID` stands for all vehicles of a company.
pub(crate) fn is_all_group_id(id_g: crate::group_type::GroupID) -> bool {
    id_g == crate::group_type::GroupID::ALL_GROUP
}

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
}

crate::base_bitset_type::impl_bitset_ops!(GroupFlags);

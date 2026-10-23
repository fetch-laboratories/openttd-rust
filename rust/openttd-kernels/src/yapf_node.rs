/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Used `yapf_node.hpp` node operations. Intrusive hash links belong to the
//! native hash table, and choice flags are unobserved by no-cache river search.
#[derive(Clone, Copy)]
pub(crate) struct Node<K> {
    pub(crate) key: K,
    pub(crate) parent: Option<usize>,
    pub(crate) cost: i32,
    pub(crate) estimate: i32,
}
impl<K> Node<K> {
    // CYapfNodeT::Set for the fields this owner observes.
    pub(crate) fn set(parent: Option<usize>, key: K) -> Self {
        Self {
            key,
            parent,
            cost: 0,
            estimate: 0,
        }
    }
    pub(crate) fn less(self, other: Self) -> bool {
        self.estimate < other.estimate
    }
}
impl Node<u32> {
    // River keys elide their constant INVALID_TRACKDIR.
    pub(crate) fn get_tile(self) -> u32 {
        self.key
    }
}

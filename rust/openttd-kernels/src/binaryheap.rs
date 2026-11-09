/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `misc/binaryheap.hpp` over owner-local node indices. Equal keys never swap.

/// `CBinaryHeapT`: a carrier which keeps its items automatically holds the smallest item at
/// the first position. The order of items is maintained by using a binary tree. The
/// implementation is used for priority queues.
///
/// This binary heap allocates just the space for item pointers (here node indices). The
/// items are allocated elsewhere. Internally the first item is never used, because that
/// simplifies the implementation. For further information about the Binary Heap
/// algorithm, see <http://www.policyalmanac.org/games/binaryHeaps.htm>.
///
/// `Default` is unallocated storage that only `reuse` makes usable.
#[derive(Default)]
pub(crate) struct BinaryHeap {
    data: Vec<usize>,
}
impl BinaryHeap {
    /// Create a binary heap with the initial reserved `capacity`.
    pub(crate) fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.push(usize::MAX);
        Self { data }
    }
    /// Empty heap over `storage`'s allocation.
    pub(crate) fn reuse(storage: Self) -> Self {
        let mut data = storage.data;
        data.clear();
        data.push(usize::MAX);
        Self { data }
    }
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.data.capacity()
    }
    /// `Begin`: get the smallest item in the binary tree.
    pub(crate) fn first(&self) -> Option<usize> {
        self.data.get(1).copied()
    }
    /// `HeapifyUp`: get position for fixing a gap (upwards). The gap is moved upwards in the
    /// binary tree until it is in order again. Returns the (gap)position where the item
    /// fits.
    fn up(&mut self, mut gap: usize, item: usize, less: &impl Fn(usize, usize) -> bool) -> usize {
        while gap > 1 {
            let parent = gap / 2;
            if !less(item, self.data[parent]) {
                break;
            }
            self.data[gap] = self.data[parent];
            gap = parent;
        }
        gap
    }
    /// `HeapifyDown`: get position for fixing a gap (downwards). The gap is moved downwards
    /// in the binary tree until it is in order again. Returns the (gap)position where the
    /// item fits.
    fn down(&mut self, mut gap: usize, item: usize, less: &impl Fn(usize, usize) -> bool) -> usize {
        let mut child = gap * 2;
        while child < self.data.len() {
            if child + 1 < self.data.len() && less(self.data[child + 1], self.data[child]) {
                child += 1;
            }
            if !less(self.data[child], item) {
                break;
            }
            self.data[gap] = self.data[child];
            gap = child;
            child = gap * 2;
        }
        gap
    }
    /// `Include`: insert new item into the priority queue, maintaining heap order.
    pub(crate) fn include(&mut self, item: usize, less: impl Fn(usize, usize) -> bool) {
        self.data.push(0);
        let gap = self.up(self.data.len() - 1, item, &less);
        self.data[gap] = item;
    }
    /// `Remove(FindIndex(item))`: remove the item from the priority queue.
    pub(crate) fn remove(&mut self, item: usize, less: impl Fn(usize, usize) -> bool) {
        // Native FindIndex matches node identity, not equal tile/estimate.
        let index = self
            .data
            .iter()
            .position(|&candidate| candidate == item)
            .unwrap();
        let last = self.data.pop().unwrap();
        if index < self.data.len() {
            let gap = self.up(index, last, &less);
            let gap = self.down(gap, last, &less);
            self.data[gap] = last;
        }
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `misc/binaryheap.hpp` over owner-local node indices. Equal keys never swap.

/// `CBinaryHeapT`: a carrier which keeps its items automatically holds the smallest item at
/// the first position. The order of items is maintained by using a binary tree. The
/// implementation is used for priority queues.
///
/// There are two major differences compared to `std::priority_queue`. First the
/// `std::priority_queue` does not support indexing/removing elements in the middle of the
/// heap/queue and second it has the biggest item first.
///
/// Item of the binary heap should support the 'lower-than' operator '<' (here `less`). It is
/// used for comparing items before moving them to their position.
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
            // compare [gap] with its parent
            let parent = gap / 2;
            if !less(item, self.data[parent]) {
                // we don't need to continue upstairs
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
        // The first child of the gap is at [parent * 2]
        let mut child = gap * 2;
        // while children are valid
        while child < self.data.len() {
            // choose the smaller child
            if child + 1 < self.data.len() && less(self.data[child + 1], self.data[child]) {
                child += 1;
            }
            // is it smaller than our parent?
            if !less(self.data[child], item) {
                // the smaller child is still bigger or same as parent => we are done
                break;
            }
            // if smaller child is smaller than parent, it will become new parent
            self.data[gap] = self.data[child];
            gap = child;
            // where do we have our new children?
            child = gap * 2;
        }
        gap
    }
    /// `Include`: insert new item into the priority queue, maintaining heap order.
    pub(crate) fn include(&mut self, item: usize, less: impl Fn(usize, usize) -> bool) {
        // Make place for new item. A gap is now at the end of the tree.
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
            // at position index we have a gap now
            // Fix binary tree up and downwards
            let gap = self.up(index, last, &less);
            let gap = self.down(gap, last, &less);
            // move last item to the proper place
            self.data[gap] = last;
        }
    }
}

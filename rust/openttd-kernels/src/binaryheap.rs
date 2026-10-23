/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `misc/binaryheap.hpp` over owner-local node indices. Equal keys never swap.

pub(crate) struct BinaryHeap {
    data: Vec<usize>,
}
impl BinaryHeap {
    pub(crate) fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.push(usize::MAX);
        Self { data }
    }
    pub(crate) fn first(&self) -> Option<usize> {
        self.data.get(1).copied()
    }
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
    pub(crate) fn include(&mut self, item: usize, less: impl Fn(usize, usize) -> bool) {
        self.data.push(0);
        let gap = self.up(self.data.len() - 1, item, &less);
        self.data[gap] = item;
    }
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

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/kdtree.hpp`: two-dimensional uint16 coordinates and signed int distances.
//! Coordinates resolve canonical records; queries allocate nothing. Coordinate readers
//! must not mutate the tree or its records. Rectangle visitors follow node/left/right order.
#![allow(unsafe_code)]

const NONE: usize = usize::MAX;
const MIN_REBALANCE: usize = 8;
#[derive(Clone, Copy)]
struct Node<T> {
    element: T,
    children: [usize; 2],
}
impl<T> Node<T> {
    fn new(element: T) -> Self {
        Self {
            element,
            children: [NONE; 2],
        }
    }
}
pub(crate) struct Kdtree<T> {
    nodes: Vec<Node<T>>,
    free: Vec<usize>,
    root: usize,
    unbalanced: usize,
}
impl<T: Copy + Ord> Kdtree<T> {
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: NONE,
            unbalanced: 0,
        }
    }
    pub(crate) fn count(&self) -> usize {
        self.nodes.len() - self.free.len()
    }
    fn add(&mut self, element: T) -> usize {
        let node = Node::new(element);
        if let Some(index) = self.free.pop() {
            self.nodes[index] = node;
            index
        } else {
            self.nodes.push(node);
            self.nodes.len() - 1
        }
    }
    fn subtree(
        &mut self,
        elements: &mut [T],
        level: usize,
        split: &impl Fn(&mut [T], usize) -> usize,
    ) -> usize {
        if elements.is_empty() {
            return NONE;
        }
        if elements.len() == 1 {
            return self.add(elements[0]);
        }
        let dim = level % 2;
        let first = split(elements, dim);
        let index = self.add(elements[first]);
        self.nodes[index].children[0] = self.subtree(&mut elements[..first], level + 1, split);
        self.nodes[index].children[1] = self.subtree(&mut elements[first + 1..], level + 1, split);
        index
    }
    pub(crate) fn clear(&mut self) {
        self.nodes.clear();
        self.free.clear();
        self.unbalanced = 0;
    }
    pub(crate) fn build(&mut self, elements: &mut [T], split: &impl Fn(&mut [T], usize) -> usize) {
        self.clear();
        if elements.is_empty() {
            return;
        }
        self.nodes.reserve(elements.len());
        self.root = self.subtree(elements, 0, split);
    }
    fn free_subtree(&mut self, node: usize) -> Vec<T> {
        let mut elements = Vec::new();
        let mut cursor = self.free.len();
        for child in self.nodes[node].children {
            if child != NONE {
                self.free.push(child);
            }
        }
        self.nodes[node].children = [NONE; 2];
        while cursor < self.free.len() {
            let index = self.free[cursor];
            let node = self.nodes[index];
            elements.push(node.element);
            for child in node.children {
                if child != NONE {
                    self.free.push(child);
                }
            }
            self.nodes[index].children = [NONE; 2];
            cursor += 1;
        }
        elements
    }
    fn is_unbalanced(&self) -> bool {
        self.count() >= MIN_REBALANCE && self.unbalanced > self.count() / 4
    }
    fn rebuild(
        &mut self,
        include: Option<T>,
        exclude: Option<T>,
        split: &impl Fn(&mut [T], usize) -> usize,
    ) -> bool {
        if self.count() < MIN_REBALANCE {
            return false;
        }
        let root = self.nodes[self.root].element;
        let mut elements = self.free_subtree(self.root);
        elements.push(root);
        if let Some(element) = include {
            elements.push(element);
        }
        if let Some(element) = exclude {
            elements.retain(|e| *e != element);
        }
        self.build(&mut elements, split);
        true
    }
    fn insert_recursive(
        &mut self,
        element: T,
        node: usize,
        level: usize,
        xy: &impl Fn(T, usize) -> u16,
    ) {
        let dim = level % 2;
        let side = usize::from(xy(element, dim) >= xy(self.nodes[node].element, dim));
        let next = self.nodes[node].children[side];
        if next == NONE {
            let index = self.add(element);
            self.nodes[node].children[side] = index;
        } else {
            self.insert_recursive(element, next, level + 1, xy);
        }
    }
    pub(crate) fn insert(
        &mut self,
        element: T,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [T], usize) -> usize,
    ) {
        if self.count() == 0 {
            self.root = self.add(element);
        } else if !self.is_unbalanced() || !self.rebuild(Some(element), None, split) {
            self.insert_recursive(element, self.root, 0, xy);
            self.unbalanced = self.unbalanced.wrapping_add(1);
        }
    }
    fn remove_recursive(
        &mut self,
        element: T,
        node: usize,
        level: usize,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [T], usize) -> usize,
    ) -> usize {
        if self.nodes[node].element == element {
            self.free.push(node);
            if self.nodes[node].children == [NONE; 2] {
                return NONE;
            }
            let mut elements = self.free_subtree(node);
            self.subtree(&mut elements, level, split)
        } else {
            let dim = level % 2;
            let side = usize::from(xy(element, dim) >= xy(self.nodes[node].element, dim));
            let next = self.nodes[node].children[side];
            let branch = self.remove_recursive(element, next, level + 1, xy, split);
            if branch != next {
                self.nodes[node].children[side] = branch;
            }
            node
        }
    }
    pub(crate) fn remove(
        &mut self,
        element: T,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [T], usize) -> usize,
    ) {
        if self.count() == 0 {
            return;
        }
        if !self.is_unbalanced() || !self.rebuild(None, Some(element), split) {
            self.root = self.remove_recursive(element, self.root, 0, xy, split);
            self.unbalanced = self.unbalanced.wrapping_add(1);
        }
    }
    fn nearest_recursive(
        &self,
        target: [u16; 2],
        node: usize,
        level: usize,
        mut limit: i32,
        xy: &impl Fn(T, usize) -> u16,
    ) -> (i32, T) {
        let n = self.nodes[node];
        let dim = level % 2;
        let coordinate = xy(n.element, dim);
        let distance = (i32::from(xy(n.element, 0)) - i32::from(target[0])).abs()
            + (i32::from(xy(n.element, 1)) - i32::from(target[1])).abs();
        let mut best = (distance, n.element);
        let side = usize::from(target[dim] >= coordinate);
        let next = n.children[side];
        if next != NONE {
            // Native near-side recursion resets the limit to int's maximum.
            best = best.min(self.nearest_recursive(target, next, level + 1, i32::MAX, xy));
        }
        limit = limit.min(best.0);
        let opposite = n.children[1 - side];
        if opposite != NONE && limit >= (i32::from(target[dim]) - i32::from(coordinate)).abs() {
            best = best.min(self.nearest_recursive(target, opposite, level + 1, limit, xy));
        }
        best
    }
    pub(crate) fn nearest(&self, target: [u16; 2], xy: &impl Fn(T, usize) -> u16) -> T {
        self.nearest_recursive(target, self.root, 0, i32::MAX, xy).1
    }
    fn contained_recursive(
        &self,
        p1: [u16; 2],
        p2: [u16; 2],
        node: usize,
        level: usize,
        xy: &impl Fn(T, usize) -> u16,
        visit: &mut impl FnMut(T),
    ) {
        let n = self.nodes[node];
        let dim = level % 2;
        let ec = xy(n.element, dim);
        let oc = xy(n.element, 1 - dim);
        if ec >= p1[dim] && ec < p2[dim] && oc >= p1[1 - dim] && oc < p2[1 - dim] {
            visit(n.element);
        }
        if p1[dim] < ec && n.children[0] != NONE {
            self.contained_recursive(p1, p2, n.children[0], level + 1, xy, visit);
        }
        if p2[dim] > ec && n.children[1] != NONE {
            self.contained_recursive(p1, p2, n.children[1], level + 1, xy, visit);
        }
    }
    pub(crate) fn contained(
        &self,
        p1: [u16; 2],
        p2: [u16; 2],
        xy: &impl Fn(T, usize) -> u16,
        mut visit: impl FnMut(T),
    ) {
        assert!(p1[0] < p2[0] && p1[1] < p2[1]);
        if self.count() != 0 {
            self.contained_recursive(p1, p2, self.root, 0, xy, &mut visit);
        }
    }
}

/// Partition live pool IDs using the configured native STL. Every subtree with
/// at least two elements makes one call; the callback reads coordinates only.
/// SAFETY: `T` is u16 or a transparent u16 pool ID, including every bit pattern.
pub(crate) unsafe fn native_split<T: Copy>(
    services: &crate::services::Services,
    elements: &mut [T],
    coordinate: impl Fn(T) -> u16,
) -> usize {
    extern "C" fn read<F: Fn(u16) -> u16>(context: *mut std::ffi::c_void, id: u16) -> u16 {
        unsafe { (&*context.cast::<F>())(id) }
    }
    fn callback<F: Fn(u16) -> u16>(_: &F) -> extern "C" fn(*mut std::ffi::c_void, u16) -> u16 {
        read::<F>
    }
    let coordinate = |id: u16| coordinate(unsafe { std::ptr::from_ref(&id).cast::<T>().read() });
    (services.kdtree_partition)(
        elements.as_mut_ptr().cast(),
        elements.len(),
        std::ptr::from_ref(&coordinate).cast_mut().cast(),
        callback(&coordinate),
    )
}

#[cfg(test)]
mod tests {
    use super::Kdtree;
    #[test]
    fn nearest_after_mutations_and_rebuild() {
        let points: [(u16, u16); 12] = [
            (0, 0),
            (10, 0),
            (0, 10),
            (10, 10),
            (5, 5),
            (5, 0),
            (5, 10),
            (0, 5),
            (10, 5),
            (5, 5),
            (65535, 65535),
            (65535, 0),
        ];
        let xy = |id: u16, dim: usize| {
            let p = points[usize::from(id)];
            if dim == 0 { p.0 } else { p.1 }
        };
        let split = |elements: &mut [u16], dim| {
            let mid = elements.len() / 2;
            elements.select_nth_unstable_by_key(mid, |id| xy(*id, dim));
            let coordinate = xy(elements[mid], dim);
            let (mut first, mut last) = (0, elements.len());
            loop {
                while first < last && xy(elements[first], dim) < coordinate {
                    first += 1;
                }
                if first == last {
                    break;
                }
                loop {
                    last -= 1;
                    if first == last || xy(elements[last], dim) < coordinate {
                        break;
                    }
                }
                if first == last {
                    break;
                }
                elements.swap(first, last);
                first += 1;
            }
            // The boundary element supplies the node's split coordinate.
            assert_eq!(xy(elements[first], dim), coordinate);
            first
        };
        let mut tree = Kdtree::new();
        let mut active = Vec::new();
        let verify = |tree: &Kdtree<u16>, active: &[u16]| {
            assert_eq!(tree.count(), active.len());
            for x in [0, 1, 5, 9, 10, 32768, 65535] {
                for y in [0, 1, 5, 9, 10, 32768, 65535] {
                    let expected = active.iter().copied().min_by_key(|id| {
                        let (px, py) = points[usize::from(*id)];
                        (u32::from(px.abs_diff(x)) + u32::from(py.abs_diff(y)), *id)
                    });
                    if let Some(expected) = expected {
                        assert_eq!(tree.nearest([x, y], &xy), expected);
                    }
                }
            }
        };
        for id in 0..12 {
            tree.insert(id, &xy, &split);
            active.push(id);
            verify(&tree, &active);
        }
        for id in [4, 0, 10, 8, 3, 6, 9, 1, 11, 2, 7, 5] {
            tree.remove(id, &xy, &split);
            active.retain(|v| *v != id);
            verify(&tree, &active);
        }
        active = (0..12).collect();
        tree.build(&mut active, &split);
        verify(&tree, &active);
        let mut contained = Vec::new();
        tree.contained([0, 0], [10, 10], &xy, |id| contained.push(id));
        contained.sort_unstable();
        assert_eq!(contained, [0, 4, 5, 7, 9]);
        tree.build(&mut [], &split);
        assert_eq!(tree.count(), 0);
        tree.insert(9, &xy, &split);
        assert_eq!(tree.nearest([0, 0], &xy), 9);
    }
}

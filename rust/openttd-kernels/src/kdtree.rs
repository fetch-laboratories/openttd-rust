/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `core/kdtree.hpp`: two-dimensional uint16 coordinates and signed int distances.
//! Coordinates resolve canonical records; queries allocate nothing. Coordinate readers
//! must not mutate the tree or its records. Rectangle visitors follow node/left/right order.

/// Index value indicating no-such-node
const INVALID_NODE: usize = usize::MAX;
/// Arbitrary value for "not worth rebalancing"
const MIN_REBALANCE_THRESHOLD: usize = 8;
/// Type of a node in the tree
#[derive(Clone, Copy)]
struct Node<T> {
    element: T,
    /// Index of node to the left and to the right, `INVALID_NODE` if none
    children: [usize; 2],
}
impl<T> Node<T> {
    fn new(element: T) -> Self {
        Self {
            element,
            children: [INVALID_NODE; 2],
        }
    }
}
/// One element and its coordinate on the dimension being split.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SplitKey<T> {
    pub coordinate: u16,
    pub element: T,
}
/// `Kdtree`: K-dimensional tree, specialised for 2-dimensional space. This is not intended
/// as a primary storage of data, but as an index into existing data. Usually the type
/// stored by this tree should be an index into an existing array. This implementation
/// assumes Manhattan distances are used. The element type T must be less-than comparable
/// for `FindNearest` to work.
pub(crate) struct Kdtree<T> {
    nodes: Vec<Node<T>>,
    free: Vec<usize>,
    /// Split buffer, reused by every build so rebuilds allocate nothing once grown.
    keys: Vec<SplitKey<T>>,
    root: usize,
    unbalanced: usize,
}
fn key<T>(element: T) -> SplitKey<T> {
    SplitKey {
        coordinate: 0,
        element,
    }
}
impl<T: Copy + Ord> Kdtree<T> {
    /// Construct a new Kdtree.
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            keys: Vec::new(),
            root: INVALID_NODE,
            unbalanced: 0,
        }
    }
    /// Get number of elements stored in tree
    pub(crate) fn count(&self) -> usize {
        self.nodes.len() - self.free.len()
    }
    /// `AddNode`: create one new node in the tree, return its index in the pool
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
    /// `BuildSubtree`: construct a subtree from elements between begin and end iterators,
    /// return index of root
    fn subtree(
        &mut self,
        keys: &mut [SplitKey<T>],
        level: usize,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) -> usize {
        if keys.is_empty() {
            return INVALID_NODE;
        }
        if keys.len() == 1 {
            return self.add(keys[0].element);
        }
        let dim = level % 2;
        for key in keys.iter_mut() {
            key.coordinate = xy(key.element, dim);
        }
        let first = split(keys);
        let index = self.add(keys[first].element);
        self.nodes[index].children[0] = self.subtree(&mut keys[..first], level + 1, xy, split);
        self.nodes[index].children[1] = self.subtree(&mut keys[first + 1..], level + 1, xy, split);
        index
    }
    /// Clear the tree.
    pub(crate) fn clear(&mut self) {
        self.nodes.clear();
        self.free.clear();
        self.unbalanced = 0;
    }
    /// Clear and rebuild the tree from a new sequence of elements, eliminating any existing
    /// contents.
    pub(crate) fn build(
        &mut self,
        elements: impl IntoIterator<Item = T>,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) {
        let mut keys = std::mem::take(&mut self.keys);
        keys.clear();
        keys.extend(elements.into_iter().map(key));
        self.build_keys(&mut keys, xy, split);
        self.keys = keys;
    }
    fn build_keys(
        &mut self,
        keys: &mut [SplitKey<T>],
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) {
        self.clear();
        if keys.is_empty() {
            return;
        }
        self.nodes.reserve(keys.len());
        self.root = self.subtree(keys, 0, xy, split);
    }
    /// `FreeSubtree`: free all children of the given node
    fn free_subtree(&mut self, node: usize, keys: &mut Vec<SplitKey<T>>) {
        let mut cursor = self.free.len();
        for child in self.nodes[node].children {
            if child != INVALID_NODE {
                self.free.push(child);
            }
        }
        self.nodes[node].children = [INVALID_NODE; 2];
        while cursor < self.free.len() {
            let index = self.free[cursor];
            let node = self.nodes[index];
            keys.push(key(node.element));
            for child in node.children {
                if child != INVALID_NODE {
                    self.free.push(child);
                }
            }
            self.nodes[index].children = [INVALID_NODE; 2];
            cursor += 1;
        }
    }
    /// Check if the entire tree is in need of rebuilding
    fn is_unbalanced(&self) -> bool {
        self.count() >= MIN_REBALANCE_THRESHOLD && self.unbalanced > self.count() / 4
    }
    /// Rebuild the tree with all existing elements, optionally adding or removing one more
    fn rebuild(
        &mut self,
        include: Option<T>,
        exclude: Option<T>,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) -> bool {
        if self.count() < MIN_REBALANCE_THRESHOLD {
            return false;
        }
        let root = self.nodes[self.root].element;
        let mut keys = std::mem::take(&mut self.keys);
        keys.clear();
        self.free_subtree(self.root, &mut keys);
        keys.push(key(root));
        if let Some(element) = include {
            keys.push(key(element));
        }
        if let Some(element) = exclude {
            keys.retain(|k| k.element != element);
        }
        self.build_keys(&mut keys, xy, split);
        self.keys = keys;
        true
    }
    /// Insert one element in the tree somewhere below `node_idx`
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
        if next == INVALID_NODE {
            let index = self.add(element);
            self.nodes[node].children[side] = index;
        } else {
            self.insert_recursive(element, next, level + 1, xy);
        }
    }
    /// Insert a single element in the tree. Repeatedly inserting single elements may cause
    /// the tree to become unbalanced. Undefined behaviour if the element already exists in
    /// the tree.
    pub(crate) fn insert(
        &mut self,
        element: T,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) {
        if self.count() == 0 {
            self.root = self.add(element);
        } else if !self.is_unbalanced() || !self.rebuild(Some(element), None, xy, split) {
            self.insert_recursive(element, self.root, 0, xy);
            self.unbalanced = self.unbalanced.wrapping_add(1);
        }
    }
    /// Find and remove one element from the tree.
    fn remove_recursive(
        &mut self,
        element: T,
        node: usize,
        level: usize,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) -> usize {
        if self.nodes[node].element == element {
            self.free.push(node);
            if self.nodes[node].children == [INVALID_NODE; 2] {
                return INVALID_NODE;
            }
            let mut keys = std::mem::take(&mut self.keys);
            keys.clear();
            self.free_subtree(node, &mut keys);
            let branch = self.subtree(&mut keys, level, xy, split);
            self.keys = keys;
            branch
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
    /// Remove a single element from the tree, if it exists. Since elements are stored in
    /// interior nodes as well as leaf nodes, removing one may require a larger subtree to be
    /// re-built. Because of this, worst case run time is as bad as a full tree rebuild.
    pub(crate) fn remove(
        &mut self,
        element: T,
        xy: &impl Fn(T, usize) -> u16,
        split: &impl Fn(&mut [SplitKey<T>]) -> usize,
    ) {
        if self.count() == 0 {
            return;
        }
        if !self.is_unbalanced() || !self.rebuild(None, Some(element), xy, split) {
            self.root = self.remove_recursive(element, self.root, 0, xy, split);
            self.unbalanced = self.unbalanced.wrapping_add(1);
        }
    }
    /// Search a sub-tree for the element nearest to a given point
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
        if next != INVALID_NODE {
            // Native near-side recursion resets the limit to int's maximum.
            best = best.min(self.nearest_recursive(target, next, level + 1, i32::MAX, xy));
        }
        limit = limit.min(best.0);
        let opposite = n.children[1 - side];
        if opposite != INVALID_NODE
            && limit >= (i32::from(target[dim]) - i32::from(coordinate)).abs()
        {
            best = best.min(self.nearest_recursive(target, opposite, level + 1, limit, xy));
        }
        best
    }
    /// `FindNearest`: find the element closest to given coordinate, in Manhattan distance.
    /// For multiple elements with the same distance, the one comparing smaller with a
    /// less-than comparison is chosen.
    pub(crate) fn nearest(&self, target: [u16; 2], xy: &impl Fn(T, usize) -> u16) -> T {
        self.nearest_recursive(target, self.root, 0, i32::MAX, xy).1
    }
    /// `FindContainedRecursive`: visit the elements of a sub-tree inside the rectangle
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
        if p1[dim] < ec && n.children[0] != INVALID_NODE {
            self.contained_recursive(p1, p2, n.children[0], level + 1, xy, visit);
        }
        if p2[dim] > ec && n.children[1] != INVALID_NODE {
            self.contained_recursive(p1, p2, n.children[1], level + 1, xy, visit);
        }
    }
    /// `FindContained`: find all items contained within the given rectangle. Start
    /// coordinates are inclusive, end coordinates are exclusive; x1<x2 && y1<y2 is a
    /// precondition.
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

/// Partition split keys by coordinate using the configured native STL. Every
/// subtree with at least two elements makes one call, which moves whole keys.
pub(crate) fn native_split<T: Copy>(
    services: &crate::services::Services,
    keys: &mut [SplitKey<T>],
) -> usize {
    // The native key is two u16 fields; pool IDs are transparent u16.
    const { assert!(size_of::<T>() == 2 && size_of::<SplitKey<T>>() == 4) };
    (services.kdtree_partition)(keys.as_mut_ptr().cast(), keys.len())
}

#[cfg(test)]
mod tests {
    use super::{Kdtree, SplitKey};
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
        let split = |elements: &mut [SplitKey<u16>]| {
            let mid = elements.len() / 2;
            elements.select_nth_unstable_by_key(mid, |k| k.coordinate);
            let coordinate = elements[mid].coordinate;
            let (mut first, mut last) = (0, elements.len());
            loop {
                while first < last && elements[first].coordinate < coordinate {
                    first += 1;
                }
                if first == last {
                    break;
                }
                loop {
                    last -= 1;
                    if first == last || elements[last].coordinate < coordinate {
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
            assert_eq!(elements[first].coordinate, coordinate);
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
        tree.build(active.iter().copied(), &xy, &split);
        verify(&tree, &active);
        let mut contained = Vec::new();
        tree.contained([0, 0], [10, 10], &xy, |id| contained.push(id));
        contained.sort_unstable();
        assert_eq!(contained, [0, 4, 5, 7, 9]);
        tree.build([], &xy, &split);
        assert_eq!(tree.count(), 0);
        tree.insert(9, &xy, &split);
        assert_eq!(tree.nearest([0, 0], &xy), 9);
    }
}

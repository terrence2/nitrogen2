// This file is part of Nitrogen.
//
// Nitrogen is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Nitrogen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Nitrogen.  If not, see <http://www.gnu.org/licenses/>.
use std::{
    cmp::Ord,
    collections::{BinaryHeap, HashSet},
    fmt,
    hash::Hash,
};

// Wraps an item in the queue to provide the value inline for sorting and fast access.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct ItemCache<K, T> {
    pub(crate) cache: K,
    pub(crate) item: T,
}

impl<K, T> fmt::Debug for ItemCache<K, T>
where
    K: fmt::Display,
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{:.02}", self.item, self.cache)
    }
}

// The core of the queue is a binary heap. Insertions go right into the heap.
// There is an additional side HashSet of deletions so that we can skip removed
// items in the output stream without having to constantly re-build the heap
// for removals. A second side-hash records contents so that we can dedup
// insertions.
//
// Operation Cost Highlights:
//   * repeated insertion and removal of the same item will be O(1)
//   * deletion is O(1) with aggregated O(N) rebuild (which we have to take anyway)
//   * initial insertion is O(logn); subsequent re-insertion is O(1)
pub(crate) struct Queue<K, T> {
    heap: BinaryHeap<ItemCache<K, T>>,
    contents: HashSet<T>,
    removals: HashSet<T>,
}

impl<K, T> Queue<K, T>
where
    K: Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + fmt::Debug,
    T: Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + fmt::Debug,
{
    pub(crate) fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            contents: HashSet::new(),
            removals: HashSet::new(),
        }
    }

    pub(crate) fn check_invariants(&self) {
        assert_eq!(self.removals.len() + self.contents.len(), self.heap.len());
        for cache in self.heap.iter() {
            assert!(
                (self.removals.contains(&cache.item) && !self.contents.contains(&cache.item))
                    || (self.contents.contains(&cache.item)
                        && !self.removals.contains(&cache.item))
            );
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.heap
            .iter()
            .map(|cache| &cache.item)
            .filter(|&item| !self.removals.contains(item))
    }

    pub(crate) fn len(&self) -> usize {
        self.heap.len() - self.removals.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn contains(&self, item: &T) -> bool {
        self.contents.contains(item)
    }

    pub(crate) fn insert(&mut self, cache: K, item: T) {
        if self.contents.contains(&item) {
            // Direct, simple duplicate.
            return;
        }
        if self.removals.contains(&item) {
            // Unremove it and add it back to contents. It's still in the heap.
            self.removals.remove(&item);
            self.contents.insert(item);
            return;
        }
        self.contents.insert(item);
        self.heap.push(ItemCache { cache, item });
    }

    pub(crate) fn remove(&mut self, item: T) {
        if !self.contents.contains(&item) {
            // Note: either never added or already in removals
            return;
        }
        self.contents.remove(&item);
        self.removals.insert(item);
    }

    pub(crate) fn peek_value(&mut self) -> K {
        loop {
            let cache = self.heap.peek().expect("empty heap in peek");
            if self.removals.contains(&cache.item) {
                assert!(!self.contents.contains(&cache.item));
                self.removals.remove(&cache.item);
                self.heap.pop().unwrap(); // discard previously lazily removed item
                continue;
            }
            return cache.cache;
        }
    }

    pub(crate) fn pop(&mut self) -> T {
        loop {
            let cache = self.heap.pop().expect("empty heap in pop");
            if self.removals.contains(&cache.item) {
                assert!(!self.contents.contains(&cache.item));
                self.removals.remove(&cache.item);
                continue;
            }
            self.contents.remove(&cache.item);
            return cache.item;
        }
    }

    // The queue stores pointers; when our algorithm updates the values we sort these queues on,
    // we need to update the order of the queue. We do that in a batch with the following pair of
    // methods, instead of incrementally, for performance. See usage in update_priorities in the
    // world tree.

    // steal_content must be paired with restore_content; no actions are allowed on the queue while
    // the contents are stolen, because other metadata is not stolen.
    pub(crate) fn steal_content(&mut self) -> Vec<ItemCache<K, T>> {
        let mut sandbag = BinaryHeap::new();
        std::mem::swap(&mut self.heap, &mut sandbag);
        sandbag.into_vec()
    }

    // The given content will be re-sorted on being re-added to the heap.
    pub(crate) fn restore_content(&mut self, content: Vec<ItemCache<K, T>>) {
        assert_eq!(self.heap.len(), 0);
        // O(n) rebuild of the queue
        let mut sandbag = BinaryHeap::from(content);
        std::mem::swap(&mut self.heap, &mut sandbag);
    }

    pub(crate) fn is_tombstoned(&self, item: &T) -> bool {
        self.removals.contains(item)
    }
}

impl<K, T> fmt::Debug for Queue<K, T>
where
    K: fmt::Display,
    T: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "heap: {:?}\nrems: {:?}\nctnt: {:?}",
            self.heap, self.removals, self.contents
        )
    }
}

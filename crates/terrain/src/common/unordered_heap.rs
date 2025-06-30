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
use bv::BitVec;
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    fmt,
    fmt::{Display, Formatter},
    marker::PhantomData,
};

// Index into the tree vec.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Handle<Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    offset: usize,
    phantom: PhantomData<Tag>,
}

impl<Tag> Handle<Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    fn new(offset: usize) -> Self {
        Self {
            offset,
            phantom: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn heap_index_as_offset(&self) -> usize {
        self.offset
    }

    #[inline]
    fn heap_index_as_u64(&self) -> u64 {
        self.offset as u64
    }
}

impl<Tag> Display for Handle<Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Hdl[{}]", self.offset)
    }
}

/// A contiguous, growing collection of items supporting:
/// 1) Fast deletion
/// 2) Efficient (front first) re-use of dead slots
/// 3) Fast iteration via a tombstone bitmap
pub(crate) struct UnorderedHeap<Item, Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    storage: Vec<Item>,
    empty_set: BinaryHeap<Reverse<Handle<Tag>>>,
    empty_slots: BitVec,
}

impl<Item, Tag> UnorderedHeap<Item, Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    #[inline]
    pub(crate) fn alloc(&mut self, value: Item) -> Handle<Tag> {
        if let Some(Reverse(index)) = self.empty_set.pop() {
            self.empty_slots.set(index.heap_index_as_u64(), false);
            self.storage[index.heap_index_as_offset()] = value;
            return index;
        }
        let index = Handle::<Tag>::new(self.storage.len());
        self.storage.push(value);
        self.empty_slots.push(false);
        index
    }

    #[inline]
    pub(crate) fn free(&mut self, index: Handle<Tag>) {
        assert!(!self.empty_slots[index.heap_index_as_u64()]);
        self.empty_slots.set(index.heap_index_as_u64(), true);
        self.empty_set.push(Reverse(index));
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.storage.len() - self.empty_set.len()
    }

    #[inline]
    pub(crate) fn capacity(&self) -> usize {
        self.storage.len()
    }

    #[inline]
    pub(crate) fn get(&self, index: Handle<Tag>) -> &Item {
        assert!(!self.empty_slots.get(index.heap_index_as_u64()));
        &self.storage[index.heap_index_as_offset()]
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, index: Handle<Tag>) -> &mut Item {
        assert!(!self.empty_slots.get(index.heap_index_as_u64()));
        &mut self.storage[index.heap_index_as_offset()]
    }
}

impl<Item, Tag> Default for UnorderedHeap<Item, Tag>
where
    Tag: Copy + Clone + Ord + 'static,
{
    fn default() -> Self {
        Self {
            storage: Vec::new(),
            empty_set: BinaryHeap::new(),
            empty_slots: BitVec::new(),
        }
    }
}

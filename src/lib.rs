//! 基于slotmap的双端队列
//! 支持从队列头部添加或弹出
//! 支持从队列尾部添加或弹出
//! 与标准库的双端队列相比，本双端队列还支持根据索引快速从任意位置删除和查询，一些时候，可快速删除的双端队列十分有用（例如pi_lib中的task_pool）
//!
//! 特色： 将双端队列本身的逻辑和索引（删除就需要依赖索引）分离，因此，十分容易和其它需要索引的数据结构共享索引。
//! 关于共享索引的意义，请参考：https://github.com/GaiaWorld/pi_lib/tree/master/dyn_uint
//!
//! 选择:
//! - 当你需要使用双端队列，并且你不需要快速从任意位置删除和查询，标准库中的双端队列是一个不错的选择
//! - 当你的部分功能需要使用从任意位置删除和查询，部分功能不需要时，不太建议你同时依赖标准库与本库的双端队列，毕竟会增减应用程序的尺寸
//! 但如果你不在意，你可以这么做！这种情况下，
//! 建议的做法是，总是使用本库或其它的代替品,本库的双端队列性能仅比标准库略低（删除功能也需要一定成本）

use std::fmt::{Debug, Formatter, Result};
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::Deref;


use slotmap::{Key, SlotMap};

/// Storage shared by deques using the same key and value types.
pub struct Slot<K: Key, T>(SlotMap<K, LinkedNode<K, T>>);

impl<K: Key, T> Default for Slot<K, T> {
    fn default() -> Self {
        Self(SlotMap::with_key())
    }
}

impl<K: Key, T> Slot<K, T> {
    pub fn new() -> Self { Self::default() }
    /// Insert a detached element for later insertion with push_key_front/back.
    pub fn insert(&mut self, el: T) -> K {
        self.0.insert(LinkedNode::new(el, K::null(), K::null()))
    }
    pub fn get(&self, key: K) -> Option<&T> { self.0.get(key).map(|node| &node.el) }
    pub fn get_mut(&mut self, key: K) -> Option<&mut T> { self.0.get_mut(key).map(|node| &mut node.el) }
    pub fn contains_key(&self, key: K) -> bool { self.0.contains_key(key) }
    pub fn len(&self) -> usize { self.0.len() }
}

/// 双端队列
#[derive(Eq, PartialEq)]
pub struct Deque<K: Key, T> {
    head: K,
    tail: K,
    len: usize,
    marker: PhantomData<T>,
}

impl<K: Key, T> Default for Deque<K, T> {
    fn default() -> Self {
        Deque::new()
    }
}

impl<K: Key, T> Deque<K, T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            head: K::null(),
            tail: K::null(),
            len: 0,
            marker: PhantomData,
        }
    }
    /// Get head key
    #[inline]
    pub fn head(&self) -> K {
        self.head
    }
    /// Get tail key
    #[inline]
    pub fn tail(&self) -> K {
        self.tail
    }
    /// Number of nodes in this deque.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn ops<'a>(&'a mut self, slot: &'a mut Slot<K, T>) -> DequeOps<'a, K, T> {
        DequeOps { deque: self, slot }
    }

    pub fn iter<'a>(&self, slot: &'a Slot<K, T>) -> Iter<'a, K, T> {
        Iter { next: self.head, container: &slot.0 }
    }
}

/// Operations on a deque and its shared storage.
pub struct DequeOps<'a, K: Key, T> {
    deque: &'a mut Deque<K, T>,
    slot: &'a mut Slot<K, T>,
}

impl<K: Key, T> Deref for DequeOps<'_, K, T> {
    type Target = Deque<K, T>;
    fn deref(&self) -> &Self::Target { self.deque }
}

impl<K: Key, T> DequeOps<'_, K, T> {

    /// Append an element to the Deque. return a key
    pub fn push_back(&mut self, el: T) -> K {
        let slot = &mut self.slot.0;
        self.deque.len += 1;
        if self.deque.tail.is_null() {
            let key = slot.insert(LinkedNode::new(el, K::null(), K::null()));
            self.deque.head = key;
            self.deque.tail = key;
            key
        } else {
            let key = slot.insert(LinkedNode::new(el, self.deque.tail, K::null()));
            unsafe {
                slot.get_unchecked_mut(self.deque.tail).next = key;
            }
            self.deque.tail = key;
            key
        }
    }

    /// Prepend an element to the Deque. return a key
    pub fn push_front(&mut self, el: T) -> K {
        let slot = &mut self.slot.0;
        self.deque.len += 1;
        if self.deque.head.is_null() {
            let key = slot.insert(LinkedNode::new(el, K::null(), K::null()));
            self.deque.head = key;
            self.deque.tail = key;
            key
        } else {
            let key = slot.insert(LinkedNode::new(el, K::null(), self.deque.head));
            unsafe {
                slot.get_unchecked_mut(self.deque.head).prev = key;
            }
            self.deque.head = key;
            key
        }
    }
    /// Append an key to the Deque
    pub fn push_key_back(&mut self, key: K) {
        let slot = &mut self.slot.0;
        let node = unsafe { slot.get_unchecked_mut(key) };
        self.deque.len += 1;
        if self.deque.tail.is_null() {
            node.prev = K::null();
            node.next = K::null();
            self.deque.tail = key;
            self.deque.head = key;
        } else {
            node.prev = self.deque.tail;
            node.next = K::null();
            unsafe {
                slot.get_unchecked_mut(self.deque.tail).next = key;
            }
            self.deque.tail = key;
        }
    }
    /// Append an key to the Deque
    pub fn push_key_front(&mut self, key: K) {
        let slot = &mut self.slot.0;
        let node = unsafe { slot.get_unchecked_mut(key) };
        self.deque.len += 1;
        if self.deque.head.is_null() {
            node.prev = K::null();
            node.next = K::null();
            self.deque.head = key;
            self.deque.tail = key;
        } else {
            node.prev = K::null();
            node.next = self.deque.head;
            unsafe {
                slot.get_unchecked_mut(self.deque.head).prev = key;
            }
            self.deque.head = key;
        }
    }
    /// Insert an element before anchor; a null anchor appends to the tail.
    /// A non-null anchor must belong to this deque.
    pub fn insert_before(&mut self, el: T, anchor: K) -> K {
        if anchor.is_null() {
            return self.push_back(el);
        }
        let slot = &mut self.slot.0;
        let prev = slot[anchor].prev;
        let key = slot.insert(LinkedNode::new(el, prev, anchor));
        slot[anchor].prev = key;
        if prev.is_null() {
            self.deque.head = key;
        } else {
            slot[prev].next = key;
        }
        self.deque.len += 1;
        key
    }
    /// Insert an element after anchor; a null anchor prepends to the head.
    /// A non-null anchor must belong to this deque.
    pub fn insert_after(&mut self, el: T, anchor: K) -> K {
        if anchor.is_null() {
            return self.push_front(el);
        }
        let slot = &mut self.slot.0;
        let next = slot[anchor].next;
        let key = slot.insert(LinkedNode::new(el, anchor, next));
        slot[anchor].next = key;
        if next.is_null() {
            self.deque.tail = key;
        } else {
            slot[next].prev = key;
        }
        self.deque.len += 1;
        key
    }
    /// Removes the last element from the Deque and returns it, or None if it is empty.
    pub fn pop_back(&mut self) -> Option<T> {
        let slot = &mut self.slot.0;
        if let Some(node) = slot.remove(self.deque.tail) {
            self.deque.len -= 1;
            self.deque.tail = node.prev;
            if self.deque.tail.is_null() {
                self.deque.head = K::null();
            } else {
                unsafe { slot.get_unchecked_mut(self.deque.tail).next = K::null() };
            }
            Some(node.el)
        } else {
            None
        }
    }

    /// Removes the first element from the Deque and returns it, or None if it is empty.
    pub fn pop_front(&mut self) -> Option<T> {
        let slot = &mut self.slot.0;
        if let Some(node) = slot.remove(self.deque.head) {
            self.deque.len -= 1;
            self.deque.head = node.next;
            if self.deque.head.is_null() {
                self.deque.tail = K::null();
            } else {
                unsafe { slot.get_unchecked_mut(self.deque.head).prev = K::null() };
            }
            Some(node.el)
        } else {
            None
        }
    }

    /// Removes the first element and key from the Deque and returns it, or None if it is empty.
    pub fn pop_kv_front(&mut self) -> Option<(K, T)> {
        let slot = &mut self.slot.0;
        let key = self.deque.head;
        if let Some(node) = slot.remove(self.deque.head) {
            self.deque.len -= 1;
            self.deque.head = node.next;
            if self.deque.head.is_null() {
                self.deque.tail = K::null();
            } else {
                unsafe { slot.get_unchecked_mut(self.deque.head).prev = K::null() };
            }
            Some((key, node.el))
        } else {
            None
        }
    }

    ///Removes and returns the element at key from the Deque.
    pub fn remove(&mut self, key: K) -> Option<T> {
        if let Some(node) = self.slot.0.remove(key) {
            self.repair(node.prev, node.next);
            Some(node.el)
        } else {
            None
        }
    }
    /// Removes the last element from the Deque and returns key, or None if it is empty.
    pub fn pop_key_back(&mut self) -> Option<K> {
        let slot = &mut self.slot.0;
        if let Some(node) = slot.get(self.deque.tail) {
            self.deque.len -= 1;
            let old = self.deque.tail;
            self.deque.tail = node.prev;
            if self.deque.tail.is_null() {
                self.deque.head = K::null();
            } else {
                unsafe { slot.get_unchecked_mut(self.deque.tail).next = K::null() };
            }
            Some(old)
        } else {
            None
        }
    }

    /// Removes the first element from the Deque and returns key, or None if it is empty.
    pub fn pop_key_front(&mut self) -> Option<K> {
        let slot = &mut self.slot.0;
        if let Some(node) = slot.get(self.deque.head) {
            self.deque.len -= 1;
            let old = self.deque.head;
            self.deque.head = node.next;
            if self.deque.head.is_null() {
                self.deque.tail = K::null();
            } else {
                unsafe { slot.get_unchecked_mut(self.deque.head).prev = K::null() };
            }
            Some(old)
        } else {
            None
        }
    }

    /// Append a Deque to the tail, consuming the source Deque.
    pub fn append(&mut self, deque: Deque<K, T>) {
        let slot = &mut self.slot.0;
        if deque.head.is_null() {
            return
        }
        self.deque.len += deque.len;
        if !self.deque.tail.is_null() {
            unsafe { slot.get_unchecked_mut(self.deque.tail).next = deque.head };
            unsafe { slot.get_unchecked_mut(deque.head).prev = self.deque.tail };
        }else{
            self.deque.head = deque.head;
        }
        self.deque.tail = deque.tail;
    }
    ///repair Deque.
    fn repair(&mut self, prev: K, next: K) {
        let slot = &mut self.slot.0;
        self.deque.len -= 1;
        if prev.is_null() {
            if next.is_null() {
                //如果该元素既不存在上一个元素，也不存在下一个元素， 则设置队列的头部None， 则设置队列的尾部None
                self.deque.head = K::null();
                self.deque.tail = K::null();
            } else {
                //如果该元素不存在上一个元素，但存在下一个元素， 则将下一个元素的上一个元素设置为None, 并设置队列的头部为该元素的下一个元素
                unsafe { slot.get_unchecked_mut(next).prev = K::null() };
                self.deque.head = next;
            }
        } else if next.is_null() {
            //如果该元素存在上一个元素，不存在下一个元素， 则将上一个元素的下一个元素设置为None, 并设置队列的尾部为该元素的上一个元素
            unsafe { slot.get_unchecked_mut(prev).next = K::null() };
            self.deque.tail = prev;
        } else {
            //如果该元素既存在上一个元素，也存在下一个元素， 则将上一个元素的下一个元素设置为本元素的下一个元素, 下一个元素的上一个元素设置为本元素的上一个元素
            unsafe { slot.get_unchecked_mut(prev).next = next };
            unsafe { slot.get_unchecked_mut(next).prev = prev };
        }
    }
    //clear Deque
    pub fn clear(&mut self) {
        let slot = &mut self.slot.0;
        while !self.deque.head.is_null() {
            let node = slot.remove(self.deque.head).unwrap();
            self.deque.head = node.next;
        }
        self.deque.tail = K::null();
        self.deque.len = 0;
    }
}

pub struct Iter<'a, K: Key, T: 'a> {
    next: K,
    container: &'a SlotMap<K, LinkedNode<K, T>>,
}

impl<'a, K: Key, T> Iterator for Iter<'a, K, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.next.is_null() {
            return None;
        }
        let node = unsafe { self.container.get_unchecked(self.next) };
        self.next = node.next;
        Some(&node.el)
    }
}

impl<K: Key, T> Debug for Deque<K, T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Deque")
            .field("first", &self.head)
            .field("last", &self.tail)
            .field("len", &self.len)
            .finish()
    }
}

struct LinkedNode<K: Key, T> {
    el: T,
    prev: K,
    next: K,
}

impl<K: Key, T> LinkedNode<K, T> {
    fn new(el: T, prev: K, next: K) -> Self {
        LinkedNode {
            el,
            prev,
            next,
        }
    }
}

impl<K: Key, T: Debug> Debug for LinkedNode<K, T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("Node")
            .field("el", &self.el)
            .field("prev", &self.prev)
            .field("next", &self.next)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::DefaultKey;

    #[test]
    fn push_existing_key_front_preserves_links() {
        let mut slot: Slot<DefaultKey, i32> = Slot::new();
        let mut deque = Deque::new();
        let first = deque.ops(&mut slot).push_back(1);
        deque.ops(&mut slot).push_back(2);
        deque.ops(&mut slot).push_back(3);
        let key = slot.insert(0);

        deque.ops(&mut slot).push_key_front(key);

        assert_eq!(slot.0[first].prev, key);
        assert_eq!(deque.ops(&mut slot).pop_back(), Some(3));
        assert_eq!(deque.ops(&mut slot).pop_back(), Some(2));
        assert_eq!(deque.ops(&mut slot).pop_back(), Some(1));
        assert_eq!(deque.ops(&mut slot).pop_back(), Some(0));
    }

    #[test]
    fn append_preserves_order_and_consumes_source() {
        let mut slot: Slot<DefaultKey, i32> = Slot::new();
        let mut destination = Deque::new();
        let mut source = Deque::new();
        destination.ops(&mut slot).push_back(1);
        source.ops(&mut slot).push_back(2);
        source.ops(&mut slot).push_back(3);

        destination.ops(&mut slot).append(source);

        assert_eq!(destination.ops(&mut slot).pop_front(), Some(1));
        assert_eq!(destination.ops(&mut slot).pop_front(), Some(2));
        assert_eq!(destination.ops(&mut slot).pop_front(), Some(3));
        assert_eq!(destination.ops(&mut slot).pop_front(), None);
    }
}

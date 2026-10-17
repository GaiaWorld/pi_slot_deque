//! 基于 slotmap 的双端队列，支持按键访问、删除和任意锚点插入。
//!
//! [Deque] 仅保存头尾键和长度，元素由 [DequeStorage] 统一存储。
//! 多个队列可以共享同一存储，通过 [Deque::ops] 创建的 [DequeOps] 修改队列。
//! 内部节点及链接关系不对外暴露。
//!
//! # 使用约束
//!
//! 队列非空期间，必须始终使用创建其元素的同一存储。
//! 非空锚点及待删除的有效键必须属于当前队列，不能使用其他队列的键。
//! 库不会验证队列归属；违反约束可能破坏链接，并使内部未检查访问产生未定义行为。
//! 丢弃队列不会自动删除存储中的元素，需要释放元素时应先调用 [DequeOps::clear]。
//!
//! # 示例
//!
//! ```
//! use pi_slot_deque::{Deque, DequeStorage};
//! use slotmap::DefaultKey;
//!
//! let mut storage = DequeStorage::<DefaultKey, i32>::new();
//! let mut deque = Deque::new();
//! let key = deque.ops(&mut storage).push_back(2);
//! deque.ops(&mut storage).insert_before(1, key);
//! storage[key] = 20;
//! assert_eq!(deque.iter(&storage).copied().collect::<Vec<_>>(), vec![1, 20]);
//! assert_eq!(deque.ops(&mut storage).remove(key), Some(20));
//! deque.ops(&mut storage).clear();
//! assert_eq!(storage.len(), 0);
//! ```

use std::fmt::{Debug, Formatter, Result};
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::{Deref, Index, IndexMut};

use slotmap::{Key, SlotMap};

/// 多个相同键类型、元素类型的队列可共享的存储。
///
/// 仅提供元素访问；创建、删除元素必须通过 [DequeOps]。
/// 下标访问返回元素本身，无效或已删除的键会导致 panic。
pub struct DequeStorage<K: Key, T>(SlotMap<K, LinkedNode<K, T>>);

impl<K: Key, T> Default for DequeStorage<K, T> {
    fn default() -> Self {
        Self(SlotMap::with_key())
    }
}

impl<K: Key, T> DequeStorage<K, T> {
    /// 创建空存储，等价于 Default::default。
    pub fn new() -> Self {
        Self::default()
    }
    /// 创建空存储，预留至少可容纳 capacity 个元素的空间。
    ///
    /// 预留容量不是元素数量或上限，后续插入超过容量时会自动扩容。
    pub fn with_capacity(capacity: usize) -> Self {
        Self(SlotMap::with_capacity_and_key(capacity))
    }
    /// 按键获取元素；键无效或元素已删除时返回 None。
    pub fn get(&self, key: K) -> Option<&T> {
        self.0.get(key).map(|node| &node.el)
    }
    /// 按键获取可变元素引用，不允许修改节点链接；无效键返回 None。
    pub fn get_mut(&mut self, key: K) -> Option<&mut T> {
        self.0.get_mut(key).map(|node| &mut node.el)
    }
    /// 返回指定元素的后继键；队尾返回 Some(空键)，无效键返回 None。
    pub fn next(&self, key: K) -> Option<K> {
        self.0.get(key).map(|node| node.next)
    }
    /// 返回指定元素的前驱键；队首返回 Some(空键)，无效键返回 None。
    pub fn prev(&self, key: K) -> Option<K> {
        self.0.get(key).map(|node| node.prev)
    }
    /// 判断键是否仍对应存储中的元素，不检查元素属于哪个队列。
    pub fn contains_key(&self, key: K) -> bool {
        self.0.contains_key(key)
    }
    /// 返回整个存储的元素数，包含共享此存储的所有队列。
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<K: Key, T> Index<K> for DequeStorage<K, T> {
    type Output = T;

    fn index(&self, key: K) -> &Self::Output {
        &self.0[key].el
    }
}

impl<K: Key, T> IndexMut<K> for DequeStorage<K, T> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.0[key].el
    }
}

/// 队列的头尾键和长度，不拥有元素存储。
///
/// 不可复制或克隆。丢弃此值不会清理元素，应按需先调用 [DequeOps::clear]。
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
    /// 创建空队列，不分配元素存储。
    #[inline]
    pub fn new() -> Self {
        Self {
            head: K::null(),
            tail: K::null(),
            len: 0,
            marker: PhantomData,
        }
    }
    /// 返回队首键；空队列返回空键。
    #[inline]
    pub fn head(&self) -> K {
        self.head
    }
    /// 返回队尾键；空队列返回空键。
    #[inline]
    pub fn tail(&self) -> K {
        self.tail
    }
    /// 返回当前队列的元素数，时间复杂度为 O(1)。
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// 同时借用队列和对应存储，创建修改操作视图。
    ///
    /// 非空队列必须传入原存储，视图存活期间独占借用两者。
    pub fn ops<'a>(&'a mut self, slot: &'a mut DequeStorage<K, T>) -> DequeOps<'a, K, T> {
        DequeOps { deque: self, slot }
    }

    /// 按队首至队尾顺序遍历元素，只借用对应存储。
    ///
    /// 必须传入原存储；迭代结束后继续调用 next 仍返回 None。
    pub fn iter<'a>(&self, slot: &'a DequeStorage<K, T>) -> Iter<'a, K, T> {
        self.iter_from(self.head, slot)
    }

    /// 从指定键开始向队尾遍历，包含起始元素；空键或无效键产生空迭代。
    ///
    /// 有效起始键必须属于当前队列，且必须传入原存储。
    pub fn iter_from<'a>(&self, key: K, slot: &'a DequeStorage<K, T>) -> Iter<'a, K, T> {
        Iter {
            next: key,
            container: &slot.0,
            reverse: false,
        }
    }

    /// 从队尾沿前驱向队首遍历；必须传入原存储。
    pub fn iter_rev<'a>(&self, slot: &'a DequeStorage<K, T>) -> Iter<'a, K, T> {
        self.iter_rev_from(self.tail, slot)
    }

    /// 从指定键开始向队首遍历，包含起始元素；空键或无效键产生空迭代。
    ///
    /// 有效起始键必须属于当前队列，且必须传入原存储。
    pub fn iter_rev_from<'a>(&self, key: K, slot: &'a DequeStorage<K, T>) -> Iter<'a, K, T> {
        Iter {
            next: key,
            container: &slot.0,
            reverse: true,
        }
    }
}

/// 队列及其共享存储的独占借用视图，由 [Deque::ops] 创建。
///
/// 插入、弹出和删除会同时维护链接及队列长度。
/// 通过只读 Deref 可查询队列的 head、tail 和 len。
pub struct DequeOps<'a, K: Key, T> {
    deque: &'a mut Deque<K, T>,
    slot: &'a mut DequeStorage<K, T>,
}

impl<K: Key, T> Deref for DequeOps<'_, K, T> {
    type Target = Deque<K, T>;
    fn deref(&self) -> &Self::Target {
        self.deque
    }
}

impl<K: Key, T> DequeOps<'_, K, T> {
    /// 将元素插入队尾，返回新键，时间复杂度为 O(1)。
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

    /// 将元素插入队首，返回新键，时间复杂度为 O(1)。
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
    /// 在锚点前插入元素并返回新键；空锚点表示追加到队尾。
    ///
    /// 非空锚点必须有效且属于当前队列，时间复杂度为 O(1)。
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
    /// 在锚点后插入元素并返回新键；空锚点表示插入队首。
    ///
    /// 非空锚点必须有效且属于当前队列，时间复杂度为 O(1)。
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
    /// 从队尾删除并返回元素；空队列返回 None，原键随删除失效。
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

    /// 从队首删除并返回元素；空队列返回 None，原键随删除失效。
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

    /// 从队首删除并返回原键和元素；空队列返回 None。
    ///
    /// 返回的键已失效，仅用于标识被删除的元素，不可再次访问存储。
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

    /// 按键删除并返回元素；无效键返回 None，成功后原键失效。
    ///
    /// 有效键必须属于当前队列，不能传入共享存储中其他队列的键。
    pub fn remove(&mut self, key: K) -> Option<T> {
        if let Some(node) = self.slot.0.remove(key) {
            self.repair(node.prev, node.next);
            Some(node.el)
        } else {
            None
        }
    }
    /// 将来源队列整体追加到队尾，消耗来源队列，时间复杂度为 O(1)。
    ///
    /// 两个队列必须使用同一存储；原元素键保持有效。
    pub fn append(&mut self, deque: Deque<K, T>) {
        let slot = &mut self.slot.0;
        if deque.head.is_null() {
            return;
        }
        self.deque.len += deque.len;
        if !self.deque.tail.is_null() {
            unsafe { slot.get_unchecked_mut(self.deque.tail).next = deque.head };
            unsafe { slot.get_unchecked_mut(deque.head).prev = self.deque.tail };
        } else {
            self.deque.head = deque.head;
        }
        self.deque.tail = deque.tail;
    }
    // 删除一个节点后连接其前后邻居，并只递减一次队列长度。
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
    /// 删除当前队列的全部元素并重置头尾和长度，不影响同存储中的其他队列。
    ///
    /// 原元素键全部失效；时间复杂度为 O(n)。
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

/// 按创建时指定的方向返回元素共享引用，包含起始键对应的元素。
pub struct Iter<'a, K: Key, T: 'a> {
    next: K,
    container: &'a SlotMap<K, LinkedNode<K, T>>,
    reverse: bool,
}

impl<'a, K: Key, T> Iterator for Iter<'a, K, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.next.is_null() {
            return None;
        }
        let Some(node) = self.container.get(self.next) else {
            self.next = K::null();
            return None;
        };
        self.next = if self.reverse { node.prev } else { node.next };
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

// 内部节点：链接仅由队列操作维护，外部只能访问元素。
struct LinkedNode<K: Key, T> {
    el: T,
    prev: K,
    next: K,
}

impl<K: Key, T> LinkedNode<K, T> {
    fn new(el: T, prev: K, next: K) -> Self {
        LinkedNode { el, prev, next }
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
    fn append_preserves_order_and_consumes_source() {
        let mut slot: DequeStorage<DefaultKey, i32> = DequeStorage::new();
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

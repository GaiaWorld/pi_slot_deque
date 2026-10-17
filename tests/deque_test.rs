use pi_slot_deque::{Deque, Slot};
use slotmap::DefaultKey;

#[test]
fn empty_operations_and_debug() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::default();
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.iter(&slot).next(), None);
    assert!(format!("{deque:?}").contains("len: 0"));
    let mut ops = deque.ops(&mut slot);
    assert_eq!(ops.pop_front(), None);
    assert_eq!(ops.pop_back(), None);
    assert_eq!(ops.pop_kv_front(), None);
    assert_eq!(ops.pop_key_front(), None);
    assert_eq!(ops.pop_key_back(), None);
    ops.clear();
    assert_eq!(ops.len(), 0);
}

#[test]
fn push_pop_and_access_elements() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let middle = deque.ops(&mut slot).push_back(2);
    let back = deque.ops(&mut slot).push_back(3);
    let front = deque.ops(&mut slot).push_front(1);
    assert_eq!(deque.len(), 3);
    assert_eq!(deque.head(), front);
    assert_eq!(deque.tail(), back);
    assert_eq!(
        deque.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(slot.get(front), Some(&1));
    *slot.get_mut(middle).unwrap() = 20;
    assert_eq!(slot.get(middle), Some(&20));
    assert_eq!(deque.ops(&mut slot).pop_back(), Some(3));
    assert_eq!(deque.ops(&mut slot).pop_kv_front(), Some((front, 1)));
    assert_eq!(deque.ops(&mut slot).pop_front(), Some(20));
    assert_eq!(deque.len(), 0);
    assert!(!slot.contains_key(back));
}

#[test]
fn insert_before_and_after_preserve_order_and_length() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let null = deque.head();
    let second = deque.ops(&mut slot).insert_before(2, null);
    let fourth = deque.ops(&mut slot).insert_after(4, second);
    let first = deque.ops(&mut slot).insert_before(1, second);
    let third = deque.ops(&mut slot).insert_after(3, second);
    let fifth = deque.ops(&mut slot).insert_before(5, null);
    assert_eq!(deque.len(), 5);
    assert_eq!(deque.head(), first);
    assert_eq!(deque.tail(), fifth);
    assert_eq!(
        deque.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(deque.ops(&mut slot).remove(third), Some(3));
    assert_eq!(deque.ops(&mut slot).pop_back(), Some(5));
    assert_eq!(deque.ops(&mut slot).pop_back(), Some(4));
    assert_eq!(deque.ops(&mut slot).pop_back(), Some(2));
    assert_eq!(deque.ops(&mut slot).pop_back(), Some(1));
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.ops(&mut slot).remove(third), None);
    assert_eq!(slot.len(), 0);
    assert!(!slot.contains_key(fourth));
}

#[test]
fn null_anchor_after_prepends_and_before_appends() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let null = deque.head();
    let back = deque.ops(&mut slot).insert_after(2, null);
    let front = deque.ops(&mut slot).insert_after(1, null);
    assert_eq!(deque.head(), front);
    assert_eq!(deque.tail(), back);
    assert_eq!(deque.ops(&mut slot).pop_front(), Some(1));
    assert_eq!(deque.ops(&mut slot).pop_front(), Some(2));
    let only = deque.ops(&mut slot).insert_before(3, null);
    assert_eq!(deque.head(), only);
    assert_eq!(deque.tail(), only);
    assert_eq!(deque.ops(&mut slot).pop_front(), Some(3));
}

#[test]
fn detached_keys_reinsert_and_remove_without_deleting_values() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let detached = slot.insert(1);
    deque.ops(&mut slot).push_key_back(detached);
    let other = deque.ops(&mut slot).push_back(2);
    assert_eq!(deque.ops(&mut slot).pop_key_front(), Some(detached));
    deque.ops(&mut slot).push_key_front(detached);
    assert!(deque.ops(&mut slot).remove_key(other));
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.ops(&mut slot).pop_key_front(), Some(detached));
    assert_eq!(deque.len(), 0);
    let null = deque.head();
    assert!(!deque.ops(&mut slot).remove_key(null));
    assert_eq!(slot.get(detached), Some(&1));
    assert_eq!(slot.get(other), Some(&2));
}

#[test]
fn key_pops_and_clear_track_length() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let first = deque.ops(&mut slot).push_front(1);
    let last = deque.ops(&mut slot).push_back(2);
    assert_eq!(deque.ops(&mut slot).pop_key_back(), Some(last));
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.ops(&mut slot).pop_key_front(), Some(first));
    assert_eq!(deque.len(), 0);
    deque.ops(&mut slot).push_key_front(last);
    deque.ops(&mut slot).push_key_back(first);
    assert_eq!(deque.len(), 2);
    deque.ops(&mut slot).clear();
    assert_eq!(deque.len(), 0);
    assert_eq!(slot.len(), 0);
}

#[test]
fn append_consumes_source_and_updates_length() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut destination = Deque::new();
    let mut source = Deque::new();
    source.ops(&mut slot).push_back(2);
    source.ops(&mut slot).push_back(3);
    destination.ops(&mut slot).append(Deque::new());
    assert_eq!(destination.len(), 0);
    destination.ops(&mut slot).append(source);
    assert_eq!(destination.len(), 2);
    let mut source = Deque::new();
    source.ops(&mut slot).push_back(4);
    destination.ops(&mut slot).append(source);
    assert_eq!(destination.len(), 3);
    assert_eq!(destination.ops(&mut slot).pop_front(), Some(2));
    assert_eq!(destination.ops(&mut slot).pop_front(), Some(3));
    assert_eq!(destination.ops(&mut slot).pop_front(), Some(4));
    assert_eq!(destination.len(), 0);
}

#[test]
fn remove_key_unlinks_head_middle_and_tail_without_removing_values() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let first = deque.ops(&mut slot).push_back(1);
    let middle = deque.ops(&mut slot).push_back(2);
    let last = deque.ops(&mut slot).push_back(3);
    assert!(deque.ops(&mut slot).remove_key(middle));
    assert_eq!(deque.len(), 2);
    assert_eq!(deque.iter(&slot).copied().collect::<Vec<_>>(), vec![1, 3]);
    assert!(deque.ops(&mut slot).remove_key(first));
    assert_eq!(deque.head(), last);
    assert_eq!(deque.len(), 1);
    assert!(deque.ops(&mut slot).remove_key(last));
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(slot.len(), 3);
}

#[test]
fn append_to_nonempty_destination_preserves_order() {
    let mut slot = Slot::<DefaultKey, i32>::new();
    let mut destination = Deque::new();
    let mut source = Deque::new();
    destination.ops(&mut slot).push_back(1);
    source.ops(&mut slot).push_back(2);
    destination.ops(&mut slot).append(source);
    assert_eq!(destination.len(), 2);
    assert_eq!(
        destination.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(destination.ops(&mut slot).pop_back(), Some(2));
    assert_eq!(destination.ops(&mut slot).pop_back(), Some(1));
}

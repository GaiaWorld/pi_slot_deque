use pi_slot_deque::{Deque, DequeStorage};
use slotmap::DefaultKey;

#[test]
fn neighbors_and_directional_iteration_follow_queue_links() {
    let mut storage = DequeStorage::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let null = deque.head();
    let first = deque.ops(&mut storage).push_back(1);
    let middle = deque.ops(&mut storage).push_back(2);
    let last = deque.ops(&mut storage).push_back(3);
    assert_eq!(storage.prev(first), Some(null));
    assert_eq!(storage.next(first), Some(middle));
    assert_eq!(storage.prev(middle), Some(first));
    assert_eq!(storage.next(middle), Some(last));
    assert_eq!(storage.next(last), Some(null));
    assert_eq!(
        deque
            .iter_from(middle, &storage)
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(
        deque.iter_rev(&storage).copied().collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert_eq!(
        deque
            .iter_rev_from(middle, &storage)
            .copied()
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(
        deque.iter_from(last, &storage).copied().collect::<Vec<_>>(),
        vec![3]
    );
    assert_eq!(
        deque
            .iter_rev_from(first, &storage)
            .copied()
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(deque.ops(&mut storage).remove(middle), Some(2));
    assert_eq!(storage.next(first), Some(last));
    assert_eq!(storage.prev(last), Some(first));
    assert_eq!(storage.next(middle), None);
    assert_eq!(storage.prev(middle), None);
    let mut invalid = deque.iter_from(middle, &storage);
    assert_eq!(invalid.next(), None);
    assert_eq!(invalid.next(), None);
    assert_eq!(deque.iter_rev_from(middle, &storage).next(), None);
    assert_eq!(deque.iter_from(null, &storage).next(), None);
    assert_eq!(deque.iter_rev_from(null, &storage).next(), None);
    assert_eq!(storage.next(null), None);
    assert_eq!(storage.prev(null), None);
}

#[test]
fn reverse_iteration_handles_empty_singleton_and_exhaustion() {
    let mut storage = DequeStorage::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    assert_eq!(deque.iter_rev(&storage).next(), None);
    let only = deque.ops(&mut storage).push_back(1);
    let mut reverse = deque.iter_rev_from(only, &storage);
    assert_eq!(reverse.next(), Some(&1));
    assert_eq!(reverse.next(), None);
    assert_eq!(reverse.next(), None);
    let mut forward = deque.iter_from(only, &storage);
    assert_eq!(forward.next(), Some(&1));
    assert_eq!(forward.next(), None);
    assert_eq!(forward.next(), None);
}

#[test]
fn with_capacity_starts_empty_and_allows_growth() {
    for capacity in [0, 2, 8] {
        let mut storage = DequeStorage::<DefaultKey, i32>::with_capacity(capacity);
        let mut deque = Deque::new();
        assert_eq!(storage.len(), 0);
        for value in 0..10 {
            let key = deque.ops(&mut storage).push_back(value);
            assert_eq!(storage[key], value);
        }
        assert_eq!(storage.len(), 10);
        assert_eq!(
            deque.iter(&storage).copied().collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
        for value in 0..10 {
            assert_eq!(deque.ops(&mut storage).pop_front(), Some(value));
        }
        assert_eq!(storage.len(), 0);
    }
}

#[test]
fn empty_operations_and_debug() {
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
    let mut deque = Deque::default();
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.iter(&slot).next(), None);
    assert!(format!("{deque:?}").contains("len: 0"));
    let mut ops = deque.ops(&mut slot);
    assert_eq!(ops.pop_front(), None);
    assert_eq!(ops.pop_back(), None);
    assert_eq!(ops.pop_kv_front(), None);
    ops.clear();
    assert_eq!(ops.len(), 0);
}

#[test]
fn push_pop_and_access_elements() {
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
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
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
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
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
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
fn append_consumes_source_and_updates_length() {
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
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
fn append_to_nonempty_destination_preserves_order() {
    let mut slot = DequeStorage::<DefaultKey, i32>::new();
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

#[test]
fn storage_indexes_values_without_exposing_nodes() {
    let mut storage = DequeStorage::<DefaultKey, String>::new();
    let mut deque = Deque::new();
    let key = deque.ops(&mut storage).push_back(String::from("before"));
    assert_eq!(storage[key], "before");
    storage[key] = String::from("after");
    assert_eq!(
        deque.ops(&mut storage).pop_front(),
        Some(String::from("after"))
    );
    assert_eq!(storage.get(key), None);
}

#[test]
#[should_panic]
fn storage_index_panics_for_removed_key() {
    let mut storage = DequeStorage::<DefaultKey, i32>::new();
    let mut deque = Deque::new();
    let key = deque.ops(&mut storage).push_back(1);
    assert_eq!(deque.ops(&mut storage).remove(key), Some(1));
    let _value = storage[key];
}

#[test]
fn clear_only_removes_current_deques_elements() {
    let mut storage = DequeStorage::<DefaultKey, i32>::new();
    let mut first = Deque::new();
    let mut second = Deque::new();
    let removed = first.ops(&mut storage).push_back(1);
    first.ops(&mut storage).push_back(2);
    let retained = second.ops(&mut storage).push_back(3);
    first.ops(&mut storage).clear();
    first.ops(&mut storage).clear();
    assert_eq!(first.len(), 0);
    assert_eq!(storage.get(removed), None);
    assert_eq!(storage.len(), 1);
    assert_eq!(storage[retained], 3);
    assert_eq!(second.ops(&mut storage).pop_front(), Some(3));
}

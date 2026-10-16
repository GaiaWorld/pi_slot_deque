use pi_slot_deque::{Deque, LinkedNode, Slot};
use slotmap::{DefaultKey, SlotMap};

#[test]
fn empty_deque_has_null_ends_and_no_elements() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::default();

    assert_eq!(deque, Deque::new());
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.pop_front(&mut slot), None);
    assert_eq!(deque.pop_back(&mut slot), None);
    assert_eq!(deque.pop_kv_front(&mut slot), None);
    assert_eq!(deque.pop_key_front(&mut slot), None);
    assert_eq!(deque.pop_key_back(&mut slot), None);
    assert_eq!(deque.iter(&slot).next(), None);
    assert_eq!(
        format!("{deque:?}"),
        format!("{:?}", Deque::<DefaultKey>::new())
    );
}

#[test]
fn push_both_ends_updates_keys_links_and_iteration_order() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let middle = deque.push_back(2, &mut slot);
    let back = deque.push_back(3, &mut slot);
    let front = deque.push_front(1, &mut slot);

    assert_eq!(deque.head(), front);
    assert_eq!(deque.tail(), back);
    assert_eq!(slot[front].next(), middle);
    assert_eq!(slot[middle].prev(), front);
    assert_eq!(slot[middle].next(), back);
    assert_eq!(slot[back].prev(), middle);
    assert_eq!(
        deque.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(deque.pop_back(&mut slot), Some(3));
    assert_eq!(deque.pop_front(&mut slot), Some(1));
    assert_eq!(deque.pop_back(&mut slot), Some(2));
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.pop_back(&mut slot), None);
}

#[test]
fn pushing_to_front_of_empty_deque_sets_both_ends() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();

    let key = deque.push_front(5, &mut slot);

    assert_eq!(deque.head(), key);
    assert_eq!(deque.tail(), key);
    assert_eq!(deque.pop_front(&mut slot), Some(5));
    assert_eq!(deque.pop_front(&mut slot), None);
}

#[test]
fn key_pop_keeps_node_in_slot_for_reinsertion_at_back() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);

    assert_eq!(deque.pop_key_front(&mut slot), Some(first));
    assert_eq!(slot[first].el, 1);
    assert_eq!(deque.head(), second);
    assert_eq!(slot[second].prev(), Deque::<DefaultKey>::new().head());
    deque.push_key_back(first, &mut slot);

    assert_eq!(deque.iter(&slot).copied().collect::<Vec<_>>(), vec![2, 1]);
    assert_eq!(slot[second].next(), first);
    assert_eq!(deque.pop_key_back(&mut slot), Some(first));
    assert_eq!(slot[first].el, 1);
    assert_eq!(deque.pop_key_back(&mut slot), Some(second));
    assert_eq!(deque.pop_key_back(&mut slot), None);
}

#[test]
fn push_key_into_empty_deque_resets_old_links() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);
    assert_eq!(deque.pop_key_back(&mut slot), Some(second));
    assert_eq!(deque.pop_key_back(&mut slot), Some(first));

    deque.push_key_front(second, &mut slot);

    assert_eq!(deque.head(), second);
    assert_eq!(deque.tail(), second);
    assert_eq!(slot[second].prev(), Deque::<DefaultKey>::new().head());
    assert_eq!(slot[second].next(), Deque::<DefaultKey>::new().head());
    assert_eq!(deque.pop_front(&mut slot), Some(2));
    deque.push_key_back(first, &mut slot);
    assert_eq!(deque.pop_front(&mut slot), Some(1));
}

#[test]
fn pop_kv_front_returns_key_and_value_and_repairs_head() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);

    assert_eq!(deque.pop_kv_front(&mut slot), Some((first, 1)));
    assert!(!slot.contains_key(first));
    assert_eq!(deque.head(), second);
    assert_eq!(slot[second].prev(), Deque::<DefaultKey>::new().head());
    assert_eq!(deque.pop_kv_front(&mut slot), Some((second, 2)));
    assert_eq!(deque.pop_kv_front(&mut slot), None);
}

#[test]
fn remove_value_unlinks_head_middle_and_tail() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let head = deque.push_back(1, &mut slot);
    let middle = deque.push_back(2, &mut slot);
    let tail = deque.push_back(3, &mut slot);
    let missing = Deque::<DefaultKey>::new().head();

    assert_eq!(deque.remove(missing, &mut slot), None);
    assert_eq!(deque.remove(middle, &mut slot), Some(2));
    assert_eq!(slot[head].next(), tail);
    assert_eq!(slot[tail].prev(), head);
    assert_eq!(deque.remove(head, &mut slot), Some(1));
    assert_eq!(deque.head(), tail);
    assert_eq!(deque.remove(tail, &mut slot), Some(3));
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.remove(tail, &mut slot), None);
}

#[test]
fn remove_key_detaches_without_deleting_node() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let head = deque.push_back(1, &mut slot);
    let middle = deque.push_back(2, &mut slot);
    let tail = deque.push_back(3, &mut slot);

    assert!(!deque.remove_key(Deque::<DefaultKey>::new().head(), &mut slot));
    assert!(deque.remove_key(middle, &mut slot));
    assert_eq!(slot[middle].el, 2);
    assert_eq!(deque.iter(&slot).copied().collect::<Vec<_>>(), vec![1, 3]);
    assert!(deque.remove_key(head, &mut slot));
    assert_eq!(deque.head(), tail);
    assert!(deque.remove_key(tail, &mut slot));
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(slot.len(), 3);
}

#[test]
fn repair_connects_neighbors_after_external_slot_removal() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let middle = deque.push_back(2, &mut slot);
    let last = deque.push_back(3, &mut slot);
    let prev = slot[middle].prev();
    let next = slot[middle].next();
    assert_eq!(slot.remove(middle).map(|node| node.el), Some(2));

    deque.repair(prev, next, &mut slot);

    assert_eq!(slot[first].next(), last);
    assert_eq!(slot[last].prev(), first);
    assert_eq!(deque.iter(&slot).copied().collect::<Vec<_>>(), vec![1, 3]);
}

#[test]
fn clear_removes_only_this_deques_nodes() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);
    let external = slot.insert(LinkedNode::new(9, deque.head(), deque.tail()));

    deque.clear(&mut slot);

    assert!(!slot.contains_key(first));
    assert!(!slot.contains_key(second));
    assert_eq!(slot[external].el, 9);
    assert_eq!(deque.head(), deque.tail());
    assert_eq!(deque.iter(&slot).next(), None);
    deque.clear(&mut slot);
    assert_eq!(deque.pop_back(&mut slot), None);
}

#[test]
fn merging_empty_and_nonempty_deques_preserves_order() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut destination = Deque::new();
    let mut source = Deque::new();
    let first = source.push_back(1, &mut slot);
    let last = source.push_back(2, &mut slot);

    destination.append(Deque::new(), &mut slot);
    destination.append(source, &mut slot);

    assert_eq!(destination.head(), first);
    assert_eq!(destination.tail(), last);
    assert_eq!(
        destination.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(destination.pop_back(&mut slot), Some(2));
    assert_eq!(destination.pop_back(&mut slot), Some(1));
}

#[test]
fn linked_node_constructor_accessors_and_debug_reflect_values() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);

    let node = LinkedNode::new(3, first, second);

    assert_eq!(node.el, 3);
    assert_eq!(node.prev(), first);
    assert_eq!(node.next(), second);
    assert!(format!("{node:?}").contains("el: 3"));
    assert!(format!("{deque:?}").contains("Deque"));
    assert_eq!(deque.pop_front(&mut slot), Some(1));
}

#[test]
fn insert_before_inserts_at_head_middle_and_end() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let null = deque.head();

    let second = deque.insert_before(2, null, &mut slot);
    assert_eq!(deque.len(), 1);
    let fourth = deque.insert_before(4, null, &mut slot);
    let first = deque.insert_before(1, second, &mut slot);
    let third = deque.insert_before(3, fourth, &mut slot);

    assert_eq!(deque.len(), 4);
    assert_eq!(deque.head(), first);
    assert_eq!(deque.tail(), fourth);
    assert_eq!(slot[first].prev(), null);
    assert_eq!(slot[first].next(), second);
    assert_eq!(slot[second].prev(), first);
    assert_eq!(slot[second].next(), third);
    assert_eq!(slot[third].prev(), second);
    assert_eq!(slot[third].next(), fourth);
    assert_eq!(slot[fourth].prev(), third);
    assert_eq!(slot[fourth].next(), null);
    assert_eq!(
        deque.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(deque.pop_back(&mut slot), Some(4));
    assert_eq!(deque.pop_back(&mut slot), Some(3));
    assert_eq!(deque.pop_back(&mut slot), Some(2));
    assert_eq!(deque.pop_back(&mut slot), Some(1));
    assert_eq!(deque.len(), 0);
}

#[test]
fn insert_after_inserts_at_head_middle_and_end() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let null = deque.head();

    let third = deque.insert_after(3, null, &mut slot);
    assert_eq!(deque.len(), 1);
    let first = deque.insert_after(1, null, &mut slot);
    let fourth = deque.insert_after(4, third, &mut slot);
    let second = deque.insert_after(2, first, &mut slot);

    assert_eq!(deque.len(), 4);
    assert_eq!(deque.head(), first);
    assert_eq!(deque.tail(), fourth);
    assert_eq!(slot[first].prev(), null);
    assert_eq!(slot[first].next(), second);
    assert_eq!(slot[second].prev(), first);
    assert_eq!(slot[second].next(), third);
    assert_eq!(slot[third].prev(), second);
    assert_eq!(slot[third].next(), fourth);
    assert_eq!(slot[fourth].prev(), third);
    assert_eq!(slot[fourth].next(), null);
    assert_eq!(
        deque.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(deque.pop_front(&mut slot), Some(1));
    assert_eq!(deque.pop_front(&mut slot), Some(2));
    assert_eq!(deque.pop_front(&mut slot), Some(3));
    assert_eq!(deque.pop_front(&mut slot), Some(4));
    assert_eq!(deque.len(), 0);
}

#[test]
fn len_tracks_all_push_pop_remove_and_repair_paths() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    assert_eq!(deque.len(), 0);
    let first = deque.push_front(1, &mut slot);
    assert_eq!(deque.len(), 1);
    let second = deque.push_back(2, &mut slot);
    assert_eq!(deque.len(), 2);
    let third = slot.insert(LinkedNode::new(3, deque.head(), deque.tail()));
    deque.push_key_front(third, &mut slot);
    assert_eq!(deque.len(), 3);
    assert_eq!(deque.pop_key_front(&mut slot), Some(third));
    assert_eq!(deque.len(), 2);
    deque.push_key_back(third, &mut slot);
    assert_eq!(deque.len(), 3);
    assert_eq!(deque.pop_kv_front(&mut slot), Some((first, 1)));
    assert_eq!(deque.len(), 2);
    assert!(deque.remove_key(second, &mut slot));
    assert_eq!(deque.len(), 1);
    assert!(!deque.remove_key(first, &mut slot));
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.remove(third, &mut slot), Some(3));
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.remove(third, &mut slot), None);
    assert_eq!(deque.pop_front(&mut slot), None);
    assert_eq!(deque.len(), 0);

    let fourth = deque.push_back(4, &mut slot);
    let fifth = deque.push_back(5, &mut slot);
    assert_eq!(deque.pop_key_back(&mut slot), Some(fifth));
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.pop_back(&mut slot), Some(4));
    assert_eq!(deque.len(), 0);
    deque.push_key_back(fifth, &mut slot);
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.pop_front(&mut slot), Some(5));
    assert_eq!(deque.len(), 0);
    assert!(!slot.contains_key(fourth));

    let sixth = deque.push_back(6, &mut slot);
    let seventh = deque.push_back(7, &mut slot);
    slot.remove(sixth);
    deque.repair(Deque::<DefaultKey>::new().head(), seventh, &mut slot);
    assert_eq!(deque.len(), 1);
    deque.clear(&mut slot);
    assert_eq!(deque.len(), 0);
    deque.clear(&mut slot);
    assert_eq!(deque.len(), 0);
}

#[test]
fn len_tracks_append_with_empty_and_nonempty_queues() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut destination = Deque::new();
    destination.push_back(1, &mut slot);
    destination.append(Deque::new(), &mut slot);
    assert_eq!(destination.len(), 1);
    let mut source = Deque::new();
    source.push_back(2, &mut slot);
    source.push_back(3, &mut slot);
    destination.append(source, &mut slot);
    assert_eq!(destination.len(), 3);
    let mut empty_destination = Deque::new();
    empty_destination.append(destination, &mut slot);
    assert_eq!(empty_destination.len(), 3);
    assert_eq!(
        empty_destination.iter(&slot).copied().collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn repair_tracks_head_tail_and_only_node_removals() {
    let mut slot: Slot<DefaultKey, i32> = SlotMap::with_key();
    let mut deque = Deque::new();
    let first = deque.push_back(1, &mut slot);
    let second = deque.push_back(2, &mut slot);
    let third = deque.push_back(3, &mut slot);
    let null = Deque::<DefaultKey>::new().head();

    let removed = slot.remove(first).unwrap();
    deque.repair(removed.prev(), removed.next(), &mut slot);
    assert_eq!(deque.len(), 2);
    assert_eq!(deque.head(), second);
    let removed = slot.remove(third).unwrap();
    deque.repair(removed.prev(), removed.next(), &mut slot);
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.tail(), second);
    let removed = slot.remove(second).unwrap();
    deque.repair(removed.prev(), removed.next(), &mut slot);
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.head(), null);
    assert_eq!(deque.tail(), null);
}

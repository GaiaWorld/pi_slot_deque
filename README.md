# pi_slot_deque

基于 slotmap 的双端队列，支持稳定键访问、按键删除和任意锚点前后插入。
队列状态与元素存储分离，多个队列可共享同一个存储。

## 类型职责

| 类型 | 职责 |
| --- | --- |
| `Deque<K, T>` | 保存当前队列的头尾键及长度，不拥有存储，不可复制或克隆 |
| `DequeStorage<K, T>` | 保存各队列的元素和内部节点，提供元素查询及下标访问 |
| `DequeOps<'a, K, T>` | 同时借用队列与存储，统一维护插入、删除、合并和清空 |
| `Iter<'a, K, T>` | 按队首至队尾顺序返回元素共享引用 |

内部节点和修复链接的方法不公开。存储不提供直接插入或删除，也不提供按键挂入、摘出节点的接口。

## 基本用法

```rust
use pi_slot_deque::{Deque, DequeStorage};
use slotmap::DefaultKey;

let mut storage = DequeStorage::<DefaultKey, i32>::new();
let mut deque = Deque::new();

let middle = deque.ops(&mut storage).push_back(2);
deque.ops(&mut storage).insert_before(1, middle);
deque.ops(&mut storage).insert_after(3, middle);
storage[middle] = 20;

assert_eq!(deque.len(), 3);
assert_eq!(deque.iter(&storage).copied().collect::<Vec<_>>(), vec![1, 20, 3]);
assert_eq!(deque.ops(&mut storage).remove(middle), Some(20));
assert_eq!(storage.get(middle), None);
deque.ops(&mut storage).clear();
```

示例中的 `slotmap` 是 `pi_slotmap` 的依赖别名，应用可在 Cargo.toml 中声明：

```toml
[dependencies]
pi_slot_deque = "0.4.1"
slotmap = { package = "pi_slotmap", version = "0.4" }
```

## 共享存储与合并

```rust
use pi_slot_deque::{Deque, DequeStorage};
use slotmap::DefaultKey;

let mut storage = DequeStorage::<DefaultKey, i32>::new();
let mut destination = Deque::new();
let mut source = Deque::new();
destination.ops(&mut storage).push_back(1);
let key = source.ops(&mut storage).push_back(2);

destination.ops(&mut storage).append(source); // 来源队列被消耗，之后不可使用。
assert_eq!(destination.len(), 2);
assert_eq!(storage[key], 2); // 合并不改变原元素键。
destination.ops(&mut storage).clear();
```

## 操作说明

| 接口 | 行为 |
| --- | --- |
| `Deque::new/default` | 创建空队列 |
| `head/tail` | 返回队首、队尾键，空队列返回空键 |
| `Deque::len` | 返回当前队列长度，O(1) |
| `ops(&mut storage)` | 创建独占借用的操作视图 |
| `iter(&storage)` | 顺序遍历元素，不修改存储 |
| `push_front/push_back` | 创建元素并插入两端，返回新键，O(1) |
| `insert_before(value, anchor)` | 锚点前插入；空锚点表示追加队尾，O(1) |
| `insert_after(value, anchor)` | 锚点后插入；空锚点表示插入队首，O(1) |
| `pop_front/pop_back` | 删除并返回两端元素，空队列返回 None，O(1) |
| `pop_kv_front` | 删除队首并返回原键与元素；返回的键已经失效，O(1) |
| `remove(key)` | 删除当前队列中指定键的元素，无效键返回 None，O(1) |
| `append(source)` | 消耗同存储的来源队列并追加到队尾，O(1) |
| `clear` | 删除当前队列的全部元素，不影响其他队列，O(n) |

`DequeStorage` 提供 `new/default`、`get`、`get_mut`、`contains_key`、`len`。
`storage[key]` 可读写元素，无效键会 panic；不确定键是否有效时使用返回 Option 的查询方法。
存储的 `len` 是所有共享队列的元素总数，不是某一个队列的长度。

## 调用约束

- 非空队列必须始终传入创建其元素的同一个存储；相同的键类型并不代表存储实例相同。
- 插入时的非空锚点及删除时的有效键必须属于当前队列，不能使用其他队列的键。
- `append` 的两个队列必须来自同一个存储。
- 库不会验证上述归属关系。违反约束可能破坏链表，并使内部未检查访问产生未定义行为；这些约束不是运行时安全检查的替代品。
- 删除、弹出或清空后，原键失效；合并后原键保持有效。
- 丢弃队列或操作视图不会自动删除存储中的元素，需要释放时应先调用 `clear`。丢弃存储会释放其中剩余元素。

## 验证

```text
cargo test
cargo doc --no-deps
```

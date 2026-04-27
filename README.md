# Hypertrie

A thread-safe, memory-efficient basic RDF hypertrie implemented in Rust, with a C++ FFI interface.

Stores RDF triples `(subject, predicate, object)` as `u32` resource IDs in a trie structure. Each level of the trie corresponds to one position in the triple. Concurrent access is handled via [`DashMap`](https://github.com/xacrimon/dashmap) (sharded locking) and `AtomicBool`/`AtomicU64` for lock-free flag and counter operations.

## Structure

```
hypertrie/
├── src/lib.rs          # Core trie logic, FFI layer, unit tests
├── tests/
│   └── integration.rs  # Concurrent and FFI stress tests
├── build.rs            # Generates include/hypertrie.h via cbindgen
├── cbindgen.toml       # C++ header generation config
├── include/
│   └── hypertrie.h     # Auto-generated C++ header (do not edit)
└── cpp/
    └── main.cpp        # C++ entry point and tests
```

## Design

| Component      | Choice                    | Reason                                                           |
|----------------|---------------------------|------------------------------------------------------------------|
| Children map   | `DashMap<u32, Arc<Node>>` | Sharded locking, concurrent reads and writes without global lock |
| End marker     | `AtomicBool`              | Lock-free flag per node                                          |
| Triple counter | `AtomicU64`               | Lock-free unique insert count                                    |
| Node sharing   | `Arc<Node>`               | Shared ownership across threads without copying                  |
| FFI            | `extern "C"` + cbindgen   | Safe, ABI-stable C/C++ interface                                 |

## Requirements

- Rust `1.93.1`+ (edition 2024)
- Cargo `1.93.1`+
- C++17-capable compiler (tested with Apple Clang 17 / GCC)

## Build

### Rust library

```bash
cargo build           # debug
cargo build --release # optimised
```

The C++ header is auto-generated at `include/hypertrie.h` on every build.

### C++ binary (macOS)

```bash
cargo build --release

g++ -std=c++17 -pthread \
    -o cpp/main \
    cpp/main.cpp \
    target/release/libhypertrie.a \
    -liconv -lc -lm
```

### C++ binary (Linux)

Run this first to get the exact flags for your system:

```bash
cargo rustc --release -- --print native-static-libs 2>&1 | grep "native-static-libs"
```

Then substitute the output into:

```bash
g++ -std=c++17 -pthread \
    -o cpp/main \
    cpp/main.cpp \
    target/release/libhypertrie.a \
    <flags from above>
```

## Test

### Rust tests (unit + integration)

```bash
cargo test
```

Expected output:

```
running 8 tests
test tests::duplicate_insert_idempotent ... ok
test tests::ffi_roundtrip ... ok
test tests::insert_and_search_found ... ok
test tests::len_tracks_unique_inserts ... ok
test tests::multiple_triples_all_found ... ok
test tests::partial_prefix_not_marked_end ... ok
test tests::search_missing_triple_returns_false ... ok
test tests::zero_triple_is_valid ... ok

running 5 tests
test concurrent_inserts_all_visible ... ok
test concurrent_read_write_no_corruption ... ok
test concurrent_reads_consistent ... ok
test ffi_concurrent_inserts_via_arc ... ok
test ffi_null_free_is_safe ... ok
```

### C++ tests

```bash
./cpp/main
```

Expected output:

```
[PASS] basic correctness
[PASS] concurrent inserts (8 threads x 100)
[PASS] null free is safe

All C++ tests passed.
```

## Rust API

```rust
use hypertrie::HyperTrie;

let trie = HyperTrie::new();

trie.insert([1, 2, 3]);          // insert triple (subject=1, predicate=2, object=3)
trie.search([1, 2, 3]);          // → true
trie.search([1, 2, 9]);          // → false
trie.len();                      // → 1 (unique triples stored)
trie.is_empty();                 // → false
```

`HyperTrie` is `Send + Sync`, wrap in `Arc` to share across threads:

```rust
use std::sync::Arc;

let trie = Arc::new(HyperTrie::new());
let trie2 = Arc::clone(&trie);

std::thread::spawn(move || {
    trie2.insert([4, 5, 6]);
});
```

## C++ API

```cpp
#include "include/hypertrie.h"
#include <memory>

using namespace hypertrie;

// RAII: frees automatically on scope exit
auto trie = std::unique_ptr<HyperTrie, decltype(&hypertrie_free)>(
    hypertrie_new(), hypertrie_free
);

hypertrie_insert(trie.get(), 1, 2, 3);
hypertrie_search(trie.get(), 1, 2, 3); // → true
hypertrie_search(trie.get(), 9, 9, 9); // → false

hypertrie_free(nullptr); // safe, null is a no-op
```

### Functions

| Function           | Signature                                   | Description              |
|--------------------|---------------------------------------------|--------------------------|
| `hypertrie_new`    | `() → HyperTrie*`                           | Allocate a new trie      |
| `hypertrie_free`   | `(HyperTrie*) → void`                       | Free the trie. Null-safe |
| `hypertrie_insert` | `(const HyperTrie*, u32, u32, u32) → void`  | Insert a triple          |
| `hypertrie_search` | `(const HyperTrie*, u32, u32, u32) → bool`  | Search for a triple      |

> **Ownership:** the caller owns the pointer returned by `hypertrie_new` and must call `hypertrie_free` exactly once. Use `unique_ptr` with a custom deleter (see above) to enforce this automatically.

## Background

### What is a HyperTrie?

A hypertrie is a trie variant designed for storing RDF triples. Each path through the trie represents one element position in the triple:

```
root
└── subject=1
    └── predicate=2
        └── object=3  [is_end=true]
        └── object=7  [is_end=true]
└── subject=4
    └── predicate=5
        └── object=6  [is_end=true]
```

A node at the object level with `is_end=true` means a complete triple was stored there. Nodes that exist only as intermediate steps on the path to another triple have `is_end=false`.

### RDF context

In RDF, resources are identified by IRIs. This implementation uses `u32` IDs, callers are expected to maintain a separate IRI-to-ID mapping and pass numeric IDs into the trie.

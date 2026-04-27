use hypertrie::{HyperTrie, hypertrie_free, hypertrie_search};
use std::sync::Arc;
use std::thread;

// Concurrent inserts
//
#[test]
fn concurrent_inserts_all_visible() {
    let trie = Arc::new(HyperTrie::new());
    let mut handles = vec![];

    for i in 0..8u32 {
        let trie: Arc<HyperTrie> = Arc::clone(&trie);
        handles.push(thread::spawn(move || {
            for j in 0..100u32 {
                trie.insert([i, j, i + j]);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    for i in 0..8u32 {
        for j in 0..100u32 {
            assert!(
                trie.search([i, j, i + j]),
                "missing triple [{i}, {j}, {}]",
                i + j
            );
        }
    }
}

// Concurrent reads
//
#[test]
fn concurrent_reads_consistent() {
    let trie = Arc::new(HyperTrie::new());

    for i in 0..100u32 {
        trie.insert([i, i * 2, i * 3]);
    }

    let mut handles = vec![];

    for _ in 0..8 {
        let trie: Arc<HyperTrie> = Arc::clone(&trie);
        handles.push(thread::spawn(move || {
            for i in 0..100u32 {
                assert!(trie.search([i, i * 2, i * 3]));
                assert!(!trie.search([i, i * 2, i * 3 + 1]));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// Concurrent read + write (no panic / corruption)
//
#[test]
fn concurrent_read_write_no_corruption() {
    let trie = Arc::new(HyperTrie::new());
    let mut handles = vec![];

    // Writers
    for i in 0..4u32 {
        let trie: Arc<HyperTrie> = Arc::clone(&trie);
        handles.push(thread::spawn(move || {
            for j in 0..50u32 {
                trie.insert([i, j, 0]);
            }
        }));
    }

    // Readers — may or may not find, must not panic or corrupt
    for i in 0..4u32 {
        let trie: Arc<HyperTrie> = Arc::clone(&trie);
        handles.push(thread::spawn(move || {
            for j in 0..50u32 {
                let _ = trie.search([i, j, 0]);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// FFI safety
//
#[test]
fn ffi_null_free_is_safe() {
    unsafe {
        hypertrie_free(std::ptr::null_mut());
    }
}

#[test]
fn ffi_concurrent_inserts_via_arc() {
    let trie = Arc::new(HyperTrie::new());
    let mut handles = vec![];

    for i in 0..4u32 {
        let trie: Arc<HyperTrie> = Arc::clone(&trie);
        handles.push(thread::spawn(move || {
            for j in 0..25u32 {
                trie.insert([i, j, i * j]);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    unsafe {
        let raw: *const HyperTrie = Arc::as_ptr(&trie);
        for i in 0..4u32 {
            for j in 0..25u32 {
                assert!(hypertrie_search(raw, i, j, i * j));
            }
        }
    }
}

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Core
//
struct Node {
    children: DashMap<u32, Arc<Node>>,
    is_end: AtomicBool,
}

impl Node {
    fn new() -> Self {
        Self {
            children: DashMap::with_capacity_and_shard_amount(0, 4),
            is_end: AtomicBool::new(false),
        }
    }
}

pub struct HyperTrie {
    root: Arc<Node>,
    count: Arc<AtomicU64>,
}

impl HyperTrie {
    pub fn new() -> Self {
        Self {
            root: Arc::new(Node::new()),
            count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn insert(&self, triple: [u32; 3]) {
        let mut current = Arc::clone(&self.root);

        for resource in triple {
            let next = {
                let entry = current
                    .children
                    .entry(resource)
                    .or_insert_with(|| Arc::new(Node::new()));
                Arc::clone(entry.value())
            };
            current = next;
        }

        let was_end = current.is_end.swap(true, Ordering::Release);
        if !was_end {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn search(&self, triple: [u32; 3]) -> bool {
        let mut current = Arc::clone(&self.root);

        for resource in triple {
            let next = match current.children.get(&resource) {
                Some(child) => Arc::clone(child.value()),
                None => return false,
            };
            current = next;
        }

        current.is_end.load(Ordering::Acquire)
    }

    pub fn len(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for HyperTrie {
    fn default() -> Self {
        Self::new()
    }
}

// FFI
//
#[unsafe(no_mangle)]
pub extern "C" fn hypertrie_new() -> *mut HyperTrie {
    Box::into_raw(Box::new(HyperTrie::new()))
}

/// # Safety
/// `ptr` must be a valid pointer returned by `hypertrie_new` and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hypertrie_free(ptr: *mut HyperTrie) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// # Safety
/// `ptr` must be a valid pointer returned by `hypertrie_new` and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hypertrie_insert(ptr: *const HyperTrie, s: u32, p: u32, o: u32) {
    unsafe { &*ptr }.insert([s, p, o]);
}

/// # Safety
/// `ptr` must be a valid pointer returned by `hypertrie_new` and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hypertrie_search(ptr: *const HyperTrie, s: u32, p: u32, o: u32) -> bool {
    unsafe { &*ptr }.search([s, p, o])
}

// Unit tests
//
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_search_found() {
        let trie = HyperTrie::new();
        trie.insert([1, 2, 3]);
        assert!(trie.search([1, 2, 3]));
    }

    #[test]
    fn search_missing_triple_returns_false() {
        let trie = HyperTrie::new();
        trie.insert([1, 2, 3]);
        assert!(!trie.search([1, 2, 4]));
        assert!(!trie.search([9, 9, 9]));
    }

    #[test]
    fn partial_prefix_not_marked_end() {
        let trie = HyperTrie::new();
        trie.insert([1, 2, 3]);
        assert!(!trie.search([1, 2, 0]));
        assert!(!trie.search([1, 0, 3]));
    }

    #[test]
    fn multiple_triples_all_found() {
        let trie = HyperTrie::new();
        trie.insert([1, 2, 3]);
        trie.insert([4, 5, 6]);
        trie.insert([1, 2, 7]);
        assert!(trie.search([1, 2, 3]));
        assert!(trie.search([4, 5, 6]));
        assert!(trie.search([1, 2, 7]));
        assert!(!trie.search([1, 2, 99]));
    }

    #[test]
    fn duplicate_insert_idempotent() {
        let trie = HyperTrie::new();
        trie.insert([1, 2, 3]);
        trie.insert([1, 2, 3]);
        assert!(trie.search([1, 2, 3]));
    }

    #[test]
    fn zero_triple_is_valid() {
        let trie = HyperTrie::new();
        assert!(!trie.search([0, 0, 0]));
        trie.insert([0, 0, 0]);
        assert!(trie.search([0, 0, 0]));
    }

    #[test]
    fn len_tracks_unique_inserts() {
        let trie = HyperTrie::new();
        assert_eq!(trie.len(), 0);
        assert!(trie.is_empty());
        trie.insert([1, 2, 3]);
        assert_eq!(trie.len(), 1);
        trie.insert([1, 2, 3]); // duplicate — should not increment
        assert_eq!(trie.len(), 1);
        trie.insert([4, 5, 6]);
        assert_eq!(trie.len(), 2);
        assert!(!trie.is_empty());
    }

    #[test]
    fn ffi_roundtrip() {
        unsafe {
            let trie = hypertrie_new();
            hypertrie_insert(trie, 1, 2, 3);
            hypertrie_insert(trie, 4, 5, 6);
            assert!(hypertrie_search(trie, 1, 2, 3));
            assert!(hypertrie_search(trie, 4, 5, 6));
            assert!(!hypertrie_search(trie, 1, 2, 9));
            hypertrie_free(trie);
        }
    }
}

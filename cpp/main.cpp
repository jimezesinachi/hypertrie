#include "../include/hypertrie.h"

#include <cassert>
#include <cstdint>
#include <iostream>
#include <memory>
#include <thread>
#include <vector>

using namespace hypertrie;

// Basic correctness check
//
void test_basic() {
    auto trie = std::unique_ptr<HyperTrie, decltype(&hypertrie_free)>(
        hypertrie_new(), hypertrie_free
    );

    hypertrie_insert(trie.get(), 1, 2, 3);
    hypertrie_insert(trie.get(), 4, 5, 6);
    hypertrie_insert(trie.get(), 1, 2, 7); // shared prefix

    assert(hypertrie_search(trie.get(), 1, 2, 3)  && "should find (1,2,3)");
    assert(hypertrie_search(trie.get(), 4, 5, 6)  && "should find (4,5,6)");
    assert(hypertrie_search(trie.get(), 1, 2, 7)  && "should find (1,2,7)");
    assert(!hypertrie_search(trie.get(), 1, 2, 9) && "should not find (1,2,9)");
    assert(!hypertrie_search(trie.get(), 9, 9, 9) && "should not find (9,9,9)");

    std::cout << "[PASS] basic correctness\n";
}

// Concurrent inserts from C++ threads
//
void test_concurrent() {
    auto trie = std::unique_ptr<HyperTrie, decltype(&hypertrie_free)>(
        hypertrie_new(), hypertrie_free
    );

    constexpr int THREADS = 8;
    constexpr int PER_THREAD = 100;

    std::vector<std::thread> threads;
    threads.reserve(THREADS);

    for (uint32_t i = 0; i < THREADS; ++i) {
        threads.emplace_back([raw = trie.get(), i]() {
            for (uint32_t j = 0; j < PER_THREAD; ++j) {
                hypertrie_insert(raw, i, j, i + j);
            }
        });
    }

    for (auto& t : threads) t.join();

    for (uint32_t i = 0; i < THREADS; ++i) {
        for (uint32_t j = 0; j < PER_THREAD; ++j) {
            assert(hypertrie_search(trie.get(), i, j, i + j) && "concurrent insert lost");
        }
    }

    std::cout << "[PASS] concurrent inserts (" << THREADS << " threads x " << PER_THREAD << ")\n";
}

// Null free is safe
//
void test_null_free() {
    hypertrie_free(nullptr);
    std::cout << "[PASS] null free is safe\n";
}

// Entry point
//
int main() {
    test_basic();
    test_concurrent();
    test_null_free();
    std::cout << "\nAll C++ tests passed.\n";
    return 0;
}

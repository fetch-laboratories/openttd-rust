/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file rust_vector.h A Rust-owned vector viewed in place by native readers. */
#ifndef RUST_RUST_VECTOR_H
#define RUST_RUST_VECTOR_H
#include <cstddef>
#include <span>
/** Rust allocates and frees the items; native tables build borrowed views with zero capacity. */
template <typename T>
struct RustVector {
	const T *items;
	size_t count;
	size_t capacity;
	size_t size() const { return this->count; }
	bool empty() const { return this->count == 0; }
	const T *begin() const { return this->items; }
	const T *end() const { return this->items + this->count; }
	const T &operator[](size_t index) const { return this->items[index]; }
	operator std::span<const T>() const { return {this->items, this->count}; }
};
#endif /* RUST_RUST_VECTOR_H */

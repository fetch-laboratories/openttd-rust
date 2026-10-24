/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry_adapter.hpp Nonowning C++ views of Rust industry allocations. */
#ifndef RUST_INDUSTRY_ADAPTER_HPP
#define RUST_INDUSTRY_ADAPTER_HPP
#include "industry_ffi.h"
#include <memory>
#include <iterator>
using RustIndustryOwner = std::unique_ptr<OpenTTDIndustry, decltype(&openttd_rust_industry_destroy)>;
using RustIndustryBuilderOwner = std::unique_ptr<OpenTTDIndustryBuilder, decltype(&openttd_rust_industry_builder_destroy)>;
/** Optional history is allocated and freed by the Rust accepted slot. */
template <typename T> struct RustIndustryHistory {
	T *pointer = nullptr;

	T *get() const { return this->pointer; }
	T *operator->() const { return this->pointer; }
	T &operator*() const { return *this->pointer; }
	bool operator==(std::nullptr_t) const { return this->pointer == nullptr; }
};
/** Views never own or destroy slot records. Mutations may invalidate all iterators. */
template <typename T, bool Produced> class RustIndustryVector {
	OpenTTDIndustry *owner;
	/* Produced selects the entry at compile time; each call is one typed entry. */
	OpenTTDIndustrySlots View() const
	{
		if constexpr (Produced) return openttd_rust_industry_produced_view(this->owner);
		else return openttd_rust_industry_accepted_view(this->owner);
	}
public:
	using value_type = T;
	using iterator = T *;
	using const_iterator = const T *;
	explicit RustIndustryVector(OpenTTDIndustry *owner) : owner(owner) {}
	T *data() { return static_cast<T *>(this->View().data); }
	const T *data() const { return static_cast<const T *>(this->View().data); }
	size_t size() const { return this->View().size; }
	bool empty() const { return this->size() == 0; }
	iterator begin() { return this->data(); }
	const_iterator begin() const { return this->data(); }
	iterator end() { auto v = this->View(); return static_cast<T *>(v.data) + v.size; }
	const_iterator end() const { auto v = this->View(); return static_cast<const T *>(v.data) + v.size; }
	auto rbegin() { return std::reverse_iterator(this->end()); }
	auto rend() { return std::reverse_iterator(this->begin()); }
	T &operator[](size_t n) { return this->data()[n]; }
	const T &operator[](size_t n) const { return this->data()[n]; }
	T &front() { return *this->begin(); }
	const T &front() const { return *this->begin(); }
	T &back() { return this->data()[this->size() - 1]; }
	const T &back() const { return this->data()[this->size() - 1]; }
	void reserve(size_t n)
	{
		if constexpr (Produced) openttd_rust_industry_produced_reserve(this->owner, n);
		else openttd_rust_industry_accepted_reserve(this->owner, n);
	}
	void resize(size_t n)
	{
		if constexpr (Produced) openttd_rust_industry_produced_resize(this->owner, n);
		else openttd_rust_industry_accepted_resize(this->owner, n);
	}
	T &emplace_back()
	{
		OpenTTDIndustrySlots v;
		if constexpr (Produced) v = openttd_rust_industry_produced_emplace_back(this->owner);
		else v = openttd_rust_industry_accepted_emplace_back(this->owner);
		return static_cast<T *>(v.data)[v.size - 1];
	}
	void push_back(const T &value) { this->emplace_back() = value; }
	void push_back(T &&value) { this->emplace_back() = std::move(value); }
	void clear() { this->resize(0); }
	void shrink_to_fit()
	{
		if constexpr (Produced) openttd_rust_industry_produced_shrink_to_fit(this->owner);
		else openttd_rust_industry_accepted_shrink_to_fit(this->owner);
	}
	iterator erase(iterator first, [[maybe_unused]] iterator last) { auto n = first - this->begin(); assert(last == this->end()); this->resize(n); return this->end(); }
};
OpenTTDIndustryBuilder *RustIndustryBuilderState() noexcept;
/** Nonowning facade for the canonical ascending IndustryID registry. */
struct RustIndustryRegistry {
	uint8_t type = 0;
	OpenTTDIndustrySlots View() const { return openttd_rust_industry_registry_view(RustIndustryBuilderState(), this->type); }
	struct Iterator {
		const uint16_t *pointer;
		IndustryID operator*() const { return IndustryID(*this->pointer); }
		Iterator &operator++() { ++this->pointer; return *this; }
		bool operator==(const Iterator &) const = default;
	};
	Iterator begin() const { return {static_cast<const uint16_t *>(this->View().data)}; }
	Iterator end() const { auto view = this->View(); return {static_cast<const uint16_t *>(view.data) + view.size}; }
	size_t size() const { return this->View().size; }
	void insert(IndustryID id) { openttd_rust_industry_registry_insert(RustIndustryBuilderState(), this->type, id.base()); }
	void erase(IndustryID id) { openttd_rust_industry_registry_erase(RustIndustryBuilderState(), this->type, id.base()); }
	void clear() { openttd_rust_industry_registry_clear(RustIndustryBuilderState(), this->type); }
};
#endif

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_owner.hpp Noncopyable Rust town owner and thin field access facade. */
#ifndef RUST_TOWN_OWNER_HPP
#define RUST_TOWN_OWNER_HPP
#include "town_ffi.h"
#include <memory>

/** Native facade over canonical arbitrary name bytes. The view lasts until its
 * owner is renamed or destroyed; the game-state mutex serializes every access. */
struct RustTownName {
	OpenTTDTownState *owner;
	std::string &presentation;
	RustTownName(OpenTTDTownState *owner, std::string &presentation) : owner(owner), presentation(presentation) {}
	std::string_view View() const { auto b = openttd_rust_town_name(this->owner); return {reinterpret_cast<const char *>(b.data), b.size}; }
	bool empty() const { return this->View().empty(); }
	void clear() { openttd_rust_town_set_name(this->owner, {nullptr, 0}); this->presentation.clear(); }
	RustTownName &operator=(std::string_view text) { openttd_rust_town_set_name(this->owner, {reinterpret_cast<const uint8_t *>(text.data()), text.size()}); this->presentation = this->View(); return *this; }
	RustTownName &operator=(const RustTownName &other) { return *this = other.View(); }
	bool operator==(std::string_view text) const { return this->View() == text; }
};

struct RustTownDelete {
	void operator()(OpenTTDTownState *state) const { openttd_rust_town_destroy(state); }
};
using RustTownOwner = std::unique_ptr<OpenTTDTownState, RustTownDelete>;

/** Nonowning native views of the sole Rust supplied-history vector. Native
 * trivial slot lifetimes start on insertion; Rust alone allocates and frees. */
template <typename T> class RustTownSupplied {
	OpenTTDTownState *owner;
public:
	using value_type = T;
	using iterator = T *;
	using const_iterator = const T *;
	explicit RustTownSupplied(OpenTTDTownState *owner) : owner(owner) {}
	T *data() { return static_cast<T *>(openttd_rust_town_supplied_view(this->owner).data); }
	const T *data() const { return static_cast<const T *>(openttd_rust_town_supplied_view(this->owner).data); }
	size_t size() const { return openttd_rust_town_supplied_view(this->owner).size; }
	bool empty() const { return this->size() == 0; }
	iterator begin() { return this->data(); }
	const_iterator begin() const { return this->data(); }
	iterator end() { const auto v = openttd_rust_town_supplied_view(this->owner); return static_cast<T *>(v.data) + v.size; }
	const_iterator end() const { const auto v = openttd_rust_town_supplied_view(this->owner); return static_cast<const T *>(v.data) + v.size; }
	void reserve(size_t count) { openttd_rust_town_supplied_reserve(this->owner, count); }
	iterator emplace(iterator at, uint8_t cargo)
	{
		const size_t offset = this->empty() ? 0 : at - this->begin();
		auto *slot = reinterpret_cast<T *>(openttd_rust_town_supplied_insert(this->owner, offset, cargo));
		return new (slot) T(cargo);
	}
	T &emplace_back(uint8_t cargo = UINT8_MAX) { return *this->emplace(this->end(), cargo); }
};
/** Compile-time-selected nonowning views preserve native counter widths. */
template <typename T, bool Class> class RustTownCountsVector {
	OpenTTDTownState *owner;
	OpenTTDTownSlots View() const
	{
		if constexpr (Class) return openttd_rust_town_class_counts(this->owner);
		else return openttd_rust_town_id_counts(this->owner);
	}
public:
	using value_type = T;
	explicit RustTownCountsVector(OpenTTDTownState *owner = nullptr) : owner(owner) {}
	T *data() { return static_cast<T *>(this->View().data); }
	const T *data() const { return static_cast<const T *>(this->View().data); }
	size_t size() const { return this->View().size; }
	T *begin() { return this->data(); }
	const T *begin() const { return this->data(); }
	T *end() { const auto v = this->View(); return static_cast<T *>(v.data) + v.size; }
	const T *end() const { const auto v = this->View(); return static_cast<const T *>(v.data) + v.size; }
	T &operator[](size_t at) { return this->data()[at]; }
	const T &operator[](size_t at) const { return this->data()[at]; }

};

template <typename T> struct RustTownBuildingCounts {
	RustTownCountsVector<T, false> id_count;
	RustTownCountsVector<T, true> class_count;
	explicit RustTownBuildingCounts(OpenTTDTownState *owner = nullptr) : id_count(owner), class_count(owner) {}
};
#endif /* RUST_TOWN_OWNER_HPP */

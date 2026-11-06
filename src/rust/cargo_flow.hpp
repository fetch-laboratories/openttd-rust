/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_flow.hpp Native borrowing facades for canonical Rust flows. */
#ifndef RUST_CARGO_FLOW_HPP
#define RUST_CARGO_FLOW_HPP
#include "cargo_flow_ffi.h"
#include <iterator>
#include <optional>

/** One owner, or a scoped borrowed facade over a stable map entry. */
class FlowStat {
	OpenTTDCargoFlow *state;
	bool owned;
	static uint32_t Draw() noexcept { return Random(); }

public:
	/** Reads canonical cumulative shares, retaining only iterator keys. */
	class SharesView {
		friend class FlowStat;
		OpenTTDCargoFlow *state;
		explicit SharesView(OpenTTDCargoFlow *state) : state(state) {}
	public:
		class const_iterator {
			friend class SharesView;
			OpenTTDCargoFlow *state = nullptr;
			uint64_t key = UINT64_MAX;
			mutable std::pair<uint32_t, StationID> value{};
			const_iterator(OpenTTDCargoFlow *state, OpenTTDCargoShare share) : state(state), key(share.found ? share.cumulative : UINT64_MAX) {}
		public:
			using value_type = std::pair<uint32_t, StationID>;
			using difference_type = std::ptrdiff_t;
			using pointer = const value_type *;
			using reference = const value_type &;
			using iterator_category = std::bidirectional_iterator_tag;
			const_iterator() = default;
			reference operator*() const
			{
				auto share = openttd_rust_flow_share(this->state, this->key, 3);
				this->value = {share.cumulative, StationID(share.station)};
				return this->value;
			}
			pointer operator->() const { return &**this; }
			const_iterator &operator++() { auto share = openttd_rust_flow_share(this->state, this->key, 1); this->key = share.found ? share.cumulative : UINT64_MAX; return *this; }
			const_iterator operator++(int) { auto old = *this; ++*this; return old; }
			const_iterator &operator--() { auto share = openttd_rust_flow_share(this->state, this->key, 2); this->key = share.found ? share.cumulative : UINT64_MAX; return *this; }
			const_iterator operator--(int) { auto old = *this; --*this; return old; }
			bool operator==(const const_iterator &other) const { return this->state == other.state && this->key == other.key; }
		};
		const_iterator begin() const { return {this->state, openttd_rust_flow_share(this->state, 0, 0)}; }
		const_iterator end() const { return {this->state, {0, 0, 0}}; }
		size_t size() const { return openttd_rust_flow_read(this->state, 2, 0); }
		bool empty() const { return this->size() == 0; }
	};
	using SharesMap = std::map<uint32_t, StationID>; ///< Call-local job import only.
private:
	SharesView view;
	FlowStat(OpenTTDCargoFlow *state, bool owned) : state(state), owned(owned), view(state) {}
public:
	FlowStat() : FlowStat(nullptr, false) { NOT_REACHED(); }
	FlowStat(StationID st, uint flow, bool restricted = false) : FlowStat(openttd_rust_flow_new(st.base(), flow, restricted), true) {}
	FlowStat(SharesMap shares, uint unrestricted) : FlowStat(StationID::Invalid(), 1)
	{
		openttd_rust_flow_change(this->state, 7, 0, unrestricted);
		for (const auto &[key, station] : shares) openttd_rust_flow_change(this->state, 8, station.base(), key);
	}
	FlowStat(const FlowStat &other) : FlowStat(openttd_rust_flow_clone(other.state), true) {}
	FlowStat(FlowStat &&other) noexcept : FlowStat(other.state, other.owned) { other.owned = false; }
	FlowStat &operator=(const FlowStat &other)
	{
		if (this != &other) { FlowStat copy(other); this->SwapShares(copy); }
		return *this;
	}
	~FlowStat() { if (this->owned) openttd_rust_flow_destroy(this->state); }
	static FlowStat Borrow(OpenTTDCargoFlow *state) { return {state, false}; }
	OpenTTDCargoFlow *RustState() const { return this->state; }
	void AppendShare(StationID st, uint flow, bool restricted = false) { openttd_rust_flow_change(this->state, restricted ? 6 : 5, st.base(), flow); }
	uint GetShare(StationID st) const { return openttd_rust_flow_read(this->state, 0, st.base()); }
	uint GetUnrestricted() const { return openttd_rust_flow_read(this->state, 1, 0); }
	const SharesView *GetShares() const { return &this->view; }
	void ChangeShare(StationID st, int flow) { openttd_rust_flow_change(this->state, 0, st.base(), static_cast<uint32_t>(flow)); }
	void RestrictShare(StationID st) { openttd_rust_flow_change(this->state, 1, st.base(), 0); }
	void ReleaseShare(StationID st) { openttd_rust_flow_change(this->state, 2, st.base(), 0); }
	void ScaleToMonthly(uint runtime) { openttd_rust_flow_change(this->state, 3, 0, runtime); }
	void Invalidate() { openttd_rust_flow_change(this->state, 4, 0, 0); }
	void SwapShares(FlowStat &other) { openttd_rust_flow_swap(this->state, other.state); }
	StationID GetViaWithRestricted(bool &is_restricted) const
	{
		uint8_t restricted = 0;
		auto station = openttd_rust_flow_via(this->state, 1, 0, 0, &restricted, Draw);
		is_restricted = restricted != 0;
		return StationID(station);
	}
	StationID GetVia() const { uint8_t unused = 0; return StationID(openttd_rust_flow_via(this->state, 0, 0, 0, &unused, Draw)); }
	StationID GetVia(StationID excluded, StationID excluded2 = StationID::Invalid()) const
	{
		uint8_t unused = 0;
		return StationID(openttd_rust_flow_via(this->state, 2, excluded.base(), excluded2.base(), &unused, Draw));
	}
};

/** Rust owns origin ordering, entries and their stable flow allocations. */
class FlowStatMap {
	OpenTTDCargoFlowMap *state;
	bool owned = true;
	explicit FlowStatMap(OpenTTDCargoFlowMap *state) : state(state), owned(false) {}
public:
	class iterator {
		friend class FlowStatMap;
		OpenTTDCargoFlowMap *state = nullptr;
		uint32_t key = UINT32_MAX;
		mutable std::optional<std::pair<StationID, FlowStat>> value;
		iterator(OpenTTDCargoFlowMap *state, OpenTTDCargoOrigin origin) : state(state), key(origin.found ? origin.origin : UINT32_MAX) {}
	public:
		using value_type = std::pair<StationID, FlowStat>;
		using difference_type = std::ptrdiff_t;
		using pointer = value_type *;
		using reference = value_type &;
		using iterator_category = std::forward_iterator_tag;
		iterator() = default;
		iterator(const iterator &other) : state(other.state), key(other.key) {}
		iterator &operator=(const iterator &other) { this->state = other.state; this->key = other.key; this->value.reset(); return *this; }
		reference operator*() const
		{
			if (!this->value) {
				auto origin = openttd_rust_flow_map_at(this->state, this->key, 1);
				this->value.emplace(StationID(origin.origin), FlowStat::Borrow(origin.flow));
			}
			return *this->value;
		}
		pointer operator->() const { return &**this; }
		iterator &operator++() { auto next = openttd_rust_flow_map_at(this->state, this->key, 2); this->key = next.found ? next.origin : UINT32_MAX; this->value.reset(); return *this; }
		iterator operator++(int) { auto old = *this; ++*this; return old; }
		bool operator==(const iterator &other) const { return this->state == other.state && this->key == other.key; }
	};
	using const_iterator = iterator;
	/** A call-scoped view, invalidated by its station goods attachment's clear. */
	static FlowStatMap Borrow(OpenTTDCargoFlowMap *state) { return FlowStatMap(state); }
	FlowStatMap() : state(openttd_rust_flow_map_new()) {}
	FlowStatMap(const FlowStatMap &other) : state(openttd_rust_flow_map_clone(other.state)) {}
	FlowStatMap(FlowStatMap &&other) noexcept : state(other.state), owned(other.owned) { other.state = nullptr; other.owned = false; }
	FlowStatMap &operator=(const FlowStatMap &other) { if (this != &other) { FlowStatMap copy(other); std::swap(this->state, copy.state); std::swap(this->owned, copy.owned); } return *this; }
	FlowStatMap &operator=(FlowStatMap &&other) noexcept { std::swap(this->state, other.state); std::swap(this->owned, other.owned); return *this; }
	~FlowStatMap() { if (this->owned && this->state != nullptr) openttd_rust_flow_map_destroy(this->state); }
	OpenTTDCargoFlowMap *RustState() const { return this->state; }
	iterator begin() const { return {this->state, openttd_rust_flow_map_at(this->state, 0, 0)}; }
	iterator end() const { return {this->state, {nullptr, 0, 0}}; }
	iterator find(StationID origin) const { return {this->state, openttd_rust_flow_map_at(this->state, origin.base(), 1)}; }
	size_t size() const { return openttd_rust_flow_map_read(this->state, 0, 0, 0); }
	bool empty() const { return this->size() == 0; }
	void clear() { openttd_rust_flow_map_change(this->state, 5, 0, 0, 0); }
	void erase(StationID origin) { openttd_rust_flow_map_erase(this->state, origin.base()); }
	iterator erase(iterator position) { auto next = position; ++next; this->erase(position->first); return next; }
	std::pair<iterator, bool> emplace(StationID origin, const FlowStat &flow)
	{
		bool inserted = openttd_rust_flow_map_insert(this->state, origin.base(), flow.RustState()) != 0;
		return {this->find(origin), inserted};
	}
	void insert(iterator first, iterator last) { for (; first != last; ++first) this->emplace(first->first, first->second); }
	uint GetFlow() const { return openttd_rust_flow_map_read(this->state, 1, 0, 0); }
	uint GetFlowVia(StationID via) const { return openttd_rust_flow_map_read(this->state, 2, 0, via.base()); }
	uint GetFlowFrom(StationID from) const { return openttd_rust_flow_map_read(this->state, 3, from.base(), 0); }
	uint GetFlowFromVia(StationID from, StationID via) const { return openttd_rust_flow_map_read(this->state, 4, from.base(), via.base()); }
	void AddFlow(StationID origin, StationID via, uint amount) { openttd_rust_flow_map_change(this->state, 0, origin.base(), via.base(), amount); }
	void PassOnFlow(StationID origin, StationID via, uint amount) { openttd_rust_flow_map_change(this->state, 1, origin.base(), via.base(), amount); }
	void RestrictFlows(StationID via) { openttd_rust_flow_map_change(this->state, 2, 0, via.base(), 0); }
	void ReleaseFlows(StationID via) { openttd_rust_flow_map_change(this->state, 3, 0, via.base(), 0); }
	void FinalizeLocalConsumption(StationID self) { openttd_rust_flow_map_change(this->state, 4, self.base(), 0, 0); }
	std::vector<StationID> DeleteFlows(StationID via)
	{
		std::vector<StationID> origins;
		openttd_rust_flow_map_delete(this->state, via.base(), &origins, [](void *ctx, uint16_t origin) noexcept { static_cast<std::vector<StationID> *>(ctx)->emplace_back(origin); });
		return origins;
	}
};
#endif /* RUST_CARGO_FLOW_HPP */

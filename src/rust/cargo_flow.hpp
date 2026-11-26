/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_flow.hpp Native facades over Rust flows and flow maps, for their C++ readers. */
#ifndef RUST_CARGO_FLOW_HPP
#define RUST_CARGO_FLOW_HPP
#include "cargo_flow_ffi.h"
#include <iterator>
#include <optional>

/** One owned flow, or a borrow of a flow map's entry. */
class FlowStat {
	OpenTTDCargoFlow *state;
	bool owned;
	static uint32_t Draw(uint32_t limit) noexcept { return RandomRange(limit); }

public:
	/** Reads a flow's cumulative shares in order. */
	class SharesView {
		friend class FlowStat;
		OpenTTDCargoFlow *state;
		explicit SharesView(OpenTTDCargoFlow *state) : state(state) {}
	public:
		class const_iterator {
			friend class SharesView;
			OpenTTDCargoFlow *state = nullptr;
			OpenTTDCargoShare share{};
			std::pair<uint32_t, StationID> value{};
			const_iterator(OpenTTDCargoFlow *state, OpenTTDCargoShare share) : state(state), share(share), value(share.cumulative, StationID(share.station)) {}
		public:
			using value_type = std::pair<uint32_t, StationID>;
			using difference_type = std::ptrdiff_t;
			using pointer = const value_type *;
			using reference = const value_type &;
			using iterator_category = std::forward_iterator_tag;
			const_iterator() = default;
			reference operator*() const { return this->value; }
			pointer operator->() const { return &this->value; }
			const_iterator &operator++() { *this = {this->state, openttd_rust_flow_shares_next(this->state, this->share.cumulative)}; return *this; }
			const_iterator operator++(int) { auto old = *this; ++*this; return old; }
			bool operator==(const const_iterator &other) const
			{
				return this->state == other.state && this->share.found == other.share.found && (this->share.found == 0 || this->share.cumulative == other.share.cumulative);
			}
		};
		const_iterator begin() const { return {this->state, openttd_rust_flow_shares_begin(this->state)}; }
		const_iterator end() const { return {this->state, {0, 0, 0}}; }
		size_t size() const { return std::distance(this->begin(), this->end()); }
	};
private:
	SharesView view;
	FlowStat(OpenTTDCargoFlow *state, bool owned) : state(state), owned(owned), view(state) {}
public:
	FlowStat(StationID st, uint flow, bool restricted = false) : FlowStat(openttd_rust_flow_new(st.base(), flow, restricted), true) {}
	FlowStat(const FlowStat &other) = delete;
	FlowStat(FlowStat &&other) noexcept : FlowStat(other.state, other.owned) { other.owned = false; }
	FlowStat &operator=(const FlowStat &other) = delete;
	~FlowStat() { if (this->owned) openttd_rust_flow_destroy(this->state); }
	static FlowStat Borrow(OpenTTDCargoFlow *state) { return {state, false}; }
	OpenTTDCargoFlow *RustState() const { return this->state; }
	void AppendShare(StationID st, uint flow, bool restricted = false) { openttd_rust_flow_append_share(this->state, st.base(), flow, restricted); }
	uint GetUnrestricted() const { return openttd_rust_flow_get_unrestricted(this->state); }
	const SharesView *GetShares() const { return &this->view; }
	StationID GetVia() const { return StationID(openttd_rust_flow_get_via(this->state, Draw)); }
};

/** Rust owns origin ordering, entries and their stable flow allocations. */
class FlowStatMap {
	OpenTTDCargoFlowMap *state;
public:
	class iterator {
		friend class FlowStatMap;
		OpenTTDCargoFlowMap *state = nullptr;
		OpenTTDCargoOrigin origin{};
		mutable std::optional<std::pair<StationID, FlowStat>> value;
		iterator(OpenTTDCargoFlowMap *state, OpenTTDCargoOrigin origin) : state(state), origin(origin) {}
	public:
		using value_type = std::pair<StationID, FlowStat>;
		using difference_type = std::ptrdiff_t;
		using pointer = value_type *;
		using reference = value_type &;
		using iterator_category = std::forward_iterator_tag;
		iterator() = default;
		iterator(const iterator &other) : state(other.state), origin(other.origin) {}
		iterator &operator=(const iterator &other) { this->state = other.state; this->origin = other.origin; this->value.reset(); return *this; }
		reference operator*() const
		{
			if (!this->value) this->value.emplace(StationID(this->origin.origin), FlowStat::Borrow(this->origin.flow));
			return *this->value;
		}
		pointer operator->() const { return &**this; }
		iterator &operator++() { this->origin = openttd_rust_flow_map_next(this->state, this->origin.origin); this->value.reset(); return *this; }
		iterator operator++(int) { auto old = *this; ++*this; return old; }
		bool operator==(const iterator &other) const
		{
			return this->state == other.state && this->origin.found == other.origin.found && (this->origin.found == 0 || this->origin.origin == other.origin.origin);
		}
	};
	using const_iterator = iterator;
	FlowStatMap() : state(openttd_rust_flow_map_new()) {}
	FlowStatMap(const FlowStatMap &other) = delete;
	FlowStatMap(FlowStatMap &&other) noexcept : state(other.state) { other.state = nullptr; }
	FlowStatMap &operator=(const FlowStatMap &other) = delete;
	FlowStatMap &operator=(FlowStatMap &&other) noexcept { std::swap(this->state, other.state); return *this; }
	~FlowStatMap() { if (this->state != nullptr) openttd_rust_flow_map_destroy(this->state); }
	OpenTTDCargoFlowMap *RustState() const { return this->state; }
	iterator begin() const { return {this->state, openttd_rust_flow_map_begin(this->state)}; }
	iterator end() const { return {this->state, {nullptr, 0, 0}}; }
	iterator find(StationID origin) const { return {this->state, openttd_rust_flow_map_find(this->state, origin.base())}; }
	bool empty() const { return this->begin() == this->end(); }
	std::pair<iterator, bool> emplace(StationID origin, const FlowStat &flow)
	{
		bool inserted = openttd_rust_flow_map_insert(this->state, origin.base(), flow.RustState()) != 0;
		return {this->find(origin), inserted};
	}
	uint GetFlow() const { return openttd_rust_flow_map_get_flow(this->state); }
	uint GetFlowVia(StationID via) const { return openttd_rust_flow_map_get_flow_via(this->state, via.base()); }
	uint GetFlowFrom(StationID from) const { return openttd_rust_flow_map_get_flow_from(this->state, from.base()); }
	uint GetFlowFromVia(StationID from, StationID via) const { return openttd_rust_flow_map_get_flow_from_via(this->state, from.base(), via.base()); }
};
#endif /* RUST_CARGO_FLOW_HPP */

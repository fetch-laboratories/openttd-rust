/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_queue.hpp Native facade for the canonical Rust loading queue. */
#ifndef RUST_STATION_QUEUE_HPP
#define RUST_STATION_QUEUE_HPP

#include "station_service_ffi.h"
#include <iterator>

struct Vehicle;

/**
 * The station shell borrows its BaseStation's Rust owner. Queue nodes have stable
 * identity until erased, like the original std::list nodes; duplicate vehicle
 * pointers are separate entries.
 */
class RustStationLoadingQueue {
	OpenTTDStationService *state;

public:
	class iterator {
		friend class RustStationLoadingQueue;
		OpenTTDStationService *state = nullptr;
		OpenTTDStationQueueEntry *node = nullptr;

		iterator(OpenTTDStationService *state, OpenTTDStationQueueEntry *node) : state(state), node(node) {}

	public:
		using value_type = Vehicle *;
		using difference_type = std::ptrdiff_t;
		using reference = Vehicle *;
		using pointer = void;
		using iterator_category = std::forward_iterator_tag;

		iterator() = default;
		Vehicle *operator*() const { return static_cast<Vehicle *>(openttd_rust_station_queue_value(this->node)); }
		iterator &operator++()
		{
			this->node = openttd_rust_station_queue_next(this->state, this->node);
			return *this;
		}
		iterator operator++(int)
		{
			iterator previous = *this;
			++*this;
			return previous;
		}
		bool operator==(const iterator &) const = default;
	};

	explicit RustStationLoadingQueue(OpenTTDStationService *state) : state(state) {}
	RustStationLoadingQueue(const RustStationLoadingQueue &) = delete;
	RustStationLoadingQueue &operator=(const RustStationLoadingQueue &) = delete;

	iterator begin() const { return {this->state, openttd_rust_station_queue_first(this->state)}; }
	iterator end() const { return {this->state, nullptr}; }
	size_t size() const { return openttd_rust_station_queue_size(this->state); }
	bool empty() const { return this->size() == 0; }
	Vehicle *front() const { return *this->begin(); }
	void push_back(Vehicle *vehicle) { openttd_rust_station_queue_push(this->state, vehicle); }
	iterator erase(iterator position) { return {this->state, openttd_rust_station_queue_erase_node(this->state, position.node)}; }
	void remove(Vehicle *vehicle) { openttd_rust_station_queue_remove(this->state, vehicle); }
	void clear() { openttd_rust_station_queue_clear(this->state); }
};

#endif /* RUST_STATION_QUEUE_HPP */

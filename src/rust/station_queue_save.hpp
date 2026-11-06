/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_queue_save.hpp Call-scoped staging for native reference-list serialization. */
#ifndef RUST_STATION_QUEUE_SAVE_HPP
#define RUST_STATION_QUEUE_SAVE_HPP

#include "station_queue.hpp"
#include <list>

/**
 * SlObject's reference-list storage operations may throw during ordinary loading
 * and fixup. Publish their partially modified list on both return and unwinding,
 * just as the original station member retained its loaded/fixed prefix. Nested
 * calls restore the previous context. Serialization is already serial in C++.
 */
class RustStationLoadingQueueSaveScope {
	inline static RustStationLoadingQueueSaveScope *current = nullptr;
	RustStationLoadingQueueSaveScope *previous;
	OpenTTDStationRecord *record;
	RustStationLoadingQueue queue;
	std::list<void *> vehicles;
	bool write;
	bool accessed = false;

public:
	RustStationLoadingQueueSaveScope(OpenTTDStationRecord *record, bool write) : previous(current), record(record), queue(record), write(write)
	{
		if (record != nullptr) for (Vehicle *vehicle : this->queue) this->vehicles.push_back(vehicle);
		current = this;
	}
	RustStationLoadingQueueSaveScope(const RustStationLoadingQueueSaveScope &) = delete;
	RustStationLoadingQueueSaveScope &operator=(const RustStationLoadingQueueSaveScope &) = delete;
	~RustStationLoadingQueueSaveScope()
	{
		if (this->write && this->accessed) {
			this->queue.clear();
			for (void *vehicle : this->vehicles) this->queue.push_back(static_cast<Vehicle *>(vehicle));
		}
		current = this->previous;
	}

	static void *Address(OpenTTDStationRecord *record)
	{
		/* Every descriptor access is enclosed by a station SlObject scope. */
		auto *scope = current;
		while (scope->record != record) scope = scope->previous;
		scope->accessed = true;
		return &scope->vehicles;
	}
};

#endif /* RUST_STATION_QUEUE_SAVE_HPP */

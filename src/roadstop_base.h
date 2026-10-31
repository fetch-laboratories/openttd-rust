/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roadstop_base.h Base class for roadstops. */

#ifndef ROADSTOP_BASE_H
#define ROADSTOP_BASE_H

#include "station_type.h"
#include "core/pool_type.hpp"
#include "vehicle_type.h"
#include "rust/road_stop_ffi.h"
#include <memory>

using RoadStopPool = Pool<RoadStop, RoadStopID, 32>;
extern RoadStopPool _roadstop_pool;

/** A Stop for a Road Vehicle */
struct RoadStop : RoadStopPool::PoolItem<&_roadstop_pool> {
	enum class RoadStopStatusFlag : uint8_t {
		Bay0Free  = 0, ///< Non-zero when bay 0 is free
		Bay1Free  = 1, ///< Non-zero when bay 1 is free
		BaseEntry = 6, ///< Non-zero when the entries on this road stop are the primary, i.e. the ones to delete
		EntryBusy = 7, ///< Non-zero when roadstop entry is busy
	};
	using RoadStopStatusFlags = EnumBitSet<RoadStopStatusFlag, uint8_t>;

	/** Drive-through entry point; Rust owns its storage, native readers only load. */
	using Entry = OpenTTDRoadStopEntry;

	/* Rust owns status and the shared entries; the destructor frees the head's entries. */
	std::unique_ptr<OpenTTDRoadStopState, decltype(&openttd_rust_road_stop_destroy)> rust_state{openttd_rust_road_stop_new(), openttd_rust_road_stop_destroy};
	/* Native typed status aliases the Rust-owned byte for the save descriptor;
	 * start its trivial lifetime in the fixed allocation. Rust stores raw bytes. */
	RoadStopStatusFlags &status = *new (std::addressof(this->rust_state->status)) RoadStopStatusFlags{RoadStopStatusFlag::Bay0Free, RoadStopStatusFlag::Bay1Free}; ///< Current status of the Stop.
	TileIndex xy = INVALID_TILE; ///< Position on the map
	RoadStop *next = nullptr; ///< Next stop of the given type at this station

	/** Initializes a RoadStop */
	inline RoadStop(TileIndex tile = INVALID_TILE) : xy(tile) { }

	OpenTTDRoadStopState *GetRustState() const { return this->rust_state.get(); }

	/**
	 * Get the drive through road stop entry struct for the given direction.
	 * @param dir The direction to get the entry for.
	 * @return the entry
	 */
	inline const Entry &GetEntry(DiagDirection dir) const
	{
		return dir >= DIAGDIR_SW ? this->rust_state->entries->west : this->rust_state->entries->east;
	}

	RoadStop *GetNextRoadStop(const struct RoadVehicle *v) const;
};

static_assert(sizeof(RoadStop::RoadStopStatusFlags) == sizeof(uint8_t));
static_assert(alignof(RoadStop::RoadStopStatusFlags) == alignof(uint8_t));
static_assert(std::is_trivially_copyable_v<RoadStop::RoadStopStatusFlags>);

#endif /* ROADSTOP_BASE_H */

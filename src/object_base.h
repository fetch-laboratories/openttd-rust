/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file object_base.h Base for all objects. */

#ifndef OBJECT_BASE_H
#define OBJECT_BASE_H

#include "rust/pool.hpp"
#include "object_type.h"
#include "tilearea_type.h"
#include "town_type.h"
#include "timer/timer_game_calendar.h"
#include "rust/object_ffi.h"

using ObjectPool = RustPool<Object, ObjectID, 64>;
extern ObjectPool _object_pool;

#include "abi-record-object_type.h"
extern "C" {
RustPoolCore *OPENTTD_RUST_CALL openttd_rust_object_pool_bind(RustPoolCore *);
OpenTTDObjectRecord *OPENTTD_RUST_CALL openttd_rust_object_record_new(Object *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_object_record_delete(OpenTTDObjectRecord *);
}

/** Native shell for inline drawing/script/chunk consumers of the Rust record. */
struct Object : ObjectPool::PoolItem<&_object_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDObjectRecord *record) const
		{
			_object_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_object_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDObjectRecord, RecordDeleter> record{openttd_rust_object_record_new(this, this->index.base())};
	OpenTTDObjectRecord &Record() { return *this->record; }
	const OpenTTDObjectRecord &Record() const { return *this->record; }
	Object() { _object_pool.BindRecord(this->index.base(), this, this->record.get()); }

	static Object *GetByTile(TileIndex tile);

	/**
	 * Increment the count of objects for this type.
	 * @param type ObjectType to increment
	 * @pre type < NUM_OBJECTS
	 */
	static inline void IncTypeCount(ObjectType type)
	{
		assert(type < NUM_OBJECTS);
		openttd_rust_object_count_increment(type);
	}

	/**
	 * Decrement the count of objects for this type.
	 * @param type ObjectType to decrement
	 * @pre type < NUM_OBJECTS
	 */
	static inline void DecTypeCount(ObjectType type)
	{
		assert(type < NUM_OBJECTS);
		openttd_rust_object_count_decrement(type);
	}

	/**
	 * Get the count of objects for this type.
	 * @param type ObjectType to query
	 * @pre type < NUM_OBJECTS
	 */
	static inline uint16_t GetTypeCount(ObjectType type)
	{
		assert(type < NUM_OBJECTS);
		return openttd_rust_object_count(type);
	}

	/** Resets object counts. */
	static inline void ResetTypeCounts()
	{
		openttd_rust_object_counts_reset();
	}

};

/** Native scratch clients query the first intersecting area. */
inline bool IsObjectIndirectlyCleared(TileIndex tile) { return openttd_rust_object_indirectly_cleared(tile.base(), *GetRustSharedServices().map->log_x); }

#endif /* OBJECT_BASE_H */

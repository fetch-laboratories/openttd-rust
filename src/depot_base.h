/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file depot_base.h Base for all depots (except hangars) */

#ifndef DEPOT_BASE_H
#define DEPOT_BASE_H

#include "depot_map.h"
#include "rust/pool.hpp"
#include "rust/owned_bytes.h"
#include "timer/timer_game_calendar.h"

#include "abi-record-depot_type.h"
extern "C" {
RustPoolCore *OPENTTD_RUST_CALL openttd_rust_depot_pool_bind(RustPoolCore *);
OpenTTDDepotRecord *OPENTTD_RUST_CALL openttd_rust_depot_record_new(Depot *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_depot_record_delete(OpenTTDDepotRecord *);
}
using DepotPool = RustPool<Depot, DepotID, 64>;
extern DepotPool _depot_pool;

struct Depot : DepotPool::PoolItem<&_depot_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDDepotRecord *record) const
		{
			_depot_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_depot_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDDepotRecord, RecordDeleter> record{openttd_rust_depot_record_new(this, this->index.base())};
	OpenTTDDepotRecord &Record() { return *this->record; }
	const OpenTTDDepotRecord &Record() const { return *this->record; }
	std::string_view Name() const { return RustBytesView(this->record->name); }
	void SetName(std::string_view name) { RustBytesAssign(this->record->name, name); }
	Depot() { _depot_pool.BindRecord(this->index.base(), this, this->record.get()); }
	Depot(TileIndex xy) : Depot() { this->Record().xy = xy; this->Record().build_date = TimerGameCalendar::date; }
	~Depot();

	static inline Depot *GetByTile(TileIndex tile) { return Depot::Get(GetDepotIndex(tile)); }
	inline bool IsOfType(const Depot *d) const { return GetTileType(d->Record().xy) == GetTileType(this->Record().xy); }
};

#endif /* DEPOT_BASE_H */

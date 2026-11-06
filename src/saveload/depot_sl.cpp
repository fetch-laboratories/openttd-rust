/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file depot_sl.cpp Code handling saving and loading of depots */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/depot_sl_compat.h"

#include "../depot_base.h"
#include "../town.h"

#include "../safeguards.h"

static TownID _town_index;

static std::string _name;
struct DepotNameScope {
	Depot *item;
	~DepotNameScope() { this->item->SetName(_name); }
};

static const SaveLoad _depot_desc[] = {
	 SLE_CONDVARNAME(Depot, Record().xy, "xy",         SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION, SLV_6),
	 SLE_CONDVARNAME(Depot, Record().xy, "xy",         SLE_UINT32,                 SLV_6, SL_MAX_VERSION),
	SLEG_CONDVAR("town_index", _town_index, SLE_UINT16,       SL_MIN_VERSION, SLV_141),
	 SLE_GENERAL_NAME(SL_REF, "town", Depot, Record().town, REF_TOWN, 0, SLV_141, SL_MAX_VERSION, 0),
	 SLE_CONDVARNAME(Depot, Record().town_cn, "town_cn",    SLE_UINT16,               SLV_141, SL_MAX_VERSION),
	SLEG_CONDSSTR("name", _name,       SLE_STR,                  SLV_141, SL_MAX_VERSION),
	 SLE_CONDVARNAME(Depot, Record().build_date, "build_date", SLE_INT32,                SLV_142, SL_MAX_VERSION),
};

struct DEPTChunkHandler : ChunkHandler {
	DEPTChunkHandler() : ChunkHandler('DEPT', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_depot_desc);

		for (Depot *depot : Depot::Iterate()) {
			SlSetArrayIndex(depot->index);
			_name = depot->Name();
			SlObject(depot, _depot_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_depot_desc, _depot_sl_compat);

		int index;

		while ((index = SlIterateArray()) != -1) {
			Depot *depot = new (DepotID(index)) Depot();
			_name.clear();
			DepotNameScope staged{depot};
			SlObject(depot, slt);

			/* Set the town 'pointer' so we can restore it later. */
			if (IsSavegameVersionBefore(SLV_141)) depot->Record().town = reinterpret_cast<Town *>(static_cast<size_t>(_town_index.base()));
		}
	}

	void FixPointers() const override
	{
		for (Depot *depot : Depot::Iterate()) {
			SlObject(depot, _depot_desc);
			if (IsSavegameVersionBefore(SLV_141)) depot->Record().town = Town::Get((size_t)depot->Record().town);
		}
	}
};

static const DEPTChunkHandler DEPT;
static const ChunkHandlerRef depot_chunk_handlers[] = {
	DEPT,
};

extern const ChunkHandlerTable _depot_chunk_handlers(depot_chunk_handlers);

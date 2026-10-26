/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file labelmaps_sl.cpp Code handling saving and loading of rail type label mappings */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/labelmaps_sl_compat.h"

#include "saveload_internal.h"
#include "../rail.h"
#include "../road.h"
#include "../newgrf_railtype.h"
#include "../newgrf_roadtype.h"

#ifdef WITH_RUST
#include "../rust/type_registry_ffi.h"
#endif

#include "../safeguards.h"

#ifndef WITH_RUST
extern std::vector<LabelObject<RailTypeLabel>> _railtype_list;
extern std::vector<LabelObject<RoadTypeLabel>> _roadtype_list;
#endif

/** Perform rail type and road type conversion if necessary. */
void AfterLoadLabelMaps()
{
	ConvertRailTypes();
	ConvertRoadTypes();

	SetCurrentRailTypeLabelList();
	SetCurrentRoadTypeLabelList();
}

struct RAILChunkHandler : ChunkHandler {
	RAILChunkHandler() : ChunkHandler('RAIL', CH_TABLE) {}

	static inline const SaveLoad description[] = {
		SLE_VAR(LabelObject<RailTypeLabel>, label, SLE_UINT32),
	};

	void Save() const override
	{
		SlTableHeader(description);

		LabelObject<RailTypeLabel> lo;
		for (RailType r = RAILTYPE_BEGIN; r != RAILTYPE_END; r++) {
			lo.label = GetRailTypeInfo(r)->label;

			SlSetArrayIndex(r);
			SlObject(&lo, description);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(description, _label_object_sl_compat);


#ifdef WITH_RUST
		openttd_rust_rail_saved_reserve();
#else
		_railtype_list.reserve(RAILTYPE_END);
#endif

		LabelObject<RailTypeLabel> lo;

		while (SlIterateArray() != -1) {
			SlObject(&lo, slt);

#ifdef WITH_RUST
		openttd_rust_rail_saved_append(lo.label, 0);
#else
		_railtype_list.push_back(lo);
#endif
		}
	}
};

struct ROTTChunkHandler : ChunkHandler {
	ROTTChunkHandler() : ChunkHandler('ROTT', CH_TABLE) {}

	static inline const SaveLoad description[] = {
		SLE_VAR(LabelObject<RoadTypeLabel>, label, SLE_UINT32),
		SLE_VAR(LabelObject<RoadTypeLabel>, subtype, SLE_UINT8),
	};

	void Save() const override
	{
		SlTableHeader(description);

		LabelObject<RoadTypeLabel> lo;
		for (RoadType r = ROADTYPE_BEGIN; r != ROADTYPE_END; r++) {
			const RoadTypeInfo *rti = GetRoadTypeInfo(r);
			lo.label = rti->label;
			lo.subtype = GetRoadTramType(r);

			SlSetArrayIndex(r);
			SlObject(&lo, description);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(description, _label_object_sl_compat);


#ifdef WITH_RUST
		openttd_rust_road_saved_reserve();
#else
		_roadtype_list.reserve(ROADTYPE_END);
#endif

		LabelObject<RoadTypeLabel> lo;

		while (SlIterateArray() != -1) {
			SlObject(&lo, slt);

#ifdef WITH_RUST
		openttd_rust_road_saved_append(lo.label, lo.subtype);
#else
		_roadtype_list.push_back(lo);
#endif
		}
	}
};

static const RAILChunkHandler RAIL;
static const ROTTChunkHandler ROTT;

static const ChunkHandlerRef labelmaps_chunk_handlers[] = {
	RAIL,
	ROTT,
};

extern const ChunkHandlerTable _labelmaps_chunk_handlers(labelmaps_chunk_handlers);


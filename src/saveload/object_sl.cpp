/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file object_sl.cpp Code handling saving and loading of objects */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/object_sl_compat.h"

#include "../object_base.h"
#include "../object_map.h"
#include "newgrf_sl.h"

#include "../safeguards.h"

static const SaveLoad _object_desc[] = {
	    SLE_VARNAME(Object, Record().location.tile, "location.tile",              SLE_UINT32),
	    SLE_VARNAME(Object, Record().location.w, "location.w",                 SLE_FILE_U8 | SLE_VAR_U16),
	    SLE_VARNAME(Object, Record().location.h, "location.h",                 SLE_FILE_U8 | SLE_VAR_U16),
	    SLE_GENERAL_NAME(SL_REF, "town", Object, Record().town, REF_TOWN, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
	    SLE_VARNAME(Object, Record().build_date, "build_date",                 SLE_UINT32),
	SLE_CONDVARNAME(Object, Record().colour, "colour",                     SLE_UINT8,                  SLV_148, SL_MAX_VERSION),
	SLE_CONDVARNAME(Object, Record().view, "view",                       SLE_UINT8,                  SLV_155, SL_MAX_VERSION),
	SLE_CONDVARNAME(Object, Record().type, "type",                       SLE_UINT16,                 SLV_186, SL_MAX_VERSION),
};

struct OBJSChunkHandler : ChunkHandler {
	OBJSChunkHandler() : ChunkHandler('OBJS', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_object_desc);

		/* Write the objects */
		for (Object *o : Object::Iterate()) {
			SlSetArrayIndex(o->index);
			SlObject(o, _object_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_object_desc, _object_sl_compat);

		int index;
		while ((index = SlIterateArray()) != -1) {
			Object *o = new (ObjectID(index)) Object();
			SlObject(o, slt);
		}
	}

	void FixPointers() const override
	{
		for (Object *o : Object::Iterate()) {
			SlObject(o, _object_desc);
			if (IsSavegameVersionBefore(SLV_148) && !IsTileType(o->Record().location.tile, MP_OBJECT)) {
				/* Due to a small bug stale objects could remain. */
				delete o;
			}
		}
	}
};

struct OBIDChunkHandler : NewGRFMappingChunkHandler {
	OBIDChunkHandler() : NewGRFMappingChunkHandler('OBID', _object_mngr) {}
};

static const OBIDChunkHandler OBID;
static const OBJSChunkHandler OBJS;
static const ChunkHandlerRef object_chunk_handlers[] = {
	OBID,
	OBJS,
};

extern const ChunkHandlerTable _object_chunk_handlers(object_chunk_handlers);

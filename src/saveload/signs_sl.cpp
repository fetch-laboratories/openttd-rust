/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signs_sl.cpp Code handling saving and loading of economy data */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/signs_sl_compat.h"

#include "../signs_base.h"
#include "../fios.h"

#include "../safeguards.h"

/** Description of a sign within the savegame. */
static std::string _name;
struct SignNameScope {
	Sign *item;
	~SignNameScope() { this->item->SetName(_name); }
};

static const SaveLoad _sign_desc[] = {
	SLEG_CONDVAR("name", _name, SLE_NAME, SL_MIN_VERSION, SLV_84),
	SLEG_CONDSSTR("name", _name, SLE_STR | SLF_ALLOW_CONTROL, SLV_84, SL_MAX_VERSION),
	SLE_CONDVARNAME(Sign, Record().x, "x",     SLE_FILE_I16 | SLE_VAR_I32, SL_MIN_VERSION, SLV_5),
	SLE_CONDVARNAME(Sign, Record().y, "y",     SLE_FILE_I16 | SLE_VAR_I32, SL_MIN_VERSION, SLV_5),
	SLE_CONDVARNAME(Sign, Record().x, "x",     SLE_INT32,                  SLV_5, SL_MAX_VERSION),
	SLE_CONDVARNAME(Sign, Record().y, "y",     SLE_INT32,                  SLV_5, SL_MAX_VERSION),
	SLE_CONDVARNAME(Sign, Record().owner, "owner", SLE_UINT8,                  SLV_6, SL_MAX_VERSION),
	SLE_CONDVARNAME(Sign, Record().z, "z",     SLE_FILE_U8  | SLE_VAR_I32, SL_MIN_VERSION, SLV_164),
	SLE_CONDVARNAME(Sign, Record().z, "z",     SLE_INT32,                SLV_164, SL_MAX_VERSION),
};

struct SIGNChunkHandler : ChunkHandler {
	SIGNChunkHandler() : ChunkHandler('SIGN', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_sign_desc);

		for (Sign *si : Sign::Iterate()) {
			SlSetArrayIndex(si->index);
			_name = si->Name();
			SlObject(si, _sign_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_sign_desc, _sign_sl_compat);

		int index;
		while ((index = SlIterateArray()) != -1) {
			Sign *si = new (SignID(index)) Sign();
			_name.clear();
			SignNameScope staged{si};
			SlObject(si, slt);
			/* Before version 6.1, signs didn't have owner.
			 * Before version 83, invalid signs were determined by si->str == 0.
			 * Before version 103, owner could be a bankrupted company.
			 *  - we can't use IsValidCompany() now, so this is fixed in AfterLoadGame()
			 * All signs that were saved are valid (including those with just 'Sign' and INVALID_OWNER).
			 *  - so set owner to OWNER_NONE if needed (signs from pre-version 6.1 would be lost) */
			if (IsSavegameVersionBefore(SLV_6, 1) || (IsSavegameVersionBefore(SLV_83) && si->Record().owner == INVALID_OWNER)) {
				si->Record().owner = OWNER_NONE;
			}

			/* Signs placed in scenario editor shall now be OWNER_DEITY */
			if (IsSavegameVersionBefore(SLV_171) && si->Record().owner == OWNER_NONE && _file_to_saveload.ftype.abstract == FT_SCENARIO) {
				si->Record().owner = OWNER_DEITY;
			}
		}
	}
};

static const SIGNChunkHandler SIGN;
static const ChunkHandlerRef sign_chunk_handlers[] = {
	SIGN,
};

extern const ChunkHandlerTable _sign_chunk_handlers(sign_chunk_handlers);

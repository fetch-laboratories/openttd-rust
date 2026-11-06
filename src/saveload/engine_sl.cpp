/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_sl.cpp Code handling saving and loading of engines */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/engine_sl_compat.h"

#include "saveload_internal.h"
#include "../engine_base.h"
#include "../string_func.h"

#include "../safeguards.h"
/** Name decoding retains its native error frame; scalars address the canonical record. */
static std::string _engine_name;
struct EngineNameScope {
	Engine *engine;
	EngineNameScope(Engine *e) : engine(e) { _engine_name = e->Name(); }
	~EngineNameScope() { this->engine->SetName(_engine_name); }
};

static const SaveLoad _engine_desc[] = {
	 SLE_CONDVAR(OpenTTDEngineLifecycle, intro_date,          SLE_FILE_U16 | SLE_VAR_I32,  SL_MIN_VERSION,  SLV_31),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, intro_date,          SLE_INT32,                  SLV_31, SL_MAX_VERSION),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, age,                 SLE_FILE_U16 | SLE_VAR_I32,  SL_MIN_VERSION,  SLV_31),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, age,                 SLE_INT32,                  SLV_31, SL_MAX_VERSION),
	     SLE_VAR(OpenTTDEngineLifecycle, reliability,         SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, reliability_spd_dec, SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, reliability_start,   SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, reliability_max,     SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, reliability_final,   SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, duration_phase_1,    SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, duration_phase_2,    SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, duration_phase_3,    SLE_UINT16),
	     SLE_VAR(OpenTTDEngineLifecycle, flags,               SLE_UINT8),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, preview_asked,       SLE_UINT16,                SLV_179, SL_MAX_VERSION),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, preview_company,     SLE_UINT8,                 SLV_179, SL_MAX_VERSION),
	     SLE_VAR(OpenTTDEngineLifecycle, preview_wait,        SLE_UINT8),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, company_avail,       SLE_FILE_U8  | SLE_VAR_U16,  SL_MIN_VERSION, SLV_104),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, company_avail,       SLE_UINT16,                SLV_104, SL_MAX_VERSION),
	 SLE_CONDVAR(OpenTTDEngineLifecycle, company_hidden,      SLE_UINT16,                SLV_193, SL_MAX_VERSION),
	SLEG_CONDSSTR("name", _engine_name, SLE_STR, SLV_84, SL_MAX_VERSION),
};

static TypedIndexContainer<std::vector<Engine>, EngineID> _temp_engine;

Engine *GetTempDataEngine(EngineID index)
{
	if (index < _temp_engine.size()) {
		return &_temp_engine[index];
	} else if (index == _temp_engine.size()) {
		return &_temp_engine.emplace_back();
	} else {
		NOT_REACHED();
	}
}

struct ENGNChunkHandler : ChunkHandler {
	ENGNChunkHandler() : ChunkHandler('ENGN', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_engine_desc);

		for (Engine *e : Engine::Iterate()) {
			SlSetArrayIndex(e->index);
			_engine_name = e->Name();
			SlObject(&e->Record(), _engine_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_engine_desc, _engine_sl_compat);

		/* As engine data is loaded before engines are initialized we need to load
		 * this information into a temporary array. This is then copied into the
		 * engine pool after processing NewGRFs by CopyTempEngineData(). */
		int index;
		while ((index = SlIterateArray()) != -1) {
			Engine *e = GetTempDataEngine(static_cast<EngineID>(index));
			{
				EngineNameScope name(e);
				SlObject(&e->Record(), slt);
			}

			if (IsSavegameVersionBefore(SLV_179)) {
				/* preview_company_rank was replaced with preview_company and preview_asked.
				 * Just cancel any previews. */
				e->Record().flags.Reset(EngineFlag{2}); // ENGINE_OFFER_WINDOW_OPEN
				e->Record().preview_company = CompanyID::Invalid();
				e->Record().preview_asked.Set();
			}
		}
	}
};

/**
 * Copy data from temporary engine array into the real engine pool.
 */
void CopyTempEngineData()
{
	for (Engine *e : Engine::Iterate()) {
		if (e->index >= _temp_engine.size()) break;

		const Engine *se = GetTempDataEngine(e->index);
		e->Record().intro_date          = se->Record().intro_date;
		e->Record().age                 = se->Record().age;
		e->Record().reliability         = se->Record().reliability;
		e->Record().reliability_spd_dec = se->Record().reliability_spd_dec;
		e->Record().reliability_start   = se->Record().reliability_start;
		e->Record().reliability_max     = se->Record().reliability_max;
		e->Record().reliability_final   = se->Record().reliability_final;
		e->Record().duration_phase_1    = se->Record().duration_phase_1;
		e->Record().duration_phase_2    = se->Record().duration_phase_2;
		e->Record().duration_phase_3    = se->Record().duration_phase_3;
		e->Record().flags               = se->Record().flags;
		e->Record().preview_asked       = se->Record().preview_asked;
		e->Record().preview_company     = se->Record().preview_company;
		e->Record().preview_wait        = se->Record().preview_wait;
		e->Record().company_avail       = se->Record().company_avail;
		e->Record().company_hidden      = se->Record().company_hidden;
		e->SetName(se->Name());
	}

	ResetTempEngineData();
}

void ResetTempEngineData()
{
	_temp_engine.clear();
	_temp_engine.shrink_to_fit();
}

struct ENGSChunkHandler : ChunkHandler {
	ENGSChunkHandler() : ChunkHandler('ENGS', CH_READONLY) {}

	void Load() const override
	{
		/* Load old separate String ID list into a temporary array. This
		 * was always 256 entries. */
		TypedIndexContainer<std::array<StringID, 256>, EngineID> names{};

		SlCopy(names.data(), std::size(names), SLE_STRINGID);

		/* Copy each string into the temporary engine array. */
		for (EngineID engine = EngineID::Begin(); engine < std::size(names); ++engine) {
			Engine *e = GetTempDataEngine(engine);
			e->SetName(CopyFromOldName(names[engine]));
		}
	}
};

/** Save and load the mapping between the engine id in the pool, and the grf file it came from. */
static const SaveLoad _engine_id_mapping_desc[] = {
	SLE_VAR(EngineIDMapping, grfid,         SLE_UINT32),
	SLE_VAR(EngineIDMapping, internal_id,   SLE_UINT16),
	SLE_VAR(EngineIDMapping, type,          SLE_UINT8),
	SLE_VAR(EngineIDMapping, substitute_id, SLE_UINT8),
};

struct EIDSChunkHandler : ChunkHandler {
	EIDSChunkHandler() : ChunkHandler('EIDS', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_engine_id_mapping_desc);

		/* Count total entries needed for combined list. */
		size_t total = 0;
		for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
			const auto mapping = _engine_mngr.Mappings(type);
			total += std::size(mapping);
		}

		/* Combine per-type mappings into single list for all types. */
		std::vector<EngineIDMapping> temp;
		temp.reserve(total);
		for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
			const auto mapping = _engine_mngr.Mappings(type);
			temp.insert(std::end(temp), std::begin(mapping), std::end(mapping));
		}

		/* Sort combined list by EngineID */
		std::ranges::sort(temp, std::less{}, &EngineIDMapping::engine);

		for (EngineIDMapping &eid : temp) {
			SlSetArrayIndex(eid.engine);
			SlObject(&eid, _engine_id_mapping_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_engine_id_mapping_desc, _engine_id_mapping_sl_compat);

		_engine_mngr.ClearMapping();

		int index;
		while ((index = SlIterateArray()) != -1) {
			EngineIDMapping eid;
			SlObject(&eid, slt);
			_engine_mngr.SetID(eid.type, eid.internal_id, eid.grfid, eid.substitute_id, static_cast<EngineID>(index));
		}
	}
};

static const EIDSChunkHandler EIDS;
static const ENGNChunkHandler ENGN;
static const ENGSChunkHandler ENGS;
static const ChunkHandlerRef engine_chunk_handlers[] = {
	EIDS,
	ENGN,
	ENGS,
};

extern const ChunkHandlerTable _engine_chunk_handlers(engine_chunk_handlers);

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file goal_sl.cpp Code handling saving and loading of goals */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/goal_sl_compat.h"

#include "../goal_base.h"

#include "../safeguards.h"

static EncodedString _goal_text;
static EncodedString _goal_progress;
struct GoalText {
	Goal *item;
	~GoalText() { this->item->Record().text = _goal_text; this->item->Record().progress = _goal_progress; }
};

static const SaveLoad _goals_desc[] = {
	     SLE_VARNAME(Goal, Record().company, "company",   SLE_FILE_U16 | SLE_VAR_U8),
	     SLE_VARNAME(Goal, Record().type, "type",      SLE_FILE_U16 | SLE_VAR_U8),
	     SLE_VARNAME(Goal, Record().dst, "dst",       SLE_UINT32),
	    SLEG_SSTR("text", _goal_text,      SLE_STR | SLF_ALLOW_CONTROL),
	SLEG_CONDSSTR("progress", _goal_progress,  SLE_STR | SLF_ALLOW_CONTROL, SLV_182, SL_MAX_VERSION),
	 SLE_CONDVARNAME(Goal, Record().completed, "completed", SLE_BOOL, SLV_182, SL_MAX_VERSION),
};

struct GOALChunkHandler : ChunkHandler {
	GOALChunkHandler() : ChunkHandler('GOAL', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_goals_desc);

		for (Goal *s : Goal::Iterate()) {
			SlSetArrayIndex(s->index);
			_goal_text = s->Record().text;
			_goal_progress = s->Record().progress;
			SlObject(s, _goals_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_goals_desc, _goals_sl_compat);

		int index;
		while ((index = SlIterateArray()) != -1) {
			Goal *s = new (GoalID(index)) Goal();
			_goal_text.clear(); _goal_progress.clear();
			GoalText staged{s};
			SlObject(s, slt);
		}
	}
};

static const GOALChunkHandler GOAL;
static const ChunkHandlerRef goal_chunk_handlers[] = {
	GOAL,
};

extern const ChunkHandlerTable _goal_chunk_handlers(goal_chunk_handlers);

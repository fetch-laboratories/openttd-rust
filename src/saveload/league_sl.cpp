/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file league_sl.cpp Code handling saving and loading of league tables */

#include "../stdafx.h"

#include "saveload.h"

#include "../league_base.h"

#include "../safeguards.h"

static EncodedString _leaguetableelement_text;
static EncodedString _leaguetableelement_score;
struct LeagueTableElementText {
	LeagueTableElement *item;
	~LeagueTableElementText() { this->item->Record().text = _leaguetableelement_text; this->item->Record().score = _leaguetableelement_score; }
};

static const SaveLoad _league_table_elements_desc[] = {
	    SLE_VARNAME(LeagueTableElement, Record().table, "table",       SLE_UINT8),
	SLE_CONDVARNAME(LeagueTableElement, Record().rating, "rating",      SLE_FILE_U64 | SLE_VAR_I64, SL_MIN_VERSION, SLV_LINKGRAPH_EDGES),
	SLE_CONDVARNAME(LeagueTableElement, Record().rating, "rating",      SLE_INT64,                  SLV_LINKGRAPH_EDGES, SL_MAX_VERSION),
	    SLE_VARNAME(LeagueTableElement, Record().company, "company",     SLE_UINT8),
	   SLEG_SSTR("text", _leaguetableelement_text,        SLE_STR | SLF_ALLOW_CONTROL),
	   SLEG_SSTR("score", _leaguetableelement_score,       SLE_STR | SLF_ALLOW_CONTROL),
	    SLE_VARNAME(LeagueTableElement, Record().link_type, "link.type",   SLE_UINT8),
	    SLE_VARNAME(LeagueTableElement, Record().link_target, "link.target", SLE_UINT32),
};

struct LEAEChunkHandler : ChunkHandler {
	LEAEChunkHandler() : ChunkHandler('LEAE', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_league_table_elements_desc);

		for (LeagueTableElement *lte : LeagueTableElement::Iterate()) {
			SlSetArrayIndex(lte->index);
			_leaguetableelement_text = lte->Record().text;
			_leaguetableelement_score = lte->Record().score;
			SlObject(lte, _league_table_elements_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlTableHeader(_league_table_elements_desc);

		int index;
		while ((index = SlIterateArray()) != -1) {
			LeagueTableElement *lte = new (LeagueTableElementID(index)) LeagueTableElement();
			_leaguetableelement_text.clear(); _leaguetableelement_score.clear();
			LeagueTableElementText staged{lte};
			SlObject(lte, slt);
		}
	}
};

static EncodedString _leaguetable_title;
static EncodedString _leaguetable_header;
static EncodedString _leaguetable_footer;
struct LeagueTableText {
	LeagueTable *item;
	~LeagueTableText() { this->item->Record().title = _leaguetable_title; this->item->Record().header = _leaguetable_header; this->item->Record().footer = _leaguetable_footer; }
};

static const SaveLoad _league_tables_desc[] = {
	SLEG_SSTR("title", _leaguetable_title, SLE_STR | SLF_ALLOW_CONTROL),
	SLEG_SSTR("header", _leaguetable_header, SLE_STR | SLF_ALLOW_CONTROL),
	SLEG_SSTR("footer", _leaguetable_footer, SLE_STR | SLF_ALLOW_CONTROL),
};

struct LEATChunkHandler : ChunkHandler {
	LEATChunkHandler() : ChunkHandler('LEAT', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_league_tables_desc);

		for (LeagueTable *lt : LeagueTable::Iterate()) {
			SlSetArrayIndex(lt->index);
			_leaguetable_title = lt->Record().title;
			_leaguetable_header = lt->Record().header;
			_leaguetable_footer = lt->Record().footer;
			SlObject(lt, _league_tables_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlTableHeader(_league_tables_desc);

		int index;
		while ((index = SlIterateArray()) != -1) {
			LeagueTable *lt = new (LeagueTableID(index)) LeagueTable();
			_leaguetable_title.clear(); _leaguetable_header.clear(); _leaguetable_footer.clear();
			LeagueTableText staged{lt};
			SlObject(lt, slt);
		}
	}
};

static const LEAEChunkHandler LEAE;
static const LEATChunkHandler LEAT;
static const ChunkHandlerRef league_chunk_handlers[] = {
	LEAE,
	LEAT,
};

extern const ChunkHandlerTable _league_chunk_handlers(league_chunk_handlers);

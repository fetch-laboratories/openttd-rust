/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file league_base.h %LeagueTable base class. */

#ifndef LEAGUE_BASE_H
#define LEAGUE_BASE_H

#include "company_type.h"
#include "goal_type.h"
#include "league_type.h"
#include "rust/pool.hpp"
#include "rust/encoded_string.h"
#include "strings_type.h"

bool IsValidLink(Link link);

using LeagueTableElementPool = RustPool<LeagueTableElement, LeagueTableElementID, 64>;
extern LeagueTableElementPool _league_table_element_pool;

using LeagueTablePool = RustPool<LeagueTable, LeagueTableID, 4>;
extern LeagueTablePool _league_table_pool;


/**
 * Struct about league table elements.
 * Each LeagueTable is composed of one or more elements. Elements are sorted by their rating (higher=better).
 **/

#include "abi-record-league_type.h"
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_league_table_element_pool_bind(RustPoolCore *);
OpenTTDLeagueTableElementRecord *OPENTTD_RUST_CALL openttd_rust_league_table_element_record_new(LeagueTableElement *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_league_table_element_record_delete(OpenTTDLeagueTableElementRecord *);
void OPENTTD_RUST_CALL openttd_rust_league_table_pool_bind(RustPoolCore *);
OpenTTDLeagueTableRecord *OPENTTD_RUST_CALL openttd_rust_league_table_record_new(LeagueTable *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_league_table_record_delete(OpenTTDLeagueTableRecord *);
}

struct LeagueTableElement : LeagueTableElementPool::PoolItem<&_league_table_element_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDLeagueTableElementRecord *record) const
		{
			_league_table_element_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_league_table_element_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDLeagueTableElementRecord, RecordDeleter> record{openttd_rust_league_table_element_record_new(this, this->index.base())};
	OpenTTDLeagueTableElementRecord &Record() { return *this->record; }
	const OpenTTDLeagueTableElementRecord &Record() const { return *this->record; }
	LeagueTableElement() { _league_table_element_pool.BindRecord(this->index.base(), this, this->record.get()); }

	Link GetLink() const { return Link(this->Record().link_type, this->Record().link_target); }
};


/** Struct about custom league tables */
struct LeagueTable : LeagueTablePool::PoolItem<&_league_table_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDLeagueTableRecord *record) const
		{
			_league_table_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_league_table_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDLeagueTableRecord, RecordDeleter> record{openttd_rust_league_table_record_new(this, this->index.base())};
	OpenTTDLeagueTableRecord &Record() { return *this->record; }
	const OpenTTDLeagueTableRecord &Record() const { return *this->record; }
	LeagueTable() { _league_table_pool.BindRecord(this->index.base(), this, this->record.get()); }

};

#endif /* LEAGUE_BASE_H */

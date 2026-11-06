/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file goal_base.h %Goal base class. */

#ifndef GOAL_BASE_H
#define GOAL_BASE_H

#include "company_type.h"
#include "goal_type.h"
#include "rust/pool.hpp"
#include "rust/encoded_string.h"
#include "strings_type.h"

using GoalPool = RustPool<Goal, GoalID, 64>;
extern GoalPool _goal_pool;

/** Struct about goals, current and completed */

#include "abi-record-goal_type.h"
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_goal_pool_bind(RustPoolCore *);
OpenTTDGoalRecord *OPENTTD_RUST_CALL openttd_rust_goal_record_new(Goal *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_goal_record_delete(OpenTTDGoalRecord *);
}

struct Goal : GoalPool::PoolItem<&_goal_pool> {
	struct RecordDeleter {
		void operator()(OpenTTDGoalRecord *record) const
		{
			_goal_pool.RetireRecord(record->index.base(), record->shell, record);
			openttd_rust_goal_record_delete(record);
		}
	};
	std::unique_ptr<OpenTTDGoalRecord, RecordDeleter> record{openttd_rust_goal_record_new(this, this->index.base())};
	OpenTTDGoalRecord &Record() { return *this->record; }
	const OpenTTDGoalRecord &Record() const { return *this->record; }
	Goal() { _goal_pool.BindRecord(this->index.base(), this, this->record.get()); }

};

#endif /* GOAL_BASE_H */

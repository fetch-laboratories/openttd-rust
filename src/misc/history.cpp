/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history.cpp Implementation of functions for storing historical data. */

#include "../stdafx.h"

#include "../core/bitmath_func.hpp"
#include "history_type.hpp"
#include "history_func.hpp"

#include "../safeguards.h"

/**
 * Update mask of valid records for a historical data.
 * @note Call only for the largest history range sub-division.
 * @param[in,out] valid_history Valid history records.
 * @param hr History range to update mask for.
 * @param cur_month Current economy month.
 */
void UpdateValidHistory(ValidHistoryMask &valid_history, const HistoryRange &hr, uint cur_month)
{
	valid_history = RustHistory::Scalar(0, hr, valid_history, 0, cur_month);
}

/**
 * Test if history data is valid, without extracting data.
 * @param valid_history Mask of valid history records.
 * @param hr History range to test.
 * @param age Age of data to test.
 * @return True iff the data for history range and age is valid.
 */
bool IsValidHistory(ValidHistoryMask valid_history, const HistoryRange &hr, uint age)
{
	return RustHistory::Scalar(1, hr, valid_history, age, TimerGameEconomy::month) != 0;
}

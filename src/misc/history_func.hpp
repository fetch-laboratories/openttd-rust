/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history_func.hpp Functions for storing historical data. */

#ifndef HISTORY_FUNC_HPP
#define HISTORY_FUNC_HPP

#include "../core/bitmath_func.hpp"
#include "../core/math_func.hpp"
#include "../timer/timer_game_economy.h"
#include "history_type.hpp"

void UpdateValidHistory(ValidHistoryMask &valid_history, const HistoryRange &hr, uint cur_month);
bool IsValidHistory(ValidHistoryMask valid_history, const HistoryRange &hr, uint age);

/**
 * Sum history data elements.
 * @note The summation should prevent overflowing, and perform transformations relevant to the type of data.
 * @tparam T type of history data element.
 * @param history History elements to sum.
 * @return Sum of history elements.
 */
template <typename T>
T SumHistory(typename std::span<const T> history);

#	include "../rust/history_adapter.hpp"

/**
 * Rotate historical data.
 * @note Call only for the largest history range sub-division.
 * @tparam T type of history data element.
 * @param history Historical data to rotate.
 * @param valid_history Mask of valid history records.
 * @param hr History range to rotate..
 * @param cur_month Current economy month.
 */
template <typename T>
void RotateHistory(HistoryData<T> &history, ValidHistoryMask valid_history, const HistoryRange &hr, uint cur_month)
{
	RustHistory::Rotate(history, valid_history, hr, cur_month);
}

/**
 * Get an average value for the previous month, as reset for the next month.
 * @param total Accrued total to average. Will be reset to zero.
 * @return Average value for the month.
 */
template <typename T, typename Taccrued>
T GetAndResetAccumulatedAverage(Taccrued &total)
{
	T result = ClampTo<T>(total / std::max(1U, TimerGameEconomy::days_since_last_month));
	total = 0;
	return result;
}

/**
 * Get historical data.
 * @tparam T type of history data element.
 * @param history History data to extract from.
 * @param valid_history Mask of valid history records.
 * @param hr History range to get.
 * @param age Age of data to get.
 * @param cur_month Current economy month.
 * @param[out] result Extracted historical data.
 * @return True iff the data for this history range and age is valid.
 */
template <typename T>
bool GetHistory(const HistoryData<T> &history, ValidHistoryMask valid_history, const HistoryRange &hr, uint age, T &result)
{
	return RustHistory::Get(history, valid_history, hr, age, result);
}

/**
 * Fill some data with historical data.
 * @param history Historical data to fill from.
 * @param valid_history Mask of valid history records.
 * @param hr History range to fill with.
 * @param fillers Fillers to fill with history data.
 */
template <uint N, typename T, typename... Tfillers>
void FillFromHistory(const HistoryData<T> &history, ValidHistoryMask valid_history, const HistoryRange &hr, Tfillers &&... fillers)
{
	T result{};
	for (uint i = 0; i != N; ++i) {
		if (GetHistory(history, valid_history, hr, N - i - 1, result)) {
			(fillers.Fill(i, result), ...);
		} else {
			(fillers.MakeInvalid(i), ...);
		}
	}
}

/**
 * Fill some data with optional historical data.
 * @param history Historical data to fill from, or nullptr if not present.
 * @param valid_history Mask of valid history records.
 * @param hr History range to fill with.
 * @param fillers Fillers to fill with history data.
 */
template <uint N, typename T, typename... Tfillers>
void FillFromHistory(const HistoryData<T> *history, ValidHistoryMask valid_history, const HistoryRange &hr, Tfillers &&... fillers)
{
	if (history != nullptr) {
		FillFromHistory<N>(*history, valid_history, hr, std::forward<Tfillers &&>(fillers)...);
		return;
	}

	/* History isn't present, fill zero or invalid instead. */
	for (uint i = 0; i != N; ++i) {
		if (IsValidHistory(valid_history, hr, N - i - 1)) {
			(fillers.MakeZero(i), ...);
		} else {
			(fillers.MakeInvalid(i), ...);
		}
	}
}

#endif /* HISTORY_FUNC_HPP */

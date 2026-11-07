/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history_adapter.hpp Typed production history queries. */
#ifndef RUST_HISTORY_ADAPTER_HPP
#define RUST_HISTORY_ADAPTER_HPP
#include "history_ffi.h"

namespace RustHistory {

/** Generic history elements retain the original templates; production owners specialize this. */
template <typename T> struct Operations;

struct IndustryOperations {
	using Element = OpenTTDIndustryPair;
	static constexpr auto Get = openttd_rust_history_get_industry;
};

struct TownOperations {
	using Element = OpenTTDTownSuppliedHistory;
	static constexpr auto Get = openttd_rust_history_get_town;
};

template <typename T>
bool Get(const HistoryData<T> &history, ValidHistoryMask mask, const HistoryRange &hr, uint age, T &result)
{
	auto query = Operations<T>::Get(reinterpret_cast<const typename Operations<T>::Element *>(history.data()), mask, &hr, age, reinterpret_cast<typename Operations<T>::Element *>(&result));
	if (!query.in_range) NOT_REACHED();
	return query.is_valid;
}

} // namespace RustHistory
#endif /* RUST_HISTORY_ADAPTER_HPP */

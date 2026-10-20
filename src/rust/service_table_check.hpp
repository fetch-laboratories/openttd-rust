/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file service_table_check.hpp Typed callback completeness checks for actual production tables. */
#ifndef OPENTTD_RUST_SERVICE_TABLE_CHECK_HPP
#define OPENTTD_RUST_SERVICE_TABLE_CHECK_HPP
#include "service_defaults.hpp"
#include <type_traits>

template <typename T> bool AbiServicesComplete(const T &table)
{
	bool complete = true;
	AbiVisitMembers(table, [&complete](const char *, auto member) {
		using Member = decltype(member);
		if constexpr (std::is_pointer_v<Member> && std::is_function_v<std::remove_pointer_t<Member>>) {
			complete &= member != nullptr && member != OpenTTDAbortService<Member>::Call;
		}
	});
	return complete;
}
#endif /* OPENTTD_RUST_SERVICE_TABLE_CHECK_HPP */

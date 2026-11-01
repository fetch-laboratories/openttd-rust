/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle-record-fixture.hpp Vehicle field access for fixtures compiled against both games: the candidate's shared record, the reference's members. */
#pragma once
#include <type_traits>

#ifdef WITH_RUST
#define FIXTURE_FIELD(v, field) ((v)->Record().field)
#define FIXTURE_CACHE(v, field) ((v)->Record().field)
#define FIXTURE_GET(v, Name, field) ((v)->Get##Name())
#define FIXTURE_SET(v, Name, field, value) ((v)->Set##Name(value))
/** A typed value as the candidate record stores it. */
template <typename T>
constexpr auto FixtureRaw(T value)
{
	if constexpr (std::is_enum_v<T>) {
		return static_cast<std::underlying_type_t<T>>(value);
	} else {
		return value.base();
	}
}
#else
#define FIXTURE_FIELD(v, field) ((v)->field)
#define FIXTURE_CACHE(v, field) ((v)->vcache.field)
#define FIXTURE_GET(v, Name, field) ((v)->field)
#define FIXTURE_SET(v, Name, field, value) ((v)->field = (value))
template <typename T>
constexpr T FixtureRaw(T value)
{
	return value;
}
#endif
#define FIXTURE_FIELD_TYPE(v, field) std::remove_reference_t<decltype(FIXTURE_FIELD(v, field))>

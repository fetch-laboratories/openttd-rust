/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle-record-fixture.hpp Vehicle and station field access for fixtures compiled against both games: the candidate's shared records, the reference's members. */
#pragma once
#include <type_traits>

#ifdef WITH_RUST
#define FIXTURE_SCOPE_VEHICLE(scope) ((scope).GetVehicle())
#define FIXTURE_NAME(st) ((st)->GetName())
#define FIXTURE_SET_NAME(st, value) ((st)->SetName(value))
#define FIXTURE_FIELD(v, field) ((v)->Record().field)
#define FIXTURE_CACHE(v, field) ((v)->Record().field)
#define FIXTURE_GET(v, Name, field) ((v)->Get##Name())
#define FIXTURE_SET(v, Name, field, value) ((v)->Set##Name(value))
/* Goods entries and airports are views by value in the candidate, members in the reference. */
#define FIXTURE_GOODS(st, cargo) ((st)->GetGoods(cargo))
#define FIXTURE_CATCHMENT(st) ((st)->GetCatchmentTiles())
#define FIXTURE_LOADING(st) ((st)->LoadingVehicles())
#define FIXTURE_INDUSTRIES_NEAR(st) ((st)->IndustriesNear())
#define FIXTURE_GOODS_RANGE(st) ((st)->Goods())
#define FIXTURE_AIRPORT(st) ((st)->GetAirport())
#define FIXTURE_RECT(st) (StationRect{const_cast<OpenTTDStationRect *>(&(st)->Record().rect)})
#define FIXTURE_SET_AIRPORT_TILE(st, value) ((st)->Record().airport.area.tile = (value).base())
#define FIXTURE_AIRPORT_TILE(a) ((a).GetTile())
#define FIXTURE_VFIELD(x, field) ((x).Record().field)
#define FIXTURE_VGET(x, Name, field) ((x).Get##Name())
#define FIXTURE_VSET(x, Name, field, value) ((x).Set##Name(value))
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
#define FIXTURE_SCOPE_VEHICLE(scope) ((scope).v)
#define FIXTURE_NAME(st) ((st)->name)
#define FIXTURE_SET_NAME(st, value) ((st)->name = (value))
#define FIXTURE_FIELD(v, field) ((v)->field)
#define FIXTURE_CACHE(v, field) ((v)->vcache.field)
#define FIXTURE_GET(v, Name, field) ((v)->field)
#define FIXTURE_SET(v, Name, field, value) ((v)->field = (value))
#define FIXTURE_GOODS(st, cargo) ((st)->goods[cargo])
#define FIXTURE_CATCHMENT(st) ((st)->catchment_tiles)
#define FIXTURE_LOADING(st) ((st)->loading_vehicles)
#define FIXTURE_INDUSTRIES_NEAR(st) ((st)->industries_near)
#define FIXTURE_GOODS_RANGE(st) ((st)->goods)
#define FIXTURE_AIRPORT(st) ((st)->airport)
#define FIXTURE_RECT(st) ((st)->rect)
#define FIXTURE_SET_AIRPORT_TILE(st, value) ((st)->airport.tile = (value))
#define FIXTURE_AIRPORT_TILE(a) ((a).tile)
#define FIXTURE_VFIELD(x, field) ((x).field)
#define FIXTURE_VGET(x, Name, field) ((x).field)
#define FIXTURE_VSET(x, Name, field, value) ((x).field = (value))
template <typename T>
constexpr T FixtureRaw(T value)
{
	return value;
}
#endif
#define FIXTURE_FIELD_TYPE(v, field) std::remove_reference_t<decltype(FIXTURE_FIELD(v, field))>

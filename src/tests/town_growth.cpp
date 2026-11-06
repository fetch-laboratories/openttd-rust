/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_growth.cpp Narrow owner/legacy staging gaps outside semantic console scenarios. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../town.h"
#include "town_protocol.hpp"
#include "../safeguards.h"

TEST_CASE("Town growth - command boundaries and callback reentry")
{
	CheckTownProtocol([](bool result) { CHECK(result); });
}

TEST_CASE("Town lifecycle - canonical metadata, histories and descriptor aliases")
{
	REQUIRE(Town::GetNumItems() == 0);
	auto *a = new (TownID(17)) Town(TileIndex{41});
	new (TownID(19)) Town();
	CHECK(a->Record().xy == TileIndex{41});
	CHECK(a->Record().exclusivity == CompanyID::Invalid());
	a->Record().population = 999;
	Town::Get(TownID(19))->Record().population = 1001;
	CHECK(GetWorldPopulation() == 2000);
	a->Record().received[TAE_FOOD].old_act = 65535;
	a->Record().goal[TAE_WATER] = TOWN_GROWTH_DESERT;
	a->Record().noise_reached = 77;
	/* The record is the owner's first member and names its shell. */
	CHECK(static_cast<void *>(std::addressof(a->Record())) == static_cast<void *>(a->growth_owner.get()));
	CHECK(a->Record().shell == a);
	a->GetOrCreateCargoSupplied(5).history[THIS_MONTH].production = 123;
	a->GetOrCreateCargoSupplied(2).history[LAST_MONTH].transported = 77;
	const auto view = openttd_rust_town_supplied_view(a->growth_owner.get());
	REQUIRE(view.size == 2);
	const auto *slots = static_cast<const OpenTTDTownSupplied *>(view.data);
	CHECK((slots[0].cargo == 2 && slots[0].history[LAST_MONTH].transported == 77));
	CHECK((slots[1].cargo == 5 && slots[1].history[THIS_MONTH].production == 123));
	CHECK(a->GetOrCreateCargoSupplied(5).history[THIS_MONTH].production == 123);
	CHECK(a->GetCargoSupplied(2)->history[LAST_MONTH].transported == 77);
	CHECK(a->GetCargoSupplied(3) == a->supplied.end());
	CHECK(a->GetCargoSupplied(INVALID_CARGO) == a->supplied.end());
	const auto snapshot = a->CopyTownCache();
	a->Record().population = 1000;
	CHECK((snapshot.population == 999 && snapshot != a->CopyTownCache()));
	_town_pool.CleanPool();
	CHECK(GetWorldPopulation() == 0);
	a = new (TownID(17)) Town();
	CHECK((a->supplied.empty() && a->Record().population == 0));
	_town_pool.CleanPool();
}

TEST_CASE("Town growth - indexed construction, descriptor aliases and pool reuse")
{
	REQUIRE(Town::GetNumItems() == 0);
	auto *a = new (TownID(17)) Town();
	auto *b = new (TownID(19)) Town();
	const auto *descriptor = _town_pool.descriptor;
	CHECK(descriptor->first_unused == 20);
	CHECK(descriptor->first_free == 0);
	CHECK(descriptor->items == 2);
	CHECK(descriptor->slots[17].shell == a);
	CHECK(descriptor->slots[17].record == a->growth_owner.get());
	CHECK(descriptor->slots[19].record == b->growth_owner.get());
	REQUIRE(Town::CanAllocateItem());
	auto *c = new Town();
	CHECK(c->index == TownID(0));
	CHECK(descriptor->slots[0].record == c->growth_owner.get());
	CHECK(static_cast<uint16_t>(a->Record().grow_counter) == 0);
	CHECK(static_cast<uint16_t>(a->Record().growth_rate) == 0);
	CHECK(a->Record().flags.base() == 0);
	a->Record().grow_counter = 65535; a->Record().growth_rate = 0x8005;
	a->Record().flags.Set(TownFlag::HasChurch);
	CHECK(b->Record().shell == b);
	a->Record().grow_counter = 61;
	/* The pre-SLV165 adapter's high bit is preserved before conversion, including
	 * the disabled sentinel. Widths/narrowing are the original CITY/TTD descriptors. */
	if (a->Record().growth_rate & 0x8000) a->Record().flags.Set(TownFlag::CustomGrowth);
	a->Record().growth_rate = TownTicksToGameTicks(a->Record().growth_rate & ~0x8000);
	a->Record().grow_counter = TownTicksToGameTicks(a->Record().grow_counter) + a->index.base() % Ticks::TOWN_GROWTH_TICKS;
	CHECK(a->Record().flags.Test(TownFlag::CustomGrowth));
	CHECK(static_cast<uint16_t>(a->Record().growth_rate) == 419);
	CHECK(static_cast<uint16_t>(a->Record().grow_counter) == 4356);
	_town_pool.CleanPool(); // Town::~Town's early return must still destroy its owner member.
	CHECK(descriptor->slots.empty());
	CHECK(descriptor->used_bitmap.empty());
	CHECK(descriptor->first_unused == 0);
	CHECK(descriptor->first_free == 0);
	CHECK(descriptor->items == 0);
	CHECK(descriptor->checked == 0);
	CHECK_FALSE(descriptor->cleaning);
	a = new (TownID(17)) Town();
	CHECK(descriptor->slots[17].record == a->growth_owner.get());
	CHECK(static_cast<uint16_t>(a->Record().grow_counter) == 0);
	CHECK(static_cast<uint16_t>(a->Record().growth_rate) == 0);
	CHECK(static_cast<uint8_t>(a->Record().fund_buildings_months) == 0);
	CHECK(static_cast<uint8_t>(a->Record().road_build_months) == 0);
	CHECK(a->Record().flags.base() == 0);
	_town_pool.CleanPool();
}

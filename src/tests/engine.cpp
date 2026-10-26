/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine.cpp Native engine owner lifetime and ABI evidence. */
#include "../stdafx.h"
#include "../engine_base.h"
#ifdef WITH_RUST
#include "../rust/engine_ffi.h"
#include "../rust/engine_services.h"
#include "../rust/type_availability_ffi.h"
#include "abi_layout.hpp"
#endif
#include "../3rdparty/catch2/catch.hpp"
#include "../safeguards.h"

TEST_CASE("Engine lifecycle canonical ABI", "[rust][engine][abi]")
{
#ifdef WITH_RUST
	ABI_LAYOUT_CHECK;
#include "abi-layout-engine.inc"
#endif
}

TEST_CASE("Engine temporary owners survive relocation and replacement", "[rust][engine][lifetime]")
{
#ifdef WITH_RUST
	std::vector<Engine> staged;
	staged.emplace_back();
	staged[0].age = -123;
	staged[0].preview_company = CompanyID{3};
	staged[0].name = std::string_view("name\0bytes", 10);
	auto *allocation = staged[0].rust_lifecycle.state;
	auto *age = &staged[0].age;
	for (size_t i = 1; i < 1024; ++i) staged.emplace_back();
	REQUIRE(staged[0].rust_lifecycle.state == allocation);
	REQUIRE(&staged[0].age == age);
	REQUIRE(staged[0].age == -123);
	REQUIRE(staged[0].preview_company == CompanyID{3});
	REQUIRE(staged[0].name.View() == std::string_view("name\0bytes", 10));
	/* Placement replacement is used by FixTTOEngines after native staging. */
	staged[0].~Engine();
	::new (&staged[0]) Engine();
	REQUIRE(staged[0].age == 0);
	REQUIRE(staged[0].preview_company == CompanyID::Invalid());
	REQUIRE(staged[0].name.empty());
	/* Canonical aliases immediately see partial decoded writes. */
	staged[0].rust_lifecycle.state->age = INT32_MIN;
	REQUIRE(staged[0].age == INT32_MIN);
	staged.clear();
#endif
}

TEST_CASE("Engine capability observations preserve type and zero-capacity gates", "[rust][engine]")
{
#ifdef WITH_RUST
	struct Case { VehicleType type; uint8_t capacity, cargo, observed_cargo; bool carries; };
	for (const Case c : {
		Case{VEH_TRAIN, 0, 0, INVALID_CARGO, false}, Case{VEH_TRAIN, 255, 0, 0, true},
		Case{VEH_ROAD, 0, 0, INVALID_CARGO, false}, Case{VEH_ROAD, 255, INVALID_CARGO, INVALID_CARGO, false},
		Case{VEH_SHIP, 0, 0, 0, true}, Case{VEH_AIRCRAFT, 0, INVALID_CARGO, INVALID_CARGO, false},
	}) {
		Engine e(c.type, 0);
		e.info.cargo_type = c.cargo;
		if (c.type == VEH_TRAIN) e.VehInfo<RailVehicleInfo>().capacity = c.capacity;
		if (c.type == VEH_ROAD) e.VehInfo<RoadVehicleInfo>().capacity = c.capacity;
		const auto carrying = RustEngineCarrying(RustEngine(&e));
		CHECK(carrying.kind == c.type);
		CHECK(carrying.ground_capacity == c.capacity);
		CHECK(carrying.default_cargo == c.observed_cargo);
		CHECK(e.CanCarryCargo() == c.carries);
		e.index = EngineID{60000};
		e.info.callback_mask = {VehicleCallbackMask::RefitCapacity, VehicleCallbackMask::ArticEngine};
		e.info.misc_flags = {EngineMiscFlag::NoDefaultCargoMultiplier, EngineMiscFlag::NoBreakdownSmoke};
		const auto config = RustEngineCapacityConfig(RustEngine(&e));
		CHECK(config.id == 60000);
		CHECK(config.callbacks == 24);
		CHECK(config.misc == 96);
	}
#endif
}

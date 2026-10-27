/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine.cpp Native engine owner lifetime and ABI evidence. */
#include "../stdafx.h"
#include "../engine_base.h"
#include "../engine_func.h"
#include "../newgrf.h"
#include <thread>
#ifdef WITH_RUST
#include "../rust/engine_ffi.h"
#include "../rust/engine_loading_ffi.h"
#include "../rust/vehicle_properties_ffi.h"
#include "../newgrf/newgrf_internal_vehicle.h"

#include "../rust/engine_services.h"
#include "../rust/type_availability_ffi.h"
#include "../rust/type_definitions_ffi.h"
#include "../rust/type_registry_ffi.h"
#include "../rail.h"
#include "../road.h"
#include "../core/backup_type.hpp"
#include "../rust/newgrf_translation_ffi.h"
#include "abi_layout.hpp"
#endif
#include "../3rdparty/catch2/catch.hpp"
#include "../safeguards.h"

TEST_CASE("Engine lifecycle canonical ABI", "[rust][engine][abi]")
{
#ifdef WITH_RUST
	ABI_LAYOUT_CHECK;
#include "abi-layout-engine.inc"
#include "abi-layout-engine_loading.inc"
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

TEST_CASE("Engine mappings preserve reservation, sorted identity and replacement metadata", "[engine][loading]")
{
	std::array<std::vector<EngineIDMapping>, VEH_COMPANY_END> saved;
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		auto view = _engine_mngr.Mappings(type);
		saved[type].assign(view.begin(), view.end());
	}
	_engine_mngr.ResetToDefaultMapping();
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		REQUIRE(_engine_mngr.Mappings(type).size() == _engine_counts[type]);
		REQUIRE(_engine_mngr.GetID(type, 0, INVALID_GRFID) == EngineID{_engine_offsets[type]});
		const auto original = _engine_mngr.GetID(type, 1, INVALID_GRFID);
		REQUIRE(_engine_mngr.UseUnreservedID(type, 1, 17, true) == original);
		REQUIRE(_engine_mngr.GetID(type, 1, 17) == EngineID::Invalid());
		REQUIRE(_engine_mngr.UseUnreservedID(type, 1, 17, false) == original);
		REQUIRE(_engine_mngr.GetID(type, 1, INVALID_GRFID) == EngineID::Invalid());
		REQUIRE(_engine_mngr.GetID(type, 1, 17) == original);
		bool visible = false;
		std::thread game([&] {
			visible = _engine_mngr.GetID(type, 1, 17) == original;
			_engine_mngr.SetID(type, 1, 17, 99, EngineID{500});
		});
		game.join();
		REQUIRE(visible);
		auto map = _engine_mngr.Mappings(type);
		REQUIRE(map.front().grfid == 17);
		REQUIRE(map.front().engine == EngineID{500});
		REQUIRE(map.front().substitute_id == 1);
		_engine_mngr.SetID(type, 300, 12, 44, EngineID{501});
		map = _engine_mngr.Mappings(type);
		REQUIRE(map.front().grfid == 12);
		REQUIRE(map.front().internal_id == 300);
		REQUIRE(map.front().substitute_id == 44);
		REQUIRE(std::ranges::is_sorted(map, std::less{}, EngineIDMappingKeyProjection{}));
	}
	_engine_mngr.ClearMapping();
	for (const auto &map : saved) for (const auto &entry : map) _engine_mngr.SetID(entry.type, entry.internal_id, entry.grfid, entry.substitute_id, entry.engine);
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

#ifdef WITH_RUST
template <typename Info, typename Labels>
static void CheckLiveTypeLabels(Info *types, size_t count, Labels labels)
{
	const auto *pointers = labels();
	for (size_t i = 0; i < count; ++i) CHECK(pointers[i] == &types[i].label);
	Backup<uint32_t> first(types[0].label, 0x51500001);
	Backup<uint32_t> last(types[count - 1].label, 0x51500002);
	CHECK(*pointers[0] == 0x51500001);
	CHECK(*pointers[count - 1] == 0x51500002);
	first.Change(0x51500003);
	CHECK(labels() == pointers);
	CHECK(*pointers[0] == 0x51500003);
	CHECK(types[0].label == 0x51500003);
	last.Restore();
	first.Restore();
}

TEST_CASE("Engine type label services expose live canonical scalars", "[rust][engine][abi]")
{
	extern RailTypeInfo _railtypes[RAILTYPE_END];
	extern RoadTypeInfo _roadtypes[ROADTYPE_END];
	CheckLiveTypeLabels(_railtypes, RAILTYPE_END, GetRustRailRegistryServices().labels);
	CheckLiveTypeLabels(_roadtypes, ROADTYPE_END, GetRustRoadRegistryServices().labels);
}
#endif

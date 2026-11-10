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
#include "../economy_func.h"
#include "../newgrf_engine.h"
#include "../table/strings.h"
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
#include "../timer/timer_game_tick.h"
#include "abi_layout.hpp"
#include "../3rdparty/catch2/catch.hpp"
#include "../safeguards.h"

TEST_CASE("Engine lifecycle canonical ABI", "[rust][engine][abi]")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-engine.inc"
#include "abi-layout-engine_loading.inc"
#include "abi-layout-engine_spec.inc"
}

TEST_CASE("Engine temporary owners survive relocation and replacement", "[rust][engine][lifetime]")
{
	std::vector<Engine> staged;
	staged.emplace_back();
	CHECK(staged[0].Record().type == VEH_INVALID);
	staged[0].index = EngineID{60000};
	CHECK(RustEngineID(RustEngine(&staged[0])) == 60000);
	CHECK_THROWS_AS(staged[0].VehInfo<RailVehicleInfo>(), std::bad_variant_access);
	CHECK(staged[0].Record().info.cargo_label.value == 0);
	CHECK_FALSE(staged[0].Record().info.cargo_label.mixed);
	staged[0].Record().age = -123;
	staged[0].Record().preview_company = CompanyID{3};
	staged[0].SetName(std::string_view("name\0bytes", 10));
	auto *allocation = staged[0].rust_lifecycle.state;
	auto *age = &staged[0].Record().age;
	for (size_t i = 1; i < 1024; ++i) staged.emplace_back();
	REQUIRE(staged[0].rust_lifecycle.state == allocation);
	REQUIRE(EngineOf(allocation) == &staged[0]);
	REQUIRE(RustEngine(&staged[0]) == allocation);
	REQUIRE(&staged[0].Record().age == age);
	REQUIRE(staged[0].Record().age == -123);
	REQUIRE(staged[0].Record().preview_company == CompanyID{3});
	REQUIRE(staged[0].Name() == std::string_view("name\0bytes", 10));
	/* Placement replacement is used by FixTTOEngines after native staging. */
	staged[0].~Engine();
	::new (&staged[0]) Engine();
	REQUIRE(staged[0].Record().age == 0);
	REQUIRE(staged[0].Record().preview_company == CompanyID::Invalid());
	REQUIRE(staged[0].Name().empty());
	/* Decoded writes address the canonical record. */
	staged[0].rust_lifecycle.state->age = INT32_MIN;
	REQUIRE(staged[0].Record().age == INT32_MIN);
	staged.clear();
}

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

TEST_CASE("Engine type availability returns canonical booleans for sparse masks", "[rust][engine][abi]")
{
	static uint64_t available;
	OpenTTDSharedServices shared{};
	shared.company_valid = [](uint8_t) noexcept { return true; };
	shared.clear_square_services.generating_world = []() noexcept { return false; };
	static OpenTTDTypeAvailabilityServices types{};
	shared.current_company = []() noexcept -> uint8_t { return 0; };
	shared.type_availability = []() noexcept -> const OpenTTDTypeAvailabilityServices * { return &types; };
	types.editor = []() noexcept { return false; };
	types.company_roadtypes = [](uint8_t) noexcept { return available; };
	types.company_railtypes = [](uint8_t) noexcept { return available; };
	Backup<uint64_t> road_hidden(*openttd_rust_road_hidden_mask(), 0);
	Backup<uint64_t> rail_hidden(*openttd_rust_rail_hidden_mask(), 0);
	for (uint64_t hidden : {UINT64_C(0), UINT64_C(1) << 61}) {
		road_hidden.Change(hidden);
		rail_hidden.Change(hidden);
		for (uint64_t mask : {UINT64_C(0), UINT64_C(1), (UINT64_C(1) << 60) | (UINT64_C(1) << 61), UINT64_MAX}) {
			available = mask;
			for (RoadType rt = ROADTYPE_BEGIN; rt != ROADTYPE_END; rt++) {
				RoadTypes native{mask};
				native.Reset(RoadTypes{hidden});
				CAPTURE(mask, hidden, rt);
				CHECK(static_cast<unsigned>(openttd_rust_has_road_type(&shared, &types, 0, rt)) == static_cast<unsigned>(native.Test(rt)));
				CHECK(static_cast<unsigned>(openttd_rust_val_road_type(&shared, rt)) == static_cast<unsigned>(native.Test(rt)));
			}
			for (RailType rt = RAILTYPE_BEGIN; rt != RAILTYPE_END; rt++) {
				CAPTURE(mask, hidden, rt);
				const bool expected = !RailTypes{hidden}.Test(rt) && RailTypes{mask}.Test(rt);
				CHECK(static_cast<unsigned>(openttd_rust_has_rail_type(&types, 0, rt)) == static_cast<unsigned>(expected));
				CHECK(static_cast<unsigned>(openttd_rust_val_rail_type(&shared, rt)) == static_cast<unsigned>(expected));
			}
		}
	}
	OpenTTDSharedServices guarded{};
	for (uint8_t rt : {uint8_t{ROADTYPE_END}, uint8_t{ROADTYPE_END + 1}, uint8_t{UINT8_MAX}}) {
		CAPTURE(rt);
		CHECK(openttd_rust_val_road_type(&guarded, static_cast<RoadType>(rt)) == 0);
	}
	for (uint8_t rt : {uint8_t{RAILTYPE_END}, uint8_t{RAILTYPE_END + 1}, uint8_t{UINT8_MAX}}) {
		CAPTURE(rt);
		CHECK(openttd_rust_val_rail_type(&guarded, static_cast<RailType>(rt)) == 0);
	}
	rail_hidden.Restore();
	road_hidden.Restore();
}


TEST_CASE("Engine runtime observes live specs and original property and price kinds", "[rust][engine]")
{
	static OpenTTDVehicleResolverServices resolver;
	static uint16_t result, observed_engine;
	static uint32_t observed_property, calls;
	static const OpenTTDEngine *runtime_engine;
	resolver.engine_by_id = [](uint16_t id) noexcept -> const OpenTTDEngine * { CHECK(id == 123); return runtime_engine; };
	resolver.resolve_callback = [](OpenTTDVehicleResolverState *, uint16_t engine, uint16_t cb, uint32_t property, uint32_t p2, int32_t *, size_t count) noexcept -> uint16_t {
		CHECK(cb == CBID_VEHICLE_MODIFY_PROPERTY);
		CHECK(p2 == 0);
		CHECK(count == 0);
		observed_engine = engine;
		observed_property = property;
		++calls;
		return result;
	};
	auto services = GetRustEngineCapabilitiesServices();
	static auto vehicles = GetRustVehicleServices(); vehicles.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; }; services.vehicles = &vehicles;
	struct Case { VehicleType type; Price build, running; uint8_t build_prop, run_prop; };
	for (const Case c : {
		Case{VEH_TRAIN, PR_BUILD_VEHICLE_TRAIN, PR_RUNNING_TRAIN_DIESEL, PROP_TRAIN_COST_FACTOR, PROP_TRAIN_RUNNING_COST_FACTOR},
		Case{VEH_ROAD, PR_BUILD_VEHICLE_ROAD, PR_RUNNING_ROADVEH, PROP_ROADVEH_COST_FACTOR, PROP_ROADVEH_RUNNING_COST_FACTOR},
		Case{VEH_SHIP, PR_BUILD_VEHICLE_SHIP, PR_RUNNING_SHIP, PROP_SHIP_COST_FACTOR, PROP_SHIP_RUNNING_COST_FACTOR},
		Case{VEH_AIRCRAFT, PR_BUILD_VEHICLE_AIRCRAFT, PR_RUNNING_AIRCRAFT, PROP_AIRCRAFT_COST_FACTOR, PROP_AIRCRAFT_RUNNING_COST_FACTOR},
	}) {
		Engine e(c.type, 0); e.index = EngineID{123};
		Backup<const OpenTTDEngine *> live_engine(runtime_engine, RustEngine(&e));
		GRFFile file; file.grfid = 0x613A0003;
		e.Record().bindings.grf_prop.grfid = 0xBAD;
		e.Record().bindings.grf_prop.grffile = &file;
		CHECK(e.GetGRFID() == file.grfid);
		e.Record().bindings.grf_prop.grffile = nullptr;
		CHECK(e.GetGRFID() == 0);
		e.Record().bindings.grf_prop.grffile = &file;
		if (c.type == VEH_TRAIN) { auto &i = e.VehInfo<RailVehicleInfo>(); i.railveh_type = RAILVEH_SINGLEHEAD; i.running_cost_class = c.running; i.cost_factor = 53; i.running_cost = 29; }
		if (c.type == VEH_ROAD) { auto &i = e.VehInfo<RoadVehicleInfo>(); i.running_cost_class = c.running; i.cost_factor = 53; i.running_cost = 29; }
		if (c.type == VEH_SHIP) { auto &i = e.VehInfo<ShipVehicleInfo>(); i.cost_factor = 53; i.running_cost = 29; }
		if (c.type == VEH_AIRCRAFT) { auto &i = e.VehInfo<AircraftVehicleInfo>(); i.cost_factor = 53; i.running_cost = 29; i.max_range = 65535; }
		for (int64_t base : {INT64_C(2560), INT64_MAX, INT64_MIN}) {
			Backup<Money> build_price(_price[c.build], Money{base});
			Backup<Money> run_price(_price[c.running], Money{base});
			for (int8_t shift : {-2, 0, 2}) {
				file.price_base_multipliers[c.build] = shift;
				file.price_base_multipliers[c.running] = shift;
				for (uint16_t value : {uint16_t{CALLBACK_FAILED}, uint16_t{0}, uint16_t{32767}, uint16_t{65534}}) {
					result = value; calls = 0;
					CHECK(openttd_rust_engine_cost(&services, RustEngine(&e)) == GetPrice(c.build, value == CALLBACK_FAILED ? 53 : value, &file, -8).base());
					CHECK(calls == 1); CHECK(observed_engine == 123); CHECK(observed_property == c.build_prop);
					CHECK(openttd_rust_engine_running_cost(&services, RustEngine(&e)) == GetPrice(c.running, value == CALLBACK_FAILED ? 29 : value, &file, -8).base());
					CHECK(calls == 2); CHECK(observed_property == c.run_prop);
				}
			}
			build_price.Restore(); run_price.Restore();
		}
		if (c.type == VEH_TRAIN || c.type == VEH_ROAD) {
			if (c.type == VEH_TRAIN) e.VehInfo<RailVehicleInfo>().running_cost_class = INVALID_PRICE;
			else e.VehInfo<RoadVehicleInfo>().running_cost_class = INVALID_PRICE;
			calls = 0;
			CHECK(openttd_rust_engine_running_cost(&services, RustEngine(&e)) == 0);
			CHECK(calls == 0);
		}
		if (c.type == VEH_TRAIN) {
			e.VehInfo<RailVehicleInfo>().railveh_type = RAILVEH_WAGON; result = CALLBACK_FAILED;
			CHECK(openttd_rust_engine_cost(&services, RustEngine(&e)) == GetPrice(PR_BUILD_VEHICLE_WAGON, 53, &file, -8).base());
		}
		if (c.type == VEH_AIRCRAFT) {
			result = CALLBACK_FAILED; CHECK(openttd_rust_engine_range(&services, RustEngine(&e)) == 65535);
			result = 32767; CHECK(openttd_rust_engine_range(&services, RustEngine(&e)) == 32767);
			CHECK(observed_property == PROP_AIRCRAFT_RANGE);
		}
		live_engine.Restore();
		e.Record().bindings.grf_prop.grffile = nullptr;
	}
}

TEST_CASE("Engine runtime enablement short circuits and lifetime retains widths", "[rust][engine]")
{
	Engine e(VEH_ROAD, 0);
	auto engine = EngineServices();
	/* Invalid strings must return before dereferencing shared climate settings. */
	engine.shared = nullptr;
	e.Record().info.string_id = STR_NEWGRF_INVALID_ENGINE;
	CHECK_FALSE(openttd_rust_engine_enabled(&engine, RustEngine(&e)));
	e.Record().info.string_id = STR_EMPTY;
	for (bool climate : {false, true}) {
		e.Record().info.climates = climate ? LandscapeTypes{_settings_game.game_creation.landscape} : LandscapeTypes{};
		CHECK(e.IsEnabled() == climate);
	}
	for (uint8_t extension : {0, 1, 255}) {
		Backup<uint8_t> setting(_settings_game.vehicle.extend_vehicle_life, extension);
		for (int32_t years : {0, 255, 65536, INT32_MAX, INT32_MIN}) {
			e.Record().info.lifelength = TimerGameCalendar::Year{years};
			const uint32_t bits = (static_cast<uint32_t>(years) + extension) * CalendarTime::DAYS_IN_LEAP_YEAR;
			CHECK(e.GetLifeLengthInDays().base() == static_cast<int32_t>(bits));
		}
		setting.Restore();
	}
}

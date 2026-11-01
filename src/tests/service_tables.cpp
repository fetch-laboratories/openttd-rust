/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file service_tables.cpp Production table completeness and typed default contracts. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../rust/services_ffi.h"
#include "../rust/service_table_check.hpp"
#include "../company_type.h"
#include "../newgrf_text_type.h"
#include "abi-visit-OpenTTDRandomServices.hpp"
#include "../settings_type.h"
#include "../rust/settings_table_ffi.h"
#include "abi_layout.hpp"
static_assert(!OPENTTD_SERVICE_RETURN_TYPE<CompanyID>);
static_assert(!OPENTTD_SERVICE_RETURN_TYPE<GRFStringID>);
/* Every production unit that defines CheckAbi_<unit>() for its callback tables. */
#define OPENTTD_ABI_CHECKED_UNITS(X) \
	X(animated_tile) \
	X(industry_cmd) \
	X(town_cmd) \
	X(station_cmd) \
	X(rail_cmd) \
	X(road_cmd) \
	X(townname) \
	X(order_cmd) \
	X(signal) \
	X(tree_cmd) \
	X(economy) \
	X(train_cmd) \
	X(ship_cmd) \
	X(group_cmd) \
	X(vehicle) \
	X(vehicle_cmd) \
	X(vehiclelist) \
	X(engine) \
	X(engine_capabilities) \
	X(aircraft_cmd) \
	X(cargopacket) \
	X(roadveh_cmd) \
	X(roadstop) \
	X(effectvehicle) \
	X(disaster_vehicle) \
	X(services) \
	X(vehicle_services) \
	X(ground_physics) \
	X(company_adapter) \
	X(autoreplace_cmd) \
	X(linkgraphjob) \
	X(water_regions) \
	X(yapf_rail) \
	X(yapf_road) \
	X(yapf_ship) \
	X(monocypher) \
	X(network_command) \
	X(network_connection) \
	X(network_frame) \
	X(network_crypto) \
	X(landscape) \
	X(genworld) \
	X(session) \
	X(object_cmd) \
	X(water_cmd) \
	X(tunnelbridge_cmd) \
	X(subsidy) \
	X(cargomonitor) \
	X(spritegroup) \
	X(storage) \
	X(cargo_registry) \
	X(generic) \
	X(grf_loading) \
	X(settings)
#define OPENTTD_DECLARE_ABI_CHECK(unit) extern bool CheckAbi_##unit();
OPENTTD_ABI_CHECKED_UNITS(OPENTTD_DECLARE_ABI_CHECK)

TEST_CASE("Rust ABI - actual production callback tables are complete")
{
#define OPENTTD_CHECK_ABI(unit) CHECK(CheckAbi_##unit());
	OPENTTD_ABI_CHECKED_UNITS(OPENTTD_CHECK_ABI)
}
TEST_CASE("Rust ABI - omitted and explicit null callbacks fail completeness")
{
	static_assert(std::is_standard_layout_v<OpenTTDRandomServices>);
	static_assert(std::is_trivially_copyable_v<OpenTTDRandomServices>);
	static_assert(std::is_aggregate_v<OpenTTDRandomServices>);
	OpenTTDRandomServices table{};
	CHECK_FALSE(AbiServicesComplete(table));
	table = {.save = []() noexcept { return OpenTTDRandomSeeds{}; }, .restore = [](OpenTTDRandomSeeds) noexcept {}, .set_seed = [](uint32_t) noexcept {}};
	CHECK(AbiServicesComplete(table));
	table.restore = nullptr;
	CHECK_FALSE(AbiServicesComplete(table));
}
TEST_CASE("Rust ABI - setting callback records")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-settings.inc"
}

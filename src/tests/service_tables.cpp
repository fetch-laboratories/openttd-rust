/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file service_tables.cpp Production table completeness and typed default contracts. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../rust/services_ffi.h"
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRandomServices.hpp"
extern bool CheckAbi_industry_cmd();
extern bool CheckAbi_town_cmd();
extern bool CheckAbi_station_cmd();
extern bool CheckAbi_rail_cmd();
extern bool CheckAbi_road_cmd();
extern bool CheckAbi_townname();
extern bool CheckAbi_order_cmd();
extern bool CheckAbi_signal();
extern bool CheckAbi_tree_cmd();
extern bool CheckAbi_economy();
extern bool CheckAbi_train_cmd();
extern bool CheckAbi_ship_cmd();
extern bool CheckAbi_group_cmd();
extern bool CheckAbi_vehicle();
extern bool CheckAbi_engine();
extern bool CheckAbi_aircraft_cmd();
extern bool CheckAbi_cargopacket();
extern bool CheckAbi_roadveh_cmd();
extern bool CheckAbi_effectvehicle();
extern bool CheckAbi_services();
extern bool CheckAbi_vehicle_services();
extern bool CheckAbi_company_adapter();
extern bool CheckAbi_autoreplace_cmd();
extern bool CheckAbi_linkgraphjob();
extern bool CheckAbi_water_regions();
extern bool CheckAbi_yapf_rail();
extern bool CheckAbi_yapf_road();
extern bool CheckAbi_yapf_ship();
extern bool CheckAbi_monocypher();
extern bool CheckAbi_network_crypto();
extern bool CheckAbi_landscape();

TEST_CASE("Rust ABI - actual production callback tables are complete")
{
	CHECK(CheckAbi_industry_cmd());
	CHECK(CheckAbi_town_cmd());
	CHECK(CheckAbi_station_cmd());
	CHECK(CheckAbi_rail_cmd());
	CHECK(CheckAbi_road_cmd());
	CHECK(CheckAbi_townname());
	CHECK(CheckAbi_order_cmd());
	CHECK(CheckAbi_signal());
	CHECK(CheckAbi_tree_cmd());
	CHECK(CheckAbi_economy());
	CHECK(CheckAbi_train_cmd());
	CHECK(CheckAbi_ship_cmd());
	CHECK(CheckAbi_group_cmd());
	CHECK(CheckAbi_vehicle());
	CHECK(CheckAbi_engine());
	CHECK(CheckAbi_aircraft_cmd());
	CHECK(CheckAbi_cargopacket());
	CHECK(CheckAbi_roadveh_cmd());
	CHECK(CheckAbi_effectvehicle());
	CHECK(CheckAbi_services());
	CHECK(CheckAbi_vehicle_services());
	CHECK(CheckAbi_company_adapter());
	CHECK(CheckAbi_autoreplace_cmd());
	CHECK(CheckAbi_linkgraphjob());
	CHECK(CheckAbi_water_regions());
	CHECK(CheckAbi_yapf_rail());
	CHECK(CheckAbi_yapf_road());
	CHECK(CheckAbi_yapf_ship());
	CHECK(CheckAbi_monocypher());
	CHECK(CheckAbi_network_crypto());
	CHECK(CheckAbi_landscape());
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
#endif

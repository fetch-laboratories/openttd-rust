#pragma once
#include "rust/trees_ffi.h"
#include "rust/landscape_loop_ffi.h"
#include "../../simulation/session-fixture.hpp"

// ABI/reentry probe, not simulation evidence. The full game harness compares map/DATE.
struct TreeProbe {
	OpenTTDTreePlantObservation plant{.tile_type = 7, .bridge = false, .ground = 0, .density = 0, .snow = false, .coast = false, .zone = 0};
	OpenTTDTreeObservation tree{.ground = 3, .density = 3, .species = 20, .count = 2, .growth = 3, .zone = 0};
	uint32_t writes = 0, reentries = 0, clears = 0;
	int64_t nested_cost = 0;
	bool abort = false;
	const OpenTTDSharedServices *shared = nullptr;
};
static TreeProbe tree_probe;
template <typename Result, typename... Args>
static Result TreeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static uint32_t TreeRandom(void *) noexcept { return UINT32_MAX; }
static const OpenTTDTreeServices tree_probe_services{
	.plant_observation = [](uint32_t) noexcept { return tree_probe.plant; },
	.tree_observation = [](uint32_t) noexcept { return tree_probe.tree; },
	.tree_count = [](uint32_t) noexcept { return uint32_t{tree_probe.tree.count}; },
	.clear_observation = [](uint32_t) noexcept { return OpenTTDTreeClearObservation{.count = tree_probe.tree.count, .species = tree_probe.tree.species}; },
	.snow_line = TreeUnused<uint8_t>,
	.sin = [](float value) noexcept { return value; },
	.cos = [](float value) noexcept { return value; },
	.make_tree = [](uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept {
		++tree_probe.writes;
		CHECK(type == 12 && count == 2 && growth == 6 && ground == 0 && density == 2);
		++tree_probe.reentries;
		tree_probe.nested_cost = openttd_rust_trees_clear_tile(tile + 1, 1, 5, false, &tree_probe_services, tree_probe.shared);
	},
	.set_ground_density = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.add_count = TreeUnused<void, uint32_t, int32_t>,
	.add_growth = TreeUnused<void, uint32_t, int32_t>,
	.set_growth = TreeUnused<void, uint32_t, uint8_t>,
	.make_clear = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.progress = []() noexcept { return tree_probe.abort; },
	.progress_total = TreeUnused<bool, uint32_t>,
	.town_rating = TreeUnused<void, uint16_t, uint32_t, bool>,
	.command_begin = [](OpenTTDTreeCommand *, uint32_t, uint32_t, bool) noexcept { return INT32_MAX; },
	.command_tile = [](OpenTTDTreeCommand *) noexcept { return uint32_t{1}; },
	.command_next = [](OpenTTDTreeCommand *) noexcept { return UINT32_MAX; },
	.command_debit = TreeUnused<void, OpenTTDTreeCommand *>,
};
static void Trees()
{
	/* Standalone ABI host has no GRF loader: install its disabled feature gate. */
	static const OpenTTDGenericServices generic{.shared = nullptr, .common = nullptr, .enabled = []() noexcept { return false; }};
	openttd_rust_generic_reset(&generic);
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	shared.make_shore = TreeUnused<void, uint32_t>;
	shared.make_snow = TreeUnused<void, uint32_t, uint32_t>;
	shared.play_sound = TreeUnused<void, uint32_t, uint16_t>;
	static const OpenTTDClearServices clear{.make_clear = [](uint32_t, ClearGround, uint32_t) noexcept { ++tree_probe.clears; }};
	static const OpenTTDWaterGroundServices water{
		.rail = nullptr,
		.clear = &clear,
		.vehicles = nullptr,
		.costs = nullptr,
		.company = nullptr,
		.non_flooding = TreeUnused<bool, uint32_t>,
		.set_non_flooding = TreeUnused<void, uint32_t, bool>,
		.coast = TreeUnused<bool, uint32_t>,
		.set_tree_ground = TreeUnused<void, uint32_t, TreeGround, uint32_t>,
		.make_sea = TreeUnused<void, uint32_t>,
		.buoy_or_dock = TreeUnused<bool, uint32_t>,
		.canal_or_river = TreeUnused<bool, uint32_t>,
		.region_context = TreeUnused<OpenTTDWaterRegionContext>,
		.remove_rail = TreeUnused<void, OpenTTDCommandCost *, uint32_t, Track>,
		.airport_tile = TreeUnused<bool, uint32_t>,
		.airport = TreeUnused<OpenTTDAirportGround, uint32_t>,
		.airport_member = TreeUnused<bool, const OpenTTDStation *, uint32_t>,
		.airport_z = TreeUnused<int32_t, const AirportFTAClass *>,
		.victim = TreeUnused<OpenTTDFloodVictim, OpenTTDVehicle *>,
		.aircraft_pose = TreeUnused<OpenTTDAircraftFloodPose, OpenTTDVehicle *>,
		.ground_pose = TreeUnused<OpenTTDGroundFloodPose, OpenTTDVehicle *>,
		.crash = TreeUnused<uint32_t, OpenTTDVehicle *>,
		.ai_event = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
		.game_event = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
		.news = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
	};
	shared.water_services = []() noexcept { return &water; };
	static OpenTTDFixtureMap map{6, 4096};
	for (uint32_t tile = 0; tile < map.size; ++tile) map.base[tile].type = (tile == 1 || tile == 2 ? MP_TREES : MP_STATION) << 4;
	/* Station tiles reach no station arm. */
	for (uint32_t tile = 0; tile < map.size; ++tile) map.extended[tile].m6 = static_cast<uint8_t>(static_cast<uint8_t>(StationType::End) << 3);
	static const OpenTTDMovementServices movement{};
	shared.movement = &movement;
	shared.map = &map.statics;
	shared.map_max_x = []() noexcept { return 63U; };
	shared.map_max_y = []() noexcept { return 63U; };
	shared.tile_type = [](uint32_t tile) noexcept { return static_cast<uint8_t>(tile == 1 || tile == 2 ? 4 : 5); };
	shared.clear_square_services = {
		.docking = [](uint32_t) noexcept { return false; },
		.generating_world = []() noexcept { return false; },
	};
	shared.mark_dirty = [](uint32_t) noexcept {};
	shared.tile_valid = [](uint32_t) noexcept { return false; };
	tree_probe.shared = &shared;
	uint8_t *counter = openttd_rust_tree_counter();
	openttd_rust_trees_initialize();
	CHECK(*counter == 0);
	*counter = 37; // Same boundary write made by DATE/TTD descriptors.
	static uint32_t tick_size;
	const auto Tick = [&](uint32_t size) {
		tick_size = size;
		SessionFixture fixture;
		auto tick_shared = shared;
		tick_shared.map = fixture.shared.map;
		fixture.services.shared = &tick_shared;
		fixture.services.trees = &tree_probe_services;
		fixture.services.tree_tick = []() noexcept { return OpenTTDTreeTickSettings{.tick_counter = 0, .size = tick_size, .climate = 0, .extra = 2}; };
		CHECK(fixture.Run());
	};
	Tick(4096);
	CHECK(*counter == 36 && counter == openttd_rust_tree_counter());
	Tick(4096 * 4096);
	CHECK(*counter == 36 && tree_probe.writes == 0);
	tree_probe.plant = {.tile_type = 0, .bridge = false, .ground = 0, .density = 2, .snow = false, .coast = false, .zone = 0};
	openttd_rust_trees_plant(1, 12, 2, 6, &tree_probe_services, &shared);
	CHECK(tree_probe.writes == 1 && openttd_rust_trees_can_plant(1, false, &tree_probe_services));
	tree_probe.plant.tile_type = 4;
	const OpenTTDCommandServices commands{};
	const auto command = openttd_rust_trees_cmd_plant(nullptr, 1, 1, 255, false,
		{.build_price = 15, .size = 4096, .climate = 0, .editor = false, .execute = false, .company_valid = false}, &tree_probe_services, &shared, nullptr, &commands, 0);
	CHECK(command.cost == 30 && command.status == 0);
	tree_probe.plant.tile_type = 0;
	OpenTTDRoadStopResolverServices roadstops{};
	roadstops.by_tile = [](uint32_t) noexcept -> BaseStation * { return nullptr; };
	auto station = OpenTTDStationLifecycleServices{};
	station.shared = &shared;
	station.roadstop = &roadstops;
	const OpenTTDLandscapeLoopServices loop{
		.shared = &shared,
		.trees = &tree_probe_services,
		.industry = {},
		.ground = {},
		.town = nullptr,
		.station = &station,
		.objects = nullptr,
	};
	const uint32_t cursor = *openttd_rust_tile_loop_cursor();
	*openttd_rust_tile_loop_cursor() = 1;
	{
		SessionFixture fixture;
		fixture.loop = {{.tick_counter = 1, .size_x = 64, .climate = 0, .extra = 2, .ambient = false, .ambient_callback = false}, &loop};
		CHECK(fixture.Run());
	}
	*openttd_rust_tile_loop_cursor() = cursor;
	CHECK(tree_probe.reentries == 1 && tree_probe.clears == 1 && tree_probe.nested_cost == 40 && tree_probe.writes == 1);
	openttd_rust_trees_initialize();
	CHECK(*counter == 0 && counter == openttd_rust_tree_counter());
	tree_probe.abort = true;
	const OpenTTDTreeGenerateSettings generate{.size = 4096, .size_x = 64, .size_y = 64, .climate = 0, .placer = 1, .height_limit = 15, .editor = false, .freeform_edges = true};
	CHECK(openttd_rust_trees_generate(generate, &tree_probe_services, &shared));
	CHECK(openttd_rust_trees_place_randomly(generate, &tree_probe_services, &shared) && tree_probe.writes == 1);
	std::printf("tree_direct_services_counter_abort_and_reentry passed\n");
}
ABI_PROBE(Trees);

#pragma once
#include "rust/trees_ffi.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/company_ffi.h"
#include "tree_map.h"
#include "../../simulation/session-fixture.hpp"

// ABI probe, not simulation evidence. The full game harness compares map/DATE.
struct TreeProbe {
	uint32_t writes = 0;
	uint8_t expense = 0;
	int64_t cost = 0;
	uint32_t error = 0;
	bool abort = false;
};
static TreeProbe tree_probe;
template <typename Result, typename... Args>
static Result TreeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static uint32_t TreeRandom(void *) noexcept { return UINT32_MAX; }
static const OpenTTDTreeServices tree_probe_services{
	.add_count = TreeUnused<void, uint32_t, int32_t>,
	.add_growth = TreeUnused<void, uint32_t, int32_t>,
	.authority = nullptr,
	.cos = [](float value) noexcept { return value; },
	.make_clear = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.make_tree = [](uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept {
		++tree_probe.writes;
		CHECK(tile == 1 && type == 12 && count == 2 && growth == 6 && ground == 0 && density == 2);
	},
	.progress = []() noexcept { return tree_probe.abort; },
	.progress_total = TreeUnused<bool, uint32_t>,
	.set_ground_density = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.set_growth = TreeUnused<void, uint32_t, uint8_t>,
	.sin = [](float value) noexcept { return value; },
	.strings = {
		.already_here = 11,
		.on_water = 13,
		.plant_limit = 12,
		.site_unsuitable = 14,
		.wrong_terrain = 15,
	},
	.town = nullptr,
};
static void Trees()
{
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	shared.mark_dirty = [](uint32_t) noexcept {};
	shared.current_company = []() noexcept -> uint8_t { return 0; };
	shared.company_valid = [](uint8_t) noexcept { return false; };
	static const OpenTTDMovementServices movement{};
	shared.movement = &movement;
	/* Tile 1 is clear grass of density 2; tile 2 holds four trees. */
	static OpenTTDFixtureMap map{6, 4096};
	for (uint32_t tile = 0; tile < map.size; ++tile) map.base[tile].type = MP_STATION << 4;
	map.base[1].type = MP_CLEAR << 4;
	map.base[1].m5 = 2;
	map.base[2].type = MP_TREES << 4;
	map.base[2].m5 = 3 << 6;
	shared.map = &map.statics;
	shared.tile_type = [](uint32_t tile) noexcept { return static_cast<uint8_t>(tile == 1 ? MP_CLEAR : tile == 2 ? MP_TREES : MP_STATION); };
	static const uint8_t climate = 1, placer = 1, height_limit = 15;
	static const bool freeform_edges = true;
	OpenTTDSettingAddresses settings{};
	settings.climate = &climate;
	settings.freeform_edges = &freeform_edges;
	settings.height_limit = &height_limit;
	settings.tree_placer = &placer;
	shared.settings = &settings;

	uint8_t *counter = openttd_rust_tree_counter();
	openttd_rust_trees_initialize();
	CHECK(*counter == 0);
	*counter = 37; // Same boundary write made by DATE/TTD descriptors.
	const auto Tick = [&](uint32_t size, uint8_t extra) {
		SessionFixture fixture;
		auto tick_shared = shared;
		OpenTTDFixtureMap tick_map{6};
		tick_map.size = size;
		fixture.extra_tree_placement = extra;
		tick_shared.map = &tick_map.statics;
		tick_shared.settings = &fixture.settings;
		tick_shared.tick_counter = []() noexcept -> uint64_t { return 0; };
		fixture.services.shared = &tick_shared;
		fixture.services.trees = &tree_probe_services;
		CHECK(fixture.Run());
	};
	/* The decrement ends both ticks before any map read: it does not wrap at 64x64, and
	 * ETP_SPREAD_RAINFOREST stops the wrapping 4096x4096 tick. */
	Tick(4096, 2);
	CHECK(*counter == 36 && counter == openttd_rust_tree_counter());
	Tick(4096 * 4096, 1);
	CHECK(*counter == 36 && tree_probe.writes == 0);

	/* PlaceTree's arctic type 12, two more trees and growth stage 6 keep the clear density. */
	openttd_rust_trees_place(1, 0x00860000, true, &tree_probe_services, &shared);
	CHECK(tree_probe.writes == 1);

	OpenTTDCommandCostServices costs{};
	costs.error = [](OpenTTDCommandCost *, uint32_t error) noexcept { tree_probe.error = error; };
	costs.expense_money = [](OpenTTDCommandCost *, uint8_t expense, int64_t money) noexcept { tree_probe.expense = expense; tree_probe.cost = money; };
	OpenTTDCompanyFinanceServices finance{};
	finance.current_company = []() noexcept -> uint8_t { return 0; };
	finance.company_finances = [](uint8_t) noexcept -> CompanyFinances * { return nullptr; };
	OpenTTDCommandServices commands{};
	commands.finance = &finance;
	commands.cost = &costs;
	commands.shared = &shared;
	int64_t *prices = openttd_rust_economy_prices();
	const int64_t saved_price = prices[PR_BUILD_TREES];
	prices[PR_BUILD_TREES] = 15;
	/* Testing tiles 1 to 2: the full tree tile only sets the message, the clear tile costs one tree. */
	const OpenTTDPlantTreeArgs args{.tile = 2, .start_tile = 1, .tree_to_plant = TREE_INVALID, .diagonal = false};
	uint64_t slot[8]{};
	openttd_rust_trees_cmd_plant(0, &args, reinterpret_cast<OpenTTDCommandCost *>(slot), &tree_probe_services, &shared, &commands);
	CHECK(tree_probe.expense == EXPENSES_OTHER && tree_probe.cost == 15 && tree_probe.error == 0);
	/* A start tile outside the map is CMD_ERROR. */
	const OpenTTDPlantTreeArgs outside{.tile = 2, .start_tile = 4096, .tree_to_plant = TREE_INVALID, .diagonal = false};
	openttd_rust_trees_cmd_plant(0, &outside, reinterpret_cast<OpenTTDCommandCost *>(slot), &tree_probe_services, &shared, &commands);
	CHECK(tree_probe.error == INVALID_STRING_ID);
	prices[PR_BUILD_TREES] = saved_price;

	openttd_rust_trees_initialize();
	CHECK(*counter == 0 && counter == openttd_rust_tree_counter());
	tree_probe.abort = true;
	CHECK(openttd_test_trees_generate(&tree_probe_services, &shared));
	CHECK(openttd_rust_trees_place_randomly(&tree_probe_services, &shared) && tree_probe.writes == 1);
	std::printf("tree_direct_services_counter_command_and_abort passed\n");
}
ABI_PROBE(Trees);

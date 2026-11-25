#include "controller-orders.hpp"
#pragma once
#include "rust/ship_control_ffi.h"
#include "rust/ship_yapf_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/engine_loading_ffi.h"
#include "rust/ground_ffi.h"
#include "rust/settings_table_ffi.h"
#include "rust/movement_ffi.h"
#include "rust/water_regions_ffi.h"

/* Saves cannot observe NOSAVE rotation coordinates. Compare their full bit domain
 * with native C++ narrowing, and exercise real nested owner mutation in a callback. */
struct ShipProbe {
	OpenTTDVehicle record{};
	ShipProbe() { record.type = VEH_SHIP; record.x_pos = 32768; record.y_pos = -32769; record.tick_counter = 255; record.running_ticks = 255; record.direction = DIR_NE; record.cur_speed = 33; }
	OpenTTDConsistState consist{};
	uint32_t stage = 0;
	OpenTTDShipState *owner = nullptr;
	const OpenTTDShipLeaves *leaves = nullptr;
	OpenTTDSharedServices services{};
	OpenTTDMovementServices movement{};
	OpenTTDWaterRegions *regions = nullptr;
};
static ShipProbe ship_probe;
template <typename Result, typename... Args>
static Result ShipProbeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
/* A 512 by 256 sea inside the void border of a map with freeform edges, so that a tile index
 * exceeds 65,535. The ship is on (10, 200) heading north-east; its depot order's tile is four
 * tiles behind it, which the real reverse search answers with "reverse". */
static constexpr uint32_t SHIP_MAP_LOG_X = 9, SHIP_MAP_WIDTH = 1 << SHIP_MAP_LOG_X, SHIP_MAP_SIZE = SHIP_MAP_WIDTH * 256;
static constexpr uint32_t SHIP_TILE = 10 + 200 * SHIP_MAP_WIDTH, SHIP_DEST = SHIP_TILE + 4;
static void ShipBoundary()
{
	using Owner = std::unique_ptr<OpenTTDShipState, decltype(&openttd_rust_ship_state_destroy)>;
	Owner state(openttd_rust_ship_state_new(), openttd_rust_ship_state_destroy);
	CHECK(openttd_rust_ship_get_state(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	for (uint32_t value = 0; value <= UINT16_MAX; ++value) {
		openttd_rust_ship_set_state(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation_x(state.get(), static_cast<int16_t>(value));
		openttd_rust_ship_set_rotation_y(state.get(), static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_state(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation_x(state.get()) == static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_rotation_y(state.get()) == static_cast<int16_t>(value));
	}
	static OpenTTDVehicleMaintenanceServices ship_maintenance;
	static OpenTTDVehicleServices ship_vehicle;
	ship_vehicle.maintenance = &ship_maintenance;
	auto *shell = &ship_probe.record;
	OpenTTDShipLeaves leaves{
		.next_depot = ShipProbeUnused<bool, uint32_t, OpenTTDShipDepot *>,
		.update_viewport = ShipProbeUnused<void, OpenTTDVehicle *, bool, bool>,
		.vehicle = &ship_vehicle
	};
	ship_probe = {};
	ship_probe.consist.current_order_time = -1;
	ship_probe.record.consist = &ship_probe.consist;
	ship_probe.record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&ship_probe);
	ship_probe.record.ship = state.get();
	ship_probe.record.tile = SHIP_TILE;
	ship_probe.record.first = shell;
	openttd_rust_ship_set_state(state.get(), TRACK_BIT_X);
	ship_probe.owner = state.get();
	ship_probe.leaves = &leaves;
	ship_probe.services = OpenTTDFixtureSharedServices(nullptr, ShipProbeUnused<uint32_t, void *>);

	/* The map, the region owner, the movement table and the settings the search reads. */
	static OpenTTDFixtureMap ship_map{SHIP_MAP_LOG_X, SHIP_MAP_SIZE};
	for (uint32_t t = 0; t < SHIP_MAP_SIZE; ++t) {
		const uint32_t x = t % SHIP_MAP_WIDTH, y = t / SHIP_MAP_WIDTH;
		ship_map.base[t] = {};
		ship_map.extended[t] = {};
		ship_map.base[t].type = (x == 0 || y == 0 || x == SHIP_MAP_WIDTH - 1 || y == SHIP_MAP_SIZE / SHIP_MAP_WIDTH - 1 ? MP_VOID : MP_WATER) << 4;
	}
	ship_probe.services.map = &ship_map.statics;
	ship_probe.services.tile_type = [](uint32_t t) noexcept -> uint8_t { return t < SHIP_MAP_SIZE ? ship_map.base[t].type >> 4 : MP_VOID; };
	/* No fixture tile is a lock. */
	ship_probe.services.water_middle_lock = [](uint32_t) noexcept { return false; };
	static const OpenTTDWaterGroundServices ship_water = [] { OpenTTDWaterGroundServices w{}; w.water_regions = []() noexcept { return ship_probe.regions; }; return w; }();
	ship_probe.services.water_services = []() noexcept -> const OpenTTDWaterGroundServices * { return &ship_water; };
	ship_probe.movement.shared = &ship_probe.services; ship_probe.movement.vehicle = &ship_vehicle;
	ship_probe.movement.native_assert = [](bool okay) noexcept { CHECK(okay); };
	ship_probe.services.movement = &ship_probe.movement;
	static const uint32_t ship_curve45_penalty = 20, ship_curve90_penalty = 100;
	static const OpenTTDSettingAddresses ship_settings = [] { OpenTTDSettingAddresses s{}; s.ship_curve45_penalty = &ship_curve45_penalty; s.ship_curve90_penalty = &ship_curve90_penalty; return s; }();
	ship_probe.services.settings = &ship_settings;
	using RegionOwner = std::unique_ptr<OpenTTDWaterRegions, decltype(&openttd_rust_water_destroy)>;
	RegionOwner regions(openttd_rust_water_new(&ship_probe.services, SHIP_MAP_WIDTH, SHIP_MAP_SIZE / SHIP_MAP_WIDTH), openttd_rust_water_destroy);
	ship_probe.regions = regions.get();
	static OpenTTDEngine *ship_engine;
	ship_engine = openttd_rust_engine_new();
	const OpenTTDEngineDefaults defaults{};
	openttd_rust_engine_construct(ship_engine, &defaults, VEH_SHIP, UINT16_MAX);
	ship_engine->vehicle_info.ship.max_speed = 100;

	/* The list's one order is a plain depot order, whose tile is the ship's destination. */
	ControllerOrders orders(&ship_vehicle, {.type = OT_GOTO_DEPOT, .flags = 0, .destination = 0x1234, .refit_cargo = 254, .wait_time = 0, .travel_time = 0, .max_speed = UINT16_MAX});
	static ControllerOrders *ship_orders;
	ship_orders = &orders;
	ship_probe.record.orders = orders.vehicle.get();
	ship_vehicle.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &ship_orders->services; };
	orders.services.vehicle = [](void *v) noexcept { CHECK(v == &ship_probe); return ship_orders->vehicle.get(); };
	orders.services.consist = [](void *) noexcept { return &ship_orders->consist; };
	orders.services.vehicle_type = [](void *) noexcept { return uint64_t{2}; };
	orders.services.invalidate_order = [](void *, int32_t reason) noexcept { CHECK(reason == -2 && ship_probe.stage++ == 0); };
	orders.services.depot_tile = [](uint16_t destination) noexcept { CHECK(destination == 0x1234); return uint64_t{SHIP_DEST}; };
	orders.services.set_destination = [](void *, uint32_t tile) noexcept { ship_probe.record.dest_tile = tile; };
	/* The search reads the ship's engine first of what the fixture serves. */
	ship_vehicle.engine = [](OpenTTDVehicle *v) noexcept -> const OpenTTDEngine * {
		CHECK(v == &ship_probe.record && ship_probe.stage++ == 1 && ship_probe.record.dest_tile == SHIP_DEST);
		return ship_engine;
	};
	leaves.update_viewport = [](OpenTTDVehicle *v, bool force_update, bool update_delta) noexcept {
		CHECK(ship_probe.stage++ == 2 && v == &ship_probe.record && force_update && update_delta);
		CHECK(ship_probe.record.direction == DIR_SW && ship_probe.record.cur_speed == 0 && ship_probe.record.hash_tile_current != nullptr);
		CHECK(openttd_rust_ship_path_size(openttd_rust_ship_state_path(ship_probe.owner)) == 0);
		CHECK(openttd_rust_ship_get_rotation_x(ship_probe.owner) == static_cast<int16_t>(ship_probe.record.x_pos));
		CHECK(openttd_rust_ship_get_rotation_y(ship_probe.owner) == static_cast<int16_t>(ship_probe.record.y_pos));
		/* The outer call holds no Rust reference to this owner during nested mutation. */
		openttd_rust_ship_path_push(openttd_rust_ship_state_path(ship_probe.owner), TRACKDIR_X_NE);
		openttd_rust_ship_destination(&ship_probe.record, 17);
		CHECK(ship_probe.record.dest_tile == 17 && openttd_rust_ship_path_size(openttd_rust_ship_state_path(ship_probe.owner)) == 0);
		openttd_rust_ship_set_rotation(ship_probe.owner, 3);
	};
	/* An active breakdown: HandleBreakdown reads the delay with the tick's counters already
	 * advanced and before any order runs, and the broken-down ship's tick ends there. */
	static OpenTTDVehicleResolverServices ship_resolver{};
	ship_resolver.breakdown_delay = [](OpenTTDVehicle *) noexcept -> uint8_t {
		CHECK(ship_probe.stage == 0 && ship_probe.record.hash_tile_current == nullptr);
		CHECK(ship_probe.record.tick_counter == 0 && ship_probe.record.running_ticks == 0 && ship_probe.consist.current_order_time == 0);
		return 5;
	};
	ship_resolver.unit = [](OpenTTDVehicle *) noexcept -> uint16_t { return 1; };
	ship_vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &ship_resolver; };
	static uint8_t ship_delay;
	ship_delay = 0;
	ship_maintenance.set_breakdown_delay = [](OpenTTDVehicle *, uint8_t delay) noexcept { ship_delay = delay; };
	ship_probe.record.breakdown_ctr = 1;
	/* A cached path element, which the broken-down tick keeps and the reverse tick clears. */
	openttd_rust_ship_path_push(openttd_rust_ship_state_path(state.get()), TRACKDIR_X_NE);
	CHECK(openttd_rust_ship_tick(shell, &leaves, &ship_probe.services));
	CHECK(openttd_rust_ship_path_size(openttd_rust_ship_state_path(state.get())) == 1);
	/* The broken-down ship's tick ends before any position update. */
	CHECK(ship_delay == 4 && ship_probe.stage == 0 && ship_probe.record.hash_tile_current == nullptr);
	/* The same tick state again, running normally: the reverse path, with the cached path. */
	ship_probe.record.breakdown_ctr = 0;
	ship_probe.record.tick_counter = 255;
	ship_probe.record.running_ticks = 255;
	ship_probe.consist.current_order_time = -1;
	CHECK(openttd_rust_ship_tick(shell, &leaves, &ship_probe.services));
	CHECK(ship_probe.stage == 3 && openttd_rust_ship_get_rotation(state.get()) == 3);
	openttd_rust_occupancy_update(nullptr, shell, true);

	/* The depot search: no depot, then the ship's own tile as a depot of its owner. */
	OpenTTDOrdersClosest closest{};
	openttd_rust_ship_find_depot(shell, &leaves, &ship_probe.services, &closest);
	CHECK(closest.found == 0);
	ship_probe.record.owner = 3;
	/* MakeShipDepot's south part along the Y axis: the depot type, the axis and the part, owned by company 3. */
	ship_map.base[SHIP_TILE].m5 = 3 << 4 | AXIS_Y << 1 | 1;
	ship_map.base[SHIP_TILE].m1 = 3;
	/* The reverse search cached this tile's region as sea. */
	openttd_rust_water_invalidate(regions.get(), &ship_probe.services, SHIP_TILE);
	leaves.next_depot = [](uint32_t first, OpenTTDShipDepot *out) noexcept {
		if (first != 0) return false;
		*out = {0x1234, SHIP_TILE};
		return true;
	};
	openttd_rust_ship_find_depot(shell, &leaves, &ship_probe.services, &closest);
	CHECK(closest.found == 1 && closest.tile == SHIP_TILE && closest.destination == 0x1234 && closest.reverse == 0);
	openttd_rust_engine_destroy(std::exchange(ship_engine, nullptr));
	state.reset(openttd_rust_ship_state_new());
	CHECK(openttd_rust_ship_get_rotation_x(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	std::printf("ship scalar widths, native transient narrowing, the real reverse search, depot search and nested owner mutation passed\n");
}
ABI_PROBE(ShipBoundary);

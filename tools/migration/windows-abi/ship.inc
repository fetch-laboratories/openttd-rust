#pragma once
#include "rust/ship_control_ffi.h"
#include "rust/engine_capabilities_ffi.h"

/* Saves cannot observe NOSAVE rotation coordinates. Compare their full bit domain
 * with native C++ narrowing, and exercise real nested owner mutation in a callback. */
struct ShipProbe {
	uint32_t x = 32768, y = static_cast<uint32_t>(-32769), dest = 0;
	uint32_t order_time = UINT32_MAX;
	uint8_t tick = 255, running = 255, direction = 1;
	uint16_t speed = 33;
	uint32_t stage = 0, paths = 0, positions = 0;
	OpenTTDVehicleTileLinks links;
	OpenTTDShipState *owner = nullptr;
	const OpenTTDShipLeaves *leaves = nullptr;
	OpenTTDSharedServices services{};
};
static ShipProbe ship_probe;
template <typename Result, typename... Args>
static Result ShipProbeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
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
	static int ship_shell; ///< Its address is the opaque vehicle handle.
	auto *shell = reinterpret_cast<OpenTTDVehicle *>(&ship_shell);
	/* Initialize every function pointer, including paths unused in this concrete gap. */
	static const OpenTTDEngineCapabilitiesServices capabilities{};
	OpenTTDShipLeaves leaves{
		.depot_dir = ShipProbeUnused<uint32_t, uint32_t>, .depot_axis = ShipProbeUnused<uint32_t, uint32_t>,
		.is_depot = ShipProbeUnused<bool, uint32_t>, .depot_index = ShipProbeUnused<uint32_t, uint32_t>,
		.wait_unbunch = ShipProbeUnused<bool, uint32_t>, .chain_depot = ShipProbeUnused<bool, uint32_t>,
		.servint = ShipProbeUnused<uint32_t, uint32_t>,
		.max_distance = ShipProbeUnused<uint32_t>, .tile_valid = ShipProbeUnused<bool, uint32_t>,
		.tile_type = ShipProbeUnused<uint32_t, uint32_t>, .water_class = ShipProbeUnused<uint32_t, uint32_t>,
		.lock_middle = ShipProbeUnused<bool, uint32_t>, .lock_dir = ShipProbeUnused<uint32_t, uint32_t>,
		.tile_min_z = ShipProbeUnused<uint32_t, uint32_t>, .tile_max_z = ShipProbeUnused<uint32_t, uint32_t>,
		.offset = ShipProbeUnused<uint32_t, uint32_t>,
		.diag_between = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .dist_square = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.dist_manhattan = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .docking = ShipProbeUnused<bool, uint32_t>,
		.dock = ShipProbeUnused<bool, uint32_t>, .dock_water = ShipProbeUnused<bool, uint32_t>,
		.station = ShipProbeUnused<uint32_t, uint32_t>, .industry_station = ShipProbeUnused<uint32_t, uint32_t>,
		.oilrig = ShipProbeUnused<bool, uint32_t>,
		.station_xy = ShipProbeUnused<uint32_t, uint32_t>, .station_contains = ShipProbeUnused<bool, uint32_t, uint32_t>,
		.station_dock = ShipProbeUnused<bool, uint32_t>, .station_visits = ShipProbeUnused<uint32_t, uint32_t>,
		.visit_set = ShipProbeUnused<void, uint32_t, uint32_t>, .arrival = ShipProbeUnused<void, uint32_t, uint32_t>,
		.leave_unbunch = ShipProbeUnused<void, uint32_t>,
		.path_result = ShipProbeUnused<void, uint32_t, bool>, .order_free = ShipProbeUnused<void, uint32_t>,
		.order_dummy = ShipProbeUnused<void, uint32_t>, .order_depot = ShipProbeUnused<void, uint32_t, uint32_t>,
		.order_leave = ShipProbeUnused<void, uint32_t>, .order_increment = ShipProbeUnused<void, uint32_t>,
		.timetable = ShipProbeUnused<void, uint32_t>,
		.start_dirty = ShipProbeUnused<void, uint32_t>, .depot_dirty = ShipProbeUnused<void, uint32_t>,
		.depot_invalidate = ShipProbeUnused<void, uint32_t>, .ships_dirty = ShipProbeUnused<void>,
		.check_orders = ShipProbeUnused<void, uint32_t>,
		.running_cost = ShipProbeUnused<int64_t, uint32_t>, .cost_divisor = ShipProbeUnused<uint32_t>,
		.pay_running = ShipProbeUnused<void, uint32_t, int64_t>, .speed_default = ShipProbeUnused<uint32_t, uint32_t>,
		.age_default = ShipProbeUnused<uint32_t, uint32_t>, .speed_frac = ShipProbeUnused<uint32_t, uint32_t, bool>,
		.update_visual = ShipProbeUnused<void, uint32_t>, .cache_invalidate = ShipProbeUnused<void, uint32_t>,
		.capabilities = &capabilities, .sprite_direction = ShipProbeUnused<void, uint32_t>,
		.tile_x = ShipProbeUnused<uint32_t, uint32_t>, .tile_y = ShipProbeUnused<uint32_t, uint32_t>,
		.build_flag = ShipProbeUnused<bool, uint32_t>, .build_random = ShipProbeUnused<void, uint32_t, uint16_t>,
		.new_position = ShipProbeUnused<OpenTTDShipPosition, uint32_t>, .exit_dir = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.track_direction = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .tracks_reach = ShipProbeUnused<uint32_t, uint32_t>,
		.path_size = ShipProbeUnused<size_t, uint32_t>,
		.path_back = ShipProbeUnused<uint32_t, uint32_t>, .path_pop = ShipProbeUnused<void, uint32_t>,
		.path_clear = ShipProbeUnused<void, uint32_t>,
		.process_orders = ShipProbeUnused<bool, uint32_t>,
		.viewport = ShipProbeUnused<void, uint32_t, bool, bool>,
		.base_viewport = ShipProbeUnused<void, uint32_t>, .visual = ShipProbeUnused<void, uint32_t>,
		.cache = ShipProbeUnused<void, uint32_t>, .play_sound = ShipProbeUnused<void, uint32_t>,
		.yapf_reverse = ShipProbeUnused<OpenTTDShipReverseResult, uint32_t, bool>, .yapf_choose = ShipProbeUnused<OpenTTDShipTrackResult, uint32_t, uint32_t>,
		.update_delta = ShipProbeUnused<void, uint32_t>, .build_owner = ShipProbeUnused<void, uint32_t>,
		.build_z = ShipProbeUnused<void, uint32_t>, .build_properties = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_dates = ShipProbeUnused<void, uint32_t>, .build_acceleration = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_prototype = ShipProbeUnused<void, uint32_t>, .build_interval_percent = ShipProbeUnused<void, uint32_t>,
		.build_capacity = ShipProbeUnused<void, uint32_t, uint32_t>, .set_tile = ShipProbeUnused<void, uint32_t, uint32_t>,
		.set_x = ShipProbeUnused<void, uint32_t, int32_t>, .set_y = ShipProbeUnused<void, uint32_t, int32_t>,
		.set_z = ShipProbeUnused<void, uint32_t, int32_t>, .set_direction = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_tick = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_running = ShipProbeUnused<void, uint32_t, uint8_t>, .set_day = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_order_time = ShipProbeUnused<void, uint32_t, int32_t>, .set_progress = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_last_station = ShipProbeUnused<void, uint32_t, uint16_t>, .set_hidden = ShipProbeUnused<void, uint32_t, bool>,
		.set_max_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_cargo_age = ShipProbeUnused<void, uint32_t, uint16_t>,
		.set_dest = ShipProbeUnused<void, uint32_t, uint32_t>, .tile = ShipProbeUnused<uint32_t, uint32_t>,
		.dest = ShipProbeUnused<uint32_t, uint32_t>, .x = ShipProbeUnused<uint32_t, uint32_t>,
		.y = ShipProbeUnused<uint32_t, uint32_t>, .z = ShipProbeUnused<uint32_t, uint32_t>,
		.direction = ShipProbeUnused<uint32_t, uint32_t>, .speed = ShipProbeUnused<uint32_t, uint32_t>,
		.tick = ShipProbeUnused<uint32_t, uint32_t>, .running = ShipProbeUnused<uint32_t, uint32_t>,
		.day = ShipProbeUnused<uint32_t, uint32_t>, .order_time = ShipProbeUnused<uint32_t, uint32_t>,
		.progress = ShipProbeUnused<uint32_t, uint32_t>, .status = ShipProbeUnused<uint32_t, uint32_t>,
		.owner = ShipProbeUnused<uint32_t, uint32_t>, .last_station = ShipProbeUnused<uint32_t, uint32_t>,
		.order_destination = ShipProbeUnused<uint32_t, uint32_t>, .order_type = ShipProbeUnused<uint32_t, uint32_t>,
		.order_max_speed = ShipProbeUnused<uint32_t, uint32_t>, .acceleration = ShipProbeUnused<uint32_t, uint32_t>,
		.max_speed = ShipProbeUnused<uint32_t, uint32_t>, .state_owner = ShipProbeUnused<OpenTTDShipState *, uint32_t>,
		.patch = ShipProbeUnused<OpenTTDWaterPatch, uint32_t>, .neighbours = ShipProbeUnused<size_t, OpenTTDWaterPatch, OpenTTDWaterPatch *>,
		.next_depot = ShipProbeUnused<bool, uint32_t, OpenTTDShipDepot *>,
		.vehicle = &ship_vehicle
	};
	ship_probe = {};
	ship_probe.owner = state.get();
	ship_probe.leaves = &leaves;
	ship_probe.services = OpenTTDFixtureSharedServices(nullptr, ShipProbeUnused<uint32_t, void *>, ShipProbeUnused<uint32_t, int32_t, int32_t, uint32_t *>);
	leaves.state_owner = [](uint32_t) noexcept { return ship_probe.owner; };
	leaves.x = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.x); };
	leaves.y = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.y); };
	leaves.dest = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.dest); };
	leaves.order_time = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.order_time); };
	leaves.tick = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.tick); };
	leaves.running = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.running); };
	leaves.direction = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.direction); };
	leaves.speed = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.speed); };
	leaves.set_tick = [](uint32_t, uint8_t value) noexcept { ship_probe.tick = value; };
	leaves.set_running = [](uint32_t, uint8_t value) noexcept { ship_probe.running = value; };
	leaves.set_order_time = [](uint32_t, int32_t value) noexcept { ship_probe.order_time = static_cast<uint32_t>(value); };
	leaves.set_direction = [](uint32_t, uint8_t value) noexcept { ship_probe.direction = value; };
	leaves.set_speed = [](uint32_t, uint16_t value) noexcept { ship_probe.speed = value; };
	leaves.set_dest = [](uint32_t, uint32_t value) noexcept { ship_probe.dest = value; };
	leaves.path_clear = [](uint32_t) noexcept { ++ship_probe.paths; };
	ship_vehicle.tile_links = [](OpenTTDVehicle *) noexcept { ++ship_probe.positions; return &ship_probe.links; };
	ship_vehicle.tile = [](OpenTTDVehicle *) noexcept { return uint32_t{0}; };
	ship_vehicle.occupancy_update = [](OpenTTDVehicle *shell) noexcept -> OpenTTDOccupancyUpdateObservation { return {.links = ship_vehicle.tile_links(shell), .tile = ship_vehicle.tile(shell)}; };
	ship_vehicle.map_log_x = []() noexcept { return uint32_t{6}; };
	ship_maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept {
		CHECK(ship_probe.stage++ == 0);
		CHECK(ship_probe.tick == 0 && ship_probe.running == 0 && ship_probe.order_time == 0);
		return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0};
	};
	leaves.process_orders = [](uint32_t) noexcept { CHECK(ship_probe.stage++ == 1); return true; };
	leaves.yapf_reverse = [](uint32_t, bool trackdir) noexcept {
		CHECK(ship_probe.stage++ == 2 && !trackdir);
		return OpenTTDShipReverseResult{true, 255};
	};
	leaves.viewport = [](uint32_t id, bool update_delta, bool force) noexcept {
		CHECK(ship_probe.stage++ == 3 && update_delta && force);
		CHECK(ship_probe.direction == 5 && ship_probe.speed == 0 && ship_probe.paths == 1 && ship_probe.positions == 1);
		CHECK(openttd_rust_ship_get_rotation_x(ship_probe.owner) == static_cast<int16_t>(ship_probe.x));
		CHECK(openttd_rust_ship_get_rotation_y(ship_probe.owner) == static_cast<int16_t>(ship_probe.y));
		/* The outer call holds no Rust reference to this owner during nested mutation. */
		openttd_rust_ship_destination(id, 17, ship_probe.leaves, &ship_probe.services);
		CHECK(ship_probe.dest == 17 && ship_probe.paths == 2);
		openttd_rust_ship_set_rotation(ship_probe.owner, 3);
	};
	CHECK(openttd_rust_ship_tick(4, shell, &leaves, &ship_probe.services));
	CHECK(ship_probe.stage == 4 && openttd_rust_ship_get_rotation(state.get()) == 3);
	openttd_rust_occupancy_update(&ship_vehicle, shell, true);
	CHECK(!openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services).valid);
	leaves.owner = [](uint32_t) noexcept { return uint32_t{3}; };
	leaves.next_depot = [](uint32_t first, OpenTTDShipDepot *out) noexcept {
		if (first != 0) return false;
		*out = {0x1234, 0x80000017, 3, 1};
		return true;
	};
	leaves.dist_square = [](uint32_t, uint32_t) noexcept { return uint32_t{49}; };
	auto depot = openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services);
	CHECK(depot.valid && depot.tile == 0x80000017 && depot.id == 0x1234);
	state.reset(openttd_rust_ship_state_new());
	CHECK(openttd_rust_ship_get_rotation_x(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	std::printf("ship scalar widths, native transient narrowing, direct order and nested owner mutation passed\n");
}
ABI_PROBE(ShipBoundary);

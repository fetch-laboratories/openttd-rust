#include "controller-orders.hpp"
#pragma once
#include "rust/ship_control_ffi.h"
#include "rust/engine_capabilities_ffi.h"

/* Saves cannot observe NOSAVE rotation coordinates. Compare their full bit domain
 * with native C++ narrowing, and exercise real nested owner mutation in a callback. */
struct ShipProbe {
	OpenTTDVehicle record{};
	ShipProbe() { record.type = VEH_SHIP; record.x_pos = 32768; record.y_pos = -32769; record.tick_counter = 255; record.running_ticks = 255; record.direction = 1; record.cur_speed = 33; }
	OpenTTDConsistState consist{};
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
	auto *shell = &ship_probe.record;
	/* Initialize every function pointer, including paths unused in this concrete gap. */
	static const OpenTTDEngineCapabilitiesServices capabilities{};
	OpenTTDShipLeaves leaves{
		.depot_dir = ShipProbeUnused<uint32_t, uint32_t>, .depot_axis = ShipProbeUnused<uint32_t, uint32_t>,
		.is_depot = ShipProbeUnused<bool, uint32_t>, .depot_index = ShipProbeUnused<uint32_t, uint32_t>,
		.wait_unbunch = ShipProbeUnused<bool, uint32_t>, .chain_depot = ShipProbeUnused<bool, uint32_t>,
		.servint = ShipProbeUnused<uint32_t, uint32_t>,
		.tile_valid = ShipProbeUnused<bool, uint32_t>,
		.tile_type = ShipProbeUnused<uint32_t, uint32_t>, .water_class = ShipProbeUnused<uint32_t, uint32_t>,
		.lock_dir = ShipProbeUnused<uint32_t, uint32_t>,
		.tile_min_z = ShipProbeUnused<uint32_t, uint32_t>, .tile_max_z = ShipProbeUnused<uint32_t, uint32_t>,
		.offset = ShipProbeUnused<uint32_t, uint32_t>,
		.diag_between = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .dist_square = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.dist_manhattan = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.dock = ShipProbeUnused<bool, uint32_t>, .dock_water = ShipProbeUnused<bool, uint32_t>,
		.station = ShipProbeUnused<uint32_t, uint32_t>, .industry_station = ShipProbeUnused<uint32_t, uint32_t>,
		.oilrig = ShipProbeUnused<bool, uint32_t>,
		.arrival = ShipProbeUnused<void, uint32_t, uint32_t>,
		.leave_unbunch = ShipProbeUnused<void, uint32_t>,
		.path_result = ShipProbeUnused<void, uint32_t, bool>, .order_free = ShipProbeUnused<void, uint32_t>,
		.order_dummy = ShipProbeUnused<void, uint32_t>, .order_depot = ShipProbeUnused<void, uint32_t, uint32_t>,
		.order_leave = ShipProbeUnused<void, uint32_t>, .order_increment = ShipProbeUnused<void, uint32_t>,
		.timetable = ShipProbeUnused<void, uint32_t>,
		.start_dirty = ShipProbeUnused<void, uint32_t>, .depot_dirty = ShipProbeUnused<void, uint32_t>,
		.depot_invalidate = ShipProbeUnused<void, uint32_t>, .ships_dirty = ShipProbeUnused<void>,
		.check_orders = ShipProbeUnused<void, uint32_t>,
		.cost_divisor = ShipProbeUnused<uint32_t>,
		.pay_running = ShipProbeUnused<void, uint32_t, int64_t>,
		.cache_invalidate = ShipProbeUnused<void, uint32_t>,
		.capabilities = &capabilities, .sprite_direction = ShipProbeUnused<void, uint32_t>,
		.tile_x = ShipProbeUnused<uint32_t, uint32_t>, .tile_y = ShipProbeUnused<uint32_t, uint32_t>,
		.build_random = ShipProbeUnused<void, uint32_t, uint16_t>,
		.new_position = ShipProbeUnused<OpenTTDShipPosition, uint32_t>, .exit_dir = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.track_direction = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .tracks_reach = ShipProbeUnused<uint32_t, uint32_t>,
		.path_size = ShipProbeUnused<size_t, uint32_t>,
		.path_back = ShipProbeUnused<uint32_t, uint32_t>, .path_pop = ShipProbeUnused<void, uint32_t>,
		.path_clear = ShipProbeUnused<void, uint32_t>,
		.viewport = ShipProbeUnused<void, uint32_t, bool, bool>,
		.base_viewport = ShipProbeUnused<void, uint32_t>,
		.play_sound = ShipProbeUnused<void, uint32_t>,
		.yapf_reverse = ShipProbeUnused<OpenTTDShipReverseResult, uint32_t, bool>, .yapf_choose = ShipProbeUnused<OpenTTDShipTrackResult, uint32_t, uint32_t>,
		.update_delta = ShipProbeUnused<void, uint32_t>, .build_owner = ShipProbeUnused<void, uint32_t>,
		.build_z = ShipProbeUnused<void, uint32_t>, .build_properties = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_dates = ShipProbeUnused<void, uint32_t>,
		.build_prototype = ShipProbeUnused<void, uint32_t>, .build_interval_percent = ShipProbeUnused<void, uint32_t>,



		.order_destination = ShipProbeUnused<uint32_t, uint32_t>, .order_type = ShipProbeUnused<uint32_t, uint32_t>,
		.order_max_speed = ShipProbeUnused<uint32_t, uint32_t>,
		.state_owner = ShipProbeUnused<OpenTTDShipState *, uint32_t>,
		.patch = ShipProbeUnused<OpenTTDWaterPatch, uint32_t>, .neighbours = ShipProbeUnused<size_t, OpenTTDWaterPatch, OpenTTDWaterPatch *>,
		.next_depot = ShipProbeUnused<bool, uint32_t, OpenTTDShipDepot *>,
		.vehicle = &ship_vehicle
	};
	ship_probe = {};
	ship_probe.consist.current_order_time = -1;
	ship_probe.record.consist = &ship_probe.consist;
	ship_probe.record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&ship_probe);
	ship_probe.owner = state.get();
	ship_probe.leaves = &leaves;
	ship_probe.services = OpenTTDFixtureSharedServices(nullptr, ShipProbeUnused<uint32_t, void *>, ShipProbeUnused<uint32_t, int32_t, int32_t, uint32_t *>);
	leaves.state_owner = [](uint32_t) noexcept { return ship_probe.owner; };
	leaves.path_clear = [](uint32_t) noexcept { ++ship_probe.paths; };
	ship_vehicle.tile_links = [](OpenTTDVehicle *) noexcept { ++ship_probe.positions; return &ship_probe.links; };
	ship_vehicle.map_log_x = []() noexcept { return uint32_t{6}; };
	ship_maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept {
		CHECK(ship_probe.stage++ == 0);
		CHECK(ship_probe.record.tick_counter == 0 && ship_probe.record.running_ticks == 0 && ship_probe.consist.current_order_time == 0);
		return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0};
	};
	ControllerOrders orders(&ship_vehicle);
	static ControllerOrders *ship_orders;
	ship_orders = &orders;
	static OpenTTDVehicle *orders_shell;
	orders_shell = shell;
	ship_vehicle.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &ship_orders->services; };
	ship_vehicle.ship_get = [](uint32_t id) noexcept { CHECK(id == 4); return orders_shell; };
	orders.services.vehicle = [](void *v) noexcept { CHECK(v == &ship_probe); return ship_orders->vehicle.get(); };
	orders.services.consist = [](void *) noexcept { return &ship_orders->consist; };
	orders.services.vehicle_type = [](void *) noexcept { return uint64_t{2}; };
	orders.services.invalidate_order = [](void *, int32_t reason) noexcept { CHECK(reason == -2 && ship_probe.stage++ == 1); };
	leaves.yapf_reverse = [](uint32_t, bool trackdir) noexcept {
		CHECK(ship_probe.stage++ == 2 && !trackdir);
		return OpenTTDShipReverseResult{true, 255};
	};
	leaves.viewport = [](uint32_t id, bool update_delta, bool force) noexcept {
		CHECK(ship_probe.stage++ == 3 && update_delta && force);
		CHECK(ship_probe.record.direction == 5 && ship_probe.record.cur_speed == 0 && ship_probe.paths == 1 && ship_probe.positions == 1);
		CHECK(openttd_rust_ship_get_rotation_x(ship_probe.owner) == static_cast<int16_t>(ship_probe.record.x_pos));
		CHECK(openttd_rust_ship_get_rotation_y(ship_probe.owner) == static_cast<int16_t>(ship_probe.record.y_pos));
		/* The outer call holds no Rust reference to this owner during nested mutation. */
		openttd_rust_ship_destination(id, 17, ship_probe.leaves, &ship_probe.services);
		CHECK(ship_probe.record.dest_tile == 17 && ship_probe.paths == 2);
		openttd_rust_ship_set_rotation(ship_probe.owner, 3);
	};
	CHECK(openttd_rust_ship_tick(4, shell, &leaves, &ship_probe.services));
	CHECK(ship_probe.stage == 4 && openttd_rust_ship_get_rotation(state.get()) == 3);
	openttd_rust_occupancy_update(&ship_vehicle, shell, true);
	CHECK(!openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services).valid);
	ship_probe.record.owner = 3;
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

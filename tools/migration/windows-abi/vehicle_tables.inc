#pragma once
/* Complete typed tables exercise road cdecl calls without importing game globals.
 * These are boundary/lifetime checks, not a second simulation oracle. */
template <typename Function> struct VehicleUnused;
template <typename Result, typename... Args>
struct VehicleUnused<Result (OPENTTD_RUST_CALL *)(Args...) noexcept> {
	static Result OPENTTD_RUST_CALL Call(Args...) noexcept
	{
		CHECK(false);
		if constexpr (!std::is_void_v<Result>) return {};
	}
};
#define VEHICLE_UNUSED(table, field) table.field = VehicleUnused<decltype(table.field)>::Call
static OpenTTDVehicleMaintenanceServices _vehicle_maintenance_probe;
struct RoadTestTables {
	OpenTTDVehicleServices vehicle;
	OpenTTDGroundServices ground;
	OpenTTDGroundPhysicsServices physics{};
	OpenTTDVehicleServices physics_vehicle{};
	OpenTTDSharedServices shared;
	OpenTTDRoadLeaves leaves;
	explicit RoadTestTables(const OpenTTDSharedServices *world) : shared(*world)
	{
		VEHICLE_UNUSED(vehicle, assert_next);
		VEHICLE_UNUSED(vehicle, record_from_shell);
		VEHICLE_UNUSED(vehicle, first_sprite);
		vehicle.maintenance = &_vehicle_maintenance_probe;
		ground.physics = &physics;
		physics.vehicle = &physics_vehicle;


		VEHICLE_UNUSED(vehicle, check_orders_only);
		VEHICLE_UNUSED(vehicle, write_profit);
		VEHICLE_UNUSED(vehicle, write_economy_service);
		VEHICLE_UNUSED(vehicle, write_calendar_service);
		VEHICLE_UNUSED(vehicle, write_breakdowns);
		VEHICLE_UNUSED(vehicle, write_reliability);
		VEHICLE_UNUSED(vehicle, profit);
		VEHICLE_UNUSED(vehicle, make_dummy);
		VEHICLE_UNUSED(vehicle, order_type);
		VEHICLE_UNUSED(vehicle, order_destination);
		VEHICLE_UNUSED(vehicle, order_nonstop);
		VEHICLE_UNUSED(ground, has_articulated);
		VEHICLE_UNUSED(vehicle, visits);
		VEHICLE_UNUSED(vehicle, waiting_for_unbunching);
		VEHICLE_UNUSED(vehicle, leave_unbunching_depot);
		VEHICLE_UNUSED(vehicle, reset_depot_unbunching);
		VEHICLE_UNUSED(vehicle, enter_depot);
		VEHICLE_UNUSED(vehicle, pathfinding_result);
		VEHICLE_UNUSED(vehicle, new_position);
		VEHICLE_UNUSED(vehicle, base_viewport);
		VEHICLE_UNUSED(vehicle, property);
		VEHICLE_UNUSED(vehicle, callback);
		VEHICLE_UNUSED(vehicle, set_max_age);
		VEHICLE_UNUSED(vehicle, length_changed);
		VEHICLE_UNUSED(vehicle, play_vehicle_sound);
		VEHICLE_UNUSED(vehicle, play_sound);
		VEHICLE_UNUSED(vehicle, order_free);
		VEHICLE_UNUSED(vehicle, order_dummy);
		VEHICLE_UNUSED(vehicle, start_stop_dirty);
		VEHICLE_UNUSED(vehicle, destroy);
		VEHICLE_UNUSED(ground, update_viewport);
		VEHICLE_UNUSED(shared, tile_type);
		VEHICLE_UNUSED(shared, tile_owner);
		VEHICLE_UNUSED(shared, station_index);
		VEHICLE_UNUSED(shared, depot_index);
		VEHICLE_UNUSED(shared, tunnel_bridge_direction);
		VEHICLE_UNUSED(shared, bridge_speed);
		VEHICLE_UNUSED(leaves, any_road_bits);
		VEHICLE_UNUSED(leaves, is_road_depot_tile);
		VEHICLE_UNUSED(leaves, depot_direction);
		VEHICLE_UNUSED(leaves, disallowed_directions);
		VEHICLE_UNUSED(leaves, is_bay_stop);
		VEHICLE_UNUSED(leaves, bay_direction);
		VEHICLE_UNUSED(leaves, is_station_road_stop);
		VEHICLE_UNUSED(leaves, choose_track);
		VEHICLE_UNUSED(leaves, find_depot);
		VEHICLE_UNUSED(leaves, tram_build_read);
		VEHICLE_UNUSED(leaves, disconnect);
		VEHICLE_UNUSED(leaves, should_stop);
		VEHICLE_UNUSED(leaves, arrival_news);
		VEHICLE_UNUSED(leaves, crash_news);
		VEHICLE_UNUSED(vehicle, engine);
		VEHICLE_UNUSED(leaves, pay_running);
		leaves.vehicle = &vehicle;
		leaves.ground = &ground;
		leaves.shared = &shared;
	}
	RoadTestTables(const RoadTestTables &) = delete;
	RoadTestTables &operator=(const RoadTestTables &) = delete;
};
#undef VEHICLE_UNUSED

#pragma once
#include "rust/aircraft_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/vehicle_visits_ffi.h"
#include "aircraft-boundary.hpp"

#include "vehicle_tables.inc"
#include "controller-orders.hpp"

/* Direct callbacks reenter with the same live scalar allocation. Deletion models
 * Aircraft::PreDestructor releasing airport blocks before freeing that owner. */
static struct AircraftBoundaryProbe {
	OpenTTDVehicleServices vehicle_services;
	OpenTTDSharedServices shared_services;
	OpenTTDAircraftLeaves leaves;
	OpenTTDAircraftVehicle vehicle;
	OpenTTDVehicle record{}, shadow{}, rotor{};
	std::unique_ptr<ControllerOrders> order_owner;
	OpenTTDVehicleCommonServices common{};
	OpenTTDVehicleRefitServices refit{};
	OpenTTDVehicleVisits visits{};
	OpenTTDVehicleTileLinks links[2];
	uint64_t blocks = UINT64_MAX;
	unsigned orders = 0, deleted = 0, moves = 0;
	OpenTTDConsistState consist{};
	uint8_t order_kind = 0;


	bool live = true;
} aircraft_probe;
static void AircraftDirectBoundary()
{
	auto &p = aircraft_probe;
	p.record.consist = &p.consist;
	p.record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&p.record);
	p.record.type = 3;
	p.record.z_pos = 1;
	p.shadow.type = 3; p.shadow.subtype = 4; p.record.next = &p.shadow;
	p.shadow.shell = reinterpret_cast<OpenTTDVehicleShell *>(&p.shadow);
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	CHECK(p.shared_services.is_water_tile != nullptr);
	auto &vehicle = p.vehicle_services;
	auto &shared = p.shared_services;
	auto &leaves = p.leaves;
	/* Every entry loads the map descriptor, including the disaster flight helpers below. */
	static OpenTTDFixtureMap disaster_map{6, 4096};
	shared.map = &disaster_map.statics;
	vehicle.order_type = [](OpenTTDVehicle *) noexcept { return aircraft_probe.order_owner->vehicle->current.type; };
	vehicle.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	vehicle.visits = []() noexcept -> const OpenTTDVehicleVisits * {
		auto &o = *aircraft_probe.order_owner;
		CHECK(o.vehicle->current.type == 3 || o.vehicle->current.type == 5);
		CHECK(aircraft_probe.visits.refit->common->orders == &o.services);
		return &aircraft_probe.visits;
	};
	p.order_owner = std::make_unique<ControllerOrders>(&vehicle);
	p.common.orders = &p.order_owner->services;
	p.refit.common = &p.common;
	p.visits.refit = &p.refit;
	vehicle.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &aircraft_probe.order_owner->services; };
	p.order_owner->services.vehicle_type = [](void *) noexcept { return uint64_t{3}; };
	p.order_owner->services.vehicle = [](void *shell) noexcept { CHECK(shell == aircraft_probe.vehicle.handle); return aircraft_probe.order_owner->vehicle.get(); };
	p.order_owner->services.consist = [](void *) noexcept { return &aircraft_probe.order_owner->consist; };
	p.order_owner->services.invalidate_order = [](void *shell, int32_t reason) noexcept {
		CHECK(shell == aircraft_probe.vehicle.handle && reason == -2);
		auto v = aircraft_probe.vehicle;
		CHECK(aircraft_probe.live);
		aircraft_probe.orders++;
		openttd_rust_aircraft_cache(&aircraft_probe.leaves, v, true);
		v.state->pos = 19;
		aircraft_probe.order_owner->vehicle->current.type = 3;
	};
	static OpenTTDEngineCapabilitiesServices capabilities{};
	static OpenTTDVehicleResolverServices resolver{};
	resolver.capabilities = &capabilities;
	vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	static OpenTTDEngine *engine_record;
	engine_record = openttd_rust_engine_new();
	const OpenTTDEngineDefaults defaults{};
	openttd_rust_engine_construct(engine_record, &defaults, VEH_AIRCRAFT, UINT16_MAX);
	engine_record->vehicle_info.aircraft.max_range = 41;
	engine_record->info.cargo_age_period = 71;
	vehicle.engine = [](OpenTTDVehicle *) noexcept -> const OpenTTDEngine * { return engine_record; };
	vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t property, int32_t fallback) noexcept { if (property == 0x0C) { CHECK(fallback == 0); return int32_t{200}; } CHECK((property == 0x1C && fallback == 71) || (property == 0x1F && fallback == 41)); return fallback; };
	p.record.subtype = 2;
	p.vehicle = {&p.record, openttd_rust_aircraft_state_new(), 17};
	p.record.aircraft = p.shadow.aircraft = p.vehicle.state;
	resolver.id = [](OpenTTDVehicle *) noexcept -> uint32_t { return 18; };
	CHECK(openttd_test_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.orders == 1 && p.vehicle.state->pos == 19 && p.vehicle.state->cached_max_range == 41 && p.vehicle.state->cached_max_range_sqr == 1681);
	CHECK(p.record.cached_max_speed == 2560 && p.record.cached_cargo_age_period == 71);
	/* A ProcessOrders callback replaces the order with OT_DUMMY. HandleLoading
	 * advances both indices even on the second call of a tick. */
	auto second = openttd_rust_order_vector_data(p.order_owner->vector.get())[0];
	openttd_rust_order_vector_insert(p.order_owner->vector.get(), 1, &second);
	p.order_owner->vehicle->current.type = 0;
	p.order_owner->services.invalidate_order = [](void *, int32_t reason) noexcept {
		CHECK(reason == -2 || reason == 0);
		if (reason == -2) aircraft_probe.order_owner->vehicle->current.type = 5;
		if (reason == 0) {
			CHECK(aircraft_probe.order_owner->vehicle->current.type == 5);
			aircraft_probe.order_owner->vehicle->current.type = 3;
		}
	};
	CHECK(openttd_test_aircraft_event(&leaves, p.vehicle, true));
	CHECK(p.order_owner->consist.real_index == 1 && p.order_owner->consist.implicit_index == 1);
	leaves.station = [](uint16_t) noexcept -> const OpenTTDStation * { CHECK(aircraft_probe.live); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{7}; };
	AircraftTestAirportServices().fta = [](const AirportSpec *) noexcept -> const AirportFTAClass * { return reinterpret_cast<const AirportFTAClass *>(&aircraft_probe); };
	leaves.airport_blocks = [](const OpenTTDStation *) noexcept { CHECK(aircraft_probe.live); return &aircraft_probe.blocks; };
	leaves.node = [](const void *ap, uint8_t) noexcept -> const void * { return ap; };
	leaves.fta_blocks = [](const void *) noexcept { return uint64_t{1} << 9; };
	vehicle.destroy = [](OpenTTDVehicle * handle) noexcept {
		CHECK(handle == aircraft_probe.vehicle.handle);
		auto v = aircraft_probe.vehicle;
		CHECK(aircraft_probe.live && v.state->crashed_counter == 10002);
		openttd_rust_aircraft_release_blocks(&aircraft_probe.leaves, v);
		CHECK((aircraft_probe.blocks & ((uint64_t{1} << 8) | (uint64_t{1} << 9) | (uint64_t{1} << 29))) == 0);
		openttd_rust_aircraft_state_destroy(v.state);
		aircraft_probe.live = false;
		aircraft_probe.deleted++;
	};
	p.record.vehstatus = 128;
	p.vehicle.state->targetairport = 7;
	p.vehicle.state->crashed_counter = 9999;
	CHECK(!openttd_test_aircraft_event(&leaves, p.vehicle, false));
	CHECK(!p.live && p.deleted == 1);
	/* A disaster has its own flags and no Aircraft state, even with subtype zero. */
	shared.map_max_x = shared.map_max_y = []() noexcept { return uint32_t{63}; };

	p.record.subtype = 0; p.record.type = 4; p.record.direction = 4; p.record.z_pos = 0; p.record.cached_max_speed = 1;
	uint8_t disaster_flags = 0;
	int32_t min = 0, max = 0;
	OpenTTDAircraftVehicle disaster{&p.record, nullptr, 81};
	openttd_rust_aircraft_flight_bounds(&leaves, disaster, &min, &max);
	CHECK(min == 30 && max == 270);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, false) == 1 && disaster_flags == 4);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, true) == 2 && disaster_flags == 4);
	std::printf("aircraft direct nested mutation, live block release/deletion and disaster flight identity passed\n");
	vehicle.engine = nullptr;
	openttd_rust_engine_destroy(std::exchange(engine_record, nullptr));
}
/* Concrete gaps: native callbacks can replace order fields/rotor counters and
 * direction/progress after the pre-speed observation. This probe forbids stale
 * snapshots at those original observation phases, using the production Rust entry. */
static void AircraftPhaseBoundary()
{
	auto &p = aircraft_probe;
	p = {};
	p.record.consist = &p.consist;
	p.record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&p.record);
	p.record.type = 3;
	p.record.z_pos = 1;
	p.shadow.type = 3; p.shadow.subtype = 4; p.record.next = &p.shadow;
	p.shadow.shell = reinterpret_cast<OpenTTDVehicleShell *>(&p.shadow);
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	auto &v = p.vehicle_services;
	auto &s = p.shared_services;
	auto &leaves = p.leaves;
	static OpenTTDFixtureMap map{6, 4096};
	s.map = &map.statics;
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	p.vehicle = {&p.record, state.get(), 17};

	p.record.tick_counter = 7;
	p.consist.current_order_time = 40;
	p.shadow.next = &p.rotor; p.rotor.type = 3; p.rotor.subtype = 6; p.rotor.vehstatus = 1;
	p.rotor.shell = reinterpret_cast<OpenTTDVehicleShell *>(&p.rotor);
	p.record.aircraft = p.shadow.aircraft = p.rotor.aircraft = state.get();
	static OpenTTDVehicleResolverServices resolver{};
	resolver.id = [](OpenTTDVehicle *record) noexcept -> uint32_t { return record == &aircraft_probe.shadow ? 18 : 19; };
	v.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	v.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	p.order_owner = std::make_unique<ControllerOrders>(&v);
	v.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &aircraft_probe.order_owner->services; };
	p.order_owner->services.vehicle_type = [](void *) noexcept { return uint64_t{3}; };
	p.order_owner->services.vehicle = [](void *shell) noexcept { CHECK(shell == aircraft_probe.vehicle.handle); return aircraft_probe.order_owner->vehicle.get(); };
	p.order_owner->services.consist = [](void *) noexcept { return &aircraft_probe.order_owner->consist; };
	p.order_owner->services.invalidate_order = [](void *, int32_t reason) noexcept { CHECK(reason == -2); aircraft_probe.orders++; aircraft_probe.order_kind = 4; aircraft_probe.order_owner->vehicle->current.type = 4; };
	v.visits = []() noexcept -> const OpenTTDVehicleVisits * { return nullptr; };
	v.order_type = [](OpenTTDVehicle *) noexcept { return aircraft_probe.order_kind; };
	CHECK(openttd_rust_aircraft_tick(&leaves, p.vehicle));
	CHECK(p.record.tick_counter == 8 && p.record.running_ticks == 1 && p.orders == 2 && p.consist.current_order_time == 41);

	/* A speed change dirties the native widget before direction/progress are
	 * read. Their mutation yields exactly one move; a stale snapshot yields none. */

	p.record.subtype = 2;
	p.order_owner->vehicle->current = openttd_rust_order_vector_data(p.order_owner->vector.get())[0];
	v.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{1}; };
	p.vehicle.state->state = 14; p.vehicle.state->targetairport = 7;
	leaves.station = [](uint16_t station) noexcept { CHECK(station == 7); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{0}; }; leaves.rotation = [](const OpenTTDStation *) noexcept { return uint8_t{0}; }; leaves.airport_width = leaves.airport_height = [](const OpenTTDStation *) noexcept { return uint8_t{8}; };
	AircraftTestAirportServices().fta = [](const AirportSpec *) noexcept -> const AirportFTAClass * { return reinterpret_cast<const AirportFTAClass *>(&aircraft_probe); };
	leaves.moving = [](const void *, uint8_t) noexcept { return OpenTTDAircraftMoving{16, 0, 16, 0}; };
	s.map_max_x = s.map_max_y = []() noexcept { return uint32_t{63}; };
	s.plane_speed = []() noexcept { return uint8_t{1}; };
	p.record.acceleration = 8; p.record.cached_max_speed = 100;
	v.start_stop_dirty = [](OpenTTDVehicle *) noexcept { aircraft_probe.record.direction = DIR_NE; aircraft_probe.record.progress = 255; };
	leaves.direction_towards = [](OpenTTDAircraftVehicle, int32_t, int32_t) noexcept { return uint8_t{1}; };
	v.new_position = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleNewPosition{aircraft_probe.record.x_pos + 1, aircraft_probe.record.y_pos, 0}; };
	/* Both callbacks mutate canonical inputs. Subtype must be loaded after
	 * UpdatePosition, then z after the rotor callback, rather than on acquisition. */
	leaves.viewport = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.moves++; aircraft_probe.record.subtype = 0; aircraft_probe.record.z_pos = 17; };
	leaves.rotor_image = [](OpenTTDAircraftVehicle) noexcept { CHECK(aircraft_probe.record.z_pos == 17); aircraft_probe.record.z_pos = 25; };
	p.shadow.y_pos = -3;
	static OpenTTDSharedServices movement_world{};
	movement_world.map = &map.statics;
	movement_world.tile_type = [](uint32_t) noexcept -> uint8_t { return 0; };
	static OpenTTDMovementServices movement{};
	movement.shared = &movement_world;
	s.movement = &movement;
	leaves.copy_sprite = [](OpenTTDVehicle *, OpenTTDVehicle *) noexcept {};
	v.base_viewport = [](OpenTTDVehicle *) noexcept {};
	v.tile_links = [](OpenTTDVehicle *shell) noexcept { return &aircraft_probe.links[shell == aircraft_probe.vehicle.handle ? 0 : 1]; };
	v.map_log_x = []() noexcept { return uint32_t{6}; };
	CHECK(openttd_test_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.moves == 1 && p.record.x_pos == 1 && p.record.y_pos == 0 && p.record.progress == 1);
	p.record.subtype = 2; p.record.z_pos = 1;
	openttd_rust_aircraft_position(&leaves, p.vehicle, 1, 0, 1);
	CHECK(p.moves == 2 && p.record.z_pos == 25);
	openttd_rust_occupancy_update(&v, p.vehicle.handle, true);
	openttd_rust_occupancy_update(&v, &aircraft_probe.shadow, true);
	std::printf("aircraft rotor/order/loading phases and post-dirty motion mutation passed\n");
}

static void AircraftOwnership()
{
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	std::unique_ptr<uint64_t, decltype(&openttd_rust_airport_blocks_destroy)> airport{openttd_rust_airport_blocks_new(), openttd_rust_airport_blocks_destroy};
	CHECK(state->targetairport == 65535 && state->last_direction == 255 && state->cached_max_range_sqr == 0 && *airport == 0);
	uint8_t &position = state->pos; uint16_t &counter = state->crashed_counter; uint64_t &blocks = *airport;
	position = 255; counter = 65535; blocks = UINT64_MAX; state->cached_max_range = 65535; state->cached_max_range_sqr = UINT32_MAX;
	CHECK(state->pos == 255 && state->crashed_counter == 65535 && *airport == UINT64_MAX && state->cached_max_range == 65535 && state->cached_max_range_sqr == UINT32_MAX);
	std::printf("aircraft_airport_canonical_scalar_lifetime passed\n");
}
ABI_PROBE(AircraftDirectBoundary);
ABI_PROBE(AircraftOwnership);
ABI_PROBE(AircraftPhaseBoundary);

#pragma once
#include "rust/aircraft_ffi.h"
#include "aircraft-boundary.hpp"

#include "vehicle_tables.inc"

/* Direct callbacks reenter with the same live scalar allocation. Deletion models
 * Aircraft::PreDestructor releasing airport blocks before freeing that owner. */
static struct AircraftBoundaryProbe {
	OpenTTDVehicleServices vehicle_services;
	OpenTTDSharedServices shared_services;
	OpenTTDAircraftLeaves leaves;
	OpenTTDAircraftVehicle vehicle;
	OpenTTDVehicleTileLinks links[2];
	uint64_t blocks = UINT64_MAX;
	unsigned orders = 0, deleted = 0, ticks = 0, rotor_reads = 0, moves = 0;
	int32_t order_time = 0, x = 0, y = 0, z = 1;
	uint8_t order_kind = 0, progress = 0, subtype = 2;
	Direction direction = DIR_N;
	uint16_t maximum_speed = 0, cargo_age = 0;
	bool live = true;
} aircraft_probe;
static void AircraftDirectBoundary()
{
	auto &p = aircraft_probe;
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	CHECK(p.shared_services.is_water_tile != nullptr);
	auto &vehicle = p.vehicle_services;
	auto &shared = p.shared_services;
	auto &leaves = p.leaves;
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{0}; };
	vehicle.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{3}; };
	vehicle.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	vehicle.loading = [](OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept {};
	vehicle.visits = []() noexcept -> const OpenTTDVehicleVisits * { return nullptr; };
	leaves.process_orders = [](OpenTTDAircraftVehicle v) noexcept {
		CHECK(aircraft_probe.live);
		aircraft_probe.orders++;
		openttd_rust_aircraft_cache(&aircraft_probe.leaves, v, true);
		v.state->pos = 19;
	};
	vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return nullptr; };
	vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t property, int32_t fallback) noexcept { CHECK(property == 0x0C && fallback == 0); return int32_t{200}; };
	leaves.cargo_age_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{71}; };
	leaves.range_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{41}; };
	vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { aircraft_probe.maximum_speed = speed; };
	vehicle.write_cargo_age = [](OpenTTDVehicle *, uint16_t age) noexcept { aircraft_probe.cargo_age = age; };
	leaves.next = [](OpenTTDAircraftVehicle v) noexcept { return v; };
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), openttd_rust_aircraft_state_new(), 17};
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.orders == 1 && p.vehicle.state->pos == 19 && p.vehicle.state->cached_max_range == 41 && p.vehicle.state->cached_max_range_sqr == 1681);
	CHECK(p.maximum_speed == 2560 && p.cargo_age == 71);
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{128}; };
	leaves.station = [](uint16_t) noexcept -> const OpenTTDStation * { CHECK(aircraft_probe.live); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{7}; };
	leaves.airport_fta = [](const OpenTTDStation *s) noexcept -> const void * { return s; };
	leaves.airport_blocks = [](const OpenTTDStation *) noexcept { CHECK(aircraft_probe.live); return &aircraft_probe.blocks; };
	leaves.node = [](const void *ap, uint8_t) noexcept -> const void * { return ap; };
	leaves.fta = [](const void *) noexcept { return OpenTTDAircraftNode{nullptr, uint64_t{1} << 9, 0, 0, 0}; };
	leaves.layout_blocks = [](const void *, uint8_t) noexcept { return uint64_t{1} << 9; };
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
	p.vehicle.state->targetairport = 7;
	p.vehicle.state->crashed_counter = 9999;
	CHECK(!openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(!p.live && p.deleted == 1);
	/* A disaster has its own flags and no Aircraft state, even with subtype zero. */
	shared.map_log_x = []() noexcept { return uint32_t{6}; };
	shared.map_max_x = shared.map_max_y = []() noexcept { return uint32_t{63}; };
	vehicle.x = vehicle.y = vehicle.z = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live == false); return int32_t{0}; };
	shared.tile_height = [](uint32_t) noexcept { return uint32_t{0}; };
	vehicle.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{0, 0}; };
	vehicle.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.direction = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.altitude_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleAltitudeInputs{1, 4}; };
	uint8_t disaster_flags = 0;
	int32_t min = 0, max = 0;
	OpenTTDAircraftVehicle disaster{reinterpret_cast<OpenTTDVehicle *>(&p), nullptr, 81};
	openttd_rust_aircraft_flight_bounds(&leaves, disaster, &min, &max);
	CHECK(min == 30 && max == 270);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, false) == 1 && disaster_flags == 4);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, true) == 2 && disaster_flags == 4);
	std::printf("aircraft direct nested mutation, live block release/deletion and disaster flight identity passed\n");
}
/* Concrete gaps: native callbacks can replace order fields/rotor counters and
 * direction/progress after the pre-speed observation. This probe forbids stale
 * snapshots at those original observation phases, using the production Rust entry. */
static void AircraftPhaseBoundary()
{
	auto &p = aircraft_probe;
	p = {};
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	auto &v = p.vehicle_services;
	auto &s = p.shared_services;
	auto &leaves = p.leaves;
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), state.get(), 17};
	v.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.tick_status = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleTickStatus{7, 0}; };
	v.write_tick = [](OpenTTDVehicle *, uint8_t tick) noexcept { CHECK(tick == 8); aircraft_probe.ticks++; };
	v.running = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.write_running = [](OpenTTDVehicle *, uint8_t running) noexcept { CHECK(running == 1); };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept {
		if (u.id == 18) { aircraft_probe.order_time = 40; aircraft_probe.rotor_reads++; }
		u.id++; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{u.id}); return u;
	};
	v.status = [](OpenTTDVehicle *u) noexcept { return static_cast<uint8_t>(u == aircraft_probe.vehicle.handle ? 0 : 1); };
	v.order_time = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.rotor_reads == 1); return aircraft_probe.order_time; };
	v.write_order_time = [](OpenTTDVehicle *, int32_t time) noexcept { CHECK(time == 41); aircraft_probe.order_time = time; };
	v.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.orders++; aircraft_probe.order_kind = 3; };
	v.loading = [](OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept { aircraft_probe.order_kind = 4; };
	v.visits = []() noexcept -> const OpenTTDVehicleVisits * { return nullptr; };
	v.order_type = [](OpenTTDVehicle *) noexcept { return aircraft_probe.order_kind; };
	CHECK(openttd_rust_aircraft_tick(&leaves, p.vehicle));
	CHECK(p.ticks == 1 && p.orders == 2 && p.rotor_reads == 1 && p.order_time == 41);

	/* A speed change dirties the native widget before direction/progress are
	 * read. Their mutation yields exactly one move; a stale snapshot yields none. */
	v.subtype = [](OpenTTDVehicle *) noexcept { return aircraft_probe.subtype; };
	v.movement_observations = [](OpenTTDVehicle *shell) noexcept {
		auto &p = aircraft_probe;
		CHECK(shell == p.vehicle.handle);
		return OpenTTDVehicleMovementObservations{.position = {.subtype = &p.subtype, .z = &p.z},
			.x = &p.x, .y = &p.y, .speed = &p.maximum_speed, .direction = &p.direction};
	};
	v.status = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept {};
	v.loading = [](OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept {};
	v.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{1}; };
	p.vehicle.state->state = 14; p.vehicle.state->targetairport = 7;
	leaves.station = [](uint16_t station) noexcept { CHECK(station == 7); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.geometry = [](const OpenTTDStation *) noexcept { return OpenTTDAircraftGeometry{0, 0, 8, 8, true}; };
	leaves.airport_fta = [](const OpenTTDStation *) noexcept -> const void * { return &aircraft_probe; };
	leaves.moving = [](const void *, uint8_t) noexcept { return OpenTTDAircraftMoving{16, 0, 16, 0}; };
	v.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{aircraft_probe.x, aircraft_probe.y}; };
	s.map_log_x = []() noexcept { return uint32_t{6}; };
	s.map_max_x = s.map_max_y = []() noexcept { return uint32_t{63}; };
	s.plane_speed = []() noexcept { return uint8_t{1}; };
	v.speed_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleSpeedInputs{0, 100, 8, 0, 0}; };
	v.write_subspeed = [](OpenTTDVehicle *, uint8_t sub) noexcept { CHECK(sub == 104); };
	v.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 2); aircraft_probe.maximum_speed = speed; };
	v.start_stop_dirty = [](OpenTTDVehicle *) noexcept { aircraft_probe.direction = DIR_NE; aircraft_probe.progress = 255; };
	v.direction_progress = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.progress == 255); return OpenTTDVehicleDirectionProgress{aircraft_probe.direction, aircraft_probe.progress}; };
	v.write_progress = [](OpenTTDVehicle *, uint8_t progress) noexcept { CHECK(progress == 1); aircraft_probe.progress = progress; };
	leaves.direction_towards = [](OpenTTDAircraftVehicle, int32_t, int32_t) noexcept { return uint8_t{1}; };
	v.direction = [](OpenTTDVehicle *) noexcept { return static_cast<uint8_t>(aircraft_probe.direction); };
	v.new_position = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleNewPosition{aircraft_probe.x + 1, aircraft_probe.y, 0}; };
	v.write_tile = [](OpenTTDVehicle *, uint32_t) noexcept {};
	v.z = [](OpenTTDVehicle *) noexcept { return int32_t{1}; };
	v.write_coordinates = [](OpenTTDVehicle *, int32_t x, int32_t y, int32_t z) noexcept { CHECK(z == 1); aircraft_probe.x = x; aircraft_probe.y = y; };
	/* Both callbacks mutate canonical inputs. Subtype must be loaded after
	 * UpdatePosition, then z after the rotor callback, rather than on acquisition. */
	leaves.viewport = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.moves++; aircraft_probe.subtype = 0; aircraft_probe.z = 17; };
	leaves.rotor_image = [](OpenTTDAircraftVehicle) noexcept { CHECK(aircraft_probe.z == 17); aircraft_probe.z = 25; };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept { if (u.id != 17) return OpenTTDAircraftVehicle{nullptr, nullptr, 1048575}; u.id = 18; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}); return u; };
	v.write_x = [](OpenTTDVehicle *, int32_t x) noexcept { CHECK(x == 1); };
	v.write_y = [](OpenTTDVehicle *, int32_t y) noexcept { CHECK(y == -3); };
	v.y = [](OpenTTDVehicle *) noexcept { return int32_t{-3}; };
	v.write_z = [](OpenTTDVehicle *, int32_t z) noexcept { CHECK(z == 0); };
	static OpenTTDSharedServices movement_world{};
	movement_world.map_log_x = []() noexcept -> uint32_t { return 6; };
	movement_world.tile_type = [](uint32_t) noexcept -> uint8_t { return 0; };
	static OpenTTDMovementServices movement{};
	movement.shared = &movement_world;
	movement.pixel_location = [](int32_t x, int32_t y) noexcept -> OpenTTDPixelLocation { return {.tile = ((uint32_t(y) >> 4) << 6) + (uint32_t(x) >> 4), .kind = 0}; };
	movement.pixel_slope = [](uint32_t) noexcept -> OpenTTDFoundationSlope { return {.z = 0, .slope = SLOPE_FLAT}; };
	s.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	leaves.copy_sprite = [](OpenTTDAircraftVehicle, OpenTTDAircraftVehicle) noexcept {};
	v.base_viewport = [](OpenTTDVehicle *) noexcept {};
	v.tile_links = [](OpenTTDVehicle *shell) noexcept { return &aircraft_probe.links[shell == aircraft_probe.vehicle.handle ? 0 : 1]; };
	v.tile = [](OpenTTDVehicle *) noexcept { return uint32_t{0}; };
	v.map_log_x = []() noexcept { return uint32_t{6}; };
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.moves == 1 && p.x == 1 && p.y == 0 && p.progress == 1);
	p.subtype = 2; p.z = 1;
	v.position_observations = [](OpenTTDVehicle *shell) noexcept {
		CHECK(shell == aircraft_probe.vehicle.handle);
		return OpenTTDVehiclePositionObservations{.subtype = &aircraft_probe.subtype, .z = &aircraft_probe.z};
	};
	openttd_rust_aircraft_position(&leaves, p.vehicle, 1, 0, 1);
	CHECK(p.moves == 2 && p.z == 25);
	openttd_rust_occupancy_update(&v, p.vehicle.handle, true);
	openttd_rust_occupancy_update(&v, reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}), true);
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

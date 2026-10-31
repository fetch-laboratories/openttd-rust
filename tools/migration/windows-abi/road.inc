#include "controller-orders.hpp"
#pragma once
#include "ground_vehicle.hpp"
#include "rust/road_stop_ffi.h"
#include "rust/road_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/engine_loading_ffi.h"
#include "rust/company_ffi.h"
#include "rust/road_yapf_ffi.h"
#include "tests/effect_protocol.hpp"
#include "rust/vehicle_effect_ffi.h"

#include "vehicle_tables.inc"

struct RoadAbiProbe {
	OpenTTDRoadState *state = openttd_rust_road_new();
	GroundStateOwner ground;
	uint16_t speed = 0;
	std::unique_ptr<OpenTTDRoadStopState, decltype(&openttd_rust_road_stop_destroy)> stop{openttd_rust_road_stop_new(), openttd_rust_road_stop_destroy};
	OpenTTDRoadStopEntries entries{};
	const OpenTTDRoadLeaves *leaves = nullptr;
	OpenTTDRoadEntry entry{.ground = ground.state, .acceleration_model = 0, .road_side = 0, .queue = false, .map_log_x = 8};
	uint32_t dest = 0, nested = 0, destroyed = 0, departures = 0, progress = 255;
	uint8_t tick = 0, running = 0, status = 0;
	uint32_t cache_properties = 0, cargo_ages = 0, length_changes = 0, viewports = 0, turns = 0, next_reads = 0;
	uint32_t progress_writes = 0, visual_stage = 0;
	uint16_t post_effect_max = 80;
	bool visual_eligible = true, reset_status = false;
	bool fallback_cache = false, engine_replaced = false;
	bool hidden_viewport_tail = false;
	int32_t order_time = 0;
};
static RoadAbiProbe *road_abi_probe;
static int road_abi_second, road_abi_hidden, road_abi_engine, road_abi_new_engine;
static int road_abi_vehicle; ///< Its address is the opaque vehicle handle.
static OpenTTDVehicle *RoadAbiHandle() { return reinterpret_cast<OpenTTDVehicle *>(&road_abi_vehicle); }
static int road_abi_stop; ///< Its address is the opaque road stop shell.
/* The road controller calls the Rust road stop owner directly; these native
 * shell services resolve the probe's single stop at tile 100. */
static const OpenTTDRoadStopServices road_abi_stops{
	.vehicle = nullptr,
	.ground = nullptr,
	.primary = [](uint32_t tile, RoadStopType) noexcept { CHECK(tile == 100); return reinterpret_cast<RoadStop *>(&road_abi_stop); },
	.tile = [](const RoadStop *) noexcept -> uint32_t { return 100; },
	.state = [](const RoadStop *) noexcept { return road_abi_probe->stop.get(); },
	.drive_read = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadStopDriveRead {
		CHECK(u == RoadAbiHandle());
		++road_abi_probe->departures;
		return {.total_length = 8, .direction = 1};
	},
};
static OpenTTDRoadMaxSpeedRead RoadAbiMaxSpeed()
{
	return {.next = nullptr, .next_state = nullptr, .tile = 100, .max_track_speed = 80, .order_max_speed = 65535, .direction = 1, .status = 0};
}
static void RoadBoundary()
{
	RoadAbiProbe probe;
	road_abi_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	static uint8_t cache_visual_byte = 0;
	OpenTTDVisualServices cache_visual{
		.vehicle = &t.vehicle, .shared = &t.shared,
		.engine = [](OpenTTDVehicle *) noexcept -> OpenTTDVisualEngine { return {.engine = nullptr, .cache = &cache_visual_byte, .id = 0, .kind = VEH_ROAD, .visual = 0x40, .callback = false}; },
	};
	auto previous_visual = openttd_rust_visual_bind(&cache_visual);
	probe.leaves = &t.leaves;
	OpenTTDVehicle *v = RoadAbiHandle();

	/* An articulated part only stores its counters. */
	t.vehicle.write_counters = [](OpenTTDVehicle *u, uint8_t tick, uint8_t running, int32_t order_time) noexcept {
		CHECK(u == RoadAbiHandle());
		auto &p = *road_abi_probe;
		p.tick = tick;
		p.running = running;
		p.order_time = order_time;
		if (p.reset_status) p.status = 0;
	};
	t.vehicle.write_tick = [](OpenTTDVehicle *u, uint8_t tick) noexcept { CHECK(u == RoadAbiHandle()); road_abi_probe->tick = tick; };
	const OpenTTDRoadTickRead part{.tick = 255, .front = false, .movement = {}, .order_time = nullptr, .running = nullptr, .status = nullptr};
	CHECK(openttd_rust_road_tick(v, probe.state, &part, &t.leaves, nullptr));
	CHECK(probe.tick == 0 && probe.running == 0 && probe.order_time == 0 && world.draws == 0);

	/* A direct callback changes canonical state before Rust reobserves it. */
	t.physics.resolve = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundOwner { return {road_abi_probe->ground.state, {}, road_abi_probe->state, VEH_ROAD}; };
	t.physics_vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	t.vehicle.subtype = [](OpenTTDVehicle *) noexcept -> uint8_t { return 1; };
	probe.ground.state->flags = 3;
	t.physics.base_crash = [](OpenTTDVehicle *u, bool flooded) noexcept -> uint32_t {
		CHECK(u == RoadAbiHandle() && !flooded);
		openttd_rust_road_set(road_abi_probe->state, 0, 64);
		CHECK(road_abi_probe->ground.state->flags == 0);
		return 7;
	};
	t.vehicle.tile = [](OpenTTDVehicle *u) noexcept -> uint32_t { CHECK(u == RoadAbiHandle()); return 100; };
	t.shared.road_stop_type = [](uint32_t tile) noexcept { CHECK(tile == 100); return RoadStopType::Bus; };
	t.shared.road_stops = []() noexcept { return &road_abi_stops; };
	t.shared.is_bay_stop = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	probe.stop->entries = &probe.entries;
	probe.entries.east.occupied = 8;
	CHECK(openttd_rust_road_crash(v, probe.state, false, &t.leaves) == 8);
	CHECK(probe.departures == 1 && openttd_rust_road_get(probe.state, 5) == 1 && probe.entries.east.occupied == 0);

	/* Entries nested in a NewGRF property callback see and change the same owner. */
	t.leaves.trackdir_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadTrackdirRead { return {.tile = 100, .direction = 1, .status = 0}; };
	t.leaves.is_bay_stop = [](uint32_t) noexcept { return false; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { road_abi_probe->dest = tile; };
	t.ground.cache_begin = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.engine_for_length = [](OpenTTDVehicle *) noexcept -> OpenTTDEngineForLength {
		auto &p = *road_abi_probe;
		CHECK(p.cache_properties == p.cargo_ages);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(p.engine_replaced ? &road_abi_new_engine : &road_abi_engine), .grf_version = 8};
	};
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * {
		auto &p = *road_abi_probe;
		CHECK(p.cargo_ages == (u == RoadAbiHandle() ? 1U : 2U));
		return p.fallback_cache && u == RoadAbiHandle() ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : nullptr;
	};
	t.vehicle.engine_id = [](OpenTTDVehicle *u) noexcept -> uint16_t {
		CHECK(u == RoadAbiHandle() && road_abi_probe->cargo_ages == 1);
		return 9; // The first part's callbacks changed the head engine.
	};
	t.leaves.shorten_factor = [](const OpenTTDEngine *e) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && road_abi_probe->cache_properties == 1);
		return 2; // Read through the engine captured before the property callback.
	};
	t.ground.cached_length = [](OpenTTDVehicle *) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties > road_abi_probe->cargo_ages);
		return 8;
	};
	t.vehicle.length_changed = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->length_changes; };
	static OpenTTDEngineCapabilitiesServices capabilities{};
	static OpenTTDVehicleResolverServices resolver{};
	static OpenTTDEngineLoadingServices loading{};
	resolver.capabilities = &capabilities;
	capabilities.grf = [](const OpenTTDEngine *e) noexcept -> const GRFFile * { CHECK(e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_new_engine)); return nullptr; };
	capabilities.loading = []() noexcept -> const OpenTTDEngineLoadingServices * { return &loading; };
	loading.unknown_refit = [](uint32_t grfid, uint16_t callback, uint16_t result) noexcept { CHECK(grfid == 0 && callback == 0x11 && result == 9); };
	t.leaves.default_max_speed = [](OpenTTDVehicle *) noexcept -> uint16_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties == 3);
		return 99;
	};
	t.ground.write_first_engine = [](OpenTTDVehicle *, uint16_t engine) noexcept { CHECK(engine == 0xFFFF || (road_abi_probe->fallback_cache && engine == 9)); };
	t.vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	t.vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *u, uint8_t property, int32_t fallback) noexcept -> int32_t {
		auto &p = *road_abi_probe;
		if (property == 0x22) return fallback;
		++p.cache_properties;
		if (p.fallback_cache) {
			p.engine_replaced = true;
			return property == 0x23 ? (u == RoadAbiHandle() ? 0xFFFF : 9) : 0;
		}
		const OpenTTDRoadMaxSpeedRead read = RoadAbiMaxSpeed();
		CHECK(openttd_rust_road_max_speed(u, p.state, p.entry.acceleration_model, &read, p.leaves) == 80);
		CHECK(openttd_rust_road_trackdir(u, p.state, p.leaves) == 0);
		++p.nested;
		openttd_rust_road_set_dest(u, p.state, 142 + p.nested, p.dest, p.leaves);
		CHECK(openttd_rust_road_path_size(p.state) == 0);
		openttd_rust_road_path_push(p.state, {10, 10815});
		return property == 0x23 ? 0 : property == 0x15 ? 20 : fallback;
	};
	t.ground.write_length = [](OpenTTDVehicle *u, uint8_t length, OpenTTDVehicle *, uint16_t total) noexcept {
		CHECK(road_abi_probe->fallback_cache ? (u == RoadAbiHandle() ? length == 6 && total == 6 : length == 1 && total == 7) : length == 8 && total == 8);
	};
	t.leaves.cargo_age_default = [](OpenTTDVehicle *) noexcept { return uint16_t{0}; };
	t.vehicle.write_cargo_age = [](OpenTTDVehicle *, uint16_t) noexcept { ++road_abi_probe->cargo_ages; };
	t.vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == (road_abi_probe->fallback_cache ? 99 : 80)); };
	openttd_rust_road_update_cache(v, probe.state, false, &t.leaves);
	CHECK(probe.nested == 2 && probe.dest == 144 && openttd_rust_road_path_get(probe.state, 0).tile == 10815);

	/* Failed/invalid properties consume late fallback reads and the post-callback link.
	 * The first, !same_length run rejects conditional reads; a changed engine
	 * still leaves fallback/error reads attached to the captured engine context. */
	probe.fallback_cache = true;
	probe.cache_properties = probe.cargo_ages = 0;
	openttd_rust_road_update_cache(v, probe.state, true, &t.leaves);
	CHECK(probe.cargo_ages == 2 && probe.length_changes == 2);

	/* Running-cost property callbacks can replace the vehicle's engine. Price retains the original engine. */
	probe.engine_replaced = false;
	t.vehicle.engine = [](OpenTTDVehicle *u) noexcept -> const OpenTTDEngine * {
		CHECK(u == RoadAbiHandle() && !road_abi_probe->engine_replaced);
		return reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine);
	};
	capabilities.road_price = [](const OpenTTDEngine *) noexcept -> Price { return static_cast<Price>(1); };
	capabilities.road_running = [](const OpenTTDEngine *) noexcept -> uint8_t { return 3; };
	t.vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	t.vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *u, uint8_t property, int32_t value) noexcept -> int32_t {
		CHECK(u == RoadAbiHandle() && property == 9 && value == 3);
		road_abi_probe->engine_replaced = true;
		return 4;
	};
	capabilities.price_shift = [](const OpenTTDEngine *e, uint32_t price) noexcept -> int32_t {
		CHECK(road_abi_probe->engine_replaced && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && price == 1);
		return -2;
	};
	const int64_t old_price = openttd_rust_economy_prices()[1];
	openttd_rust_economy_prices()[1] = 123;
	CHECK(openttd_rust_road_running_cost(v, probe.state, &t.leaves) == 123);
	openttd_rust_economy_prices()[1] = old_price;

	/* A non-station order must not read its destination at the bay stopping frame. */
	openttd_rust_road_set(probe.state, 0, 32);
	openttd_rust_road_set(probe.state, 1, 20);
	openttd_rust_road_set(probe.state, 6, 1); // Skip nearby-vehicle traversal.
	t.leaves.step = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadStep {
		uint16_t rd = openttd_rust_road_drive_entry(0, 32, 21);
		CHECK((rd & 0xC0) == 0 && (rd & 15) < 15);
		return {.tile = 100, .x = (rd & 15) + 1, .y = (rd >> 8) & 15, .z = 0, .cur_speed = 0, .direction = 1, .length = 8, .front = true, .tram = false};
	};
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr, .next_ground = nullptr}; };
	t.shared.station_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == 100); return 55; };
	t.leaves.is_drive_through = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	probe.stop->status = 0xFF; // Entering the bay frees its entrance.
	t.vehicle.write_last_station = [](OpenTTDVehicle *, uint16_t station) noexcept { CHECK(station == 55); };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.leaves.is_bay_stop = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	t.vehicle.play_vehicle_sound = [](OpenTTDVehicle *, VehicleSoundEvent event) noexcept { CHECK(event == 1); return true; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	static OpenTTDFixtureMap map{6, 4096};
	for (uint32_t tile = 0; tile < map.size; ++tile) map.base[tile].type = 9 << 4;
	static OpenTTDSharedServices movement_world{};
	movement_world.map = &map.statics;
	movement_world.tile_type = [](uint32_t) noexcept -> uint8_t { return 9; };
	static OpenTTDMovementServices movement{};
	movement.shared = &movement_world;
	movement.resolve = [](OpenTTDVehicle *) noexcept -> OpenTTDMovementVehicle { return {.train = {}, .road = road_abi_probe->state, .ship = nullptr, .kind = static_cast<VehicleType>(1)}; };
	movement.is_tunnel = [](uint32_t) noexcept { return true; };
	movement.pose = [](OpenTTDVehicle *) noexcept -> OpenTTDMovementPose { return {.z = 10, .direction = 1}; };
	t.shared.movement = &movement;
	t.shared.map = &map.statics;
	t.leaves.after_enter_tile = [](OpenTTDVehicle *, uint8_t flags) noexcept -> OpenTTDRoadEntered { CHECK(flags == 4); return {.tile = 100, .cur_speed = 0, .flags = flags, .direction = 1, .order_type = 0}; };
	t.vehicle.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 0); };
	CHECK(!openttd_rust_road_individual(v, probe.state, nullptr, nullptr, &probe.entry, &t.leaves, nullptr));
	CHECK(openttd_rust_road_get(probe.state, 0) == 36 && probe.stop->status == 0x7F);
	probe.stop->entries = nullptr;
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 6, 0);

	/* Visual eligibility precedes lookup; effect reentry changes the later speed inputs. */
	static std::unique_ptr<ControllerOrders> road_orders;
	road_orders = std::make_unique<ControllerOrders>(&t.vehicle);
	t.vehicle.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &road_orders->services; };
	road_orders->services.vehicle = [](void *shell) noexcept { CHECK(shell == RoadAbiHandle()); return road_orders->vehicle.get(); };
	road_orders->services.consist = [](void *) noexcept { return &road_orders->consist; };
	road_orders->services.vehicle_type = [](void *) noexcept { return uint64_t{1}; };
	road_orders->services.station_location = [](void *, uint16_t) noexcept { return uint64_t{100}; };
	road_orders->services.set_destination = [](void *shell, uint32_t destination) noexcept {
		CHECK(destination == 100);
		auto *u = static_cast<OpenTTDVehicle *>(shell);
		auto &p = *road_abi_probe;
		openttd_rust_road_set_dest(u, p.state, destination, p.dest, p.leaves);
	};
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept { return road_orders->vehicle->current.type; };
	t.vehicle.visits = []() noexcept -> const OpenTTDVehicleVisits * { return nullptr; };
	OpenTTDEffectLeaves emission_leaves = EffectTestLeaves();
	auto *emission_owner = openttd_rust_effect_new();
	static OpenTTDEffectState *current_emission_owner;
	current_emission_owner = emission_owner;
	OpenTTDEffectCreationServices creation{
		.effect = &emission_leaves, .shared = &t.shared,
		.can_allocate = [](size_t) noexcept { return true; },
		.allocate = []() noexcept -> OpenTTDEffectAllocation { return {effect_test_world, current_emission_owner}; },
		.place = [](void *shell, int32_t x, int32_t y, int32_t z, uint8_t type) noexcept { static_cast<EffectTestWorld *>(shell)->view = {x, y, z, 0, 0, 0, type, 0}; },
		.delta = [](void *) noexcept {}, .unclickable = [](void *) noexcept {},
		.position = [](const OpenTTDVehicle *) noexcept -> OpenTTDEffectPosition { return {}; },
	};
	auto previous_creation = openttd_rust_effect_bind(&creation);
	auto emission_vehicle = t.vehicle;
	emission_vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	emission_vehicle.play_vehicle_sound = [](OpenTTDVehicle *u, VehicleSoundEvent event) noexcept {
		auto &p = *road_abi_probe;
		CHECK(p.visual_stage == 1 && p.visual_eligible && event == VSE_VISUAL_EFFECT); p.visual_stage = 3;
		openttd_rust_road_set(p.state, 3, 1);
		openttd_rust_road_set_dest(u, p.state, 100, p.dest, p.leaves);
		p.post_effect_max = 40; ++p.nested; return false;
	};
	auto emission_shared = t.shared;
	emission_shared.is_bridge = emission_shared.bridge_above = [](uint32_t) noexcept { return false; };
	static uint8_t emission_cache = 0x11;
	OpenTTDVisualServices emission{
		.vehicle = &emission_vehicle, .shared = &emission_shared,
		.gate = [](OpenTTDVehicle *, OpenTTDVisualGate *gate) noexcept {
			auto &p = *road_abi_probe; p.visual_stage = 1;
			*gate = {.speed = 2, .status = 0, .smoke = uint8_t(p.visual_eligible ? 2 : 0), .primary = true};
		},
		.show = [](OpenTTDVehicle *) noexcept -> OpenTTDVisualShow { return {2, VEH_ROAD, 2}; },
		.part = [](OpenTTDVehicle *) noexcept -> OpenTTDVisualPart { return {.tile = 100, .kind = VEH_ROAD, .status = 0, .tick = 0, .cache = emission_cache}; },
		.offset = [](OpenTTDVehicle *) noexcept -> OpenTTDVisualOffset { return {DIR_NE, 8}; },
		.depot = [](uint32_t) noexcept { return false; }, .tunnel = [](uint32_t) noexcept { return false; },
	};
	openttd_rust_visual_bind(&emission);
	t.leaves.max_speed_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadMaxSpeedRead {
		auto &p = *road_abi_probe; CHECK(p.visual_eligible && p.visual_stage == 1); return RoadAbiMaxSpeed();
	};
	t.leaves.speed_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadSpeedRead {
		auto &p = *road_abi_probe;
		CHECK(p.visual_stage == (p.visual_eligible ? 3U : 1U));
		p.visual_stage = 4;
		auto read = RoadAbiMaxSpeed(); read.max_track_speed = p.post_effect_max;
		return {.max = read, .x = 0, .y = 0, .z = 0, .length = 8, .tram = false};
	};
	// Ground speed now executes instead of being stubbed: original model gives
	// speed 2/distance 1; realistic kickoff gives speed 4/distance 3.
	t.physics.speed_input = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundSpeedInput { CHECK(road_abi_probe->visual_stage == 4); return {0, 0}; };
	t.physics.acceleration_input = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundAccelerationInput { return {0, 0, 0, RoadType{0}}; };
	probe.ground.state->cache.cached_weight = 1;
	t.physics_vehicle.write_subspeed = [](OpenTTDVehicle *, uint8_t value) noexcept { CHECK(value == (road_abi_probe->entry.acceleration_model == 1 ? 2 : 0)); };
	t.physics_vehicle.write_speed = [](OpenTTDVehicle *, uint16_t value) noexcept { CHECK(value == (road_abi_probe->entry.acceleration_model == 1 ? 4 : 2)); road_abi_probe->speed = value; };
	t.physics_vehicle.speed = t.vehicle.speed = [](OpenTTDVehicle *) noexcept { return road_abi_probe->speed; };
	t.physics_vehicle.progress = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.physics_vehicle.write_progress = [](OpenTTDVehicle *, uint8_t value) noexcept { CHECK(value == 0); road_abi_probe->progress = value; };
	t.physics_vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.status = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		CHECK(part == RoadAbiHandle() && front == part);
		++road_abi_probe->viewports;
		road_abi_probe->progress = 7;
		++road_abi_probe->next_reads;
		return {.next = nullptr, .final_progress = static_cast<uint8_t>(road_abi_probe->progress)};
	};
	t.leaves.crash_part = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadCrashPart { return {.next = nullptr, .next_state = nullptr, .z = 0, .crossing = false}; };
	OpenTTDRoadTickRead front{.tick = 255, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	// Fresh scalar loads and the post-write crashed-bit reset precede controller entry.
	probe.order_time = INT32_MAX;
	probe.running = UINT8_MAX;
	probe.status = 128;
	probe.reset_status = true;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	probe.reset_status = false;
	CHECK(probe.tick == 0 && probe.status == 0);
	CHECK(probe.nested == 3 && probe.dest == 100 && probe.order_time == INT32_MIN && probe.running == 0 && probe.progress == 7 && probe.next_reads == 1 && world.draws == 0);

	/* Stopped parts preserve running ticks; later status resets must survive the entry. */
	const auto breakdown_probe = _vehicle_maintenance_probe.breakdown_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = road_abi_probe->status};
	};
	const auto last_speed_dirty = t.physics_vehicle.start_stop_dirty;
	probe.ground.state->cache.last_speed = UINT16_MAX;
	t.physics_vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {
		road_abi_probe->status = 0;
	};
	probe.status = 2;
	probe.running = UINT8_MAX;
	probe.order_time = INT32_MAX;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.running == UINT8_MAX && probe.order_time == INT32_MIN && probe.status == 0);
	_vehicle_maintenance_probe.breakdown_probe = breakdown_probe;
	t.physics_vehicle.start_stop_dirty = last_speed_dirty;

	/* Skip effects with no lookup; realistic UpdateSpeed still observes post-effect inputs. */
	probe.visual_eligible = false; probe.entry.acceleration_model = 1;
	front.movement = probe.entry;
	/* Hidden vehicles skip viewport callbacks, then use the direct canonical link. */
	t.vehicle.status = [](OpenTTDVehicle *) noexcept -> uint8_t { return 1; };
	t.vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { CHECK(road_abi_probe->viewports == 1); ++road_abi_probe->next_reads; return nullptr; };
	t.vehicle.progress = [](OpenTTDVehicle *) noexcept -> uint8_t { CHECK(road_abi_probe->viewports == 1 && road_abi_probe->next_reads == 2); return static_cast<uint8_t>(road_abi_probe->progress); };
	t.vehicle.write_progress = [](OpenTTDVehicle *, uint8_t progress) noexcept { CHECK(progress == 3); road_abi_probe->progress = progress; };
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.viewports == 1 && probe.next_reads == 2 && probe.progress == 3);
	probe.visual_eligible = true; probe.entry.acceleration_model = 0;
	front.movement = probe.entry;

	/* Viewport reentry can add a later visible part. Its progress mutation is
	 * observed only after that viewport and any hidden final part's Next read. */

	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle());
		CHECK((p.viewports == 0 && part == front) || (p.viewports == 1 && part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second)));
		++p.viewports;
		p.progress = part == front ? 7 : 0;
		openttd_rust_road_set_dest(front, p.state, 200 + p.viewports, p.dest, p.leaves);
		++p.next_reads;
		OpenTTDVehicle *next = part == front ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : p.hidden_viewport_tail ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) : nullptr;
		return {.next = next, .final_progress = next == nullptr ? uint8_t(p.progress) : uint8_t(99)};
	};
	t.vehicle.status = [](OpenTTDVehicle *part) noexcept -> uint8_t {
		if (part == RoadAbiHandle()) return 0;
		bool hidden = part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden);
		CHECK(hidden ? road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 : part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) && road_abi_probe->viewports == 1);
		return hidden ? 1 : 0;
	};
	t.vehicle.next = [](OpenTTDVehicle *part) noexcept -> OpenTTDVehicle * {
		CHECK(part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2);
		++road_abi_probe->next_reads;
		return nullptr;
	};
	t.vehicle.progress = [](OpenTTDVehicle *front) noexcept -> uint8_t {
		CHECK(front == RoadAbiHandle() && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 && road_abi_probe->next_reads == 3);
		return road_abi_probe->progress;
	};
	t.vehicle.write_progress = [](OpenTTDVehicle *front, uint8_t progress) noexcept {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle() && p.viewports == 2 && p.next_reads == (p.hidden_viewport_tail ? 3U : 2U) && progress == 1);
		++p.progress_writes;
		p.progress = progress;
	};
	for (bool hidden : {false, true}) {
		probe.hidden_viewport_tail = hidden;
		probe.viewports = probe.next_reads = probe.progress_writes = 0;
		CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
		CHECK(probe.viewports == 2 && probe.next_reads == (hidden ? 3U : 2U) && probe.progress_writes == 1 && probe.progress == 1 && probe.dest == 202);
	}

	/* Crash animation reads the next link after the turn/viewport callback. */
	openttd_rust_road_set(probe.state, 5, 0);
	probe.status = 128;
	t.vehicle.direction = [](OpenTTDVehicle *u) noexcept -> uint8_t { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 0); return 1; };
	t.ground.turn = [](OpenTTDVehicle *u, uint8_t direction) noexcept { CHECK(u == RoadAbiHandle() && direction < 8); ++road_abi_probe->turns; };
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 1); return nullptr; };
	const OpenTTDRoadTickRead animation{.tick = 7, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	CHECK(openttd_rust_road_tick(v, probe.state, &animation, &t.leaves, nullptr));
	CHECK(probe.turns == 1 && world.draws == 1);

	/* The final-part deletion returns its copied alive result only. */
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 5, 2219);
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr, .next_ground = nullptr}; };
	t.vehicle.detach_last = [](OpenTTDVehicle *previous, OpenTTDVehicle *last, OpenTTDVehicle *first) noexcept {
		CHECK(previous == RoadAbiHandle() && last == previous && first == previous);
	};
	t.vehicle.destroy = [](OpenTTDVehicle *u) noexcept {
		CHECK(u == RoadAbiHandle());
		openttd_rust_road_destroy(road_abi_probe->state);
		road_abi_probe->state = nullptr;
		++road_abi_probe->destroyed;
	};
	probe.status = 128;
	const OpenTTDRoadTickRead crashed{.tick = 31, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	CHECK(!openttd_rust_road_tick(v, probe.state, &crashed, &t.leaves, nullptr));
	CHECK(probe.state == nullptr && probe.destroyed == 1 && world.draws == 1);
	openttd_rust_visual_bind(previous_visual);
	openttd_rust_effect_bind(previous_creation);
	openttd_rust_effect_destroy(emission_owner);
	std::printf("road_native_typed_reentry_path_and_owner_destruction passed\n");
}

/* Moving and partly entered consists cannot use IsChainInDepot's service path,
 * but FindClosestRoadDepot still selects their current depot at distance zero. */
struct RoadServiceProbe {
	OpenTTDRoadState *head = openttd_rust_road_new();
	OpenTTDRoadState *tail = openttd_rust_road_new();
	uint32_t tile = 3091, dest = 0;
	uint16_t speed = 5;
	bool depot_tile = true;
	uint32_t depot_orders = 0, services = 0, searches = 0;
};
static RoadServiceProbe *road_service_probe;
static int road_service_vehicles[2]; ///< Their addresses are the opaque head and tail handles.
static OpenTTDVehicle *RoadServiceHandle(int part) { return reinterpret_cast<OpenTTDVehicle *>(&road_service_vehicles[part]); }
static void RoadServiceBoundary()
{
	RoadServiceProbe probe;
	road_service_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	t.leaves.max_depot_penalty = []() noexcept -> uint32_t { return 300; };
	/* Direct maintenance: aged out, no reliability decay, no breakdown, service due. */
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.max_economy_date = OpenTTDEconomyDate{0};
	maintenance.economy_age = [](OpenTTDVehicle *) noexcept { return OpenTTDEconomyDate{0}; };
	t.vehicle.day = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.vehicle.write_day = [](OpenTTDVehicle *, uint8_t day) noexcept { CHECK(day == 1); };
	maintenance.check_breakdown = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceCheckBreakdown r{};
		r.no_service = true;
		return r;
	};
	maintenance.breakdown_guard = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceBreakdownGuard r{};
		r.counter = 1;
		return r;
	};
	maintenance.automatic = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceAutomatic{}; };
	t.vehicle.status = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	maintenance.interval = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceInterval r{};
		r.now = OpenTTDEconomyDate{1};
		return r;
	};
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept {};
	maintenance.reset = [](OpenTTDVehicle *) noexcept { ++road_service_probe->services; return OpenTTDVehicleMaintenanceReset{}; };
	t.vehicle.write_economy_service = t.vehicle.write_calendar_service = [](OpenTTDVehicle *, int32_t) noexcept {};
	t.vehicle.write_breakdowns = [](OpenTTDVehicle *, uint8_t) noexcept {};
	t.vehicle.write_reliability = [](OpenTTDVehicle *, uint16_t) noexcept {};
	maintenance.chance_write = [](OpenTTDVehicle *, uint8_t) noexcept {};
	t.vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	t.leaves.service_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadServiceRead {
		auto &p = *road_service_probe;
		return {.tile = p.tile, .servint = 1, .cur_speed = p.speed};
	};
	t.leaves.is_road_depot_tile = [](uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); return road_service_probe->depot_tile; };
	t.leaves.link_tile = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLinkTile {
		auto &p = *road_service_probe;
		bool head = u == RoadServiceHandle(0);
		return {.link = {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? p.tail : nullptr, .next_ground = nullptr}, .tile = p.tile};
	};
	t.leaves.link = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLink {
		bool head = u == RoadServiceHandle(0);
		return {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? road_service_probe->tail : nullptr, .next_ground = nullptr};
	};
	t.leaves.find_depot = [](OpenTTDVehicle *, int32_t penalty) noexcept -> OpenTTDRoadDepotResult {
		CHECK(penalty == 300);
		++road_service_probe->searches;
		return {.tile = UINT32_MAX, .length = UINT32_MAX};
	};
	t.shared.depot_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == road_service_probe->tile); return 55; };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.ground.go_to_depot_service = [](OpenTTDVehicle *, uint16_t depot) noexcept { CHECK(depot == 55); ++road_service_probe->depot_orders; };
	t.vehicle.dest_tile = [](OpenTTDVehicle *) noexcept -> uint32_t { return road_service_probe->dest; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); road_service_probe->dest = tile; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.check_orders = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	auto day = [&]() { openttd_rust_road_economy_day(RoadServiceHandle(0), probe.head, true, &t.leaves); };
	for (bool moving : {true, false}) {
		probe.speed = moving ? 5 : 0;
		probe.dest = 42;
		openttd_rust_road_set(probe.head, 0, 254);
		openttd_rust_road_set(probe.tail, 0, moving ? 254 : 1);
		openttd_rust_road_path_push(probe.head, {10, 10815});
		day();
		CHECK(probe.dest == probe.tile && openttd_rust_road_path_size(probe.head) == 0);
	}
	CHECK(probe.depot_orders == 2 && probe.services == 0 && world.draws == 0);
	openttd_rust_road_set(probe.tail, 0, 254);
	day();
	CHECK(probe.services == 1 && probe.depot_orders == 2);
	probe.depot_tile = false;
	day();
	CHECK(probe.searches == 1);
	openttd_rust_road_destroy(probe.head);
	openttd_rust_road_destroy(probe.tail);
	std::printf("road_service_moving_partial_parked_and_search_branches passed\n");
}
ABI_PROBE(RoadBoundary);
ABI_PROBE(RoadServiceBoundary);

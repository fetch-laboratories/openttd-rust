#include "controller-orders.hpp"
#pragma once
#include "ground_vehicle.hpp"
#include "rust/road_stop_ffi.h"
#include "rust/road_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/engine_loading_ffi.h"
#include "rust/company_ffi.h"
#include "rust/fleet_ffi.h"
#include "rust/settings_table_ffi.h"
#include "rust/movement_ffi.h"
#include "rust/road_yapf_ffi.h"
#include "tests/effect_protocol.hpp"
#include "rust/vehicle_effect_ffi.h"

#include "vehicle_tables.inc"

static OpenTTDVehicle road_abi_vehicle{};
struct RoadAbiProbe {
	OpenTTDRoadState *state = openttd_rust_road_new();
	GroundStateOwner ground, tail_ground;
	uint16_t &speed = road_abi_vehicle.cur_speed;
	std::unique_ptr<OpenTTDRoadStopState, decltype(&openttd_rust_road_stop_destroy)> stop{openttd_rust_road_stop_new(), openttd_rust_road_stop_destroy};
	OpenTTDRoadStopEntries entries{};
	const OpenTTDRoadLeaves *leaves = nullptr;
	OpenTTDRoadEntry entry{.ground = ground.state, .acceleration_model = 0, .road_side = 0, .queue = false, .map_log_x = 8};
	uint32_t &dest = road_abi_vehicle.dest_tile;
	uint8_t &progress = road_abi_vehicle.progress;
	uint32_t nested = 0, destroyed = 0;
	uint8_t &tick = road_abi_vehicle.tick_counter, &running = road_abi_vehicle.running_ticks, &status = road_abi_vehicle.vehstatus;
	uint32_t cache_properties = 0, cargo_ages = 0, length_changes = 0, viewports = 0, turns = 0;
	uint32_t visual_stage = 0;
	uint16_t post_effect_max = 80;
	bool visual_eligible = true, reset_status = false;
	bool fallback_cache = false, engine_replaced = false;
	bool hidden_viewport_tail = false;
	OpenTTDConsistState consist{};
	OpenTTDVehicleOrderState orders{};
	int32_t &order_time = consist.current_order_time;
};
static RoadAbiProbe *road_abi_probe;
static OpenTTDVehicle road_abi_second{}, road_abi_hidden{};
static int road_abi_engine, road_abi_new_engine;
static OpenTTDVehicle *RoadAbiHandle() { road_abi_vehicle.shell = reinterpret_cast<OpenTTDVehicleShell *>(&road_abi_vehicle); return &road_abi_vehicle; }
static int road_abi_stop; ///< Its address is the opaque road stop shell.
/* The road controller calls the Rust road stop owner directly; these native
 * shell services resolve the probe's single stop at tile 100. */
static const OpenTTDRoadStopServices road_abi_stops{
	.vehicle = nullptr,
	.ground = nullptr,
	.primary = [](uint32_t tile, RoadStopType) noexcept { CHECK(tile == 100); return reinterpret_cast<RoadStop *>(&road_abi_stop); },
	.tile = [](const RoadStop *) noexcept -> uint32_t { return 100; },
	.state = [](const RoadStop *) noexcept { return road_abi_probe->stop.get(); },
};
static void RoadBoundary()
{
	RoadAbiProbe probe;
	road_abi_vehicle = {}; road_abi_vehicle.progress = 255; road_abi_vehicle.tile = 100; road_abi_vehicle.direction = 1; road_abi_vehicle.type = VEH_ROAD; road_abi_vehicle.subtype = 1;
	road_abi_second.shell = reinterpret_cast<OpenTTDVehicleShell *>(&road_abi_second); road_abi_second.type = VEH_ROAD; road_abi_second.subtype = 1;
	road_abi_hidden.shell = reinterpret_cast<OpenTTDVehicleShell *>(&road_abi_hidden); road_abi_hidden.type = VEH_ROAD; road_abi_hidden.subtype = 1;
	road_abi_probe = &probe;
	road_abi_vehicle.road = probe.state; road_abi_vehicle.ground = probe.ground.state;
	road_abi_vehicle.orders = &probe.orders; road_abi_vehicle.consist = &probe.consist;
	road_abi_vehicle.first = &road_abi_vehicle;
	probe.orders.current.max_speed = UINT16_MAX;
	probe.ground.state->cache.cached_max_track_speed = 80;
	probe.ground.state->cache.cached_veh_length = 8;
	road_abi_second.road = probe.state; road_abi_second.ground = probe.tail_ground.state; road_abi_second.orders = &probe.orders;
	road_abi_hidden.road = probe.state; road_abi_hidden.ground = probe.ground.state;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	t.shared.is_level_crossing = [](uint32_t) noexcept { return false; };
	static uint8_t cache_visual_byte = 0;
	OpenTTDVisualServices cache_visual{
		.vehicle = &t.vehicle, .shared = &t.shared,
		.engine = [](OpenTTDVehicle *) noexcept -> OpenTTDVisualEngine { return {.engine = nullptr, .kind = VEH_ROAD, .visual = 0x40, .callback = false}; },
	};
	road_abi_vehicle.visual = road_abi_second.visual = road_abi_hidden.visual = &cache_visual_byte;
	auto previous_visual = openttd_rust_visual_bind(&cache_visual);
	probe.leaves = &t.leaves;
	OpenTTDVehicle *v = RoadAbiHandle();

	/* An articulated part only stores its counters. */

	road_abi_vehicle.subtype = 0; road_abi_vehicle.tick_counter = 255;
	const OpenTTDRoadEntry part{};
	CHECK(openttd_rust_road_tick(v, probe.state, &part, &t.leaves, nullptr));
	CHECK(probe.tick == 0 && probe.running == 0 && probe.order_time == 0 && world.draws == 0);
	road_abi_vehicle.subtype = 1;

	/* A direct callback changes canonical state before Rust reobserves it. */
	t.physics.resolve = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundOwner { return {road_abi_probe->ground.state, {}, road_abi_probe->state, VEH_ROAD}; };
	probe.ground.state->flags = 3;
	t.physics.base_crash = [](OpenTTDVehicle *u, bool flooded) noexcept -> uint32_t {
		CHECK(u == RoadAbiHandle() && !flooded);
		openttd_rust_road_set(road_abi_probe->state, 0, 64);
		CHECK(road_abi_probe->ground.state->flags == 0);
		return 7;
	};
	t.shared.road_stop_type = [](uint32_t tile) noexcept { CHECK(tile == 100); return RoadStopType::Bus; };
	t.shared.road_stops = []() noexcept { return &road_abi_stops; };
	t.shared.is_bay_stop = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	probe.stop->entries = &probe.entries;
	probe.entries.east.occupied = 8;
	/* Leaving reads the record's direction and the ground total length. */
	probe.ground.state->cache.cached_total_length = 8;
	CHECK(openttd_rust_road_crash(v, probe.state, false, &t.leaves) == 8);
	CHECK(openttd_rust_road_get(probe.state, 5) == 1 && probe.entries.east.occupied == 0);

	/* Entries nested in a NewGRF property callback see and change the same owner. */


	t.leaves.is_bay_stop = [](uint32_t) noexcept { return false; };
	t.vehicle.engine_for_length = [](OpenTTDVehicle *) noexcept -> OpenTTDEngineForLength {
		auto &p = *road_abi_probe;
		CHECK(p.cache_properties == p.cargo_ages);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(p.engine_replaced ? &road_abi_new_engine : &road_abi_engine), .grf_version = 8};
	};
	t.leaves.shorten_factor = [](const OpenTTDEngine *e) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && road_abi_probe->cache_properties == 1);
		return 2; // Read through the engine captured before the property callback.
	};
	t.vehicle.length_changed = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->length_changes; };
	static OpenTTDEngineCapabilitiesServices capabilities{};
	static OpenTTDVehicleResolverServices resolver{};
	static OpenTTDEngineLoadingServices loading{};
	resolver.capabilities = &capabilities;
	static NewGRFCache grf_cache{};
	resolver.cache = [](OpenTTDVehicle *) noexcept { return &grf_cache; };
	capabilities.grf = [](const OpenTTDEngine *e) noexcept -> const GRFFile * { CHECK(e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_new_engine)); return nullptr; };
	capabilities.loading = []() noexcept -> const OpenTTDEngineLoadingServices * { return &loading; };
	loading.unknown_refit = [](uint32_t grfid, uint16_t callback, uint16_t result) noexcept { CHECK(grfid == 0 && callback == 0x11 && result == 9); };
	t.leaves.default_max_speed = [](uint16_t) noexcept -> uint16_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties == 3);
		return 99;
	};
	t.vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	t.vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *u, uint8_t property, int32_t fallback) noexcept -> int32_t {
		auto &p = *road_abi_probe;
		if (property == 0x22) return fallback;
		++p.cache_properties;
		if (p.fallback_cache) {
			p.engine_replaced = true;
			RoadAbiHandle()->engine_type = 9;
			return property == 0x23 ? (u == RoadAbiHandle() ? 0xFFFF : 9) : 0;
		}
			CHECK(openttd_rust_road_max_speed(u, p.state, p.entry.acceleration_model, p.leaves) == 80);
		CHECK(openttd_rust_road_trackdir(u, p.state, p.leaves) == 0);
		++p.nested;
		openttd_rust_road_set_dest(u, p.state, 142 + p.nested);
		CHECK(openttd_rust_road_path_size(p.state) == 0);
		openttd_rust_road_path_push(p.state, {10, 10815});
		return property == 0x23 ? 0 : property == 0x15 ? 20 : fallback;
	};
	t.leaves.cargo_age_default = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->cargo_ages; return uint16_t{0}; };
	openttd_rust_road_update_cache(v, probe.state, false, &t.leaves);
	CHECK(road_abi_vehicle.cached_max_speed == 80);
	CHECK(probe.ground.state->cache.cached_veh_length == 8 && probe.ground.state->cache.cached_total_length == 8 && probe.ground.state->cache.first_engine == UINT16_MAX);
	CHECK(probe.nested == 2 && probe.dest == 144 && openttd_rust_road_path_get(probe.state, 0).tile == 10815);

	/* Failed/invalid properties consume late fallback reads and the post-callback link.
	 * The first, !same_length run rejects conditional reads; a changed engine
	 * still leaves fallback/error reads attached to the captured engine context. */
	probe.fallback_cache = true;
	road_abi_vehicle.next = &road_abi_second;
	probe.cache_properties = probe.cargo_ages = 0;
	openttd_rust_road_update_cache(v, probe.state, true, &t.leaves);
	CHECK(probe.cargo_ages == 2 && probe.length_changes == 2 && road_abi_vehicle.cached_max_speed == 99);
	CHECK(probe.ground.state->cache.cached_veh_length == 6 && probe.ground.state->cache.cached_total_length == 7 && probe.tail_ground.state->cache.cached_veh_length == 1 && probe.tail_ground.state->cache.first_engine == 9);

	road_abi_vehicle.next = nullptr;
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
	road_abi_vehicle.subtype = 1;
	const uint16_t rd = openttd_rust_road_drive_entry(0, 32, 21);
	CHECK((rd & 0xC0) == 0 && (rd & 15) < 15);
	road_abi_vehicle.x_pos = (rd & 15) + 1; road_abi_vehicle.y_pos = (rd >> 8) & 15;


	t.shared.station_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == 100); return 55; };
	t.leaves.is_drive_through = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	probe.stop->status = 0xFF; // Entering the bay frees its entrance.
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
	movement.road_kind = [](OpenTTDVehicle *) noexcept { return RTT_ROAD; };
	movement.resolve = [](OpenTTDVehicle *) noexcept -> OpenTTDMovementVehicle { return {.train = {}, .road = road_abi_probe->state, .ship = nullptr, .kind = static_cast<VehicleType>(1)}; };
	movement.is_tunnel = [](uint32_t) noexcept { return true; };
	road_abi_vehicle.z_pos = 10; road_abi_vehicle.direction = 1;
	t.shared.movement = &movement;
	t.shared.map = &map.statics;


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
		openttd_rust_road_set_dest(u, p.state, destination);
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
		.allocate = []() noexcept -> OpenTTDEffectAllocation { return {effect_test_world, &effect_test_world->record, current_emission_owner}; },
		.delta = [](void *) noexcept {},
	};
	auto previous_creation = openttd_rust_effect_bind(&creation);
	auto emission_vehicle = t.vehicle;
	emission_vehicle.play_vehicle_sound = [](OpenTTDVehicle *u, VehicleSoundEvent event) noexcept {
		auto &p = *road_abi_probe;
		CHECK(p.visual_stage == 1 && p.visual_eligible && event == VSE_VISUAL_EFFECT); p.visual_stage = 3;
		openttd_rust_road_set(p.state, 3, 1);
		openttd_rust_road_set_dest(u, p.state, 100);
		p.speed = 0; road_abi_vehicle.subspeed = 0;
		p.post_effect_max = 40; p.ground.state->cache.cached_max_track_speed = 40; ++p.nested; return false;
	};
	auto emission_shared = t.shared;
	emission_shared.is_bridge = emission_shared.bridge_above = [](uint32_t) noexcept { return false; };
	static uint8_t emission_cache = 0x11;
	OpenTTDVisualServices emission{
		.vehicle = &emission_vehicle, .shared = &emission_shared,
		/* Eligibility and emission both read the smoke setting before the sound. */
		.smoke = []() noexcept -> uint8_t {
			auto &p = *road_abi_probe; p.visual_stage = 1;
			return p.visual_eligible ? 2 : 0;
		},
		.depot = [](uint32_t) noexcept { return false; }, .tunnel = [](uint32_t) noexcept { return false; },
	};
	openttd_rust_visual_bind(&emission);
	road_abi_vehicle.visual = &emission_cache;




	// Ground speed now executes instead of being stubbed: original model gives
	// speed 2/distance 1; realistic kickoff gives speed 4/distance 3.
	static OpenTTDVehicleResolverServices physics_resolver{};
	physics_resolver.roadtype = [](OpenTTDVehicle *) noexcept { return RoadType{0}; };
	t.physics_vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &physics_resolver; };
	probe.ground.state->cache.cached_weight = 1;
	t.physics_vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.ground.update_viewport = [](OpenTTDVehicle *part) noexcept {
		auto *front = RoadAbiHandle();
		CHECK(part == RoadAbiHandle() && front == part);
		++road_abi_probe->viewports;
		road_abi_probe->progress = 7;
		part->next = nullptr;
	};


	OpenTTDRoadEntry front = probe.entry;
	/* Emission reads the record's speed: at least 2 passes the visual gate. */
	probe.speed = 2;
	// Fresh scalar loads and the post-write crashed-bit reset precede controller entry.
	probe.order_time = INT32_MAX;
	probe.running = UINT8_MAX;
	probe.status = 0;
	road_abi_vehicle.tick_counter = 255;
	probe.progress = 0;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	probe.reset_status = false;
	CHECK(probe.tick == 0 && probe.status == 0);
	CHECK(probe.speed == 2);
	CHECK(probe.nested == 3 && probe.dest == 100 && probe.order_time == INT32_MIN && probe.running == 0 && probe.progress == 7 && world.draws == 0);

	/* Stopped parts preserve running ticks; later status resets must survive the entry. */
	const auto last_speed_dirty = t.physics_vehicle.start_stop_dirty;
	probe.ground.state->cache.last_speed = UINT16_MAX;
	t.physics_vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {
		road_abi_probe->status = 0;
	};
	probe.status = 2;
	probe.running = UINT8_MAX;
	probe.order_time = INT32_MAX;
	probe.progress = 0;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.running == UINT8_MAX && probe.order_time == INT32_MIN && probe.status == 0);
	t.physics_vehicle.start_stop_dirty = last_speed_dirty;

	/* Skip effects with no lookup; realistic UpdateSpeed still observes post-effect inputs. */
	probe.visual_eligible = false; probe.entry.acceleration_model = 1;
	probe.speed = 0; road_abi_vehicle.subspeed = 0;
	front = probe.entry;
	/* Hidden vehicles skip viewport callbacks, then use the direct canonical link. */
	probe.status = 1;
	probe.progress = 0;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.viewports == 1 && probe.progress == 3);
	probe.visual_eligible = true; probe.entry.acceleration_model = 0;
	front = probe.entry;

	/* Viewport reentry can add a later visible part. Its progress mutation is
	 * observed only after that viewport and any hidden final part's Next read. */

	t.ground.update_viewport = [](OpenTTDVehicle *part) noexcept {
		auto *front = RoadAbiHandle();
		auto &p = *road_abi_probe;
		CHECK((p.viewports == 0 && part == front) || (p.viewports == 1 && part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second)));
		++p.viewports;
		p.progress = part == front ? 7 : 0;
		openttd_rust_road_set_dest(front, p.state, 200 + p.viewports);
		part->next = part == front ? &road_abi_second : p.hidden_viewport_tail ? &road_abi_hidden : nullptr;
	};
	for (bool hidden : {false, true}) {
		probe.hidden_viewport_tail = hidden;
		probe.viewports = 0;
		probe.status = 0; road_abi_hidden.vehstatus = 1;
		probe.progress = 0;
		/* The emission chance reads the record tick; zero after the increment emits. */
		road_abi_vehicle.tick_counter = 255;
		CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
		CHECK(probe.viewports == 2 && probe.progress == 1 && probe.dest == 202);
	}

	road_abi_vehicle.next = nullptr; road_abi_second.next = nullptr;
	/* Crash animation reads the next link after the turn/viewport callback. */
	openttd_rust_road_set(probe.state, 5, 0);
	probe.status = 128;
	t.physics.viewport = [](OpenTTDVehicle *u, bool force, bool delta) noexcept { CHECK(u == RoadAbiHandle() && u->direction < 8 && force && delta); ++road_abi_probe->turns; };
	road_abi_vehicle.tick_counter = 7;
	const OpenTTDRoadEntry animation = probe.entry;
	CHECK(openttd_rust_road_tick(v, probe.state, &animation, &t.leaves, nullptr));
	CHECK(probe.turns == 1 && world.draws == 1);

	/* The final-part deletion returns its copied alive result only. */
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 5, 2219);


	static OpenTTDVehicle crash_record{};
	crash_record.first = &crash_record;
	crash_record.last_station_visited = 5;
	t.vehicle.record_from_shell = [](void *shell) noexcept -> OpenTTDVehicle * {
		if (shell == nullptr) return nullptr;
		CHECK(shell == RoadAbiHandle());
		return &crash_record;
	};
	t.vehicle.destroy = [](OpenTTDVehicle *u) noexcept {
		CHECK(u == RoadAbiHandle());
		openttd_rust_road_destroy(road_abi_probe->state);
		road_abi_probe->state = nullptr;
		++road_abi_probe->destroyed;
	};
	probe.status = 128;
	road_abi_vehicle.tick_counter = 31;
	const OpenTTDRoadEntry crashed = probe.entry;
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
	uint32_t tile = 3091;
	uint16_t speed = 5;
	bool depot_tile = true;
	uint32_t depot_orders = 0, services = 0, searches = 0;
};
static RoadServiceProbe *road_service_probe;
static OpenTTDVehicle road_service_vehicles[2]{};
static OpenTTDGroundState road_service_ground[2]{};
static OpenTTDVehicleOrderState road_service_orders[2]{};
static OpenTTDVehicle *RoadServiceHandle(int part) { auto *record = &road_service_vehicles[part]; record->shell = reinterpret_cast<OpenTTDVehicleShell *>(record); return record; }
static void RoadServiceBoundary()
{
	RoadServiceProbe probe;
	road_service_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	t.leaves.max_depot_penalty = []() noexcept -> uint32_t { return 300; };
	static VehicleDefaultSettings defaults{}; defaults.servint_roadveh = 1;
	static OpenTTDSettingServices settings{};
	settings.company_vehicle_defaults = [](uint8_t) noexcept { return &defaults; };
	static OpenTTDCompanyServices company{};
	company.settings = []() noexcept -> const OpenTTDSettingServices & { return settings; };
	static OpenTTDCommandServices commands{};
	commands.company = []() noexcept -> const OpenTTDCompanyServices * { return &company; };
	static OpenTTDFleetTransactionServices transactions{}; transactions.commands = &commands;
	_vehicle_maintenance_probe.transaction = &transactions;
	/* Direct maintenance: aged out, no reliability decay, no breakdown, service due. */
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.max_economy_date = OpenTTDEconomyDate{0};
	maintenance.economy_age = [](OpenTTDVehicle *) noexcept { return OpenTTDEconomyDate{0}; };
	maintenance.check_breakdown = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceCheckBreakdown r{};
		r.no_service = true;
		return r;
	};
	maintenance.breakdown_guard = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceBreakdownGuard r{};
		return r;
	};
	maintenance.automatic = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceAutomatic{}; };
	maintenance.interval = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceInterval r{};
		r.now = OpenTTDEconomyDate{1};
		return r;
	};
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept {};
	maintenance.reset = [](OpenTTDVehicle *record) noexcept { if (record == RoadServiceHandle(0)) ++road_service_probe->services; return OpenTTDVehicleMaintenanceReset{}; };
	t.vehicle.write_economy_service = t.vehicle.write_calendar_service = [](OpenTTDVehicle *, int32_t) noexcept {};
	t.vehicle.write_breakdowns = [](OpenTTDVehicle *, uint8_t) noexcept {};
	t.vehicle.write_reliability = [](OpenTTDVehicle *, uint16_t) noexcept {};
	maintenance.chance_write = [](OpenTTDVehicle *, uint8_t) noexcept {};


	t.leaves.is_road_depot_tile = [](uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); return road_service_probe->depot_tile; };




	t.leaves.find_depot = [](OpenTTDVehicle *, int32_t penalty) noexcept -> OpenTTDRoadDepotResult {
		CHECK(penalty == 300);
		++road_service_probe->searches;
		return {.tile = UINT32_MAX, .length = UINT32_MAX};
	};
	t.shared.depot_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == road_service_probe->tile); return 55; };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *v) noexcept {
		CHECK((v->ground->flags & (1 << GVF_SUPPRESS_IMPLICIT_ORDERS)) != 0);
		CHECK((v->orders->current.type & 15) == OT_GOTO_DEPOT && v->orders->current.destination == 55 && v->orders->current.refit_cargo == CARGO_NO_REFIT);
		++road_service_probe->depot_orders;
	};
	t.vehicle.check_orders_only = [](OpenTTDVehicle *) noexcept {};
	auto day = [&]() {
		for (int i : {0, 1}) { auto *record = RoadServiceHandle(i); record->ground = &road_service_ground[i]; record->orders = &road_service_orders[i]; record->type = VEH_ROAD; record->tile = probe.tile; record->cur_speed = probe.speed; record->road = i == 0 ? probe.head : probe.tail; record->next = i == 0 ? RoadServiceHandle(1) : nullptr; }
		openttd_rust_road_economy_day(RoadServiceHandle(0), probe.head, true, &t.leaves); };
	for (bool moving : {true, false}) {
		probe.speed = moving ? 5 : 0;
		RoadServiceHandle(0)->dest_tile = 42;
		openttd_rust_road_set(probe.head, 0, 254);
		openttd_rust_road_set(probe.tail, 0, moving ? 254 : 1);
		openttd_rust_road_path_push(probe.head, {10, 10815});
		day();
		CHECK(RoadServiceHandle(0)->dest_tile == probe.tile && openttd_rust_road_path_size(probe.head) == 0);
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

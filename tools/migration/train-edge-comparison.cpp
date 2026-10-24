/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train-edge-comparison.cpp Bounded NewGRF-only copied-world curve/reversal gap. */
#include "stdafx.h"
#include "core/math_func.hpp"
#include "direction_func.h"
#include "rust/train_ffi.h"
#include "rust/movement_ffi.h"
#include "rust/vehicle_ffi.h"
#include <cstdio>
#include <vector>

/* The original selected bodies are included unchanged. Shared world operations
 * only record their call order; no map, train controller, or NewGRF is emulated. */
using TileIndex = uint32_t;
constexpr uint VEHICLE_LENGTH = 8;
constexpr uint8_t TRACK_BIT_DEPOT = 0x80, TRACK_BIT_WORMHOLE = 0x40;
constexpr int GVF_GOINGUP_BIT = 0, GVF_GOINGDOWN_BIT = 1, MP_TUNNELBRIDGE = 9;
constexpr int AM_ORIGINAL = 0;
struct { struct { int train_acceleration_model; } vehicle; } _settings_game;
struct RailTypeInfo { uint8_t curve_speed; } rail;
static uint8_t GetRailType(TileIndex) { return 0; }
static const RailTypeInfo *GetRailTypeInfo(uint8_t) { return &rail; }
enum class VehState { Hidden = 0 };
struct Status {
	uint8_t bits;
	bool Test(VehState) const { return (this->bits & 1) != 0; }
	void Set(VehState, bool value) { this->bits = (this->bits & ~1) | uint8_t(value); }
};
static std::vector<uint64_t> events;
struct Train {
	uint32_t id;
	Train *next;
	uint8_t track;
	Direction direction;
	int x_pos, y_pos, z_pos;
	TileIndex tile;
	uint16_t gv_flags;
	Status vehstatus;
	struct { uint16_t cached_veh_length; } gcache;
	struct { bool cached_tilt; int16_t cached_curve_speed_mod; } tcache;
	OpenTTDTrainState *state;
	OpenTTDVehicleTileLinks links{};
	const Train *First() const;
	Train *Next() const { return this->next; }
	uint16_t GetCurveSpeedLimit() const;
	void UpdatePosition() { events.push_back(0x100 + this->id); }
	void UpdateViewport(bool a, bool b) { assert(a && b); events.push_back(0x200 + this->id); }
	void UpdateInclination(bool, bool) { std::abort(); }
};
static Train trains[16];
const Train *Train::First() const { return trains; }
static void VehicleEnterTile(Train *v, TileIndex tile, int x, int y)
{
	assert(tile == v->tile && x == v->x_pos && y == v->y_pos);
	events.push_back(0x300 + v->id);
}
static TileIndex TileVirtXY(int x, int y) { return uint32_t((y >> 4) * 256 + (x >> 4)); }
static bool IsTileType(TileIndex, int) { return false; }
static bool IsBridgeTile(TileIndex) { return false; }
#include "train-reference.inc"

static OpenTTDTrainHandle Handle(Train *v) noexcept { return {v, v == nullptr ? nullptr : v->state}; }
static Train &Part(OpenTTDTrainHandle part) noexcept { return *static_cast<Train *>(part.shell); }
static uint32_t UnexpectedRandom(void *) noexcept { std::abort(); }
static uint32_t UnexpectedIndustry(int32_t, int32_t, uint32_t *) noexcept { std::abort(); }
static OpenTTDMovementServices movement;
static OpenTTDVehicleServices vehicle;
static uint32_t entered_tile;
static OpenTTDTrainServices Leaves()
{
	OpenTTDTrainServices leaves{};
	leaves.read_direction = [](OpenTTDTrainHandle h) noexcept { return uint8_t(Part(h).direction); };
	leaves.read_next = [](OpenTTDTrainHandle h) noexcept { return Handle(Part(h).next); };
	leaves.read_curve_limit = [](OpenTTDTrainHandle h) noexcept { const Train &v = Part(h); return OpenTTDTrainCurveLimitRead{Handle(v.next), v.direction}; };
	leaves.read_length = [](OpenTTDTrainHandle h) noexcept { return Part(h).gcache.cached_veh_length; };
	leaves.read_reverse_swap = [](OpenTTDTrainHandle h) noexcept { const Train &v = Part(h); return OpenTTDTrainReverseSwapRead{v.tile, v.x_pos, v.y_pos, v.z_pos, v.direction, v.vehstatus.bits}; };
	leaves.read_status = [](OpenTTDTrainHandle h) noexcept { return Part(h).vehstatus.bits; };
	leaves.read_gv_flags = [](OpenTTDTrainHandle h) noexcept { return Part(h).gv_flags; };
	leaves.read_after_swap = [](OpenTTDTrainHandle h) noexcept { const Train &v = Part(h); return OpenTTDTrainAfterSwapRead{v.tile, v.x_pos, v.y_pos}; };
	leaves.read_articulated = [](OpenTTDTrainHandle h) noexcept { return uint8_t(Part(h).id % 2); };
	leaves.write_tile = [](OpenTTDTrainHandle h, uint32_t value) noexcept { Part(h).tile = value; };
	leaves.write_x = [](OpenTTDTrainHandle h, int32_t value) noexcept { Part(h).x_pos = value; };
	leaves.write_y = [](OpenTTDTrainHandle h, int32_t value) noexcept { Part(h).y_pos = value; };
	leaves.write_z = [](OpenTTDTrainHandle h, int32_t value) noexcept { Part(h).z_pos = value; };
	leaves.write_direction = [](OpenTTDTrainHandle h, uint8_t value) noexcept { Part(h).direction = Direction(value); };
	leaves.write_gv_flags = [](OpenTTDTrainHandle h, uint16_t value) noexcept { Part(h).gv_flags = value; };
	leaves.write_status = [](OpenTTDTrainHandle h, uint8_t value) noexcept { Part(h).vehstatus.bits = value; };
	leaves.acc_model = [](OpenTTDTrainHandle) noexcept { return uint64_t(_settings_game.vehicle.train_acceleration_model); };
	leaves.curve_advantage = [](OpenTTDTrainHandle) noexcept { return uint64_t(rail.curve_speed); };
	vehicle.tile_links = [](OpenTTDVehicle *v) noexcept { return &reinterpret_cast<Train *>(v)->links; };
	vehicle.tile = [](OpenTTDVehicle *v) noexcept { auto &part = *reinterpret_cast<Train *>(v); part.UpdatePosition(); return part.tile; };
	vehicle.occupancy_update = [](OpenTTDVehicle *shell) noexcept -> OpenTTDOccupancyUpdateObservation { return {.links = vehicle.tile_links(shell), .tile = vehicle.tile(shell)}; };
	vehicle.map_log_x = []() noexcept { return uint32_t{8}; };
	leaves.vehicle = &vehicle;
	leaves.viewport = [](OpenTTDTrainHandle h, uint8_t a, uint8_t b) noexcept { Part(h).UpdateViewport(a != 0, b != 0); };
	leaves.tile_virt = [](int32_t x, int32_t y) noexcept { return uint32_t(TileVirtXY(x, y)); };
	leaves.is_tunnelbridge = [](OpenTTDTrainHandle, uint32_t) noexcept { return uint64_t(0); };
	return leaves;
}
static uint16_t RustCurve()
{
	const auto leaves = Leaves();
	const OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, UnexpectedRandom, UnexpectedIndustry);
	return openttd_rust_train_curve_limit(Handle(trains), &leaves, &shared);
}
static void RustReverse(int left, int right)
{
	const auto leaves = Leaves();
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, UnexpectedRandom, UnexpectedIndustry);
	movement.shared = &shared;
	shared.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	// The copied world has no depot/tunnel tiles. Observe the real owner's
	// admission call through typed map/vehicle services, retaining event order.
	shared.tile_type = [](uint32_t tile) noexcept { entered_tile = tile; return uint8_t{1}; };
	movement.resolve = [](OpenTTDVehicle *shell) noexcept {
		auto *v = reinterpret_cast<Train *>(shell);
		assert(entered_tile == v->tile);
		events.push_back(0x300 + v->id);
		return OpenTTDMovementVehicle{Handle(v), nullptr, nullptr, VehicleType{0}};
	};
	movement.rail_depot_tile = [](uint32_t) noexcept { return false; };
	openttd_rust_train_reverse_swap(Handle(trains), left, right, &leaves, &shared);
}
static uint32_t random_state = 130;
static uint32_t Draw() { random_state = random_state * 1664525U + 1013904223U; return random_state; }
static void Sync()
{
	for (Train &v : trains) {
		openttd_rust_train_state_set_track(v.state, v.track);
		openttd_rust_train_state_set_cached_tilt(v.state, v.tcache.cached_tilt);
		openttd_rust_train_state_set_cached_curve_speed_mod(v.state, static_cast<int16_t>(uint16_t(v.tcache.cached_curve_speed_mod)));
	}
}
static std::vector<uint64_t> State()
{
	auto result = events;
	for (Train &v : trains) {
		result.insert(result.end(), {uint64_t(v.track), uint64_t(v.direction), uint64_t(uint32_t(v.x_pos)), uint64_t(uint32_t(v.y_pos)), uint64_t(uint32_t(v.z_pos)), v.tile, v.gv_flags, v.vehstatus.bits});
	}
	return result;
}
int main()
{
	for (uint32_t i = 0; i < 16; ++i) { trains[i].id = i; trains[i].state = openttd_rust_train_state_new(); }
	for (uint32_t trial = 0; trial < 20000; ++trial) {
		uint32_t count = 1 + Draw() % 16;
		_settings_game.vehicle.train_acceleration_model = trial % 2;
		rail.curve_speed = uint8_t(Draw());
		for (uint32_t i = 0; i < 16; ++i) {
			Train &v = trains[i];
			v.next = i + 1 < count ? &trains[i + 1] : nullptr;
			v.direction = Direction(Draw() % 8); v.gcache.cached_veh_length = 1 + Draw() % 8;
			v.tcache.cached_tilt = (Draw() & 1) != 0; v.tcache.cached_curve_speed_mod = int16_t(Draw());
			v.track = uint8_t(1 << (Draw() % 8)); v.gv_flags = uint16_t(Draw()); v.vehstatus.bits = uint8_t(Draw());
			v.x_pos = int(Draw() % 4000); v.y_pos = int(Draw() % 4000); v.z_pos = int(Draw() % 256); v.tile = Draw() % 65536;
		}
		Sync();
		if (trains[0].GetCurveSpeedLimit() != RustCurve()) { std::fprintf(stderr, "curve mismatch %u\n", trial); return 1; }
		Train original[16]; std::copy(std::begin(trains), std::end(trains), std::begin(original));
		int l = int(Draw() % count), r = int(Draw() % count);
		events.clear(); ReverseTrainSwapVeh(trains, l, r); auto expected = State();
		std::copy(std::begin(original), std::end(original), std::begin(trains)); Sync(); events.clear(); RustReverse(l, r);
		for (Train &v : trains) v.track = uint8_t(openttd_rust_train_state_get_track(v.state));
		if (expected != State()) { std::fprintf(stderr, "reversal mismatch %u\n", trial); return 1; }
		for (Train &v : trains) openttd_rust_occupancy_update(&vehicle, reinterpret_cast<OpenTTDVehicle *>(&v), true);
	}
	for (Train &v : trains) openttd_rust_train_state_destroy(v.state);
	std::printf("20000 variable-length curve and 20000 reversal cases equal through production Rust entries\n");
}

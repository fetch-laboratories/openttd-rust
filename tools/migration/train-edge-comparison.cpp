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
#include "rust/vehicle_record_ffi.h"
#include "rust/train_ffi.h"
#include "abi-record-ground.h"
#include "rust/movement_ffi.h"
#include "rust/vehicle_ffi.h"
#include "tests/fixture_map.hpp"
#include <cstdio>
#include <vector>

/* The original selected bodies are included unchanged. Shared world operations
 * only record their call order; no map, train controller, or NewGRF is emulated. */
using TileIndex = uint32_t;
constexpr uint VEHICLE_LENGTH = 8;
constexpr uint8_t TRACK_BIT_DEPOT = 0x80, TRACK_BIT_WORMHOLE = 0x40;
constexpr int GVF_GOINGUP_BIT = 0, GVF_GOINGDOWN_BIT = 1, MP_TUNNELBRIDGE = 9;
constexpr int AM_ORIGINAL = 0;
struct { struct { uint8_t train_acceleration_model; } vehicle; } _settings_game;
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
	OpenTTDVehicle record{};
	OpenTTDGroundState ground{};
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
/* The reference's asserted entry tiles, in order, and the next one the candidate must use. */
static std::vector<uint32_t> entered; static size_t next_entry;
const Train *Train::First() const { return trains; }
static void VehicleEnterTile(Train *v, TileIndex tile, int x, int y)
{
	assert(tile == v->tile && x == v->x_pos && y == v->y_pos);
	events.push_back(0x300 + v->id); entered.push_back(tile);
}
static TileIndex TileVirtXY(int x, int y) { return uint32_t((y >> 4) * 256 + (x >> 4)); }
static bool IsTileType(TileIndex, int) { return false; }
static bool IsBridgeTile(TileIndex) { return false; }
#include "train-reference.inc"

static OpenTTDTrainHandle Handle(Train *v) noexcept { return {v == nullptr ? nullptr : &v->record, v == nullptr ? nullptr : v->state}; }
static Train &Part(OpenTTDTrainHandle part) noexcept { return *reinterpret_cast<Train *>(part.record->shell); }
static uint32_t UnexpectedRandom(void *) noexcept { std::abort(); }
static uint32_t UnexpectedIndustry(int32_t, int32_t, uint32_t *) noexcept { std::abort(); }
static OpenTTDMovementServices movement;
static OpenTTDVehicleServices vehicle;
static const OpenTTDSettingAddresses _settings_catalog{.train_acceleration_model = &_settings_game.vehicle.train_acceleration_model};
static OpenTTDTrainServices Leaves()
{
	OpenTTDTrainServices leaves{};
	leaves.curve_advantage = [](OpenTTDTrainHandle) noexcept { return uint64_t(rail.curve_speed); };
	vehicle.tile_links = [](OpenTTDVehicle *v) noexcept { auto &part = *reinterpret_cast<Train *>(v->shell); part.UpdatePosition(); return &part.links; };
	vehicle.map_log_x = []() noexcept { return uint32_t{8}; };
	leaves.vehicle = &vehicle;
	leaves.viewport = [](OpenTTDTrainHandle h, uint8_t a, uint8_t b) noexcept { Part(h).UpdateViewport(a != 0, b != 0); };
	leaves.tile_virt = [](int32_t x, int32_t y) noexcept { return uint32_t(TileVirtXY(x, y)); };
	return leaves;
}
static uint16_t RustCurve()
{
	const auto leaves = Leaves();
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, UnexpectedRandom, UnexpectedIndustry);
	shared.settings = &_settings_catalog;
	return openttd_rust_train_curve_limit(Handle(trains), &leaves, &shared);
}
static void RustReverse(int left, int right)
{
	const auto leaves = Leaves();
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, UnexpectedRandom, UnexpectedIndustry);
	movement.shared = &shared;
	shared.movement = &movement;
	shared.settings = &_settings_catalog;
	// The 256 by 256 map is plain rail only on the tile of the reference's next
	// entry, which it asserted is that vehicle's own tile, and clear elsewhere.
	// Depot entry reads the map in place and VehicleEnterTile reads the tile type
	// there, so an entry on any other tile skips the vehicle callback and changes
	// the compared events. The copied world has no depot/tunnel tiles. Observe the
	// real owner's admission call through typed map/vehicle services, retaining
	// event order.
	static OpenTTDFixtureMap map{8, 256 * 256};
	next_entry = 0; if (!entered.empty()) map.base[entered[0]].type = 1 << 4; // MP_RAILWAY
	shared.map = &map.statics;
	movement.resolve = [](OpenTTDVehicle *shell) noexcept {
		auto *v = reinterpret_cast<Train *>(shell->shell);
		events.push_back(0x300 + v->id);
		map.base[entered[next_entry]].type = 0; if (++next_entry < entered.size()) map.base[entered[next_entry]].type = 1 << 4;
		return OpenTTDMovementVehicle{Handle(v), nullptr, nullptr, VehicleType{0}};
	};
	openttd_rust_train_reverse_swap(Handle(trains), left, right, &leaves, &shared);
	for (uint32_t tile : entered) map.base[tile].type = 0; // MP_CLEAR
}
static uint32_t random_state = 130;
static uint32_t Draw() { random_state = random_state * 1664525U + 1013904223U; return random_state; }
static void Sync()
{
	for (Train &v : trains) {
		v.record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&v);
		v.record.train = v.state;
		v.record.ground = &v.ground;
		v.record.next = v.next == nullptr ? nullptr : &v.next->record;
		v.record.first = &trains[0].record;
		v.record.tile = v.tile;
		v.record.x_pos = v.x_pos; v.record.y_pos = v.y_pos; v.record.z_pos = v.z_pos;
		v.record.direction = v.direction; v.record.vehstatus = v.vehstatus.bits;
		v.record.subtype = (v.id % 2) << 2;
		v.ground.cache.cached_veh_length = v.gcache.cached_veh_length;
		v.ground.flags = v.gv_flags;
		(v.state)->track = v.track;
		(v.state)->cached_tilt = (v.tcache.cached_tilt) != 0;
		(v.state)->cached_curve_speed_mod = static_cast<int16_t>(uint16_t(v.tcache.cached_curve_speed_mod));
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
		events.clear(); entered.clear(); ReverseTrainSwapVeh(trains, l, r); auto expected = State();
		std::copy(std::begin(original), std::end(original), std::begin(trains)); Sync(); events.clear(); RustReverse(l, r);
		for (Train &v : trains) {
			v.track = uint8_t(v.state->track);
			v.tile = v.record.tile; v.x_pos = v.record.x_pos; v.y_pos = v.record.y_pos; v.z_pos = v.record.z_pos;
			v.direction = Direction(v.record.direction); v.vehstatus.bits = v.record.vehstatus; v.gv_flags = v.ground.flags;
		}
		if (expected != State()) { std::fprintf(stderr, "reversal mismatch %u\n", trial); return 1; }
		for (Train &v : trains) openttd_rust_occupancy_update(&vehicle, &v.record, true);
	}
	for (Train &v : trains) openttd_rust_train_state_destroy(v.state);
	std::printf("20000 variable-length curve and 20000 reversal cases equal through production Rust entries\n");
}

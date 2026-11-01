/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roadvehicle.cpp Road owner lifetime and serialization-boundary evidence. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../roadveh.h"
#include "../train.h"
#include "../settings_type.h"
#include "../rust/services_road.h"
#include "../rust/vehicle_services.h"
#include "abi_layout.hpp"
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDVehicleResolverServices.hpp"
#include "../safeguards.h"

/* The road corpus has no tram/articulated turning witness. Keep the unchanged
 * original movement tables solely in this fixture to check every reachable table
 * entry; this does not establish controller equivalence for those vehicles. */
namespace {
struct RoadDriveEntry { uint8_t x, y; };
#include "reference-roadveh_movement.h"
static const std::pair<const RoadDriveEntry *, size_t> source_rows[] = {
	{_roadveh_drive_data_0, std::size(_roadveh_drive_data_0)},
	{_roadveh_drive_data_1, std::size(_roadveh_drive_data_1)},
	{_roadveh_drive_data_2, std::size(_roadveh_drive_data_2)},
	{_roadveh_drive_data_3, std::size(_roadveh_drive_data_3)},
	{_roadveh_drive_data_4, std::size(_roadveh_drive_data_4)},
	{_roadveh_drive_data_5, std::size(_roadveh_drive_data_5)},
	{_roadveh_drive_data_6, std::size(_roadveh_drive_data_6)},
	{_roadveh_drive_data_7, std::size(_roadveh_drive_data_7)},
	{_roadveh_drive_data_8, std::size(_roadveh_drive_data_8)},
	{_roadveh_drive_data_9, std::size(_roadveh_drive_data_9)},
	{_roadveh_drive_data_10, std::size(_roadveh_drive_data_10)},
	{_roadveh_drive_data_11, std::size(_roadveh_drive_data_11)},
	{_roadveh_drive_data_12, std::size(_roadveh_drive_data_12)},
	{_roadveh_drive_data_13, std::size(_roadveh_drive_data_13)},
	{_roadveh_drive_data_14, std::size(_roadveh_drive_data_14)},
	{_roadveh_drive_data_15, std::size(_roadveh_drive_data_15)},
	{_roadveh_drive_data_16, std::size(_roadveh_drive_data_16)},
	{_roadveh_drive_data_17, std::size(_roadveh_drive_data_17)},
	{_roadveh_drive_data_18, std::size(_roadveh_drive_data_18)},
	{_roadveh_drive_data_19, std::size(_roadveh_drive_data_19)},
	{_roadveh_drive_data_20, std::size(_roadveh_drive_data_20)},
	{_roadveh_drive_data_21, std::size(_roadveh_drive_data_21)},
	{_roadveh_drive_data_22, std::size(_roadveh_drive_data_22)},
	{_roadveh_drive_data_23, std::size(_roadveh_drive_data_23)},
	{_roadveh_drive_data_24, std::size(_roadveh_drive_data_24)},
	{_roadveh_drive_data_25, std::size(_roadveh_drive_data_25)},
	{_roadveh_drive_data_26, std::size(_roadveh_drive_data_26)},
	{_roadveh_drive_data_27, std::size(_roadveh_drive_data_27)},
	{_roadveh_drive_data_28, std::size(_roadveh_drive_data_28)},
	{_roadveh_drive_data_29, std::size(_roadveh_drive_data_29)},
	{_roadveh_drive_data_30, std::size(_roadveh_drive_data_30)},
	{_roadveh_drive_data_31, std::size(_roadveh_drive_data_31)},
	{_rv_station_left_sw_far, std::size(_rv_station_left_sw_far)},
	{_rv_station_left_nw_far, std::size(_rv_station_left_nw_far)},
	{_rv_station_left_sw_near, std::size(_rv_station_left_sw_near)},
	{_rv_station_left_nw_near, std::size(_rv_station_left_nw_near)},
	{_rv_station_left_ne_far, std::size(_rv_station_left_ne_far)},
	{_rv_station_left_se_far, std::size(_rv_station_left_se_far)},
	{_rv_station_left_ne_near, std::size(_rv_station_left_ne_near)},
	{_rv_station_left_se_near, std::size(_rv_station_left_se_near)},
	{_rv_station_right_sw_far, std::size(_rv_station_right_sw_far)},
	{_rv_station_right_nw_far, std::size(_rv_station_right_nw_far)},
	{_rv_station_right_sw_near, std::size(_rv_station_right_sw_near)},
	{_rv_station_right_nw_near, std::size(_rv_station_right_nw_near)},
	{_rv_station_right_ne_far, std::size(_rv_station_right_ne_far)},
	{_rv_station_right_se_far, std::size(_rv_station_right_se_far)},
	{_rv_station_right_ne_near, std::size(_rv_station_right_ne_near)},
	{_rv_station_right_se_near, std::size(_rv_station_right_se_near)},
	{_roadveh_tram_turn_ne_0, std::size(_roadveh_tram_turn_ne_0)},
	{_roadveh_tram_turn_ne_1, std::size(_roadveh_tram_turn_ne_1)},
	{_roadveh_tram_turn_se_0, std::size(_roadveh_tram_turn_se_0)},
	{_roadveh_tram_turn_se_1, std::size(_roadveh_tram_turn_se_1)},
	{_roadveh_tram_turn_sw_0, std::size(_roadveh_tram_turn_sw_0)},
	{_roadveh_tram_turn_sw_1, std::size(_roadveh_tram_turn_sw_1)},
	{_roadveh_tram_turn_nw_0, std::size(_roadveh_tram_turn_nw_0)},
	{_roadveh_tram_turn_nw_1, std::size(_roadveh_tram_turn_nw_1)},
};
}

TEST_CASE("Road vehicles - original movement and stop tables")
{
	for (uint8_t tram = 0; tram < 2; ++tram) {
		for (size_t state = 0; state < std::size(_road_road_drive_data); ++state) {
			const RoadDriveEntry *row = _road_drive_data[tram][state];
			if (row == nullptr) continue;
			const auto source = std::find_if(std::begin(source_rows), std::end(source_rows), [&](const auto &item) { return item.first == row; });
			REQUIRE(source != std::end(source_rows));
			for (uint8_t frame = 0; frame < source->second; ++frame) {
				const auto entry = row[frame];
				CHECK(openttd_rust_road_drive_entry(tram, static_cast<uint8_t>(state), frame) == (entry.x | (entry.y << 8)));
			}
		}
	}
	for (size_t index = 0; index < std::size(_road_stop_stop_frame); ++index) {
		CHECK(openttd_rust_road_stop_frame(static_cast<uint32_t>(index)) == _road_stop_stop_frame[index]);
	}
}

TEST_CASE("Road vehicles - canonical path, nested save staging and indexed pool reuse")
{
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new RoadVehicle();
	auto *b = new RoadVehicle();
	const VehicleID reused_index = a->index;
	CHECK(a->GetState() == 0);
	CHECK(a->GetFrame() == 0);
	CHECK(a->GetBlockedCounter() == 0);
	CHECK(a->GetOvertaking() == 0);
	CHECK(a->GetOvertakingCounter() == 0);
	CHECK(a->GetCrashedCounter() == 0);
	CHECK(a->GetReverseCounter() == 0);
	CHECK(a->PathSize() == 0);
	a->SetState(255);
	a->SetFrame(254);
	a->SetBlockedCounter(65535);
	a->SetOvertaking(253);
	a->SetOvertakingCounter(252);
	a->SetCrashedCounter(65534);
	a->SetReverseCounter(251);
	a->PathPush(TRACKDIR_X_NE, TileIndex(17));
	a->PathPush(TRACKDIR_Y_SE, TileIndex(23));
	CHECK(a->PathSize() == 2);
	CHECK(a->PathBack().tile == TileIndex(23));
	CHECK(a->PathBack().trackdir == TRACKDIR_Y_SE);
	auto copy = a->CopyPath();
	a->PathPop();
	CHECK(a->PathBack().tile == TileIndex(17));
	a->ReplacePath(copy);
	{
		RoadVehicleStateScope saving(a, false);
		CHECK(RoadVehicleStateScope::State() == 255);
		CHECK(RoadVehicleStateScope::Frame() == 254);
		CHECK(RoadVehicleStateScope::BlockedCounter() == 65535);
		CHECK(RoadVehicleStateScope::Overtaking() == 253);
		CHECK(RoadVehicleStateScope::OvertakingCounter() == 252);
		CHECK(RoadVehicleStateScope::CrashedCounter() == 65534);
		CHECK(RoadVehicleStateScope::ReverseCounter() == 251);
		CHECK(RoadVehicleStateScope::Path()[0].tile == TileIndex(17));
		RoadVehicleStateScope::PathTrackdirs().push_back(TRACKDIR_X_NE);
		{
			RoadVehicleStateScope loading(b, true);
			RoadVehicleStateScope::State() = 7;
			RoadVehicleStateScope::Frame() = 9;
			RoadVehicleStateScope::Path().emplace_back(TRACKDIR_Y_SE, TileIndex(31));
			CHECK(RoadVehicleStateScope::PathTrackdirs().empty());
			CHECK(b->GetState() == 0);
			CHECK(b->PathSize() == 0);
		}
		CHECK(b->GetState() == 7);
		CHECK(b->GetFrame() == 9);
		CHECK(b->PathBack().tile == TileIndex(31));
		CHECK(RoadVehicleStateScope::State() == 255);
		CHECK(RoadVehicleStateScope::PathTrackdirs().size() == 1);
		RoadVehicleStateScope::State() = 1;
		RoadVehicleStateScope::Path().clear();
	}
	CHECK(a->GetState() == 255); // Save/reference fixup does not mutate the owner.
	CHECK(a->PathSize() == 2);
	try {
		RoadVehicleStateScope loading(a, true);
		RoadVehicleStateScope::CrashedCounter() = 47;
		RoadVehicleStateScope::Path().pop_back();
		throw 1;
	} catch (int) {}
	CHECK(a->GetCrashedCounter() == 47);
	CHECK(a->PathSize() == 1); // Partial loads commit when the loader unwinds.
	CHECK(a->PathBack().tile == TileIndex(17));
	a->ClearPath();
	CHECK(a->PathSize() == 0);
	/* PreDestructor and Vehicle::~Vehicle return early during pool cleanup, but
	 * the canonical owner's member deleter must still run before ID reuse. */
	_vehicle_pool.CleanPool();
	auto *loaded = new (reused_index) RoadVehicle();
	CHECK(loaded->index == reused_index);
	CHECK(loaded->GetState() == 0);
	CHECK(loaded->GetCrashedCounter() == 0);
	CHECK(loaded->PathSize() == 0);
	_vehicle_pool.CleanPool();
}

namespace {
OpenTTDRoadSpeedPart speed_parts[2];
int bridge_reads;
OpenTTDRoadSpeedPart SpeedPart(OpenTTDVehicle *v) noexcept { return speed_parts[reinterpret_cast<uintptr_t>(v) - 1]; }
uint16_t BridgeSpeed(uint32_t) noexcept { ++bridge_reads; return 37; }
}
TEST_CASE("Road max speed - front reversal, later curve assignment and hidden bridges")
{
	auto *front = openttd_rust_road_new(), *tail = openttd_rust_road_new();
	auto *v = reinterpret_cast<OpenTTDVehicle *>(1);
	auto shared = OpenTTDSharedServices{};
	shared.bridge_speed = BridgeSpeed;
	auto leaves = OpenTTDRoadLeaves{};
	leaves.shared = &shared;
	leaves.speed_part = SpeedPart;
	for (uint16_t model = 0; model < 2; ++model) for (uint16_t state = 0; state < 256; ++state) for (uint16_t tail_state = 0; tail_state < 256; ++tail_state) {
		for (uint8_t hidden : {0, 1, 2, 3}) for (uint8_t direction : {0, 1}) {
			openttd_rust_road_set(front, 0, state); openttd_rust_road_set(tail, 0, tail_state);
			OpenTTDRoadMaxSpeedRead r{};
			r.next = v;
			r.next_state = tail;
			r.max_track_speed = static_cast<uint16_t>(tail_state * 257);
			r.order_max_speed = static_cast<uint16_t>(state * 257);
			r.direction = 1;
			r.status = static_cast<uint8_t>((hidden & 1) | 128);
			speed_parts[0] = {};
			speed_parts[0].direction = direction;
			speed_parts[0].status = static_cast<uint8_t>((hidden >> 1) | 64);
			/* Pinned GetCurrentMaxSpeed loop: this->state, assignment (not min), then bridge. */
			int expected = r.max_track_speed, reads = 0;
			auto part_hidden = hidden & 1;
			for (auto [part_state, dir] : {std::pair{state, uint16_t{1}}, std::pair{tail_state, uint16_t{direction}}}) {
				if (model == AM_REALISTIC) {
					if (state <= RVSB_TRACKDIR_MASK && IsReversingRoadTrackdir(static_cast<Trackdir>(state))) { expected = r.max_track_speed / 2; break; }
					else if ((dir & 1) == 0) expected = r.max_track_speed * 3 / 4;
				}
				if (part_state == RVSB_WORMHOLE && !part_hidden) { expected = std::min(expected, 37 * 2); ++reads; }
				part_hidden = hidden >> 1;
			}
			bridge_reads = 0;
			CHECK(openttd_rust_road_max_speed(v, front, model, &r, &leaves) == std::min(expected, r.order_max_speed * 2));
			CHECK(bridge_reads == reads);
		}
	}
	openttd_rust_road_destroy(front); openttd_rust_road_destroy(tail);
}
TEST_CASE("Ground cache - canonical aliases on normal and indexed construction")
{
	static_assert(std::is_trivially_destructible_v<OpenTTDGroundState>);
	auto check = [](auto *v) {
		CHECK(&v->gcache == &v->rust_ground.state->cache);
		CHECK(&v->gv_flags == &v->rust_ground.state->flags);
		CHECK(v->gcache.first_engine == EngineID::Invalid());
		v->gv_flags = 0x8003;
		CHECK(v->rust_ground.state->flags == 0x8003);
		v->gcache.last_speed = 65535;
		CHECK(v->rust_ground.state->cache.last_speed == 65535);
	};
	REQUIRE(Vehicle::CanAllocateItem());
	check(new RoadVehicle());
	check(new (VehicleID(37)) Train());
	_vehicle_pool.CleanPool();
}
TEST_CASE("Vehicle resolver - boundary layouts and complete service table")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-vehicle_resolver.inc"
	CHECK(AbiServicesComplete(GetRustVehicleResolverServices()));
}
TEST_CASE("NewGRF cache - canonical native aliases, snapshots and indexed reuse")
{
	static_assert(std::is_trivially_destructible_v<NewGRFCache>);
	auto check = [](Vehicle *v) {
		CHECK(&v->grf_cache == v->rust_grf_cache.state);
		CHECK(v->grf_cache == NewGRFCache{});
		v->grf_cache.consist_cargo_information = 0x12345678;
		v->grf_cache.cache_valid = 31;
		CHECK(v->rust_grf_cache.state->consist_cargo_information == 0x12345678);
		NewGRFCache diagnostic = v->grf_cache;
		v->InvalidateNewGRFCache();
		CHECK(v->rust_grf_cache.state->cache_valid == 0);
		CHECK(diagnostic.cache_valid == 31);
		CHECK(v->grf_cache.consist_cargo_information == diagnostic.consist_cargo_information);
	};
	REQUIRE(Vehicle::CanAllocateItem());
	auto *v = new RoadVehicle();
	auto id = v->index;
	check(v);
	_vehicle_pool.CleanPool();
	v = new (id) RoadVehicle();
	check(v);
	check(new (VehicleID(37)) Train());
	_vehicle_pool.CleanPool();
	/* The base Effect shell has no engine/company pre-destructor requirements. */
	REQUIRE(Vehicle::CanAllocateItem());
	auto *effect = new Vehicle(VEH_EFFECT);
	id = effect->index;
	check(effect);
	delete effect;
	effect = new (id) Vehicle(VEH_EFFECT);
	check(effect);
	delete effect;
}
TEST_CASE("Road tick prologue - actual scalar places and nonfront entry")
{
	REQUIRE(Vehicle::CanAllocateItem());
	auto *v = new RoadVehicle();
	v->Record().subtype = 1 << GVSF_FRONT;
	{
		const auto read = RustRoadTickRead(v);
		static_assert(std::is_same_v<decltype(read.status), const uint8_t *>);
		static_assert(std::is_same_v<decltype(read.running), const decltype(v->Record().running_ticks) *>);
		static_assert(std::is_same_v<decltype(read.order_time), const std::remove_reference_t<decltype(v->current_order_time)> *>);
		CHECK(read.movement.ground == v->rust_ground.state);
		CHECK(static_cast<void *>(read.movement.ground) != static_cast<void *>(v->GetRustState()));
		CHECK(read.status == &v->Record().vehstatus);
		CHECK(read.running == &v->Record().running_ticks);
		CHECK(read.order_time == &v->current_order_time);
		v->Record().running_ticks = UINT8_MAX;
		v->current_order_time = INT32_MAX;
		v->SetVehStatus({VehState::Stopped});
		CHECK(*read.running == RustVehicleRunning(v));
		CHECK(*read.order_time == RustVehicleOrderTime(v));
		CHECK(*read.status == RustVehicleStatus(v));
	}
	v->Record().subtype = 0;
	v->Record().tick_counter = UINT8_MAX;
	CHECK(v->Tick()); // Actual nonfront entry consumes no front-only place or callback.
	CHECK(v->Record().tick_counter == 0);
	CHECK(v->Record().running_ticks == UINT8_MAX);
	CHECK(v->current_order_time == INT32_MAX);
	_vehicle_pool.CleanPool();
}
TEST_CASE("Road links - ground owners follow the current consist")
{
	REQUIRE(Vehicle::CanAllocateItem(3));
	auto *front = new RoadVehicle();
	auto *a = new RoadVehicle();
	auto *b = new RoadVehicle();
	const auto &leaves = GetRustRoadLeaves();
	for (auto *tail : {a, b}) {
		front->SetNext(tail);
		const auto link = leaves.link(RustVehicle(front));
		CHECK(link.next == RustVehicle(tail));
		CHECK(link.next_state == tail->GetRustState());
		CHECK(link.next_ground == tail->rust_ground.state);
		CHECK(static_cast<void *>(link.next_ground) != static_cast<void *>(link.next_state));
		const auto tile = leaves.link_tile(RustVehicle(front));
		CHECK(tile.link.next_ground == tail->rust_ground.state);
	}
	front->SetNext(nullptr);
	CHECK(leaves.link(RustVehicle(front)).next_ground == nullptr);
	_vehicle_pool.CleanPool();
}
TEST_CASE("Road visual eligibility precedes maximum-speed lookup")
{
	REQUIRE(Vehicle::CanAllocateItem());
	auto *road = new RoadVehicle(); road->Record().subtype = 1 << GVSF_FRONT;
	const auto saved_smoke = _settings_game.vehicle.smoke_amount;
	for (uint8_t smoke : {0, 1, 2}) for (uint8_t status : {0, 1, 2, 16, 128}) for (uint16_t speed : {0, 1, 2}) {
		_settings_game.vehicle.smoke_amount = smoke; road->SetVehStatus(VehStates{status}); road->Record().cur_speed = speed;
		CHECK(road->VisualEffectEligible() == (smoke != 0 && (status & 18) == 0 && speed >= 2));
	}
	_settings_game.vehicle.smoke_amount = saved_smoke; _vehicle_pool.CleanPool();
}

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
int bridge_reads;
uint16_t BridgeSpeed(uint32_t) noexcept { ++bridge_reads; return 37; }
}
TEST_CASE("Road max speed - front reversal, later curve assignment and hidden bridges")
{
	auto *front = openttd_rust_road_new(), *tail = openttd_rust_road_new();
	OpenTTDVehicle record{}, tail_record{};
	OpenTTDGroundState ground{};
	OpenTTDVehicleOrderState orders{};
	record.ground = &ground; record.orders = &orders; record.road = front;
	tail_record.road = tail;
	auto *v = &record;
	auto shared = OpenTTDSharedServices{};
	shared.bridge_speed = BridgeSpeed;
	auto leaves = OpenTTDRoadLeaves{};
	leaves.shared = &shared;
	for (uint16_t model = 0; model < 2; ++model) for (uint16_t state = 0; state < 256; ++state) for (uint16_t tail_state = 0; tail_state < 256; ++tail_state) {
		for (uint8_t hidden : {0, 1, 2, 3}) for (uint8_t direction : {0, 1}) {
			front->state = static_cast<uint8_t>(state); tail->state = static_cast<uint8_t>(tail_state);
			struct { uint16_t max_track_speed, order_max_speed; } r{};
			record.next = &tail_record;
			tail_record.next = nullptr;
			r.max_track_speed = static_cast<uint16_t>(tail_state * 257);
			r.order_max_speed = static_cast<uint16_t>(state * 257);
			record.direction = 1;
			record.vehstatus = static_cast<uint8_t>((hidden & 1) | 128);
			ground.cache.cached_max_track_speed = r.max_track_speed; orders.current.max_speed = r.order_max_speed;
			tail_record.direction = direction;
			tail_record.vehstatus = static_cast<uint8_t>((hidden >> 1) | 64);
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
			CHECK(openttd_rust_road_max_speed(v, front, model, &leaves) == std::min(expected, r.order_max_speed * 2));
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
		CHECK(v->gcache.first_engine == EngineID::Invalid().base());
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
TEST_CASE("Train record - native fields, canonical multihead links and indexed reuse")
{
	REQUIRE(Vehicle::CanAllocateItem());
	Train *front = new Train();
	Train *rear = new (VehicleID(37)) Train();
	CHECK(front->GetTrainFlags().base() == 0);
	CHECK(front->GetOtherMultiheadedPart() == nullptr);
	front->SetOtherMultiheadedPart(rear);
	rear->SetOtherMultiheadedPart(front);
	CHECK(front->GetRustState()->other_multiheaded_part == rear->record);
	CHECK(rear->GetOtherMultiheadedPart() == front);
	front->GetRustState()->flags = 0x8001;
	front->GetRustState()->cached_curve_speed_mod = -32768;
	CHECK(front->GetTrainFlags().base() == 0x8001);
	CHECK(front->GetCachedCurveSpeedMod() == -32768);
	front->SetCrashAnimPos(65535);
	CHECK(front->GetRustState()->crash_anim_pos == 65535);
	_vehicle_pool.CleanPool();
	rear = new (VehicleID(37)) Train();
	CHECK(rear->GetOtherMultiheadedPart() == nullptr);
	CHECK(rear->GetRustState()->crash_anim_pos == 0);
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
	CHECK(v->record->ground == v->rust_ground.state);
	CHECK(v->record->road == v->GetRustState());
	CHECK(v->record->consist == v->rust_orders.get());
	v->Record().running_ticks = UINT8_MAX;
	v->current_order_time = INT32_MAX;

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
	for (auto *tail : {a, b}) {
		front->SetNext(tail);
		CHECK(front->record->next == tail->record);
		CHECK(front->record->next->road == tail->GetRustState());
		CHECK(front->record->next->ground == tail->rust_ground.state);
		CHECK(static_cast<void *>(tail->record->ground) != static_cast<void *>(tail->record->road));
	}
	front->SetNext(nullptr);
	CHECK(front->record->next == nullptr);
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

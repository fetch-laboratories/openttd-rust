/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file train.cpp Train PBS depot and speed limit branches not observable in semantic saves. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../rust/train_ffi.h"
#include "../rust/pbs_ffi.h"
#include "../rust/train_reservation_ffi.h"
#include "../tile_type.h"
#include "../track_type.h"
#include "follower_fixture.hpp"

TEST_CASE("PBS depot early return does not read or write train_on_res")
{
	FollowerFixture map;
	map.Tile(23, MP_RAILWAY, 3 << 6);
	map.train.read_tile = [](OpenTTDTrainHandle) noexcept -> uint32_t { return 23; };
	map.train.read_trackdir = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainTrackdirRead { return {23, 1, 0}; };
	map.train.first_track = [](OpenTTDTrainHandle, uint8_t) noexcept -> uint64_t { return 0; };
	map.train.track_direction = [](OpenTTDTrainHandle, uint8_t, uint8_t) noexcept -> uint64_t { return 0; };
	map.pbs.depot_bits = [](uint32_t) noexcept { return TRACK_BIT_NONE; };
	openttd_rust_train_state_set_track(map.state.get(), TRACK_BIT_X);
	void *other = nullptr;
	const auto result = openttd_rust_pbs_follow_train(&map.pbs, {map.state.get(), map.state.get()}, reinterpret_cast<OpenTTDPbsTrainOutput *>(&other));
	CHECK(result.tile == 23); CHECK(result.td == 0); CHECK(result.okay == 0); CHECK(other == nullptr);
}

extern "C" void openttd_test_train_state_set_cached_max_curve_speed(OpenTTDTrainState *, uint16_t);
namespace {
bool _reservation_front;
unsigned _reservation_front_reads;
unsigned _reservation_assert_failures;
}

TEST_CASE("Train reservation - native front-engine assertion precedes depot return")
{
	for (bool assertions : {false, true}) for (bool front : {false, true}) {
		CAPTURE(assertions, front);
		FollowerFixture map;
		map.Tile(23, MP_RAILWAY, TRACK_BIT_X);
		map.pbs.assertions = assertions;
		_reservation_front = front;
		_reservation_front_reads = 0;
		_reservation_assert_failures = 0;
		map.train.read_front = [](OpenTTDTrainHandle) noexcept -> uint8_t { ++_reservation_front_reads; return _reservation_front; };
		map.movement.native_assert = [](bool okay) noexcept { if (!okay) ++_reservation_assert_failures; };
		auto leaves = OpenTTDTrainReservationLeaves{};
		leaves.shared = &map.shared;
		leaves.read_free = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainReservationFreeRead { return {23}; };
		leaves.trackdir = [](void *, OpenTTDTrainHandle) noexcept -> uint64_t { return TRACKDIR_X_NE; };
		leaves.is_station = [](void *, OpenTTDTrainHandle, uint32_t) noexcept -> uint64_t { return 0; };
		leaves.is_depot = [](void *, OpenTTDTrainHandle, uint32_t) noexcept -> uint64_t { return 1; };
		leaves.exit_dir = [](void *, OpenTTDTrainHandle, uint8_t) noexcept -> uint64_t { return DIAGDIR_NE; };
		leaves.depot_dir = [](void *, OpenTTDTrainHandle, uint32_t) noexcept -> uint64_t { return DIAGDIR_SW; };
		openttd_rust_train_reservation_free({map.Vehicle(), map.state.get()}, map.Vehicle(), &leaves, nullptr);
		CHECK(_reservation_front_reads == unsigned(assertions));
		CHECK(_reservation_assert_failures == unsigned(assertions && !front));
	}
}

namespace {
uint8_t _train_speed_status;
uint64_t _train_speed_model;
}

TEST_CASE("Train speed - depot break and wormhole limits")
{
	/* Abort defaults stand for every service the original loop must not reach. */
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> owner(openttd_rust_train_state_new(), openttd_rust_train_state_destroy);
	const OpenTTDTrainHandle handle{owner.get(), owner.get()};
	openttd_test_train_state_set_cached_max_curve_speed(owner.get(), 180);
	auto table = OpenTTDTrainServices{};
	table.read_current_max_speed = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainCurrentMaxSpeedRead { return {17, 200, 100}; };
	table.acc_model = [](OpenTTDTrainHandle) noexcept { return _train_speed_model; };
	table.is_station = [](OpenTTDTrainHandle, uint32_t) noexcept -> uint64_t { return 0; };
	table.read_status = [](OpenTTDTrainHandle) noexcept { return _train_speed_status; };
	table.read_tile = [](OpenTTDTrainHandle) noexcept -> uint32_t { return 23; };
	table.bridge_speed = [](OpenTTDTrainHandle, uint32_t tile) noexcept -> uint64_t {
		CHECK(tile == 23);
		return 90;
	};
	table.read_next = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainHandle { return {}; };
	table.order_max_speed = [](OpenTTDTrainHandle) noexcept -> uint64_t { return 160; };
	table.read_max_track_speed = [](OpenTTDTrainHandle) noexcept -> uint16_t { return 200; };
	const OpenTTDSharedServices shared{};
	struct Case { TrackBits track; uint8_t status; int speed; };
	for (uint64_t model : {0, 1}) {
		_train_speed_model = model;
		for (const Case test : {
			Case{TRACK_BIT_X, 0, 160},
			Case{TRACK_BIT_WORMHOLE, 1, 160},
			Case{TRACK_BIT_WORMHOLE, 0, 90},
			Case{TRACK_BIT_DEPOT, 0, model == 1 ? 61 : 160},
		}) {
			CAPTURE(model, test.track, test.status);
			openttd_rust_train_state_set_track(owner.get(), test.track);
			_train_speed_status = test.status;
			CHECK(openttd_rust_train_current_max_speed(handle, &table, &shared) == test.speed);
		}
	}
}
TEST_CASE("Train reservation - backoff 255 skips lookahead before owner reads")
{
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> owner(openttd_rust_train_state_new(), openttd_rust_train_state_destroy);
	auto leaves = OpenTTDTrainReservationLeaves{};
	leaves.backoff = []() noexcept -> uint8_t { return 255; };
	openttd_rust_train_reservation_check_next({owner.get(), owner.get()}, reinterpret_cast<OpenTTDVehicle *>(owner.get()), &leaves, nullptr);
	CHECK(openttd_rust_train_state_get_track(owner.get()) == 0);
}

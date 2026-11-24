/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file train.cpp Train PBS depot and speed limit branches not observable in semantic saves. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../rust/train_ffi.h"
#include "../rust/ground_vehicle_ffi.h"
#include "abi-record-ground.h"
#include "../rust/pbs_ffi.h"
#include "../rust/train_reservation_ffi.h"
#include "../rust/services_assert.h"
#include "../tile_type.h"
#include "../track_type.h"
#include "follower_fixture.hpp"

TEST_CASE("PBS depot early return does not read or write train_on_res")
{
	FollowerFixture map;
	map.Tile(23, MP_RAILWAY, 3 << 6);
	map.record.tile = 23;
	map.record.direction = 1;
	map.pbs.depot_bits = [](uint32_t) noexcept { return TRACK_BIT_NONE; };
	(map.state.get())->track = TRACK_BIT_X;
	void *other = nullptr;
	const auto result = openttd_rust_pbs_follow_train(&map.pbs, {&map.record, map.state.get()}, reinterpret_cast<OpenTTDPbsTrainOutput *>(&other));
	CHECK(result.tile == 23); CHECK(result.td == 0); CHECK(result.okay == 0); CHECK(other == nullptr);
}

namespace {
unsigned _reservation_assert_failures;
/* The assertion policy keeps the table it is bound to, so the binding's is static. */
OpenTTDMapStatics _reservation_assert_map{};
OpenTTDSharedServices _reservation_assert_shared{};
}

TEST_CASE("Train reservation - native front-engine assertion precedes depot return")
{
	for (bool assertions : {false, true}) for (bool front : {false, true}) {
		CAPTURE(assertions, front);
		FollowerFixture map;
		map.Tile(23, MP_RAILWAY, (3 << 6) | DIAGDIR_SW); // A rail depot facing away from the train.
		map.record.subtype = front ? 1 : 0;
		map.record.tile = 23;
		map.record.direction = DIR_NE;
		(map.state.get())->track = TRACK_BIT_X;
		_reservation_assert_failures = 0;
		_reservation_assert_map.assertions = assertions;
		_reservation_assert_shared.map = &_reservation_assert_map;
		openttd_rust_assert_bind([](bool okay) noexcept { if (!okay) ++_reservation_assert_failures; }, &_reservation_assert_shared);
		openttd_rust_train_reservation_free({map.Vehicle(), map.state.get()}, &map.train, &map.shared);
		openttd_rust_assert_bind(RustAssertCondition, &GetRustSharedServices());
		CHECK(_reservation_assert_failures == unsigned(assertions && !front));
	}
}

TEST_CASE("Train speed - depot break and wormhole limits")
{
	/* Abort defaults stand for every service the original loop must not reach. */
	FollowerFixture map;
	OpenTTDGroundState ground{};
	map.record.ground = &ground;
	map.record.tile = 23;
	ground.cache.cached_max_track_speed = 200;
	map.record.cur_speed = 100;
	map.order_state.current.max_speed = 160;
	const OpenTTDTrainHandle handle{map.Vehicle(), map.state.get()};
	map.state->cached_max_curve_speed = 180;
	uint8_t acceleration = 0;
	OpenTTDSettingAddresses settings{};
	settings.train_acceleration_model = &acceleration;
	map.shared.settings = &settings;
	map.shared.bridge_speed = [](uint32_t tile) noexcept -> uint16_t {
		CHECK(tile == 23);
		return 90;
	};
	struct Case { TrackBits track; uint8_t status; int speed; };
	for (uint64_t model : {0, 1}) {
		acceleration = static_cast<uint8_t>(model);
		for (const Case test : {
			Case{TRACK_BIT_X, 0, 160},
			Case{TRACK_BIT_WORMHOLE, 1, 160},
			Case{TRACK_BIT_WORMHOLE, 0, 90},
			Case{TRACK_BIT_DEPOT, 0, model == 1 ? 61 : 160},
		}) {
			CAPTURE(model, test.track, test.status);
			map.state->track = test.track;
			map.record.vehstatus = test.status;
			CHECK(openttd_rust_train_current_max_speed(handle, &map.train, &map.shared) == test.speed);
		}
	}
}
TEST_CASE("Train reservation - backoff 255 skips lookahead before owner reads")
{
	FollowerFixture map;
	uint8_t backoff = 255;
	OpenTTDSettingAddresses settings{};
	settings.path_backoff_interval = &backoff;
	map.shared.settings = &settings;
	openttd_rust_train_reservation_check_next({map.Vehicle(), map.state.get()}, &map.train, &map.shared);
	CHECK((map.state.get())->track == 0);
}

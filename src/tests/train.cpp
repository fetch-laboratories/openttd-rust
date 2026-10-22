/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file train.cpp Train speed observation order not observable in semantic saves. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../rust/train_ffi.h"
#include "../track_type.h"

namespace {
std::string _train_speed_reads;
uint8_t _train_speed_status;
uint64_t _train_speed_model;
}

TEST_CASE("Train speed - depot break and wormhole read order")
{
	/* Semantic saves cover live train movement but cannot distinguish extra or
	 * reordered read-only services. This calls the production Rust entry with
	 * abort defaults for every service the original loop must not reach. */
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> owner(openttd_rust_train_state_new(), openttd_rust_train_state_destroy);
	const OpenTTDTrainHandle handle{owner.get(), owner.get()};
	openttd_rust_train_state_set_cached_max_curve_speed(owner.get(), 180);
	OpenTTDTrainServices table{};
	table.read_current_max_speed = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainCurrentMaxSpeedRead { return {17, 200, 100}; };
	table.acc_model = [](OpenTTDTrainHandle) noexcept { return _train_speed_model; };
	table.is_station = [](OpenTTDTrainHandle, uint32_t) noexcept -> uint64_t { return 0; };
	table.read_status = [](OpenTTDTrainHandle) noexcept {
		_train_speed_reads += 'h';
		return _train_speed_status;
	};
	table.read_tile = [](OpenTTDTrainHandle) noexcept -> uint32_t {
		_train_speed_reads += 't';
		return 23;
	};
	table.bridge_speed = [](OpenTTDTrainHandle, uint32_t tile) noexcept -> uint64_t {
		_train_speed_reads += 'b';
		CHECK(tile == 23);
		return 90;
	};
	table.read_next = [](OpenTTDTrainHandle) noexcept -> OpenTTDTrainHandle {
		_train_speed_reads += 'n';
		return {};
	};
	table.order_max_speed = [](OpenTTDTrainHandle) noexcept -> uint64_t {
		_train_speed_reads += 'o';
		return 160;
	};
	table.read_max_track_speed = [](OpenTTDTrainHandle) noexcept -> uint16_t {
		_train_speed_reads += 'm';
		return 200;
	};
	const OpenTTDSharedServices shared{};
	struct Case { TrackBits track; uint8_t status; const char *reads; int speed; };
	for (uint64_t model : {0, 1}) {
		_train_speed_model = model;
		for (const Case test : {
			Case{TRACK_BIT_X, 0, "nom", 160},
			Case{TRACK_BIT_WORMHOLE, 1, "hnom", 160},
			Case{TRACK_BIT_WORMHOLE, 0, "htbnom", 90},
			Case{TRACK_BIT_DEPOT, 0, model == 1 ? "om" : "nom", model == 1 ? 61 : 160},
		}) {
			CAPTURE(model, test.track, test.status);
			openttd_rust_train_state_set_track(owner.get(), test.track);
			_train_speed_status = test.status;
			_train_speed_reads.clear();
			CHECK(openttd_rust_train_current_max_speed(handle, &table, &shared) == test.speed);
			CHECK(_train_speed_reads == test.reads);
		}
	}
}
#endif

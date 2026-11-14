/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follower_fixture.hpp Concrete tile layouts for production follower/search fixtures. */
#ifndef FOLLOWER_FIXTURE_HPP
#define FOLLOWER_FIXTURE_HPP
#include "../rust/movement_ffi.h"
#include "../rust/vehicle_record_ffi.h"
#include "fixture_map.hpp"
#include "../rust/ground_vehicle_ffi.h"
#include "../rust/train_ffi.h"
#include "../rust/pbs_ffi.h"
#include "../tile_storage_type.h"
#include "../vehicle_type.h"
#include "../transport_type.h"
#include "../rust/train_state_ffi.h"
#include "../rail_map.h"
#include "../road_map.h"
#include "../tile_map.h"
#include <vector>
#include <memory>

struct FollowerFixture {
	inline static FollowerFixture *active;
	OpenTTDFixtureMap tiles;
	OpenTTDSharedServices shared{};
	OpenTTDMovementServices movement{};
	OpenTTDVehicleServices vehicle{};
	OpenTTDVehicle record{};
	OpenTTDOrdersLeaves orders{};
	OpenTTDVehicleOrderState order_state{};
	OpenTTDGroundPhysicsServices physics{};
	OpenTTDTrainServices train{};
	OpenTTDRailMapServices rail{};
	OpenTTDRailServices ground{};
	OpenTTDPbsServices pbs{};
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> state{openttd_rust_train_state_new(), openttd_rust_train_state_destroy};
	explicit FollowerFixture(uint32_t width = 64, uint32_t height = 64) : tiles(std::countr_zero(width), width * height) {
		active = this;
		record.shell = reinterpret_cast<OpenTTDVehicleShell *>(this);
		record.train = state.get();
		record.first = &record;
		record.type = VEH_TRAIN;
		record.owner = 0;
		record.orders = &order_state;
		vehicle.record_from_shell = [](void *) noexcept { return &active->record; };
		shared.map = &tiles.statics;
		shared.tile_type = [](uint32_t t) noexcept -> uint8_t { return t < active->tiles.size ? active->tiles.base[t].type >> 4 : MP_VOID; };
		shared.is_plain_rail = [](uint32_t t) noexcept { return active->tiles.base[t].m5 >> 6 <= 1; };
		shared.forbid_90_deg = []() noexcept { return false; };
		shared.mark_dirty = [](uint32_t) noexcept {};
		shared.movement = &movement;
		shared.pbs_services = [](const OpenTTDSharedServices *) noexcept -> const OpenTTDPbsServices * { return &active->pbs; };
		shared.may_have_road = [](uint32_t) noexcept { return true; };
		movement.native_assert = [](bool okay) noexcept { if (!okay) std::abort(); };
		movement.shared = &shared; movement.vehicle = &vehicle; movement.train = &train;

		vehicle.train_handle = [](OpenTTDVehicle *v) noexcept -> OpenTTDTrainHandle { return {v, active->state.get()}; };
		movement.road_kind = [](OpenTTDVehicle *) noexcept { return RTT_ROAD; };
		movement.road_compatible = [](OpenTTDVehicle *) noexcept -> uint64_t { return 1; };
		movement.road_type_speed = [](RoadType) noexcept -> uint16_t { return 0; };
		order_state.current.max_speed = 40;
		orders.vehicle = [](void *) noexcept { return &active->order_state; };
		movement.orders = &orders;
		ground.tracks = [](uint32_t t) noexcept { return static_cast<TrackBits>(active->tiles.base[t].m5 & 63); };
		rail.rail_reservation = [](uint32_t t) noexcept { return static_cast<TrackBits>(active->tiles.base[t].m2 & 63); };
		physics.rail_speed = [](RailType) noexcept -> uint16_t { return 0; };
		train.physics = &physics;
		pbs.shared = &shared; pbs.rail = &rail; pbs.train = &train;
		pbs.handle = [](const ::Train *) noexcept -> OpenTTDTrainHandle { return {&active->record, active->state.get()}; };
		pbs.rail90 = [](uint8_t, uint8_t, bool def) noexcept { return def; };
		shared.all_compatible_railtypes = [](uint64_t types) noexcept { return types; };
		pbs.compatible = [](uint8_t type) noexcept -> uint64_t { return uint64_t{1} << type; };
		pbs.show_reservation = []() noexcept { return false; };
		(state.get())->compatible_railtypes = 1;
		(state.get())->railtypes = 1;
	}
	void Tile(uint32_t t, uint8_t kind, uint8_t bits = 0) { tiles.base[t] = {}; tiles.extended[t] = {}; tiles.base[t].type = kind << 4; tiles.base[t].m5 = bits; }
	OpenTTDVehicle *Vehicle() { return &record; }
};
#endif

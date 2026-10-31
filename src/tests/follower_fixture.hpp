/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follower_fixture.hpp Concrete tile layouts for production follower/search fixtures. */
#ifndef FOLLOWER_FIXTURE_HPP
#define FOLLOWER_FIXTURE_HPP
#include "../rust/movement_ffi.h"
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
	uint32_t width;
	std::vector<TileBaseStorage> base;
	std::vector<TileExtendedStorage> extended;
	OpenTTDSharedServices shared{};
	OpenTTDMovementServices movement{};
	OpenTTDVehicleServices vehicle{};
	OpenTTDOrdersLeaves orders{};
	OpenTTDVehicleOrderState order_state{};
	OpenTTDGroundPhysicsServices physics{};
	OpenTTDTrainServices train{};
	OpenTTDRailMapServices rail{};
	OpenTTDRailServices ground{};
	OpenTTDPbsServices pbs{};
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> state{openttd_rust_train_state_new(), openttd_rust_train_state_destroy};
	explicit FollowerFixture(uint32_t width = 64, uint32_t height = 64) : width(width), base(width * height), extended(width * height) {
		active = this;
		shared.map_log_x = []() noexcept -> uint32_t { return std::countr_zero(active->width); };
		shared.tile_type = [](uint32_t t) noexcept -> uint8_t { return t < active->base.size() ? active->base[t].type >> 4 : MP_VOID; };
		shared.is_plain_rail = [](uint32_t t) noexcept { return active->base[t].m5 >> 6 <= 1; };
		shared.forbid_90_deg = []() noexcept { return false; };
		shared.mark_dirty = [](uint32_t) noexcept {};
		shared.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &active->movement; };
		shared.pbs_services = [](const OpenTTDSharedServices *) noexcept -> const OpenTTDPbsServices * { return &active->pbs; };
		shared.normal_road_bits = [](uint32_t t, uint8_t tram) noexcept { return static_cast<RoadBits>((tram ? active->base[t].m3 : active->base[t].m5) & 15); };
		shared.road_tile_kind = [](uint32_t t) noexcept { return static_cast<RoadTileType>(active->base[t].m5 >> 6); };
		shared.road_depot_direction = [](uint32_t t) noexcept -> uint8_t { return active->base[t].m5 & 3; };
		shared.has_road_works = [](uint32_t) noexcept { return false; };
		shared.may_have_road = [](uint32_t) noexcept { return true; };
		movement.tile_types = []() noexcept -> OpenTTDTileTypeAllocation { return {active->base.data(), active->extended.data(), static_cast<uint32_t>(active->base.size()), true}; };
		movement.native_assert = [](bool okay) noexcept { if (!okay) std::abort(); };
		movement.shared = &shared; movement.vehicle = &vehicle; movement.train = &train;
		movement.rail_map = &rail; movement.rail = &ground;
		vehicle.owner = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
		vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return VEH_TRAIN; };
		vehicle.train_handle = [](OpenTTDVehicle *v) noexcept -> OpenTTDTrainHandle { return {v, active->state.get()}; };
		movement.road_kind = [](OpenTTDVehicle *) noexcept { return RTT_ROAD; };
		movement.road_compatible = [](OpenTTDVehicle *) noexcept -> uint64_t { return 1; };
		movement.road_type_speed = [](RoadType) noexcept -> uint16_t { return 0; };
		order_state.current.max_speed = 40;
		orders.vehicle = [](void *) noexcept { return &active->order_state; };
		movement.orders = &orders;
		movement.normal_road = [](uint32_t t) noexcept { return active->base[t].m5 >> 6 == 0; };
		movement.has_tile_road_type = [](uint32_t, RoadTramType) noexcept { return true; };
		movement.normal_road_status_tail = [](uint32_t t) noexcept -> OpenTTDNormalRoadStatusTail { return {static_cast<uint8_t>((active->base[t].m5 >> 4) & 3), false}; };
		movement.water_type = [](uint32_t t) noexcept { return static_cast<WaterTileType>(active->base[t].m5 >> 4); };
		movement.tile_flat = [](uint32_t) noexcept { return true; };
		movement.pixel_location = [](int32_t x, int32_t y) noexcept -> OpenTTDPixelLocation {
			const uint32_t t = (uint32_t(y) >> 4) * active->width + (uint32_t(x) >> 4);
			return {t, active->shared.tile_type(t)};
		};
		movement.pixel_slope = [](uint32_t) noexcept -> OpenTTDFoundationSlope { return {0, SLOPE_FLAT}; };
		movement.max_pixel_z = [](uint32_t) noexcept -> int32_t { return 0; };
		ground.tracks = [](uint32_t t) noexcept { return static_cast<TrackBits>(active->base[t].m5 & 63); };
		rail.rail_tile_type = [](uint32_t t) noexcept { return static_cast<RailTileType>(active->base[t].m5 >> 6); };
		rail.is_rail_depot = [](uint32_t t) noexcept { return active->base[t].m5 >> 6 == 3; };
		rail.has_signal_trackdir = [](uint32_t, uint8_t) noexcept { return false; };
		rail.rail_reservation = [](uint32_t t) noexcept { return static_cast<TrackBits>(active->base[t].m2 & 63); };
		physics.rail_speed = [](RailType) noexcept -> uint16_t { return 0; };
		physics.display_max_speed = [](OpenTTDVehicle *) noexcept -> int32_t { return 100; };
		train.physics = &physics; train.read_owner = [](OpenTTDTrainHandle) noexcept -> uint8_t { return 0; };
		pbs.shared = &shared; pbs.rail = &rail; pbs.train = &train;
		pbs.handle = [](const ::Train *v) noexcept -> OpenTTDTrainHandle { return {const_cast<::Train *>(v), active->state.get()}; };
		pbs.rail90 = [](uint8_t, uint8_t, bool def) noexcept { return def; };
		pbs.all_compatible = [](uint64_t types) noexcept { return types; };
		pbs.compatible = [](uint8_t type) noexcept -> uint64_t { return uint64_t{1} << type; };
		pbs.show_reservation = []() noexcept { return false; };
		openttd_rust_train_state_set_compatible_railtypes(state.get(), 1);
		openttd_rust_train_state_set_railtypes(state.get(), 1);
	}
	void Tile(uint32_t t, uint8_t kind, uint8_t bits = 0) { base[t] = {}; extended[t] = {}; base[t].type = kind << 4; base[t].m5 = bits; }
	OpenTTDVehicle *Vehicle() { return reinterpret_cast<OpenTTDVehicle *>(this); }
};
#endif

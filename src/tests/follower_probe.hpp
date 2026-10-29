/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follower_probe.hpp Native-test bridge to production Rust follower methods. */
#ifndef FOLLOWER_PROBE_HPP
#define FOLLOWER_PROBE_HPP
#include "../pathfinder/follow_track.hpp"
#include "../rust/follow_track_ffi.h"
extern "C" {
void openttd_test_follower_init(OpenTTDFollowTrackState *, const OpenTTDSharedServices *, uint8_t, Vehicle *, uint8_t, uint64_t, bool, bool, bool);
bool openttd_test_follower_follow(const OpenTTDSharedServices *, OpenTTDFollowTrackState *, uint32_t, uint8_t);
bool openttd_test_follower_mask(const OpenTTDSharedServices *, OpenTTDFollowTrackState *);
int32_t openttd_test_follower_speed(const OpenTTDSharedServices *, uint8_t, Vehicle *, uint32_t);
}
/** Evidence adapter calling the unchanged Rust follower implementation. */
template <TransportType Ttr_type_, typename VehicleType, bool T90deg_turns_allowed_ = true, bool Tmask_reserved_tracks = false>
struct FollowerProbeT : OpenTTDFollowTrackState {
	enum ErrorCode : uint8_t { EC_NONE, EC_OWNER, EC_RAIL_ROAD_TYPE, EC_90DEG, EC_NO_WAY, EC_RESERVED };
	explicit FollowerProbeT(const VehicleType *v = nullptr, RailTypes types = INVALID_RAILTYPES) { Init(v, types); }
	FollowerProbeT(Owner owner, RailTypes types = INVALID_RAILTYPES) {
		assert(IsRailTT());
		openttd_test_follower_init(this, &GetRustSharedServices(), TT(), nullptr, owner.base(), types.base(), Allow90degTurns(), DoTrackMasking(), false);
	}
	void Init(const VehicleType *v, RailTypes types) {
		openttd_test_follower_init(this, &GetRustSharedServices(), TT(), const_cast<Vehicle *>(static_cast<const Vehicle *>(v)), 0, types.base(), Allow90degTurns(), DoTrackMasking(), true);
	}
	void Init(Owner owner, RailTypes types) {
		openttd_test_follower_init(this, &GetRustSharedServices(), TT(), const_cast<Vehicle *>(this->veh), owner.base(), types.base(), Allow90degTurns(), DoTrackMasking(), false);
	}
	static constexpr TransportType TT() { return Ttr_type_; }
	static constexpr bool IsWaterTT() { return TT() == TRANSPORT_WATER; }
	static constexpr bool IsRailTT() { return TT() == TRANSPORT_RAIL; }
	static constexpr bool IsRoadTT() { return TT() == TRANSPORT_ROAD; }
	static constexpr bool Allow90degTurns() { return T90deg_turns_allowed_; }
	static constexpr bool DoTrackMasking() { return Tmask_reserved_tracks; }
	bool Follow(TileIndex tile, Trackdir td) { return openttd_test_follower_follow(&GetRustSharedServices(), this, tile.base(), td); }
	bool MaskReservedTracks() { return openttd_test_follower_mask(&GetRustSharedServices(), this); }
	int GetSpeedLimit(int *min = nullptr) const { return GetSpeedLimit(this->old_tile, static_cast<const VehicleType *>(this->veh), min); }
	static int GetSpeedLimit(TileIndex tile, const VehicleType *v, int *min = nullptr) {
		int max = openttd_test_follower_speed(&GetRustSharedServices(), TT(), const_cast<Vehicle *>(static_cast<const Vehicle *>(v)), tile.base());
		if (min != nullptr) *min = 0;
		return max;
	}
};
typedef FollowerProbeT<TRANSPORT_WATER, Ship,        true > FollowerProbeWater;
typedef FollowerProbeT<TRANSPORT_ROAD,  RoadVehicle, true > FollowerProbeRoad;
typedef FollowerProbeT<TRANSPORT_RAIL,  Train,       true > FollowerProbeRail;

typedef FollowerProbeT<TRANSPORT_RAIL,  Train,       false> FollowerProbeRailNo90;

typedef FollowerProbeT<TRANSPORT_RAIL, Train, true,  true > FollowerProbeFreeRail;
typedef FollowerProbeT<TRANSPORT_RAIL, Train, false, true > FollowerProbeFreeRailNo90;


#endif

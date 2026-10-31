/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follower_probe.hpp Native-test bridge to production Rust follower methods. */
#ifndef FOLLOWER_PROBE_HPP
#define FOLLOWER_PROBE_HPP
#include "../pathfinder/follow_track.hpp"
#include "../rust/services_ffi.h"
#include <memory>
extern "C" {
/** Opaque boxed Rust follower. */
struct OpenTTDTestFollower;
/** Follower fields read after each call. */
struct OpenTTDTestFollowerOutput {
	bool followed;
	uint32_t old_tile, new_tile;
	int32_t tiles_skipped;
	uint16_t new_td_bits;
	uint8_t old_td, exitdir;
	bool is_tunnel, is_bridge, is_station;
	uint8_t err;
};
OpenTTDTestFollower *openttd_test_follower_new(const OpenTTDSharedServices *, uint8_t, Vehicle *, uint8_t, uint64_t, bool, bool);
void openttd_test_follower_delete(OpenTTDTestFollower *);
OpenTTDTestFollowerOutput openttd_test_follower_follow(const OpenTTDSharedServices *, OpenTTDTestFollower *, uint32_t, uint8_t);
OpenTTDTestFollowerOutput openttd_test_follower_mask(const OpenTTDSharedServices *, OpenTTDTestFollower *);
int32_t openttd_test_follower_speed(const OpenTTDSharedServices *, uint8_t, Vehicle *, uint32_t);
uint32_t openttd_test_platform_length(const OpenTTDSharedServices *, BaseStation *, uint32_t, uint8_t, bool);
}
using OpenTTDTestFollowerPtr = std::unique_ptr<OpenTTDTestFollower, decltype(&openttd_test_follower_delete)>;
/** A production follower over `shared`, constructed from a vehicle or an owner. */
inline OpenTTDTestFollowerPtr OpenTTDTestFollowerNew(const OpenTTDSharedServices &shared, TransportType mode, Vehicle *v, uint8_t owner, uint64_t types, bool allow90, bool from_vehicle)
{
	return {openttd_test_follower_new(&shared, mode, v, owner, types, allow90, from_vehicle), openttd_test_follower_delete};
}
/** Typed outputs of one production follow; failure clears the trackdirs. */
struct OpenTTDTestFollow {
	TileIndex new_tile;
	TrackdirBits new_td_bits;
	int tiles_skipped;
};
inline OpenTTDTestFollow OpenTTDTestFollowOnce(const OpenTTDSharedServices &shared, TransportType mode, Vehicle *v, uint8_t owner, uint64_t types, bool from_vehicle, uint32_t tile, uint8_t td)
{
	const auto out = openttd_test_follower_follow(&shared, OpenTTDTestFollowerNew(shared, mode, v, owner, types, true, from_vehicle).get(), tile, td);
	return {TileIndex{out.new_tile}, out.followed ? static_cast<TrackdirBits>(out.new_td_bits) : TRACKDIR_BIT_NONE, out.tiles_skipped};
}
/** Evidence adapter calling the unchanged Rust follower implementation. */
template <TransportType Ttr_type_, typename VehicleType, bool T90deg_turns_allowed_ = true, bool Tmask_reserved_tracks = false>
struct FollowerProbeT {
	enum ErrorCode : uint8_t { EC_NONE, EC_OWNER, EC_RAIL_ROAD_TYPE, EC_90DEG, EC_NO_WAY, EC_RESERVED };
	OpenTTDTestFollowerPtr state{nullptr, openttd_test_follower_delete};
	const Vehicle *veh = nullptr;
	TileIndex old_tile = INVALID_TILE, new_tile = INVALID_TILE;
	Trackdir old_td = INVALID_TRACKDIR;
	TrackdirBits new_td_bits = TRACKDIR_BIT_NONE;
	DiagDirection exitdir = INVALID_DIAGDIR;
	bool is_tunnel = false, is_bridge = false, is_station = false;
	int tiles_skipped = 0;
	ErrorCode err = EC_NONE;
	explicit FollowerProbeT(const VehicleType *v = nullptr, RailTypes types = INVALID_RAILTYPES) { Init(v, types); }
	FollowerProbeT(Owner owner, RailTypes types = INVALID_RAILTYPES) {
		assert(IsRailTT());
		Init(owner, types);
	}
	void Init(const VehicleType *v, RailTypes types) {
		this->veh = v;
		this->state = OpenTTDTestFollowerNew(GetRustSharedServices(), TT(), const_cast<Vehicle *>(this->veh), 0, types.base(), Allow90degTurns(), true);
	}
	void Init(Owner owner, RailTypes types) {
		this->state = OpenTTDTestFollowerNew(GetRustSharedServices(), TT(), const_cast<Vehicle *>(this->veh), owner.base(), types.base(), Allow90degTurns(), false);
	}
	static constexpr TransportType TT() { return Ttr_type_; }
	static constexpr bool IsWaterTT() { return TT() == TRANSPORT_WATER; }
	static constexpr bool IsRailTT() { return TT() == TRANSPORT_RAIL; }
	static constexpr bool IsRoadTT() { return TT() == TRANSPORT_ROAD; }
	static constexpr bool Allow90degTurns() { return T90deg_turns_allowed_; }
	static constexpr bool DoTrackMasking() { return Tmask_reserved_tracks; }
	bool Follow(TileIndex tile, Trackdir td) { return this->Read(openttd_test_follower_follow(&GetRustSharedServices(), this->state.get(), tile.base(), td)); }
	bool MaskReservedTracks() { return !DoTrackMasking() || this->Read(openttd_test_follower_mask(&GetRustSharedServices(), this->state.get())); }
	int GetSpeedLimit(int *min = nullptr) const { return GetSpeedLimit(this->old_tile, static_cast<const VehicleType *>(this->veh), min); }
	static int GetSpeedLimit(TileIndex tile, const VehicleType *v, int *min = nullptr) {
		int max = openttd_test_follower_speed(&GetRustSharedServices(), TT(), const_cast<Vehicle *>(static_cast<const Vehicle *>(v)), tile.base());
		if (min != nullptr) *min = 0;
		return max;
	}
private:
	bool Read(const OpenTTDTestFollowerOutput &out) {
		this->old_tile = TileIndex{out.old_tile}; this->new_tile = TileIndex{out.new_tile};
		this->old_td = static_cast<Trackdir>(out.old_td); this->new_td_bits = static_cast<TrackdirBits>(out.new_td_bits);
		this->exitdir = static_cast<DiagDirection>(out.exitdir); this->tiles_skipped = out.tiles_skipped;
		this->is_tunnel = out.is_tunnel; this->is_bridge = out.is_bridge; this->is_station = out.is_station;
		this->err = static_cast<ErrorCode>(out.err);
		return out.followed;
	}
};
typedef FollowerProbeT<TRANSPORT_WATER, Ship,        true > FollowerProbeWater;
typedef FollowerProbeT<TRANSPORT_ROAD,  RoadVehicle, true > FollowerProbeRoad;
typedef FollowerProbeT<TRANSPORT_RAIL,  Train,       true > FollowerProbeRail;

typedef FollowerProbeT<TRANSPORT_RAIL,  Train,       false> FollowerProbeRailNo90;

typedef FollowerProbeT<TRANSPORT_RAIL, Train, true,  true > FollowerProbeFreeRail;
typedef FollowerProbeT<TRANSPORT_RAIL, Train, false, true > FollowerProbeFreeRailNo90;


#endif

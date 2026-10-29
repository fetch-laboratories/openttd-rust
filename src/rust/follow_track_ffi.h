/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follow_track_ffi.h Stack-owned follower facade. */
#ifndef OPENTTD_RUST_FOLLOW_TRACK_FFI_H
#define OPENTTD_RUST_FOLLOW_TRACK_FFI_H
#include "services_ffi.h"
#include "../vehicle_type.h"
#include "../tile_type.h"
#include "../track_type.h"
#include "../rail_type.h"
#include "../direction_type.h"
#include "../company_type.h"
extern "C" {
struct OpenTTDFollowTrackState {
	const Vehicle *veh;
	RailTypes railtypes;
	TileIndex old_tile, new_tile;
	int32_t tiles_skipped;
	TrackdirBits new_td_bits;
	Owner veh_owner;
	Trackdir old_td;
	DiagDirection exitdir;
	bool is_tunnel, is_bridge, is_station;
	uint8_t err, mode;
	bool allow90, masking;
};
void openttd_rust_follower_init(OpenTTDFollowTrackState *, const OpenTTDSharedServices *, uint8_t, Vehicle *, uint8_t, uint64_t, bool, bool, bool);
bool openttd_rust_follower_follow(const OpenTTDSharedServices *, OpenTTDFollowTrackState *, uint32_t, uint8_t);
bool openttd_rust_follower_mask(const OpenTTDSharedServices *, OpenTTDFollowTrackState *);
int32_t openttd_rust_follower_speed(const OpenTTDSharedServices *, uint8_t, Vehicle *, uint32_t);
uint32_t openttd_rust_platform_length(const OpenTTDSharedServices *, uint16_t, uint32_t, uint8_t, bool);
}
#endif

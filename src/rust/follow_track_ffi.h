/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file follow_track_ffi.h Platform length entry and layout-checked follower evidence record. */
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
uint32_t openttd_rust_platform_length(const OpenTTDSharedServices *, uint16_t, uint32_t, uint8_t, bool);
}
#endif

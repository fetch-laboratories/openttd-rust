/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file yapf_rail.cpp The rail pathfinding. */

#include "../../stdafx.h"

#include "yapf.hpp"
#include "yapf_cache.h"
#include "yapf_node_rail.hpp"
#include "yapf_costrail.hpp"
#include "yapf_destrail.hpp"
#include "../../viewport_func.h"
#include "../../newgrf_station.h"

#include "../../safeguards.h"

#include "../../rust/rail_yapf_ffi.h"
#include "../../rust/services_yapf.h"
#include "../../rust/services_map.h"
#include "../../rust/services_rail.h"
#include "../../rust/services_station.h"
#include "../../rust/vehicle_services.h"
#include <cstdio>

struct RailYapfContext {
	const Train *train;
	bool *path_found = nullptr;
	PBSTileInfo *target = nullptr;
	TileIndex *destination = nullptr;
};
static OpenTTDRailTrain RailTrain(void *context) noexcept
{
	const Train *v = static_cast<RailYapfContext *>(context)->train;
	const Train *rear = v->Last();
	return {v->GetCompatibleRailTypes().base(), RustAllCompatibleRailTypes(v->GetRailTypes().base()),
		v->GetTile().base(), rear->GetTile().base(), TileVirtXY(v->Record().x_pos, v->Record().y_pos).base(), TileVirtXY(rear->Record().x_pos, rear->Record().y_pos).base(),
		v->GetDestTile().base(), RustGroundCachedTotalLength(&v->gcache), static_cast<uint32_t>(std::min<int>(RustVehicleDisplayMaxSpeed(v), RustVehicleOrderMaxSpeed(v))),
		v->current_order.GetDestination().base(), GetRustSharedServices().train_trackdir(v), GetRustSharedServices().train_trackdir(rear),
		static_cast<uint8_t>(v->GetTrack() == TRACK_BIT_WORMHOLE), static_cast<uint8_t>(rear->GetTrack() == TRACK_BIT_WORMHOLE),
		static_cast<uint8_t>(v->current_order.GetType()), static_cast<uint8_t>(v->current_order.GetDepotActionType().Test(OrderDepotActionFlag::NearestDepot)),
		static_cast<uint8_t>(v->current_order.IsType(OT_GOTO_WAYPOINT) && !Waypoint::Get(v->current_order.GetDestination().ToStationID())->IsSingleTile())};
}
static OpenTTDRailTile RailTile(uint32_t tile, uint8_t direction) noexcept
{
	const TileIndex t{tile};
	const Trackdir td = static_cast<Trackdir>(direction);
	OpenTTDRailTile result{};
	result.flags = (IsTileType(t, MP_RAILWAY) ? 1U : 0U) | (IsTileType(t, MP_ROAD) ? 2U : 0U)
		| (HasStationTileRail(t) ? 4U : 0U) | (RustIsRailStation(tile) ? 8U : 0U)
		| (IsRailWaypointTile(t) ? 16U : 0U) | (IsRailDepotTile(t) ? 32U : 0U)
		| (IsPlainRailTile(t) ? 64U : 0U) | (IsLevelCrossingTile(t) ? 128U : 0U)
		| (IsBridgeTile(t) ? 256U : 0U) | (RustIsTunnelTile(tile) ? 512U : 0U);
	result.railtype = RustTileRailType(tile);
	if (IsPlainRailTile(t)) result.tracks = RustRailTracks(tile);
	if (HasStationTileRail(t)) {
		result.station = MapSvcStationIndex(tile);
		result.station_track = RustRailStationTrack(tile);
		if (RustIsRailStation(tile) && RustStationReserved(tile)) result.flags |= 1024U;
	}
	if (IsTileType(t, MP_TUNNELBRIDGE)) {
		result.tunnel_dir = GetTunnelBridgeDirection(t);
		result.other_end = RustOtherTunnelBridgeEnd(tile);
	}
	result.uphill = IsUphillTrackdir(GetTileSlope(t), td);
	if (IsTileType(t, MP_RAILWAY)) {
		result.signal_along = RustRailHasSignalTrackdir(tile, td);
		result.signal_against = RustRailHasSignalTrackdir(tile, ReverseTrackdir(td));
		if (result.signal_along || result.signal_against) {
			result.signal_type = RustRailSignalType(tile, TrackdirToTrack(td));
			result.oneway = RustRailOnewaySignal(tile, TrackdirToTrack(td));
		}
		if (result.signal_along) result.signal_green = RustRailSignalState(tile, td) == SIGNAL_STATE_GREEN;
	}
	if (RustOnewaySignalBlocking(tile, td)) result.flags |= 2048U;
	return result;
}
static uint32_t RailClosestStation(void *context, uint8_t station) noexcept
{
	const Train *v = static_cast<RailYapfContext *>(context)->train;
	return CalcClosestStationTile(v->current_order.GetDestination().ToStationID(), v->GetTile(), station ? StationType::Rail : StationType::RailWaypoint).base();
}
static void RailOutput(void *context, uint8_t mask, const OpenTTDRailStep *result) noexcept
{
	auto &ctx = *static_cast<RailYapfContext *>(context);
	if ((mask & 1) && ctx.target != nullptr) ctx.target->tile = TileIndex{result->target_tile};
	if ((mask & 2) && ctx.destination != nullptr) *ctx.destination = TileIndex{result->destination};
	if ((mask & 4) && ctx.path_found != nullptr) *ctx.path_found = result->found;
	if ((mask & 8) && ctx.target != nullptr) {
		ctx.target->tile = TileIndex{result->target_tile};
		ctx.target->trackdir = static_cast<Trackdir>(result->target_td);
		ctx.target->okay = result->target_okay;
	}
}
static void RailDebug(void *context, uint8_t kind, const uint32_t *stats, uint32_t length) noexcept
{
	if (kind == 0) {
		const Train *v = static_cast<RailYapfContext *>(context)->train;
		const float ratio = stats[3] == 0 ? 0.0f : static_cast<float>(stats[3]) / static_cast<float>(stats[3] + stats[4]) * 100.0f;
		RustYapfDebug('t', stats[5] != 0, v->unitnumber, stats[0], stats[1], stats[2], ratio, static_cast<int32_t>(stats[6]), static_cast<int32_t>(stats[7]));
	} else if (kind == 3) {
		if (stats[0] == 0) Debug(desync, 2, "warning: ChooseRailTrack cache mismatch: {} vs {}", stats[1], stats[2]);
		else Debug(desync, 2, "warning: {} cache mismatch: {} vs {}", stats[0] == 1 ? "CheckReverseTrain" : stats[0] == 2 ? "FindNearestDepotTwoWay" : "FindSafeTile", stats[1] ? "T" : "F", stats[2] ? "T" : "F");
	}
	if (kind == 6) {
		/* Temporary diagnostic views preserve the original DumpTarget field
		 * labels, logical parent/segment identity and formatting. They are
		 * populated only after a mismatch; they never participate in search. */
		struct Nodes {
			std::deque<CYapfRailNode> items;
			void Dump(DumpTarget &dmp) const { dmp.WriteStructT("data", &this->items); }
		} nodes;
		nodes.items.resize((length - 2) / 17);
		std::deque<CYapfRailSegment> segments;
		for (uint32_t i = 2; i < length; i += 17) {
			auto &n = nodes.items[stats[i]];
			n.key.Set(TileIndex{stats[i + 1]}, static_cast<Trackdir>(stats[i + 2]));
			n.parent = stats[i + 3] == UINT32_MAX ? nullptr : &nodes.items[stats[i + 3]];
			n.cost = static_cast<int32_t>(stats[i + 4]);
			n.estimate = static_cast<int32_t>(stats[i + 5]);
			if (stats[i + 16] == segments.size()) segments.emplace_back(n.key);
			n.segment = &segments[stats[i + 16]];
			n.segment->last_tile = TileIndex{stats[i + 6]};
			n.segment->last_td = static_cast<Trackdir>(stats[i + 7]);
			n.segment->cost = static_cast<int32_t>(stats[i + 8]);
			n.segment->last_signal_tile = TileIndex{stats[i + 9]};
			n.segment->last_signal_td = static_cast<Trackdir>(stats[i + 10]);
			n.segment->end_segment_reason = EndSegmentReasons{static_cast<uint16_t>(stats[i + 11])};
			n.num_signals_passed = static_cast<uint16_t>(stats[i + 12]);
			n.flags_u.inherited_flags = 0;
			n.flags_u.flags_s.target_seen = (stats[i + 13] & 1) != 0;
			n.flags_u.flags_s.choice_seen = (stats[i + 13] & 2) != 0;
			n.flags_u.flags_s.last_signal_was_red = (stats[i + 13] & 4) != 0;
			n.last_red_signal_type = static_cast<SignalType>(stats[i + 14]);
			n.last_signal_type = static_cast<SignalType>(stats[i + 15]);
		}
		DumpTarget dmp;
		dmp.WriteStructT("nodes", &nodes);
		dmp.WriteValue("num_steps", stats[1]);
		auto file = FileHandle::Open(stats[0] == 1 ? "yapf1.txt"sv : "yapf2.txt"sv, "wt");
		assert(file.has_value());
		fwrite(dmp.m_out.data(), 1, dmp.m_out.size(), *file);
	}
}
static const OpenTTDRailLeaves _rail_leaves{
	.train = RailTrain, .tile = RailTile,
	.closest_station = RailClosestStation,
	.output = RailOutput, .debug = RailDebug,
};
static OpenTTDRailStep RailRun(RailYapfContext &context, uint8_t kind, TileIndex tile = INVALID_TILE, Trackdir td = INVALID_TRACKDIR, bool override_railtype = false, bool reserve = false, int max_cost = 0)
{
	const auto &s = _settings_game.pf.yapf;
	OpenTTDRailSettings settings{s.max_search_nodes, s.rail_firstred_penalty, s.rail_firstred_exit_penalty, s.rail_lastred_penalty, s.rail_lastred_exit_penalty,
		s.rail_station_penalty, s.rail_slope_penalty, s.rail_curve45_penalty, s.rail_curve90_penalty, s.rail_depot_reverse_penalty, s.rail_crossing_penalty, s.rail_look_ahead_max_signals,
		s.rail_look_ahead_signal_p0, s.rail_look_ahead_signal_p1, s.rail_look_ahead_signal_p2, s.rail_pbs_cross_penalty, s.rail_pbs_station_penalty, s.rail_pbs_signal_back_penalty, s.rail_doubleslip_penalty,
		s.rail_longer_platform_penalty, s.rail_longer_platform_per_tile_penalty, s.rail_shorter_platform_penalty, s.rail_shorter_platform_per_tile_penalty, static_cast<uint8_t>(s.rail_firstred_twoway_eol)};
	const OpenTTDRailInput input{&context, context.train, &GetRustSharedServices(), settings, Map::SizeX(), tile.base(), max_cost, _debug_desync_level, kind, static_cast<uint8_t>(td), static_cast<uint8_t>(override_railtype), static_cast<uint8_t>(_settings_game.pf.forbid_90_deg), static_cast<uint8_t>(reserve)};
	using Owner = std::unique_ptr<OpenTTDRailYapf, decltype(&openttd_rust_rail_destroy)>;
	Owner owner{openttd_rust_rail_new(&input, &_rail_leaves), openttd_rust_rail_destroy};
	for (;;) {
		const auto result = openttd_rust_rail_step(owner.get());
		if (result.action == 0) return result;
		/* These NewGRF callbacks are the only ordinary reentry boundary. Rust
		 * returned before them, releasing owner and bank access scopes. */
		const TileIndex trigger{result.tile};
		auto *st = BaseStation::GetByTile(trigger);
		TriggerStationRandomisation(st, trigger, StationRandomTrigger::PathReservation);
		TriggerStationAnimation(st, trigger, StationAnimationTrigger::PathReservation);
	}
}
Track YapfTrainChooseTrack(const Train *v, TileIndex, DiagDirection, TrackBits tracks, bool &path_found, bool reserve_track, PBSTileInfo *target, TileIndex *dest)
{
	RailYapfContext context{v, &path_found, target, dest};
	const auto result = RailRun(context, 0, INVALID_TILE, INVALID_TRACKDIR, false, reserve_track);
	return result.td != INVALID_TRACKDIR ? TrackdirToTrack(static_cast<Trackdir>(result.td)) : FindFirstTrack(tracks);
}
bool YapfTrainCheckReverse(const Train *v)
{
	RailYapfContext context{v};
	return RailRun(context, 1).value;
}
FindDepotData YapfTrainFindNearestDepot(const Train *v, int max_penalty)
{
	RailYapfContext context{v};
	const auto result = RailRun(context, 2, INVALID_TILE, INVALID_TRACKDIR, false, false, max_penalty);
	return {TileIndex{result.tile}, result.best_length, result.reverse != 0};
}
bool YapfTrainFindNearestSafeTile(const Train *v, TileIndex tile, Trackdir td, bool override_railtype)
{
	RailYapfContext context{v};
	return RailRun(context, 3, tile, td, override_railtype).value;
}
void YapfNotifyTrackLayoutChange(TileIndex, Track)
{
	openttd_rust_rail_invalidate();
}

#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRailLeaves.hpp"
bool CheckAbi_yapf_rail() { return AbiServicesComplete(_rail_leaves); }

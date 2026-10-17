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

#ifdef WITH_RUST
#include "../../rust/rail_yapf_ffi.h"
#include "../../rust/services_rail.h"
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
	return {v->GetCompatibleRailTypes().base(), GetAllCompatibleRailTypes(v->GetRailTypes()).base(),
		v->tile.base(), rear->tile.base(), TileVirtXY(v->x_pos, v->y_pos).base(), TileVirtXY(rear->x_pos, rear->y_pos).base(),
		v->dest_tile.base(), v->gcache.cached_total_length, static_cast<uint32_t>(std::min<int>(v->GetDisplayMaxSpeed(), v->current_order.GetMaxSpeed())),
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
		| (IsBridgeTile(t) ? 256U : 0U) | (IsTunnelTile(t) ? 512U : 0U);
	result.railtype = GetTileRailType(t);
	result.reserved = RustReservedTracks(tile);
	if (IsPlainRailTile(t)) result.tracks = RustRailTracks(tile);
	if (HasStationTileRail(t)) {
		result.station = GetStationIndex(t).base();
		result.station_track = RustRailStationTrack(tile);
		if (RustIsRailStation(tile) && HasStationReservation(t)) result.flags |= 1024U;
	}
	if (IsTileType(t, MP_TUNNELBRIDGE)) {
		result.tunnel_dir = GetTunnelBridgeDirection(t);
		result.other_end = RustOtherTunnelBridgeEnd(tile);
	}
	result.uphill = IsUphillTrackdir(GetTileSlope(t), td);
	if (IsBridgeTile(t)) result.flat_ramp = HasBridgeFlatRamp(GetTileSlope(t), DiagDirToAxis(GetTunnelBridgeDirection(t)));
	if (IsTileType(t, MP_RAILWAY)) {
		result.signal_along = RustRailHasSignalTrackdir(tile, td);
		result.signal_against = RustRailHasSignalTrackdir(tile, ReverseTrackdir(td));
		if (result.signal_along || result.signal_against) {
			result.signal_type = RustRailSignalType(tile, TrackdirToTrack(td));
			result.oneway = IsOnewaySignal(t, TrackdirToTrack(td));
		}
		if (result.signal_along) result.signal_green = RustRailSignalState(tile, td) == SIGNAL_STATE_GREEN;
	}
	if (HasOnewaySignalBlockingTrackdir(t, td)) result.flags |= 2048U;
	return result;
}
template <typename Follower>
static OpenTTDRailFollow RailFollowT(void *context, uint32_t tile, uint8_t direction, uint64_t types, bool mask) noexcept
{
	Follower follower{static_cast<RailYapfContext *>(context)->train, RailTypes{types}};
	bool followed = follower.Follow(TileIndex{tile}, static_cast<Trackdir>(direction));
	if (followed && mask) followed = follower.MaskReservedTracks();
	int min_speed = 0;
	int max_speed = follower.GetSpeedLimit(&min_speed);
	return {follower.new_tile.base(), follower.tiles_skipped, min_speed, max_speed,
		static_cast<uint16_t>(follower.new_td_bits), static_cast<uint8_t>(followed), static_cast<uint8_t>(follower.err), static_cast<uint8_t>(follower.is_station)};
}
static OpenTTDRailFollow RailFollow(void *context, uint32_t tile, uint8_t td, uint64_t types, uint8_t no90, uint8_t mask) noexcept
{
	if (no90) return RailFollowT<CFollowTrackFreeRailNo90>(context, tile, td, types, mask);
	return RailFollowT<CFollowTrackFreeRail>(context, tile, td, types, mask);
}
static uint8_t RailFree(void *context, uint32_t tile, uint8_t td, uint8_t no90) noexcept
{
	return IsWaitingPositionFree(static_cast<RailYapfContext *>(context)->train, TileIndex{tile}, static_cast<Trackdir>(td), no90);
}
static uint8_t RailCompatibleStation(uint32_t tile, uint32_t start) noexcept
{
	return IsCompatibleTrainStationTile(TileIndex{tile}, TileIndex{start});
}
static uint32_t RailPlatformLength(uint32_t tile, uint8_t dir) noexcept
{
	return BaseStation::GetByTile(TileIndex{tile})->GetPlatformLength(TileIndex{tile}, static_cast<DiagDirection>(dir));
}
static uint32_t RailClosestStation(void *context, uint8_t station) noexcept
{
	const Train *v = static_cast<RailYapfContext *>(context)->train;
	return CalcClosestStationTile(v->current_order.GetDestination().ToStationID(), v->tile, station ? StationType::Rail : StationType::RailWaypoint).base();
}
static uint16_t RailDestinationDirs(uint32_t tile) noexcept
{
	return TrackStatusToTrackdirBits(RustRailTrackStatus(tile, INVALID_DIAGDIR));
}
static void RailOrigin(void *context, uint32_t *tile, uint8_t *td) noexcept
{
	const auto origin = FollowTrainReservation(static_cast<RailYapfContext *>(context)->train);
	*tile = origin.tile.base();
	*td = origin.trackdir;
}
static uint8_t RailWrite(uint32_t tile, uint8_t direction, uint8_t operation) noexcept
{
	const TileIndex t{tile};
	const Trackdir td = static_cast<Trackdir>(direction);
	switch (operation) {
		case 0: MarkTileDirtyByTile(t); break;
		case 1: return TryReserveRailTrack(t, TrackdirToTrack(td));
		case 2: UnreserveRailTrack(t, TrackdirToTrack(td)); break;
		case 3: SetRailStationReservation(t, true); MarkTileDirtyByTile(t); break;
		case 4: SetRailStationReservation(t, false); break;
		case 5: RustRailSetSignalState(tile, td, SIGNAL_STATE_RED); MarkTileDirtyByTile(t); break;
		case 6: RustRailSetSignalState(tile, td, SIGNAL_STATE_GREEN); break;
		default: NOT_REACHED();
	}
	return true;
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
		Debug(yapf, 3, "[YAPFt]{}{:4d} - {} rounds - {} open - {} closed - CHR {:4.1f}% - C {} D {}",
			stats[5] ? '-' : '!', v->unitnumber, stats[0], stats[1], stats[2], ratio, static_cast<int32_t>(stats[6]), static_cast<int32_t>(stats[7]));
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
	.train = RailTrain, .tile = RailTile, .follow = RailFollow, .free = RailFree,
	.compatible_station = RailCompatibleStation, .platform_length = RailPlatformLength,
	.closest_station = RailClosestStation, .destination_dirs = RailDestinationDirs,
	.origin = RailOrigin, .write = RailWrite, .output = RailOutput, .debug = RailDebug,
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
#else

template <typename Tpf> void DumpState(Tpf &pf1, Tpf &pf2)
{
	DumpTarget dmp1, dmp2;
	pf1.DumpBase(dmp1);
	pf2.DumpBase(dmp2);
	auto f1 = FileHandle::Open("yapf1.txt"sv, "wt");
	auto f2 = FileHandle::Open("yapf2.txt"sv, "wt");
	assert(f1.has_value());
	assert(f2.has_value());
	fwrite(dmp1.m_out.data(), 1, dmp1.m_out.size(), *f1);
	fwrite(dmp2.m_out.data(), 1, dmp2.m_out.size(), *f2);
}

template <class Types>
class CYapfReserveTrack {
public:
	typedef typename Types::Tpf Tpf; ///< the pathfinder class (derived from THIS class)
	typedef typename Types::TrackFollower TrackFollower;
	typedef typename Types::NodeList::Item Node; ///< this will be our node type

protected:
	/** to access inherited pathfinder */
	inline Tpf &Yapf()
	{
		return *static_cast<Tpf *>(this);
	}

private:
	TileIndex res_dest_tile; ///< The reservation target tile
	Trackdir res_dest_td; ///< The reservation target trackdir
	Node *res_dest_node; ///< The reservation target node
	TileIndex res_fail_tile; ///< The tile where the reservation failed
	Trackdir res_fail_td; ///< The trackdir where the reservation failed
	TileIndex origin_tile; ///< Tile our reservation will originate from

	std::vector<std::pair<TileIndex, Trackdir>> signals_set_to_red; ///< List of signals turned red during a path reservation.

	bool FindSafePositionProc(TileIndex tile, Trackdir td)
	{
		if (IsSafeWaitingPosition(Yapf().GetVehicle(), tile, td, true, !TrackFollower::Allow90degTurns())) {
			this->res_dest_tile = tile;
			this->res_dest_td = td;
			return false;   // Stop iterating segment
		}
		return true;
	}

	/** Reserve a railway platform. Tile contains the failed tile on abort. */
	bool ReserveRailStationPlatform(TileIndex &tile, DiagDirection dir)
	{
		TileIndex     start = tile;
		TileIndexDiff diff = TileOffsByDiagDir(dir);

		do {
			if (HasStationReservation(tile)) return false;
			SetRailStationReservation(tile, true);
			MarkTileDirtyByTile(tile);
			tile = TileAdd(tile, diff);
		} while (IsCompatibleTrainStationTile(tile, start) && tile != this->origin_tile);

		auto *st = Station::GetByTile(start);
		TriggerStationRandomisation(st, start, StationRandomTrigger::PathReservation);
		TriggerStationAnimation(st, start, StationAnimationTrigger::PathReservation);

		return true;
	}

	/** Try to reserve a single track/platform. */
	bool ReserveSingleTrack(TileIndex tile, Trackdir td)
	{
		Trackdir rev_td = ReverseTrackdir(td);
		if (IsRailStationTile(tile)) {
			if (!ReserveRailStationPlatform(tile, TrackdirToExitdir(rev_td))) {
				/* Platform could not be reserved, undo. */
				this->res_fail_tile = tile;
				this->res_fail_td = td;
			}
		} else {
			if (!TryReserveRailTrack(tile, TrackdirToTrack(td))) {
				/* Tile couldn't be reserved, undo. */
				this->res_fail_tile = tile;
				this->res_fail_td = td;
				return false;
			}

			/* Green path signal opposing the path? Turn to red. */
			if (HasPbsSignalOnTrackdir(tile, rev_td) && GetSignalStateByTrackdir(tile, rev_td) == SIGNAL_STATE_GREEN) {
				this->signals_set_to_red.emplace_back(tile, rev_td);
				SetSignalStateByTrackdir(tile, rev_td, SIGNAL_STATE_RED);
				MarkTileDirtyByTile(tile);
			}

			if (IsRailWaypointTile(tile)) {
				auto *st = BaseStation::GetByTile(tile);
				TriggerStationRandomisation(st, tile, StationRandomTrigger::PathReservation);
				TriggerStationAnimation(st, tile, StationAnimationTrigger::PathReservation);
			}
		}

		return tile != this->res_dest_tile || td != this->res_dest_td;
	}

	/** Unreserve a single track/platform. Stops when the previous failure is reached. */
	bool UnreserveSingleTrack(TileIndex tile, Trackdir td)
	{
		if (IsRailStationTile(tile)) {
			TileIndex     start = tile;
			TileIndexDiff diff = TileOffsByDiagDir(TrackdirToExitdir(ReverseTrackdir(td)));
			while ((tile != this->res_fail_tile || td != this->res_fail_td) && IsCompatibleTrainStationTile(tile, start)) {
				SetRailStationReservation(tile, false);
				tile = TileAdd(tile, diff);
			}
		} else if (tile != this->res_fail_tile || td != this->res_fail_td) {
			UnreserveRailTrack(tile, TrackdirToTrack(td));
		}
		return (tile != this->res_dest_tile || td != this->res_dest_td) && (tile != this->res_fail_tile || td != this->res_fail_td);
	}

public:
	/** Set the target to where the reservation should be extended. */
	inline void SetReservationTarget(Node *node, TileIndex tile, Trackdir td)
	{
		this->res_dest_node = node;
		this->res_dest_tile = tile;
		this->res_dest_td = td;
	}

	/** Check the node for a possible reservation target. */
	inline void FindSafePositionOnNode(Node *node)
	{
		assert(node->parent != nullptr);

		/* We will never pass more than two signals, no need to check for a safe tile. */
		if (node->parent->num_signals_passed >= 2) return;

		if (!node->IterateTiles(Yapf().GetVehicle(), Yapf(), *this, &CYapfReserveTrack<Types>::FindSafePositionProc)) {
			this->res_dest_node = node;
		}
	}

	/** Try to reserve the path till the reservation target. */
	bool TryReservePath(PBSTileInfo *target, TileIndex origin)
	{
		this->res_fail_tile = INVALID_TILE;
		this->origin_tile = origin;

		if (target != nullptr) {
			target->tile = this->res_dest_tile;
			target->trackdir = this->res_dest_td;
			target->okay = false;
		}

		/* Don't bother if the target is reserved. */
		if (!IsWaitingPositionFree(Yapf().GetVehicle(), this->res_dest_tile, this->res_dest_td)) return false;

		this->signals_set_to_red.clear();
		for (Node *node = this->res_dest_node; node->parent != nullptr; node = node->parent) {
			node->IterateTiles(Yapf().GetVehicle(), Yapf(), *this, &CYapfReserveTrack<Types>::ReserveSingleTrack);
			if (this->res_fail_tile != INVALID_TILE) {
				/* Reservation failed, undo. */
				Node *fail_node = this->res_dest_node;
				TileIndex stop_tile = this->res_fail_tile;
				do {
					/* If this is the node that failed, stop at the failed tile. */
					this->res_fail_tile = fail_node == node ? stop_tile : INVALID_TILE;
					fail_node->IterateTiles(Yapf().GetVehicle(), Yapf(), *this, &CYapfReserveTrack<Types>::UnreserveSingleTrack);
				} while (fail_node != node && (fail_node = fail_node->parent) != nullptr);

				/* Re-instate green path signals we turned to red. */
				for (auto [sig_tile, td] : this->signals_set_to_red) {
					SetSignalStateByTrackdir(sig_tile, td, SIGNAL_STATE_GREEN);
				}

				return false;
			}
		}

		if (target != nullptr) target->okay = true;

		if (Yapf().CanUseGlobalCache(*this->res_dest_node)) {
			YapfNotifyTrackLayoutChange(INVALID_TILE, INVALID_TRACK);
		}

		return true;
	}
};

template <class Types>
class CYapfFollowAnyDepotRailT {
public:
	typedef typename Types::Tpf Tpf; ///< the pathfinder class (derived from THIS class)
	typedef typename Types::TrackFollower TrackFollower;
	typedef typename Types::NodeList::Item Node; ///< this will be our node type
	typedef typename Node::Key Key; ///< key to hash tables

protected:
	/** to access inherited path finder */
	inline Tpf &Yapf()
	{
		return *static_cast<Tpf *>(this);
	}

public:
	/**
	 * Called by YAPF to move from the given node to the next tile. For each
	 *  reachable trackdir on the new tile creates new node, initializes it
	 *  and adds it to the open list by calling Yapf().AddNewNode(n)
	 */
	inline void PfFollowNode(Node &old_node)
	{
		TrackFollower follower{Yapf().GetVehicle()};
		if (follower.Follow(old_node.GetLastTile(), old_node.GetLastTrackdir())) {
			Yapf().AddMultipleNodes(&old_node, follower);
		}
	}

	/** return debug report character to identify the transportation type */
	inline char TransportTypeChar() const
	{
		return 't';
	}

	static FindDepotData stFindNearestDepotTwoWay(const Train *v, TileIndex t1, Trackdir td1, TileIndex t2, Trackdir td2, int max_penalty, int reverse_penalty)
	{
		Tpf pf1;
		/*
		 * With caching enabled it simply cannot get a reliable result when you
		 * have limited the distance a train may travel. This means that the
		 * cached result does not match uncached result in all cases and that
		 * causes desyncs. So disable caching when finding for a depot that is
		 * nearby. This only happens with automatic servicing of vehicles,
		 * so it will only impact performance when you do not manually set
		 * depot orders and you do not disable automatic servicing.
		 */
		if (max_penalty != 0) pf1.DisableCache(true);
		FindDepotData result1 = pf1.FindNearestDepotTwoWay(v, t1, td1, t2, td2, max_penalty, reverse_penalty);

		if (_debug_desync_level >= 2) {
			Tpf pf2;
			pf2.DisableCache(true);
			FindDepotData result2 = pf2.FindNearestDepotTwoWay(v, t1, td1, t2, td2, max_penalty, reverse_penalty);
			if (result1.tile != result2.tile || (result1.reverse != result2.reverse)) {
				Debug(desync, 2, "warning: FindNearestDepotTwoWay cache mismatch: {} vs {}",
						result1.tile != INVALID_TILE ? "T" : "F",
						result2.tile != INVALID_TILE ? "T" : "F");
				DumpState(pf1, pf2);
			}
		}

		return result1;
	}

	inline FindDepotData FindNearestDepotTwoWay(const Train *v, TileIndex t1, Trackdir td1, TileIndex t2, Trackdir td2, int max_penalty, int reverse_penalty)
	{
		/* set origin and destination nodes */
		Yapf().SetOrigin(t1, td1, t2, td2, reverse_penalty);
		Yapf().SetTreatFirstRedTwoWaySignalAsEOL(true);
		Yapf().SetDestination(v);
		Yapf().SetMaxCost(max_penalty);

		/* find the best path */
		if (!Yapf().FindPath(v)) return FindDepotData();

		/* Some path found. */
		Node *n = Yapf().GetBestNode();

		/* walk through the path back to the origin */
		Node *node = n;
		while (node->parent != nullptr) {
			node = node->parent;
		}

		/* if the origin node is our front vehicle tile/Trackdir then we didn't reverse
		 * but we can also look at the cost (== 0 -> not reversed, == reverse_penalty -> reversed) */
		return FindDepotData(n->GetLastTile(), n->cost, node->cost != 0);
	}
};

template <class Types>
class CYapfFollowAnySafeTileRailT : public CYapfReserveTrack<Types> {
public:
	typedef typename Types::Tpf Tpf; ///< the pathfinder class (derived from THIS class)
	typedef typename Types::TrackFollower TrackFollower;
	typedef typename Types::NodeList::Item Node; ///< this will be our node type
	typedef typename Node::Key Key; ///< key to hash tables

protected:
	/** to access inherited path finder */
	inline Tpf &Yapf()
	{
		return *static_cast<Tpf *>(this);
	}

public:
	/**
	 * Called by YAPF to move from the given node to the next tile. For each
	 *  reachable trackdir on the new tile creates new node, initializes it
	 *  and adds it to the open list by calling Yapf().AddNewNode(n)
	 */
	inline void PfFollowNode(Node &old_node)
	{
		TrackFollower follower{Yapf().GetVehicle(), Yapf().GetCompatibleRailTypes()};
		if (follower.Follow(old_node.GetLastTile(), old_node.GetLastTrackdir()) && follower.MaskReservedTracks()) {
			Yapf().AddMultipleNodes(&old_node, follower);
		}
	}

	/** Return debug report character to identify the transportation type */
	inline char TransportTypeChar() const
	{
		return 't';
	}

	static bool stFindNearestSafeTile(const Train *v, TileIndex t1, Trackdir td, bool override_railtype)
	{
		/* Create pathfinder instance */
		Tpf pf1;
		bool result1;
		if (_debug_desync_level < 2) {
			result1 = pf1.FindNearestSafeTile(v, t1, td, override_railtype, false);
		} else {
			bool result2 = pf1.FindNearestSafeTile(v, t1, td, override_railtype, true);
			Tpf pf2;
			pf2.DisableCache(true);
			result1 = pf2.FindNearestSafeTile(v, t1, td, override_railtype, false);
			if (result1 != result2) {
				Debug(desync, 2, "warning: FindSafeTile cache mismatch: {} vs {}", result2 ? "T" : "F", result1 ? "T" : "F");
				DumpState(pf1, pf2);
			}
		}

		return result1;
	}

	bool FindNearestSafeTile(const Train *v, TileIndex t1, Trackdir td, bool override_railtype, bool dont_reserve)
	{
		/* Set origin and destination. */
		Yapf().SetOrigin(t1, td);
		Yapf().SetTreatFirstRedTwoWaySignalAsEOL(true);
		Yapf().SetDestination(v, override_railtype);

		if (!Yapf().FindPath(v)) return false;

		/* Found a destination, set as reservation target. */
		Node *node = Yapf().GetBestNode();
		this->SetReservationTarget(node, node->GetLastTile(), node->GetLastTrackdir());

		/* Walk through the path back to the origin. */
		Node *prev = nullptr;
		while (node->parent != nullptr) {
			prev = node;
			node = node->parent;

			this->FindSafePositionOnNode(prev);
		}

		return dont_reserve || this->TryReservePath(nullptr, node->GetLastTile());
	}
};

template <class Types>
class CYapfFollowRailT : public CYapfReserveTrack<Types> {
public:
	typedef typename Types::Tpf Tpf; ///< the pathfinder class (derived from THIS class)
	typedef typename Types::TrackFollower TrackFollower;
	typedef typename Types::NodeList::Item Node; ///< this will be our node type
	typedef typename Node::Key Key; ///< key to hash tables

protected:
	/** to access inherited path finder */
	inline Tpf &Yapf()
	{
		return *static_cast<Tpf *>(this);
	}

public:
	/**
	 * Called by YAPF to move from the given node to the next tile. For each
	 *  reachable trackdir on the new tile creates new node, initializes it
	 *  and adds it to the open list by calling Yapf().AddNewNode(n)
	 */
	inline void PfFollowNode(Node &old_node)
	{
		TrackFollower follower{Yapf().GetVehicle()};
		if (follower.Follow(old_node.GetLastTile(), old_node.GetLastTrackdir())) {
			Yapf().AddMultipleNodes(&old_node, follower);
		}
	}

	/** return debug report character to identify the transportation type */
	inline char TransportTypeChar() const
	{
		return 't';
	}

	static Trackdir stChooseRailTrack(const Train *v, TileIndex tile, DiagDirection enterdir, TrackBits tracks, bool &path_found, bool reserve_track, PBSTileInfo *target, TileIndex *dest)
	{
		/* create pathfinder instance */
		Tpf pf1;
		Trackdir result1;

		if (_debug_desync_level < 2) {
			result1 = pf1.ChooseRailTrack(v, tile, enterdir, tracks, path_found, reserve_track, target, dest);
		} else {
			result1 = pf1.ChooseRailTrack(v, tile, enterdir, tracks, path_found, false, nullptr, nullptr);
			Tpf pf2;
			pf2.DisableCache(true);
			Trackdir result2 = pf2.ChooseRailTrack(v, tile, enterdir, tracks, path_found, reserve_track, target, dest);
			if (result1 != result2) {
				Debug(desync, 2, "warning: ChooseRailTrack cache mismatch: {} vs {}", result1, result2);
				DumpState(pf1, pf2);
			}
		}

		return result1;
	}

	inline Trackdir ChooseRailTrack(const Train *v, TileIndex, DiagDirection, TrackBits, bool &path_found, bool reserve_track, PBSTileInfo *target, TileIndex *dest)
	{
		if (target != nullptr) target->tile = INVALID_TILE;
		if (dest != nullptr) *dest = INVALID_TILE;

		/* set origin and destination nodes */
		PBSTileInfo origin = FollowTrainReservation(v);
		Yapf().SetOrigin(origin.tile, origin.trackdir, INVALID_TILE, INVALID_TRACKDIR, 1);
		Yapf().SetTreatFirstRedTwoWaySignalAsEOL(true);
		Yapf().SetDestination(v);

		/* find the best path */
		path_found = Yapf().FindPath(v);

		/* if path not found - return INVALID_TRACKDIR */
		Trackdir next_trackdir = INVALID_TRACKDIR;
		Node *node = Yapf().GetBestNode();
		if (node != nullptr) {
			/* reserve till end of path */
			this->SetReservationTarget(node, node->GetLastTile(), node->GetLastTrackdir());

			/* path was found or at least suggested
			 * walk through the path back to the origin */
			Node *prev = nullptr;
			while (node->parent != nullptr) {
				prev = node;
				node = node->parent;

				this->FindSafePositionOnNode(prev);
			}

			/* If the best PF node has no parent, then there is no (valid) best next trackdir to return.
			 * This occurs when the PF is called while the train is already at its destination. */
			if (prev == nullptr) return INVALID_TRACKDIR;

			/* return trackdir from the best origin node (one of start nodes) */
			Node &best_next_node = *prev;
			next_trackdir = best_next_node.GetTrackdir();

			if (reserve_track && path_found) {
				if (dest != nullptr) *dest = Yapf().GetBestNode()->GetLastTile();
				this->TryReservePath(target, node->GetLastTile());
			}
		}

		/* Treat the path as found if stopped on the first two way signal(s). */
		path_found |= Yapf().stopped_on_first_two_way_signal;
		return next_trackdir;
	}

	static bool stCheckReverseTrain(const Train *v, TileIndex t1, Trackdir td1, TileIndex t2, Trackdir td2, int reverse_penalty)
	{
		Tpf pf1;
		bool result1 = pf1.CheckReverseTrain(v, t1, td1, t2, td2, reverse_penalty);

		if (_debug_desync_level >= 2) {
			Tpf pf2;
			pf2.DisableCache(true);
			bool result2 = pf2.CheckReverseTrain(v, t1, td1, t2, td2, reverse_penalty);
			if (result1 != result2) {
				Debug(desync, 2, "warning: CheckReverseTrain cache mismatch: {} vs {}", result1 ? "T" : "F", result2 ? "T" : "F");
				DumpState(pf1, pf2);
			}
		}

		return result1;
	}

	inline bool CheckReverseTrain(const Train *v, TileIndex t1, Trackdir td1, TileIndex t2, Trackdir td2, int reverse_penalty)
	{
		/* create pathfinder instance
		 * set origin and destination nodes */
		Yapf().SetOrigin(t1, td1, t2, td2, reverse_penalty);
		Yapf().SetTreatFirstRedTwoWaySignalAsEOL(false);
		Yapf().SetDestination(v);

		/* find the best path */
		if (!Yapf().FindPath(v)) return false;

		/* path was found
		 * walk through the path back to the origin */
		Node *node = Yapf().GetBestNode();
		while (node->parent != nullptr) {
			node = node->parent;
		}

		/* check if it was reversed origin */
		bool reversed = (node->cost != 0);
		return reversed;
	}
};

template <class Tpf_, class Ttrack_follower, template <class Types> class TdestinationT, template <class Types> class TfollowT>
struct CYapfRail_TypesT {
	typedef CYapfRail_TypesT<Tpf_, Ttrack_follower, TdestinationT, TfollowT>  Types;

	typedef Tpf_                                Tpf;
	typedef Ttrack_follower                     TrackFollower;
	typedef CRailNodeList                       NodeList;
	typedef Train                               VehicleType;
	typedef CYapfBaseT<Types>                   PfBase;
	typedef TfollowT<Types>                     PfFollow;
	typedef CYapfOriginTileTwoWayT<Types>       PfOrigin;
	typedef TdestinationT<Types>                PfDestination;
	typedef CYapfSegmentCostCacheGlobalT<Types> PfCache;
	typedef CYapfCostRailT<Types>               PfCost;
};

template <typename Types>
struct CYapfRailBase : CYapfT<Types> {
	typedef typename Types::NodeList::Item Node;

	/**
	 * In some cases an intermediate node branch should be pruned.
	 * The most prominent case is when a red EOL signal is encountered, but
	 * there was a segment change (e.g. a rail type change) before that. If
	 * the branch would not be pruned, the rail type change location would
	 * remain the best intermediate node, and thus the vehicle would still
	 * go towards the red EOL signal.
	 */
	void PruneIntermediateNodeBranch(Node *n)
	{
		bool intermediate_on_branch = false;
		while (n != nullptr && !n->segment->end_segment_reason.Test(EndSegmentReason::ChoiceFollows)) {
			if (n == this->best_intermediate_node) intermediate_on_branch = true;
			n = n->parent;
		}
		if (intermediate_on_branch) this->best_intermediate_node = n;
	}
};

struct CYapfRail         : CYapfRailBase<CYapfRail_TypesT<CYapfRail        , CFollowTrackRail    , CYapfDestinationTileOrStationRailT, CYapfFollowRailT>> {};
struct CYapfRailNo90     : CYapfRailBase<CYapfRail_TypesT<CYapfRailNo90    , CFollowTrackRailNo90, CYapfDestinationTileOrStationRailT, CYapfFollowRailT>> {};

struct CYapfAnyDepotRail     : CYapfRailBase<CYapfRail_TypesT<CYapfAnyDepotRail,     CFollowTrackRail    , CYapfDestinationAnyDepotRailT     , CYapfFollowAnyDepotRailT>> {};
struct CYapfAnyDepotRailNo90 : CYapfRailBase<CYapfRail_TypesT<CYapfAnyDepotRailNo90, CFollowTrackRailNo90, CYapfDestinationAnyDepotRailT     , CYapfFollowAnyDepotRailT>> {};

struct CYapfAnySafeTileRail     : CYapfRailBase<CYapfRail_TypesT<CYapfAnySafeTileRail    , CFollowTrackFreeRail    , CYapfDestinationAnySafeTileRailT , CYapfFollowAnySafeTileRailT>> {};
struct CYapfAnySafeTileRailNo90 : CYapfRailBase<CYapfRail_TypesT<CYapfAnySafeTileRailNo90, CFollowTrackFreeRailNo90, CYapfDestinationAnySafeTileRailT , CYapfFollowAnySafeTileRailT>> {};


Track YapfTrainChooseTrack(const Train *v, TileIndex tile, DiagDirection enterdir, TrackBits tracks, bool &path_found, bool reserve_track, PBSTileInfo *target, TileIndex *dest)
{
	Trackdir td_ret = _settings_game.pf.forbid_90_deg
		? CYapfRailNo90::stChooseRailTrack(v, tile, enterdir, tracks, path_found, reserve_track, target, dest)
		: CYapfRail::stChooseRailTrack(v, tile, enterdir, tracks, path_found, reserve_track, target, dest);

	return (td_ret != INVALID_TRACKDIR) ? TrackdirToTrack(td_ret) : FindFirstTrack(tracks);
}

bool YapfTrainCheckReverse(const Train *v)
{
	const Train *last_veh = v->Last();

	/* get trackdirs of both ends */
	Trackdir td = v->GetVehicleTrackdir();
	Trackdir td_rev = ReverseTrackdir(last_veh->GetVehicleTrackdir());

	/* tiles where front and back are */
	TileIndex tile = v->tile;
	TileIndex tile_rev = last_veh->tile;

	int reverse_penalty = 0;

	if (v->GetTrack() == TRACK_BIT_WORMHOLE) {
		/* front in tunnel / on bridge */
		DiagDirection dir_into_wormhole = GetTunnelBridgeDirection(tile);

		if (TrackdirToExitdir(td) == dir_into_wormhole) tile = GetOtherTunnelBridgeEnd(tile);
		/* Now 'tile' is the tunnel entry/bridge ramp the train will reach when driving forward */

		/* Current position of the train in the wormhole */
		TileIndex cur_tile = TileVirtXY(v->x_pos, v->y_pos);

		/* Add distance to drive in the wormhole as penalty for the forward path, i.e. bonus for the reverse path
		 * Note: Negative penalties are ok for the start tile. */
		reverse_penalty -= DistanceManhattan(cur_tile, tile) * YAPF_TILE_LENGTH;
	}

	if (last_veh->GetTrack() == TRACK_BIT_WORMHOLE) {
		/* back in tunnel / on bridge */
		DiagDirection dir_into_wormhole = GetTunnelBridgeDirection(tile_rev);

		if (TrackdirToExitdir(td_rev) == dir_into_wormhole) tile_rev = GetOtherTunnelBridgeEnd(tile_rev);
		/* Now 'tile_rev' is the tunnel entry/bridge ramp the train will reach when reversing */

		/* Current position of the last wagon in the wormhole */
		TileIndex cur_tile = TileVirtXY(last_veh->x_pos, last_veh->y_pos);

		/* Add distance to drive in the wormhole as penalty for the revere path. */
		reverse_penalty += DistanceManhattan(cur_tile, tile_rev) * YAPF_TILE_LENGTH;
	}

	/* slightly hackish: If the pathfinders finds a path, the cost of the first node is tested to distinguish between forward- and reverse-path. */
	if (reverse_penalty == 0) reverse_penalty = 1;

	bool reverse = _settings_game.pf.forbid_90_deg
		? CYapfRailNo90::stCheckReverseTrain(v, tile, td, tile_rev, td_rev, reverse_penalty)
		: CYapfRail::stCheckReverseTrain(v, tile, td, tile_rev, td_rev, reverse_penalty);

	return reverse;
}

FindDepotData YapfTrainFindNearestDepot(const Train *v, int max_penalty)
{
	const Train *last_veh = v->Last();

	PBSTileInfo origin = FollowTrainReservation(v);
	TileIndex last_tile = last_veh->tile;
	Trackdir td_rev = ReverseTrackdir(last_veh->GetVehicleTrackdir());

	return _settings_game.pf.forbid_90_deg
		? CYapfAnyDepotRailNo90::stFindNearestDepotTwoWay(v, origin.tile, origin.trackdir, last_tile, td_rev, max_penalty, YAPF_INFINITE_PENALTY)
		: CYapfAnyDepotRail::stFindNearestDepotTwoWay(v, origin.tile, origin.trackdir, last_tile, td_rev, max_penalty, YAPF_INFINITE_PENALTY);
}

bool YapfTrainFindNearestSafeTile(const Train *v, TileIndex tile, Trackdir td, bool override_railtype)
{
	return _settings_game.pf.forbid_90_deg
		? CYapfAnySafeTileRailNo90::stFindNearestSafeTile(v, tile, td, override_railtype)
		: CYapfAnySafeTileRail::stFindNearestSafeTile(v, tile, td, override_railtype);
}

/** if any track changes, this counter is incremented - that will invalidate segment cost cache */
int CSegmentCostCacheBase::s_rail_change_counter = 0;

void YapfNotifyTrackLayoutChange(TileIndex tile, Track track)
{
	CSegmentCostCacheBase::NotifyTrackLayoutChange(tile, track);
}

#endif // WITH_RUST

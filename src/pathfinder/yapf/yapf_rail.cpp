/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file yapf_rail.cpp The rail pathfinding. */

#include "../../stdafx.h"

#include "yapf.hpp"
#include "yapf_node_rail.hpp"

#include "../../safeguards.h"

#include "../../rust/rail_yapf_ffi.h"
#include "../../rust/vehicle_services.h"

void YapfRailDumpState(const uint32_t *words, uint32_t length) noexcept
{
	/* The words rebuild the original's node and segment views for DumpTarget:
	 * the file number, num_steps, then 17 words per node. */
	struct Nodes {
		std::deque<CYapfRailNode> items;
		void Dump(DumpTarget &dmp) const { dmp.WriteStructT("data", &this->items); }
	} nodes;
	nodes.items.resize((length - 2) / 17);
	std::deque<CYapfRailSegment> segments;
	for (uint32_t i = 2; i < length; i += 17) {
		auto &n = nodes.items[words[i]];
		n.key.Set(TileIndex{words[i + 1]}, static_cast<Trackdir>(words[i + 2]));
		n.parent = words[i + 3] == UINT32_MAX ? nullptr : &nodes.items[words[i + 3]];
		n.cost = static_cast<int32_t>(words[i + 4]);
		n.estimate = static_cast<int32_t>(words[i + 5]);
		if (words[i + 16] == segments.size()) segments.emplace_back(n.key);
		n.segment = &segments[words[i + 16]];
		n.segment->last_tile = TileIndex{words[i + 6]};
		n.segment->last_td = static_cast<Trackdir>(words[i + 7]);
		n.segment->cost = static_cast<int32_t>(words[i + 8]);
		n.segment->last_signal_tile = TileIndex{words[i + 9]};
		n.segment->last_signal_td = static_cast<Trackdir>(words[i + 10]);
		n.segment->end_segment_reason = EndSegmentReasons{static_cast<uint16_t>(words[i + 11])};
		n.num_signals_passed = static_cast<uint16_t>(words[i + 12]);
		n.flags_u.inherited_flags = 0;
		n.flags_u.flags_s.target_seen = (words[i + 13] & 1) != 0;
		n.flags_u.flags_s.choice_seen = (words[i + 13] & 2) != 0;
		n.flags_u.flags_s.last_signal_was_red = (words[i + 13] & 4) != 0;
		n.last_red_signal_type = static_cast<SignalType>(words[i + 14]);
		n.last_signal_type = static_cast<SignalType>(words[i + 15]);
	}
	DumpTarget dmp;
	dmp.WriteStructT("nodes", &nodes);
	dmp.WriteValue("num_steps", words[1]);
	auto file = FileHandle::Open(words[0] == 1 ? "yapf1.txt"sv : "yapf2.txt"sv, "wt");
	assert(file.has_value());
	fwrite(dmp.m_out.data(), 1, dmp.m_out.size(), *file);
}

Track YapfTrainChooseTrack(const Train *v, TileIndex tile, DiagDirection enterdir, TrackBits tracks, bool &path_found, bool reserve_track, PBSTileInfo *target, TileIndex *dest)
{
	OpenTTDTrainReservationPbs record{};
	if (target != nullptr) record = {target->tile.base(), {}, static_cast<uint8_t>(target->trackdir), static_cast<uint8_t>(target->okay)};
	uint32_t dest_tile = dest != nullptr ? dest->base() : 0;
	const Track track = static_cast<Track>(openttd_rust_yapf_train_choose_track(&GetRustSharedServices(), RustVehicle(v), tile.base(), enterdir, tracks, &path_found, reserve_track, target != nullptr ? &record : nullptr, dest != nullptr ? &dest_tile : nullptr));
	if (target != nullptr) *target = {TileIndex{record.tile}, static_cast<Trackdir>(record.td), record.okay != 0};
	if (dest != nullptr) *dest = TileIndex{dest_tile};
	return track;
}

bool YapfTrainCheckReverse(const Train *v)
{
	return openttd_rust_yapf_train_check_reverse(&GetRustSharedServices(), RustVehicle(v)) != 0;
}

FindDepotData YapfTrainFindNearestDepot(const Train *v, int max_penalty)
{
	const auto result = openttd_rust_yapf_train_find_nearest_depot(&GetRustSharedServices(), RustVehicle(v), max_penalty);
	return {TileIndex{result.tile}, result.best_length, result.reverse != 0};
}

bool YapfTrainFindNearestSafeTile(const Train *v, TileIndex tile, Trackdir td, bool override_railtype)
{
	return openttd_rust_yapf_train_find_nearest_safe_tile(&GetRustSharedServices(), RustVehicle(v), tile.base(), td, override_railtype) != 0;
}

void YapfNotifyTrackLayoutChange(TileIndex tile, Track track)
{
	openttd_rust_rail_invalidate(tile.base(), track);
}

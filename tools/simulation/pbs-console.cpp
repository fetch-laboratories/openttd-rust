/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file pbs-console.cpp Transient endpoint/output and trigger-order gap against unchanged bodies. */
#include "stdafx.h"
#include "viewport_func.h"
#include "vehicle_func.h"
#include "newgrf_station.h"
#include "pathfinder/follow_track.hpp"
#include "console_internal.h"
#include "console_func.h"
#include "bridge_map.h"
#include "clear_map.h"
#ifdef WITH_RUST
#include "rust/animation_ffi.h"
#include "rust/pbs_ffi.h"
#include "rust/station_resolver_ffi.h"
#endif
using PbsTrace = std::vector<std::tuple<uint8_t, uint32_t, uint16_t>>;
static PbsTrace *_pbs_trace = nullptr;
#define PBS_TRIGGER_WRAPPER(Symbol, Trigger, Kind) \
extern "C" void __real_##Symbol(BaseStation *, TileIndex, Trigger, CargoType); \
extern "C" void __wrap_##Symbol(BaseStation *st, TileIndex t, Trigger trigger, CargoType cargo) { \
	if (_pbs_trace != nullptr) _pbs_trace->emplace_back(Kind, t.base(), st->index.base()); \
	__real_##Symbol(st, t, trigger, cargo); \
}
#ifndef WITH_RUST
PBS_TRIGGER_WRAPPER(_Z27TriggerStationRandomisationP11BaseStationN10StrongType7TypedefIj12TileIndexTagJNS1_7CompareENS1_7IntegerENS1_10CompatibleIiEENS6_IlEEEEE20StationRandomTriggerh, StationRandomTrigger, 0)
PBS_TRIGGER_WRAPPER(_Z23TriggerStationAnimationP11BaseStationN10StrongType7TypedefIj12TileIndexTagJNS1_7CompareENS1_7IntegerENS1_10CompatibleIiEENS6_IlEEEEE23StationAnimationTriggerh, StationAnimationTrigger, 1)
#else
/* PBS calls the Rust owners directly. Observe their entries for both the unchanged
 * body's native facade and the Rust caller, forwarding every typed argument. */
extern "C" decltype(openttd_rust_station_randomisation) __real_openttd_rust_station_randomisation;
extern "C" void __wrap_openttd_rust_station_randomisation(BaseStation *st, uint32_t tile, uint8_t trigger, uint8_t cargo)
{
	if (_pbs_trace != nullptr) _pbs_trace->emplace_back(0, tile, st->index.base());
	__real_openttd_rust_station_randomisation(st, tile, trigger, cargo);
}
static_assert(std::is_same_v<decltype(__wrap_openttd_rust_station_randomisation), decltype(openttd_rust_station_randomisation)>);
extern "C" decltype(openttd_rust_trigger_station) __real_openttd_rust_trigger_station;
extern "C" void __wrap_openttd_rust_trigger_station(BaseStation *st, uint32_t tile, uint8_t trigger, uint8_t cargo)
{
	if (_pbs_trace != nullptr) _pbs_trace->emplace_back(1, tile, st->index.base());
	__real_openttd_rust_trigger_station(st, tile, trigger, cargo);
}
static_assert(std::is_same_v<decltype(__wrap_openttd_rust_trigger_station), decltype(openttd_rust_trigger_station)>);
#endif
#undef PBS_TRIGGER_WRAPPER
static TrackBits PbsTrainTrack(Train *v) {
#ifdef WITH_RUST
	return v->GetTrack();
#else
	return v->track;
#endif
}
static void PbsTrainTrack(Train *v, TrackBits track) {
#ifdef WITH_RUST
	v->SetTrack(track);
#else
	v->track = track;
#endif
}
static RailTypes PbsTrainRailTypes(const Train *v) {
#ifdef WITH_RUST
	return v->GetRailTypes();
#else
	return v->railtypes;
#endif
}
#include "pbs-original.hpp"
using PbsMap = std::vector<std::array<uint16_t, 10>>;
static PbsMap PbsMapRead()
{
	PbsMap rows; for (TileIndex i(0); i < Map::Size(); ++i) { Tile t(i); rows.push_back({t.type(), t.height(), t.m1(), t.m2(), t.m3(), t.m4(), t.m5(), t.m6(), t.m7(), t.m8()}); } return rows;
}
static void PbsMapRestore(const PbsMap &rows)
{
	for (uint32_t i = 0; i < rows.size(); ++i) { Tile t(TileIndex{i}); const auto &r = rows[i]; t.type() = uint8_t(r[0]); t.height() = uint8_t(r[1]); t.m1() = uint8_t(r[2]); t.m2() = r[3]; t.m3() = uint8_t(r[4]); t.m4() = uint8_t(r[5]); t.m5() = uint8_t(r[6]); t.m6() = uint8_t(r[7]); t.m7() = uint8_t(r[8]); t.m8() = r[9]; }
}
static void PbsRequire(bool okay) { if (!okay) { fmt::print(stderr, "PBS mismatch\n"); std::abort(); } }
static bool PbsSame(PBSTileInfo a, PBSTileInfo b) { return a.tile == b.tile && a.trackdir == b.trackdir && a.okay == b.okay; }
static bool PbsCheckMask(TileIndex tile, Trackdir td)
{
	PbsOriginalMask ft{GetTileOwner(tile), GetRailTypeInfo(GetTileRailType(tile))->compatible_railtypes};
	if (!ft.Follow(tile, td)) return false;
#ifdef WITH_RUST
	OpenTTDFollowTrackState actual{};
	openttd_rust_follower_init(&actual, &GetRustSharedServices(), TRANSPORT_RAIL, nullptr, 0, 0, true, true, false);
	actual.old_tile = ft.old_tile; actual.old_td = ft.old_td; actual.new_tile = ft.new_tile; actual.new_td_bits = ft.new_td_bits;
	actual.exitdir = ft.exitdir; actual.is_station = ft.is_station; actual.tiles_skipped = ft.tiles_skipped;
	PbsRequire(openttd_rust_follower_mask(&GetRustSharedServices(), &actual) == expected && actual.new_td_bits == ft.new_td_bits && actual.err == ft.err);
#endif
	return true;
}
static PBSTileInfo PbsFollow(Owner owner, RailTypes types, TileIndex tile, Trackdir td)
{
#ifdef WITH_RUST
	auto r = openttd_rust_pbs_follow(&GetPbsServices(), owner.base(), types.base(), tile.base(), td);
	return {TileIndex{r.tile}, static_cast<Trackdir>(r.td), r.okay != 0};
#else
	return PbsOriginalFollowReservation(owner, types, tile, td);
#endif
}
static bool PbsProbe([[maybe_unused]] std::span<std::string_view> args)
{
	const auto original = PbsMapRead(); const Randomizer random = _random; uint32_t checked = 0;
	for (Train *v : Train::Iterate()) if (v->IsFrontEngine()) {
		for (Vehicle *initial : {static_cast<Vehicle *>(nullptr), static_cast<Vehicle *>(v)}) { Vehicle *a = initial, *b = initial; PbsRequire(PbsSame(PbsOriginalFollowTrainReservation(v, &a), FollowTrainReservation(v, &b)) && a == b); ++checked; }
		for (bool end : {false, true}) for (bool no90 : {false, true}) { PbsRequire(PbsOriginalIsSafeWaitingPosition(v, v->tile, v->GetVehicleTrackdir(), end, no90) == IsSafeWaitingPosition(v, v->tile, v->GetVehicleTrackdir(), end, no90)); PbsRequire(PbsOriginalIsWaitingPositionFree(v, v->tile, v->GetVehicleTrackdir(), no90) == IsWaitingPositionFree(v, v->tile, v->GetVehicleTrackdir(), no90)); }
	}
	for (TileIndex t(0); t < Map::Size(); ++t) {
		PbsRequire(PbsOriginalGetReservedTrackbits(t) == GetReservedTrackbits(t));
		for (Trackdir td : SetTrackdirBitIterator(TrackStatusToTrackdirBits(GetTileTrackStatus(t, TRANSPORT_RAIL, 0)))) PbsCheckMask(t, td);
		for (Track track : SetTrackBitIterator(GetReservedTrackbits(t))) { PbsRequire(PbsOriginalGetTrainForReservation(t, track) == GetTrainForReservation(t, track)); ++checked; }
		if (IsRailStationTile(t)) { auto dir = AxisToDiagDir(GetRailStationAxis(t)); PbsOriginalSetRailStationPlatformReservation(t, dir, false); const auto before = PbsMapRead(); PbsOriginalSetRailStationPlatformReservation(t, dir, true); const auto expected = PbsMapRead(); PbsMapRestore(before); SetRailStationPlatformReservation(t, dir, true); PbsRequire(expected == PbsMapRead()); PbsMapRestore(original);
			PbsTrace a, b; SetRailStationReservation(t, false); _pbs_trace = &a; PbsRequire(PbsOriginalTryReserveRailTrack(t, GetRailStationTrack(t), true)); const auto state = PbsMapRead(); const Randomizer expected_random = _random; PbsMapRestore(original); SetRailStationReservation(t, false); _random = random; _pbs_trace = &b; PbsRequire(TryReserveRailTrack(t, GetRailStationTrack(t), true)); _pbs_trace = nullptr; PbsRequire(state == PbsMapRead() && a.size() == 2 && a == b && _random.state[0] == expected_random.state[0] && _random.state[1] == expected_random.state[1]); PbsMapRestore(original); _random = random;
			const TileIndex next = TileAddByDiagDir(t, dir), last = TileAddByDiagDir(next, dir); if (IsCompatibleTrainStationTile(next, t) && IsCompatibleTrainStationTile(last, t)) { SetRailStationPlatformReservation(t, dir, false); SetRailStationReservation(t, true); SetRailStationReservation(next, true); const auto td = DiagDirToDiagTrackdir(dir); PbsRequire(PbsSame(PbsOriginalFollowReservation(GetTileOwner(t), GetRailTypeInfo(GetTileRailType(t))->compatible_railtypes, t, td), PbsFollow(GetTileOwner(t), GetRailTypeInfo(GetTileRailType(t))->compatible_railtypes, t, td))); PbsMapRestore(original); }
		}
	}
	/* A valid four-corner reservation loop, then collocated live trains in hash
	 * order opposite pool ID order. Restore every canonical scalar before play. */
	const TileIndex a = TileXY(20, 20); const std::array<TileIndex, 4> tiles{a, a + 1, a + Map::SizeX() + 1, a + Map::SizeX()}; const std::array<Track, 4> tracks{TRACK_LOWER, TRACK_RIGHT, TRACK_UPPER, TRACK_LEFT};
	for (uint y = 19; y <= 23; ++y) for (uint x = 19; x <= 23; ++x) Tile(TileXY(x, y)).height() = 0;
	for (uint i = 0; i < 4; ++i) { MakeRailNormal(tiles[i], Owner(0), TrackToTrackBits(tracks[i]), RAILTYPE_RAIL); SetTrackReservation(tiles[i], TrackToTrackBits(tracks[i])); }
	PbsRequire(PbsCheckMask(a, TRACKDIR_LOWER_W));
	PbsRequire(PbsSame(PbsOriginalFollowReservation(Owner(0), RailTypes(1), a, TRACKDIR_LOWER_W), PbsFollow(Owner(0), RailTypes(1), a, TRACKDIR_LOWER_W))); PbsRequire(GetTrainForReservation(a, TRACK_LOWER) == PbsOriginalGetTrainForReservation(a, TRACK_LOWER));
	std::vector<std::tuple<Train *, TileIndex, TrackBits>> moved;
	for (Train *v : Train::Iterate()) if (v->IsFrontEngine() && !v->vehstatus.Test(VehState::Crashed) && moved.size() < 2) { moved.emplace_back(v, v->tile, PbsTrainTrack(v)); v->tile = tiles[1]; PbsTrainTrack(v, TRACK_BIT_WORMHOLE); v->UpdatePosition(); }
	PbsRequire(moved.size() == 2);
	PbsRequire(!IsWaitingPositionFree(std::get<0>(moved[0]), a, TRACKDIR_LOWER_W, false));
	PbsRequire(moved.size() == 2 && GetTrainForReservation(a, TRACK_LOWER) == std::get<0>(moved[0]) && GetTrainForReservation(a, TRACK_LOWER) == PbsOriginalGetTrainForReservation(a, TRACK_LOWER));
	/* One-way blocking, its PBS search exception, and opposing PBS waiting policy. */
	SetHasSignals(tiles[1], true); SetSignalType(tiles[1], TRACK_RIGHT, SIGTYPE_BLOCK); SetPresentSignals(tiles[1], SignalAlongTrackdir(TRACKDIR_RIGHT_S));
	PbsRequire(GetTrainForReservation(tiles[1], TRACK_RIGHT) == PbsOriginalGetTrainForReservation(tiles[1], TRACK_RIGHT));
	SetPresentSignals(tiles[1], SignalAlongTrackdir(TRACKDIR_RIGHT_N)); PbsRequire(PbsSame(PbsOriginalFollowReservation(Owner(0), RailTypes(1), a, TRACKDIR_LOWER_W), PbsFollow(Owner(0), RailTypes(1), a, TRACKDIR_LOWER_W)));
	SetSignalType(tiles[1], TRACK_RIGHT, SIGTYPE_PBS_ONEWAY); for (bool end : {false, true}) PbsRequire(PbsOriginalIsSafeWaitingPosition(std::get<0>(moved[0]), a, TRACKDIR_LOWER_W, end, false) == IsSafeWaitingPosition(std::get<0>(moved[0]), a, TRACKDIR_LOWER_W, end, false));
	SetPresentSignals(tiles[1], SignalAlongTrackdir(TRACKDIR_RIGHT_S)); PbsRequire(GetTrainForReservation(tiles[1], TRACK_RIGHT) == PbsOriginalGetTrainForReservation(tiles[1], TRACK_RIGHT));
	for (auto [v, tile, track] : moved) { v->tile = tile; PbsTrainTrack(v, track); v->UpdatePosition(); }
	PbsMapRestore(original);
	/* The 2006 input has a rail tunnel and road bridges. Add a native rail bridge
	 * for the transient end query without changing pool or infrastructure state. */
	const TileIndex bridge = TileXY(40, 40); for (uint y = 39; y <= 41; ++y) for (uint x = 39; x <= 43; ++x) { MakeClear(TileXY(x, y), CLEAR_GRASS, 3); Tile(TileXY(x, y)).height() = 0; }
	MakeRailBridgeRamp(bridge, Owner(0), 0, DIAGDIR_SW, RAILTYPE_RAIL); SetBridgeMiddle(bridge + 1, AXIS_X); MakeRailBridgeRamp(bridge + 2, Owner(0), 0, DIAGDIR_NE, RAILTYPE_RAIL);
	/* Isolate end lookup from unrelated live paths; both roles restore the same map. */
	for (TileIndex t(0); t < Map::Size(); ++t) for (Track track : SetTrackBitIterator(GetReservedTrackbits(t))) UnreserveRailTrack(t, track);
	const auto cleared = PbsMapRead(); std::array<uint32_t, 2> ends{};
	/* The other end can hold the train even when the queried end has no vehicle. */
	for (bool bridge : {false, true}) for (TileIndex t(0); t < Map::Size(); ++t) if (IsTileType(t, MP_TUNNELBRIDGE) && GetTunnelBridgeTransportType(t) == TRANSPORT_RAIL && IsBridgeTile(t) == bridge) {
		bool occupied = false; for ([[maybe_unused]] Vehicle *v : VehiclesOnTile(t)) occupied = true; if (occupied) continue;
		Train *v = std::get<0>(moved[0]); const TileIndex old = v->tile; const TrackBits track = PbsTrainTrack(v); const TileIndex other = GetOtherTunnelBridgeEnd(t); SetTunnelBridgeReservation(t, true); SetTunnelBridgeReservation(other, false); v->tile = other; PbsTrainTrack(v, TRACK_BIT_WORMHOLE); v->UpdatePosition();
		PbsRequire(GetTrainForReservation(t, DiagDirToDiagTrack(GetTunnelBridgeDirection(t))) == v && PbsOriginalGetTrainForReservation(t, DiagDirToDiagTrack(GetTunnelBridgeDirection(t))) == v); ++ends[bridge]; v->tile = old; PbsTrainTrack(v, track); v->UpdatePosition(); PbsMapRestore(cleared); break;
	}
	PbsMapRestore(original); _random = random; fmt::print(stderr, "PBS checked {}\nPBS ends {} {}\n", checked, ends[0], ends[1]); PbsRequire(checked != 0); return true;
}
// PBS registration
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv() { __real__Z22IConsoleStdLibRegisterv(); IConsole::CmdRegister("pbs_probe", PbsProbe); }

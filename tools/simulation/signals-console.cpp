/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file signals-console.cpp Identical real-map inputs and pending-set inspection. */
/* signal.cpp is included by the scenario adapter builder before this file, so
 * portable work sets are visible without changing the pinned source checkout. */
#include "console_internal.h"
#include "console_func.h"
#include "command_func.h"
#include "company_func.h"
#include "rail_cmd.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "depot_base.h"
#include "station_base.h"
#include "tunnelbridge_map.h"
#include "company_base.h"
#include "clear_map.h"
#include "company_cmd.h"
#include "landscape_cmd.h"
#include "newgrf_station.h"
#include "road_cmd.h"
#include "station_cmd.h"
#include "terraform_cmd.h"
#include "tunnelbridge_cmd.h"
#include "vehicle_func.h"
#include "settings_type.h"
#include "../../../src/rust/signal_ffi.h"

#ifdef WITH_RUST
/* Measurement belongs only to this relinked fixture. The ordinary native query
 * and scalar accessor remain unchanged; count actual short-circuited reads. */
static uint64_t *_signal_occupancy_reads = nullptr;
extern "C" uint8_t __real_openttd_rust_train_state_get_track(const OpenTTDTrainState *);
extern "C" uint8_t __wrap_openttd_rust_train_state_get_track(const OpenTTDTrainState *state)
{
	if (_signal_occupancy_reads != nullptr) ++*_signal_occupancy_reads;
	return __real_openttd_rust_train_state_get_track(state);
}
static bool SignalCountTrainOnTile(uint32_t tile) noexcept
{
	uint64_t reads = 0;
	Backup<uint64_t *> counter(_signal_occupancy_reads, &reads);
	bool result = SignalTrainOnTile(tile);
	counter.Restore();
	openttd_rust_signal_count_occupancy(reads);
	return result;
}
static bool SignalCountTrainOnTracks(uint32_t tile, uint8_t tracks) noexcept
{
	uint64_t reads = 0;
	Backup<uint64_t *> counter(_signal_occupancy_reads, &reads);
	bool result = SignalTrainOnTracks(tile, tracks);
	counter.Restore();
	openttd_rust_signal_count_occupancy(reads);
	return result;
}
#endif

extern Company *DoStartupNewCompany(bool is_ai, CompanyID company);

[[noreturn]] static void SignalFixtureAbort(int line)
{
	fmt::print(stderr, "SIGNALS setup-failure {}\n", line);
	std::fflush(stderr);
	std::abort();
}
static void SignalCost(const char *name, const CommandCost &cost)
{
	fmt::print(stderr, "SIGNALS cost {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage());
	if (cost.Failed()) SignalFixtureAbort(__LINE__);
}
#ifndef WITH_RUST
template <typename Direction, uint N>
static void PrintPending(SmallSet<Direction, N> &set)
{
	std::array<std::pair<TileIndex, Direction>, N> buffer;
	uint count = set.Items();
	/* Inspect by drain/restore in the same translation unit. Add never dedups;
	 * the overflow flag is unchanged and restoration preserves the active order. */
	for (uint i = count; i > 0; --i) set.Get(&buffer[i - 1].first, &buffer[i - 1].second);
	for (uint i = 0; i < count; ++i) {
		fmt::print(stderr, " {}:{}", buffer[i].first.base(), static_cast<uint8_t>(buffer[i].second));
		set.Add(buffer[i].first, buffer[i].second);
	}
}
#endif
static void SignalPending(const char *name)
{
#ifdef WITH_RUST
	OpenTTDSignalSnapshot snapshot;
	openttd_rust_signal_inspect(&snapshot);
	fmt::print(stderr, "SIGNALS pending {} {} {} {} {} {} {} {}", name, snapshot.tbu_count, snapshot.tbd_count, snapshot.global_count, snapshot.overflow[0], snapshot.overflow[1], snapshot.overflow[2], snapshot.last_owner);
	for (auto [items, count] : {std::pair{snapshot.tbu, snapshot.tbu_count}, std::pair{snapshot.tbd, snapshot.tbd_count}, std::pair{snapshot.global, snapshot.global_count}}) {
		fmt::print(stderr, " |");
		for (uint i = 0; i < count; ++i) fmt::print(stderr, " {}:{}", items[i].tile, items[i].dir);
	}
#else
	fmt::print(stderr, "SIGNALS pending {} {} {} {} {} {} {} {} |", name, _tbuset.Items(), _tbdset.Items(), _globset.Items(), _tbuset.Overflowed(), _tbdset.Overflowed(), _globset.Overflowed(), _last_owner.base());
	PrintPending(_tbuset);
	fmt::print(stderr, " |");
	PrintPending(_tbdset);
	fmt::print(stderr, " |");
	PrintPending(_globset);
#endif
	fmt::print(stderr, "\n");
}
static std::vector<std::pair<TileIndex, Track>> SignalTracks(Owner owner)
{
	std::vector<std::pair<TileIndex, Track>> tracks;
	for (TileIndex tile(0); tile < Map::Size(); ++tile) {
		if (!IsTileType(tile, MP_RAILWAY) || GetTileOwner(tile) != owner || !HasSignals(tile)) continue;
		for (Track track = TRACK_BEGIN; track < TRACK_END; ++track) if (HasTrack(tile, track) && HasSignalOnTrack(tile, track)) tracks.emplace_back(tile, track);
	}
	return tracks;
}
static void SignalClassify(const std::vector<std::pair<TileIndex, Track>> &tracks, Owner owner, const char *name)
{
	uint states[3] = {};
	for (auto [tile, track] : tracks) ++states[UpdateSignalsOnSegment(tile, TrackdirToExitdir(TrackToTrackdir(track)), owner)];
	fmt::print(stderr, "SIGNALS {} {} {} {}\n", name, states[0], states[1], states[2]);
}
#ifndef WITH_RUST
template <typename Direction, uint N>
static OpenTTDSignalSetTrace SignalSetTrace(std::string_view name)
{
	SmallSet<Direction, N> set(name);
	Direction zero = static_cast<Direction>(0), invalid = static_cast<Direction>(255);
	set.Add(TileIndex(1), zero);
	set.Add(TileIndex(2), invalid);
	set.Add(TileIndex(1), zero);
	set.Add(TileIndex(1), static_cast<Direction>(2));
	set.Remove(TileIndex(1), zero);
	OpenTTDSignalSetTrace r{};
	r.duplicate_count = set.Items();
	TileIndex tile;
	Direction dir;
	for (auto &item : r.duplicate_pop) { set.Get(&tile, &dir); item = {tile.base(), static_cast<uint8_t>(dir)}; }
	set.Reset();
	for (uint i = 0; i < N; ++i) set.Add(TileIndex(i), static_cast<Direction>(i & 3));
	r.full_failed = !set.Add(TileIndex(UINT32_MAX), invalid);
	r.overflow_full = set.Overflowed();
	set.Remove(TileIndex(0), zero);
	/* Drain/restore exposes the first element without accessing private data. */
	std::array<std::pair<TileIndex, Direction>, N> buffer;
	uint count = set.Items();
	for (uint i = count; i > 0; --i) set.Get(&buffer[i - 1].first, &buffer[i - 1].second);
	r.first_after_remove = {buffer[0].first.base(), static_cast<uint8_t>(buffer[0].second)};
	for (uint i = 0; i < count; ++i) set.Add(buffer[i].first, buffer[i].second);
	r.refill_succeeded = set.Add(TileIndex(UINT32_MAX), invalid);
	set.Get(&tile, &dir);
	r.refill_pop = {tile.base(), static_cast<uint8_t>(dir)};
	while (set.Get(&tile, &dir)) { }
	r.drained_count = set.Items();
	r.overflow_drained = set.Overflowed();
	/* Sticky overflow must not reject insertion into the drained set. */
	r.refill_succeeded &= set.Add(TileIndex(UINT32_MAX), invalid);
	set.Reset();
	r.overflow_reset = set.Overflowed();
	r.refill_succeeded &= set.Items() == 0;
	return r;
}
#endif
static void SignalSetProbe()
{
	OpenTTDSignalSetTrace traces[3];
#ifdef WITH_RUST
	openttd_rust_signal_workset_probe(&_signal_services, traces);
#else
	traces[0] = SignalSetTrace<Trackdir, 64>("_tbuset");
	traces[1] = SignalSetTrace<DiagDirection, 256>("_tbdset");
	traces[2] = SignalSetTrace<DiagDirection, 128>("_globset");
#endif
	for (uint i = 0; i < 3; ++i) {
		auto &r = traces[i];
		fmt::print(stderr, "SIGNALS sets {} {}", std::array{64, 256, 128}[i], r.duplicate_count);
		for (auto &item : r.duplicate_pop) fmt::print(stderr, " {}:{}", item.tile, item.dir);
		fmt::print(stderr, " {} {} {}:{} {} {}:{} {} {} {}\n", r.full_failed, r.overflow_full, r.first_after_remove.tile, r.first_after_remove.dir, r.refill_succeeded, r.refill_pop.tile, r.refill_pop.dir, r.drained_count, r.overflow_drained, r.overflow_reset);
	}
}
/* Declared terrain input: flatten an unused clear/tree rectangle at height 1.
 * Rail and signal geometry is then built through real commands. */
static TileIndex SignalFlatArea(uint n)
{
	for (uint y = 2; y + n + 2 < Map::SizeY(); ++y) {
		for (uint x = 2; x + n + 2 < Map::SizeX(); ++x) {
			bool clear = true;
			for (uint dy = 0; dy < n + 2 && clear; ++dy) for (uint dx = 0; dx < n + 2; ++dx) {
				TileType type = GetTileType(TileXY(x + dx, y + dy));
				if (type != MP_CLEAR && type != MP_TREES) { clear = false; break; }
			}
			if (!clear) continue;
			TileIndex origin = TileXY(x, y);
			for (uint dy = 0; dy < n + 2; ++dy) for (uint dx = 0; dx < n + 2; ++dx) {
				TileIndex tile = origin + TileDiffXY(dx, dy);
				SetTileHeight(tile, 1);
				MakeClear(tile, CLEAR_GRASS, 3);
			}
			return origin;
		}
	}
	SignalFixtureAbort(__LINE__);
}
static void SignalRichCompany(Owner owner)
{
	Company *c = Company::Get(owner);
#ifdef WITH_RUST
	c->Finances().money = Money(1000000000);
#else
	c->money = Money(1000000000);
#endif
}
static void SignalOverflowLayout(Owner owner)
{
	constexpr uint N = 20;
	TileIndex origin = SignalFlatArea(N);
	SignalRichCompany(owner);
	std::vector<std::pair<TileIndex, Track>> boundary;
	for (uint y = 1; y <= N; ++y) for (uint x = 1; x <= N; ++x) {
		TileIndex tile = origin + TileDiffXY(x, y);
		bool edge_x = x == 1 || x == N, edge_y = y == 1 || y == N;
		if (edge_x && edge_y) continue;
		if (!edge_y) SignalCost("rail-x", Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlag::Execute, tile, RAILTYPE_RAIL, TRACK_X, false));
		if (!edge_x) SignalCost("rail-y", Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlag::Execute, tile, RAILTYPE_RAIL, TRACK_Y, false));
		if (edge_x || edge_y) boundary.emplace_back(tile, edge_x ? TRACK_X : TRACK_Y);
	}
	for (auto [tile, track] : boundary) SignalCost("boundary", Command<CMD_BUILD_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, tile, track, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0));
	UpdateSignalsInBuffer();
	TileIndex center = origin + TileDiffXY(2, 2);
	fmt::print(stderr, "SIGNALS overflow {}\n", static_cast<uint8_t>(UpdateSignalsOnSegment(center, DIAGDIR_NE, owner)));
	SignalPending("overflow-reset");
	for (uint i = 0; i < 12; ++i) SignalCost("open-boundary", Command<CMD_REMOVE_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, boundary[i].first, boundary[i].second));
	UpdateSignalsInBuffer();
	fmt::print(stderr, "SIGNALS follow-up {}\n", static_cast<uint8_t>(UpdateSignalsOnSegment(center, DIAGDIR_NE, owner)));
	SignalPending("overflow-follow-up");
	/* Command-built exit/combo geometry; declared live signal states isolate
	 * no-green and multiple-green exit rules before ordinary propagation. */
	for (uint i = 12; i < boundary.size(); ++i) {
		auto [tile, track] = boundary[i];
		SignalType kind = i == 12 ? SIGTYPE_COMBO : SIGTYPE_EXIT;
		SignalCost("exit-combo", Command<CMD_BUILD_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, tile, track, kind, SIG_ELECTRIC, true, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, SignalOnTrack(track)));
	}
	UpdateSignalsInBuffer();
	for (SignalState state : {SIGNAL_STATE_RED, SIGNAL_STATE_GREEN}) {
		for (uint i = 12; i < boundary.size(); ++i) {
			auto [tile, track] = boundary[i];
			Trackdir td = TrackToTrackdir(track);
			SetSignalStateByTrackdir(tile, td, state);
			SetSignalStateByTrackdir(tile, ReverseTrackdir(td), state);
		}
		fmt::print(stderr, "SIGNALS combo-input {} {}\n", state, static_cast<uint8_t>(UpdateSignalsOnSegment(center, DIAGDIR_NE, owner)));
	}
	CompanyID other = CompanyID::Invalid();
	for (const Company *company : Company::Iterate()) if (company->index != owner) { other = company->index; break; }
	if (other == CompanyID::Invalid()) {
		Company *company = DoStartupNewCompany(false, CompanyID::Invalid());
		if (company == nullptr) SignalFixtureAbort(__LINE__);
		other = company->index;
	}
	if (other == CompanyID::Invalid()) SignalFixtureAbort(__LINE__);
	TileIndex foreign = origin + TileDiffXY(0, 2);
	Backup<CompanyID> foreign_company(_current_company, other);
	SignalCost("foreign-rail", Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlag::Execute, foreign, RAILTYPE_RAIL, TRACK_X, false));
	UpdateSignalsInBuffer();
	foreign_company.Restore();
	fmt::print(stderr, "SIGNALS owner-barrier {}\n", static_cast<uint8_t>(UpdateSignalsOnSegment(foreign, DIAGDIR_SW, owner)));
	/* Empty perimeter and its empty neighbour exercise the buffer continue. */
	AddSideToSignalBuffer(origin, DIAGDIR_SE, owner);
	UpdateSignalsInBuffer();
	SignalPending("empty-neighbour");

}
/* Rail command owner inputs (#265): every result is printed and compared; the
 * geometry is built only through real commands on a declared flat area. */
static void RailPrint(std::string_view name, const CommandCost &cost)
{
	fmt::print(stderr, "RAILCMD {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), static_cast<int>(cost.GetExpensesType()));
}
/** Test run, then execution, as a client command would perform them. */
template <Commands Tcmd, typename... Args>
static void RailRun(std::string_view name, Args... args)
{
	RailPrint(fmt::format("{}-test", name), Command<Tcmd>::Do(DoCommandFlags{}, args...));
	RailPrint(name, Command<Tcmd>::Do(DoCommandFlag::Execute, args...));
	UpdateSignalsInBuffer(); // As a posted command does after execution.
}
static void RailTerraform(std::string_view name, TileIndex tile, Slope corners, bool up)
{
	RailPrint(name, std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(DoCommandFlag::Execute, tile, corners, up)));
}
static CompanyID RailCompany()
{
	Company *company = DoStartupNewCompany(false, CompanyID::Invalid());
	if (company == nullptr) SignalFixtureAbort(__LINE__);
	SignalRichCompany(company->index);
	return company->index;
}
static void RailCommandsLayout(Owner owner)
{
	TileIndex o = SignalFlatArea(24);
	SignalRichCompany(owner);
	auto t = [o](uint x, uint y) { return o + TileDiffXY(x, y); };
	auto signal = [](SignalType type, bool convert, bool skip, bool ctrl, uint8_t copy) {
		return std::tuple{TRACK_X, type, SIG_ELECTRIC, convert, skip, ctrl, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, uint8_t{0}, copy};
	};
	/* Drags: straight, repeated, strict, diagonal and invalid. */
	RailRun<CMD_BUILD_RAILROAD_TRACK>("drag", t(12, 2), t(2, 2), RAILTYPE_RAIL, TRACK_X, false, false);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("drag-again", t(12, 2), t(2, 2), RAILTYPE_RAIL, TRACK_X, false, false);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("drag-strict", t(12, 2), t(2, 2), RAILTYPE_RAIL, TRACK_X, false, true);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("drag-diagonal", t(16, 8), t(20, 4), RAILTYPE_RAIL, TRACK_UPPER, false, false);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("drag-invalid", t(5, 7), t(2, 4), RAILTYPE_RAIL, TRACK_X, false, false);
	/* Single pieces: errors, type conversion, signals in the way, crossings of tracks. */
	RailRun<CMD_BUILD_SINGLE_RAIL>("already", t(3, 2), RAILTYPE_RAIL, TRACK_X, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("mono-on-rail", t(3, 2), RAILTYPE_MONO, TRACK_Y, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("electric-on-rail", t(4, 2), RAILTYPE_ELECTRIC, TRACK_Y, false);
	RailRun<CMD_BUILD_SINGLE_SIGNAL>("signal", t(6, 2), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0);
	RailRun<CMD_BUILD_SINGLE_RAIL>("signals-first", t(6, 2), RAILTYPE_RAIL, TRACK_Y, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("auto-remove", t(6, 2), RAILTYPE_RAIL, TRACK_Y, true);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("remove-diagonal", t(6, 2), TRACK_X);
	RailRun<CMD_BUILD_SINGLE_SIGNAL>("signal-8", t(8, 2), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("remove-signalled", t(8, 2), TRACK_X);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("remove-missing", t(9, 2), TRACK_Y);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("remove-clear", t(9, 5), TRACK_Y);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("remove-last", t(12, 2), TRACK_X);
	/* Slopes and foundations. */
	RailTerraform("raise-14", t(14, 2), SLOPE_N, true);
	RailRun<CMD_BUILD_SINGLE_RAIL>("slope-x", t(14, 2), RAILTYPE_RAIL, TRACK_X, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("slope-y", t(14, 2), RAILTYPE_RAIL, TRACK_Y, false);
	RailTerraform("raise-17", t(17, 2), SLOPE_N, true);
	RailTerraform("steep-17", t(17, 2), SLOPE_N, true);
	RailRun<CMD_BUILD_SINGLE_RAIL>("steep-x", t(17, 2), RAILTYPE_RAIL, TRACK_X, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("steep-lower", t(17, 2), RAILTYPE_RAIL, TRACK_LOWER, false);
	/* Level crossings on a company road. */
	for (uint x = 4; x <= 8; ++x) RailPrint("road", Command<CMD_BUILD_ROAD>::Do(DoCommandFlag::Execute, t(x, 10), ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
	RailPrint("oneway", Command<CMD_BUILD_ROAD>::Do(DoCommandFlag::Execute, t(4, 10), ROAD_X, ROADTYPE_ROAD, DRD_SOUTHBOUND, TownID::Invalid()));
	RailRun<CMD_BUILD_SINGLE_RAIL>("crossing-oneway", t(4, 10), RAILTYPE_RAIL, TRACK_Y, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("crossing", t(6, 10), RAILTYPE_RAIL, TRACK_Y, false);
	RailRun<CMD_BUILD_SINGLE_RAIL>("crossing-again", t(6, 10), RAILTYPE_RAIL, TRACK_Y, false);
	RailPrint("along-road-test", Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlags{}, t(5, 10), RAILTYPE_RAIL, TRACK_X, false));
	RailRun<CMD_REMOVE_SINGLE_RAIL>("crossing-remove", t(6, 10), TRACK_Y);
	RailRun<CMD_BUILD_SINGLE_RAIL>("crossing-rebuild", t(7, 10), RAILTYPE_RAIL, TRACK_Y, false);
	/* Depots: build, same, rotate, slopes, rail over a depot. */
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot", t(20, 12), RAILTYPE_RAIL, DIAGDIR_NE);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-same", t(20, 12), RAILTYPE_RAIL, DIAGDIR_NE);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-rotate", t(20, 12), RAILTYPE_RAIL, DIAGDIR_SE);
	RailPrint("rail-on-depot-test", Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlags{}, t(20, 12), RAILTYPE_RAIL, TRACK_X, false));
	RailTerraform("raise-16", t(16, 12), SLOPE_N, true);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-wrong-slope", t(16, 12), RAILTYPE_RAIL, DIAGDIR_SW);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-slope", t(16, 12), RAILTYPE_RAIL, DIAGDIR_NE);
	/* Single signals: cycle, type, variant, PBS conversion, copy and errors. */
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("signal-3", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, false, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("cycle-side", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, false, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("cycle-type", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, true, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("variant", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, true, false, true, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("to-pbs", t(3, 2), a...); }, signal(SIGTYPE_PBS_ONEWAY, true, false, false, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("copy", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, false, 0xC));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("skip", t(3, 2), a...); }, signal(SIGTYPE_BLOCK, false, true, false, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("convert-none", t(5, 2), a...); }, signal(SIGTYPE_BLOCK, true, false, false, 0));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("copy-invalid", t(5, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, false, 0x1));
	std::apply([&](auto... a) { RailRun<CMD_BUILD_SINGLE_SIGNAL>("overlap", t(4, 2), a...); }, signal(SIGTYPE_BLOCK, false, false, false, 0));
	RailRun<CMD_REMOVE_SINGLE_SIGNAL>("signal-remove", t(3, 2), TRACK_X);
	RailRun<CMD_REMOVE_SINGLE_SIGNAL>("signal-remove-none", t(3, 2), TRACK_X);
	RailRun<CMD_BUILD_SIGNAL_TRACK>("signal-drag", t(2, 2), t(11, 2), TRACK_X, SIGTYPE_PBS, SIG_SEMAPHORE, false, false, false, 2);
	RailRun<CMD_BUILD_SIGNAL_TRACK>("signal-density", t(2, 2), t(11, 2), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, 0);
	/* Autofill through a rail bridge, minimised gaps and removal. */
	RailRun<CMD_BUILD_RAILROAD_TRACK>("approach-w", t(6, 18), t(2, 18), RAILTYPE_RAIL, TRACK_X, false, false);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("approach-e", t(16, 18), t(11, 18), RAILTYPE_RAIL, TRACK_X, false, false);
	RailRun<CMD_BUILD_BRIDGE>("bridge", t(10, 18), t(7, 18), TRANSPORT_RAIL, BridgeType{0}, uint8_t{RAILTYPE_RAIL});
	RailRun<CMD_BUILD_SIGNAL_TRACK>("autofill", t(2, 18), t(16, 18), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, true, false, 3);
	RailRun<CMD_REMOVE_SIGNAL_TRACK>("autofill-remove", t(2, 18), t(16, 18), TRACK_X, true);
	RailRun<CMD_BUILD_SIGNAL_TRACK>("minimise", t(2, 18), t(16, 18), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, true, true, 4);
	RailRun<CMD_BUILD_SIGNAL_TRACK>("minimise-drag", t(2, 2), t(11, 2), TRACK_X, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, true, 3);
	RailRun<CMD_REMOVE_SIGNAL_TRACK>("signal-drag-remove", t(2, 2), t(11, 2), TRACK_X, false);
	/* Bridge over plain rail and over a depot. */
	RailRun<CMD_BUILD_SINGLE_RAIL>("under-bridge", t(4, 14), RAILTYPE_RAIL, TRACK_Y, false);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-under", t(5, 14), RAILTYPE_RAIL, DIAGDIR_NE);
	RailRun<CMD_BUILD_BRIDGE>("bridge-above", t(8, 14), t(2, 14), TRANSPORT_RAIL, BridgeType{0}, uint8_t{RAILTYPE_RAIL});
	/* Terraforming a single-corner track and a depot entrance edge. */
	RailRun<CMD_BUILD_SINGLE_RAIL>("right", t(22, 4), RAILTYPE_RAIL, TRACK_RIGHT, false);
	RailTerraform("terraform-corner", t(22, 4), SLOPE_W, true);
	RailRun<CMD_BUILD_TRAIN_DEPOT>("depot-edge", t(22, 8), RAILTYPE_RAIL, DIAGDIR_NE);
	RailTerraform("terraform-depot", t(22, 8), SLOPE_S, false);
	RailTerraform("terraform-track", t(9, 2), SLOPE_N, true);
	/* Station, then orthogonal, diagonal, incompatible and empty conversions. */
	RailPrint("station", Command<CMD_BUILD_RAIL_STATION>::Do(DoCommandFlag::Execute, t(2, 22), RAILTYPE_RAIL, AXIS_X, 1, 3, STAT_CLASS_DFLT, 0, NEW_STATION, false));
	RailRun<CMD_CONVERT_RAIL>("convert", t(24, 24), t(1, 1), RAILTYPE_ELECTRIC, false);
	RailRun<CMD_CONVERT_RAIL>("convert-diagonal", t(12, 12), t(2, 2), RAILTYPE_RAIL, true);
	RailRun<CMD_CONVERT_RAIL>("convert-mono", t(16, 18), t(2, 14), RAILTYPE_MONO, false);
	RailRun<CMD_CONVERT_RAIL>("convert-empty", t(24, 24), t(23, 23), RAILTYPE_RAIL, false);
	/* Clearing: auto errors, plain track and depot. */
	RailPrint("clear-auto", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Auto, t(2, 2)));
	RailPrint("clear-auto-depot", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Auto, t(20, 12)));
	RailRun<CMD_LANDSCAPE_CLEAR>("clear", t(2, 2));
	RailRun<CMD_LANDSCAPE_CLEAR>("clear-depot", t(20, 12));
	/* A second company's track: obstacles, ownership, then a merger transfer. */
	UpdateSignalsInBuffer();
	CompanyID other = RailCompany();
	{
		Backup<CompanyID> as_other(_current_company, other);
		RailRun<CMD_BUILD_RAILROAD_TRACK>("other", t(7, 7), t(7, 6), RAILTYPE_RAIL, TRACK_Y, false, false);
		RailRun<CMD_BUILD_SINGLE_SIGNAL>("other-signal", t(7, 6), TRACK_Y, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0);
		RailRun<CMD_BUILD_TRAIN_DEPOT>("other-depot", t(9, 8), RAILTYPE_RAIL, DIAGDIR_NE);
		as_other.Restore();
	}
	RailRun<CMD_BUILD_RAILROAD_TRACK>("obstacle", t(12, 6), t(2, 6), RAILTYPE_RAIL, TRACK_X, false, false);
	RailRun<CMD_BUILD_RAILROAD_TRACK>("obstacle-strict", t(12, 7), t(2, 7), RAILTYPE_RAIL, TRACK_X, false, true);
	RailRun<CMD_REMOVE_RAILROAD_TRACK>("remove-owned", t(12, 6), t(2, 6), TRACK_X);
	RailPrint("clear-other", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Auto, t(7, 6)));
	ChangeOwnershipOfCompanyItems(other, owner);
	/* A third company's track is cleared when the company is deleted. */
	UpdateSignalsInBuffer();
	CompanyID third = RailCompany();
	{
		Backup<CompanyID> as_third(_current_company, third);
		RailRun<CMD_BUILD_RAILROAD_TRACK>("third", t(18, 16), t(14, 16), RAILTYPE_RAIL, TRACK_X, false, false);
		as_third.Restore();
	}
	RailPrint("delete-third", Command<CMD_COMPANY_CTRL>::Do(DoCommandFlag::Execute, CCA_DELETE, third, CRR_MANUAL, INVALID_CLIENT_ID));
	UpdateSignalsInBuffer();
	/* Live reservations exist only while trains run with path reservation. */
	_settings_game.pf.reserve_paths = true;
	IConsoleCmdExec("schedule on-next-calendar-month rail-live.scr");
}
/** Commands on reserved, train-free track of the live network. */
static void RailLiveReservations(Owner owner)
{
	std::vector<TileIndex> reserved;
	for (TileIndex tile(0); tile < Map::Size(); ++tile) {
		if (!IsPlainRailTile(tile) || GetTileOwner(tile) != owner || GetRailReservationTrackBits(tile) == TRACK_BIT_NONE) continue;
		if (TracksOverlap(GetTrackBits(tile)) || EnsureNoVehicleOnGround(tile).Failed()) continue;
		reserved.push_back(tile);
	}
	fmt::print(stderr, "RAILCMD live-reserved {}\n", reserved.size());
	if (reserved.size() < 3) return;
	Track track[3];
	for (uint i = 0; i < 3; ++i) track[i] = FindFirstTrack(GetRailReservationTrackBits(reserved[i]));
	RailRun<CMD_BUILD_SINGLE_SIGNAL>("live-signal", reserved[0], track[0], SIGTYPE_PBS, SIG_ELECTRIC, false, true, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0);
	RailType live = GetRailType(reserved[1]);
	fmt::print(stderr, "RAILCMD live-type {}\n", live);
	RailRun<CMD_CONVERT_RAIL>("live-convert", reserved[1], reserved[1], live == RAILTYPE_MONO ? RAILTYPE_RAIL : RAILTYPE_MONO, false);
	RailRun<CMD_REMOVE_SINGLE_RAIL>("live-remove", reserved[2], track[2]);
	UpdateSignalsInBuffer();
}
static bool SignalsScenario(std::span<std::string_view> args)
{
	if (args.size() != 2) return false;
	if (args[1] == "sets") { SignalSetProbe(); return true; }
	UpdateSignalsInBuffer();
	if (args[1] == "inspect") { SignalPending("reload"); return true; }
	Train *train = nullptr;
	for (Vehicle *v : Vehicle::Iterate()) if (v->type == VEH_TRAIN && Train::From(v)->IsFrontEngine()) { train = Train::From(v); break; }
	if (train == nullptr) SignalFixtureAbort(__LINE__);
	Owner owner = train->owner;
	Backup<CompanyID> company(_current_company, owner);
	if (args[1] == "overflow") { SignalOverflowLayout(owner); company.Restore(); return true; }
	if (args[1] == "rail-commands") { RailCommandsLayout(owner); company.Restore(); return true; }
	if (args[1] == "rail-live") { RailLiveReservations(owner); company.Restore(); return true; }
	auto tracks = SignalTracks(owner);
	if (args[1] == "inspect-live") { SignalClassify(tracks, owner, "live-states"); SignalPending("live"); company.Restore(); return true; }
	if (tracks.size() < 6) SignalFixtureAbort(__LINE__);
	auto [tile, track] = tracks.front();
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	SignalPending("initial");
	AddSideToSignalBuffer(tile, DIAGDIR_NE, owner);
	AddSideToSignalBuffer(tile, DIAGDIR_NE, owner);
	AddSideToSignalBuffer(tile, DIAGDIR_SW, owner);
	SignalPending("duplicates");
	UpdateSignalsInBuffer();
	SignalPending("public-flush");
	for (uint i = 0; i < 31; ++i) AddTrackToSignalBuffer(tile, track, owner);
	SignalPending("below-threshold");
	AddTrackToSignalBuffer(tile, track, owner);
	SignalPending("forced-flush");
	SetSignalsOnBothDir(tile, track, owner);
	SignalPending("private-flush");
	SignalClassify(tracks, owner, "states");
	SaveRandomSeeds(&after);
	bool same_rng = before.random.state[0] == after.random.state[0] && before.random.state[1] == after.random.state[1] && before.interactive_random.state[0] == after.interactive_random.state[0] && before.interactive_random.state[1] == after.interactive_random.state[1];
	fmt::print(stderr, "SIGNALS rng {}\n", same_rng);
	if (!same_rng) SignalFixtureAbort(__LINE__);
	/* Real signal build/remove transactions on the existing rail. */
	SignalCost("remove", Command<CMD_REMOVE_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, tile, track));
	SignalCost("build", Command<CMD_BUILD_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, tile, track, SIGTYPE_BLOCK, SIG_ELECTRIC, false, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, 0));
	for (uint i = 0; i < 60 && i < tracks.size(); ++i) {
		auto [t, tr] = tracks[i];
		SignalType kind = static_cast<SignalType>(i % 6);
		SignalCost("convert", Command<CMD_BUILD_SINGLE_SIGNAL>::Do(DoCommandFlag::Execute, t, tr, kind, SIG_ELECTRIC, true, false, false, SIGTYPE_BLOCK, SIGTYPE_PBS_ONEWAY, 0, IsPbsSignal(kind) ? KillFirstBit(SignalOnTrack(tr)) : SignalOnTrack(tr)));
	}
	UpdateSignalsInBuffer();
	SignalPending("commands");
	SignalClassify(tracks, owner, "command-states");
	IConsoleCmdExec("schedule on-next-calendar-month signals-live.scr");
	company.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("signals_scenario", SignalsScenario);
}

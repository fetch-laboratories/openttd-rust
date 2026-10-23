/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fleet-console.cpp Identical command inputs for unchanged-reference fleet witnesses. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "group_cmd.h"
#include "autoreplace_cmd.h"
#include "autoreplace_func.h"
#include "vehicle_cmd.h"
#include "vehicle_func.h"
#include "roadveh.h"
#include "train.h"
#include "train_cmd.h"
#include "order_cmd.h"
#include "depot_base.h"
#include "station_base.h"
#include "engine_func.h"
#include "cargotype.h"
#include "rail_map.h"
#include "road_map.h"
#include "water_map.h"
#include "settings_type.h"
#include "vehiclelist.h"
#include "console_func.h"
#include "news_gui.h"
#include "table/strings.h"

[[noreturn]] static void FixtureAbort(int line) { fmt::print(stderr, "FLEET setup-failure {}\n", line); std::fflush(stderr); std::abort(); }

/* Arithmetic and default-drag inputs absent from stock traffic. The reference
 * calls its unchanged bodies; the candidate calls the production Rust owner. */
template <class T, class Info>
static void GroundPhysicsProbe(T *v, Info &info)
{
	Backup<GroundVehicleCache> cache(v->gcache);
	Backup<uint16_t> flags(v->gv_flags), speed(v->cur_speed), maximum(v->vcache.cached_max_speed);
	Backup<uint8_t> subspeed(v->subspeed), progress(v->progress), drag(info.air_drag);
	Backup<VehStates> status(v->vehstatus);
	uint cases = 0;
	for (uint8_t air : {0, 1, 255}) for (uint16_t max : {0, 10, 11, 2048}) {
		drag.Change(air); maximum.Change(max); v->PowerChanged();
		fmt::print(stderr, "FLEET ground-drag {} {} {} {}\n", v->type, air, max, v->gcache.cached_air_drag);
		++cases;
	}
	maximum.Restore(); drag.Restore();
	for (uint16_t spd : {0, 1, 65535}) for (bool brake : {false, true}) for (uint16_t slope : {0, 1, 2}) {
		speed.Change(spd); subspeed.Change(255); progress.Change(19);
		status.Change(brake ? VehStates{VehState::Stopped} : VehStates{}); flags.Change(slope);
		v->gcache.cached_weight = 8000000;
		v->gcache.cached_power = 8000000; // Kickoff narrows watts through std::min<int>.
		v->gcache.cached_max_te = UINT32_MAX;
		v->gcache.cached_axle_resistance = uint16_t(10U * v->gcache.cached_weight);
		v->gcache.cached_air_drag = 4096;
		v->gcache.cached_slope_resistance = 800000000;
		int acceleration = v->GetAcceleration();
		int distance = 0;
		if constexpr (std::is_same_v<T, Train>) distance = v->UpdateSpeed();
		fmt::print(stderr, "FLEET ground-arithmetic {} {} {} {} {} {} {} {} {}\n", v->type, spd, brake, slope, acceleration, distance, v->cur_speed, v->subspeed, v->progress);
		++cases;
	}
	fmt::print(stderr, "FLEET ground-probe {} {} {}\n", v->type, cases, static_cast<Vehicle *>(v)->GetMaxWeight());
	cache.Restore(); flags.Restore(); speed.Restore(); subspeed.Restore(); progress.Restore(); status.Restore();
}

static void Cost(const char *name, const CommandCost &cost)
{
	fmt::print(stderr, "FLEET cost {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage());
}
static void Checked(const char *name, const CommandCost &cost)
{
	Cost(name, cost);
	if (cost.Failed()) FixtureAbort(__LINE__);
}
static std::string GroupName(const Group *g)
{
#ifdef WITH_RUST
	return g->GetName();
#else
	return g->name;
#endif
}
static Money &CompanyMoney(Company *c)
{
#ifdef WITH_RUST
	return c->Finances().money;
#else
	return c->money;
#endif
}
static Vehicle *BuildFixture(VehicleType type)
{
	Vehicle *source = nullptr;
	for (Vehicle *v : Vehicle::Iterate()) if (v->type == type && v->IsPrimaryVehicle() && v->GetNumOrders() >= 2) { source = v; break; }
	if (source == nullptr) FixtureAbort(__LINE__);
	_current_company = source->owner;
	/* Declared funding input isolates actual construction/replacement policy. */
	CompanyMoney(Company::Get(_current_company)) = Money(1000000000);
	TileIndex tile = INVALID_TILE;
	if (type == VEH_AIRCRAFT) {
		for (Station *st : Station::Iterate()) if (st->owner == _current_company && st->airport.GetNumHangars() != 0) { tile = st->airport.GetHangarTile(0); break; }
	} else {
		for (Depot *depot : Depot::Iterate()) {
			if (GetTileOwner(depot->xy) != _current_company) continue;
			if ((type == VEH_TRAIN && IsRailDepotTile(depot->xy)) || (type == VEH_SHIP && IsShipDepotTile(depot->xy)) || (type == VEH_ROAD && IsRoadDepotTile(depot->xy))) { tile = depot->xy; break; }
		}
	}
	if (tile == INVALID_TILE) FixtureAbort(__LINE__);
	EngineID engine = EngineID::Invalid();
	for (const Engine *e : Engine::Iterate()) {
		if (!IsEngineBuildable(e->index, type, _current_company)) continue;
		if (type == VEH_TRAIN && RailVehInfo(e->index)->railveh_type == RAILVEH_WAGON) continue;
		if (std::get<0>(Command<CMD_BUILD_VEHICLE>::Do({}, tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID)).Succeeded()) { engine = e->index; break; }
	}
	if (engine == EngineID::Invalid()) FixtureAbort(__LINE__);
	auto [cost, id, cap, mail, cargo] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, engine, false, INVALID_CARGO, INVALID_CLIENT_ID);
	Checked("build-fixture", cost);
	Vehicle *v = Vehicle::Get(id);
	Checked("share-fixture", Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_SHARE, v->index, source->index));
	if (type == VEH_TRAIN) {
		EngineID wagon = EngineID::Invalid();
		for (const Engine *e : Engine::Iterate()) {
			if (e->type != type || RailVehInfo(e->index)->railveh_type != RAILVEH_WAGON || !IsEngineBuildable(e->index, type, _current_company)) continue;
			if (std::get<0>(Command<CMD_BUILD_VEHICLE>::Do({}, tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID)).Succeeded()) { wagon = e->index; break; }
		}
		if (wagon == EngineID::Invalid()) FixtureAbort(__LINE__);
		for (int i = 0; i < 6; ++i) {
			auto built = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, wagon, false, INVALID_CARGO, INVALID_CLIENT_ID);
			Checked("build-wagon", std::get<0>(built));
			Checked("attach-wagon", Command<CMD_MOVE_RAIL_VEHICLE>::Do(DoCommandFlag::Execute, std::get<1>(built), v->index, false));
		}
	}
	return v;
}
static uint CargoCount(const Vehicle *v)
{
	uint count = 0;
	for (const Vehicle *u = v; u != nullptr; u = u->Next()) count += u->cargo.TotalCount();
	return count;
}
static bool SameRandom(const SavedRandomSeeds &a, const SavedRandomSeeds &b)
{
	return std::equal(std::begin(a.random.state), std::end(a.random.state), std::begin(b.random.state)) && std::equal(std::begin(a.interactive_random.state), std::end(a.interactive_random.state), std::begin(b.interactive_random.state));
}
/* Unchanged public iterator bodies are the reference oracle for inputs absent
 * from traffic: full-bucket threshold, extreme unsigned arithmetic and live next
 * after a callback moves/removes the current shell. No pinned source is edited. */
static void OccupancyProbe()
{
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	if (!Vehicle::CanAllocateItem(3)) FixtureAbort(__LINE__);
	std::array<Vehicle *, 3> shells{new Vehicle(VEH_ROAD), new Vehicle(VEH_ROAD), new Vehicle(VEH_ROAD)};
	TileIndex tile(Map::Size() + 1), alias(tile.base() + (128U << Map::LogX()));
	for (auto *v : shells) { v->tile = tile; v->x_pos = 16; v->y_pos = 0; v->z_pos = 0; v->UpdatePosition(); }
	/* Same hash, distinct tile: update must not move a shell to the head. */
	shells[1]->tile = alias; shells[1]->UpdatePosition();
	auto Trace = [&](auto range) {
		uint64_t trace = 0; uint count = 0;
		for (const Vehicle *v : range) { trace = trace * 131 + v->index.base(); ++count; }
		fmt::print(stderr, "FLEET occupancy-trace {} {}\n", count, trace);
	};
	Trace(VehiclesOnTile(tile)); Trace(VehiclesOnTile(alias));
	for (auto [x, y] : {std::pair{16, 16}, {INT32_MIN, INT32_MAX}, {INT32_MAX, INT32_MIN}}) {
		for (uint distance : {1015U, 1016U, 1017U, UINT32_MAX, 0x80000000U}) Trace(VehiclesNearTileXY(x, y, distance));
	}
	auto it = VehiclesOnTile(tile).begin();
	if (*it != shells[2]) FixtureAbort(__LINE__);
	shells[2]->tile = alias; shells[2]->UpdatePosition(); // Same bucket retains LIVE next.
	++it;
	if (*it != shells[0]) FixtureAbort(__LINE__); // Alias filtering skips shell 1.
	/* Move the current head to a distinct bucket; advance now follows its NEW chain. */
	shells[2]->tile = TileIndex(tile.base() + 1); shells[2]->UpdatePosition();
	shells[1]->tile = tile; shells[1]->UpdatePosition();
	it = VehiclesOnTile(tile).begin();
	if (*it != shells[1]) FixtureAbort(__LINE__);
	shells[1]->tile = shells[2]->tile; shells[1]->UpdatePosition();
	++it;
	if (it != std::default_sentinel) FixtureAbort(__LINE__);
	auto near = VehiclesNearTileXY(16, 0, 16).begin();
	if (*near != shells[0]) FixtureAbort(__LINE__);
	shells[0]->tile = shells[2]->tile; shells[0]->UpdatePosition();
	uint live = 0;
	for (++near; near != std::default_sentinel; ++near) ++live;
	if (live != 5) FixtureAbort(__LINE__); // LIVE next plus the later bucket revisits the moved shell.
	fmt::print(stderr, "FLEET occupancy-live-near {}\n", live);
	Cost("occupancy-ground", EnsureNoVehicleOnGround(TileIndex(1)));
	Cost("occupancy-wormhole", TunnelBridgeIsFree(tile, shells[2]->tile));
	ResetVehicleHash();
	for (Vehicle *v : Vehicle::Iterate()) v->UpdatePosition();
	Trace(VehiclesOnTile(shells[2]->tile));
	for (Vehicle *v : shells) delete v;
	SaveRandomSeeds(&after);
	fmt::print(stderr, "FLEET occupancy-probe 18 live-next {}\n", SameRandom(before, after));
}

/** Running road vehicle 12 of padhattan-ridge-2000 heads to a service depot; the witness runs on 2001-01-01. */
static VehicleID _drain_vehicle = VehicleID::Invalid();
static UnitID _drain_unit = 0;
static bool FleetScenario(std::span<std::string_view> args)
{
	if (args.size() != 2) return false;
	Backup<CompanyID> company(_current_company);
	if (args[1].starts_with("transaction-mass-")) {
		VehicleType type = args[1].ends_with("train") ? VEH_TRAIN : args[1].ends_with("road") ? VEH_ROAD : args[1].ends_with("ship") ? VEH_SHIP : VEH_AIRCRAFT;
		Vehicle *v = BuildFixture(type);
		VehicleID original = v->index;
		TileIndex tile = v->tile;
		VehicleListIdentifier list(VL_SHARED_ORDERS, type, v->owner, v->index);
		Checked("mass-start-test", Command<CMD_MASS_START_STOP>::Do({}, tile, true, true, list));
		bool test_stopped = v->vehstatus.Test(VehState::Stopped);
		Checked("mass-start", Command<CMD_MASS_START_STOP>::Do(DoCommandFlag::Execute, tile, true, true, list));
		bool started = !v->vehstatus.Test(VehState::Stopped);
		Cost("mass-send-test", Command<CMD_SEND_VEHICLE_TO_DEPOT>::Do({}, VehicleID::Invalid(), {DepotCommandFlag::MassSend, DepotCommandFlag::Service}, list));
		Cost("mass-send", Command<CMD_SEND_VEHICLE_TO_DEPOT>::Do(DoCommandFlag::Execute, VehicleID::Invalid(), {DepotCommandFlag::MassSend, DepotCommandFlag::Service}, list));
		Checked("mass-stop", Command<CMD_MASS_START_STOP>::Do(DoCommandFlag::Execute, tile, false, false, list));
		bool stopped = v->vehstatus.Test(VehState::Stopped);
		v->age = TimerGameCalendar::Date(v->max_age.base() + 366);
		Checked("mass-renew-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, ALL_GROUP, v->engine_type, v->engine_type, true));
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		Cost("mass-replace-test", Command<CMD_DEPOT_MASS_AUTOREPLACE>::Do({}, tile, type));
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET mass-test {} {}\n", SameRandom(before, after), Vehicle::IsValidID(original));
		Checked("mass-replace", Command<CMD_DEPOT_MASS_AUTOREPLACE>::Do(DoCommandFlag::Execute, tile, type));
		fmt::print(stderr, "FLEET mass {} {} {} {}\n", test_stopped, started, stopped, !Vehicle::IsValidID(original));
		Cost("mass-sell-test", Command<CMD_DEPOT_SELL_ALL_VEHICLES>::Do({}, tile, type));
		Checked("mass-sell", Command<CMD_DEPOT_SELL_ALL_VEHICLES>::Do(DoCommandFlag::Execute, tile, type));
		VehicleList remaining;
		BuildDepotVehicleList(type, tile, &remaining, &remaining);
		fmt::print(stderr, "FLEET mass-empty {}\n", remaining.empty());
		Cost("mass-sell-empty", Command<CMD_DEPOT_SELL_ALL_VEHICLES>::Do({}, tile, type));
		return true;
	}
	if (args[1].starts_with("transaction-refit-")) {
		VehicleType type = args[1].ends_with("train") ? VEH_TRAIN : args[1].ends_with("road") ? VEH_ROAD : args[1].ends_with("ship") ? VEH_SHIP : VEH_AIRCRAFT;
		Vehicle *v = BuildFixture(type);
		StationID station = StationID::Invalid();
		for (const Order &o : v->Orders()) if (o.IsType(OT_GOTO_STATION)) { station = o.GetDestination().ToStationID(); break; }
		if (station == StationID::Invalid()) FixtureAbort(__LINE__);
		for (Vehicle *u = v; u != nullptr; u = u->Next()) {
			if (u->cargo_cap == 0) continue;
			uint16_t amount = std::min<uint>(u->cargo_cap + 7U, UINT16_MAX);
			if (!CargoPacket::CanAllocateItem()) FixtureAbort(__LINE__);
			u->cargo.Append(new CargoPacket(amount, 3, station, u->tile, Money(133)));
			u->refit_cap = amount;
		}
		uint before_count = CargoCount(v);
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		for (DoCommandFlags flags : {DoCommandFlags{DoCommandFlag::QueryCost}, DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
			auto [cost, capacity, mail, capacities] = Command<CMD_REFIT_VEHICLE>::Do(flags, v->index, v->cargo_type, 0xFF, false, false, 0);
			Checked("refit-loaded", cost);
			fmt::print(stderr, "FLEET refit {} {} {} {} {}\n", type, flags.base(), capacity, mail, CargoCount(v));
			for (uint cap : capacities) fmt::print(stderr, "FLEET refit-capacity {}\n", cap);
		}
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET refit-loaded {} {} {}\n", SameRandom(before, after), before_count > CargoCount(v), v->IsStoppedInDepot());
		Cost("refit-invalid-cargo", std::get<0>(Command<CMD_REFIT_VEHICLE>::Do({}, v->index, NUM_CARGO, 0, false, false, 0)));
		Checked("start-refit-fixture", Command<CMD_START_STOP_VEHICLE>::Do(DoCommandFlag::Execute, v->index, false));
		Cost("refit-running", std::get<0>(Command<CMD_REFIT_VEHICLE>::Do({}, v->index, v->cargo_type, 0, false, false, 0)));
		return true;
	}
	if (args[1].starts_with("transaction-build-")) {
		if (args[1] == "transaction-build-road") OccupancyProbe();
		VehicleType type = args[1].ends_with("road") ? VEH_ROAD : VEH_AIRCRAFT;
		Vehicle *v = BuildFixture(type);
		if (type == VEH_ROAD) GroundPhysicsProbe(RoadVehicle::From(v), Engine::Get(v->engine_type)->VehInfo<RoadVehicleInfo>());
		SavedRandomSeeds before, after;
		bool restored = true;
		/* Every buildable engine through query, test and execute, then one speculative build for another cargo. */
		for (const Engine *e : Engine::Iterate()) {
			if (!IsEngineBuildable(e->index, type, _current_company)) continue;
			for (DoCommandFlags flags : {DoCommandFlags{DoCommandFlag::QueryCost}, DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
				auto [cost, id, cap, mail, cargo] = Command<CMD_BUILD_VEHICLE>::Do(flags, v->tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
				fmt::print(stderr, "FLEET build {} {} {} {} {} {} {}\n", e->index, flags.base(), cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), cap, mail);
				for (const Vehicle *u = Vehicle::GetIfValid(id); u != nullptr; u = u->Next()) fmt::print(stderr, "FLEET part {} {} {} {} {}\n", u->subtype, u->cargo_type, u->cargo_cap, u->z_pos, u->random_bits);
			}
			for (const CargoSpec *cs : CargoSpec::Iterate()) {
				CargoType cargo = cs->Index();
				if (cargo == e->GetDefaultCargoType()) continue;
				SaveRandomSeeds(&before);
				auto [cost, id, cap, mail, cargoes] = Command<CMD_BUILD_VEHICLE>::Do({}, v->tile, e->index, false, cargo, INVALID_CLIENT_ID);
				SaveRandomSeeds(&after);
				restored = restored && SameRandom(before, after);
				fmt::print(stderr, "FLEET build-refit {} {} {} {} {} {}\n", e->index, cargo, cost.Succeeded(), cost.GetCost(), cap, mail);
				break;
			}
		}
		auto [clone_cost, clone] = Command<CMD_CLONE_VEHICLE>::Do(DoCommandFlag::Execute, v->tile, v->index, true);
		Checked("clone-shared", clone_cost);
		fmt::print(stderr, "FLEET build-restored {} {}\n", restored, Vehicle::Get(clone)->IsOrderListShared());
		return true;
	}
	if (args[1] == "transaction-clone") {
		/* A train longer than one tile cannot rejoin: execute sells the front and the unjoined wagon. */
		Vehicle *v = BuildFixture(VEH_TRAIN);
		uint before = 0, after = 0;
		for (const Vehicle *u : Vehicle::Iterate()) if (u->owner == _current_company) ++before;
		Backup<uint8_t> max_length(_settings_game.vehicle.max_train_length);
		max_length.Change(1);
		auto [cost, id] = Command<CMD_CLONE_VEHICLE>::Do(DoCommandFlag::Execute, v->tile, v->index, false);
		max_length.Restore();
		for (const Vehicle *u : Vehicle::Iterate()) if (u->owner == _current_company) ++after;
		fmt::print(stderr, "FLEET clone-join {} {} {} {}\n", cost.Succeeded(), cost.GetErrorMessage() == STR_ERROR_TRAIN_TOO_LONG, id == VehicleID::Invalid(), before == after);
		return true;
	}
	if (args[1] == "transaction-rail") {
		/* Depot consist transactions: free chains, normalisation, same-chain and cross-train moves,
		 * front promotion/demotion, refusals with restoration, dual heads, sales and ID reuse. */
		Train *v = Train::From(BuildFixture(VEH_TRAIN));
		GroundPhysicsProbe(v, Engine::Get(v->engine_type)->VehInfo<RailVehicleInfo>());
		TileIndex tile = v->tile;
		EngineID wagon = v->GetNextVehicle()->engine_type;
		EngineID single = EngineID::Invalid(), multi = EngineID::Invalid();
		for (const Engine *e : Engine::Iterate()) {
			if (e->type != VEH_TRAIN || !std::get<0>(Command<CMD_BUILD_VEHICLE>::Do({}, tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID)).Succeeded()) continue;
			RailVehicleTypes kind = RailVehInfo(e->index)->railveh_type;
			if (kind == RAILVEH_SINGLEHEAD && single == EngineID::Invalid()) single = e->index;
			if (kind == RAILVEH_MULTIHEAD && multi == EngineID::Invalid()) multi = e->index;
		}
		if (single == EngineID::Invalid() || multi == EngineID::Invalid()) FixtureAbort(__LINE__);
		auto build = [&](EngineID e, bool free) {
			auto built = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, e, free, INVALID_CARGO, INVALID_CLIENT_ID);
			Checked("rail-build", std::get<0>(built));
			return std::get<1>(built);
		};
		auto move = [&](const char *name, DoCommandFlags flags, VehicleID src, VehicleID dst, bool chain) {
			CommandCost cost = Command<CMD_MOVE_RAIL_VEHICLE>::Do(flags, src, dst, chain);
			Cost(name, cost);
			return cost;
		};
		auto sell = [&](const char *name, DoCommandFlags flags, VehicleID id, bool chain) {
			CommandCost cost = Command<CMD_SELL_VEHICLE>::Do(flags, id, chain, false, INVALID_CLIENT_ID);
			Cost(name, cost);
			return cost;
		};
		auto chain = [](VehicleID id) {
			std::string out;
			const Train *t = Train::GetIfValid(id);
			if (t == nullptr) return std::string("gone");
			for (const Train *u = t->First(); u != nullptr; u = u->Next()) out += fmt::format("{}:{}:{}:{} ", u->index, u->subtype, u->engine_type, u->cargo_cap);
			return out;
		};
		auto dump = [&](const char *name, VehicleID id) {
			const Train *t = Train::GetIfValid(id);
			if (t == nullptr) { fmt::print(stderr, "FLEET rail {} gone\n", name); return; }
			t = t->First();
			fmt::print(stderr, "FLEET rail {} head {} unit {} group {} orders {} length {} front {} money {} chain {}\n", name, t->index, t->unitnumber, t->group_id.base(), t->GetNumOrders(), t->gcache.cached_total_length, t->IsFrontEngine(), CompanyMoney(Company::Get(_current_company)), chain(id));
		};
		SavedRandomSeeds before, after;
		/* The second free wagon joins the first; a new engine with free vehicles takes the chain. */
		VehicleID w1 = build(wagon, false);
		VehicleID w2 = build(wagon, false);
		bool joined = Train::Get(w2)->First()->index == w1;
		dump("free-chain", w1);
		VehicleID a = build(single, true);
		bool normalized = Train::Get(w1)->First()->index == a;
		dump("normalized", a);
		/* Same-chain move, test then execute. */
		VehicleID second = v->GetNextVehicle()->index, last = v->Last()->index;
		std::string original = chain(v->index);
		SaveRandomSeeds(&before);
		move("same-chain-test", {}, second, last, false);
		SaveRandomSeeds(&after);
		bool test_restored = chain(v->index) == original && SameRandom(before, after);
		move("same-chain", DoCommandFlag::Execute, second, last, false);
		dump("same-chain", v->index);
		/* Train B: E1 E2 W sharing the fixture's orders. Moving E1 behind W hands the front to E2. */
		VehicleID e1 = build(single, false), e2 = build(single, false), wb = build(wagon, false);
		Checked("rail-share", Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_SHARE, e1, v->index));
		move("b-engine", DoCommandFlag::Execute, e2, e1, false);
		move("b-wagon", DoCommandFlag::Execute, wb, e2, false);
		dump("b", e1);
		move("front-trashed", DoCommandFlag::Execute, e1, wb, false);
		bool transferred = Train::Get(e2)->IsFrontEngine() && Train::Get(e2)->GetNumOrders() == v->GetNumOrders() && Train::Get(e1)->unitnumber == 0;
		dump("front-trashed", e2);
		/* The trailing engine becomes a new train in the default group. */
		move("new-front", DoCommandFlag::Execute, e1, VehicleID::Invalid(), false);
		bool new_front = Train::Get(e1)->IsFrontEngine() && Train::Get(e1)->group_id == DEFAULT_GROUP && Train::Get(e1)->unitnumber != 0;
		dump("new-front", e1);
		dump("after-new-front", e2);
		/* Length refusals restore both chains, for moves and sales. */
		Backup<uint8_t> max_length(_settings_game.vehicle.max_train_length);
		max_length.Change(1);
		std::string src_before = chain(e2), dst_before = chain(v->index);
		CommandCost too_long = move("too-long", DoCommandFlag::Execute, wb, v->index, false);
		bool refused = too_long.Failed() && too_long.GetErrorMessage() == STR_ERROR_TRAIN_TOO_LONG && chain(e2) == src_before && chain(v->index) == dst_before;
		CommandCost sell_long = sell("sell-too-long", DoCommandFlag::Execute, v->index, false);
		bool sell_refused = sell_long.Failed() && chain(v->index) == dst_before;
		max_length.Restore();
		dump("restored", v->index);
		/* A multiheaded engine, a wagon between its heads, then the front moved away takes its rear along. */
		VehicleID m = build(multi, false), wm = build(wagon, false);
		move("mh-wagon", DoCommandFlag::Execute, wm, m, false);
		dump("mh-wagon", m);
		move("mh-split", DoCommandFlag::Execute, m, VehicleID::Invalid(), false);
		const Train *mt = Train::Get(m);
		bool paired = mt->Next() != nullptr && mt->Next() == mt->other_multiheaded_part && Train::Get(wm)->IsFreeWagon();
		dump("mh", m);
		dump("mh-free", wm);
		/* Sales: a free wagon, a chain from a wagon, a front engine handing over its orders. */
		sell("sell-wagon-test", {}, wm, false);
		sell("sell-wagon", DoCommandFlag::Execute, wm, false);
		sell("sell-chain", DoCommandFlag::Execute, v->GetNextVehicle()->GetNextVehicle()->index, true);
		dump("sold-chain", v->index);
		VehicleID e3 = build(single, false);
		move("c-engine", DoCommandFlag::Execute, e3, e2, false);
		dump("c", e2);
		sell("sell-front", DoCommandFlag::Execute, e2, false);
		bool sold_transfer = Train::IsValidID(e3) && Train::Get(e3)->IsFrontEngine() && Train::Get(e3)->GetNumOrders() == v->GetNumOrders();
		dump("sold-front", e3);
		/* Freed IDs are reused by the next builds. */
		VehicleID reused = build(wagon, false);
		fmt::print(stderr, "FLEET rail reused {} {}\n", reused, reused == e2 || reused == wm);
		dump("reused", reused);
		fmt::print(stderr, "FLEET rail-checks {} {} {} {} {} {} {} {} {}\n", joined, normalized, test_restored, transferred, new_front, refused, sell_refused, paired, sold_transfer);
		return true;
	}
	if (args[1] == "transaction-meta") {
		for (VehicleType type : {VEH_TRAIN, VEH_ROAD, VEH_SHIP, VEH_AIRCRAFT}) {
			Vehicle *v = nullptr;
			for (Vehicle *u : Vehicle::Iterate()) if (u->type == type && u->IsPrimaryVehicle()) { v = u; break; }
			if (v == nullptr) FixtureAbort(__LINE__);
			company.Change(v->owner);
			std::string name = fmt::format("Transaction snowman \xE2\x98\x83 {}", type);
			Checked("name-test", Command<CMD_RENAME_VEHICLE>::Do({}, v->index, name));
			Checked("name-exec", Command<CMD_RENAME_VEHICLE>::Do(DoCommandFlag::Execute, v->index, name));
			Cost("name-same", Command<CMD_RENAME_VEHICLE>::Do({}, v->index, name));
			Checked("name-autoreplace", Command<CMD_RENAME_VEHICLE>::Do(DoCommandFlag::AutoReplace, v->index, name));
			Cost("name-long", Command<CMD_RENAME_VEHICLE>::Do({}, v->index, std::string(1000, 'x')));
			Cost("interval-invalid", Command<CMD_CHANGE_SERVICE_INT>::Do({}, v->index, 0, true, true));
			Checked("interval-test", Command<CMD_CHANGE_SERVICE_INT>::Do({}, v->index, 50, true, true));
			Checked("interval-exec", Command<CMD_CHANGE_SERVICE_INT>::Do(DoCommandFlag::Execute, v->index, 50, true, true));
			fmt::print(stderr, "FLEET transaction-meta {} {} {} {} {}\n", type, v->name == name, v->GetServiceInterval(), v->ServiceIntervalIsCustom(), v->ServiceIntervalIsPercent());
			Checked("name-reset", Command<CMD_RENAME_VEHICLE>::Do(DoCommandFlag::Execute, v->index, std::string{}));
			Checked("interval-default", Command<CMD_CHANGE_SERVICE_INT>::Do(DoCommandFlag::Execute, v->index, 0, false, false));
		}
		return true;
	}
	if (args[1] == "drain" || args[1] == "drain-cash") {
		/* Typed inputs on an ordinary save: a running vehicle past renewal age, autorenew on, and for the cash case only the reserve. */
		Vehicle *v = Vehicle::Get(VehicleID(12));
		if (v->type != VEH_ROAD || v->vehstatus.Test(VehState::Stopped) || !v->current_order.IsType(OT_GOTO_DEPOT) || v->owner != _local_company) FixtureAbort(__LINE__);
		Company *c = Company::Get(v->owner);
		c->settings.engine_renew = true;
		v->age = TimerGameCalendar::Date(v->max_age.base() + 366);
		if (args[1] == "drain-cash") CompanyMoney(c) = Money(c->settings.engine_renew_money);
		_drain_vehicle = v->index;
		_drain_unit = v->unitnumber;
		fmt::print(stderr, "FLEET drain input {} {} {}\n", v->index, v->engine_type, CompanyMoney(c));
		IConsoleCmdExec("schedule on-next-calendar-month fleet-drain.scr");
		return true;
	}
	if (args[1] == "drain-inspect") {
		const Vehicle *old = Vehicle::GetIfValid(_drain_vehicle);
		fmt::print(stderr, "FLEET drain old {} {} {} money {}\n", old != nullptr, old != nullptr && old->vehstatus.Test(VehState::Stopped), old != nullptr ? old->age.base() : -1, CompanyMoney(Company::Get(_local_company)));
		for (const Vehicle *u : Vehicle::Iterate()) {
			if (u->type != VEH_ROAD || !u->IsPrimaryVehicle() || u->unitnumber != _drain_unit || u->index == _drain_vehicle) continue;
			fmt::print(stderr, "FLEET drain new {} {} {} {} {}\n", u->index, u->engine_type, u->age.base(), u->vehstatus.Test(VehState::Stopped), u->IsInDepot());
		}
		for (const NewsItem &ni : GetNews()) {
			if (ni.advice_type != AdviceType::AutorenewFailed) continue;
			const VehicleID *ref = std::get_if<VehicleID>(&ni.ref1);
			std::string text;
			for (char ch : ni.headline.GetDecodedString()) if (ch == '\n' || (ch >= ' ' && ch <= '~')) text += ch == '\n' ? ' ' : ch;
			fmt::print(stderr, "FLEET drain news {} {}\n", ref != nullptr ? ref->base() : -1, text);
		}
		return true;
	}
	if (args[1] == "inspect") {
		for (Group *g : Group::Iterate()) if (GroupName(g) == "Fleet snowman \xE2\x98\x83") {
			company.Change(g->owner);
			fmt::print(stderr, "FLEET reloaded {} {} {}\n", GroupIsInGroup(g->index, g->parent), GetGroupNumVehicle(g->owner, g->parent, g->vehicle_type), g->livery.colour1 == COLOUR_RED);
		}
		company.Restore();
		return true;
	}
	Vehicle *v = args[1] == "ship" ? BuildFixture(VEH_SHIP) : args[1] == "aircraft" ? BuildFixture(VEH_AIRCRAFT) : args[1].starts_with("train") ? BuildFixture(VEH_TRAIN) : Vehicle::Get(VehicleID(126));
	if (!v->IsInDepot()) FixtureAbort(__LINE__);
	company.Change(v->owner);
	if (args[1] == "list") {
		/* "Manage list -> Create group" (vehicle_gui.cpp) posts NEW_GROUP, VehicleID::Invalid() and the window's list. */
		VehicleListIdentifier vli(VL_STANDARD, v->type, v->owner, v->owner);
		VehicleList list;
		if (!GenerateVehicleSortList(&list, vli) || list.size() < 2) FixtureAbort(__LINE__);
		size_t groups = Group::GetNumItems();
		auto [test_cost, test_group] = Command<CMD_ADD_VEHICLE_GROUP>::Do({}, NEW_GROUP, VehicleID::Invalid(), false, vli);
		Cost("list-test", test_cost);
		/* A network veh_id of 0xFFFFFFFF is not VehicleID::Invalid(): no list, no vehicle. */
		auto [wide_cost, wide_group] = Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, NEW_GROUP, VehicleID(0xFFFFFFFF), false, vli);
		Cost("list-wide-id", wide_cost);
		VehicleListIdentifier invalid(VLT_END, v->type, v->owner, v->owner);
		auto [invalid_cost, invalid_group] = Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, NEW_GROUP, VehicleID::Invalid(), false, invalid);
		Cost("list-invalid", invalid_cost);
		fmt::print(stderr, "FLEET list-rejected {} {} {} {}\n", test_group == GroupID::Invalid(), wide_group == GroupID::Invalid(), invalid_group == GroupID::Invalid(), Group::GetNumItems() == groups);
		auto [cost, group] = Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, NEW_GROUP, VehicleID::Invalid(), false, vli);
		Checked("list-create", cost);
		bool members = Group::IsValidID(group);
		for (const Vehicle *u : list) members &= u->group_id == group;
		fmt::print(stderr, "FLEET list-group {} {} {}\n", members, GetGroupNumVehicle(v->owner, group, v->type) == list.size(), Group::GetNumItems() == groups + 1);
	} else if (args[1] == "groups" || args[1] == "groups-store") {
		auto [parent_cost, parent] = Command<CMD_CREATE_GROUP>::Do(DoCommandFlag::Execute, v->type, GroupID::Invalid());
		Checked("parent", parent_cost);
		auto [child_cost, child] = Command<CMD_CREATE_GROUP>::Do(DoCommandFlag::Execute, v->type, parent);
		Checked("child", child_cost);
		Checked("rename", Command<CMD_ALTER_GROUP>::Do(DoCommandFlag::Execute, AlterGroupMode::Rename, child, GroupID::Invalid(), "Fleet snowman \xE2\x98\x83"));
		Checked("membership", std::get<0>(Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, child, v->index, false, VehicleListIdentifier{})));
		Checked("all-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, ALL_GROUP, v->engine_type, v->engine_type, false));
		bool old = false;
		EngineID replacement = EngineReplacementForCompany(Company::Get(v->owner), v->engine_type, child, &old);
		fmt::print(stderr, "FLEET rule all {} {}\n", replacement == v->engine_type, old);
		Checked("protection", Command<CMD_SET_GROUP_FLAG>::Do(DoCommandFlag::Execute, parent, GroupFlag::ReplaceProtection, true, true));
		replacement = EngineReplacementForCompany(Company::Get(v->owner), v->engine_type, child, &old);
		fmt::print(stderr, "FLEET rule protected {} {}\n", replacement == EngineID::Invalid(), old);
		Checked("parent-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, parent, v->engine_type, v->engine_type, false));
		replacement = EngineReplacementForCompany(Company::Get(v->owner), v->engine_type, child, &old);
		fmt::print(stderr, "FLEET rule inherited {} {}\n", replacement == v->engine_type, old);
		Cost("recursive-parent", Command<CMD_ALTER_GROUP>::Do(DoCommandFlag::Execute, AlterGroupMode::SetParent, parent, child, ""));
		Checked("livery", Command<CMD_SET_GROUP_LIVERY>::Do(DoCommandFlag::Execute, parent, true, COLOUR_RED));
		fmt::print(stderr, "FLEET groups {} {} {} {}\n", GroupIsInGroup(child, parent), GetGroupNumVehicle(v->owner, parent, v->type), GetGroupNumEngines(v->owner, parent, v->engine_type), Group::Get(child)->livery.colour1 == COLOUR_RED);
		if (args[1] == "groups") {
		Checked("delete", Command<CMD_DELETE_GROUP>::Do(DoCommandFlag::Execute, parent));
		fmt::print(stderr, "FLEET deleted {} {} {}\n", !Group::IsValidID(parent), !Group::IsValidID(child), v->group_id == DEFAULT_GROUP);
		}
	} else if (args[1] == "train-wagons") {
		/* Typed inputs: a single-headed old train holding the fixture's wagons plus one,
		 * so it fills four tiles exactly, and a free chain whose second unit is an old engine. */
		TileIndex tile = v->tile;
		EngineID wagon = v->GetNextVehicle()->engine_type;
		auto build = [&](EngineID e) {
			auto built = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, e, false, INVALID_CARGO, INVALID_CLIENT_ID);
			Checked("build-extra", std::get<0>(built));
			return std::get<1>(built);
		};
		auto buildable = [&](const Engine *e, RailVehicleTypes kind) {
			return e->type == VEH_TRAIN && RailVehInfo(e->index)->railveh_type == kind && std::get<0>(Command<CMD_BUILD_VEHICLE>::Do({}, tile, e->index, false, INVALID_CARGO, INVALID_CLIENT_ID)).Succeeded();
		};
		EngineID engine = EngineID::Invalid();
		EngineID multihead = EngineID::Invalid();
		for (const Engine *e : Engine::Iterate()) {
			if (!buildable(e, RAILVEH_SINGLEHEAD)) continue;
			for (const Engine *m : Engine::Iterate()) if (buildable(m, RAILVEH_MULTIHEAD) && CheckAutoreplaceValidity(e->index, m->index, _current_company)) { multihead = m->index; break; }
			if (multihead != EngineID::Invalid()) { engine = e->index; break; }
		}
		if (engine == EngineID::Invalid()) FixtureAbort(__LINE__);
		VehicleID train = build(engine);
		Checked("share-extra", Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_SHARE, train, v->index));
		std::vector<VehicleID> moved;
		for (const Vehicle *u = v; u != nullptr; u = u->Next()) if (Train::From(u)->IsWagon()) moved.push_back(u->index);
		for (VehicleID id : moved) Checked("move-wagon", Command<CMD_MOVE_RAIL_VEHICLE>::Do(DoCommandFlag::Execute, id, train, false));
		Checked("attach-extra", Command<CMD_MOVE_RAIL_VEHICLE>::Do(DoCommandFlag::Execute, build(wagon), train, false));
		VehicleID free_head = build(wagon);
		VehicleID free_engine = build(engine);
		Checked("free-chain", Command<CMD_MOVE_RAIL_VEHICLE>::Do(DoCommandFlag::Execute, free_engine, free_head, false));
		for (VehicleID id : {train, free_engine}) Vehicle::Get(id)->age = TimerGameCalendar::Date(Vehicle::Get(id)->max_age.base() + 366);
		uint16_t old_length = Train::Get(train)->gcache.cached_total_length;
		fmt::print(stderr, "FLEET wagons-input {} {}\n", moved.size() + 1, old_length);
		auto [group_cost, group] = Command<CMD_CREATE_GROUP>::Do(DoCommandFlag::Execute, VEH_TRAIN, GroupID::Invalid());
		Checked("wagon-group", group_cost);
		Checked("wagon-membership", std::get<0>(Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, group, train, false, VehicleListIdentifier{})));
		Checked("wagon-removal", Command<CMD_SET_GROUP_FLAG>::Do(DoCommandFlag::Execute, group, GroupFlag::ReplaceWagonRemoval, true, false));
		Checked("renew-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, ALL_GROUP, engine, engine, true));
		Checked("wagon-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, ALL_GROUP, wagon, wagon, false));
		Checked("engine-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, group, engine, multihead, false));
		/* ReplaceFreeUnit: the engine is not a front engine, so it is replaced on its own. */
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		Cost("free-test", Command<CMD_AUTOREPLACE_VEHICLE>::Do({}, free_engine));
		SaveRandomSeeds(&after);
		bool free_rng = SameRandom(before, after);
		Checked("free-execute", Command<CMD_AUTOREPLACE_VEHICLE>::Do(DoCommandFlag::Execute, free_engine));
		const Train *head = Train::Get(free_head);
		fmt::print(stderr, "FLEET free-replaced {} {} {} {}\n", free_rng, !Vehicle::IsValidID(free_engine), head->IsFreeWagon(), head->GetNextUnit() != nullptr && head->GetNextUnit()->engine_type == engine && head->GetNextUnit()->age == 0);
		/* ReplaceChain: a different, longer engine; replaced wagons that no longer fit are sold. */
		VehicleID original = train;
		uint vehicles = 0;
		for (const Vehicle *u : Vehicle::Iterate()) if (u->type == VEH_TRAIN && u->owner == _current_company) ++vehicles;
		SaveRandomSeeds(&before);
		Cost("wagons-test", Command<CMD_AUTOREPLACE_VEHICLE>::Do({}, original));
		SaveRandomSeeds(&after);
		bool chain_rng = SameRandom(before, after);
		Checked("wagons-execute", Command<CMD_AUTOREPLACE_VEHICLE>::Do(DoCommandFlag::Execute, original));
		const Train *replaced = nullptr;
		for (const Train *t : Train::Iterate()) if (t->IsFrontEngine() && t->group_id == group) replaced = t;
		if (replaced == nullptr) FixtureAbort(__LINE__);
		uint wagons = 0;
		for (const Train *u = replaced; u != nullptr; u = u->GetNextUnit()) if (u->IsWagon()) ++wagons;
		uint now = 0;
		for (const Vehicle *u : Vehicle::Iterate()) if (u->type == VEH_TRAIN && u->owner == _current_company) ++now;
		fmt::print(stderr, "FLEET wagons-replaced {} {} {} {} {} {}\n", chain_rng, !Vehicle::IsValidID(original), replaced->engine_type == multihead, wagons, replaced->gcache.cached_total_length <= old_length, static_cast<int>(now) - static_cast<int>(vehicles));
	} else {
		Company *c = Company::Get(v->owner);
		/* Typed inputs: old-enough actual command-built vehicles, carrying cargo. */
		v->age = TimerGameCalendar::Date(v->max_age.base() + 366);
		uint injected = 0;
		StationID first = StationID::Invalid();
		for (const Order &order : v->Orders()) if (order.IsType(OT_GOTO_STATION)) { first = order.GetDestination().ToStationID(); break; }
		if (first == StationID::Invalid()) FixtureAbort(__LINE__);
		for (Vehicle *u = v; u != nullptr; u = u->Next()) {
			if (u->cargo_cap == 0) continue;
			uint16_t amount = std::min<uint16_t>(u->cargo_cap, 7);
			if (!CargoPacket::CanAllocateItem()) FixtureAbort(__LINE__);
			u->cargo.Append(new CargoPacket(amount, 3, first, u->tile, Money(133)));
			injected += amount;
		}
		if (injected == 0) FixtureAbort(__LINE__);
		Checked("renew-rule", Command<CMD_SET_AUTOREPLACE>::Do(DoCommandFlag::Execute, ALL_GROUP, v->engine_type, v->engine_type, true));
		VehicleID original = v->index;
		EngineID engine = v->engine_type;
		uint orders = v->GetNumOrders();
		uint old_count = CargoCount(v);
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		Cost("test", Command<CMD_AUTOREPLACE_VEHICLE>::Do({}, original));
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET test-rng {} original {}\n", SameRandom(before, after), Vehicle::IsValidID(original));
		fmt::print(stderr, "FLEET test-cargo {}\n", CargoCount(v) == old_count);
		if (args[1] == "cash" || args[1] == "train-rollback") {
			Backup<Money> cash(CompanyMoney(c));
			Backup<uint8_t> max_length(_settings_game.vehicle.max_train_length);
			if (args[1] == "cash") cash.Change(Money(0));
			else max_length.Change(1);
			SaveRandomSeeds(&before);
			Cost("rollback", Command<CMD_AUTOREPLACE_VEHICLE>::Do(DoCommandFlag::Execute, original));
			SaveRandomSeeds(&after);
			fmt::print(stderr, "FLEET rollback {} {} {} {}\n", SameRandom(before, after), Vehicle::IsValidID(original), CargoCount(v) == old_count, v->GetNumOrders() == orders);
			max_length.Restore();
			cash.Restore();
		} else {
			Checked("execute", Command<CMD_AUTOREPLACE_VEHICLE>::Do(DoCommandFlag::Execute, original));
			fmt::print(stderr, "FLEET replaced {}\n", !Vehicle::IsValidID(original));
			bool transferred = false;
			for (Vehicle *u : Vehicle::Iterate()) if (u->IsPrimaryVehicle() && u->engine_type == engine && u->age == 0 && u->GetNumOrders() == orders && CargoCount(u) == old_count) transferred = true;
			fmt::print(stderr, "FLEET transferred {} {}\n", transferred, old_count > 0);
		}
	}
	company.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("fleet_scenario", FleetScenario);
}

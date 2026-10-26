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
#include "newgrf.h"
#include "newgrf_config.h"
#include "newgrf_engine.h"
#include "newgrf_text.h"
#include "table/strings.h"
#include "newgrf_engine.h"
#include "newgrf.h"
#include "aircraft.h"
#include "ship.h"

#include "effectvehicle_base.h"
#include "effectvehicle_func.h"
#include "bridge_map.h"
#include "tunnel_map.h"
#include "newgrf_sound.h"

[[noreturn]] static void FixtureAbort(int line) { fmt::print(stderr, "FLEET setup-failure {}\n", line); std::fflush(stderr); std::abort(); }
static bool SameRandom(const SavedRandomSeeds &a, const SavedRandomSeeds &b);

/* Observe the real resolver at its Action2 entry, then run a normal deterministic
 * group (including register 100). No callback service or command body is replaced. */
struct AttachmentObserver : SpriteGroup {
	AttachmentObserver() : SpriteGroup() {}
	const SpriteGroup *program = nullptr;
	mutable uint calls = 0;
	mutable bool valid = true;
	ResolverResult Resolve(ResolverObject &object) const override
	{
		if (object.callback != CBID_TRAIN_ALLOW_WAGON_ATTACH) return {};
		const auto &resolver = static_cast<const VehicleResolverObject &>(object);
		const Train *self = Train::From(resolver.self_scope.v), *head = Train::From(resolver.parent_scope.v);
		++this->calls;
		this->valid &= self->Next() == nullptr && self->Previous() == nullptr && self->First() == self;
		this->valid &= self->gcache.first_engine == EngineID::Invalid() && self->grf_cache.cache_valid == 0 && head->grf_cache.cache_valid == 0;
		bool available = true;
		uint length = 0;
		for (const Train *part = head; part != nullptr; part = part->Next()) ++length;
		/* var 40 caches the current, temporarily shortened chain, not the final consist. */
		this->valid &= resolver.self_scope.GetVariable(0x40, 0, available) == 0;
		this->valid &= resolver.parent_scope.GetVariable(0x40, 0, available) == (length - 1) * 0x10100;
		this->valid &= available;
		return SpriteGroup::Resolve(this->program, object, false);
	}
};

static void AttachmentCallbacks(Train *head)
{
	Engine *engine = Engine::Get(head->engine_type);
	Backup<CargoGRFFileProps> props(engine->grf_prop);
	GRFFile grf;
	grf.grfid = 0x156CA110;
	auto config = std::make_unique<GRFConfig>("attachment-fixture.grf");
	config->ident.grfid = grf.grfid;
	_grfconfig.push_back(std::move(config));
	engine->grf_prop.grffile = &grf;
	StringID text = AddGRFString(grf.grfid, GRFSTR_MISC_GRF_TEXT, 0x7F, true, false, "attachment text", STR_UNDEFINED);
	StringID regtext = AddGRFString(grf.grfid, GRFStringID(0xD042), 0x7F, true, false, "attachment register", STR_UNDEFINED);
	/* Pool-allocated Action2 inputs stay alive until the engine's original groups
	 * are restored. The observer delegates to the unchanged Action2 interpreter. */
	if (!SpriteGroup::CanAllocateItem(3)) FixtureAbort(__LINE__);
	auto *result = new CallbackResultSpriteGroup(CALLBACK_FAILED);
	auto *program = new DeterministicSpriteGroup();
	program->size = DSG_SIZE_DWORD;
	program->adjusts = {
		{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0xD042},
		{.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100},
	};
	program->default_result.group = result;
	auto *observer = new AttachmentObserver();
	observer->program = program;
	engine->grf_prop.spritegroups.clear();
	engine->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, observer);
	struct Case { uint8_t version; uint16_t result; StringID error; };
	const Case cases[] = {
		{7, 0, text}, {7, 0xFD, STR_ERROR_INCOMPATIBLE_RAIL_TYPES},
		{7, 0xFE, STR_NULL}, {7, 0xFF, STR_NULL}, {7, 0x100, STR_NULL},
		{8, 0, text}, {8, 0x400, STR_NULL}, {8, 0x401, STR_NULL},
		{8, 0x40F, regtext}, {8, 0x402, STR_ERROR_INCOMPATIBLE_RAIL_TYPES},
		{8, 0x403, STR_ERROR_INCOMPATIBLE_RAIL_TYPES}, {8, CALLBACK_FAILED, STR_NULL},
	};
	for (const auto &c : cases) {
		grf.grf_version = c.version; result->result = c.result;
		for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{DoCommandFlag::Execute}}) {
			Train *wagon = head->GetNextVehicle();
			EngineID first = wagon->gcache.first_engine;
			observer->calls = 0; observer->valid = true;
			SavedRandomSeeds before, after; SaveRandomSeeds(&before);
			CommandCost cost = Command<CMD_MOVE_RAIL_VEHICLE>::Do(flags, wagon->index, head->Last()->index, false);
			SaveRandomSeeds(&after);
			bool restored = wagon->First() == head && wagon->gcache.first_engine == first;
			bool outcome = c.error == STR_NULL ? cost.Succeeded() : cost.Failed() && cost.GetErrorMessage() == c.error;
			fmt::print(stderr, "FLEET attachment {} {} {} {} {} {} {} {}\n", c.version, c.result, flags.Test(DoCommandFlag::Execute), outcome, observer->calls, observer->valid, restored, SameRandom(before, after));
		}
	}
	props.Restore();
	head->InvalidateNewGRFCacheOfChain();
	delete observer; delete program; delete result;
	_grfconfig.pop_back();
}

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

/* Public-variable results over live stock shells, compared against the reference. */
static void ResolverVariables(Vehicle *v)
{
	GRFFile grf;
	grf.grf_version = 8;
	for (uint i = 0; i < NUM_CARGO; ++i) grf.cargo_map[i] = NUM_CARGO - 1 - i;
	VehicleResolverObject ro(v->engine_type, v, VehicleResolverObject::WO_NONE);
	ro.grffile = &grf;
	ro.SetRegister(0x10E, 0); ro.SetRegister(0x10F, 1);
	uint64_t digest = 0;
	uint count = 0;
	auto query = [&](uint8_t variable, uint32_t parameter) {
		bool available = true;
		uint32_t value = ro.self_scope.GetVariable(variable, parameter, available);
		digest = digest * 31337 + value + available;
		++count;
	};
	for (uint8_t variable : {0x25, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x7A}) query(variable, 0);
	for (uint variable = 0x80; variable <= 0xFF; ++variable) query(variable, 0);
	for (uint32_t parameter : {0x5FU, 0x61U, 0x84U, 0xFFFFU}) query(0x61, parameter);
	VehicleResolverObject purchase(v->engine_type, nullptr, VehicleResolverObject::WO_NONE);
	purchase.grffile = &grf;
	for (uint8_t variable : {0x43, 0x46, 0x47, 0x48, 0x49, 0x4B, 0x7A, 0x92, 0x93, 0xC4, 0xC6, 0xC7, 0xDA, 0xF2, 0xFF}) {
		bool available = true;
		digest = digest * 31337 + purchase.self_scope.GetVariable(variable, 0, available) + available;
		++count;
	}
	fmt::print(stderr, "FLEET resolver-variables {} {} {} {}\n", v->type, v->subtype, count, digest);
	v->InvalidateNewGRFCacheOfChain();
}

/* Actual airport movement flags at every position, all byte states and both
 * loading/depot order branches, through the original vehicle variable API. */
static void ResolverAircraft(Aircraft *v)
{
	const Station *st = GetTargetAirportIfValid(v);
	if (st == nullptr) FixtureAbort(__LINE__);
	const AirportFTAClass *airport = st->airport.GetFTA();
	Backup<uint8_t> state(v->state), pos(v->pos);
	Backup<Order> order(v->current_order);
	VehicleResolverObject ro(v->engine_type, v, VehicleResolverObject::WO_NONE);
	uint64_t digest = 0;
	uint count = 0;
	for (bool loading : {false, true}) {
		if (loading) v->current_order.MakeLoading(false); else v->current_order.MakeGoToDepot(DepotID(0), OrderDepotTypeFlags{});
		for (uint p = 0; p < airport->nofelements; ++p) for (uint s = 0; s < 256; ++s) {
			state.Change(s); pos.Change(p);
			bool available = true;
			digest = digest * 31337 + ro.self_scope.GetVariable(0xE2, 0, available);
			digest = digest * 31337 + ro.self_scope.GetVariable(0xE6, 0, available);
			if (!available) FixtureAbort(__LINE__);
			++count;
		}
	}
	order.Restore(); pos.Restore(); state.Restore();
	fmt::print(stderr, "FLEET resolver-aircraft {} {}\n", count, digest);
}

/* Empty real-group vectors and capacity boundaries are absent from the stock
 * traffic. Exercise the real public callback/property paths, restoring the
 * canonical cargo list, capacity, order and engine root before returning. */
static void ResolverRealBoundaries(Train *v, Engine *engine)
{
	if (v->cargo.TotalCount() != 0 || !SpriteGroup::CanAllocateItem(7)) FixtureAbort(__LINE__);
	Backup<decltype(engine->grf_prop.spritegroups)> sprites(engine->grf_prop.spritegroups);
	Backup<uint16_t> capacity(v->cargo_cap);
	Backup<Order> order(v->First()->current_order);
	const uint16_t original_capacity = v->cargo_cap;
	const Order original_order = v->First()->current_order;
	Backup<Randomizer> interactive(_interactive_random);
	_interactive_random.SetSeed(12345);
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	std::array<const SpriteGroup *, 3> loaded, loading;
	for (uint i = 0; i < 3; ++i) {
		loaded[i] = new CallbackResultSpriteGroup(101 + i);
		loading[i] = new CallbackResultSpriteGroup(211 + i);
	}
	auto *real = new RealSpriteGroup();
	engine->grf_prop.spritegroups.clear();
	engine->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, real);
	for (uint mask = 0; mask < 4; ++mask) {
		real->loaded.clear(); real->loading.clear();
		if (mask & 1) real->loaded.assign(loaded.begin(), loaded.end());
		if (mask & 2) real->loading.assign(loading.begin(), loading.end());
		fmt::print(stderr, "FLEET resolver-real-purchase {} {}\n", mask, GetEngineProperty(v->engine_type, PROP_TRAIN_SPEED, 17));
		/* 0: moving, 1: loading, 2: both disabled, 3/4: only one disabled. */
		for (uint state = 0; state < 5; ++state) {
			if (state == 0) v->First()->current_order.MakeDummy(); else v->First()->current_order.MakeLoading(false);
			if (state == 2 || state == 3) v->First()->current_order.SetLoadType(OrderLoadType::NoLoad);
			if (state == 2 || state == 4) v->First()->current_order.SetUnloadType(OrderUnloadType::NoUnload);
			fmt::print(stderr, "FLEET resolver-real-order {} {} {} {}\n", mask, state, to_underlying(v->First()->current_order.GetLoadType()), to_underlying(v->First()->current_order.GetUnloadType()));
			for (auto [cap, count] : {std::pair{0, 0}, {0, 1}, {8, 0}, {8, 1}, {8, 2}, {8, 3}, {8, 5}, {8, 6}, {8, 7}, {8, 8}, {8, 9}}) {
				capacity.Change(cap);
				if (count != 0) {
					if (!CargoPacket::CanAllocateItem()) FixtureAbort(__LINE__);
					v->cargo.Append(new CargoPacket(count, 3, StationID::Invalid(), v->tile, Money(133)));
				}
				fmt::print(stderr, "FLEET resolver-real-selection {} {} {} {} {} {}\n", mask, state, cap, count, v->cargo.StoredCount(), GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v));
				v->cargo.Truncate();
			}
		}
	}
	capacity.Restore(); order.Restore(); sprites.Revert();
	bool roots_restored = sprites.Verify();
	sprites.Trash();
	delete real;
	for (auto *group : loaded) delete group;
	for (auto *group : loading) delete group;
	SaveRandomSeeds(&after);
	fmt::print(stderr, "FLEET resolver-real-restored {} {} {} {} {} {}\n", roots_restored && v->cargo.TotalCount() == 0 && v->cargo_cap == original_capacity && v->First()->current_order.Equals(original_order), SameRandom(before, after), after.random.state[0], after.random.state[1], after.interactive_random.state[0], after.interactive_random.state[1]);
	interactive.Restore();
}
/* Override registration order is observable: a default entry can win before a
 * cargo-specific one. Use actual engine registrations and the uncached callback
 * path, without editing either resolver's original selection body. */
static void ResolverOverrideOrder(Train *wagon)
{
	Engine *engine = Engine::Get(wagon->engine_type);
	if (wagon->gcache.first_engine == EngineID::Invalid() || !SpriteGroup::CanAllocateItem(3)) FixtureAbort(__LINE__);
	Backup<decltype(engine->overrides)> overrides(engine->overrides);
	Backup<decltype(engine->grf_prop.spritegroups)> sprites(engine->grf_prop.spritegroups);
	Backup<CargoType> cargo(wagon->cargo_type);
	const CargoType original_cargo = wagon->cargo_type;
	const CargoType other_cargo = (original_cargo + 1) % NUM_CARGO;
	Backup<Randomizer> interactive(_interactive_random);
	_interactive_random.SetSeed(12345);
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	auto *first = new CallbackResultSpriteGroup(311);
	auto *second = new CallbackResultSpriteGroup(419);
	auto *ordinary = new CallbackResultSpriteGroup(523);
	engine->grf_prop.spritegroups.clear();
	engine->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, ordinary);
	std::array<EngineID, 2> matching{EngineID::Invalid(), wagon->gcache.first_engine};
	std::array<EngineID, 1> rejected{EngineID::Invalid()};
	for (uint test = 0; test < 6; ++test) {
		engine->overrides.clear();
		/* Neither a wrong engine nor a wrong cargo may steal the match. */
		SetWagonOverrideSprites(wagon->engine_type, CargoGRFFileProps::SG_DEFAULT, second, rejected);
		SetWagonOverrideSprites(wagon->engine_type, (original_cargo + 2) % NUM_CARGO, second, matching);
		if (test == 0) {
			SetWagonOverrideSprites(wagon->engine_type, CargoGRFFileProps::SG_DEFAULT, first, matching);
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, second, matching);
		} else if (test == 1) {
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, second, matching);
			SetWagonOverrideSprites(wagon->engine_type, CargoGRFFileProps::SG_DEFAULT, first, matching);
		} else if (test == 2) {
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, first, matching);
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, second, matching);
		} else if (test == 3) {
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, nullptr, matching);
			SetWagonOverrideSprites(wagon->engine_type, CargoGRFFileProps::SG_DEFAULT, second, matching);
		} else if (test == 4) {
			SetWagonOverrideSprites(wagon->engine_type, original_cargo, first, matching);
		}
		for (uint input = 0; input < 2; ++input) {
			cargo.Change(input == 0 ? original_cargo : other_cargo);
			fmt::print(stderr, "FLEET resolver-override-order {} {} {}\n", test, input, GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, wagon->engine_type, wagon));
		}
	}
	cargo.Restore(); sprites.Restore(); overrides.Restore();
	delete ordinary; delete second; delete first;
	SaveRandomSeeds(&after);
	fmt::print(stderr, "FLEET resolver-override-restored {} {} {} {} {} {}\n", wagon->cargo_type == original_cargo, SameRandom(before, after), after.random.state[0], after.random.state[1], after.interactive_random.state[0], after.interactive_random.state[1]);
	interactive.Restore();
}

/* Observe callback-36 sign extension at its 15-bit boundary and failure-only
 * fallback through both live-vehicle and purchase property entry points. */
static void ResolverPropertyBoundaries(Train *v, Engine *engine)
{
	if (!SpriteGroup::CanAllocateItem()) FixtureAbort(__LINE__);
	Backup<decltype(engine->grf_prop.spritegroups)> sprites(engine->grf_prop.spritegroups);
	Backup<Randomizer> interactive(_interactive_random);
	_interactive_random.SetSeed(12345);
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	auto *det = new DeterministicSpriteGroup();
	det->size = DSG_SIZE_DWORD;
	det->var_scope = VSG_SCOPE_SELF;
	det->default_result.calculated_result = true;
	for (uint32_t value : {0U, 1U, 0x3FFEU, 0x3FFFU, 0x4000U, 0x4001U, 0x7FFEU, 0x7FFFU, 0xFFFFU}) {
		engine->grf_prop.spritegroups.clear();
		if (value != CALLBACK_FAILED) engine->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
		det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = value}};
		for (int fallback : {INT32_MIN, -17, 0, 17, INT32_MAX}) for (bool is_signed : {false, true}) {
			fmt::print(stderr, "FLEET resolver-property-boundary {} {} {} {} {}\n", value, fallback, is_signed, GetVehicleProperty(v, PROP_TRAIN_SPEED, fallback, is_signed), GetEngineProperty(v->engine_type, PROP_TRAIN_SPEED, fallback, nullptr, is_signed));
		}
	}
	sprites.Revert();
	const bool restored = engine->grf_prop.spritegroups == sprites.GetOriginalValue();
	sprites.Restore();
	delete det;
	SaveRandomSeeds(&after);
	fmt::print(stderr, "FLEET resolver-property-restored {} {} {} {} {} {}\n", restored, SameRandom(before, after), after.random.state[0], after.random.state[1], after.interactive_random.state[0], after.interactive_random.state[1]);
	interactive.Restore();
}

/* Exercise real Action2 groups through the unchanged public native API on both
 * runtimes. Cache values are captured before any diagnostic rebuild can hide them. */
static void ResolverProbe()
{
	Train *v = nullptr;
	for (Train *t : Train::Iterate()) if (t->IsFrontEngine() && t->Next() != nullptr) { v = t; break; }
	if (v == nullptr) FixtureAbort(__LINE__);
	ResolverVariables(v);
	Engine *e = Engine::Get(v->engine_type);
	Backup<decltype(e->grf_prop.spritegroups)> sprites(e->grf_prop.spritegroups);
	Backup<const GRFFile *> file(e->grf_prop.grffile);
	GRFFile grf;
	grf.grf_version = 8;
	for (uint i = 0; i < NUM_CARGO; ++i) grf.cargo_map[i] = NUM_CARGO - 1 - i;
	file.Change(&grf);
	ResolverRealBoundaries(v, e);
	ResolverOverrideOrder(v->GetNextVehicle());
	ResolverPropertyBoundaries(v, e);
	if (!SpriteGroup::CanAllocateItem(8)) FixtureAbort(__LINE__);
	auto *det = new DeterministicSpriteGroup();
	det->size = DSG_SIZE_DWORD;
	det->var_scope = VSG_SCOPE_SELF;
	det->default_result.calculated_result = true;
	det->adjusts.push_back({.operation = DSGA_OP_RST, .variable = 0x42, .and_mask = UINT32_MAX});
	e->grf_prop.spritegroups.clear();
	e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
	v->InvalidateNewGRFCacheOfChain();
	{
		VehicleResolverObject ro(v->engine_type, v, VehicleResolverObject::WO_NONE);
		bool available = true;
		uint32_t translated = ro.self_scope.GetVariable(0x42, 0, available);
		NewGRFCache before = v->grf_cache;
		uint32_t hit = ro.self_scope.GetVariable(0x42, 0, available);
		uint16_t callback = GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v);
		fmt::print(stderr, "FLEET resolver-cache {} {} {} {} {}\n", available, translated, before.consist_cargo_information, hit == translated, callback == (translated & 0x7FFF));
		ro.grffile = nullptr;
		uint32_t untranslated = ro.self_scope.GetVariable(0x42, 0, available);
		fmt::print(stderr, "FLEET resolver-translation {} {}\n", untranslated, v->grf_cache == before);
		GRFFile caller;
		caller.grf_version = 7;
		for (uint i = 0; i < NUM_CARGO; ++i) caller.cargo_map[i] = i;
		ro.grffile = &caller;
		uint32_t version7 = ro.self_scope.GetVariable(0x42, 0, available);
		caller.grf_version = 8;
		uint32_t version8 = ro.self_scope.GetVariable(0x42, 0, available);
		fmt::print(stderr, "FLEET resolver-caller-grf {} {} {}\n", version7 == untranslated, version8 != translated, v->grf_cache == before);
		Backup<decltype(e->badges)> badges(e->badges, std::vector<BadgeID>{BadgeID(0x12345678)});
		caller.badge_list = {BadgeID(0x12345678)};
		uint32_t badge_count = ro.self_scope.GetVariable(0x64, 0, available);
		fmt::print(stderr, "FLEET resolver-wide-badge {} {}\n", available, badge_count > 0);
		badges.Restore();
		ro.grffile = nullptr;

		ro.SetRegister(0x10F, 1);
		ro.SetRegister(0x10E, 0);
		uint32_t relative = ro.self_scope.GetVariable(0x61, 0x40, available);
		VehicleResolverObject other(v->engine_type, v->Next(), VehicleResolverObject::WO_NONE);
		uint32_t direct = other.self_scope.GetVariable(0x40, 0, available);
		fmt::print(stderr, "FLEET resolver-var61 {} {} {}\n", available, relative, relative == direct);
		det->var_scope = VSG_SCOPE_PARENT;
		det->adjusts[0].variable = 0x84;
		uint16_t parent = GetVehicleCallbackParent(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v, v->Next());
		fmt::print(stderr, "FLEET resolver-parent {}\n", parent == (v->Next()->index.base() & 0x7FFF));
		ro.SetRegister(0x100, 1);
		auto *scope = ro.GetScope(VSG_SCOPE_RELATIVE, 0);
		fmt::print(stderr, "FLEET resolver-relative {}\n", scope->GetVariable(0x84, 0, available) == v->Next()->index.base());
		uint64_t relative_digest = 0;
		uint relative_cases = 0;
		for (int32_t reg : {INT32_MIN, -1, 0, 1, INT32_MAX}) for (uint8_t selector : {0, 1, 0x40, 0x41, 0x80, 0x81, 0xC0, 0xC1}) {
			ro.SetRegister(0x100, reg);
			bool valid = true;
			uint32_t value = ro.GetScope(VSG_SCOPE_RELATIVE, selector)->GetVariable(0x84, 0, valid);
			relative_digest = relative_digest * 31337 + value + valid;
			++relative_cases;
		}
		fmt::print(stderr, "FLEET resolver-relative-matrix {} {}\n", relative_cases, relative_digest);
		ro.GetScope(VSG_SCOPE_RELATIVE, 1);
		ro.self_scope.SetVehicle(v->Next());
		bool valid = true;
		uint32_t fresh = ro.GetScope(VSG_SCOPE_RELATIVE, 1)->GetVariable(0x84, 0, valid);
		const Vehicle *expected = v->Next()->Next();
		fmt::print(stderr, "FLEET resolver-relative-uncached {}\n", fresh == (expected == nullptr ? UINT_MAX : expected->index.base()) && valid == (expected != nullptr));
		ro.self_scope.SetVehicle(v);
		uint64_t policy_digest = 0;
		uint policy_cases = 0;
		for (CallbackID cb : {CBID_NO_CALLBACK, CBID_RANDOM_TRIGGER, CBID_TRAIN_ALLOW_WAGON_ATTACH, CBID_VEHICLE_START_STOP_CHECK, CBID_VEHICLE_32DAY_CALLBACK, CBID_VEHICLE_COLOUR_MAPPING, CBID_VEHICLE_SPAWN_VISUAL_EFFECT, CBID_VEHICLE_MODIFY_PROPERTY}) {
			ro.callback = cb;
			for (uint32_t parameter : {0x5FU, 0x61U, 0x84U}) {
				bool allowed = true;
				uint32_t value = ro.self_scope.GetVariable(0x61, parameter, allowed);
				policy_digest = policy_digest * 31337 + value + allowed;
				++policy_cases;
			}
		}
		ro.callback = CBID_NO_CALLBACK;
		fmt::print(stderr, "FLEET resolver-var61-policy {} {}\n", policy_cases, policy_digest);

		/* Nested deterministic procedure changes relative scope, then returns to self.
		 * Output registers are copied by ResolveCallback after the nested evaluation. */
		auto *procedure = new DeterministicSpriteGroup();
		procedure->size = DSG_SIZE_DWORD; procedure->var_scope = VSG_SCOPE_RELATIVE;
		procedure->default_result.calculated_result = true;
		procedure->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x84, .and_mask = UINT32_MAX}};
		det->var_scope = VSG_SCOPE_SELF;
		det->adjusts = {
			{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 1},
			{.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100},
			{.operation = DSGA_OP_RST, .variable = 0x7E, .and_mask = UINT32_MAX, .subroutine = procedure},
			{.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101},
			{.operation = DSGA_OP_RST, .variable = 0x84, .and_mask = UINT32_MAX},
		};
		std::array<int32_t, 2> registers{};
		uint16_t nested = GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v, registers);
		fmt::print(stderr, "FLEET resolver-reentry {} {} {}\n", nested == (v->index.base() & 0x7FFF), registers[0] == 1, registers[1] == static_cast<int32_t>(v->Next()->index.base() & 0x7FFF));
		det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0x7FFE}};
		fmt::print(stderr, "FLEET resolver-property {} {}\n", GetVehicleProperty(v, PROP_TRAIN_SPEED, 17, true) == -2, GetVehicleProperty(v, PROP_TRAIN_SPEED, 17) == 0x7FFE);
		SavedRandomSeeds probability_before, probability_after;
		SaveRandomSeeds(&probability_before);
		auto probability = TestVehicleBuildProbability(v, v->engine_type, BuildProbabilityType::Reversed);
		SaveRandomSeeds(&probability_after);
		fmt::print(stderr, "FLEET resolver-probability {} {} {}\n", probability.has_value(), probability.value_or(false), !SameRandom(probability_before, probability_after));
		e->grf_prop.spritegroups.clear();
		SaveRandomSeeds(&probability_before);
		probability = TestVehicleBuildProbability(v, v->engine_type, BuildProbabilityType::Reversed);
		SaveRandomSeeds(&probability_after);
		fmt::print(stderr, "FLEET resolver-probability-failed {} {}\n", !probability.has_value(), SameRandom(probability_before, probability_after));
		e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
		det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x61, .parameter = 0x84, .and_mask = UINT32_MAX}};
		auto *fallback = new CallbackResultSpriteGroup(123);
		det->error_group = fallback;
		fmt::print(stderr, "FLEET resolver-unavailable {}\n", GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v) == 123);
		/* DoResolve clears changed registers; the actual group above supplies them. */
		delete fallback; delete procedure;
	}
	{
		auto *low = new CallbackResultSpriteGroup(7);
		auto *high = new CallbackResultSpriteGroup(11);
		auto *real = new RealSpriteGroup();
		real->loaded = {low, high}; real->loading = {high, low};
		e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, real);
		Backup<Order> order(v->current_order);
		for (bool loading : {false, true}) {
			if (loading) v->current_order.MakeLoading(false); else v->current_order.MakeDummy();
			fmt::print(stderr, "FLEET resolver-real {} {}\n", loading, GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v));
		}
		fmt::print(stderr, "FLEET resolver-purchase-real {}\n", GetEngineProperty(v->engine_type, PROP_TRAIN_SPEED, 17) == 11);
		order.Restore();
		e->grf_prop.spritegroups.clear();
		delete real; delete high; delete low;
	}
	auto *result = new CallbackResultSpriteGroup(7);
	auto *random = new RandomizedSpriteGroup();
	random->var_scope = VSG_SCOPE_SELF;
	random->triggers = 0x1F;
	random->cmp_mode = RSG_CMP_ANY;
	random->lowest_randbit = 3;
	random->groups = {result, result};
	e->grf_prop.spritegroups.clear();
	e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, random);
	for (Train *u = v; u != nullptr; u = u->Next()) u->tcache.cached_override = random;
	for (VehicleRandomTrigger trigger : {VehicleRandomTrigger::NewCargo, VehicleRandomTrigger::Depot, VehicleRandomTrigger::Empty, VehicleRandomTrigger::Callback32}) {
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		TriggerVehicleRandomisation(v, trigger);
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET resolver-trigger {} {}\n", to_underlying(trigger), !SameRandom(before, after));
		uint index = 0;
		for (Train *u = v; u != nullptr; u = u->Next()) {
			fmt::print(stderr, "FLEET resolver-random {} {} {} {} {}\n", to_underlying(trigger), index++, u->random_bits, u->waiting_random_triggers.base(), u->grf_cache.cache_valid);
		}
	}
	/* NewCargo propagates AnyNewCargo; its waiting bit must have been consumed. */
	bool propagated = true;
	for (Train *u = v; u != nullptr; u = u->Next()) propagated &= !u->waiting_random_triggers.Test(VehicleRandomTrigger::AnyNewCargo);
	fmt::print(stderr, "FLEET resolver-anynew {}\n", propagated);
	/* A matching group with one outcome reseeds zero bits but still draws RNG. */
	random->groups = {result};
	uint16_t bits = v->random_bits;
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before); TriggerVehicleRandomisation(v, VehicleRandomTrigger::Callback32); SaveRandomSeeds(&after);
	fmt::print(stderr, "FLEET resolver-zero-reseed {} {}\n", bits == v->random_bits, !SameRandom(before, after));
	v->waiting_random_triggers = VehicleRandomTriggers(0x10);
	FillNewGRFVehicleCache(v);
	NewGRFCache cache = v->grf_cache;
	VehicleID id = v->index;
	uint8_t waiting = v->waiting_random_triggers.base();
	bits = v->random_bits;
	/* Native reload destroys these synthetic pooled groups and resets engine specs.
	 * No group survives: reinstall fresh deterministic data afterwards. Shell random
	 * bits/waiting triggers survive; the transient cache is rebuilt, never restored. */
	sprites.Trash(); file.Trash();
	ReloadNewGRFData();
	v = Train::Get(id); e = Engine::Get(v->engine_type);
	fmt::print(stderr, "FLEET resolver-grf-reload {} {} {} {}\n", cache.cache_valid == 31, bits == v->random_bits, waiting == v->waiting_random_triggers.base(), v->grf_cache.cache_valid);
	Backup<decltype(e->grf_prop.spritegroups)> reinstalled(e->grf_prop.spritegroups);
	Backup<const GRFFile *> refile(e->grf_prop.grffile, &grf);
	if (!SpriteGroup::CanAllocateItem()) FixtureAbort(__LINE__);
	det = new DeterministicSpriteGroup();
	det->size = DSG_SIZE_DWORD; det->var_scope = VSG_SCOPE_SELF;
	det->default_result.calculated_result = true;
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x42, .and_mask = UINT32_MAX}};
	e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, det);
	v->InvalidateNewGRFCacheOfChain();
	{
		VehicleResolverObject ro(v->engine_type, v, VehicleResolverObject::WO_NONE);
		bool available = true;
		uint32_t value = ro.self_scope.GetVariable(0x42, 0, available);
		fmt::print(stderr, "FLEET resolver-reinstalled {} {}\n", available, GetVehicleCallback(CBID_VEHICLE_MODIFY_PROPERTY, 0, 0, v->engine_type, v) == (value & 0x7FFF));
	}
	reinstalled.Restore(); refile.Restore(); delete det;
	v->InvalidateNewGRFCacheOfChain();
	fmt::print(stderr, "FLEET resolver-persist {} {} {}\n", id.base(), v->random_bits, v->waiting_random_triggers.base());

	fmt::print(stderr, "FLEET resolver-checkpoint complete\n");
}

struct VisualEffectObserver : SpriteGroup {
	VisualEffectObserver() : SpriteGroup() {}
	const SpriteGroup *program = nullptr;
	mutable uint spawn = 0, sound = 0;
	uint16_t visual = 0x41;
	bool failed = false;
	ResolverResult Resolve(ResolverObject &object) const override
	{
		if (object.callback == CBID_VEHICLE_VISUAL_EFFECT) return CallbackResult(this->visual);
		if (object.callback == CBID_VEHICLE_SOUND_EFFECT) { if (object.callback_param1 == VSE_VISUAL_EFFECT) ++this->sound; return {}; }
		if (object.callback == CBID_VEHICLE_SPAWN_VISUAL_EFFECT) { ++this->spawn; return this->failed ? ResolverResult{} : SpriteGroup::Resolve(this->program, object, false); }
		return {};
	}
};

static void VisualEffectProbe()
{
	RoadVehicle *v = nullptr;
	for (RoadVehicle *u : RoadVehicle::Iterate()) if (u->IsPrimaryVehicle() && u->Next() == nullptr) { v = u; break; }
	if (v == nullptr) FixtureAbort(__LINE__);
	Engine *e = Engine::Get(v->engine_type);
	fmt::print(stderr, "FLEET effect-load-cache {}\n", v->vcache.cached_vis_effect);
	Backup<CargoGRFFileProps> props(e->grf_prop);
	Backup<VehicleCallbackMasks> mask(e->info.callback_mask);
	Backup<uint8_t> smoke(_settings_game.vehicle.smoke_amount, 2);
	Backup<bool> sound(_settings_client.sound.vehicle, true);
	Backup<VehicleCache> cache(v->vcache);
	Backup<VehStates> status(v->vehstatus, VehStates{});
	Backup<uint16_t> speed(v->cur_speed, 2);
	Backup<uint8_t> tick(v->tick_counter, 0);
	Backup<Direction> direction(v->direction, DIR_NE);
	Backup<uint8_t> length(v->gcache.cached_veh_length, 5);
	Backup<TileIndex> tile(v->tile);
	for (uint32_t index = 0; index < Map::Size(); ++index) { TileIndex t{index}; if (IsTileType(t, MP_CLEAR) && !IsBridgeAbove(t)) { tile.Change(t); break; } }
	Backup<int> x(v->x_pos, TileX(v->tile) * TILE_SIZE + 8), y(v->y_pos, TileY(v->tile) * TILE_SIZE + 8), z(v->z_pos, 30);
	GRFFile grf; grf.grf_version = 8; grf.grfid = 0x466EFFE0;
	auto config = std::make_unique<GRFConfig>("effect-fixture.grf");
	config->ident.grfid = grf.grfid;
	_grfconfig.push_back(std::move(config));
	Backup<uint8_t> default_visual(e->VehInfo<RoadVehicleInfo>().visual_effect, VE_DEFAULT);
	e->grf_prop.grffile = &grf;
	e->info.callback_mask.Set(VehicleCallbackMask::VisualEffect).Set(VehicleCallbackMask::SoundEffect);
	if (!SpriteGroup::CanAllocateItem(2)) FixtureAbort(__LINE__);
	auto *observer = new VisualEffectObserver();
	auto *program = new DeterministicSpriteGroup();
	observer->program = program;
	program->size = DSG_SIZE_DWORD;
	program->default_result.calculated_result = true;
	e->grf_prop.spritegroups.clear();
	e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, observer);
	for (uint16_t result : {uint16_t(0x2003), uint16_t(0), uint16_t(0x4001)}) {
		program->adjusts.clear();
		for (auto [reg, value] : {std::pair{0x100U, 0xFE03FCF1U}, {0x101U, 0x04FE02F2U}, {0x102U, 0x0601FFF3U}}) {
			program->adjusts.push_back({.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = value});
			program->adjusts.push_back({.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = reg});
		}
		program->adjusts.push_back({.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = result});
		std::set<VehicleID> before;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) before.insert(u->index);
		v->UpdateVisualEffect();
		VehicleCache copied(v->vcache);
		v->vcache.cached_vis_effect = 0x42;
		bool independent = copied.cached_vis_effect == 0x41;
		v->vcache = copied;
		auto expected = _random; expected.Next();
		uint old_sound = observer->sound, old_spawn = observer->spawn;
		v->ShowVisualEffect();
		fmt::print(stderr, "FLEET effect-emission {} cache {} independent {} callbacks {} sound {} rng {}\n", result, v->vcache.cached_vis_effect, independent, observer->spawn - old_spawn, observer->sound - old_sound, _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1]);
		uint count = 0;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) if (!before.contains(u->index)) {
			++count;
			fmt::print(stderr, "FLEET effect-created {} {} {} {} {} {}\n", result, u->index.base(), u->subtype, u->x_pos - v->x_pos, u->y_pos - v->y_pos, u->z_pos - v->z_pos);
		}
		fmt::print(stderr, "FLEET effect-count {} {}\n", result, count);
	}
	for (const char *label : {"failed", "unsupported", "smoke-off", "stopped", "slow", "hidden", "bridge", "depot", "tunnel", "bridge-middle"}) {
		observer->failed = std::string_view(label) == "failed";
		program->adjusts[0].and_mask = std::string_view(label) == "unsupported" ? 0xFE03FCF0 : 0xFE03FCF1;
		uint old_sound = observer->sound, old_spawn = observer->spawn;
		SavedRandomSeeds before, after; SaveRandomSeeds(&before);
		auto expected = _random;
		const bool attempted = observer->failed || std::string_view(label) == "unsupported" || std::string_view(label) == "bridge-middle";
		if (attempted) expected.Next();
		smoke.Change(std::string_view(label) == "smoke-off" ? 0 : 2);
		v->cur_speed = std::string_view(label) == "slow" ? 1 : 2;
		v->vehstatus = std::string_view(label) == "stopped" ? VehStates{VehState::Stopped} : std::string_view(label) == "hidden" ? VehStates{VehState::Hidden} : VehStates{};
		Tile current(v->tile);
		auto original_tile = std::tuple{current.type(), current.m1(), current.m2(), current.m3(), current.m4(), current.m5(), current.m6(), current.m7(), current.m8()};
		const int original_x = v->x_pos;
		if (std::string_view(label) == "depot") MakeRoadDepot(current, v->owner, DepotID::Invalid(), DIAGDIR_NE, ROADTYPE_ROAD);
		if (std::string_view(label) == "tunnel") MakeRoadTunnel(current, v->owner, DIAGDIR_NE, ROADTYPE_ROAD, INVALID_ROADTYPE);
		if (std::string_view(label) == "bridge-middle") { MakeBridgeRamp(current, v->owner, 0, DIAGDIR_NE, TRANSPORT_ROAD); SetBridgeMiddle(current, AXIS_X); v->x_pos += TILE_SIZE; }
		if (std::string_view(label) == "bridge") SetBridgeMiddle(v->tile, AXIS_X);
		const size_t effects = EffectVehicle::GetNumItems();
		v->ShowVisualEffect();
		std::tie(current.type(), current.m1(), current.m2(), current.m3(), current.m4(), current.m5(), current.m6(), current.m7(), current.m8()) = original_tile;
		v->x_pos = original_x;
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET effect-gate {} callbacks {} sound {} count {} rng {}\n", label, observer->spawn - old_spawn, observer->sound - old_sound, EffectVehicle::GetNumItems() - effects, _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1] && (attempted || SameRandom(before, after)));
	}
	v->cur_speed = 2; v->vehstatus = {};
	for (const char *label : {"steam", "diesel", "electric", "steam-tick", "electric-tick", "diesel-top"}) {
		const std::string_view name(label);
		const uint8_t model = name.starts_with("steam") ? 1 : name.starts_with("diesel") ? 2 : 3;
		v->vcache.cached_vis_effect = (model << VE_TYPE_START) | VE_OFFSET_CENTRE;
		v->tick_counter = name.ends_with("tick") ? 1 : 0;
		v->cur_speed = name.ends_with("top") ? v->GetCurrentMaxSpeed() : 2;
		SetRandomSeed(8);
		auto expected = _random;
		if (name == "diesel" || name == "electric") expected.Next();
		std::set<VehicleID> before;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) before.insert(u->index);
		const uint old_sound = observer->sound;
		v->ShowVisualEffect();
		uint count = 0;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) if (!before.contains(u->index)) {
			++count;
			fmt::print(stderr, "FLEET effect-stock-created {} {} {} {} {} {}\n", label, u->index.base(), u->subtype, u->x_pos - v->x_pos, u->y_pos - v->y_pos, u->z_pos - v->z_pos);
		}
		fmt::print(stderr, "FLEET effect-stock {} count {} sound {} rng {}\n", label, count, observer->sound - old_sound, _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1]);
	}
	for (uint16_t callback : {uint16_t(CALLBACK_FAILED), uint16_t(0), uint16_t(0xFF), uint16_t(0x80), uint16_t(0x141), uint16_t(0xC1)}) {
		observer->visual = callback;
		v->vcache.cached_vis_effect = 0x41;
		SavedRandomSeeds before, after; SaveRandomSeeds(&before);
		v->UpdateVisualEffect(callback != 0xC1);
		SaveRandomSeeds(&after);
		fmt::print(stderr, "FLEET effect-cache {} {} rng {}\n", callback, v->vcache.cached_vis_effect, SameRandom(before, after));
	}
	default_visual.Restore();
	_grfconfig.pop_back();
	e->grf_prop = props.GetOriginalValue();
	delete program; delete observer;
	props.Restore(); mask.Restore(); smoke.Restore(); sound.Restore(); cache.Restore(); status.Restore(); speed.Restore(); tick.Restore(); direction.Restore(); length.Restore(); tile.Restore(); x.Restore(); y.Restore(); z.Restore();
	fmt::print(stderr, "FLEET effect-checkpoint complete\n");
}

static VehicleRailFlags VisualTrainFlags(const Train *v)
{
#ifdef WITH_RUST
	return v->GetTrainFlags();
#else
	return v->flags;
#endif
}
static RailTypes VisualTrainRail(const Train *v)
{
#ifdef WITH_RUST
	return v->GetRailTypes();
#else
	return v->railtypes;
#endif
}
static void VisualTrainState(Train *v, VehicleRailFlags flags, RailTypes rail)
{
#ifdef WITH_RUST
	v->SetTrainFlags(flags); v->SetRailTypes(rail);
#else
	v->flags = flags; v->railtypes = rail;
#endif
}
static void VisualTrainProbe()
{
	Train *v = nullptr;
	for (Train *t : Train::Iterate()) if (t->IsFrontEngine() && VisualTrainRail(t).base() != 0) { v = t; break; }
	if (v == nullptr) FixtureAbort(__LINE__);
	TileIndex track = INVALID_TILE;
	for (uint32_t i = 0; i < Map::Size(); ++i) { TileIndex t{i}; if (IsPlainRailTile(t) && !IsBridgeAbove(t) && HasPowerOnRail(VisualTrainRail(v), GetTileRailType(t))) { track = t; break; } }
	if (track == INVALID_TILE) FixtureAbort(__LINE__);
	Backup<TileIndex> tile(v->tile, track);
	Backup<int> x(v->x_pos, TileX(track) * TILE_SIZE + 8), y(v->y_pos, TileY(track) * TILE_SIZE + 8), z(v->z_pos, GetSlopePixelZ(TileX(track) * TILE_SIZE + 8, TileY(track) * TILE_SIZE + 8, true));
	std::vector<std::pair<Train *, VehicleCache>> caches;
	for (Train *t = v; t != nullptr; t = t->Next()) { caches.emplace_back(t, t->vcache); t->vcache.cached_vis_effect = 0x40; }
	Engine *e = Engine::Get(v->engine_type);
	Backup<CargoGRFFileProps> props(e->grf_prop);
	Backup<VehicleCallbackMasks> mask(e->info.callback_mask);
	Backup<RailVehicleInfo> info(e->VehInfo<RailVehicleInfo>());
	Backup<uint8_t> smoke(_settings_game.vehicle.smoke_amount, 2);
	Backup<bool> sound(_settings_client.sound.vehicle, true);
	Backup<VehStates> status(v->vehstatus, VehStates{});
	Backup<uint16_t> speed(v->cur_speed, 2);
	Backup<uint8_t> tick(v->tick_counter, 0);
	Backup<Direction> direction(v->direction, DIR_NE);
	Backup<uint8_t> length(v->gcache.cached_veh_length, 5);
	const auto flags = VisualTrainFlags(v);
	const auto rail = VisualTrainRail(v);
	GRFFile grf; grf.grf_version = 8;
	e->grf_prop.grffile = &grf;
	e->info.callback_mask = VehicleCallbackMask::SoundEffect;
	if (!SpriteGroup::CanAllocateItem(2)) FixtureAbort(__LINE__);
	auto *observer = new VisualEffectObserver();
	auto *program = new DeterministicSpriteGroup();
	observer->program = program;
	program->size = DSG_SIZE_DWORD;
	program->default_result.calculated_result = true;
	program->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0xFE03FCF1}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 1}};
	e->grf_prop.spritegroups.clear(); e->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, observer);
	for (const char *label : {"stock-flipped", "reversing", "unpowered", "advanced-flipped", "advanced-center"}) {
		const std::string_view name(label);
		v->vcache.cached_vis_effect = name.starts_with("advanced") ? 0x41 : 0x11;
		VisualTrainState(v, VehicleRailFlags{flags}.Set(VehicleRailFlag::Flipped).Set(VehicleRailFlag::Reversing, name == "reversing"), name == "unpowered" ? RailTypes{} : rail);
		program->adjusts.back().and_mask = name == "advanced-center" ? 0x2001 : 1;
		SetRandomSeed(8); auto expected = _random;
		if (name.starts_with("advanced")) expected.Next();
		std::set<VehicleID> before;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) before.insert(u->index);
		uint old_sound = observer->sound;
		v->ShowVisualEffect();
		uint count = 0;
		for (const EffectVehicle *u : EffectVehicle::Iterate()) if (!before.contains(u->index)) {
			++count;
			fmt::print(stderr, "FLEET effect-train-created {} {} {} {} {} {}\n", label, u->index.base(), u->subtype, u->x_pos - v->x_pos, u->y_pos - v->y_pos, u->z_pos - v->z_pos);
		}
		fmt::print(stderr, "FLEET effect-train {} count {} sound {} rng {}\n", label, count, observer->sound - old_sound, _random.state[0] == expected.state[0] && _random.state[1] == expected.state[1]);
	}
	e->info.callback_mask = {};
	for (uint8_t kind : {uint8_t(EC_STEAM), uint8_t(EC_DIESEL), uint8_t(EC_ELECTRIC), uint8_t(EC_MONORAIL), uint8_t(255)}) {
		e->VehInfo<RailVehicleInfo>().engclass = static_cast<EngineClass>(kind); e->VehInfo<RailVehicleInfo>().railveh_type = RAILVEH_SINGLEHEAD; e->VehInfo<RailVehicleInfo>().visual_effect = VE_DEFAULT;
		v->UpdateVisualEffect();
		fmt::print(stderr, "FLEET effect-train-default {} {}\n", kind, v->vcache.cached_vis_effect);
	}
	e->VehInfo<RailVehicleInfo>().railveh_type = RAILVEH_WAGON; e->VehInfo<RailVehicleInfo>().engclass = EC_STEAM;
	v->UpdateVisualEffect(); fmt::print(stderr, "FLEET effect-train-wagon {}\n", v->vcache.cached_vis_effect);
	VisualTrainState(v, flags, rail);
	tile.Restore(); x.Restore(); y.Restore(); z.Restore();
	props.Restore(); mask.Restore(); info.Restore(); smoke.Restore(); sound.Restore(); status.Restore(); speed.Restore(); tick.Restore(); direction.Restore(); length.Restore();
	for (auto &[t, cache] : caches) t->vcache = cache;
	delete program; delete observer;
	fmt::print(stderr, "FLEET effect-train-checkpoint complete\n");
}

/** Running road vehicle 12 of padhattan-ridge-2000 heads to a service depot; the witness runs on 2001-01-01. */
static VehicleID _drain_vehicle = VehicleID::Invalid();
static UnitID _drain_unit = 0;
static bool FleetScenario(std::span<std::string_view> args)
{
	if (args.size() != 2) return false;
	if (args[1] == "effect-emission") { VisualEffectProbe(); return true; }
	if (args[1] == "effect-emission-train") { VisualTrainProbe(); return true; }
	Backup<CompanyID> company(_current_company);
	if (args[1] == "resolver") { ResolverProbe(); company.Restore(); return true; }
	if (args[1] == "resolver-inspect") {
		Train *v = nullptr;
		for (Train *t : Train::Iterate()) if (t->IsFrontEngine() && t->Next() != nullptr) { v = t; break; }
		if (v == nullptr) FixtureAbort(__LINE__);
		fmt::print(stderr, "FLEET resolver-saved {} {} {}\n", v->index.base(), v->random_bits, v->waiting_random_triggers.base());
		ResolverVariables(v);
		company.Restore(); return true;
	}
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
		ResolverVariables(v);
		if (type == VEH_AIRCRAFT) ResolverAircraft(Aircraft::From(v));
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
		AttachmentCallbacks(v);
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
	if (args[1] == "ship") ResolverVariables(v);
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

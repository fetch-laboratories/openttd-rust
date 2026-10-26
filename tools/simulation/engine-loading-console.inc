/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Real FILE-loaded engine definitions, native EIDS reload and construction/refit commands. */
#include "newgrf.h"
#include "newgrf_config.h"
#include "newgrf_engine.h"
#include "newgrf_cargo.h"
#include "newgrf/newgrf_internal_vehicle.h"
#include "vehicle_cmd.h"
#include "order_cmd.h"
#include "depot_base.h"
#include "station_base.h"
#include "depot_map.h"
#include "water_map.h"
#include "road_map.h"
#include "core/random_func.hpp"
static void LoadingRequire(bool value, const char *name)
{
	fmt::print(stderr, "ENGINE loading-case {} {}\n", name, value);
	if (!value) FixtureAbort(__LINE__);
}
static void LoadingScan()
{
	const char *folder = std::getenv("OPENTTD_ENGINE_LOADING");
	if (folder == nullptr) return;
	for (uint which = 1; which <= (std::getenv("OPENTTD_ENGINE_PROPERTIES") == nullptr ? 5U : 6U); ++which) {
		auto config = std::make_unique<GRFConfig>(fmt::format("{}/owner470-{}.grf", folder, which));
		LoadingRequire(FillGRFDetails(*config, which == 4), "file-scan");
		LoadingRequire(config->ident.grfid == 0x4700A000 + which, "file-identity");
		if (which == 4) {
			config->flags.Set(GRFConfigFlag::Static);
			_grfconfig_static.push_back(std::make_unique<GRFConfig>(*config));
		}
		_all_grfs.push_back(std::move(config));
	}
}
static std::span<const EngineIDMapping> LoadingMappings(VehicleType type)
{
#ifdef WITH_RUST
	return _engine_mngr.Mappings(type);
#else
	return _engine_mngr.mappings[type];
#endif
}
static Engine *LoadingEngine(uint file, VehicleType type, uint16_t local)
{
	const auto *grf = GetFileByGRFID(0x4700A000 + file);
	if (grf == nullptr) FixtureAbort(__LINE__);
	EngineID id = GetNewEngineID(grf, type, local);
	if (!Engine::IsValidID(id)) FixtureAbort(__LINE__);
	return Engine::Get(id);
}
static void LoadingInspect()
{
	const CargoType goods = GetCargoTypeByLabel(CT_GOODS), mail = GetCargoTypeByLabel(CT_MAIL), passengers = GetCargoTypeByLabel(CT_PASSENGERS);
	LoadingRequire(IsValidCargoType(goods) && IsValidCargoType(mail) && IsValidCargoType(passengers), "cargo-inputs");
	const bool dynamic = _settings_game.vehicle.dynamic_engines;
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		Engine *a = LoadingEngine(1, type, 300), *b = LoadingEngine(2, type, 300), *c = LoadingEngine(3, type, 300);
		LoadingRequire((a->index != b->index) == dynamic && a == c, "scopes-overrides");
		LoadingRequire(a->info.cargo_type == goods && a->info.refit_mask == ((CargoTypes{1} << goods) | (CargoTypes{1} << mail)), "custom-refit-v8");
		LoadingRequire(b->info.cargo_type == (dynamic ? mail : goods), "local-slot-version");
		LoadingRequire(_engine_mngr.GetID(type, 0, dynamic ? INVALID_GRFID : GetFileByGRFID(0x4700A001)->grfid) == EngineID::Invalid(), "reservation");
		fmt::print(stderr, "ENGINE loading-engine {} {} {} {} {} {} {} {} {}\n", type, a->index, b->index, a->GetGRFID(), a->info.cargo_type, a->info.refit_mask, a->info.climates.base(), a->info.variant_id, a->info.callback_mask.base());
		const auto map = LoadingMappings(type);
		LoadingRequire(std::ranges::is_sorted(map, std::less{}, EngineIDMappingKeyProjection{}), "mapping-order");
		uint64_t digest = 14695981039346656037ULL;
		for (const auto &m : map) for (uint64_t field : {uint64_t(m.grfid), uint64_t(m.internal_id), uint64_t(m.type), uint64_t(m.substitute_id), uint64_t(m.engine.base())}) digest = (digest ^ field) * 1099511628211ULL;
		fmt::print(stderr, "ENGINE loading-mapping {} {} {}\n", type, map.size(), digest);
		for (const auto &[slot, expected] : {std::pair{0, uint16_t{1}}, {1, uint16_t{2}}, {2, uint16_t{0}}, {3, uint16_t{CALLBACK_FAILED}}}) {
			uint16_t result = GetVehicleCallback(CBID_VEHICLE_CUSTOM_REFIT, 0, slot, a->index, nullptr);
			LoadingRequire(result == expected, "file-callback");
		}
	}
	for (uint16_t local : {302, 304, 309, 314}) LoadingRequire(LoadingEngine(1, VEH_TRAIN, local)->info.climates.None(), "disabled-cargo-or-label");
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 301)->info.cargo_type == passengers && LoadingEngine(1, VEH_TRAIN, 301)->info.refit_mask == 0 && LoadingEngine(1, VEH_TRAIN, 301)->IsEnabled(), "empty-refit");
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 312)->info.cargo_type == passengers && LoadingEngine(1, VEH_TRAIN, 312)->info.refit_mask == _cargo_mask, "unset-refit");
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 313)->info.cargo_type == mail && LoadingEngine(1, VEH_TRAIN, 313)->info.refit_mask == (CargoTypes{1} << mail), "required-classes");
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 315)->info.cargo_type == mail, "missing-cargo-label");
	const auto stale = _engine_mngr.GetID(VEH_TRAIN, 450, dynamic ? 0x4700A005 : INVALID_GRFID);
	LoadingRequire(Engine::IsValidID(stale) && GetFileByGRFID(0x4700A005) == nullptr && Engine::Get(stale)->info.string_id == STR_NEWGRF_INVALID_ENGINE, "persistent-mapping");
	fmt::print(stderr, "ENGINE loading-stale {}\n", stale);
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 303)->VehInfo<RailVehicleInfo>().railtypes == RailTypes{RAILTYPE_RAIL, RAILTYPE_ELECTRIC}, "rail-label-list");
	for (uint16_t local : {305, 306, 310}) LoadingRequire(LoadingEngine(1, VEH_TRAIN, local)->info.variant_id == EngineID::Invalid(), "variant-cycle");
	LoadingRequire(LoadingEngine(1, VEH_TRAIN, 307)->info.variant_id == LoadingEngine(1, VEH_TRAIN, 308)->index && LoadingEngine(1, VEH_TRAIN, 308)->display_flags.Test(EngineDisplayFlag::HasVariants), "variant-parent");
	for (uint16_t local : {302, 303}) LoadingRequire(LoadingEngine(1, VEH_ROAD, local)->info.climates.None(), "road-label-class");
	LoadingRequire(LoadingEngine(1, VEH_ROAD, 301)->VehInfo<RoadVehicleInfo>().roadtype == ROADTYPE_ROAD && LoadingEngine(1, VEH_ROAD, 304)->VehInfo<RoadVehicleInfo>().roadtype == ROADTYPE_TRAM, "road-tram-labels");
	LoadingRequire(LoadingEngine(1, VEH_ROAD, 300)->VehInfo<RoadVehicleInfo>().max_speed == 292, "road-speed");
	const GRFFile *names = GetFileByGRFID(0x4700A004);
	const auto size = Engine::GetNumItems();
	LoadingRequire(names != nullptr && GetNewEngine(names, VEH_TRAIN, 65000, true) == nullptr && Engine::GetNumItems() == size, "static-no-allocation");
	LoadingRequire(_engine_mngr.GetID(VEH_TRAIN, 1, INVALID_GRFID) == EngineID{1}, "static-no-reservation");
	LoadingRequire(GetNewEngineID(names, VEH_TRAIN, 1) == (dynamic ? EngineID::Invalid() : EngineID{1}), "static-lookup");
	fmt::print(stderr, "ENGINE loading-liveries {}\n", _loaded_newgrf_features.used_liveries);
}
static void PropertiesInspect()
{
	LoadingRequire(_loaded_newgrf_features.has_2CC, "properties-2cc");
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		Engine *e = LoadingEngine(1, type, 500), *b = LoadingEngine(1, type, 501), *c = LoadingEngine(1, type, 502);
		LoadingRequire(e->info.decay_speed == 17 && e->info.lifelength == TimerGameCalendar::Year{60} && e->info.base_life == TimerGameCalendar::Year{90} && e->info.load_amount == 7, "properties-common");
		LoadingRequire(e->info.callback_mask.base() == 0x220 && e->info.misc_flags.base() == 2 && e->info.extra_flags.None(), "properties-masks");
		LoadingRequire(c->list_position < b->list_position && b->list_position < e->list_position, "properties-order");
		fmt::print(stderr, "ENGINE properties-state {} {} {} {} {} {} {} {} {} {}\n", type, e->index, e->info.base_intro, e->info.cargo_type, e->info.refit_mask, e->info.callback_mask.base(), e->info.misc_flags.base(), c->list_position, b->list_position, e->list_position);
	}
	const auto &train = LoadingEngine(1, VEH_TRAIN, 500)->VehInfo<RailVehicleInfo>();
	LoadingRequire(train.railveh_type == RAILVEH_MULTIHEAD && train.power == 700 && train.max_speed == 80 && train.weight == 86 && train.running_cost_class == static_cast<Price>(42) && train.visual_effect == 0xcf && train.curve_speed_mod == -384 && train.cost_factor == 53 && train.running_cost == 29, "properties-train");
	const auto &road = LoadingEngine(1, VEH_ROAD, 500)->VehInfo<RoadVehicleInfo>();
	LoadingRequire(road.roadtype == ROADTYPE_ROAD && road.max_speed == 492 && road.power == 127 && road.weight == 64 && road.running_cost_class == static_cast<Price>(45) && road.visual_effect == 0xcf && road.cost_factor == 53 && road.running_cost == 29, "properties-road");
	const auto &ship = LoadingEngine(1, VEH_SHIP, 500)->VehInfo<ShipVehicleInfo>();
	LoadingRequire(ship.max_speed == 300 && ship.acceleration == 1 && ship.capacity == 200 && ship.ocean_speed_frac == 7 && ship.canal_speed_frac == 9 && ship.visual_effect == 0xcf && ship.cost_factor == 53 && ship.running_cost == 29, "properties-ship");
	const auto &air = LoadingEngine(1, VEH_AIRCRAFT, 500)->VehInfo<AircraftVehicleInfo>();
	LoadingRequire(air.subtype == AIR_CTOL && air.max_speed == 1024 && air.passenger_capacity == 96 && air.mail_capacity == 20 && air.cost_factor == 53 && air.running_cost == 29, "properties-aircraft");
	const auto &traction = LoadingEngine(6, VEH_TRAIN, 500)->VehInfo<RailVehicleInfo>();
	LoadingRequire(traction.engclass == EC_ELECTRIC && traction.railtypes == RailTypes{RAILTYPE_ELECTRIC}, "properties-traction");
	SavedRandomSeeds rng; SaveRandomSeeds(&rng);
	fmt::print(stderr, "ENGINE properties-rng {} {} {} {}\n", rng.random.state[0], rng.random.state[1], rng.interactive_random.state[0], rng.interactive_random.state[1]);
}
static void PropertiesReadFailures()
{
	GRFFile *file = GetFileByGRFID(0x4700A001);
	Backup<GRFFile *> current(_cur_gps.grffile, file);
	auto call = [](VehicleType type, uint first, uint last, int prop, ByteReader &reader) {
		switch (type) {
			case VEH_TRAIN: return GrfChangeInfoHandler<GSF_TRAINS>::Activation(first, last, prop, reader);
			case VEH_ROAD: return GrfChangeInfoHandler<GSF_ROADVEHICLES>::Activation(first, last, prop, reader);
			case VEH_SHIP: return GrfChangeInfoHandler<GSF_SHIPS>::Activation(first, last, prop, reader);
			case VEH_AIRCRAFT: return GrfChangeInfoHandler<GSF_AIRCRAFT>::Activation(first, last, prop, reader);
			default: FixtureAbort(__LINE__);
		}
	};
	auto failed = [&](VehicleType type, uint last, int prop, std::span<const uint8_t> bytes) {
		ByteReader reader(bytes.data(), bytes.size());
		try { call(type, 500, last, prop, reader); } catch (const OTTDByteReaderSignal &) { return reader.Remaining(); }
		FixtureAbort(__LINE__);
	};
	const BadgeID old_badge = GetOrCreateBadge("vehicle/old").index, new_badge = GetOrCreateBadge("vehicle/new").index;
	const auto old_list = file->badge_list; file->badge_list = {new_badge};
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		Engine *e = LoadingEngine(1, type, 500), *next = LoadingEngine(1, type, 501);
		const auto old_mask = e->info.callback_mask, next_mask = next->info.callback_mask;
		const auto old_badges = e->badges; e->badges = {old_badge};
		const std::array<uint8_t, 1> one{0xa5};
		const int mask = std::array{0x1e, 0x17, 0x12, 0x14}[type], badges = std::array{0x33, 0x2a, 0x26, 0x24}[type];
		LoadingRequire(failed(type, 502, mask, one) == 0 && e->info.callback_mask.base() == 0x2a5 && next->info.callback_mask == next_mask, "properties-partial-range");
		const std::array<uint8_t, 5> partial_badges{2, 0, 0, 0, 0};
		LoadingRequire(failed(type, 501, badges, partial_badges) == 1 && e->badges == std::vector<BadgeID>{old_badge} && GetBadge(new_badge)->features.Test(static_cast<GrfSpecFeature>(type)), "properties-partial-badges");
		e->badges = old_badges; e->info.callback_mask = old_mask;
		const std::array<uint8_t, 3> order{0xff, 0xf6, 1};
		const int order_prop = std::array{0x1a, 0x20, 0x1b, 0x1b}[type];
		LoadingRequire(failed(type, 502, order_prop, order) == 0, "properties-partial-order");
		CommitVehicleListOrderChanges();
		LoadingRequire(next->list_position < e->list_position && e->list_position < LoadingEngine(1, type, 502)->list_position, "properties-order-published");
		CommitVehicleListOrderChanges();
		LoadingRequire(e->list_position < next->list_position && next->list_position < LoadingEngine(1, type, 502)->list_position, "properties-queue-cleared");
	}
	file->badge_list = old_list;
	current.Restore();
	ReloadNewGRFData();
}
static void LoadingBuild(uint16_t local = 300)
{
	Company *company = *Company::Iterate().begin();
	Backup<CompanyID> current(_current_company, company->index);
#ifdef WITH_RUST
	company->Finances().money = Money{1000000000};
#else
	company->money = Money{1000000000};
#endif
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		TileIndex tile = INVALID_TILE;
		if (type == VEH_AIRCRAFT) {
			for (Station *station : Station::Iterate()) if (station->owner == company->index && station->airport.GetNumHangars() != 0) { tile = station->airport.GetHangarTile(0); break; }
		} else {
			for (Depot *depot : Depot::Iterate()) {
				if (GetTileOwner(depot->xy) != company->index) continue;
				if ((type == VEH_TRAIN && IsRailDepotTile(depot->xy)) || (type == VEH_ROAD && IsRoadDepotTile(depot->xy)) || (type == VEH_SHIP && IsShipDepotTile(depot->xy))) { tile = depot->xy; break; }
			}
		}
		LoadingRequire(tile != INVALID_TILE, "depot");
		Engine *engine = LoadingEngine(1, type, local);
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		auto tested = Command<CMD_BUILD_VEHICLE>::Do({}, tile, engine->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
		SaveRandomSeeds(&after);
		LoadingRequire(std::get<0>(tested).Succeeded() && TypeSameRng(before, after), "build-test");
		auto [cost, id, capacity, mail_capacity, cargoes] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, engine->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
		LoadingRequire(cost.Succeeded() && cost.GetCost() == std::get<0>(tested).GetCost() && capacity == std::get<2>(tested) && mail_capacity == std::get<3>(tested) && Vehicle::Get(id)->engine_type == engine->index, "build-execute");
		fmt::print(stderr, "ENGINE loading-build {} {} {} {} {} {}\n", type, id, engine->index, cost.GetCost(), capacity, mail_capacity);
		const CargoType mail = GetCargoTypeByLabel(CT_MAIL), passengers = GetCargoTypeByLabel(CT_PASSENGERS);
		SaveRandomSeeds(&before);
		const auto original = Vehicle::Get(id)->cargo_type;
		auto excluded = Command<CMD_REFIT_VEHICLE>::Do({}, id, passengers, 0, false, false, 0);
		auto refused = Command<CMD_REFIT_VEHICLE>::Do({}, id, NUM_CARGO, 0, false, false, 0);
		auto refit_test = Command<CMD_REFIT_VEHICLE>::Do({}, id, mail, 0, false, false, 0);
		SaveRandomSeeds(&after);
		LoadingRequire(std::get<0>(refused).Failed() && std::get<0>(excluded).Succeeded() && std::get<1>(excluded) == 0 && std::get<0>(refit_test).Succeeded() && Vehicle::Get(id)->cargo_type == original && TypeSameRng(before, after), "refit-tests");
		auto refit = Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, id, mail, 0, false, false, 0);
		LoadingRequire(std::get<0>(refit).Succeeded() && std::get<0>(refit).GetCost() == std::get<0>(refit_test).GetCost() && std::get<1>(refit) == std::get<1>(refit_test) && Vehicle::Get(id)->cargo_type == mail, "refit-execute");
		fmt::print(stderr, "ENGINE loading-refit {} {} {} {} {}\n", type, id, std::get<0>(refit).GetCost(), std::get<1>(refit), Vehicle::Get(id)->cargo_cap);
		if (local == 500) {
			bool orders = false;
			for (const Vehicle *source : Vehicle::Iterate()) {
				if (source->index == id || source->type != type || source->owner != company->index || !source->IsPrimaryVehicle() || source->GetNumOrders() == 0) continue;
				if (type == VEH_ROAD && (source->cargo_type == passengers) != (Vehicle::Get(id)->cargo_type == passengers)) continue;
				if (Command<CMD_CLONE_ORDER>::Do({}, CO_COPY, id, source->index).Failed()) continue;
				orders = Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_COPY, id, source->index).Succeeded();
				break;
			}
			LoadingRequire(orders, "properties-orders");
			LoadingRequire(Command<CMD_START_STOP_VEHICLE>::Do(DoCommandFlag::Execute, id, false).Succeeded(), "properties-start");
			SaveRandomSeeds(&after);
			fmt::print(stderr, "ENGINE properties-built {} {} {} {} {} {}\n", type, id, after.random.state[0], after.random.state[1], after.interactive_random.state[0], after.interactive_random.state[1]);
		}
	}
	current.Restore();
}
static bool LoadingScenario(std::span<std::string_view> args)
{
	const bool properties = args[1].starts_with("properties"), inspect = args[1].ends_with("inspect");
	if (properties) _interactive_random.SetSeed(0x491);
	const uint16_t local = properties ? 500 : 300;
	if (!inspect) {
		_settings_game.vehicle.dynamic_engines = args[1] != "loading-shared";
		_grfconfig.clear();
		for (uint which : {1, 2, 3, 5}) {
			const GRFConfig *found = FindGRFConfig(0x4700A000 + which, FGCM_NEWEST);
			if (found == nullptr) FixtureAbort(__LINE__);
			_grfconfig.push_back(std::make_unique<GRFConfig>(*found));
		}
		if (properties) {
			const GRFConfig *found = FindGRFConfig(0x4700A006, FGCM_NEWEST);
			if (found == nullptr) FixtureAbort(__LINE__);
			_grfconfig.push_back(std::make_unique<GRFConfig>(*found));
		}
		AppendStaticGRFConfigs(_grfconfig);
		const auto size = Engine::GetPoolSize();
		SavedRandomSeeds before, expected, after;
		SaveRandomSeeds(&before);
		Random(); Random(); SaveRandomSeeds(&expected); RestoreRandomSeeds(before);
		ReloadNewGRFData();
		std::erase_if(_grfconfig, [](const auto &config) { return config->ident.grfid == 0x4700A005; });
		ReloadNewGRFData();
		SaveRandomSeeds(&after);
		LoadingRequire(TypeSameRng(expected, after), "loading-rng");
		LoadingRequire(Engine::GetPoolSize() > size, "pool-growth");
	}
	LoadingInspect();
	if (properties) { if (!inspect) PropertiesReadFailures(); PropertiesInspect(); }
	uint moving = 0;
	if (inspect) {
		for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
			uint count = 0;
			for (const Vehicle *v : Vehicle::Iterate()) {
				if (v->type != type || !v->IsPrimaryVehicle() || v->engine_type != LoadingEngine(1, type, local)->index) continue;
				LoadingRequire(v->cargo_type == GetCargoTypeByLabel(CT_MAIL) && v->cargo_cap > 0, "reloaded-vehicle");
				fmt::print(stderr, "ENGINE loading-vehicle {} {} {} {} {}\n", type, v->index, v->engine_type, v->cargo_type, v->cargo_cap);
				if (properties) {
					moving += !v->IsInDepot() && v->cur_speed != 0;
					fmt::print(stderr, "ENGINE properties-movement {} {} {} {} {}\n", type, v->tile, v->x_pos, v->y_pos, v->cur_speed);
				}
				++count;
			}
			LoadingRequire(count == 1, "reloaded-count");
		}
	}
	if (properties && inspect) LoadingRequire(moving > 0, "properties-movement");
	LoadingBuild(local);
	if (args.size() == 3) LoadingRequire(SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save-eids");
	fmt::print(stderr, "ENGINE loading-complete true\n");
	if (properties) fmt::print(stderr, "ENGINE properties-complete true\n");
	return true;
}

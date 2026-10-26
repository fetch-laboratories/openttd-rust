/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Real FILE-loaded engine definitions, native EIDS reload and construction/refit commands. */
#include "newgrf.h"
#include "newgrf_config.h"
#include "newgrf_engine.h"
#include "newgrf_cargo.h"
#include "newgrf/newgrf_internal_vehicle.h"
#include "vehicle_cmd.h"
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
	for (uint which = 1; which <= 5; ++which) {
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
static void LoadingBuild()
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
		Engine *engine = LoadingEngine(1, type, 300);
		SavedRandomSeeds before, after;
		SaveRandomSeeds(&before);
		auto tested = Command<CMD_BUILD_VEHICLE>::Do({}, tile, engine->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
		SaveRandomSeeds(&after);
		LoadingRequire(std::get<0>(tested).Succeeded() && TypeSameRng(before, after), "build-test");
		auto [cost, id, capacity, mail_capacity, cargoes] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, tile, engine->index, false, INVALID_CARGO, INVALID_CLIENT_ID);
		LoadingRequire(cost.Succeeded() && Vehicle::Get(id)->engine_type == engine->index, "build-execute");
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
		LoadingRequire(std::get<0>(refit).Succeeded() && Vehicle::Get(id)->cargo_type == mail, "refit-execute");
		fmt::print(stderr, "ENGINE loading-refit {} {} {} {} {}\n", type, id, std::get<0>(refit).GetCost(), std::get<1>(refit), Vehicle::Get(id)->cargo_cap);
	}
	current.Restore();
}
static bool LoadingScenario(std::span<std::string_view> args)
{
	if (args[1] != "loading-inspect") {
		_settings_game.vehicle.dynamic_engines = args[1] != "loading-shared";
		_grfconfig.clear();
		for (uint which : {1, 2, 3, 5}) {
			const GRFConfig *found = FindGRFConfig(0x4700A000 + which, FGCM_NEWEST);
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
	if (args[1] == "loading-inspect") {
		for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
			uint count = 0;
			for (const Vehicle *v : Vehicle::Iterate()) {
				if (v->type != type || !v->IsPrimaryVehicle() || v->engine_type != LoadingEngine(1, type, 300)->index) continue;
				LoadingRequire(v->cargo_type == GetCargoTypeByLabel(CT_MAIL) && v->cargo_cap > 0, "reloaded-vehicle");
				fmt::print(stderr, "ENGINE loading-vehicle {} {} {} {} {}\n", type, v->index, v->engine_type, v->cargo_type, v->cargo_cap);
				++count;
			}
			LoadingRequire(count == 1, "reloaded-count");
		}
	}
	LoadingBuild();
	if (args.size() == 3) LoadingRequire(SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save-eids");
	fmt::print(stderr, "ENGINE loading-complete true\n");
	return true;
}

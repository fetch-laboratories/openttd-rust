/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Real Action2 programs through house callbacks, plus native state inputs absent
 * from external-GRF-free saves. Every transient spec/group is restored. */
#include "newgrf_house.h"
#include "newgrf_town.h"
extern void RebuildTownCaches();
static void HouseResolverScenario(const std::vector<std::pair<TileIndex, HouseID>> &sites)
{
	auto require = [](bool condition) { if (!condition) std::abort(); };
#ifdef WITH_RUST
	auto reset_classes = [] {
		openttd_rust_house_resolver_exchange(&GetHouseResolverServices());
		openttd_rust_house_classes_reset();
	};
	auto allocate_class = openttd_rust_house_class_allocate;
#else
	auto reset_classes = ResetHouseClassIDs;
	auto allocate_class = AllocateHouseClassID;
#endif
	auto origin = std::ranges::find_if(sites, [](const auto &s) { return HouseSpec::Get(s.second)->building_flags.Test(BuildingFlag::Size2x2); });
	require(origin != sites.end());
	const TileIndex tile = origin->first;
	Town *town = Town::GetByTile(tile);
	const HouseID old_size = static_cast<HouseID>(HouseSpec::Specs().size());
	const HouseID first = old_size + 3;
	std::array<TileIndex, 4> parts{tile, tile + TileDiffXY(0, 1), tile + TileDiffXY(1, 0), tile + TileDiffXY(1, 1)};
	std::array<HouseID, 4> old_ids;
	std::array<EntityIDMapping, 4> old_mappings;
	GRFFile grf, foreign; grf.grfid = 0x40912345; grf.grf_version = 8; foreign.grfid = 0x40954321; foreign.grf_version = 7;
	const CargoType cargo = GetCargoTypeByLabel(CT_PASSENGERS);
	require(IsValidCargoType(cargo)); grf.cargo_list = {CT_PASSENGERS};
	reset_classes();
	require(allocate_class(42, grf.grfid) == 1 && allocate_class(42, foreign.grfid) == 2 && allocate_class(42, grf.grfid) == 1);
	reset_classes();
	require(allocate_class(7, foreign.grfid) == 1 && allocate_class(42, grf.grfid) == 2);
	HouseSpec::Specs().resize(first + 4);
	for (size_t i = 0; i < parts.size(); ++i) {
		old_ids[i] = GetHouseType(parts[i]);
		old_mappings[i] = _house_mngr.mappings[first + i];
		_house_mngr.mappings[first + i] = {.grfid = grf.grfid, .entity_id = static_cast<uint16_t>(0x45 + i), .substitute_id = old_ids[i]};
		*HouseSpec::Get(first + i) = *HouseSpec::Get(old_ids[i]);
		auto *spec = HouseSpec::Get(first + i);
		spec->grf_prop.grffile = &grf; spec->grf_prop.grfid = grf.grfid; spec->grf_prop.local_id = 0x345 + i; spec->class_id = 2;
		SetHouseType(parts[i], first + i);
	}
	RebuildTownCaches();
	fmt::print(stderr, "HOUSE_RESOLVER class-reset true\n");
	CallbackResultSpriteGroup result(7), error(0x7FFE);
	DeterministicSpriteGroup program;
	program.size = DSG_SIZE_DWORD; program.var_scope = VSG_SCOPE_SELF;
	program.default_result.group = &result; program.error_group = &error;
	int32_t offset = 0;
	auto run = [&](const char *label, CallbackID callback, uint8_t variable, uint8_t parameter, bool prospective, TileIndex where, int view = 0, VarSpriteGroupScope scope = VSG_SCOPE_SELF) {
		program.var_scope = scope;
		program.adjusts = {
			{.operation = DSGA_OP_RST, .variable = variable, .parameter = parameter, .and_mask = UINT32_MAX},
			{.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101},
		};
		if (variable == 0x64) program.adjusts.insert(program.adjusts.begin(), {
			{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = static_cast<uint32_t>(offset)},
			{.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100},
		});
		HouseSpec::Get(first)->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, &program);
		HouseSpec::Get(first)->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, &program);
		std::array<int32_t, 2> registers{};
		uint16_t cb = GetHouseCallback(callback, 0x1234, 0x5678, first, town, where, registers, prospective, 0xA5, CargoTypes{1} << cargo, view);
		fmt::print(stderr, "HOUSE_RESOLVER {} {:X} {} {} {} {}\n", label, variable, parameter, cb, registers[0], registers[1]);
		require(cb == 7 || cb == 0x7FFE);
		return static_cast<uint32_t>(registers[1]);
	};
	for (uint8_t var : {0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x60, 0x61, 0x62, 0x63, 0x66, 0x67}) {
		run("construction", CBID_HOUSE_ALLOW_CONSTRUCTION, var, 0, true, tile);
		run("production", CBID_HOUSE_PRODUCE_CARGO, var, 0, false, tile);
		run("prototype", CBID_HOUSE_ALLOW_CONSTRUCTION, var, 0, true, INVALID_TILE, 5);
	}
	require(run("local-mapping", CBID_HOUSE_PRODUCE_CARGO, 0x61, 0x45, false, tile) == 0x04040101);
	HouseSpec::Get(first + 1)->grf_prop.grffile = &foreign;
	run("foreign-house", CBID_HOUSE_PRODUCE_CARGO, 0x66, 0x10, false, tile);
	HouseSpec::Get(first + 1)->grf_prop.grffile = nullptr;
	run("missing-house-file", CBID_HOUSE_PRODUCE_CARGO, 0x66, 0x10, false, tile);
	require(run("missing-house-grfid", CBID_HOUSE_PRODUCE_CARGO, 0x67, 0x10, false, tile) == grf.grfid);
	HouseSpec::Get(first + 1)->grf_prop.grffile = &grf;
	/* Construction checks run over unrelated houses; random bits/triggers must
	 * use the supplied prospective state rather than the existing map fields. */
	RandomizedSpriteGroup random;
	random.var_scope = VSG_SCOPE_SELF; random.lowest_randbit = 0; random.triggers = 1;
	CallbackResultSpriteGroup zero(10), one(11); random.groups = {&zero, &one};
	HouseSpec::Get(first)->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, &random);
	require(GetHouseCallback(CBID_HOUSE_ALLOW_CONSTRUCTION, 0, 0, first, town, tile, {}, true, 0xA5) == 11);
	HouseSpec::Get(first)->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, &random);
	HouseResolverObject rerandom(first, tile, town, CBID_RANDOM_TRIGGER);
	rerandom.SetWaitingRandomTriggers(HouseRandomTriggers{1}); rerandom.ResolveRerandomisation();
	fmt::print(stderr, "HOUSE_RESOLVER random {} {} true\n", rerandom.GetReseedSum(), rerandom.GetUsedRandomTriggers().base());
	/* No other house shares this file: all four parts of the origin must be
	 * excluded, even when queried from the south-east part. */
	for (size_t i = 0; i < parts.size(); ++i) {
		HouseResolverObject query(first + i, parts[i], town);
		bool available = true;
		uint32_t distance = query.GetScope()->GetVariable(0x65, 0x83, available);
		require(available && distance == 0);
	}
	run("origin-exclusion", CBID_HOUSE_PRODUCE_CARGO, 0x65, 0x83, false, tile);
	for (TileIndex nearby : SpiralTileSequence(tile, 9)) {
		if (!IsTileType(nearby, MP_HOUSE) || std::ranges::find(parts, nearby) != parts.end()) continue;
		HouseID old = GetHouseType(nearby); SetHouseType(nearby, first);
		require(run("nearest-match", CBID_HOUSE_PRODUCE_CARGO, 0x65, 0x84, false, tile) == DistanceManhattan(tile, nearby));
		SetHouseType(nearby, old); break;
	}
	town->cache.building_counts.id_count[first] = UINT16_MAX;
	town->cache.building_counts.class_count[2] = UINT16_MAX;
	require(run("count-clamps", CBID_HOUSE_PRODUCE_CARGO, 0x44, 0, false, tile) == 0x04FF01FF);

	/* Ordered StationFinder must OR canonical acceptance histories. It is
	 * call-local, independent of TileLoop_Town's static finder. */
	std::vector<Station *> stations;
	for (uint8_t state : {4, 8, 16, 32}) {
		require(Station::CanAllocateItem());
		Station *station = new Station(tile); _station_kdtree.Insert(station->index); station->town = town; station->owner = OWNER_NONE;
		station->goods[cargo].status = GoodsEntry::States{state};
		station->catchment_tiles.Initialize(TileArea(tile, 2, 2));
		for (TileIndex part : parts) station->catchment_tiles.SetTile(part);
		town->stations_near.insert(station); stations.push_back(station);
	}
	offset = 0;
	require(run("acceptance-history", CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED, 0x64, 0, false, tile) == 31);
	offset = 0xFFFF;
	require(run("signed-offset", CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED, 0x64, 0, false, parts[3]) == 31);
	run("invalid-cargo", CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED, 0x64, 255, false, tile);
	for (auto *station : stations) { town->stations_near.erase(station); delete station; }
	/* Preserve signed ratings, high-byte aliases, transported percentages,
	 * native history periods and null/unavailable parent behavior. */
	const auto old_rating = town->ratings[0]; town->ratings[0] = -1234;
	const auto old_received = town->received; const auto old_population = town->cache.population;
	town->cache.population = UINT32_MAX;
	town->received[TAE_FOOD].new_act = 0xABCD; town->received[TAE_FOOD].old_act = 0x5678;
	town->received[TAE_WATER].new_act = 0x2345; town->received[TAE_WATER].old_act = 0xFEDC;

	for (CargoLabel label : {CT_PASSENGERS, CT_MAIL}) {
		CargoType type = GetCargoTypeByLabel(label); if (!IsValidCargoType(type)) continue;
		auto &supply = town->GetOrCreateCargoSupplied(type);
		supply.history[THIS_MONTH].production = 100000; supply.history[THIS_MONTH].transported = 45678;
		supply.history[LAST_MONTH].production = 456; supply.history[LAST_MONTH].transported = 123;
	}

	for (uint8_t var : {0x40, 0x41, 0x80, 0x81, 0x82, 0x83, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x9C, 0x9D, 0x9E, 0x9F, 0xAE, 0xB2, 0xB6, 0xB9, 0xBA, 0xBB, 0xBC, 0xBD, 0xBE, 0xBF, 0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xFF}) {
		run("parent", CBID_HOUSE_ALLOW_CONSTRUCTION, var, 0, true, tile, 0, VSG_SCOPE_PARENT);
	}
	town->ratings[0] = old_rating; town->received = old_received; town->cache.population = old_population;
	HouseResolverObject future(first, tile, town, CBID_HOUSE_ALLOW_CONSTRUCTION, 0, 0, true);
	const size_t psa_count = town->psa_list.size(); future.GetScope(VSG_SCOPE_PARENT)->StorePSA(0, 409);
	require(psa_count == town->psa_list.size());
	program.var_scope = VSG_SCOPE_PARENT;
	program.adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 409}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 3}};
	HouseSpec::Get(first)->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, &program);
	require(GetHouseCallback(CBID_HOUSE_ALLOW_CONSTRUCTION, 0, 0, first, town, tile, {}, true) == 7 && psa_count == town->psa_list.size());
	TownResolverObject null_town(&grf, nullptr, true); bool available = true;
	require(null_town.GetScope()->GetVariable(0x41, 0, available) == UINT32_MAX && !available);
	fmt::print(stderr, "HOUSE_RESOLVER parent-readonly-null true\n");
	for (size_t i = 0; i < parts.size(); ++i) { SetHouseType(parts[i], old_ids[i]); _house_mngr.mappings[first + i] = old_mappings[i]; }
	HouseSpec::Specs().resize(old_size); reset_classes(); RebuildTownCaches();
	fmt::print(stderr, "HOUSE_RESOLVER construction-production-var44-var64-var65 true\n");
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Subsidy control boundaries and actual final-delivery payments on the stock save. */
#include "news_func.h"
#include "vehicle-record-fixture.hpp"
#include "strings_func.h"
#include "table/strings.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "window_func.h"
#include "subsidy_base.h"
#include "subsidy_func.h"
#include "subsidy_cmd.h"
#include "industry.h"
#include "town.h"
#include "town_map.h"
#include "station_base.h"
#include "economy_base.h"
#include "economy_func.h"
#include "timer/timer_game_economy.h"
#ifdef WITH_RUST
#include "rust/subsidy_ffi.h"
#include "rust/clock_ffi.h"
#include "rust/services_station.h"
static void SubsidyMonthlyLoop() { openttd_rust_clock_subsidy_monthly(&GetRustSubsidyServices(), nullptr); }
#else
bool FindSubsidyPassengerRoute();
bool FindSubsidyTownCargoRoute();
bool FindSubsidyIndustryCargoRoute();
#include "subsidy-monthly-reference.inc"
#endif
static void SubsidyClean() { for (Subsidy *s : Subsidy::Iterate()) delete s; RebuildSubsidisedSourceAndDestinationCache(); }
static Subsidy *FixtureSubsidy(CargoType cargo, Source src, Source dst, uint16_t remaining) { if (!Subsidy::CanAllocateItem()) FixtureAbort(__LINE__); return new Subsidy(cargo, src, dst, remaining); }
static bool SameRandom(const Randomizer &r) { return r.state[0] == _random.state[0] && r.state[1] == _random.state[1]; }
static Randomizer SubsidyChance(uint target) { for (uint seed = 1;; ++seed) { _random.SetSeed(seed); Randomizer r = _random; if (r.Next(16) == target) return r; } }
static bool SubsidyScenario(std::span<std::string_view> args)
{
	if (args[1] == "subsidies-inspect") { bool loaded = false; for (Subsidy *s : Subsidy::Iterate()) loaded |= s->remaining == 54321 && s->IsAwarded(); fmt::print(stderr, "SUBSIDY loaded {}\n", loaded); return true; }
	AutoRestoreBackup<Randomizer> interactive_seed(_interactive_random, _interactive_random);
	_interactive_random.SetSeed(12345);
	SubsidyClean();
	Vehicle *front = nullptr; for (Vehicle *v : Vehicle::Iterate()) if (FIXTURE_GET(v, Type, type) == VEH_TRAIN && v->IsPrimaryVehicle() && v->cargo_payment == nullptr) { front = v; break; }
	if (front == nullptr || Town::GetNumItems() < 2 || Industry::GetNumItems() < 2) FixtureAbort(__LINE__);
	CargoType cargo = CargoSpec::town_production_cargoes[TPE_PASSENGERS][0]->Index();
	/* The loaded catchment of a passenger-accepting station covers town a's houses
	 * and lists industry j; town b and industry i lie outside it. */
	Station *st = nullptr; Town *a = nullptr; Industry *j = nullptr;
	for (Station *candidate : Station::Iterate()) {
		if (st != nullptr || !HasBit(candidate->always_accepted, cargo) || candidate->industries_near.begin() == candidate->industries_near.end()) continue;
		BitmapTileIterator it(candidate->catchment_tiles);
		for (TileIndex tile = it; tile != INVALID_TILE; tile = ++it) {
			if (!IsTileType(tile, MP_HOUSE)) continue;
			st = candidate; a = Town::GetByTile(tile); j = candidate->industries_near.begin()->industry; break;
		}
	}
	if (st == nullptr) FixtureAbort(__LINE__);
	auto covers_town = [st](const Town *t) { BitmapTileIterator it(st->catchment_tiles); for (TileIndex tile = it; tile != INVALID_TILE; tile = ++it) if (IsTileType(tile, MP_HOUSE) && GetTownIndex(tile) == t->index) return true; return false; };
	auto lists = [st](const Industry *ind) { for (const auto &entry : st->industries_near) if (entry.industry == ind) return true; return false; };
	Town *b = nullptr; for (Town *t : Town::Iterate()) if (b == nullptr && !covers_town(t)) b = t;
	Industry *i = nullptr; for (Industry *ind : Industry::Iterate()) if (i == nullptr && !lists(ind)) i = ind;
	if (b == nullptr || i == nullptr) FixtureAbort(__LINE__);
	Backup<CompanyID> current(_current_company, OWNER_DEITY);
	Backup<FIXTURE_FIELD_TYPE(front, last_station_visited)> last(FIXTURE_FIELD(front, last_station_visited), FixtureRaw(st->index));
	fmt::print(stderr, "SUBSIDY nearby true\n");
	Source town{a->index, SourceType::Town}, other{b->index, SourceType::Town}, industry{i->index, SourceType::Industry}, destination{j->index, SourceType::Industry};
	bool commands = true; for (Source src : {town, industry}) for (Source dst : {town, destination}) { commands &= CmdCreateSubsidy({}, cargo, src, dst).Succeeded(); commands &= CmdCreateSubsidy(DoCommandFlag::Execute, cargo, src, dst).Succeeded(); }
	commands &= CmdCreateSubsidy(DoCommandFlag::Execute, cargo, town, town).Succeeded(); // duplicates/self routes remain legal
	bool refusals = CmdCreateSubsidy({}, INVALID_CARGO, town, other).Failed() && CmdCreateSubsidy({}, cargo, {Source::Invalid, SourceType::Town}, other).Failed();
	current.Change(FIXTURE_GET(front, Owner, owner)); refusals &= CmdCreateSubsidy({}, cargo, town, other).Failed(); current.Change(OWNER_DEITY);
	fmt::print(stderr, "SUBSIDY commands {} {}\n", commands, refusals);
	bool classes = CheckSubsidised(cargo, FIXTURE_GET(front, Owner, owner), town, st) && CheckSubsidised(cargo, FIXTURE_GET(front, Owner, owner), industry, st);
	bool rival = !CheckSubsidised(cargo, CompanyID((FIXTURE_GET(front, Owner, owner).base() + 1) % 15), town, st);
	fmt::print(stderr, "SUBSIDY classes {} {}\n", classes, rival);
	bool multipliers = true;
	for (uint8_t multiplier = 0; multiplier < 4; ++multiplier) {
		SubsidyClean(); _settings_game.difficulty.subsidy_multiplier = multiplier;
		for (Source dst : {town, other, industry, destination}) CmdCreateSubsidy(DoCommandFlag::Execute, cargo, town, dst);
		if (!CargoPacket::CanAllocateItem() || !CargoPayment::CanAllocateItem()) FixtureAbort(__LINE__);
		CargoPacket *cp = new CargoPacket(st->index, 10, town); cp->UpdateLoadingTile(st->xy); TileIndex tile = TileXY((TileX(st->xy) + 10) % Map::SizeX(), TileY(st->xy));
		CargoPayment *payment = new CargoPayment(front); Money base = GetTransportedGoodsIncome(10, cp->GetDistance(tile), cp->GetPeriodsInTransit(), cargo);
		payment->PayFinalDelivery(cargo, cp, 10, tile);
		Money expected = multiplier == 0 ? base + (base >> 1) : base * (multiplier + 1);
		multipliers &= base > 0 && payment->route_profit == expected;
		for (Subsidy *s : Subsidy::Iterate()) multipliers &= (s->awarded == FIXTURE_GET(front, Owner, owner)) == (s->dst == town || s->dst == destination);
		payment->route_profit = payment->visual_profit = payment->visual_transfer = 0; delete payment; delete cp;
	}
	fmt::print(stderr, "SUBSIDY multipliers {}\n", multipliers);
	SubsidyClean(); _settings_game.difficulty.subsidy_duration = 65535; Subsidy *s = FixtureSubsidy(cargo, town, town, 0); RebuildSubsidisedSourceAndDestinationCache(); CheckSubsidised(cargo, FIXTURE_GET(front, Owner, owner), town, st);
	#ifdef WITH_RUST
	if (&s->cargo_type != &s->rust_state->cargo_type || &s->remaining != &s->rust_state->remaining || &s->awarded != &s->rust_state->awarded || &s->src != &s->rust_state->src || &s->dst != &s->rust_state->dst) FixtureAbort(__LINE__);
#endif
	bool wrap = s->remaining == uint16_t(65535U * 12); s->remaining = 0; _settings_game.difficulty.subsidy_duration = 0; RebuildSubsidisedSourceAndDestinationCache(); SubsidyMonthlyLoop(); wrap &= s->remaining == 65535;
	fmt::print(stderr, "SUBSIDY wrap {}\n", wrap);
	Randomizer unchanged = _random; SubsidyMonthlyLoop(); bool disabled = SameRandom(unchanged); s->remaining = 1; FixtureSubsidy(cargo, town, other, 1); Randomizer expected = SubsidyChance(4); SubsidyMonthlyLoop();
	bool expired = Subsidy::GetNumItems() == 0 && SameRandom(expected);
	_settings_game.difficulty.subsidy_duration = 1; _settings_game.linkgraph.distribution_pax = _settings_game.linkgraph.distribution_mail = _settings_game.linkgraph.distribution_armoured = _settings_game.linkgraph.distribution_default = DT_ASYMMETRIC;
	unchanged = _random; SubsidyMonthlyLoop(); disabled &= SameRandom(unchanged); FixtureSubsidy(cargo, town, other, 1); expected = SubsidyChance(4); SubsidyMonthlyLoop(); expired &= Subsidy::GetNumItems() == 0 && SameRandom(expected);
	fmt::print(stderr, "SUBSIDY disabled {} {}\n", disabled, expired);
	_settings_game.difficulty.subsidy_duration = 1;
	_settings_game.linkgraph.distribution_pax = _settings_game.linkgraph.distribution_mail = _settings_game.linkgraph.distribution_armoured = _settings_game.linkgraph.distribution_default = DT_MANUAL;
	while (Subsidy::CanAllocateItem()) { FixtureSubsidy(cargo, town, other, 12); } RebuildSubsidisedSourceAndDestinationCache();
	bool retries = true; for (uint chance : {0U, 2U, 3U}) { expected = SubsidyChance(chance); SubsidyMonthlyLoop(); retries &= SameRandom(expected); }
	fmt::print(stderr, "SUBSIDY retries {}\n", retries); SubsidyClean();
	_settings_game.linkgraph.distribution_pax = _settings_game.linkgraph.distribution_mail = _settings_game.linkgraph.distribution_armoured = _settings_game.linkgraph.distribution_default = DT_MANUAL;
	std::vector<std::pair<Town *, uint32_t>> populations; std::vector<std::pair<uint32_t *, uint32_t>> histories;
	for (Town *t : Town::Iterate()) { populations.emplace_back(t, t->cache.population); t->cache.population = 0; for (auto &entry : t->supplied) { auto &count = entry.history[LAST_MONTH].transported; histories.emplace_back(&count, count); count = 0; } }
	SubsidyChance(0); SubsidyMonthlyLoop(); fmt::print(stderr, "SUBSIDY retry-rng {} {}\n", _random.state[0], _random.state[1]);
	for (auto [t, value] : populations) { t->cache.population = 2000; }
	bool found[3]{}; uint index = 0;
	for (uint chance : {0U, 2U, 3U}) {
		SubsidyChance(chance); auto before = Subsidy::GetNumItems();
		SubsidyMonthlyLoop(); found[index++] = Subsidy::GetNumItems() > before;
		fmt::print(stderr, "SUBSIDY random-route {} {} {} {} {}\n", chance, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
		for (const Subsidy *offer : Subsidy::Iterate()) fmt::print(stderr, "SUBSIDY random-offer {} {} {} {} {} {} {}\n", chance, offer->index.base(), offer->cargo_type, to_underlying(offer->src.type), offer->src.id, to_underlying(offer->dst.type), offer->dst.id);
	}
	for (auto [field, value] : histories) { *field = value; } for (auto [t, value] : populations) { t->cache.population = value; }
	fmt::print(stderr, "SUBSIDY routes {} {} {}\n", found[0], found[1], found[2]); SubsidyClean();
	if (!Company::CanAllocateItem()) { FixtureAbort(__LINE__); } Company *empty = new Company(); s = FixtureSubsidy(cargo, industry, other, 12); s->awarded = empty->index; RebuildSubsidisedSourceAndDestinationCache(); ChangeOwnershipOfCompanyItems(empty->index, FIXTURE_GET(front, Owner, owner)); bool transfer = s->awarded == FIXTURE_GET(front, Owner, owner);
	s->awarded = empty->index; ChangeOwnershipOfCompanyItems(empty->index, INVALID_OWNER); bool removed = Subsidy::GetNumItems() == 0; delete empty;
	s = FixtureSubsidy(cargo, industry, other, 12); RebuildSubsidisedSourceAndDestinationCache();
#ifdef WITH_RUST
	openttd_rust_subsidy_delete_with(&GetRustSubsidyServices(), industry);
#else
	DeleteSubsidyWith(industry);
#endif
	removed &= Subsidy::GetNumItems() == 0;
	fmt::print(stderr, "SUBSIDY transfer {} {}\n", transfer, removed);
	s = FixtureSubsidy(cargo, town, other, 54321); s->awarded = FIXTURE_GET(front, Owner, owner); RebuildSubsidisedSourceAndDestinationCache();
	last.Restore(); current.Restore();
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	return true;
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport-resolver-console.hpp Actual transient Action2 programs on both games. */
#include "newgrf_airporttiles.h"
static AirportSpec *AirportFixtureBaseSpec(uint8_t type)
{
#ifdef WITH_RUST
	return AirportSpec::RustRawSpec(type);
#else
	return AirportSpec::GetWithoutOverride(type);
#endif
}
static void StationAirportFixture(TileIndex origin, DoCommandFlags test, DoCommandFlags exec, bool reloaded = false)
{
	GRFFile file; file.badge_list = {BadgeID{0}, BadgeID{1}}; file.grfid = 0x41503418; file.grf_version = 8;
	auto *spec = AirportFixtureBaseSpec(AT_SMALL); AirportSpec saved = *spec;
	spec->badges = {BadgeID{0}}; spec->grf_prop.grffile = &file; spec->grf_prop.grfid = file.grfid; spec->grf_prop.local_id = 41;
	auto gfx = spec->layouts[0].tiles.front().gfx;
	auto *ats = const_cast<AirportTileSpec *>(AirportTileSpec::Get(gfx)); AirportTileSpec saved_tile = *ats;
	ats->badges = {BadgeID{1}}; ats->grf_prop.grffile = &file; ats->grf_prop.grfid = file.grfid; ats->grf_prop.local_id = 18;
	StationNeed(SpriteGroup::CanAllocateItem(6), __LINE__);
	auto *result = new CallbackResultSpriteGroup(3), *other = new CallbackResultSpriteGroup(4);
	auto *parent = new DeterministicSpriteGroup(); parent->var_scope = VSG_SCOPE_PARENT; parent->size = DSG_SIZE_DWORD;
	parent->default_result = {true, nullptr};
	parent->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x0C, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 9}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0x418}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 7}, {.operation = DSGA_OP_RST, .variable = 0x7C, .parameter = 7, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	auto *det = new DeterministicSpriteGroup(); det->var_scope = VSG_SCOPE_SELF; det->size = DSG_SIZE_DWORD; det->default_result = {false, result};
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x7E, .and_mask = UINT32_MAX, .subroutine = parent}, {.operation = DSGA_OP_RST, .variable = 0x60, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101}, {.operation = DSGA_OP_RST, .variable = 0x61, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x102}, {.operation = DSGA_OP_RST, .variable = 0x62, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x103}};
	if (reloaded) ats->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, det);
	ats->animation.triggers.Set(AirportAnimationTrigger::Built); ats->animation.speed = 0; ats->animation.frames = 8;
	ats->callback_mask.Set({AirportTileCallbackMask::AnimationSpeed, AirportTileCallbackMask::AnimationNextFrame});
	Station *st = nullptr;
	if (reloaded) { for (Station *s : Station::Iterate()) if (s->name == "Airport resolver") st = s; StationNeed(st != nullptr, __LINE__); }
	else {
		auto check = [&](const char *label, DoCommandFlags flags, TileIndex tile, uint8_t layout, bool success) { auto cost = Command<CMD_BUILD_AIRPORT>::Do(flags, tile, AT_SMALL, layout, NEW_STATION, true); StationCost(label, cost); StationNeed(cost.Succeeded() == success, __LINE__); };
		TileIndex tile = origin + TileDiffXY(3, 3);
		for (bool enabled : {false, true}) for (int offset : {-1, 0, 1}) for (bool never : {false, true}) {
			spec->enabled = enabled; spec->min_year = TimerGameCalendar::year + offset; spec->max_year = TimerGameCalendar::year - 1; _settings_game.station.never_expire_airports = never;
			for (DoCommandFlags flags : {test, exec}) check("airport-admission", flags, tile, 0, enabled && offset <= 0 && never);
			if (enabled && offset <= 0 && never) { StationNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(exec, tile).Succeeded(), __LINE__); }
		}
		spec->enabled = true; spec->min_year = TimerGameCalendar::year; spec->max_year = TimerGameCalendar::year; _settings_game.station.never_expire_airports = false;
		for (Direction rotation : {DIR_N, DIR_E, DIR_S, DIR_W}) {
			spec->layouts[0].rotation = rotation;
			for (uint8_t layout : {0, 1, 255}) for (uint x : {Map::SizeX() - 5, Map::SizeX() - 4, Map::SizeX() - 3}) {
				TileIndex edge = TileXY(x, Map::SizeY() - 5); fmt::print(stderr, "STCMD airport-bounds {} {} {} {}\n", rotation, layout, x, spec->IsWithinMapBounds(layout, edge));
				if (!spec->IsWithinMapBounds(layout, edge)) for (DoCommandFlags flags : {test, exec}) check("airport-bounds-reject", flags, edge, layout, false);
			}
		}
		spec->layouts[0].rotation = saved.layouts[0].rotation;
		/* Temporary admission builds have no PSA; install the program for the
		 * retained airport whose storage is saved and reloaded. */
		ats->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, det);
		check("airport-build-test", test, tile, 0, true); check("airport-build", exec, tile, 0, true);
		st = Station::GetByTile(tile); st->name = "Airport resolver";
		StationNeed(st->airport.psa != nullptr && st->airport.psa->GetValue(7) == 0x418, __LINE__);
	}
	TileIndex tile = st->airport.tile + ToTileIndexDiff(spec->layouts[0].tiles.front().ti);
	/* A real Built event after reload reinstalls the transient program. This is
	 * runtime evidence, not proof of loading an external GRF file. */
	StationNeed(TriggerAirportTileAnimation(st, tile, AirportAnimationTrigger::Built), __LINE__);
	AirportTileResolverObject callback(ats, tile, st, CBID_AIRPTILE_ANIMATION_TRIGGER);
	std::array<int32_t, 4> registers{}; StationNeed(callback.ResolveCallback(registers) == 3 && registers[0] == 0x418, __LINE__);
	StationNeed(st->airport.psa->GetValue(9) == CBID_AIRPTILE_ANIMATION_TRIGGER, __LINE__);
	fmt::print(stderr, "STCMD airport-built {} {} {} {} {}\n", reloaded, registers[0], registers[1], registers[2], registers[3]);
	result->result = 0; AnimateAirportTile(tile); StationNeed(st->airport.psa->GetValue(9) == CBID_AIRPTILE_ANIMATION_NEXT_FRAME, __LINE__); result->result = 3; fmt::print(stderr, "STCMD airport-animation {} {}\n", GetAnimationFrame(tile), st->airport.psa->GetValue(7));

	/* A real parent-scope zero write must not allocate airport storage. */
	auto *persistent = st->airport.psa; st->airport.psa = nullptr; auto program = det->adjusts;
	det->var_scope = VSG_SCOPE_PARENT;
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 17}, {.operation = DSGA_OP_RST, .variable = 0x7C, .parameter = 17, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	StationNeed(callback.ResolveCallback(registers) == 3 && registers[0] == 0 && st->airport.psa == nullptr, __LINE__);
	st->airport.psa = persistent; det->adjusts = std::move(program); det->var_scope = VSG_SCOPE_SELF;
	fmt::print(stderr, "STCMD airport-storage-zero no-allocation\n");
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, det); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, other);
	/* Native direct availability and actual deterministic Action2 agree. Logs,
	 * output registers and the final semantic saves are compared across games. */
	auto variables = [&](const char *label, ResolverObject &object, ScopeResolver &scope) {
		for (uint var = 0x40; var <= 0xFA; ++var) for (uint8_t parameter : {0, 1, 7, 0x10, 0xFF}) {
			bool available = true; auto value = scope.GetVariable(var, parameter, available);
			det->adjusts = {{.operation = DSGA_OP_RST, .variable = static_cast<uint8_t>(var), .parameter = parameter, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
			std::array<int32_t, 1> regs{}; auto cb = object.ResolveCallback(regs);
			if (available) StationNeed(cb == 3 && static_cast<uint32_t>(regs[0]) == value, __LINE__);
			fmt::print(stderr, "STCMD airport-variable {} {:x} {} {} {} {} {}\n", label, var, parameter, available, value, cb, regs[0]);
		}
	};
	for (uint8_t version : {7, 8}) {
		file.grf_version = version; AirportResolverObject airport(tile, st, spec, st->airport.layout); AirportTileResolverObject tiles(ats, tile, st);
		variables("airport", airport, airport.airport_scope); variables("tile", tiles, tiles.tiles_scope);
	}
	file.grf_version = 8;
	/* Null purchase roots, layout/badges before the null gate, station parameter
	 * narrowing and the signed build-date clamp. */
	AirportResolverObject live_root(tile, st, spec, 0), purchase_root(tile, nullptr, spec, 0);
	StationNeed(live_root.ResolveCallback({}) == 3 && purchase_root.ResolveCallback({}) == 4, __LINE__);
	fmt::print(stderr, "STCMD airport-roots live-default purchase\n");
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, det);
	AirportResolverObject preview(tile, nullptr, spec, 2), nowhere(INVALID_TILE, nullptr, spec, 3);
	variables("preview", preview, preview.airport_scope); variables("nowhere", nowhere, nowhere.airport_scope);
	for (int32_t date : {0, CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR.base() - 1, CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR.base() + 65536}) {
		auto old = st->build_date; st->build_date = TimerGameCalendar::Date(date); AirportResolverObject query(tile, st, spec, 0); bool ok = true;
		fmt::print(stderr, "STCMD airport-date {} {}\n", date, query.airport_scope.GetVariable(0xFA, 0, ok)); st->build_date = old;
	}

	AirportResolverObject narrow(tile, st, spec, 0);
	for (uint8_t var : {0x60, 0x61, 0x64, 0x69, 0x8C}) for (uint32_t parameter : {0U, 1U, 255U, 256U, 257U, UINT32_MAX}) {
		bool ok = true; auto value = narrow.airport_scope.GetVariable(var, parameter, ok);
		det->adjusts = {{.operation = DSGA_OP_RST, .variable = var, .parameter = static_cast<uint8_t>(parameter), .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
		std::array<int32_t, 1> regs{}; auto cb = narrow.ResolveCallback(regs); if (ok) StationNeed(cb == 3 && static_cast<uint32_t>(regs[0]) == value, __LINE__);
		fmt::print(stderr, "STCMD airport-parameter {} {} {} {}\n", var, parameter, ok, value);
	}
	AirportResolverObject airport(tile, st, spec, 0); auto *town = airport.GetTown(); StationNeed(town != nullptr, __LINE__);
	auto *original_town = st->town; st->town = nullptr; StationNeed(airport.GetTown() == town, __LINE__);
	AirportResolverObject retry(tile, st, spec, 0); StationNeed(retry.GetTown() == nullptr, __LINE__); st->town = original_town;
#ifdef WITH_RUST
	StationNeed(retry.GetTown() != nullptr && preview.GetTown()->context.readonly && nowhere.GetTown() == nullptr, __LINE__);
#else
	StationNeed(retry.GetTown() != nullptr && preview.GetTown()->readonly && nowhere.GetTown() == nullptr, __LINE__);
#endif
	det->var_scope = VSG_SCOPE_PARENT; det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0x418}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 7}, {.operation = DSGA_OP_RST, .variable = 0x7C, .parameter = 7, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	std::array<int32_t, 1> town_regs{}; StationNeed(airport.ResolveCallback(town_regs) == 3 && town_regs[0] == 0x418, __LINE__);
	det->adjusts[2].and_mask = 0x419; StationNeed(preview.ResolveCallback(town_regs) == 3 && town_regs[0] == 0x418, __LINE__);
	StationNeed(nowhere.ResolveCallback({}) == CALLBACK_FAILED, __LINE__);
	det->var_scope = VSG_SCOPE_RELATIVE; StationNeed(airport.ResolveCallback({}) == CALLBACK_FAILED, __LINE__);
	fmt::print(stderr, "STCMD airport-town readonly retry default {}\n", town_regs[0]); det->var_scope = VSG_SCOPE_SELF;
	/* Actual randomized groups select station and tile-packed bits without RNG. */
	auto *random = new RandomizedSpriteGroup(); random->var_scope = VSG_SCOPE_SELF; random->groups = {result, other};
	auto rng = _random;
	for (uint8_t bit : {0, 7, 16, 17}) {
		random->lowest_randbit = bit; ats->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, random); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, random);
		AirportResolverObject a(tile, st, spec, 0); AirportTileResolverObject t(ats, tile, st);
		fmt::print(stderr, "STCMD airport-random {} {} {} {} {}\n", bit, a.airport_scope.GetRandomBits(), t.tiles_scope.GetRandomBits(), a.ResolveCallback({}), t.ResolveCallback({}));
	}
	StationNeed(rng.state[0] == _random.state[0] && rng.state[1] == _random.state[1], __LINE__);
	ats->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, det); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, det);
	/* IDs: nonmember, original, overridden same/foreign, custom with/without
	 * groups. Restore tile/spec metadata before saving. */
	auto *custom = const_cast<AirportTileSpec *>(AirportTileSpec::Get(NEW_AIRPORTTILE_OFFSET)); AirportTileSpec saved_custom = *custom;
	custom->grf_prop = ats->grf_prop; custom->grf_prop.subst_id = 12; custom->grf_prop.override_id = INVALID_AIRPORTTILE;
	det->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x62, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}};
	for (bool foreign : {false, true}) for (bool groups : {false, true}) for (bool over : {false, true}) {
		custom->grf_prop.grfid = file.grfid + foreign; custom->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, groups ? result : nullptr); ats->grf_prop.override_id = over ? NEW_AIRPORTTILE_OFFSET : INVALID_AIRPORTTILE;
		for (StationGfx id : {gfx, StationGfx{NEW_AIRPORTTILE_OFFSET}}) { SetStationGfx(tile, id); AirportTileResolverObject query(ats, tile, st); std::array<int32_t, 1> regs{}; query.ResolveCallback(regs); fmt::print(stderr, "STCMD airport-id {} {} {} {} {} {}\n", id, foreign, groups, over, GetTranslatedAirportTileID(id), regs[0]); }
	}
	SetStationGfx(tile, gfx); *custom = std::move(saved_custom);
	for (uint8_t offset : {0, 0x11, 0x88, 0xFF}) for (uint8_t var : {0x43, 0x60, 0x61, 0x62}) {
		det->adjusts[0].variable = var; det->adjusts[0].parameter = offset; AirportTileResolverObject query(ats, tile, st); std::array<int32_t, 1> regs{}; query.ResolveCallback(regs);
		fmt::print(stderr, "STCMD airport-offset {} {} {}\n", var, offset, regs[0]);
	}

	auto north = st->airport.tile;
	for (TileIndex base : {tile + TileDiffXY(1, 2), TileXY(Map::SizeX() - 1, Map::SizeY() - 1)}) {
		st->airport.tile = base; det->adjusts[0].variable = 0x43;
		AirportTileResolverObject query(ats, tile, st); std::array<int32_t, 1> regs{}; query.ResolveCallback(regs);
		fmt::print(stderr, "STCMD airport-relative-wrap {} {}\n", base.base(), regs[0]);
	}
	st->airport.tile = north;
	/* Live selection: missing/disabled custom, substitute and its override. */
	std::vector<EntityIDMapping> mappings(_airport_mngr.mappings.begin(), _airport_mngr.mappings.end()); auto *new_spec = AirportFixtureBaseSpec(NEW_AIRPORT_OFFSET); AirportSpec saved_new = *new_spec;
	for (bool enabled : {false, true}) for (uint32_t grfid : {0U, file.grfid}) for (uint16_t substitute : {uint16_t{AT_INVALID}, uint16_t{AT_SMALL}}) {
		new_spec->enabled = enabled; new_spec->grf_prop.override_id = AT_INVALID; _airport_mngr.mappings[NEW_AIRPORT_OFFSET] = {grfid, 0, substitute};
		for (uint16_t over : {uint16_t{AT_INVALID}, uint16_t{AT_LARGE}}) { spec->grf_prop.override_id = over; fmt::print(stderr, "STCMD airport-selection {} {} {} {} {}\n", enabled, grfid, substitute, over, AirportSpec::Get(NEW_AIRPORT_OFFSET)->GetIndex()); }
	}
	std::copy(mappings.begin(), mappings.end(), _airport_mngr.mappings.begin()); *new_spec = std::move(saved_new);
	fmt::print(stderr, "STCMD airport-state {} {} {} {} {}\n", st->index.base(), st->random_bits, st->airport.psa->GetValue(7), _random.state[0], _random.state[1]);
	*ats = std::move(saved_tile); *spec = std::move(saved);
	delete random; delete det; delete parent; delete other; delete result;
}

#include "newgrf_config.h"
#include "newgrf_badge.h"
#include "newgrf/newgrf_internal.h"
#include "strings_func.h"
static uint32_t AirportDefinitionID(char initial = 'A')
{
	return uint32_t(initial) | uint32_t('4') << 8 | uint32_t('9') << 16 | uint32_t('2') << 24;
}
/* Full load recovery frees failed payloads. Production entries on both games expose
 * the per-read mutations, retained old layout and badge-seen effects before cleanup. */
static void AirportDefinitionPartialReads()
{
	GRFFile file; file.grf_version = 8; file.grfid = AirportDefinitionID('P');
	AutoRestoreBackup current(_cur_gps.grffile, &file);
	auto read = [&](bool tile, uint first, uint last, int property, std::span<const uint8_t> data) {
		ByteReader reader(data.data(), data.size()); int result = -1; bool bounds = false;
		try { result = tile ? GrfChangeInfoHandler<GSF_AIRPORTTILES>::Activation(first, last, property, reader) : GrfChangeInfoHandler<GSF_AIRPORTS>::Activation(first, last, property, reader); }
		catch (const OTTDByteReaderSignal &) { bounds = true; }
		fmt::print(stderr, "STCMD airport-def-partial {} {} {} {} {} {} {}\n", tile, first, last, property, result, bounds, reader.Remaining());
		return std::pair(result, bounds);
	};
	const std::array<uint8_t, 1> zero{0}, one{1}, disable{255};
	StationNeed(read(false, 0, 1, 9, {}).first == CIR_UNKNOWN && file.airportspec.size() == 1 && file.airportspec[0] == nullptr, __LINE__);
	StationNeed(read(false, 0, 1, 0x0C, {}).first == CIR_INVALID_ID, __LINE__);
	StationNeed(!read(false, 0, 1, 8, zero).second, __LINE__);
	auto *spec = file.airportspec[0].get(); auto fsm = spec->fsm;
	StationNeed(!read(false, 0, 1, 8, one).second && file.airportspec[0].get() == spec && spec->grf_prop.subst_id == 0 && spec->fsm == fsm, __LINE__);
	StationNeed(!read(false, 7, 8, 8, disable).second && !AirportFixtureBaseSpec(7)->enabled, __LINE__);
	std::array<uint8_t, 4> years{0x34, 0x12, 0xff, 0xff};
	for (size_t n = 0; n <= years.size(); n++) {
		spec->min_year = TimerGameCalendar::Year{1}; spec->max_year = TimerGameCalendar::Year{2};
		StationNeed(read(false, 0, 1, 0x0C, std::span(years).first(n)).second == (n < years.size()), __LINE__);
		StationNeed(spec->min_year == (n >= 2 ? 0x1234 : 1) && spec->max_year == (n == 4 ? CalendarTime::MAX_YEAR.base() : 2), __LINE__);
	}
	const std::array<uint8_t, 15> layout{1, 0xff, 0xff, 0xff, 0xff, 3, 0xff, 2, 0xff, 2, 0xff, 0xfe, 99, 0, 0};
	std::vector<uint8_t> complete(layout.begin(), layout.end()); complete.push_back(0x80);
	for (size_t n = 0; n < complete.size(); n++) {
		auto count = spec->layouts.size(); auto x = spec->size_x, y = spec->size_y; auto gfx = spec->layouts[0].tiles[0].gfx;
		StationNeed(read(false, 0, 1, 0x0A, std::span(complete).first(n)).second, __LINE__);
		StationNeed(spec->layouts.size() == count && spec->layouts[0].tiles[0].gfx == gfx && spec->size_x == x && spec->size_y == y, __LINE__);
	}
	StationNeed(!read(false, 0, 1, 0x0A, complete).second && spec->layouts[0].rotation == DIR_E && spec->layouts[0].tiles[0].ti.x == -1 && spec->layouts[0].tiles[1].gfx == 0xFE && spec->size_x == 3 && spec->size_y == 3, __LINE__);
	StationNeed(!read(true, 0, 1, 8, zero).second && _airporttile_mngr.GetID(0, file.grfid) == NEW_AIRPORTTILE_OFFSET && file.airtspec[0]->animation.frames == 0, __LINE__);
	for (size_t n = 0; n < 2; n++) {
		const std::array<uint8_t, 2> animation{7, 0};
		file.airtspec[0]->animation.frames = 0; file.airtspec[0]->animation.status = AnimationStatus::NoAnimation;
		StationNeed(read(true, 0, 1, 0x0F, std::span(animation).first(n)).second && file.airtspec[0]->animation.frames == (n ? 7 : 0) && file.airtspec[0]->animation.status == AnimationStatus::NoAnimation, __LINE__);
	}
	BadgeID badge = GetOrCreateBadge("owner492/partial").index; file.badge_list = {badge};
	const std::array<uint8_t, 4> badges{2, 0, 0, 0};
	StationNeed(read(true, 0, 1, 0x12, badges).second && file.airtspec[0]->badges.empty() && GetBadge(badge)->features.Test(GSF_TRAMTYPES) && !GetBadge(badge)->features.Test(GSF_AIRPORTTILES), __LINE__);
	StationNeed(read(false, 0, 1, 0x12, badges).second && spec->badges.empty() && GetBadge(badge)->features.Test(GSF_AIRPORTS), __LINE__);
	fmt::print(stderr, "STCMD airport-def-partial-state years layouts badges\n");
}
static void AirportDefinitionLoad(std::string_view folder)
{
	AirportDefinitionPartialReads(); SetRandomSeed(0x49200001); _grfconfig.clear();
	_airport_mngr.ResetMapping(); _airporttile_mngr.ResetMapping();
	auto exhausted = std::make_unique<GRFConfig>(std::string(folder) + "/exhausted.grf");
	StationNeed(FillGRFDetails(*exhausted, false, NO_DIRECTORY), __LINE__);
	exhausted->flags.Reset(GRFConfigFlag::InitOnly); _grfconfig.push_back(std::move(exhausted));
	ReloadNewGRFData();
	StationNeed(AirportTileSpec::Get(0)->animation.frames == 19 && AirportTileSpec::Get(0)->grf_prop.local_id == 182 && _airport_mngr.GetID(118, AirportDefinitionID('E')) == AT_INVALID, __LINE__);
	fmt::print(stderr, "STCMD airport-def-exhaustion narrowed-tile-zero airport-rejected\n");
	_grfconfig.clear();
	_airport_mngr.ResetMapping(); _airporttile_mngr.ResetMapping();
	for (const char *name : {"airports", "competing", "broken-airport", "broken-tile"}) {
		auto config = std::make_unique<GRFConfig>(std::string(folder) + "/" + name + ".grf");
		StationNeed(FillGRFDetails(*config, false, NO_DIRECTORY), __LINE__);
		config->flags.Reset(GRFConfigFlag::InitOnly); _grfconfig.push_back(std::move(config));
	}
	ReloadNewGRFData();
}
static void AirportDefinitionFixture(TileIndex origin, DoCommandFlags test, DoCommandFlags exec, bool reload)
{
	const auto grfid = AirportDefinitionID();
	if (reload) ReloadNewGRFData();
	const GRFFile *file = GetFileByGRFID(grfid);
	StationNeed(file != nullptr && GetGRFConfig(grfid)->status == GCS_ACTIVATED && file->airportspec.empty() && file->airtspec.empty(), __LINE__);
	uint16_t type = _airport_mngr.GetID(0, grfid), gfx = _airporttile_mngr.GetID(0, grfid);
	StationNeed(type == NEW_AIRPORT_OFFSET && gfx == NEW_AIRPORTTILE_OFFSET, __LINE__);
	const auto *spec = AirportSpec::Get(type); const auto *tile_spec = AirportTileSpec::Get(gfx);
	StationNeed(AirportFixtureBaseSpec(0)->grf_prop.override_id == type && !AirportFixtureBaseSpec(0)->enabled && spec->grf_prop.subst_id == 0 && spec->size_x == 4 && spec->size_y == 3 && spec->layouts.size() == 1 && spec->layouts[0].rotation == DIR_N && spec->layouts[0].tiles.size() == 12 && spec->layouts[0].tiles[0].gfx == gfx, __LINE__);
	StationNeed(spec->min_year == 0 && spec->max_year == CalendarTime::MAX_YEAR && spec->catchment == MAX_CATCHMENT && spec->noise_level == 7 && spec->maintenance_cost == 123 && spec->IsAvailable() && GetString(spec->name) == "Airport 492", __LINE__);
	StationNeed(AirportTileSpec::Get(0)->grf_prop.override_id == gfx && !AirportTileSpec::Get(0)->enabled && tile_spec->animation.frames == 8 && tile_spec->animation.speed == 0 && tile_spec->grf_prop.subst_id == 0 && tile_spec->badges == spec->badges && tile_spec->badges.size() == 1 && GetBadge(tile_spec->badges[0])->features.Test(GSF_TRAMTYPES), __LINE__);
	for (char initial : {'B', 'T'}) { auto *config = GetGRFConfig(AirportDefinitionID(initial)); StationNeed(config != nullptr && config->status == GCS_DISABLED && config->errors.size() == 1 && config->errors[0].message == STR_NEWGRF_ERROR_READ_BOUNDS, __LINE__); }
	fmt::print(stderr, "STCMD airport-def-registry {} {} {} {} {}\n", type, gfx, spec->index, spec->class_index, AirportClass::GetClassCount());
	for (AirportClassID c = APC_BEGIN; c < AirportClass::GetClassCount(); c++) { const auto *klass = AirportClass::Get(c); for (uint i = 0; i < klass->GetSpecCount(); i++) fmt::print(stderr, "STCMD airport-def-class {} {} {}\n", c, i, klass->GetSpec(i)->GetIndex()); }
	for (uint id = 0; id < NUM_AIRPORTS; id++) { auto *value = AirportFixtureBaseSpec(id); if (value->grf_prop.grfid || value->grf_prop.override_id != AT_INVALID) fmt::print(stderr, "STCMD airport-def-spec {} {} {} {} {} {}\n", id, value->enabled, value->grf_prop.grfid, value->grf_prop.local_id, value->grf_prop.subst_id, value->grf_prop.override_id); }
	for (const auto &part : spec->layouts[0].tiles) fmt::print(stderr, "STCMD airport-def-layout {} {} {}\n", part.ti.x, part.ti.y, part.gfx);
	for (uint id = 0; id < NUM_AIRPORTTILES; id++) { const auto &map = _airporttile_mngr.mappings[id]; if (map.grfid) fmt::print(stderr, "STCMD airport-def-tile-map {} {} {} {}\n", id, map.grfid, map.entity_id, map.substitute_id); }
	Station *station = nullptr;
	if (!reload) {
		TileIndex at = origin + TileDiffXY(2, 2), destination = origin + TileDiffXY(18, 18);
		_settings_game.economy.station_noise_level = true;
		Town *town = ClosestTownFromTile(at, UINT_MAX); auto old_noise = town->noise_reached; town->noise_reached = town->MaxTownNoise();
		auto refused = Command<CMD_BUILD_AIRPORT>::Do(test, at, type, 0, NEW_STATION, true); StationCost("airport-def-noise", refused);
		StationNeed(refused.Failed() && refused.GetErrorMessage() == STR_ERROR_LOCAL_AUTHORITY_REFUSES_NOISE, __LINE__); town->noise_reached = old_noise;
		_settings_game.economy.station_noise_level = false;
		auto build = [&](DoCommandFlags flags, TileIndex t) { return Command<CMD_BUILD_AIRPORT>::Do(flags, t, type, 0, NEW_STATION, true); };
		auto price = build(test, at); StationCost("airport-def-test", price); auto cost = build(exec, at); StationCost("airport-def-exec", cost);
		StationNeed(price.Succeeded() && cost.Succeeded() && price.GetCost() == cost.GetCost(), __LINE__);
		station = Station::GetByTile(at); station->name = "Airport definitions";
		StationNeed(build(exec, destination).Succeeded(), __LINE__); auto *other = Station::GetByTile(destination);
		EngineID engine = EngineID::Invalid();
		for (const Engine *e : Engine::IterateType(VEH_AIRCRAFT)) if (engine == EngineID::Invalid() && IsEngineBuildable(e->index, VEH_AIRCRAFT, _current_company) && e->VehInfo<AircraftVehicleInfo>().subtype & AIR_CTOL) engine = e->index;
		StationNeed(engine != EngineID::Invalid(), __LINE__);
		auto hangar = station->airport.GetHangarTile(0);
		auto [built, vehicle, refit, capacity, cargo] = Command<CMD_BUILD_VEHICLE>::Do(exec, hangar, engine, false, INVALID_CARGO, INVALID_CLIENT_ID); StationCost("airport-def-aircraft", built); StationNeed(built.Succeeded(), __LINE__);
		Vehicle *plane = Vehicle::Get(vehicle); plane->name = "Airport definition plane";
		for (StationID target : {other->index, station->index}) { Order order; order.MakeGoToStation(target); order.SetStopLocation(OrderStopLocation::FarEnd); StationNeed(Command<CMD_INSERT_ORDER>::Do(exec, vehicle, VehicleOrderID(plane->GetNumOrders()), order).Succeeded(), __LINE__); }
		StationNeed(Command<CMD_START_STOP_VEHICLE>::Do(exec, vehicle, false).Succeeded(), __LINE__);
	}
	else for (Station *st : Station::Iterate()) if (st->name == "Airport definitions") station = st;
	StationNeed(station != nullptr && station->airport.type == type && station->airport.GetSpec() == spec, __LINE__);
	TileIndex at = station->airport.tile; StationNeed(GetAirportGfx(at) == gfx, __LINE__);
	AirportTileResolverObject callback(tile_spec, at, station, CBID_AIRPTILE_ANIMATION_TRIGGER);
	StationNeed(callback.ResolveCallback({}) == 3 && TriggerAirportTileAnimation(station, at, AirportAnimationTrigger::Built), __LINE__);
	AnimateAirportTile(at); StationNeed(GetAnimationFrame(at) == 3, __LINE__);
	fmt::print(stderr, "STCMD airport-def-callbacks {} {} {}\n", station->index.base(), callback.ResolveCallback({}), GetAnimationFrame(at));
	fmt::print(stderr, "STCMD airport-def-occupied {} {} {} {}\n", station->airport.type, GetAirportGfx(at), spec->grf_prop.grfid, tile_spec->grf_prop.local_id);
	for (Aircraft *plane : Aircraft::Iterate()) if (plane->name == "Airport definition plane") fmt::print(stderr, "STCMD airport-def-plane {} {} {} {}\n", plane->index.base(), plane->x_pos, plane->y_pos, plane->z_pos);
	fmt::print(stderr, "STCMD airport-def-rng {} {}\n", _random.state[0], _random.state[1]);
}

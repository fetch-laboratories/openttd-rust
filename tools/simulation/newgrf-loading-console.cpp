/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf-loading-console.cpp Production loading from deterministic on-disk GRFs. */
#include "stdafx.h"
#include "vehicle-record-fixture.hpp"
#include "engine-record-fixture.hpp"
#include "console_internal.h"
#include "newgrf/newgrf_act2.cpp"
#include "newgrf_text.cpp"
#ifdef WITH_RUST
#include "rust/byte_span_ffi.h"
#else
struct OpenTTDByteSpan { const uint8_t *data; size_t size; };
#endif
#include "language.h"
#include "house.h"
#include "newgrf_airport.h"
#include "newgrf_airporttiles.h"
#include "newgrf_badge.h"
#include "newgrf_canal.h"
#include "newgrf_generic.h"
#include "newgrf_object.h"
#include "newgrf_roadstop.h"
#include "road.h"
#include "rail.h"
#include "newgrf_station.h"
#include "station_cmd.h"
#include "landscape.h"
#include "timer/timer_game_calendar.h"
#ifdef WITH_RUST
#include "tests/world_generation_probe.hpp"
#include "rust/services_ffi.h"
#endif
#include "tile_map.h"
#include "vehicle_base.h"
#include "industry_cmd.h"
#include "newgrf_industries.h"
#include "newgrf_industrytiles.h"
#include "industry_map.h"
#include "industrytype.h"
#include "rail_type.h"
#include "command_func.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "depot_base.h"
#include "depot_map.h"
#include "engine_base.h"
#include "engine_func.h"
#include "newgrf.h"
#include "newgrf_config.h"
#include "newgrf_engine.h"
#include "newgrf/newgrf_internal.h"
#include "settings_type.h"
#include "saveload/saveload.h"
#include "vehicle_cmd.h"
#include "newgrf_townname.h"
#include "strings_func.h"
#include "town.h"
#include "town_cmd.h"
#include "townname_func.h"
#include "table/strings.h"

static uint32_t ID(const char *text)
{
	return uint32_t(uint8_t(text[0])) | uint32_t(uint8_t(text[1])) << 8 | uint32_t(uint8_t(text[2])) << 16 | uint32_t(uint8_t(text[3])) << 24;
}
static void Require(bool value, const char *name)
{
	fmt::print(stderr, "LOADING case {} {}\n", name, value);
	if (!value) std::abort();
}
static std::string Hex(std::string_view text)
{
	std::string result;
	for (char c : text) result += fmt::format("{:02X}", uint8_t(c));
	return result;
}
#ifdef WITH_RUST
extern "C" uint8_t openttd_test_grf_text_dump(const void *, void (*)(const void *, uint32_t, uint32_t, uint8_t, StringID, StringID, OpenTTDByteSpan) noexcept);
#endif
/** Saves omit translated bytes, defaults, language selection and table StringIDs. */
static void GrfStrings(const char *phase)
{
	auto emit = [](const void *context, uint32_t grfid, uint32_t stringid, uint8_t language, StringID fallback, StringID returned, OpenTTDByteSpan text) noexcept {
		Require(GetGRFStringID(grfid, GRFStringID{stringid}) == returned, "text-id");
		fmt::print(stderr, "LOADING text {} row {:08X} {:X} {:X} {} {} {}\n", static_cast<const char *>(context), grfid, stringid, language, fallback, returned, text.size == 0 ? "-" : Hex({reinterpret_cast<const char *>(text.data), text.size}));
	};
#ifdef WITH_RUST
	uint8_t saved = openttd_test_grf_text_dump(phase, emit);
#else
	uint8_t saved = _current_lang_id;
	for (uint32_t index = 0; index < _grf_text.size(); index++) {
		const auto &entry = _grf_text[StringIndexInTab{index}];
		for (const auto &text : entry.textholder) emit(phase, entry.grfid, entry.stringid.base(), text.langid, entry.def_string, MakeStringID(TEXT_TAB_NEWGRF_START, StringIndexInTab{index}), {reinterpret_cast<const uint8_t *>(text.text.data()), text.text.size()});
	}
#endif
	for (uint8_t language : {0, 1, 2, 7, 96, 127}) {
		SetCurrentGrfLangID(language);
#ifdef WITH_RUST
		uint8_t current = openttd_test_grf_text_dump(nullptr, [](const void *, uint32_t, uint32_t, uint8_t, StringID, StringID, OpenTTDByteSpan) noexcept {});
#else
		uint8_t current = _current_lang_id;
#endif
		uint32_t old = 0;
		for (uint8_t mask = 0; mask < 32; mask++) if (CheckGrfLangID(mask, 6)) old |= 1U << mask;
		StringID id = GetGRFStringID(ID("TX52"), GRFStringID{0xD000});
		fmt::print(stderr, "LOADING text {} language {} {} {} {} {} {}\n", phase, language, current, old, CheckGrfLangID(language, 7), CheckGrfLangID(0x7F, 7), id == STR_UNDEFINED ? "-" : Hex(GetStringPtr(id)));
	}
	SetCurrentGrfLangID(saved);
	fmt::print(stderr, "LOADING text {} byte80 {}\n", phase, Hex(TranslateTTDPatchCodes(0, 1, false, "\x80", static_cast<StringControlCode>('!'))));
	if (GetGRFConfig(ID("TX52")) != nullptr) {
		const GRFConfig *before = GetGRFConfig(ID("TB52"));
		const GRFConfig *bounds = GetGRFConfig(ID("TE52"));
		Require(before->status == GCS_DISABLED && before->errors.size() == 1 && before->errors.front().message == STR_NEWGRF_ERROR_LOAD_AFTER && before->errors.front().data == GetString(STR_NEWGRF_ERROR_AFTER_TRANSLATED_FILE), "text-load-after");
		Require(bounds->status == GCS_DISABLED && bounds->errors.size() == 1 && bounds->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "text-read-bounds");
		Require(GetGRFStringID(ID("TX52"), GRFStringID{0xD00A}) == STR_UNDEFINED && GetGRFStringID(ID("TX52"), GRFStringID{0xD00B}) != STR_UNDEFINED && GetGRFStringID(ID("TX52"), GRFStringID{0xFFFF}) == STR_UNDEFINED, "text-empty-and-wrapped");
	}
}

/** Normalize deterministic fixture groups by their terminal callback result. */
static int BindingResult(const SpriteGroup *group)
{
	if (group == nullptr) return -1;
	if (const auto *callback = dynamic_cast<const CallbackResultSpriteGroup *>(group)) return callback->result;
	const auto *deterministic = dynamic_cast<const DeterministicSpriteGroup *>(group);
	Require(deterministic != nullptr && !deterministic->default_result.calculated_result, "binding-group-kind");
	return BindingResult(deterministic->default_result.group);
}
/** Binding roots, feature flags and override order do not persist in saves. */
static void GrfBindings(const char *phase)
{
	const GRFFile *file = GetFileByGRFID(ID("BD52"));
	if (file == nullptr) return;
	const CargoType passengers = GetCargoTypeByLabel(CT_PASSENGERS);
	std::array<uint32_t, 4> rng{_random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]};
	auto standard = [&](uint8_t feature, const auto *spec) {
		const auto &p = spec->grf_prop;
		int def = BindingResult(p.GetSpriteGroup(StandardSpriteGroup::Default)), purchase = BindingResult(p.GetSpriteGroup(StandardSpriteGroup::Purchase));
		Require(p.grfid == file->grfid && p.local_id == 0 && def == 145 && purchase == 146, "binding-standard");
		fmt::print(stderr, "LOADING binding {} standard {} {:08X} {} {} {}\n", phase, feature, p.grfid, p.local_id, def, purchase);
	};
	standard(7, HouseSpec::Get(_house_mngr.GetID(0, file->grfid)));
	standard(9, GetIndustryTileSpec(_industile_mngr.GetID(0, file->grfid)));
	standard(10, GetIndustrySpec(_industry_mngr.GetID(0, file->grfid)));
	standard(13, AirportSpec::Get(static_cast<uint8_t>(_airport_mngr.GetID(0, file->grfid))));
	standard(15, ObjectSpec::Get(_object_mngr.GetID(0, file->grfid)));
	standard(17, AirportTileSpec::Get(_airporttile_mngr.GetID(0, file->grfid)));
	auto cargo_groups = [&](uint8_t feature, const auto *spec) {
		const auto &p = spec->grf_prop;
		int def = BindingResult(p.GetSpriteGroup(CargoGRFFileProps::SG_DEFAULT)), purchase = BindingResult(p.GetSpriteGroup(CargoGRFFileProps::SG_PURCHASE));
		int na = BindingResult(p.GetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA));
		Require(p.grfid == file->grfid && def == 145 && purchase == 146 && na == 146, "binding-station-first");
		fmt::print(stderr, "LOADING binding {} station {} {:08X} {} {} {} {}\n", phase, feature, p.grfid, BindingResult(p.GetSpriteGroup(passengers)), def, purchase, na);
	};
	cargo_groups(4, file->stations[0].get()); cargo_groups(20, file->roadstops[0].get());
	for (uint16_t local : {0, 1, 2, 3, 27}) {
		const Engine *engine = Engine::Get(GetNewEngineID(file, VEH_TRAIN, local));
		const auto &p = EngineBindings(engine).grf_prop;
		fmt::print(stderr, "LOADING binding {} engine {} {} {:08X} {} {} {}\n", phase, local, engine->index, p.grfid, BindingResult(FixtureEngineSprite(engine, passengers)), BindingResult(FixtureEngineSprite(engine, CargoGRFFileProps::SG_DEFAULT)), BindingResult(FixtureEngineSprite(engine, CargoGRFFileProps::SG_PURCHASE)));
		for (uint i = 0; i < EngineBindings(engine).overrides.size(); i++) {
			const auto &override = EngineBindings(engine).overrides[i];
			std::string ids;
			for (EngineID id : override.engines) ids += fmt::format("{},", id.base());
			fmt::print(stderr, "LOADING binding {} override {} {} {} {} {}\n", phase, local, i, override.cargo, BindingResult(override.group), ids);
		}
	}
	EngineID wagon = GetNewEngineID(file, VEH_TRAIN, 27);
	for (uint16_t local : {0, 1, 2, 3}) {
		int result = BindingResult(GetWagonOverrideSpriteSet(wagon, local == 1 ? CargoGRFFileProps::SG_PURCHASE : passengers, GetNewEngineID(file, VEH_TRAIN, local)));
		Require(result == (local < 2 ? 146 : local == 2 ? 148 : 150), "binding-wagon-order");
		fmt::print(stderr, "LOADING binding {} selected {} {}\n", phase, local, result);
	}
	auto leaf = [&](uint8_t feature, const GRFFile *owner, const SpriteGroup *group) {
		Require(owner == file, "binding-leaf-owner");
		fmt::print(stderr, "LOADING binding {} leaf {} {:08X} {}\n", phase, feature, owner->grfid, BindingResult(group));
	};
	leaf(5, _water_feature[0].grffile, _water_feature[0].group);
	leaf(11, CargoSpec::Get(0)->grffile, CargoSpec::Get(0)->group);
	const RailTypeInfo *rail = GetRailTypeInfo(file->railtype_map[0]);
	leaf(16, rail->grffile[0], rail->group[0]);
	for (uint8_t feature : {18, 19}) {
		const RoadTypeInfo *road = GetRoadTypeInfo(feature == 18 ? file->roadtype_map[0] : file->tramtype_map[0]);
		leaf(feature, road->grffile[0], road->group[0]);
	}
	const auto &badge = GetBadge(file->badge_map.at(0))->grf_prop;
	Require(badge.grfid == file->grfid && BindingResult(badge.GetSpriteGroup(GSF_TRAINS)) == 146 && BindingResult(badge.GetSpriteGroup(GSF_DEFAULT)) == 145, "binding-badge");
	fmt::print(stderr, "LOADING binding {} badge {} {}\n", phase, BindingResult(badge.GetSpriteGroup(GSF_TRAINS)), BindingResult(badge.GetSpriteGroup(GSF_DEFAULT)));
	auto generic = GetAiPurchaseCallbackResult(GSF_ROADVEHICLES, passengers, 0, IT_AI_UNKNOWN, IT_AI_TOWN, 1, AICE_ROAD_CHECK_ENGINE, 1, 1);
	Require(generic.first != nullptr && generic.first->grfid == ID("BH52") && generic.second == 152, "binding-generic-order");
	fmt::print(stderr, "LOADING binding {} generic {:08X} {}\n", phase, generic.first->grfid, generic.second);
	for (const char *id : {"BE52", "BD52", "BG52", "BH52", "BP52", "BO52", "BQ52", "BR52"}) {
		const GRFConfig *config = GetGRFConfig(ID(id));
		const GRFFile *grf = GetFileByGRFID(ID(id));
		Require(grf != nullptr && ((std::string_view(id) != "BG52" && std::string_view(id) != "BH52") || grf->grf_features.base() == 0), "binding-flags");
		fmt::print(stderr, "LOADING binding {} flags {} {} {:X}\n", phase, id, config->status, grf->grf_features.base());
	}
	Require(rng == std::array<uint32_t, 4>{_random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]}, "binding-observer-rng");
	fmt::print(stderr, "LOADING binding {} rng {} {} {} {}\n", phase, rng[0], rng[1], rng[2], rng[3]);
}

/** Native style lookup, display and loader state; parsing and name bytes moved to newgrf_townname::evidence_tests. */
static void GrfTownStyles(const char *phase, bool loaded)
{
	auto names = GetGRFTownNameList();
	auto params = [](uint16_t gen) { return TownNameParams(static_cast<uint8_t>(BUILTIN_TOWNNAME_GENERATOR_COUNT + gen)); };
	fmt::print(stderr, "LOADING townname {} styles {}\n", phase, names.size());
	for (uint gen = 0; gen <= names.size(); gen++) {
		TownNameParams par = params(gen);
		StringID name = GetGRFTownNameName(gen);
		fmt::print(stderr, "LOADING townname {} style {} {:08X} {} {} {}\n", phase, gen, par.grfid, par.type, gen < names.size() && name == names[gen], Hex(GetString(name)));
	}
	if (!loaded) return;
	Require(params(4).grfid == 0 && params(4).type == SPECSTR_TOWNNAME_START && GetGRFTownNameName(4) == STR_UNDEFINED, "townname-fallback");
	GRFConfig *truncated = GetGRFConfig(ID("TNS5"));
	Require(truncated->status == GCS_DISABLED && truncated->errors.size() == 1 && truncated->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "townname-partial-read");
}
/** Found one town per NewGRF style through the production name generator. */
static void GrfTownNameTowns()
{
	AutoRestoreBackup company(_current_company, OWNER_DEITY);
	auto names = GetGRFTownNameList();
	for (uint gen = 0; gen < names.size(); gen++) {
		AutoRestoreBackup style(_settings_game.game_creation.town_name, static_cast<uint8_t>(BUILTIN_TOWNNAME_GENERATOR_COUNT + gen));
		uint32_t parts = 0;
		bool generated = GenerateTownName(_random, &parts);
		TownID id = TownID::Invalid();
		for (uint32_t n = 1 + 997 * gen; n < Map::Size() && id == TownID::Invalid(); n += 31) {
			TileIndex tile(n);
			if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT) continue;
			id = std::get<TownID>(Command<CMD_FOUND_TOWN>::Do(DoCommandFlag::Execute, tile, TSZ_SMALL, false, TL_ORIGINAL, false, parts, {}));
		}
		Require(generated && id != TownID::Invalid(), "townname-found");
		const Town *t = Town::Get(id);
		fmt::print(stderr, "LOADING townname town {} {} {:08X} {} {:08X} {}\n", gen, id.base(), t->townnamegrfid, t->townnametype, t->townnameparts, Hex(GetTownName(t)));
	}
}
static void Inspect(bool reload)
{
	Require(_cur_gps.file == nullptr && _cur_gps.grffile == nullptr && _cur_gps.grfconfig == nullptr && _cur_gps.nfo_line == 0 && _cur_gps.skip_sprites == 0 && std::ranges::all_of(_cur_gps.spritegroups, [](const auto *group) { return group == nullptr; }), "execution-restored");
	GRFConfig *main = GetGRFConfig(ID("R471"));
	GRFFile *file = GetFileByGRFID(ID("R471"));
	Require(main != nullptr && file != nullptr && main->status == GCS_ACTIVATED, "active");
	Require(file->GetParam(0) == 77 && file->GetParam(1) == 123 && file->GetParam(2) == 2, "d6-jumps");
	Require(file->GetParam(31) == 66 && file->GetParam(32) == 55 && file->GetParam(35) == 0x201, "patch-flags-stages");
	Require(file->GetParam(40) == 1 && file->GetParam(41) == 0xC0000000 && file->GetParam(42) == 0xFFFFFFFC && file->GetParam(43) == 17, "arithmetic");
	fmt::print(stderr, "LOADING patch-map {}\n", file->GetParam(30));
	Require(file->GetParam(22) == 0xFFFFFFFE, "action6-carry-reuse");
	Require(file->GetParam(3) != 0 && file->GetParam(4) == UINT32_MAX, "grm");
	GRFConfig *broken = GetGRFConfig(ID("B471"));
	Require(broken != nullptr && broken->status == GCS_DISABLED && broken->errors.size() == 1 && broken->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "bounds");
	const GRFFile *partial = GetFileByGRFID(ID("B471"));
	Require(partial != nullptr && partial->GetParam(0) == 21, "partial-parameter");
	EngineID id = GetNewEngineID(file, VEH_ROAD, 0);
	Require(id != EngineID::Invalid() && Engine::Get(id)->VehInfo<RoadVehicleInfo>().max_speed == 124 && Engine::Get(id)->VehInfo<RoadVehicleInfo>().capacity == 33, "engine");
	EngineID partial_id = GetNewEngineID(partial, VEH_ROAD, 1);
	Require(partial_id != EngineID::Invalid() && Engine::Get(partial_id)->VehInfo<RoadVehicleInfo>().max_speed == 88, "partial-native-parser");
	Require(GetGRFConfig(ID("V471"))->status == GCS_DISABLED, "inhibition");
	GRFConfig *fatal = GetGRFConfig(ID("F471"));
	Require(fatal->status == GCS_DISABLED && fatal->errors.size() == 1 && fatal->errors.front().message == STR_NEWGRF_ERROR_GRM_FAILED, "fatal-grm");
	Require(GetGRFConfig(ID("Z471"))->status == GCS_DISABLED && GetGRFConfig(ID("Z471"))->errors.empty(), "skip-before-action8");
	Require(GetGRFConfig(ID("E471"))->status == GCS_ACTIVATED && GetFileByGRFID(ID("E471"))->param.empty(), "skip-after-action8");
	GRFConfig *graph_broken = GetGRFConfig(ID("B485"));
	Require(graph_broken != nullptr && graph_broken->status == GCS_DISABLED && graph_broken->errors.size() == 1 && graph_broken->errors.front().message == STR_NEWGRF_ERROR_READ_BOUNDS, "action2-file-truncated");
	const auto *graph = dynamic_cast<const DeterministicSpriteGroup *>(FixtureEngineSprite(Engine::Get(id), CargoGRFFileProps::SG_DEFAULT));
	Require(graph != nullptr && std::ranges::any_of(graph->ranges, [](const auto &r) { return r.low == 21 && r.high == 32 && dynamic_cast<const DeterministicSpriteGroup *>(r.result.group) != nullptr; }), "action2-file-graph");
	const auto &range = *std::ranges::find_if(graph->ranges, [](const auto &r) { return r.low == 21; });
	const auto *subroutine = dynamic_cast<const DeterministicSpriteGroup *>(range.result.group);
	const auto *calculated = dynamic_cast<const DeterministicSpriteGroup *>(subroutine->adjusts[0].subroutine);
	const auto *real = dynamic_cast<const RealSpriteGroup *>(subroutine->ranges[0].result.group);
	const auto *random = dynamic_cast<const RandomizedSpriteGroup *>(graph->ranges[1].result.group);
	Require(graph->size == DSG_SIZE_DWORD && subroutine->adjusts[0].divmod_val == 1 && calculated->size == DSG_SIZE_WORD && calculated->ranges.empty() && calculated->default_result.calculated_result && real != nullptr && real->loaded.size() == 2 && real->loading.size() == 1 && calculated->error_group == real->loading[0] && subroutine->default_result.group == real->loading[0] && graph->default_result.group == nullptr && random != nullptr && random->groups.size() == 3 && random->triggers == 2, "action2-normalized-graph");
	Require(Engine::Get(id)->DetermineCapacity(nullptr) == 55, "action2-file-capacity");
	IndustryGfx gfx = _industile_mngr.GetID(0, file->grfid);
	Industry *context = *Industry::Iterate().begin();
	IndustryTileResolverObject tile_resolver(gfx, context->location.tile, context);
	const auto *layout = tile_resolver.Resolve<TileLayoutSpriteGroup>();
	Require(layout != nullptr && layout->dts.seq.size() == 1 && layout->dts.registers.size() == 2, "action2-layout-parsed");
	uint8_t stage = 3;
	auto processed = layout->ProcessRegisters(tile_resolver, &stage);
	Require(stage == 0 && processed.GetLayout().GetSequence().size() == 1 && processed.GetLayout().ground.sprite == 1, "action2-layout-consumer");
	for (uint8_t variant = 0; variant < 3; variant++) {
		const auto *spec = GetIndustrySpec(_industry_mngr.GetID(variant, file->grfid));
		const auto *wrapper = dynamic_cast<const DeterministicSpriteGroup *>(spec->grf_prop.GetSpriteGroup(true));
		const auto *production = dynamic_cast<const IndustryProductionSpriteGroup *>(wrapper->ranges[0].result.group);
		Require(production != nullptr && production->version == variant && (variant != 0 || (production->subtract_input[0] == -2 && production->add_output[0] == 7)), "action2-production-parsed");
		if (reload) {
			bool found = false;
			for (const Industry *industry : Industry::Iterate()) found |= industry->type == _industry_mngr.GetID(variant, file->grfid) && industry->produced[0].waiting >= 7;
			Require(found, "action2-industry-reload");
		}
	}
	if (!reload) Require(GetGRFConfig(ID("MISS"))->status == GCS_NOT_FOUND, "missing");
	fmt::print(stderr, "LOADING state {} {} {} {} {}\n", id.base(), partial_id.base(), file->GetParam(3), broken->errors.front().nfo_line, broken->errors.front().param_value[0]);
	fmt::print(stderr, "LOADING {} true\n", reload ? "reload" : "complete");
}
static void Action2Consumers()
{
	const GRFFile *file = GetFileByGRFID(ID("R471"));
	AutoRestoreBackup company(_current_company, CompanyID(0));
	const StationSpec *station = file->stations[0].get();
	Require(GetStationCallback(CBID_STATION_AVAILABILITY, 0, 0, station, nullptr, INVALID_TILE) == 1, "action2-station-availability");
	bool station_built = false;
	for (uint32_t n = 1; n < Map::Size() && !station_built; ++n) {
		TileIndex tile(n);
		if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT || TileHeight(tile) == 0) continue;
		auto cost = Command<CMD_BUILD_RAIL_STATION>::Do({DoCommandFlag::Execute, DoCommandFlag::Auto}, tile, RAILTYPE_RAIL, AXIS_X, 1, 1, station->class_index, station->index, NEW_STATION, true);
		station_built = cost.Succeeded();
	}
	Require(station_built, "action2-station-command");
	AutoRestoreBackup founder(_current_company, OWNER_DEITY);
	AutoRestoreBackup multiple(_settings_game.economy.multiple_industry_per_town, true);
	for (uint8_t variant = 0; variant < 3; variant++) {
		IndustryType type = _industry_mngr.GetID(variant, file->grfid);
		bool built = false;
		for (uint32_t n = 1; n < Map::Size() && !built; n += 17) {
			TileIndex tile(n);
			if (!IsTileType(tile, MP_CLEAR) || GetTileSlope(tile) != SLOPE_FLAT || TileHeight(tile) == 0) continue;
			auto cost = Command<CMD_BUILD_INDUSTRY>::Do(DoCommandFlag::Execute, tile, type, 0, true, 485 + variant);
			if (!cost.Succeeded()) continue;
			for (Industry *industry : Industry::Iterate()) {
				if (industry->type != type) continue;
				auto before = industry->produced[0].waiting;
				IndustryProductionCallback(industry, 1);
				Require(industry->produced[0].waiting == before + 7, fmt::format("action2-production-v{}", variant).c_str());
				fmt::print(stderr, "LOADING production {} {} {} {}\n", variant, type, industry->produced[0].waiting, industry->random);
				built = true;
				break;
			}
		}
		Require(built, "action2-industry-command");
	}
}
static uint8_t SeasonalHighest()
{
#ifdef WITH_RUST
	return openttd_test_snow_line_highest(&GetRustSharedServices());
#else
	return HighestSnowLine();
#endif
}
static void SeasonalSnow(const char *phase, bool loaded)
{
	AutoRestoreBackup date(TimerGameCalendar::date, TimerGameCalendar::date);
	auto *file = GetFileByGRFID(ID("SN17"));
	Require(IsSnowLineSet() == loaded, "seasonal-state");
	for (int year : {1999, 2000, 2001, 2100}) {
		auto first = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year}, 0, 1);
		auto end = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year + 1}, 0, 1);
		for (auto date = first; date < end; ++date) {
			TimerGameCalendar::date = date;
			auto ymd = TimerGameCalendar::ConvertDateToYMD(date);
			fmt::print(stderr, "LOADING snow {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n", phase, year, ymd.month, ymd.day, IsSnowLineSet(), GetSnowLine(), SeasonalHighest(), LowestSnowLine(), file == nullptr ? 0 : file->grf_version, _settings_game.construction.map_height_limit, _settings_game.game_creation.snow_line_height, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
		}
	}
}
static bool LoadingScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") {
		/* Interactive RNG is unsaved; observe it from a deterministic reload command seed. */
		_interactive_random.SetSeed(0x61700001);
		if (GetGRFConfig(ID("TX52")) != nullptr) { Require(ReadLanguagePack(GetLanguage(7)), "text-language-pack"); ReloadNewGRFData(); }
		Inspect(true); GrfTownStyles("inspect", false); GrfStrings("inspect"); GrfBindings("inspect"); SeasonalSnow("inspect", true); return true;
	}
	if (args.size() < 3) return false;
	SetRandomSeed(0x47100001);
	_grfconfig.clear();
	bool strings = std::filesystem::exists(std::filesystem::path(args[2]) / "text-modern.grf");
	if (strings) Require(ReadLanguagePack(GetLanguage(7)), "text-language-pack");
	std::vector<const char *> files{"main", "graph-broken", "broken", "victim", "fatal", "before", "after", "names-a", "names-bad", "names-b", "names-short", "snow", "snow-duplicate"};
	if (strings) files.insert(files.end(), {"translation-before", "text-modern", "text-old", "text-translate", "text-translate7", "translation-bounds"});
	if (strings) { files.insert(files.begin(), "bindings-empty"); files.insert(files.end(), {"bindings-main", "bindings-partial-ids", "bindings-after-ids", "bindings-partial-group", "bindings-after-group", "bindings-generic-a", "bindings-generic-b"}); }
	for (const char *name : files) {
		auto c = std::make_unique<GRFConfig>(std::string(args[2]) + "/" + name + ".grf");
		Require(FillGRFDetails(*c, false, NO_DIRECTORY), "file-scan");
		c->flags.Reset(GRFConfigFlag::InitOnly);
		_grfconfig.push_back(std::move(c));
	}
	auto missing = std::make_unique<GRFConfig>(std::string(args[2]) + "/absent.grf");
	missing->ident.grfid = ID("MISS");
	_grfconfig.push_back(std::move(missing));
	_settings_game.vehicle.dynamic_engines = false;
	ReloadNewGRFData();
	if (strings) Require(GetGRFConfig(ID("BE52"))->status == GCS_ACTIVATED, "binding-empty-predecessor");
	SeasonalSnow("first", true);
	ResetNewGRFData();
	SeasonalSnow("reset", false);
	ReloadNewGRFData();
	SeasonalSnow("reconstructed", true);
	Inspect(false);
	GrfTownStyles("first", true);
	GrfStrings("first");
	GrfBindings("first");
	ReloadNewGRFData();
	Inspect(false);
	GrfTownStyles("reload", true);
	GrfStrings("reload");
	GrfBindings("reload");
	SeasonalSnow("reload", true);
	GrfTownNameTowns();
	EngineID id = GetNewEngineID(GetFileByGRFID(ID("R471")), VEH_ROAD, 0);
	bool built = false;
	for (const Depot *depot : Depot::Iterate()) {
		if (GetDepotVehicleType(depot->xy) != VEH_ROAD) continue;
		AutoRestoreBackup company(_current_company, GetTileOwner(depot->xy));
		auto [cost, vehicle, capacity, mail, cargo] = Command<CMD_BUILD_VEHICLE>::Do(DoCommandFlag::Execute, depot->xy, id, false, INVALID_CARGO, INVALID_CLIENT_ID);
		if (!cost.Succeeded()) continue;
		fmt::print(stderr, "LOADING build {} {} {}\n", cost.Succeeded(), vehicle.base(), capacity);
		Require(capacity == 55 && FIXTURE_FIELD(Vehicle::Get(vehicle), cargo_cap) == 55, "action2-build-capacity");
		auto [refit, refit_capacity, refit_mail, refit_cargos] = Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, vehicle, Vehicle::Get(vehicle)->cargo_type, 1, false, true, 1);
		Require(refit.Succeeded() && refit_capacity == 55, "action2-refit-capacity");
		TriggerVehicleRandomisation(Vehicle::Get(vehicle), VehicleRandomTrigger::Depot);
		Require(!Vehicle::Get(vehicle)->waiting_random_triggers.Test(VehicleRandomTrigger::Depot), "action2-random-trigger");
		fmt::print(stderr, "LOADING randomized {} {} {} {}\n", Vehicle::Get(vehicle)->random_bits, Vehicle::Get(vehicle)->waiting_random_triggers.base(), _random.state[0], _random.state[1]);
		built = true;
		break;
	}
	Require(built, "usable-definition");
	Action2Consumers();
	fmt::print(stderr, "LOADING rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	std::erase_if(_grfconfig, [](const auto &c) { return c->status == GCS_NOT_FOUND; });
	if (args.size() > 3) Require(SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, "save");
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("loading_scenario", LoadingScenario);
}

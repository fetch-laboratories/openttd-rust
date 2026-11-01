/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading_services.inc Typed native loading services. */
#include "newgrf_loading_ffi.h"
#include "newgrf_act2_ffi.h"
#include "industry_definition_ffi.h"
#include "airport_definitions_ffi.h"
#include "type_registry_ffi.h"
#include "newgrf_global_ffi.h"
#include "generic_ffi.h"
#include "../string_func.h"
#include "services_newgrf.h"
#include "cargo_registry_ffi.h"
#include "settings_services.h"
#include "environment_services.h"
#include "services_map.h"
#include "service_table_check.hpp"
/** Each parser catches exactly the DecodeSpecialSprite domain, retaining partial effects. */
static_assert(GLS_FILESCAN == 0 && GLS_SAFETYSCAN == 1 && GLS_LABELSCAN == 2 && GLS_INIT == 3 && GLS_RESERVE == 4 && GLS_ACTIVATION == 5);
static_assert(GCS_UNKNOWN == 0 && GCS_DISABLED == 1 && GCS_NOT_FOUND == 2 && GCS_INITIALISED == 3 && GCS_ACTIVATED == 4);
static_assert(GSF_GLOBALVAR == 8 && GSF_CARGOES == 11 && to_underlying(GrfMiscBit::SecondRockyTileSet) == 6);
template <void (*Parser)(ByteReader &)>
static bool ParseGrfAction(const uint8_t *data, size_t size) noexcept
{
	ByteReader reader(data, size);
	try { Parser(reader); return true; } catch (...) { return false; }
}
const OpenTTDGrfLoadingServices &GetRustGrfLoadingServices() noexcept
{
	static const OpenTTDGrfLoadingServices services{
		.global = &GetRustGlobalVariableServices(),
		.cargo = &GetRustCargoRegistryServices(),
		.engine = &GetRustEngineLoadingServices(),
		.houses = &GetHouseDefinitionServices(),
		.industry = &GetIndustryDefinitionServices(),
		.airports = &GetAirportDefinitionServices(),
		.types = &GetRustTypeRegistryServices(),
		.action2 = &GetRustAction2Services(),
		.engine_range = [](GrfSpecFeature feature) noexcept { return OpenTTDGrfEngineRange{_engine_offsets[feature], _engine_counts[feature]}; },
		.parameter = RustGRFParameter,
		.file_id = RustGRFFileID,
		.config_id = [](const GRFConfig *c) noexcept { return c->ident.grfid; },
		.config_status = [](const GRFConfig *c) noexcept { return c->status; },
		.config_flags = [](const GRFConfig *c) noexcept { return c->flags.base(); },
		.set_status = [](GRFConfig *c, GRFStatus status) noexcept { c->status = status; },
		.set_flag = [](GRFConfig *c, uint8_t flag, bool set) noexcept { c->flags.Set(static_cast<GRFConfigFlag>(flag), set); },
		.set_config_id = [](GRFConfig *c, uint32_t id) noexcept { c->ident.grfid = id; },
		.set_file_info = [](GRFFile *f, uint32_t id, uint8_t version) noexcept { f->grfid = id; f->grf_version = version; },
		.config_version = [](const GRFConfig *c) noexcept { return c->version; },
		.param_count = [](const GRFFile *f) noexcept { return f->param.size(); },
		.set_parameter = [](GRFFile *f, uint8_t target, uint32_t value) noexcept { if (target >= f->param.size()) f->param.resize(target + 1); f->param[target] = value; },
		.find_config = [](uint32_t id, uint32_t mask) noexcept { return GetGRFConfig(id, mask); },
		.find_file = RustGRFFileByID,
		.file_for_config = [](const GRFConfig *c) noexcept { auto *f = GetFileByFilename(c->filename); if (f == nullptr) UserError("File '{}' lost in cache.\n", c->filename); return f; },
		.error = [](GRFConfig *c, uint32_t line, StringID message, bool current) noexcept { auto it = std::ranges::find(c->errors, line, &GRFError::nfo_line); if (it == c->errors.end()) it = c->errors.emplace(it, STR_NEWGRF_ERROR_MSG_FATAL, line, message); if (current) it->param_value[0] = line; return &*it; },
		.error_name = [](GRFError *e, const GRFConfig *c) noexcept { e->data = c->GetName(); },
		.error_property = RustGRFErrorProperty,
		.networking = RustNetworking,
		.read_byte = [](SpriteFile *f) noexcept { return f->ReadByte(); },
		.read_word = [](SpriteFile *f) noexcept { return f->ReadWord(); },
		.read_dword = [](SpriteFile *f) noexcept { return f->ReadDword(); },
		.read_block = [](SpriteFile *f, uint8_t *data, size_t size) noexcept { f->ReadBlock(data, size); },
		.position = [](const SpriteFile *f) noexcept { return f->GetPos(); },
		.seek = [](SpriteFile *f, size_t pos, bool relative) noexcept { f->SeekTo(pos, relative ? SEEK_CUR : SEEK_SET); },
		.skip = [](SpriteFile *f, size_t size) noexcept { f->SkipBytes(size); },
		.container_version = [](const SpriteFile *f) noexcept { return f->GetContainerVersion(); },
		.sprite_offsets = [](SpriteFile *f) noexcept { ReadGRFSpriteOffsets(*f); },
		.skip_sprite_data = [](SpriteFile *f, uint8_t type, uint16_t size) noexcept { return SkipSpriteData(*f, type, size); },
		.open_cached = [](const GRFConfig *c, Subdirectory subdir) noexcept { return &OpenCachedSpriteFile(c->filename, subdir, c->palette & GRFP_USE_MASK); },
		.open_temporary = [](const GRFConfig *c, Subdirectory subdir, void *temporary) noexcept { return &static_cast<std::optional<SpriteFile> *>(temporary)->emplace(c->filename, subdir, c->palette & GRFP_USE_MASK); },
		.scan_name = [](GRFConfig *c, uint32_t id, const uint8_t *data, size_t size) noexcept { AddGRFTextToList(c->name, 0x7F, id, false, {reinterpret_cast<const char *>(data), size}); },
		.scan_info = [](GRFConfig *c, uint32_t id, const uint8_t *data, size_t size) noexcept { AddGRFTextToList(c->info, 0x7F, id, true, {reinterpret_cast<const char *>(data), size}); },
		.set_pitch = [](GRFFile *f, uint32_t value) noexcept { f->traininfo_vehicle_pitch = value; },
		.set_width = [](GRFFile *f, uint32_t value) noexcept { f->traininfo_vehicle_width = value; },
		.set_rail_cost = [](RailType rt, uint16_t value) noexcept { extern RailTypeInfo _railtypes[RAILTYPE_END]; RustSetRailCostMultiplier(&_railtypes[rt], value); },
		.set_misc = [](uint8_t value) noexcept { _misc_grf_features = GrfMiscBits(value); },
		.safe_change_info = ParseGrfAction<&GrfActionHandler<0x00>::SafetyScan>,
		.reserve_change_info = ParseGrfAction<&GrfActionHandler<0x00>::Reserve>,
		.feature_change_info = ParseGrfAction<&GrfActionHandler<0x00>::Activation>,
		.skip_act1 = ParseGrfAction<&GrfActionHandler<0x01>::FileScan>,
		.new_sprite_set = ParseGrfAction<&GrfActionHandler<0x01>::Activation>,
		.feature_map_sprite_group = ParseGrfAction<&GrfActionHandler<0x03>::Activation>,
		.feature_new_name = ParseGrfAction<&GrfActionHandler<0x04>::Activation>,
		.skip_act5 = ParseGrfAction<&GrfActionHandler<0x05>::FileScan>,
		.graphics_new = ParseGrfAction<&GrfActionHandler<0x05>::Activation>,
		.skip_act_a = ParseGrfAction<&GrfActionHandler<0x0A>::FileScan>,
		.sprite_replace = ParseGrfAction<&GrfActionHandler<0x0A>::Activation>,
		.grf_load_error = ParseGrfAction<&GrfActionHandler<0x0B>::Init>,
		.grf_comment = ParseGrfAction<&GrfActionHandler<0x0C>::Init>,
		.skip_act11 = ParseGrfAction<&GrfActionHandler<0x11>::FileScan>,
		.grf_sound = ParseGrfAction<&GrfActionHandler<0x11>::Init>,
		.skip_act12 = ParseGrfAction<&GrfActionHandler<0x12>::FileScan>,
		.load_font_glyph = ParseGrfAction<&GrfActionHandler<0x12>::Activation>,
		.translate_grf_strings = ParseGrfAction<&GrfActionHandler<0x13>::Activation>,
		.static_grf_info = ParseGrfAction<&GrfActionHandler<0x14>::FileScan>,
		.never_expire_airports = RustNeverExpireAirports,
		.max_bridge_length = RustMaxBridgeLength,
		.never_expire_vehicles = RustNeverExpireVehicles,
		.station_noise = RustStationNoiseEnabled,
		.gradual_loading = RustGradualLoading,
		.signal_side = RustTrainSignalSide,
		.disable_elrails = RustDisableElrails,
		.build_on_slopes = RustBuildOnSlopes,
		.freight_trains = RustFreightTrains,
		.wagon_speed_limits = RustWagonSpeedLimits,
		.allow_town_roads = RustAllowTownRoads,
		.generating_world = OpenTTDGeneratingWorld,
		.improved_load = RustImprovedLoad,
		.dynamic_engines = RustDynamicEngines,
		.inflation = RustInflation,
		.starting_year = RustStartingYear,
		.plane_speed = RustPlaneSpeed,
		.height_limit = RustMapHeightLimit,
		.generation_seed = RustGenerationSeed,
		.log_x = OpenTTDMapLogX,
		.log_y = OpenTTDMapLogY,
		.read_bounds = STR_NEWGRF_ERROR_READ_BOUNDS,
		.unexpected_sprite = STR_NEWGRF_ERROR_UNEXPECTED_SPRITE,
		.multiple_action8 = STR_NEWGRF_ERROR_MULTIPLE_ACTION_8,
		.grm_failed = STR_NEWGRF_ERROR_GRM_FAILED,
		.static_desync = STR_NEWGRF_ERROR_STATIC_GRF_CAUSES_DESYNC,
		.list_property_too_long = STR_NEWGRF_ERROR_LIST_PROPERTY_TOO_LONG,
		.invalid_id = STR_NEWGRF_ERROR_INVALID_ID,
		.force_disabled = STR_NEWGRF_ERROR_FORCEFULLY_DISABLED,
		.colourmap_sprite = SPR_2CCMAP_BASE,
		.slopes_sprite = SPR_SLOPES_BASE,
		.shore_sprite = SPR_SHORE_BASE,
		.generic = &GetRustGenericServices(),
		.clock = []() noexcept { return OpenTTDGrfClock{TimerGameTick::counter, _display_opt}; },
		.set_clock = [](const OpenTTDGrfClock *c) noexcept { TimerGameTick::counter = c->ticks; _display_opt = c->display; },
		.using_wallclock_units = RustUsingWallclockUnits,
		.config_at = [](size_t i) noexcept { return i < _grfconfig.size() ? _grfconfig[i].get() : nullptr; },
		.exists = [](const GRFConfig *c, Subdirectory subdir) noexcept { return FioCheckFileExists(c->filename, subdir); },
		.init_file = [](const GRFConfig *c) noexcept { InitNewGRFFile(*c); },
		.too_many = [](GRFConfig *c) noexcept { Debug(grf, 0, "'{}' is not loaded as the maximum number of non-static GRFs has been reached", c->filename); c->errors.emplace_back(STR_NEWGRF_ERROR_MSG_FATAL, 0, STR_NEWGRF_ERROR_TOO_MANY_NEWGRFS_LOADED); },
		.clear_errors = [](GRFConfig *c) noexcept { c->errors.clear(); },
		.cleanup_strings = []() noexcept { CleanUpStrings(); },
		.reset_badges = []() noexcept { ResetBadges(); },
		.reset_bridges = []() noexcept { ResetBridges(); },
		.reset_prices = []() noexcept { ResetPriceBaseMultipliers(); },
		.reset_currencies = []() noexcept { ResetCurrencies(); },
		.object_classes = &ObjectClass::GetServices(),
		.reset_objects = []() noexcept { ResetObjects(); },
		.station_classes = &StationClass::GetServices(),
		.airport_classes = &AirportClass::GetServices(),
		.roadstop_classes = &RoadStopClass::GetServices(),
		.reset_water_features = []() noexcept { _water_feature.fill({}); },
		.reset_faces = []() noexcept { ResetFaces(); },
		.reset_files = []() noexcept { _grf_files.clear(); _grf_files.reserve(_grfconfig.size()); },
		.reset_loaded_features = []() noexcept { _loaded_newgrf_features.has_2CC = false; _loaded_newgrf_features.used_liveries = 1 << LS_DEFAULT; _loaded_newgrf_features.shore = SHORE_REPLACE_NONE; _loaded_newgrf_features.tram = TRAMWAY_REPLACE_DEPOT_NONE; },
		.initialize_sounds = []() noexcept { InitializeSoundPool(); },
		.clean_spritegroups = []() noexcept { _spritegroup_pool.CleanPool(); },
		.finalise_strings = []() noexcept { FinaliseStringMapping(); },
		.finalise_badges = []() noexcept { FinaliseBadges(); },
		.finalise_canals = []() noexcept { FinaliseCanals(); },
		.finalise_objects = []() noexcept { FinaliseObjectsArray(); },
		.sort_industries = []() noexcept { SortIndustryTypes(); },
		.industry_legend = []() noexcept { BuildIndustriesLegend(); },
		.linkstats_legend = []() noexcept { BuildLinkStatsLegend(); },
		.activate_shore = []() noexcept { ActivateOldShore(); },
		.activate_tram_depot = []() noexcept { ActivateOldTramDepot(); },
		.aging_stops = []() noexcept { SetYearEngineAgingStops(); },
		.finalise_prices = []() noexcept { FinalisePriceBaseMultipliers(); },
		.baseset_dir = BASESET_DIR,
		.newgrf_dir = NEWGRF_DIR,
		.assert_override_size = []([[maybe_unused]] size_t actual, [[maybe_unused]] uint32_t expected) noexcept { assert(actual == expected); },
		.assert_file_match = []([[maybe_unused]] const GRFConfig *c, [[maybe_unused]] const GRFFile *f) noexcept { assert(GetFileByGRFID(c->ident.grfid) == f); },
		.unexpected_large = []() noexcept { GrfMsg(0, "LoadNewGRFFile: Unexpectedly large sprite, disabling"); },
		.unexpected_real = []() noexcept { GrfMsg(0, "LoadNewGRFFile: Unexpected sprite, disabling"); },
		.missing_file = [](const GRFConfig *c) noexcept { Debug(grf, 0, "NewGRF file is missing '{}'; disabling", c->filename); },
		.sprite_grm_failed = [](uint16_t count) noexcept { GrfMsg(0, "ParamSet: GRM: Unable to allocate {} sprites; try changing NewGRF order", count); },
		.grm_failure = [](uint16_t count, bool engines, bool fatal) noexcept { std::string_view type = engines ? "vehicles" : "cargoes"; if (fatal) { GrfMsg(0, "ParamSet: GRM: Unable to allocate {} {}, deactivating", count, type); } else { GrfMsg(1, "ParamSet: GRM: Unable to allocate {} {}", count, type); } },
		.unknown_operation = [](uint8_t op) noexcept { GrfMsg(0, "ParamSet: Unknown operation {}, skipping", op); },
		.id_mismatch = [](uint32_t old_id, uint32_t new_id) noexcept { Debug(grf, 0, "GRFInfo: GRFID {:08X} in FILESCAN stage does not match GRFID {:08X} in INIT/RESERVE/ACTIVATION stage", std::byteswap(old_id), std::byteswap(new_id)); },
		.invalid_version = [](const GRFConfig *c, uint32_t id, uint8_t version, const uint8_t *name, size_t len) noexcept { Debug(grf, 0, "{}: NewGRF \"{}\" (GRFID {:08X}) uses GRF version {}, which is incompatible with this version of OpenTTD.", c->filename, StrMakeValid(std::string_view{reinterpret_cast<const char *>(name), len}), std::byteswap(id), version); },
		.null_error = STR_NULL,
		.unknown_action = [](uint8_t action) noexcept { GrfMsg(7, "DecodeSpecialSprite: Skipping unknown action 0x{:02X}", action); },
		.handling_action = [](uint8_t action, GrfLoadingStage stage) noexcept { GrfMsg(7, "DecodeSpecialSprite: Handling action 0x{:02X} in stage {}", action, stage); },
		.preloaded_sprite = []() noexcept { GrfMsg(7, "DecodeSpecialSprite: Using preloaded pseudo sprite data"); },
		.unexpected_data = []() noexcept { GrfMsg(2, "DecodeSpecialSprite: Unexpected data block, skipping"); },
		.unexpected_import = []() noexcept { GrfMsg(2, "DecodeSpecialSprite: Unexpected import block, skipping"); },
		.read_bounds_error = []() noexcept { GrfMsg(1, "DecodeSpecialSprite: Tried to read past end of pseudo-sprite data"); },
		.reading_file = [](const GRFConfig *c) noexcept { Debug(grf, 2, "LoadNewGRFFile: Reading NewGRF-file '{}'", c->filename); },
		.invalid_format = []() noexcept { Debug(grf, 7, "LoadNewGRFFile: Custom .grf has invalid format"); },
		.unsupported_compression = []() noexcept { Debug(grf, 7, "LoadNewGRFFile: Unsupported compression format"); },
		.loaded_sprites = [](SpriteID count) noexcept { Debug(sprite, 2, "LoadNewGRF: Currently {} sprites are loaded", count); },
		.unsupported_variable = [](uint8_t param) noexcept { GrfMsg(1, "Unsupported in-game variable 0x{:02X}", param); },
		.cfg_real = []() noexcept { GrfMsg(2, "CfgApply: Ignoring (next sprite is real, unsupported)"); },
		.cfg_unset = [](uint32_t param) noexcept { GrfMsg(2, "CfgApply: Ignoring (param {} not set)", param); },
		.cfg_applying = [](uint32_t size, uint32_t param, uint32_t offset) noexcept { GrfMsg(8, "CfgApply: Applying {} bytes from parameter 0x{:02X} at offset 0x{:04X}", size, param, offset); },
		.skip_undefined = [](uint8_t param) noexcept { GrfMsg(7, "SkipIf: Param {} undefined, skipping test", param); },
		.skip_test = [](uint8_t cond, uint8_t param, uint32_t value) noexcept { GrfMsg(7, "SkipIf: Test condtype {}, param 0x{:02X}, condval 0x{:08X}", cond, param, value); },
		.skip_condition = [](uint8_t cond) noexcept { GrfMsg(1, "SkipIf: Unsupported condition type {:02X}. Ignoring", cond); },
		.skip_grf_condition = [](uint8_t cond) noexcept { GrfMsg(1, "SkipIf: Unsupported GRF condition type {:02X}. Ignoring", cond); },
		.skip_unknown_grf = [](uint32_t id) noexcept { GrfMsg(7, "SkipIf: GRFID 0x{:08X} unknown, skipping test", std::byteswap(id)); },
		.skip_false = []() noexcept { GrfMsg(2, "SkipIf: Not skipping sprites, test was false"); },
		.skip_jump = [](uint8_t label, uint32_t line) noexcept { GrfMsg(2, "SkipIf: Jumping to label 0x{:X} at line {}, test was true", label, line); },
		.skip_count = [](uint8_t count) noexcept { GrfMsg(2, "SkipIf: Skipping {} sprites, test was true", count); },
		.loaded_info = [](const GRFConfig *c, uint8_t version, uint32_t id, const uint8_t *name, size_t len) noexcept { Debug(grf, 1, "GRFInfo: Loaded GRFv{} set {:08X} - {} (palette: {}, version: {})", version, std::byteswap(id), StrMakeValid(std::string_view{reinterpret_cast<const char *>(name), len}), (c->palette & GRFP_USE_MASK) ? "Windows" : "DOS", c->version); },
		.inhibiting = [](const GRFConfig *c) noexcept { GrfMsg(2, "GRFInhibit: Deactivating file '{}'", c->filename); },
		.defined_label = [](uint8_t label) noexcept { GrfMsg(2, "DefineGotoLabel: GOTO target with label 0x{:02X}", label); },
		.unknown_patch = [](uint8_t param) noexcept { GrfMsg(2, "ParamSet: Unknown Patch variable 0x{:02X}.", param); },
		.grm_reserved = [](uint16_t count, bool engines, uint32_t start) noexcept { GrfMsg(2, "ParamSet: GRM: Reserving {} {} at {}", count, engines ? "vehicles" : "cargoes", start); },
		.param_defined = [](uint8_t target) noexcept { GrfMsg(7, "ParamSet: Param {} already defined, skipping", target); },
		.sprites_allocated = [](uint16_t count, SpriteID sprite) noexcept { GrfMsg(4, "ParamSet: GRM: Allocated {} sprites at {}", count, sprite); },
		.sprites_using = [](SpriteID sprite) noexcept { GrfMsg(4, "ParamSet: GRM: Using pre-allocated sprites at {}", sprite); },
		.sprite_operation = [](uint8_t op) noexcept { GrfMsg(1, "ParamSet: GRM: Unsupported operation {} for general sprites", op); },
		.grm_feature = [](GrfSpecFeature feature) noexcept { GrfMsg(1, "ParamSet: GRM: Unsupported feature 0x{:X}", feature); },
		.unimplemented_target = [](uint8_t target) noexcept { GrfMsg(7, "ParamSet: Skipping unimplemented target 0x{:02X}", target); },
		.unknown_target = [](uint8_t target) noexcept { GrfMsg(7, "ParamSet: Skipping unknown target 0x{:02X}", target); },
		.translate_text = [](uint32_t grfid, uint8_t lang, bool allow_newlines, const uint8_t *text, size_t len) noexcept -> OpenTTDByteSpan {
			auto &result = *_cur_gps.translated_text;
			result = TranslateTTDPatchCodes(grfid, lang, allow_newlines, {reinterpret_cast<const char *>(text), len});
			return {reinterpret_cast<const uint8_t *>(result.data()), result.size()};
		},
		.add_string = [](uint32_t grfid, uint32_t stringid, uint8_t lang, bool new_scheme, bool allow_newlines, const uint8_t *text, size_t len, StringID def_string) noexcept { return AddGRFString(grfid, GRFStringID{stringid}, lang, new_scheme, allow_newlines, {reinterpret_cast<const char *>(text), len}, def_string); },
		.undefined = STR_UNDEFINED,
		.town_definition = [](uint8_t id) noexcept { GrfMsg(6, "FeatureTownName: definition 0x{:02X}", id); },
		.town_lang = [](uint8_t lang, const uint8_t *name, size_t len) noexcept { GrfMsg(6, "FeatureTownName: lang 0x{:X} -> '{}'", lang, std::string_view{reinterpret_cast<const char *>(name), len}); },
		.town_parts = [](uint8_t parts) noexcept { GrfMsg(6, "FeatureTownName: {} parts", parts); },
		.town_part = [](uint32_t partnum, uint8_t texts, uint8_t bitstart, uint8_t bitcount) noexcept { GrfMsg(6, "FeatureTownName: part {} contains {} texts and will use GB(seed, {}, {})", partnum, texts, bitstart, bitcount); },
		.town_missing = [](uint8_t ref_id) noexcept { GrfMsg(0, "FeatureTownName: definition 0x{:02X} doesn't exist, deactivating", ref_id); },
		.town_reference = [](uint32_t partnum, uint32_t textnum, uint8_t ref_id, uint8_t prob) noexcept { GrfMsg(6, "FeatureTownName: part {}, text {}, uses intermediate definition 0x{:02X} (with probability {})", partnum, textnum, ref_id, prob); },
		.town_text = [](uint32_t partnum, uint32_t textnum, const uint8_t *text, size_t len, uint8_t prob) noexcept { GrfMsg(6, "FeatureTownName: part {}, text {}, '{}' (with probability {})", partnum, textnum, std::string_view{reinterpret_cast<const char *>(text), len}, prob); },
		.town_total = [](uint32_t partnum, uint16_t maxprob) noexcept { GrfMsg(6, "FeatureTownName: part {}, total probability {}", partnum, maxprob); },
	};
	return services;
}
#include "abi-visit-OpenTTDAirportDefinitionServices.hpp"
#include "abi-visit-OpenTTDGrfLoadingServices.hpp"
#include "abi-visit-OpenTTDAction2Services.hpp"
#include "abi-visit-OpenTTDHouseDefinitionServices.hpp"
#include "abi-visit-OpenTTDStationClassServices.hpp"
#include "abi-visit-OpenTTDRoadStopClassServices.hpp"
#include "abi-visit-OpenTTDAirportClassServices.hpp"
#include "abi-visit-OpenTTDObjectClassServices.hpp"
bool CheckAbi_grf_loading()
{
	const auto &s = GetRustGrfLoadingServices();
	return AbiServicesComplete(GetRustAction2Services()) && AbiServicesComplete(s) && AbiServicesComplete(*s.houses) && AbiServicesComplete(*s.airports) && AbiServicesComplete(*s.station_classes) && AbiServicesComplete(*s.roadstop_classes) && AbiServicesComplete(*s.airport_classes) && AbiServicesComplete(*s.object_classes);
}

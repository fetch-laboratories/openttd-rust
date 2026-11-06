/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading_services.inc Typed native loading services. */
#include "newgrf_loading_ffi.h"
#include "clock_ffi.h"
#include "company_adapter.h"
#include "newgrf_act2_ffi.h"
#include "industry_definition_ffi.h"
#include "airport_definitions_ffi.h"
#include "type_registry_ffi.h"
#include "newgrf_global_ffi.h"
#include "generic_ffi.h"
#include "../string_func.h"
#include "../strings_func.h"
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
		.clocks = &GetRustClockServices(),
		.finance = &GetRustCompanyFinanceServices(),
		.global = &GetRustGlobalVariableServices(),
		.cargo = &GetRustCargoRegistryServices(),
		.engine = &GetRustEngineLoadingServices(),
		.houses = &GetHouseDefinitionServices(),
		.industry = &GetIndustryDefinitionServices(),
		.airports = &GetAirportDefinitionServices(),
		.types = &GetRustTypeRegistryServices(),
		.action2 = &GetRustAction2Services(),
		.text = &GetRustGrfTextServices(),
		.bindings = &GetRustGrfBindingServices(),
		.stations = &GetStationDefinitionServices(),
		.engine_range = [](GrfSpecFeature feature) noexcept { return OpenTTDGrfEngineRange{_engine_offsets[feature], _engine_counts[feature]}; },
		.config_md5 = [](const GRFConfig *c) noexcept -> OpenTTDByteSpan { return {c->ident.md5sum.data(), c->ident.md5sum.size()}; },
		.config_id = [](const GRFConfig *c) noexcept { return c->ident.grfid; },
		.config_status = [](const GRFConfig *c) noexcept { return c->status; },
		.config_flags = [](const GRFConfig *c) noexcept { return c->flags.base(); },
		.set_status = [](GRFConfig *c, GRFStatus status) noexcept { c->status = status; },
		.set_flag = [](GRFConfig *c, uint8_t flag, bool set) noexcept { c->flags.Set(static_cast<GRFConfigFlag>(flag), set); },
		.set_config_id = [](GRFConfig *c, uint32_t id) noexcept { c->ident.grfid = id; },
		.set_file_info = [](GRFFile *f, uint32_t id, uint8_t version) noexcept { f->grfid = id; f->grf_version = version; },
		.config_version = [](const GRFConfig *c) noexcept { return c->version; },
		.config_filename = [](const GRFConfig *c) noexcept { return OpenTTDByteSpan{reinterpret_cast<const uint8_t *>(c->filename.data()), c->filename.size()}; },
		.config_palette = [](const GRFConfig *c) noexcept { return c->palette; },
		.param_count = [](const GRFFile *f) noexcept { return f->param.size(); },
		.set_parameter = [](GRFFile *f, uint8_t target, uint32_t value) noexcept { if (target >= f->param.size()) f->param.resize(target + 1); f->param[target] = value; },
		.find_config = [](uint32_t id, uint32_t mask) noexcept { return GetGRFConfig(id, mask); },
		.file_for_config = [](const GRFConfig *c) noexcept { auto *f = GetFileByFilename(c->filename); if (f == nullptr) UserError("File '{}' lost in cache.\n", c->filename); return f; },
		.error = [](GRFConfig *c, uint32_t line, StringID message, bool current) noexcept { auto it = std::ranges::find(c->errors, line, &GRFError::nfo_line); if (it == c->errors.end()) it = c->errors.emplace(it, STR_NEWGRF_ERROR_MSG_FATAL, line, message); if (current) it->param_value[0] = line; return &*it; },
		.error_name = [](GRFError *e, const GRFConfig *c) noexcept { e->data = c->GetName(); },
		.error_property = RustGRFErrorProperty,
		.error_after_translated_file = [](GRFError *error) noexcept { error->data = GetString(STR_NEWGRF_ERROR_AFTER_TRANSLATED_FILE); },
		.load_after = STR_NEWGRF_ERROR_LOAD_AFTER,
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
		.static_grf_info = ParseGrfAction<&GrfActionHandler<0x14>::FileScan>,
		.gradual_loading = RustGradualLoading,
		.signal_side = RustTrainSignalSide,
		.freight_trains = RustFreightTrains,
		.wagon_speed_limits = RustWagonSpeedLimits,
		.improved_load = RustImprovedLoad,
		.inflation = RustInflation,
		.starting_year = RustStartingYear,
		.plane_speed = RustPlaneSpeed,
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
		.config_at = [](size_t i) noexcept { return i < _grfconfig.size() ? _grfconfig[i].get() : nullptr; },
		.exists = [](const GRFConfig *c, Subdirectory subdir) noexcept { return FioCheckFileExists(c->filename, subdir); },
		.init_file = [](const GRFConfig *c) noexcept { InitNewGRFFile(*c); },
		.too_many = [](GRFConfig *c) noexcept { c->errors.emplace_back(STR_NEWGRF_ERROR_MSG_FATAL, 0, STR_NEWGRF_ERROR_TOO_MANY_NEWGRFS_LOADED); },
		.clear_errors = [](GRFConfig *c) noexcept { c->errors.clear(); },
		.reset_badges = []() noexcept { ResetBadges(); },
		.reset_bridges = []() noexcept { ResetBridges(); },
		.reset_prices = []() noexcept { ResetPriceBaseMultipliers(); },
		.reset_currencies = []() noexcept { ResetCurrencies(); },
		.object_classes = &ObjectClass::GetServices(),
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
		.null_error = STR_NULL,
		.undefined = STR_UNDEFINED,
	};
	return services;
}
#include "abi-visit-OpenTTDAirportDefinitionServices.hpp"
#include "abi-visit-OpenTTDGrfBindingServices.hpp"
#include "abi-visit-OpenTTDGrfLoadingServices.hpp"
#include "abi-visit-OpenTTDGrfTextServices.hpp"
#include "abi-visit-OpenTTDAction2Services.hpp"
#include "abi-visit-OpenTTDHouseDefinitionServices.hpp"
#include "abi-visit-OpenTTDStationClassServices.hpp"
#include "abi-visit-OpenTTDRoadStopClassServices.hpp"
#include "abi-visit-OpenTTDAirportClassServices.hpp"
bool CheckAbi_grf_loading()
{
	const auto &s = GetRustGrfLoadingServices();
	return AbiServicesComplete(GetRustAction2Services()) && AbiServicesComplete(s) && AbiServicesComplete(*s.bindings) && AbiServicesComplete(*s.text) && AbiServicesComplete(*s.houses) && AbiServicesComplete(*s.airports) && AbiServicesComplete(*s.station_classes) && AbiServicesComplete(*s.roadstop_classes) && AbiServicesComplete(*s.airport_classes);
}

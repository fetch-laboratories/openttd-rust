#ifdef WITH_RUST
#include "rust/newgrf_loading_ffi.h"
#include "rust/newgrf_global_ffi.h"
#include "rust/services_newgrf.h"
#include "rust/services_cargo.h"
#include "rust/settings_services.h"
#include "rust/environment_services.h"
#include "rust/services_map.h"
#include "rust/service_table_check.hpp"
/** Each parser catches exactly the DecodeSpecialSprite domain, retaining partial effects. */
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
		.find_file = [](uint32_t id) noexcept { return GetFileByGRFID(id); },
		.file_for_config = [](const GRFConfig *c) noexcept { auto *f = GetFileByFilename(c->filename); if (f == nullptr) UserError("File '{}' lost in cache.\n", c->filename); return f; },
		.error = [](GRFConfig *c, uint32_t line, StringID message, bool current) noexcept { auto it = std::ranges::find(c->errors, line, &GRFError::nfo_line); if (it == c->errors.end()) it = c->errors.emplace(it, STR_NEWGRF_ERROR_MSG_FATAL, line, message); if (current) it->param_value[0] = line; return &*it; },
		.error_name = [](GRFError *e, const GRFConfig *c) noexcept { e->data = c->GetName(); },
		.networking = []() noexcept { return _networking; },
		.cargo_label = OpenTTDCargoByLabel,
		.rail_label = [](RailTypeLabel label) noexcept { return GetRailTypeByLabel(label); },
		.road_label = [](RoadTypeLabel label) noexcept { return GetRoadTypeByLabel(label); },
		.road_is_road = [](RoadType rt) noexcept { return RustRoadTypeIsRoad(rt); },
		.road_is_tram = SharedRoadTypeIsTram,
		.read_byte = [](SpriteFile *f) noexcept { return f->ReadByte(); },
		.read_word = [](SpriteFile *f) noexcept { return f->ReadWord(); },
		.read_dword = [](SpriteFile *f) noexcept { return f->ReadDword(); },
		.read_block = [](SpriteFile *f, uint8_t *data, size_t size) noexcept { f->ReadBlock(data, size); },
		.position = [](const SpriteFile *f) noexcept { return f->GetPos(); },
		.seek = [](SpriteFile *f, size_t pos, bool relative) noexcept { f->SeekTo(pos, relative ? SEEK_CUR : SEEK_SET); },
		.skip = [](SpriteFile *f, size_t size) noexcept { f->SkipBytes(size); },
		.container_version = [](const SpriteFile *f) noexcept { return f->GetContainerVersion(); },
		.sprite_offsets = [](SpriteFile *f) noexcept { ReadGRFSpriteOffsets(*f); },
		.skip_sprite_data = [](SpriteFile *f, uint8_t type, uint32_t size) noexcept { SkipSpriteData(*f, type, size); },
		.open_file = [](const GRFConfig *c, Subdirectory subdir, void *temporary) noexcept { bool remap = c->palette & GRFP_USE_MASK; if (temporary != nullptr) { auto &slot = *static_cast<std::optional<SpriteFile> *>(temporary); return &slot.emplace(c->filename, subdir, remap); } return &OpenCachedSpriteFile(c->filename, subdir, remap); },
		.scan_name = [](GRFConfig *c, uint32_t id, const uint8_t *data, size_t size) noexcept { AddGRFTextToList(c->name, 0x7F, id, false, {reinterpret_cast<const char *>(data), size}); },
		.scan_info = [](GRFConfig *c, uint32_t id, const uint8_t *data, size_t size) noexcept { AddGRFTextToList(c->info, 0x7F, id, true, {reinterpret_cast<const char *>(data), size}); },
		.set_pitch = [](GRFFile *f, uint32_t value) noexcept { f->traininfo_vehicle_pitch = value; },
		.set_width = [](GRFFile *f, uint32_t value) noexcept { f->traininfo_vehicle_width = value; },
		.set_rail_cost = [](RailType rt, uint16_t value) noexcept { extern RailTypeInfo _railtypes[RAILTYPE_END]; _railtypes[rt].cost_multiplier = value; },
		.set_misc = [](uint8_t value) noexcept { _misc_grf_features = GrfMiscBits(value); },
		.safe_change_info = ParseGrfAction<&GrfActionHandler<0x00>::SafetyScan>,
		.reserve_change_info = ParseGrfAction<&GrfActionHandler<0x00>::Reserve>,
		.feature_change_info = ParseGrfAction<&GrfActionHandler<0x00>::Activation>,
		.skip_act1 = ParseGrfAction<&GrfActionHandler<0x01>::FileScan>,
		.new_sprite_set = ParseGrfAction<&GrfActionHandler<0x01>::Activation>,
		.new_sprite_group = ParseGrfAction<&GrfActionHandler<0x02>::Activation>,
		.grf_unsafe = ParseGrfAction<&GrfActionHandler<0x03>::SafetyScan>,
		.feature_map_sprite_group = ParseGrfAction<&GrfActionHandler<0x03>::Activation>,
		.feature_new_name = ParseGrfAction<&GrfActionHandler<0x04>::Activation>,
		.skip_act5 = ParseGrfAction<&GrfActionHandler<0x05>::FileScan>,
		.graphics_new = ParseGrfAction<&GrfActionHandler<0x05>::Activation>,
		.skip_act_a = ParseGrfAction<&GrfActionHandler<0x0A>::FileScan>,
		.sprite_replace = ParseGrfAction<&GrfActionHandler<0x0A>::Activation>,
		.grf_load_error = ParseGrfAction<&GrfActionHandler<0x0B>::Init>,
		.grf_comment = ParseGrfAction<&GrfActionHandler<0x0C>::Init>,
		.feature_town_name = ParseGrfAction<&GrfActionHandler<0x0F>::Init>,
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
		.force_disabled = STR_NEWGRF_ERROR_FORCEFULLY_DISABLED,
		.colourmap_sprite = SPR_2CCMAP_BASE,
		.slopes_sprite = SPR_SLOPES_BASE,
		.shore_sprite = SPR_SHORE_BASE,
		.generic = &GetRustGenericServices(),
		.clock = []() noexcept { return OpenTTDGrfClock{TimerGameCalendar::date.base(), TimerGameCalendar::year.base(), TimerGameCalendar::date_fract, TimerGameEconomy::date.base(), TimerGameEconomy::year.base(), TimerGameEconomy::date_fract, TimerGameTick::counter, _display_opt}; },
		.set_clock = [](const OpenTTDGrfClock *c) noexcept { TimerGameCalendar::date = TimerGameCalendar::Date{c->date}; TimerGameCalendar::year = TimerGameCalendar::Year{c->year}; TimerGameCalendar::date_fract = c->date_fract; TimerGameEconomy::date = TimerGameEconomy::Date{c->economy_date}; TimerGameEconomy::year = TimerGameEconomy::Year{c->economy_year}; TimerGameEconomy::date_fract = c->economy_date_fract; TimerGameTick::counter = c->ticks; _display_opt = c->display; },
		.economy_ymd_to_date = [](int32_t y, uint8_t m, uint8_t d) noexcept { return TimerGameEconomy::ConvertYMDToDate(TimerGameEconomy::Year{y}, m, d).base(); },
		.config_at = [](size_t i) noexcept { return i < _grfconfig.size() ? _grfconfig[i].get() : nullptr; },
		.exists = [](const GRFConfig *c, Subdirectory subdir) noexcept { return FioCheckFileExists(c->filename, subdir); },
		.init_file = [](const GRFConfig *c) noexcept { InitNewGRFFile(*c); },
		.override_grf = [](uint32_t source, uint32_t target) noexcept { SetNewGRFOverride(source, target); },
		.too_many = [](GRFConfig *c) noexcept { c->errors.emplace_back(STR_NEWGRF_ERROR_MSG_FATAL, 0, STR_NEWGRF_ERROR_TOO_MANY_NEWGRFS_LOADED); },
		.clear_errors = [](GRFConfig *c) noexcept { c->errors.clear(); },
		.cleanup_strings = []() noexcept { CleanUpStrings(); },
		.cleanup_townnames = []() noexcept { CleanUpGRFTownNames(); },
		.reset_badges = []() noexcept { ResetBadges(); },
		.setup_engines = []() noexcept { SetupEngines(); },
		.reset_bridges = []() noexcept { ResetBridges(); },
		.reset_railtypes = []() noexcept { ResetRailTypes(); },
		.reset_roadtypes = []() noexcept { ResetRoadTypes(); },
		.init_engine_loading = []() noexcept { _gted.resize(Engine::GetPoolSize()); for (const Engine *e : Engine::IterateType(VEH_TRAIN)) { _gted[e->index].railtypelabels.clear(); for (RailType rt : e->VehInfo<RailVehicleInfo>().railtypes) _gted[e->index].railtypelabels.push_back(GetRailTypeInfo(rt)->label); } },
		.reset_prices = []() noexcept { ResetPriceBaseMultipliers(); },
		.reset_currencies = []() noexcept { ResetCurrencies(); },
		.reset_houses = []() noexcept { ResetHouses(); },
		.reset_industries = []() noexcept { ResetIndustries(); },
		.reset_object_classes = []() noexcept { ObjectClass::Reset(); },
		.reset_objects = []() noexcept { ResetObjects(); },
		.reset_station_classes = []() noexcept { StationClass::Reset(); },
		.reset_airport_classes = []() noexcept { AirportClass::Reset(); },
		.reset_airports = []() noexcept { AirportSpec::ResetAirports(); },
		.reset_airporttiles = []() noexcept { AirportTileSpec::ResetAirportTiles(); },
		.reset_roadstop_classes = []() noexcept { RoadStopClass::Reset(); },
		.reset_water_features = []() noexcept { _water_feature.fill({}); },
		.reset_faces = []() noexcept { ResetFaces(); },
		.clear_snowline = []() noexcept { ClearSnowLine(); },
		.reset_files = []() noexcept { _grf_files.clear(); _grf_files.reserve(_grfconfig.size()); },
		.setup_cargo = []() noexcept { SetupCargoForClimate(_settings_game.game_creation.landscape); },
		.reset_loaded_features = []() noexcept { _loaded_newgrf_features.has_2CC = false; _loaded_newgrf_features.used_liveries = 1 << LS_DEFAULT; _loaded_newgrf_features.shore = SHORE_REPLACE_NONE; _loaded_newgrf_features.tram = TRAMWAY_REPLACE_DEPOT_NONE; },
		.clear_engine_overrides = []() noexcept { _grf_id_overrides.clear(); },
		.initialize_sounds = []() noexcept { InitializeSoundPool(); },
		.clean_spritegroups = []() noexcept { _spritegroup_pool.CleanPool(); },
		.reset_callbacks = []() noexcept { ResetCallbacks(false); },
		.finish_callbacks = []() noexcept { ResetCallbacks(true); },
		.finalise_strings = []() noexcept { FinaliseStringMapping(); },
		.finalise_badges = []() noexcept { FinaliseBadges(); },
		.finalise_cargo = []() noexcept { FinaliseCargoArray(); },
		.calculate_refits = []() noexcept { CalculateRefitMasks(); },
		.finalise_engines = []() noexcept { FinaliseEngineArray(); },
		.finalise_canals = []() noexcept { FinaliseCanals(); },
		.finalise_houses = []() noexcept { FinaliseHouseArray(); },
		.finalise_industries = []() noexcept { FinaliseIndustriesArray(); },
		.finalise_objects = []() noexcept { FinaliseObjectsArray(); },
		.sort_cargo = []() noexcept { InitializeSortedCargoSpecs(); },
		.sort_industries = []() noexcept { SortIndustryTypes(); },
		.industry_legend = []() noexcept { BuildIndustriesLegend(); },
		.linkstats_legend = []() noexcept { BuildLinkStatsLegend(); },
		.finalise_airports = []() noexcept { FinaliseAirportsArray(); },
		.bind_airports = []() noexcept { BindAirportSpecs(); },
		.townnames = []() noexcept { InitGRFTownGeneratorNames(); },
		.vehicle_list_orders = []() noexcept { CommitVehicleListOrderChanges(); },
		.activate_shore = []() noexcept { ActivateOldShore(); },
		.activate_tram_depot = []() noexcept { ActivateOldTramDepot(); },
		.init_railtypes = []() noexcept { InitRailTypes(); },
		.init_roadtypes = []() noexcept { InitRoadTypes(); },
		.aging_stops = []() noexcept { SetYearEngineAgingStops(); },
		.finalise_prices = []() noexcept { FinalisePriceBaseMultipliers(); },
		.clear_engine_loading = []() noexcept { _gted.clear(); },
		.cargo_translation_map = []() noexcept { BuildCargoTranslationMap(); },
		.bind_engine_types = []() noexcept {
	for (Engine *e : Engine::IterateType(VEH_ROAD)) {
		if (_gted[e->index].rv_max_speed != 0) {
			/* Set RV maximum speed from the mph/0.8 unit value */
			e->VehInfo<RoadVehicleInfo>().max_speed = _gted[e->index].rv_max_speed * 4;
		}

		RoadTramType rtt = e->info.misc_flags.Test(EngineMiscFlag::RoadIsTram) ? RTT_TRAM : RTT_ROAD;

		const GRFFile *file = e->GetGRF();
		if (file == nullptr || _gted[e->index].roadtramtype == 0) {
			e->VehInfo<RoadVehicleInfo>().roadtype = (rtt == RTT_TRAM) ? ROADTYPE_TRAM : ROADTYPE_ROAD;
			continue;
		}

		/* Remove +1 offset. */
		_gted[e->index].roadtramtype--;

		const std::vector<RoadTypeLabel> *list = (rtt == RTT_TRAM) ? &file->tramtype_list : &file->roadtype_list;
		if (_gted[e->index].roadtramtype < list->size())
		{
			RoadTypeLabel rtl = (*list)[_gted[e->index].roadtramtype];
			RoadType rt = GetRoadTypeByLabel(rtl);
			if (rt != INVALID_ROADTYPE && GetRoadTramType(rt) == rtt) {
				e->VehInfo<RoadVehicleInfo>().roadtype = rt;
				continue;
			}
		}

		/* Road type is not available, so disable this engine */
		e->info.climates = {};
	}

	for (Engine *e : Engine::IterateType(VEH_TRAIN)) {
		RailTypes railtypes{};
		for (RailTypeLabel label : _gted[e->index].railtypelabels) {
			auto rt = GetRailTypeByLabel(label);
			if (rt != INVALID_RAILTYPE) railtypes.Set(rt);
		}

		if (railtypes.Any()) {
			e->VehInfo<RailVehicleInfo>().railtypes = railtypes;
			e->VehInfo<RailVehicleInfo>().intended_railtypes = railtypes;
		} else {
			/* Rail type is not available, so disable this engine */
			e->info.climates = {};
		}
	}

		},
		.baseset_dir = BASESET_DIR,
		.newgrf_dir = NEWGRF_DIR,
	};
	return services;
}
#include "abi-visit-OpenTTDGrfLoadingServices.hpp"
bool CheckAbi_grf_loading() { return AbiServicesComplete(GetRustGrfLoadingServices()); }
#endif

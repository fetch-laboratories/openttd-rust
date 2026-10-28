/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_adapter.hpp Native pool/configuration/display services only. */
#include "engine_ffi.h"
#include "vehicle_services.h"
#include "company_services.h"
#include "settings_services.h"
#include "engine_services.h"
#include "presentation_services.h"
#include "environment_services.h"
#include "type_availability_services.h"
static_assert(MAX_COMPANIES == 15 && OWNER_DEITY.base() == 18 && MAX_LENGTH_ENGINE_NAME_CHARS == 32);
const OpenTTDTypeAvailabilityServices &GetRustTypeAvailabilityServices() noexcept
{
	static const OpenTTDTypeAvailabilityServices services{
		.next = Engine_next,
		.rail_availability = RustEngineRailAvailability,
		.road_availability = RustEngineRoadAvailability,
		.engine_railtypes = RustEngineRailTypes,
		.engine_roadtype = RustEngineRoadType,
		.rail_introduction = RustRailTypeIntroduction,
		.road_introduction = RustRoadTypeIntroduction,
		.rail_introduces = [](RailType rt) noexcept { return RustRailTypeIntroduces(GetRailTypeInfo(rt)); },
		.road_introduces = [](RoadType rt) noexcept { return RustRoadTypeIntroduces(GetRoadTypeInfo(rt)); },
		.road_construction = RustRoadTypeConstruction,
		.company_railtypes = RustCompanyRailTypes,
		.company_roadtypes = RustCompanyRoadTypes,
		.set_company_railtypes = RustSetCompanyRailTypes,
		.set_company_roadtypes = RustSetCompanyRoadTypes,
		.editor = OpenTTDEditor,
#ifdef WITH_ASSERT
		.assertions = true,
#else
		.assertions = false,
#endif
	};
	return services;
}
static OpenTTDEngineRef EnginePrototype(const Engine *e) noexcept
{
	/* Unpooled prototypes may not initialize PoolItem::index. Read that field
	 * only at the original startup-seed/retirement/new-month branches. */
	return {.shell = RustEngine(e), .state = e->rust_lifecycle.state, .id = 0, .type = RustEngineType(RustEngine(e))};
}
static OpenTTDEngineSyncVariant Engine_sync_variant(const OpenTTDEngine *e) noexcept
{
	const auto &info = EngineOf(e)->info;
	return {.parent = RustEngineVariant(e), .sync = info.extra_flags.Test(ExtraEngineFlag::SyncReliability)};
}
static OpenTTDEngineDisplayVariant Engine_display_variant(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell);
	return {.parent = RustEngineVariant(e), .display = e->display_last_variant.base()};
}
static OpenTTDEnginePreviewVariant Engine_preview_variant(const OpenTTDEngine *e) noexcept
{
	const auto &info = EngineOf(e)->info;
	return {.parent = RustEngineVariant(e), .join = info.extra_flags.Test(ExtraEngineFlag::JoinPreview)};
}
static OpenTTDEngineReliability Engine_reliability(const OpenTTDEngine *shell) noexcept
{
	const auto &info = EngineOf(shell)->info;
	return {.base_life = info.base_life.base(), .retire_early = info.retire_early};
}
static OpenTTDEngineStartup Engine_startup(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell); const auto &info = e->info;
	return {.base_intro = RustEngineBaseIntro(shell), .grfid = e->GetGRFID(), .base_life = info.base_life.base(), .decay_speed = info.decay_speed, .climate = RustEngineClimate(shell)};
}
static OpenTTDEngineSeed Engine_seed_config(const OpenTTDEngine *e) noexcept
{
	const auto &info = EngineOf(e)->info;
	return {.base_intro = RustEngineBaseIntro(e), .lifelength = info.lifelength.base(), .decay_speed = info.decay_speed, .retire_early = info.retire_early};
}
static OpenTTDEngineYear Engine_aging_year(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell);
	return {.base_intro = RustEngineBaseIntro(shell), .lifelength = e->info.lifelength.base()};
}

static OpenTTDEngineStartSettings Engine_start_settings() noexcept { return {.generation_seed = RustGenerationSeed(), .starting_year = RustStartingYear(), .date = TimerGameCalendar::date.base()}; }

static bool Engine_enabled(const OpenTTDEngine *e) noexcept { return EngineOf(e)->IsEnabled(); }

static void Engine_clear_last_variant(const OpenTTDEngine *shell) noexcept
{
	const Engine *removed = EngineOf(shell);
	for (Engine *e : Engine::IterateType(removed->type)) {
		if (e->display_last_variant == removed->index) e->display_last_variant = EngineID::Invalid();
	}
}
static void Engine_refresh_type(VehicleType type) noexcept { AddRemoveEngineFromAutoreplaceAndBuildWindows(type); }
static void Engine_refresh_lists(bool dirty) noexcept
{
	InvalidateWindowClassesData(WC_BUILD_VEHICLE);
	if (dirty) { SetWindowClassesDirty(WC_BUILD_VEHICLE); SetWindowClassesDirty(WC_REPLACE_VEHICLE); }
}
static OpenTTDEngineCompanyPreview Engine_company_preview(const Company *c) noexcept
{
	return {.blocked = c->Finances().block_preview, .performance = c->Finances().old_economy[0].performance_history};
}
static OpenTTDEngineVehicleCargo Engine_vehicle_cargo(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.engine = RustEngine(v->GetEngine()), .cargo = RustVehicleCargoType(handle)};
}

static uint Engine_company_engine_count(uint8_t company, uint16_t engine) noexcept { return GetGroupNumEngines(CompanyID{company}, ALL_GROUP, EngineID{engine}); }
static void Engine_block_preview(uint8_t company) noexcept { Company::Get(CompanyID{company})->Finances().block_preview = 20; }



static void Engine_toolbar_release(VehicleType type) noexcept
{
	if (type == VEH_ROAD) InvalidateWindowData(WC_BUILD_TOOLBAR, TRANSPORT_ROAD);
	if (type == VEH_SHIP) InvalidateWindowData(WC_BUILD_TOOLBAR, TRANSPORT_WATER);
	if (type == VEH_AIRCRAFT) InvalidateWindowData(WC_BUILD_TOOLBAR, TRANSPORT_AIR);
}
static void Engine_toolbar_enable(VehicleType type) noexcept { InvalidateWindowData(WC_MAIN_TOOLBAR, 0); Engine_toolbar_release(type); }
static void Engine_close_preview(uint16_t engine) noexcept { CloseWindowById(WC_ENGINE_PREVIEW, engine); }
static void Engine_invalidate_preview(uint16_t engine) noexcept { InvalidateWindowData(WC_ENGINE_PREVIEW, engine); }
static void Engine_offer_preview(uint16_t engine, uint8_t company, bool ai) noexcept
{
	if (ai) AI::NewEvent(CompanyID{company}, new ScriptEventEnginePreview(EngineID{engine}));
	if (RustIsInteractiveCompany(company)) ShowEnginePreviewWindow(EngineID{engine});
}
static void Engine_available_event(uint16_t engine) noexcept { AI::BroadcastNewEvent(new ScriptEventEngineAvailable(EngineID{engine})); }
static void Engine_available_news(uint16_t engine) noexcept
{
	EngineID index{engine};
	AddNewsItem(GetEncodedString(STR_NEWS_NEW_VEHICLE_NOW_AVAILABLE_WITH_TYPE, GetEngineCategoryName(index), PackEngineNameDParam(index, EngineNameContext::PreviewNews)), NewsType::NewVehicles, NewsStyle::Vehicle, {}, index);
}
static void Engine_no_engines(int32_t date) noexcept
{
	if (date < INT32_MAX) ShowErrorMessage(GetEncodedString(STR_ERROR_NO_VEHICLES_AVAILABLE_YET), GetEncodedString(STR_ERROR_NO_VEHICLES_AVAILABLE_YET_EXPLANATION, TimerGameCalendar::Date{date}), WL_WARNING);
	else ShowErrorMessage(GetEncodedString(STR_ERROR_NO_VEHICLES_AVAILABLE_AT_ALL), GetEncodedString(STR_ERROR_NO_VEHICLES_AVAILABLE_AT_ALL_EXPLANATION), WL_WARNING);
}
static const OpenTTDEngineServices &EngineServices() noexcept
{
	static const OpenTTDEngineServices services{
		.shared = &GetRustSharedServices(),
		.rng = &GetRustRandomServices(),
		.types = &GetRustTypeAvailabilityServices(),
		.next = Engine_next,
		.resolve = Engine_resolve,
		.sync_variant = Engine_sync_variant,
		.display_variant = Engine_display_variant,
		.preview_variant = Engine_preview_variant,
		.extra_flags = RustEngineExtraFlags,
		.engine_id = RustEngineID,
		.reliability = Engine_reliability,
		.startup = Engine_startup,
		.seed_config = Engine_seed_config,
		.base_intro = RustEngineBaseIntro,
		.aging_year = Engine_aging_year,
		.calendar_year = GetRustSharedServices().calendar_year,
		.never_expire = RustNeverExpireVehicles,
		.start_settings = Engine_start_settings,
		.train_limit = RustTrainLimit,
		.road_limit = RustRoadVehicleLimit,
		.ship_limit = RustShipLimit,
		.aircraft_limit = RustAircraftLimit,
		.ai_train_disabled = RustAITrainDisabled,
		.ai_road_disabled = RustAIRoadDisabled,
		.ai_ship_disabled = RustAIShipDisabled,
		.ai_aircraft_disabled = RustAIAircraftDisabled,
		.enabled = Engine_enabled,
		.climate = RustEngineClimate,
		.rail_kind = Engine_rail_kind,
		.clear_last_variant = Engine_clear_last_variant,
		.refresh_type = Engine_refresh_type,
		.refresh_lists = Engine_refresh_lists,
		.company_next = Engine_company_next,
		.company_preview = Engine_company_preview,
		.vehicle_next = Engine_vehicle_next,
		.vehicle_cargo = Engine_vehicle_cargo,
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.company_valid = GetRustSharedServices().company_valid,
		.compatible_railtypes = Engine_compatible_railtypes,
		.powered_roadtypes = Engine_powered_roadtypes,
		.company_railtypes = RustCompanyRailTypes,
		.company_roadtypes = RustCompanyRoadTypes,
		.company_engine_count = Engine_company_engine_count,
		.block_preview = Engine_block_preview,
		.current_company = RustCurrentCompanyID,
		.local_company = RustLocalCompanyID,
		.toolbar_enable = Engine_toolbar_enable,
		.toolbar_release = Engine_toolbar_release,
		.close_preview = Engine_close_preview,
		.invalidate_preview = Engine_invalidate_preview,
		.offer_preview = Engine_offer_preview,
		.available_event = Engine_available_event,
		.available_news = Engine_available_news,
		.no_engines = Engine_no_engines,
		.screen_dirty = RustScreenDirty,
		.utf8_length = GetRustSharedServices().utf8_length,
	};
	return services;
}

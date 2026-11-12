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
#include "company_adapter.h"
#include "settings_services.h"
#include "engine_services.h"
#include "presentation_services.h"
#include "environment_services.h"
#include "type_availability_services.h"
static_assert(MAX_COMPANIES == 15 && OWNER_DEITY.base() == 18 && MAX_LENGTH_ENGINE_NAME_CHARS == 32);
const OpenTTDTypeAvailabilityServices &GetRustTypeAvailabilityServices() noexcept
{
	static const OpenTTDTypeAvailabilityServices services{
#ifdef WITH_ASSERT
		.assertions = true,
#else
		.assertions = false,
#endif
		.company_railtypes = RustCompanyRailTypes,
		.company_roadtypes = RustCompanyRoadTypes,
		.editor = OpenTTDEditor,
		.next = Engine_next,
		.rail_introduces = [](RailType rt) noexcept { return RustRailTypeIntroduces(GetRailTypeInfo(rt)); },
		.rail_introduction = RustRailTypeIntroduction,
		.road_flags = [](const RoadTypeInfo *rti) noexcept { return rti->flags.base(); },
		.road_info = [](RoadType rt) noexcept { return GetRoadTypeInfo(rt); },
		.road_introduces = RustRoadTypeIntroduces,
		.road_introduction_date = RustRoadTypeIntroductionDate,
		.road_introduction_required = [](const RoadTypeInfo *rti) noexcept { return rti->introduction_required_roadtypes.base(); },
		.road_label = [](const RoadTypeInfo *rti) noexcept { return RustRoadTypeLabel(rti); },
		.set_company_railtypes = RustSetCompanyRailTypes,
		.set_company_roadtypes = RustSetCompanyRoadTypes,
		.shared = &GetRustSharedServices(),
	};
	return services;
}
static OpenTTDEngineRef EnginePrototype(const Engine *e) noexcept
{
	/* Unpooled prototypes may not initialize PoolItem::index. Read that field
	 * only at the original startup-seed/retirement/new-month branches. */
	return {.shell = RustEngine(e), .state = e->rust_lifecycle.state, .id = 0, .type = RustEngineType(RustEngine(e))};
}
static uint16_t Engine_display_variant(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell);
	return e->display_last_variant.base();
}

static OpenTTDEngineStartSettings Engine_start_settings() noexcept { return {.starting_year = RustStartingYear(), .date = TimerGameCalendar::date.base()}; }


static void Engine_clear_last_variant(const OpenTTDEngine *shell) noexcept
{
	const Engine *removed = EngineOf(shell);
	for (Engine *e : Engine::IterateType(removed->Record().type)) {
		if (e->display_last_variant == removed->index) e->display_last_variant = EngineID::Invalid();
	}
}
static void Engine_refresh_type(VehicleType type) noexcept { AddRemoveEngineFromAutoreplaceAndBuildWindows(type); }
static void Engine_refresh_lists(bool dirty) noexcept
{
	InvalidateWindowClassesData(WC_BUILD_VEHICLE);
	if (dirty) { SetWindowClassesDirty(WC_BUILD_VEHICLE); SetWindowClassesDirty(WC_REPLACE_VEHICLE); }
}
static OpenTTDEngineVehicleCargo Engine_vehicle_cargo(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.engine = RustEngine(v->GetEngine()), .cargo = RustVehicleCargoType(handle)};
}

static uint Engine_company_engine_count(uint8_t company, uint16_t engine) noexcept { return GetGroupNumEngines(CompanyID{company}, ALL_GROUP, EngineID{engine}); }



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
const OpenTTDEngineServices &EngineServices() noexcept
{
	static const OpenTTDEngineServices services{
		.shared = &GetRustSharedServices(),
		.finance = &GetRustCompanyFinanceServices(),
		.rng = &GetRustRandomServices(),
		.types = &GetRustTypeAvailabilityServices(),
		.resolve = Engine_resolve,
		.display_variant = Engine_display_variant,
		.engine_id = RustEngineID,
		.start_settings = Engine_start_settings,
		.road_limit = RustRoadVehicleLimit,
		.ship_limit = RustShipLimit,
		.aircraft_limit = RustAircraftLimit,
		.ai_train_disabled = RustAITrainDisabled,
		.ai_road_disabled = RustAIRoadDisabled,
		.ai_ship_disabled = RustAIShipDisabled,
		.ai_aircraft_disabled = RustAIAircraftDisabled,
		.clear_last_variant = Engine_clear_last_variant,
		.refresh_type = Engine_refresh_type,
		.refresh_lists = Engine_refresh_lists,
		.company_next = Engine_company_next,
		.vehicle_next = Engine_vehicle_next,
		.vehicle_cargo = Engine_vehicle_cargo,
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.company_engine_count = Engine_company_engine_count,
		.local_company = RustLocalCompanyID,
		.toolbar_enable = Engine_toolbar_enable,
		.toolbar_release = Engine_toolbar_release,
		.close_preview = Engine_close_preview,
		.invalidate_preview = Engine_invalidate_preview,
		.offer_preview = Engine_offer_preview,
		.available_event = Engine_available_event,
		.available_news = Engine_available_news,
		.no_engines = Engine_no_engines,
	};
	return services;
}

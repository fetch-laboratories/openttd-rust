/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file settings_table.cpp The tables of all the settings as well as the implementation of most of their callbacks.
 */

#include "stdafx.h"
#include "settings_table.h"
#include "currency.h"
#include "screenshot.h"
#include "network/network.h"
#include "network/network_func.h"
#include "network/core/config.h"
#include "pathfinder/pathfinder_type.h"
#include "linkgraph/linkgraphschedule.h"
#include "genworld.h"
#include "train.h"
#include "news_func.h"
#include "window_func.h"
#include "company_func.h"
#include "timer/timer_game_calendar.h"
#if defined(WITH_FREETYPE) || defined(_WIN32) || defined(WITH_COCOA)
#define HAS_TRUETYPE_FONT
#include "fontcache.h"
#endif
#include "textbuf_gui.h"
#include "rail_gui.h"
#include "elrail_func.h"
#include "error.h"
#include "town.h"
#include "video/video_driver.hpp"
#include "sound/sound_driver.hpp"
#include "music/music_driver.hpp"
#include "blitter/factory.hpp"
#include "base_media_base.h"
#include "base_media_music.h"
#include "base_media_sounds.h"
#include "ai/ai_config.hpp"
#include "ai/ai.hpp"
#include "game/game_config.hpp"
#include "ship.h"
#include "smallmap_gui.h"
#include "roadveh.h"
#include "roadveh_cmd.h"
#include "vehicle_func.h"
#include "viewport_func.h"
#include "void_map.h"
#include "station_func.h"
#include "station_base.h"
#include "rust/settings_table_ffi.h"

#include "table/strings.h"
#include "table/settings.h"

#include "safeguards.h"

SettingTable _company_settings{ _company_settings_table };
SettingTable _currency_settings{ _currency_settings_table };
SettingTable _difficulty_settings{ _difficulty_settings_table };
SettingTable _multimedia_settings{ _multimedia_settings_table };
SettingTable _economy_settings{ _economy_settings_table };
SettingTable _game_settings{ _game_settings_table };
SettingTable _gui_settings{ _gui_settings_table };
SettingTable _linkgraph_settings{ _linkgraph_settings_table };
SettingTable _locale_settings{ _locale_settings_table };
SettingTable _misc_settings{ _misc_settings_table };
SettingTable _network_private_settings{ _network_private_settings_table };
SettingTable _network_secrets_settings{ _network_secrets_settings_table };
SettingTable _network_settings{ _network_settings_table };
SettingTable _news_display_settings{ _news_display_settings_table };
SettingTable _old_gameopt_settings{ _old_gameopt_settings_table };
SettingTable _pathfinding_settings{ _pathfinding_settings_table };
SettingTable _script_settings{ _script_settings_table };
SettingTable _window_settings{ _window_settings_table };
SettingTable _world_settings{ _world_settings_table };
#if defined(_WIN32) && !defined(DEDICATED)
SettingTable _win32_settings{ _win32_settings_table };
#endif /* _WIN32 */


/* Begin - Callback Functions for the various settings. */

/** Switch setting title depending on wallclock setting */
static StringID SettingTitleWallclock(const IntSettingDesc &sd)
{
	return TimerGameEconomy::UsingWallclockUnits(_game_mode == GM_MENU) ? sd.str + 1 : sd.str;
}

/** Switch setting help depending on wallclock setting */
static StringID SettingHelpWallclock(const IntSettingDesc &sd)
{
	return TimerGameEconomy::UsingWallclockUnits(_game_mode == GM_MENU) ? sd.str_help + 1 : sd.str_help;
}

/** Setting values for velocity unit localisation */
static std::pair<StringParameter, StringParameter> SettingsValueVelocityUnit(const IntSettingDesc &, int32_t value)
{
	StringID val;
	switch (value) {
		case 0: val = STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_IMPERIAL; break;
		case 1: val = STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_METRIC; break;
		case 2: val = STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_SI; break;
		case 3: val = TimerGameEconomy::UsingWallclockUnits(_game_mode == GM_MENU) ? STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_GAMEUNITS_SECS : STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_GAMEUNITS_DAYS; break;
		case 4: val = STR_CONFIG_SETTING_LOCALISATION_UNITS_VELOCITY_KNOTS; break;
		default: NOT_REACHED();
	}
	return {val, {}};
}

/** A negative value has another string (the one after "strval"). */
static std::pair<StringParameter, StringParameter> SettingsValueAbsolute(const IntSettingDesc &sd, int32_t value)
{
	return {sd.str_val + ((value >= 0) ? 1 : 0), abs(value)};
}

/** Service Interval Settings Default Value displays the correct units or as a percentage */
static std::pair<StringParameter, StringParameter>  ServiceIntervalSettingsValueText(const IntSettingDesc &sd, int32_t value)
{
	VehicleDefaultSettings *vds;
	if (_game_mode == GM_MENU || !Company::IsValidID(_current_company)) {
		vds = &_settings_client.company.vehicle;
	} else {
		vds = &Company::Get(_current_company)->settings.vehicle;
	}

	StringID str;
	if (value == 0) {
		str = sd.str_val + 3;
	} else if (vds->servint_ispercent) {
		str = sd.str_val + 2;
	} else if (TimerGameEconomy::UsingWallclockUnits(_game_mode == GM_MENU)) {
		str = sd.str_val + 1;
	} else {
		str = sd.str_val;
	}
	return {str, value};
}

/** Reposition the main toolbar as the setting changed. */
static void v_PositionMainToolbar(int32_t)
{
	if (_game_mode != GM_MENU) PositionMainToolbar(nullptr);
}

/** Reposition the statusbar as the setting changed. */
static void v_PositionStatusbar(int32_t)
{
	if (_game_mode != GM_MENU) {
		PositionStatusbar(nullptr);
		PositionNewsMessage(nullptr);
		PositionNetworkChatWindow(nullptr);
	}
}

/**
 * Redraw the smallmap after a colour scheme change.
 */
static void RedrawSmallmap(int32_t)
{
	BuildLandLegend();
	BuildOwnerLegend();
	SetWindowClassesDirty(WC_SMALLMAP);
}

/** Redraw linkgraph links after a colour scheme change. */
static void UpdateLinkgraphColours(int32_t)
{
	BuildLinkStatsLegend();
	MarkWholeScreenDirty();
}

static void StationSpreadChanged(int32_t)
{
	InvalidateWindowData(WC_SELECT_STATION, 0);
	InvalidateWindowData(WC_BUILD_STATION, 0);
}

static void UpdateConsists(int32_t) { openttd_rust_settings_update_consists(&GetRustSettingServices()); }
static void UpdateAllServiceInterval(int32_t new_value) { openttd_rust_settings_update_all_service_interval(&GetRustSettingServices(), new_value); }
static bool CanUpdateServiceInterval(VehicleType, int32_t &new_value) { return openttd_rust_settings_can_update_service_interval(&GetRustSettingServices(), new_value); }
static void UpdateServiceInterval(VehicleType type, int32_t new_value) { openttd_rust_settings_update_service_interval(&GetRustSettingServices(), type, new_value); }
static int32_t GetDefaultServiceInterval(const IntSettingDesc &sd, VehicleType type) { return openttd_rust_settings_default_service_interval(&GetRustSettingServices(), sd.def, type); }
static std::tuple<int32_t, uint32_t> GetServiceIntervalRange(const IntSettingDesc &)
{
	OpenTTDSettingRange range = openttd_rust_settings_service_interval_range(&GetRustSettingServices());
	return {range.min, range.max};
}
static void TrainAccelerationModelChanged(int32_t) { openttd_rust_settings_train_acceleration_model_changed(&GetRustSettingServices()); }
static void TrainSlopeSteepnessChanged(int32_t) { openttd_rust_settings_train_slope_steepness_changed(&GetRustSettingServices()); }
static void RoadVehAccelerationModelChanged(int32_t) { openttd_rust_settings_road_acceleration_model_changed(&GetRustSettingServices()); }
static void RoadVehSlopeSteepnessChanged(int32_t) { openttd_rust_settings_road_slope_steepness_changed(&GetRustSettingServices()); }

static void TownFoundingChanged(int32_t)
{
	if (_game_mode != GM_EDITOR && _settings_game.economy.found_town == TF_FORBIDDEN) {
		CloseWindowById(WC_FOUND_TOWN, 0);
	} else {
		InvalidateWindowData(WC_FOUND_TOWN, 0);
	}
}

static void ZoomMinMaxChanged(int32_t)
{
	ConstrainAllViewportsZoom();
	GfxClearSpriteCache();
	InvalidateWindowClassesData(WC_SPRITE_ALIGNER);
	if (AdjustGUIZoom(false)) {
		ReInitAllWindows(true);
	}
}

static void SpriteZoomMinChanged(int32_t)
{
	GfxClearSpriteCache();
	/* Force all sprites to redraw at the new chosen zoom level */
	MarkWholeScreenDirty();
}

/**
 * Update any possible saveload window and delete any newgrf dialogue as
 * its widget parts might change. Reinit all windows as it allows access to the
 * newgrf debug button.
 */
static void InvalidateNewGRFChangeWindows(int32_t)
{
	InvalidateWindowClassesData(WC_SAVELOAD);
	CloseWindowByClass(WC_GAME_OPTIONS);
	ReInitAllWindows(false);
}

static void InvalidateCompanyLiveryWindow(int32_t)
{
	InvalidateWindowClassesData(WC_COMPANY_COLOUR, -1);
	ResetVehicleColourMap();
}

static void DifficultyNoiseChange(int32_t) { openttd_rust_settings_difficulty_noise_change(&GetRustSettingServices()); }

static void MaxNoAIsChange(int32_t)
{
	if (GetGameSettings().difficulty.max_no_competitors != 0 &&
			AI::GetInfoList()->empty() &&
			(!_networking || _network_server)) {
		ShowErrorMessage(GetEncodedString(STR_WARNING_NO_SUITABLE_AI), {}, WL_CRITICAL);
	}

	InvalidateWindowClassesData(WC_GAME_OPTIONS, 0);
}

/**
 * Check whether the road side may be changed.
 * @return true if the road side may be changed.
 */
static bool CheckRoadSide(int32_t &) { return openttd_rust_settings_check_road_side(&GetRustSettingServices()); }

/**
 * Conversion callback for _gameopt_settings_game.landscape
 * It converts (or try) between old values and the new ones,
 * without losing initial setting of the user
 * @param value that was read from config file
 * @return the "hopefully" converted value
 */
static std::optional<uint32_t> ConvertLandscape(std::string_view value)
{
	/* try with the old values */
	static constexpr std::initializer_list<std::string_view> _old_landscape_values{"normal"sv, "hilly"sv, "desert"sv, "candy"sv};
	return OneOfManySettingDesc::ParseSingleValue(value, _old_landscape_values);
}

static bool CheckFreeformEdges(int32_t &new_value) { return openttd_rust_settings_check_freeform_edges(&GetRustSettingServices(), new_value); }
static void UpdateFreeformEdges(int32_t new_value) { openttd_rust_settings_update_freeform_edges(&GetRustSettingServices(), new_value); }
static bool CheckDynamicEngines(int32_t &) { return openttd_rust_settings_check_dynamic_engines(&GetRustSettingServices()); }
static bool CheckMaxHeightLevel(int32_t &new_value) { return openttd_rust_settings_check_max_height_level(&GetRustSettingServices(), new_value); }
static void StationCatchmentChanged(int32_t) { openttd_rust_settings_station_catchment_changed(&GetRustSettingServices()); }

static void MaxVehiclesChanged(int32_t)
{
	InvalidateWindowClassesData(WC_BUILD_TOOLBAR);
	MarkWholeScreenDirty();
}

/**
 * Replace a passwords that are a literal asterisk with an empty string.
 * @param newval The new string value for this password field.
 * @return Always true.
 */
static bool ReplaceAsteriskWithEmptyPassword(std::string &newval)
{
	if (newval == "*") newval.clear();
	return true;
}

/** Update the game info, and send it to the clients when we are running as a server. */
static void UpdateClientConfigValues()
{
	NetworkServerUpdateGameInfo();

	InvalidateWindowData(WC_CLIENT_LIST, 0);

	if (_network_server) {
		NetworkServerSendConfigUpdate();
	}
}

static void ChangeTimekeepingUnits(int32_t) { openttd_rust_settings_change_timekeeping_units(&GetRustSettingServices()); }
static void ChangeMinutesPerYear(int32_t new_value) { openttd_rust_settings_change_minutes_per_year(&GetRustSettingServices(), new_value); }
static std::tuple<int32_t, uint32_t> GetMinutesPerYearRange(const IntSettingDesc &)
{
	OpenTTDSettingRange range = openttd_rust_settings_minutes_per_year_range(&GetRustSettingServices());
	return {range.min, range.max};
}
static bool CanChangeTimetableMode(int32_t &) { return openttd_rust_settings_can_change_timetable_mode(&GetRustSettingServices()); }

/* End - Callback Functions */

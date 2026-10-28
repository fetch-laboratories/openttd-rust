/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file session_ffi.h Simulation-session control and named native exception frames. */
#ifndef RUST_SESSION_FFI_H
#define RUST_SESSION_FFI_H
#include "world_generation_ffi.h"
#include "vehicle_ffi.h"
#include "linkgraph_schedule_ffi.h"
#include "../fios.h"
#include "../saveload/saveload.h"
#include "station_service_ffi.h"
#include "vehicle_visits_ffi.h"
struct NewGRFScanCallback;
struct OpenTTDSessionFrame;
struct OpenTTDSessionLoad;
struct OpenTTDSessionDimensions { uint32_t x, y; };
struct OpenTTDSessionSecondaryColour { Colours colour; bool used; };
/* Serialized entries; native resource/exception frames live on their callers' stacks.
	* No state access spans a callback, progress yield, reset or network reentry. */
struct OpenTTDSessionServices {
	const OpenTTDClockServices *clock;
	const OpenTTDCompanyServices *company;
	const OpenTTDSharedServices *shared;
	const OpenTTDScheduleServices *schedule;
	const OpenTTDWorldServices *(*world)() noexcept OPENTTD_SERVICE_DEFAULT(world);
	bool (*networking)() noexcept OPENTTD_SERVICE_DEFAULT(networking);
	bool (*network_server)() noexcept OPENTTD_SERVICE_DEFAULT(network_server);
	bool (*dedicated)() noexcept OPENTTD_SERVICE_DEFAULT(dedicated);
	bool (*server_requested)() noexcept OPENTTD_SERVICE_DEFAULT(server_requested);
	void (*clear_server_requested)() noexcept OPENTTD_SERVICE_DEFAULT(clear_server_requested);
	bool (*network_available)() noexcept OPENTTD_SERVICE_DEFAULT(network_available);
	bool (*exit_game)() noexcept OPENTTD_SERVICE_DEFAULT(exit_game);
	void (*set_exit)() noexcept OPENTTD_SERVICE_DEFAULT(set_exit);
	bool (*modal)() noexcept OPENTTD_SERVICE_DEFAULT(modal);
	void (*network_reboot)() noexcept OPENTTD_SERVICE_DEFAULT(network_reboot);
	void (*network_disconnect)() noexcept OPENTTD_SERVICE_DEFAULT(network_disconnect);
	bool (*network_server_start)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(network_server_start);
	void (*network_background)() noexcept OPENTTD_SERVICE_DEFAULT(network_background);
	void (*network_start_game)() noexcept OPENTTD_SERVICE_DEFAULT(network_start_game);
	void (*network_join)() noexcept OPENTTD_SERVICE_DEFAULT(network_join);
	void (*kill_ai)() noexcept OPENTTD_SERVICE_DEFAULT(kill_ai);
	void (*load_config)() noexcept OPENTTD_SERVICE_DEFAULT(load_config);
	void (*reset_session_stats)() noexcept OPENTTD_SERVICE_DEFAULT(reset_session_stats);
	void (*reset_file_mode)() noexcept OPENTTD_SERVICE_DEFAULT(reset_file_mode);
	void (*generate_id)() noexcept OPENTTD_SERVICE_DEFAULT(generate_id);
	void (*reset_windows)() noexcept OPENTTD_SERVICE_DEFAULT(reset_windows);
	void (*setup_windows)() noexcept OPENTTD_SERVICE_DEFAULT(setup_windows);
	void (*fix_title_zoom)() noexcept OPENTTD_SERVICE_DEFAULT(fix_title_zoom);
	void (*release_cursor)() noexcept OPENTTD_SERVICE_DEFAULT(release_cursor);
	void (*check_glyphs)() noexcept OPENTTD_SERVICE_DEFAULT(check_glyphs);
	void (*music)() noexcept OPENTTD_SERVICE_DEFAULT(music);
	void (*rail_gui)() noexcept OPENTTD_SERVICE_DEFAULT(rail_gui);
	void (*road_gui)() noexcept OPENTTD_SERVICE_DEFAULT(road_gui);
	void (*check_engines)() noexcept OPENTTD_SERVICE_DEFAULT(check_engines);
	void (*check_industries)() noexcept OPENTTD_SERVICE_DEFAULT(check_industries);
	void (*close_saveload)() noexcept OPENTTD_SERVICE_DEFAULT(close_saveload);
	void (*joining_social)() noexcept OPENTTD_SERVICE_DEFAULT(joining_social);
	void (*ask_survey)() noexcept OPENTTD_SERVICE_DEFAULT(ask_survey);
	void (*process_async_save)() noexcept OPENTTD_SERVICE_DEFAULT(process_async_save);
	void (*sprite_lru)() noexcept OPENTTD_SERVICE_DEFAULT(sprite_lru);
	void (*remote_messages)() noexcept OPENTTD_SERVICE_DEFAULT(remote_messages);
	void (*palette)() noexcept OPENTTD_SERVICE_DEFAULT(palette);
	void (*sound)() noexcept OPENTTD_SERVICE_DEFAULT(sound);
	void (*social_callbacks)() noexcept OPENTTD_SERVICE_DEFAULT(social_callbacks);
	void (*news)() noexcept OPENTTD_SERVICE_DEFAULT(news);
	void (*window_tick)() noexcept OPENTTD_SERVICE_DEFAULT(window_tick);
	void (*check_caches)() noexcept OPENTTD_SERVICE_DEFAULT(check_caches);
	void (*reset_engine_overrides)() noexcept OPENTTD_SERVICE_DEFAULT(reset_engine_overrides);
	void (*dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	void (*set_local_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_local_company);
	uint8_t (*first_playable_company)() noexcept OPENTTD_SERVICE_DEFAULT(first_playable_company);
	bool (*reload_config)() noexcept OPENTTD_SERVICE_DEFAULT(reload_config);
	void (*reset_grf)(bool) noexcept OPENTTD_SERVICE_DEFAULT(reset_grf);
	void (*autosave_frequency)(bool) noexcept OPENTTD_SERVICE_DEFAULT(autosave_frequency);
	void (*leave_survey)() noexcept OPENTTD_SERVICE_DEFAULT(leave_survey);
	AbstractFileType (*file_kind)() noexcept OPENTTD_SERVICE_DEFAULT(file_kind);
	OpenTTDSessionDimensions (*map_dimensions)() noexcept OPENTTD_SERVICE_DEFAULT(map_dimensions);
	bool (*has_gui)() noexcept OPENTTD_SERVICE_DEFAULT(has_gui);
	bool (*pause_on_newgame)() noexcept OPENTTD_SERVICE_DEFAULT(pause_on_newgame);
	Colours (*starting_colour)() noexcept OPENTTD_SERVICE_DEFAULT(starting_colour);
	OpenTTDSessionSecondaryColour (*secondary_colour)() noexcept OPENTTD_SERVICE_DEFAULT(secondary_colour);
	bool (*disable_elrails)() noexcept OPENTTD_SERVICE_DEFAULT(disable_elrails);
	void (*settings_elrail)(bool) noexcept OPENTTD_SERVICE_DEFAULT(settings_elrail);
	void (*company_settings)() noexcept OPENTTD_SERVICE_DEFAULT(company_settings);
	void (*company_colour)(bool, Colours) noexcept OPENTTD_SERVICE_DEFAULT(company_colour);
	void (*post_pause)(PauseMode, bool) noexcept OPENTTD_SERVICE_DEFAULT(post_pause);
	void (*reset_ai_config)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_ai_config);
	void (*reset_game_config)() noexcept OPENTTD_SERVICE_DEFAULT(reset_game_config);
	void (*publish_settings)() noexcept OPENTTD_SERVICE_DEFAULT(publish_settings);
	void (*publish_old_settings)() noexcept OPENTTD_SERVICE_DEFAULT(publish_old_settings);
	void (*newgame_year)(int32_t) noexcept OPENTTD_SERVICE_DEFAULT(newgame_year);
	bool (*fallback_sound_warning)() noexcept OPENTTD_SERVICE_DEFAULT(fallback_sound_warning);
	void (*show_sound_warning)() noexcept OPENTTD_SERVICE_DEFAULT(show_sound_warning);
	bool (*survey_ask)() noexcept OPENTTD_SERVICE_DEFAULT(survey_ask);
	void (*show_load_error)(bool) noexcept OPENTTD_SERVICE_DEFAULT(show_load_error);
	void (*social_mode)(GameMode) noexcept OPENTTD_SERVICE_DEFAULT(social_mode);
	void (*load_failed_debug)(bool) noexcept OPENTTD_SERVICE_DEFAULT(load_failed_debug);
	void (*pause_query)() noexcept OPENTTD_SERVICE_DEFAULT(pause_query);
	void (*pause_change)(uint8_t, PauseMode) noexcept OPENTTD_SERVICE_DEFAULT(pause_change);
	void (*pause_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(pause_dirty);
	bool (*full_animation)() noexcept OPENTTD_SERVICE_DEFAULT(full_animation);
	bool (*reconnect_due)() noexcept OPENTTD_SERVICE_DEFAULT(reconnect_due);
	bool (*console_script)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(console_script);
	bool (*game_script)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(game_script);
	bool (*ai_script)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(ai_script);
	bool (*startup_company)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(startup_company);
	bool (*network_loop)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(network_loop);
	bool (*network_reconnect)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(network_reconnect);
	bool (*state_loop)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(state_loop);
	bool (*real_time)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(real_time);
	bool (*scan)(OpenTTDSessionFrame *, NewGRFScanCallback *) noexcept OPENTTD_SERVICE_DEFAULT(scan);
	OpenTTDWorldSetupFrame *(*world_setup_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(world_setup_begin);
	bool (*world_setup_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(world_setup_end);
	bool (*world_run)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(world_run);
	bool (*heightmap_screenshot)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(heightmap_screenshot);
	bool (*desync_save)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(desync_save);
	SaveOrLoadResult (*load)(OpenTTDSessionFrame *, OpenTTDSessionLoad *) noexcept OPENTTD_SERVICE_DEFAULT(load);
	SaveOrLoadResult (*intro_load)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(intro_load);
	SaveOrLoadResult (*save)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(save);
	bool (*failed)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(failed);
	OpenTTDConsoleException * (*console_exception)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(console_exception);
	void (*state_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(state_begin);
	void (*company_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(company_begin);
	void (*company_restore)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(company_restore);
	void (*scripts_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(scripts_begin);
	void (*scripts_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(scripts_end);
	void (*paused_measurements)() noexcept OPENTTD_SERVICE_DEFAULT(paused_measurements);
	bool (*scripts_enabled)() noexcept OPENTTD_SERVICE_DEFAULT(scripts_enabled);
	void (*assert_local_company)() noexcept OPENTTD_SERVICE_DEFAULT(assert_local_company);
	int32_t (*desync_level)() noexcept OPENTTD_SERVICE_DEFAULT(desync_level);
	bool (*tick_timer)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(tick_timer);
	const OpenTTDGenerationLoop * (*tile_loop_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(tile_loop_begin);
	void (*tile_loop_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(tile_loop_end);
	OpenTTDVehicleSchedulerTables (*vehicle_tables)(OpenTTDSessionFrame *, bool) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_tables);
	OpenTTDCommandCost * (*vehicle_cost)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cost);
	void (*vehicle_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_end);
	OpenTTDCompanyLandscaping (*landscaping)() noexcept OPENTTD_SERVICE_DEFAULT(landscaping);
	OpenTTDScheduleTime (*schedule_time)() noexcept OPENTTD_SERVICE_DEFAULT(schedule_time);
	const OpenTTDTownLeaves *town;
	const OpenTTDTreeServices *trees;
	const OpenTTDStationServices *station;
	const OpenTTDStationLinks *links;
	const OpenTTDVehicleVisits *visits;
	OpenTTDTreeTickSettings (*tree_tick)() noexcept OPENTTD_SERVICE_DEFAULT(tree_tick);
	OpenTTDIndustryTickRecord (*industry_tick)() noexcept OPENTTD_SERVICE_DEFAULT(industry_tick);
	OpenTTDIndustryBuilder *(*industry_builder)() noexcept OPENTTD_SERVICE_DEFAULT(industry_builder);
	OpenTTDCompanyTick (*company_tick)() noexcept OPENTTD_SERVICE_DEFAULT(company_tick);
	bool (*post_ai)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(post_ai);
	const OpenTTDRoadCommandContext *(*road_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(road_begin);
	void (*road_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(road_end);
	void (*landscape_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(landscape_begin);
	void (*landscape_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(landscape_end);
	void *(*schedule_measurement)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(schedule_measurement);
	void (*schedule_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(schedule_end);

};
const OpenTTDSessionServices &GetRustSessionServices() noexcept;
void RustAskUnsafeUnpause() noexcept;
void RustSaveDesyncGame();
bool RustRunWorldGeneration(OpenTTDSessionFrame *) noexcept;
OpenTTDTreeTickSettings RustTreeTickSettings() noexcept;
OpenTTDIndustryTickRecord RustIndustryTickRecord() noexcept;
OpenTTDVehicleSchedulerTables RustVehicleSchedulerTables(OpenTTDCommandCost *refit = nullptr, OpenTTDCommandCost *temporary = nullptr) noexcept;
OpenTTDScheduleTime RustScheduleTime() noexcept;
extern "C" {
bool openttd_rust_session_generate(const OpenTTDSessionServices *, OpenTTDSessionFrame *, GenWorldMode, uint32_t, uint32_t, bool);
bool openttd_rust_session_intro(const OpenTTDSessionServices *, OpenTTDSessionFrame *, bool);
void openttd_rust_session_settings(const OpenTTDSessionServices *);
bool openttd_rust_session_switch(const OpenTTDSessionServices *, OpenTTDSessionFrame *, SwitchMode);
bool openttd_rust_session_safe_load(const OpenTTDSessionServices *, OpenTTDSessionFrame *, OpenTTDSessionLoad *, GameMode);
bool openttd_rust_session_state_loop(const OpenTTDSessionServices *, OpenTTDSessionFrame *);
bool openttd_rust_session_game_loop(const OpenTTDSessionServices *, OpenTTDSessionFrame *);
bool openttd_rust_session_scan(NewGRFScanCallback *);
bool openttd_rust_session_pause(const OpenTTDSessionServices *, PauseMode, bool, bool);
}
#endif

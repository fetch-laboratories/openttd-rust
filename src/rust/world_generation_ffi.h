/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file world_generation_ffi.h Complete world-generation lifecycle boundary. */
#ifndef RUST_WORLD_GENERATION_FFI_H
#define RUST_WORLD_GENERATION_FFI_H
#include "landscape_generation_ffi.h"
#include "town_founding_ffi.h"
#include "industry_ffi.h"
#include "object_ffi.h"
#include "engine_ffi.h"
#include "disaster_ffi.h"
#include "clock_ffi.h"
struct OpenTTDWorldFrame;
struct OpenTTDWorldSettings { uint8_t height_limit, flat_height, heightmap_height, generator; };
struct OpenTTDWorldLandscape { OpenTTDGenerationFrame *frame; const OpenTTDHeightmapDecoder *decoder; OpenTTDLandscapeGenerationSettings settings; };
struct OpenTTDWorldTown { OpenTTDTownFoundingFrame *frame; OpenTTDTownNameFrame *names; const OpenTTDRoadCommandContext *road; TownLayout layout; };
struct OpenTTDWorldObject { OpenTTDObjectGenerationFrame *frame; OpenTTDCommandCost *out, *tmp, *aux; };
/* All calls are serialized by the game-state mutex. Progress may yield; no map
 * descriptor or owner borrow spans it. Ordinary native exceptions stay in the
 * call-local frame; Rust returns before a non-abort exception is rethrown. */
struct OpenTTDWorldServices {
	const OpenTTDLandscapeGenerationServices *landscape;
	const OpenTTDClockCompany *company;
	const OpenTTDTownFoundingServices *town;
	const OpenTTDIndustryServices *industry;
	const OpenTTDIndustryGenerationServices *industry_generation;
	const OpenTTDObjectServices *objects;
	const OpenTTDEngineServices *engines;
	const OpenTTDDisasterServices *disasters;
	bool (*modal)() noexcept OPENTTD_SERVICE_DEFAULT(modal);
	void (*set_modal)(bool) noexcept OPENTTD_SERVICE_DEFAULT(set_modal);
	uint8_t (*local_company)() noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	void (*set_local_company)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_local_company);
	void (*restore_companies)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(restore_companies);
	bool (*initialize)(OpenTTDWorldFrame *, uint32_t, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(initialize);
	void (*prepare_progress)() noexcept OPENTTD_SERVICE_DEFAULT(prepare_progress);
	OpenTTDWorldSettings (*settings)() noexcept OPENTTD_SERVICE_DEFAULT(settings);
	void (*height_limit)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(height_limit);
	GameMode (*game_mode)() noexcept OPENTTD_SERVICE_DEFAULT(game_mode);
	uint32_t (*seed)() noexcept OPENTTD_SERVICE_DEFAULT(seed);
	void (*set_seed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_seed);
	uint32_t (*interactive_random)() noexcept OPENTTD_SERVICE_DEFAULT(interactive_random);
	void (*random_seed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_seed);
	bool (*load_sprites)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(load_sprites);
	void (*string_widths)() noexcept OPENTTD_SERVICE_DEFAULT(string_widths);
	void (*reset_windows)() noexcept OPENTTD_SERVICE_DEFAULT(reset_windows);
	void (*setup_windows)() noexcept OPENTTD_SERVICE_DEFAULT(setup_windows);
	void (*unshow_error)() noexcept OPENTTD_SERVICE_DEFAULT(unshow_error);
	void (*close_windows)() noexcept OPENTTD_SERVICE_DEFAULT(close_windows);
	void (*hide_windows)() noexcept OPENTTD_SERVICE_DEFAULT(hide_windows);
	void (*show_progress)() noexcept OPENTTD_SERVICE_DEFAULT(show_progress);
	void (*show_windows)() noexcept OPENTTD_SERVICE_DEFAULT(show_windows);
	void (*reset_object)() noexcept OPENTTD_SERVICE_DEFAULT(reset_object);
	void (*show_grf_error)() noexcept OPENTTD_SERVICE_DEFAULT(show_grf_error);
	void (*randomizers)() noexcept OPENTTD_SERVICE_DEFAULT(randomizers);
	void (*startup_companies)() noexcept OPENTTD_SERVICE_DEFAULT(startup_companies);
	void (*place_cursor)() noexcept OPENTTD_SERVICE_DEFAULT(place_cursor);
	void (*scroll)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(scroll);
	void (*busy)(bool) noexcept OPENTTD_SERVICE_DEFAULT(busy);
	void (*close_progress)() noexcept OPENTTD_SERVICE_DEFAULT(close_progress);
	void (*show_error)() noexcept OPENTTD_SERVICE_DEFAULT(show_error);
	void (*dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	bool (*set_progress)(OpenTTDWorldFrame *, GenWorldProgress, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_progress);
	bool (*increase_progress)(OpenTTDWorldFrame *, GenWorldProgress) noexcept OPENTTD_SERVICE_DEFAULT(increase_progress);
	bool (*town_roads)() noexcept OPENTTD_SERVICE_DEFAULT(town_roads);
	bool (*freeform)() noexcept OPENTTD_SERVICE_DEFAULT(freeform);
	void (*count_land)() noexcept OPENTTD_SERVICE_DEFAULT(count_land);
	OpenTTDWorldLandscape (*landscape_begin)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(landscape_begin);
	bool (*landscape_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(landscape_end);
	OpenTTDWorldTown (*town_begin)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(town_begin);
	bool (*town_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(town_end);
	OpenTTDIndustryBuilder * (*industry_builder)() noexcept OPENTTD_SERVICE_DEFAULT(industry_builder);
	OpenTTDIndustryGenerationContext * (*industry_begin)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(industry_begin);
	void (*industry_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(industry_end);
	OpenTTDWorldObject (*object_begin)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(object_begin);
	void (*object_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(object_end);
	OpenTTDTreeGenerateSettings (*tree_settings)() noexcept OPENTTD_SERVICE_DEFAULT(tree_settings);
	void (*tree_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(tree_end);
	const OpenTTDGenerationLoop * (*loop_begin)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(loop_begin);
	void (*loop_end)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(loop_end);
	void (*advance_tick)() noexcept OPENTTD_SERVICE_DEFAULT(advance_tick);
	bool (*script_start)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(script_start);
	bool (*script_loop)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(script_loop);
	bool (*script_exists)() noexcept OPENTTD_SERVICE_DEFAULT(script_exists);
	bool (*script_sleeping)() noexcept OPENTTD_SERVICE_DEFAULT(script_sleeping);
	void (*trash_company)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(trash_company);
	bool (*company_valid)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(company_valid);
	void (*restore_company)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(restore_company);
	bool (*callback)(OpenTTDWorldFrame *, GWDoneProc *) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	uint8_t (*exception_kind)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(exception_kind);
	void (*storage_discard)(void *) noexcept OPENTTD_SERVICE_DEFAULT(storage_discard);
	bool (*dedicated)() noexcept OPENTTD_SERVICE_DEFAULT(dedicated);
	bool (*exit_game)() noexcept OPENTTD_SERVICE_DEFAULT(exit_game);
	void (*set_exit)() noexcept OPENTTD_SERVICE_DEFAULT(set_exit);
	void (*abort_mode)(SwitchMode) noexcept OPENTTD_SERVICE_DEFAULT(abort_mode);
	bool (*switch_mode)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(switch_mode);
	void (*debug_start)() noexcept OPENTTD_SERVICE_DEFAULT(debug_start);
	void (*debug_done)() noexcept OPENTTD_SERVICE_DEFAULT(debug_done);
	void (*debug_abort)() noexcept OPENTTD_SERVICE_DEFAULT(debug_abort);
	void (*debug_map)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_map);
	bool (*desync)() noexcept OPENTTD_SERVICE_DEFAULT(desync);
	bool (*save)(OpenTTDWorldFrame *) noexcept OPENTTD_SERVICE_DEFAULT(save);
};
const OpenTTDWorldServices &GetRustWorldServices() noexcept;
extern "C" {
void openttd_rust_world_set_callback(GWDoneProc *);
void openttd_rust_world_set_abort_callback(GWAbortProc *);
void openttd_rust_world_request_abort();
bool openttd_rust_world_aborted(const OpenTTDWorldServices *);
bool openttd_rust_world_handle_abort(const OpenTTDWorldServices *, OpenTTDWorldFrame *);
bool openttd_rust_world_setup(const OpenTTDWorldServices *, OpenTTDWorldFrame *, GenWorldMode, uint32_t, uint32_t, bool);
uint8_t openttd_rust_world_generate(const OpenTTDWorldServices *, OpenTTDWorldFrame *);
}
#endif /* RUST_WORLD_GENERATION_FFI_H */

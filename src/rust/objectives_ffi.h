/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file objectives_ffi.h Native record access and ordinary leaves for objectives. */
#ifndef RUST_OBJECTIVES_FFI_H
#define RUST_OBJECTIVES_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
struct Goal; struct StoryPage; struct StoryPageElement; struct LeagueTableElement; class EncodedString;
struct OpenTTDGoalRecord; struct OpenTTDStoryPageRecord; struct OpenTTDStoryPageElementRecord; struct OpenTTDLeagueTableRecord; struct OpenTTDLeagueTableElementRecord;
struct OpenTTDCompanyServices;
struct OpenTTDOrdersLeaves;
struct OpenTTDSubsidyServices;
struct OpenTTDRoadCommandServices;
/** Rust owns canonical records and bytes; native constructors/destructors retain shells.
 * Text spans borrow native command/GUI inputs. No borrow crosses a reentrant hook. */
struct OpenTTDObjectivesServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCompanyServices *company;
	OpenTTDGoalRecord *(*goal_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(goal_allocate);
	OpenTTDStoryPageRecord *(*page_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(page_allocate);
	OpenTTDStoryPageElementRecord *(*element_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(element_allocate);
	OpenTTDLeagueTableRecord *(*table_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(table_allocate);
	OpenTTDLeagueTableElementRecord *(*entry_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(entry_allocate);
	void (*element_destroy)(OpenTTDStoryPageElementRecord *) noexcept OPENTTD_SERVICE_DEFAULT(element_destroy);
	void (*entry_destroy)(OpenTTDLeagueTableElementRecord *) noexcept OPENTTD_SERVICE_DEFAULT(entry_destroy);
	const OpenTTDOrdersLeaves *orders;
	const OpenTTDSubsidyServices *subsidy;
	const OpenTTDRoadCommandServices *road_commands;
	bool (*vehicle_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_valid);
	OpenTTDByteSpan (*text_view)(const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(text_view);
	void (*window_data)(uint16_t, uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(window_data);
	void (*window_classes)(uint16_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(window_classes);
	void (*window_close)(uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(window_close);
	void (*question_show)(uint16_t, uint8_t, uint32_t, const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(question_show);
	void (*story_show)(uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(story_show);
	void (*question_event)(uint16_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(question_event);
	void (*push_event)(uint8_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(push_event);
	void (*tile_event)(uint8_t, uint16_t, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_event);
	void (*vehicle_event)(uint8_t, uint16_t, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_event);
};
const OpenTTDObjectivesServices &GetRustObjectivesServices() noexcept;
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_story_page_destroy(const OpenTTDObjectivesServices *, uint16_t);
uint8_t openttd_rust_create_goal_abi(const OpenTTDObjectivesServices *, uint16_t, uint8_t, uint8_t, uint32_t, const EncodedString *, uint16_t *);
extern "C++" inline bool openttd_rust_create_goal(const OpenTTDObjectivesServices *a0, uint16_t a1, uint8_t a2, uint8_t a3, uint32_t a4, const EncodedString *a5, uint16_t *a6) { return openttd_rust_create_goal_abi(a0, a1, a2, a3, a4, a5, a6) != 0; }
uint8_t openttd_rust_remove_goal_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_remove_goal(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_remove_goal_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_set_goal_destination_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t, uint32_t);
extern "C++" inline bool openttd_rust_set_goal_destination(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, uint8_t a3, uint32_t a4) { return openttd_rust_set_goal_destination_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_set_goal_text_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
extern "C++" inline bool openttd_rust_set_goal_text(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, const EncodedString *a3) { return openttd_rust_set_goal_text_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_set_goal_progress_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
extern "C++" inline bool openttd_rust_set_goal_progress(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, const EncodedString *a3) { return openttd_rust_set_goal_progress_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_set_goal_completed_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, bool);
extern "C++" inline bool openttd_rust_set_goal_completed(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, bool a3) { return openttd_rust_set_goal_completed_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_goal_question_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint32_t, bool, uint32_t, uint8_t, const EncodedString *);
extern "C++" inline bool openttd_rust_goal_question(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, uint32_t a3, bool a4, uint32_t a5, uint8_t a6, const EncodedString *a7) { return openttd_rust_goal_question_abi(a0, a1, a2, a3, a4, a5, a6, a7) != 0; }
uint8_t openttd_rust_goal_question_answer_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t);
extern "C++" inline bool openttd_rust_goal_question_answer(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, uint8_t a3) { return openttd_rust_goal_question_answer_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_create_story_page_abi(const OpenTTDObjectivesServices *, uint16_t, uint8_t, const EncodedString *, uint16_t *);
extern "C++" inline bool openttd_rust_create_story_page(const OpenTTDObjectivesServices *a0, uint16_t a1, uint8_t a2, const EncodedString *a3, uint16_t *a4) { return openttd_rust_create_story_page_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_create_story_page_element_abi(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint8_t, uint32_t, const EncodedString *, uint16_t *);
extern "C++" inline bool openttd_rust_create_story_page_element(const OpenTTDObjectivesServices *a0, uint16_t a1, uint32_t a2, uint16_t a3, uint8_t a4, uint32_t a5, const EncodedString *a6, uint16_t *a7) { return openttd_rust_create_story_page_element_abi(a0, a1, a2, a3, a4, a5, a6, a7) != 0; }
uint8_t openttd_rust_update_story_page_element_abi(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint32_t, const EncodedString *);
extern "C++" inline bool openttd_rust_update_story_page_element(const OpenTTDObjectivesServices *a0, uint16_t a1, uint32_t a2, uint16_t a3, uint32_t a4, const EncodedString *a5) { return openttd_rust_update_story_page_element_abi(a0, a1, a2, a3, a4, a5) != 0; }
uint8_t openttd_rust_set_story_page_title_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
extern "C++" inline bool openttd_rust_set_story_page_title(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, const EncodedString *a3) { return openttd_rust_set_story_page_title_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_set_story_page_date_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, int32_t);
extern "C++" inline bool openttd_rust_set_story_page_date(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, int32_t a3) { return openttd_rust_set_story_page_date_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_show_story_page_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_show_story_page(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_show_story_page_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_remove_story_page_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_remove_story_page(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_remove_story_page_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_remove_story_page_element_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_remove_story_page_element(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_remove_story_page_element_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_story_page_button_abi(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint32_t);
extern "C++" inline bool openttd_rust_story_page_button(const OpenTTDObjectivesServices *a0, uint16_t a1, uint32_t a2, uint16_t a3, uint32_t a4) { return openttd_rust_story_page_button_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_create_league_table_abi(const OpenTTDObjectivesServices *, uint16_t, const EncodedString *, const EncodedString *, const EncodedString *, uint8_t *);
extern "C++" inline bool openttd_rust_create_league_table(const OpenTTDObjectivesServices *a0, uint16_t a1, const EncodedString *a2, const EncodedString *a3, const EncodedString *a4, uint8_t *a5) { return openttd_rust_create_league_table_abi(a0, a1, a2, a3, a4, a5) != 0; }
uint8_t openttd_rust_create_league_table_element_abi(const OpenTTDObjectivesServices *, uint16_t, uint8_t, int64_t, uint8_t, const EncodedString *, const EncodedString *, uint8_t, uint32_t, uint16_t *);
extern "C++" inline bool openttd_rust_create_league_table_element(const OpenTTDObjectivesServices *a0, uint16_t a1, uint8_t a2, int64_t a3, uint8_t a4, const EncodedString *a5, const EncodedString *a6, uint8_t a7, uint32_t a8, uint16_t *a9) { return openttd_rust_create_league_table_element_abi(a0, a1, a2, a3, a4, a5, a6, a7, a8, a9) != 0; }
uint8_t openttd_rust_update_league_table_element_data_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t, const EncodedString *, uint8_t, uint32_t);
extern "C++" inline bool openttd_rust_update_league_table_element_data(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, uint8_t a3, const EncodedString *a4, uint8_t a5, uint32_t a6) { return openttd_rust_update_league_table_element_data_abi(a0, a1, a2, a3, a4, a5, a6) != 0; }
uint8_t openttd_rust_update_league_table_element_score_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t, int64_t, const EncodedString *);
extern "C++" inline bool openttd_rust_update_league_table_element_score(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2, int64_t a3, const EncodedString *a4) { return openttd_rust_update_league_table_element_score_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_remove_league_table_element_abi(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
extern "C++" inline bool openttd_rust_remove_league_table_element(const OpenTTDObjectivesServices *a0, uint16_t a1, uint16_t a2) { return openttd_rust_remove_league_table_element_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_valid_link_abi(const OpenTTDObjectivesServices *, uint8_t, uint32_t);
extern "C++" inline bool openttd_rust_valid_link(const OpenTTDObjectivesServices *a0, uint8_t a1, uint32_t a2) { return openttd_rust_valid_link_abi(a0, a1, a2) != 0; }
uint8_t openttd_rust_story_button_validate_colour_abi(uint32_t);
extern "C++" inline bool openttd_rust_story_button_validate_colour(uint32_t a0) { return openttd_rust_story_button_validate_colour_abi(a0) != 0; }
uint8_t openttd_rust_story_button_validate_flags_abi(uint32_t);
extern "C++" inline bool openttd_rust_story_button_validate_flags(uint32_t a0) { return openttd_rust_story_button_validate_flags_abi(a0) != 0; }
uint8_t openttd_rust_story_button_validate_cursor_abi(uint32_t);
extern "C++" inline bool openttd_rust_story_button_validate_cursor(uint32_t a0) { return openttd_rust_story_button_validate_cursor_abi(a0) != 0; }
uint8_t openttd_rust_story_button_validate_vehicle_type_abi(uint32_t);
extern "C++" inline bool openttd_rust_story_button_validate_vehicle_type(uint32_t a0) { return openttd_rust_story_button_validate_vehicle_type_abi(a0) != 0; }
}
#endif /* RUST_OBJECTIVES_FFI_H */

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file objectives_ffi.h Native record access and ordinary leaves for objectives. */
#ifndef RUST_OBJECTIVES_FFI_H
#define RUST_OBJECTIVES_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
struct Goal; struct StoryPage; struct StoryPageElement; struct LeagueTableElement; class EncodedString;
struct OpenTTDCompanyServices;
struct OpenTTDOrdersLeaves;
struct OpenTTDSubsidyServices;
struct OpenTTDRoadCommandServices;
/** Records and strings stay in native pools. Pointers address their live fields;
 * no pointer or borrow survives destruction, pool cleanup or a reentrant callback.
 * Commands hold the game-state mutex. Events enqueue without entering the VM. */
struct OpenTTDObjectivesServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCompanyServices *company;
	Goal * (*goal)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(goal);
	uint8_t * (*goal_company)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_company);
	uint8_t * (*goal_type)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_type);
	uint32_t * (*goal_dst)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_dst);
	EncodedString * (*goal_text)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_text);
	EncodedString * (*goal_progress)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_progress);
	bool * (*goal_completed)(Goal *) noexcept OPENTTD_SERVICE_DEFAULT(goal_completed);
	uint32_t (*goal_count)() noexcept OPENTTD_SERVICE_DEFAULT(goal_count);
	bool (*goal_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(goal_can_allocate);
	StoryPage * (*page)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(page);
	uint8_t * (*page_company)(StoryPage *) noexcept OPENTTD_SERVICE_DEFAULT(page_company);
	EncodedString * (*page_title)(StoryPage *) noexcept OPENTTD_SERVICE_DEFAULT(page_title);
	int32_t * (*page_date)(StoryPage *) noexcept OPENTTD_SERVICE_DEFAULT(page_date);
	uint32_t (*page_count)() noexcept OPENTTD_SERVICE_DEFAULT(page_count);
	bool (*page_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(page_can_allocate);
	StoryPageElement * (*element)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(element);
	uint16_t * (*element_page)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_page);
	uint8_t * (*element_type)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_type);
	EncodedString * (*element_text)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_text);
	uint32_t * (*element_referenced_id)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_referenced_id);
	void (*element_destroy)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_destroy);
	uint32_t (*element_count)() noexcept OPENTTD_SERVICE_DEFAULT(element_count);
	bool (*element_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(element_can_allocate);
	LeagueTableElement * (*entry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(entry);
	uint8_t * (*entry_company)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_company);
	uint8_t * (*entry_table)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_table);
	int64_t * (*entry_rating)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_rating);
	EncodedString * (*entry_text)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_text);
	EncodedString * (*entry_score)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_score);
	uint8_t * (*entry_link_type)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_link_type);
	uint32_t * (*entry_link_target)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_link_target);
	void (*entry_destroy)(LeagueTableElement *) noexcept OPENTTD_SERVICE_DEFAULT(entry_destroy);
	bool (*entry_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(entry_can_allocate);
	uint16_t (*goal_allocate)(uint8_t, uint32_t, uint8_t, const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(goal_allocate);
	uint16_t (*page_allocate)(uint32_t, int32_t, uint8_t, const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(page_allocate);
	uint16_t (*element_allocate)(uint32_t, uint8_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(element_allocate);
	StoryPageElement * (*element_next)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(element_next);
	uint16_t (*element_id)(StoryPageElement *) noexcept OPENTTD_SERVICE_DEFAULT(element_id);
	bool (*page_cleaning)() noexcept OPENTTD_SERVICE_DEFAULT(page_cleaning);
	bool (*table_can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(table_can_allocate);
	uint8_t (*table_allocate)(const EncodedString *, const EncodedString *, const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(table_allocate);
	uint16_t (*entry_allocate)(uint8_t, int64_t, uint8_t, const EncodedString *, const EncodedString *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(entry_allocate);
	const OpenTTDOrdersLeaves *orders;
	bool (*client_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(client_valid);
	const OpenTTDSubsidyServices *subsidy;
	const OpenTTDRoadCommandServices *road_commands;
	bool (*vehicle_valid)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_valid);
	bool (*text_empty)(const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(text_empty);
	void (*text_assign)(EncodedString *, const EncodedString *) noexcept OPENTTD_SERVICE_DEFAULT(text_assign);
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
void openttd_rust_story_page_destruct(const OpenTTDObjectivesServices *, uint16_t);
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

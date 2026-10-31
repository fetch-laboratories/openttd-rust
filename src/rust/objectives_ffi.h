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
bool openttd_rust_create_goal(const OpenTTDObjectivesServices *, uint16_t, uint8_t, uint8_t, uint32_t, const EncodedString *, uint16_t *);
bool openttd_rust_remove_goal(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
bool openttd_rust_set_goal_destination(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t, uint32_t);
bool openttd_rust_set_goal_text(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
bool openttd_rust_set_goal_progress(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
bool openttd_rust_set_goal_completed(const OpenTTDObjectivesServices *, uint16_t, uint16_t, bool);
bool openttd_rust_goal_question(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint32_t, bool, uint32_t, uint8_t, const EncodedString *);
bool openttd_rust_goal_question_answer(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t);
bool openttd_rust_create_story_page(const OpenTTDObjectivesServices *, uint16_t, uint8_t, const EncodedString *, uint16_t *);
bool openttd_rust_create_story_page_element(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint8_t, uint32_t, const EncodedString *, uint16_t *);
bool openttd_rust_update_story_page_element(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint32_t, const EncodedString *);
bool openttd_rust_set_story_page_title(const OpenTTDObjectivesServices *, uint16_t, uint16_t, const EncodedString *);
bool openttd_rust_set_story_page_date(const OpenTTDObjectivesServices *, uint16_t, uint16_t, int32_t);
bool openttd_rust_show_story_page(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
bool openttd_rust_remove_story_page(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
bool openttd_rust_remove_story_page_element(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
bool openttd_rust_story_page_button(const OpenTTDObjectivesServices *, uint16_t, uint32_t, uint16_t, uint32_t);
bool openttd_rust_create_league_table(const OpenTTDObjectivesServices *, uint16_t, const EncodedString *, const EncodedString *, const EncodedString *, uint8_t *);
bool openttd_rust_create_league_table_element(const OpenTTDObjectivesServices *, uint16_t, uint8_t, int64_t, uint8_t, const EncodedString *, const EncodedString *, uint8_t, uint32_t, uint16_t *);
bool openttd_rust_update_league_table_element_data(const OpenTTDObjectivesServices *, uint16_t, uint16_t, uint8_t, const EncodedString *, uint8_t, uint32_t);
bool openttd_rust_update_league_table_element_score(const OpenTTDObjectivesServices *, uint16_t, uint16_t, int64_t, const EncodedString *);
bool openttd_rust_remove_league_table_element(const OpenTTDObjectivesServices *, uint16_t, uint16_t);
bool openttd_rust_valid_link(const OpenTTDObjectivesServices *, uint8_t, uint32_t);
void openttd_rust_story_page_destruct(const OpenTTDObjectivesServices *, uint16_t);
bool openttd_rust_story_button_validate_colour(uint32_t);
bool openttd_rust_story_button_validate_flags(uint32_t);
bool openttd_rust_story_button_validate_cursor(uint32_t);
bool openttd_rust_story_button_validate_vehicle_type(uint32_t);
}
#endif /* RUST_OBJECTIVES_FFI_H */

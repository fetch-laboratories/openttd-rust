/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_objectives.h Native objectives shell and GUI/VM operations. */
#ifndef RUST_OBJECTIVES_SERVICES_H
#define RUST_OBJECTIVES_SERVICES_H
#include "objectives_ffi.h"
#include "company_adapter.h"
#include "presentation_services.h"
#include "orders_ffi.h"
#include "road_command_ffi.h"
#include "../goal_base.h"
#include "../story_base.h"
#include "../league_base.h"
#include "../industry.h"
#include "../town.h"
#include "../vehicle_base.h"
#include "../gui.h"
#include "../window_func.h"
#include "../game/game.hpp"
#include "../network/network_base.h"
#include "../network/network_func.h"
#include "../script/api/script_story_page.hpp"
#include "../script/api/script_event_types.hpp"
inline OpenTTDGoalRecord *RustObjectives_goal_allocate() noexcept { return (new Goal())->record.get(); }
inline OpenTTDStoryPageRecord *RustObjectives_page_allocate() noexcept { return (new StoryPage())->record.get(); }
inline OpenTTDStoryPageElementRecord *RustObjectives_element_allocate() noexcept { return (new StoryPageElement())->record.get(); }
inline OpenTTDLeagueTableRecord *RustObjectives_table_allocate() noexcept { return (new LeagueTable())->record.get(); }
inline OpenTTDLeagueTableElementRecord *RustObjectives_entry_allocate() noexcept { return (new LeagueTableElement())->record.get(); }
inline void RustObjectives_element_destroy(OpenTTDStoryPageElementRecord *p) noexcept { delete p->shell; }
inline void RustObjectives_entry_destroy(OpenTTDLeagueTableElementRecord *p) noexcept { delete p->shell; }
inline OpenTTDByteSpan RustObjectives_text_view(const EncodedString *text) noexcept { return RustEncodedString::Span(*text); }
inline bool RustObjectives_vehicle_valid(uint32_t id) noexcept { return Vehicle::IsValidID(VehicleID(id)); }
inline void RustObjectives_window_close(uint16_t cls, uint32_t number) noexcept { CloseWindowById(WindowClass(cls), number); }
inline void RustObjectives_question_show(uint16_t id, uint8_t type, uint32_t buttons, const EncodedString * text) noexcept { ShowGoalQuestion(id, GoalQuestionType(type), buttons, *text); }
inline void RustObjectives_story_show(uint8_t company, uint16_t page) noexcept { ShowStoryBook(CompanyID(company), StoryPageID(page), true); }
inline void RustObjectives_question_event(uint16_t id, uint8_t company, uint32_t button) noexcept { Game::NewEvent(new ScriptEventGoalQuestionAnswer(id, CompanyID(company), ScriptGoal::QuestionButton(button))); }
inline void RustObjectives_push_event(uint8_t company, uint16_t page, uint16_t element) noexcept { Game::NewEvent(new ScriptEventStoryPageButtonClick(CompanyID(company), StoryPageID(page), StoryPageElementID(element))); }
inline void RustObjectives_tile_event(uint8_t company, uint16_t page, uint16_t element, uint32_t tile) noexcept { Game::NewEvent(new ScriptEventStoryPageTileSelect(CompanyID(company), StoryPageID(page), StoryPageElementID(element), TileIndex(tile))); }
inline void RustObjectives_vehicle_event(uint8_t company, uint16_t page, uint16_t element, uint32_t vehicle) noexcept { Game::NewEvent(new ScriptEventStoryPageVehicleSelect(CompanyID(company), StoryPageID(page), StoryPageElementID(element), VehicleID(vehicle))); }
inline const OpenTTDObjectivesServices &GetRustObjectivesServices() noexcept
{
	static const OpenTTDObjectivesServices services{
		.shared = &GetRustSharedServices(),
		.company = &GetRustCompanyServices(),
		.goal_allocate = RustObjectives_goal_allocate,
		.page_allocate = RustObjectives_page_allocate,
		.element_allocate = RustObjectives_element_allocate,
		.table_allocate = RustObjectives_table_allocate,
		.entry_allocate = RustObjectives_entry_allocate,
		.element_destroy = RustObjectives_element_destroy,
		.entry_destroy = RustObjectives_entry_destroy,
		.orders = &GetRustOrdersLeaves(),
		.subsidy = &GetRustSubsidyServices(),
		.road_commands = &GetRustRoadCommandServices(),
		.vehicle_valid = RustObjectives_vehicle_valid,
		.text_view = RustObjectives_text_view,
		.window_data = RustInvalidateWindowData,
		.window_classes = RustInvalidateWindowClassesData,
		.window_close = RustObjectives_window_close,
		.question_show = RustObjectives_question_show,
		.story_show = RustObjectives_story_show,
		.question_event = RustObjectives_question_event,
		.push_event = RustObjectives_push_event,
		.tile_event = RustObjectives_tile_event,
		.vehicle_event = RustObjectives_vehicle_event,
	};
	return services;
}
static_assert(COLOUR_END == 16 && SPBC_END == 55);
static_assert(VEH_INVALID == 255 && VEH_COMPANY_END == 4);
static_assert(GT_NONE == 0 && GT_TILE == 1 && GT_INDUSTRY == 2 && GT_TOWN == 3 && GT_COMPANY == 4 && GT_STORY_PAGE == 5);
static_assert(LT_NONE == 0 && LT_TILE == 1 && LT_INDUSTRY == 2 && LT_TOWN == 3 && LT_COMPANY == 4 && LT_STORY_PAGE == 5);
static_assert(SPET_TEXT == 0 && SPET_LOCATION == 1 && SPET_GOAL == 2 && SPET_BUTTON_PUSH == 3 && SPET_BUTTON_TILE == 4 && SPET_BUTTON_VEHICLE == 5);
static_assert(SPBF_FLOAT_LEFT == 1 && SPBF_FLOAT_RIGHT == 2 && GOAL_QUESTION_BUTTON_COUNT == 18 && GQT_QUESTION == 0 && GQT_END == 4);
static_assert(sizeof(GoalID) == sizeof(uint16_t) && sizeof(StoryPageID) == sizeof(uint16_t) && sizeof(StoryPageElementID) == sizeof(uint16_t));
static_assert(sizeof(LeagueTableID) == sizeof(uint8_t) && sizeof(LeagueTableElementID) == sizeof(uint16_t));
#endif /* RUST_OBJECTIVES_SERVICES_H */
static_assert(WC_MAIN_TOOLBAR == 1);
static_assert(WC_GOALS_LIST == 38);
static_assert(WC_GOAL_QUESTION == 13);
static_assert(WC_STORY_BOOK == 39);
static_assert(WC_COMPANY_LEAGUE == 81);

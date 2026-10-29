/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_objectives.h Canonical native objectives record access. */
#ifndef RUST_OBJECTIVES_SERVICES_H
#define RUST_OBJECTIVES_SERVICES_H
#include "objectives_ffi.h"
#include "company_adapter.h"
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
inline Goal * RustObjectives_goal(uint16_t id) noexcept { return Goal::GetIfValid(GoalID(id)); }
inline uint8_t * RustObjectives_goal_company(Goal * p) noexcept { return reinterpret_cast<uint8_t *>(&p->company); }
inline uint8_t * RustObjectives_goal_type(Goal * p) noexcept { return reinterpret_cast<uint8_t *>(&p->type); }
inline uint32_t * RustObjectives_goal_dst(Goal * p) noexcept { return reinterpret_cast<uint32_t *>(&p->dst); }
inline EncodedString * RustObjectives_goal_text(Goal * p) noexcept { return reinterpret_cast<EncodedString *>(&p->text); }
inline EncodedString * RustObjectives_goal_progress(Goal * p) noexcept { return reinterpret_cast<EncodedString *>(&p->progress); }
inline bool * RustObjectives_goal_completed(Goal * p) noexcept { return reinterpret_cast<bool *>(&p->completed); }
inline uint32_t RustObjectives_goal_count() noexcept { return Goal::GetNumItems(); }
inline bool RustObjectives_goal_can_allocate() noexcept { return Goal::CanAllocateItem(); }
inline StoryPage * RustObjectives_page(uint32_t id) noexcept { return StoryPage::GetIfValid(id); }
inline uint8_t * RustObjectives_page_company(StoryPage * p) noexcept { return reinterpret_cast<uint8_t *>(&p->company); }
inline EncodedString * RustObjectives_page_title(StoryPage * p) noexcept { return reinterpret_cast<EncodedString *>(&p->title); }
inline int32_t * RustObjectives_page_date(StoryPage * p) noexcept { return reinterpret_cast<int32_t *>(&p->date); }
inline uint32_t RustObjectives_page_count() noexcept { return StoryPage::GetNumItems(); }
inline bool RustObjectives_page_can_allocate() noexcept { return StoryPage::CanAllocateItem(); }
inline StoryPageElement * RustObjectives_element(uint16_t id) noexcept { return StoryPageElement::GetIfValid(StoryPageElementID(id)); }
inline uint16_t * RustObjectives_element_page(StoryPageElement * p) noexcept { return reinterpret_cast<uint16_t *>(&p->page); }
inline uint8_t * RustObjectives_element_type(StoryPageElement * p) noexcept { return reinterpret_cast<uint8_t *>(&p->type); }
inline EncodedString * RustObjectives_element_text(StoryPageElement * p) noexcept { return reinterpret_cast<EncodedString *>(&p->text); }
inline uint32_t * RustObjectives_element_referenced_id(StoryPageElement * p) noexcept { return reinterpret_cast<uint32_t *>(&p->referenced_id); }
inline void RustObjectives_element_destroy(StoryPageElement * p) noexcept { delete p; }
inline uint32_t RustObjectives_element_count() noexcept { return StoryPageElement::GetNumItems(); }
inline bool RustObjectives_element_can_allocate() noexcept { return StoryPageElement::CanAllocateItem(); }
inline LeagueTableElement * RustObjectives_entry(uint16_t id) noexcept { return LeagueTableElement::GetIfValid(LeagueTableElementID(id)); }
inline uint8_t * RustObjectives_entry_company(LeagueTableElement * p) noexcept { return reinterpret_cast<uint8_t *>(&p->company); }
inline uint8_t * RustObjectives_entry_table(LeagueTableElement * p) noexcept { return reinterpret_cast<uint8_t *>(&p->table); }
inline int64_t * RustObjectives_entry_rating(LeagueTableElement * p) noexcept { return reinterpret_cast<int64_t *>(&p->rating); }
inline EncodedString * RustObjectives_entry_text(LeagueTableElement * p) noexcept { return reinterpret_cast<EncodedString *>(&p->text); }
inline EncodedString * RustObjectives_entry_score(LeagueTableElement * p) noexcept { return reinterpret_cast<EncodedString *>(&p->score); }
inline uint8_t * RustObjectives_entry_link_type(LeagueTableElement * p) noexcept { return reinterpret_cast<uint8_t *>(&p->link.type); }
inline uint32_t * RustObjectives_entry_link_target(LeagueTableElement * p) noexcept { return reinterpret_cast<uint32_t *>(&p->link.target); }
inline void RustObjectives_entry_destroy(LeagueTableElement * p) noexcept { delete p; }
inline bool RustObjectives_entry_can_allocate() noexcept { return LeagueTableElement::CanAllocateItem(); }
inline uint16_t RustObjectives_goal_allocate(uint8_t type, uint32_t dest, uint8_t company, const EncodedString * text) noexcept { return (new Goal(GoalType(type), dest, CompanyID(company), *text))->index.base(); }
inline uint16_t RustObjectives_page_allocate(uint32_t sort, int32_t date, uint8_t company, const EncodedString * text) noexcept { return (new StoryPage(sort, TimerGameCalendar::Date(date), CompanyID(company), *text))->index.base(); }
inline uint16_t RustObjectives_element_allocate(uint32_t sort, uint8_t type, uint16_t page) noexcept { return (new StoryPageElement(sort, StoryPageElementType(type), StoryPageID(page)))->index.base(); }
inline StoryPageElement * RustObjectives_element_next(uint32_t from) noexcept { for (auto *p : StoryPageElement::Iterate(from)) return p; return nullptr; }
inline uint16_t RustObjectives_element_id(StoryPageElement * p) noexcept { return p->index.base(); }
inline bool RustObjectives_page_cleaning() noexcept { return StoryPage::CleaningPool(); }
inline bool RustObjectives_table_can_allocate() noexcept { return LeagueTable::CanAllocateItem(); }
inline uint8_t RustObjectives_table_allocate(const EncodedString * title, const EncodedString * header, const EncodedString * footer) noexcept { return (new LeagueTable(*title, *header, *footer))->index.base(); }
inline uint16_t RustObjectives_entry_allocate(uint8_t table, int64_t rating, uint8_t company, const EncodedString * text, const EncodedString * score, uint8_t type, uint32_t target) noexcept { return (new LeagueTableElement(LeagueTableID(table), rating, CompanyID(company), *text, *score, Link(LinkType(type), target)))->index.base(); }
inline uint32_t RustObjectives_own_client() noexcept { return static_cast<uint32_t>(_network_own_client_id); }
inline bool RustObjectives_client_valid(uint32_t id) noexcept { return NetworkClientInfo::GetByClientID(ClientID(id)) != nullptr; }
inline bool RustObjectives_industry_valid(uint32_t id) noexcept { return Industry::IsValidID(id); }
inline bool RustObjectives_town_valid(uint32_t id) noexcept { return Town::IsValidID(id); }
inline bool RustObjectives_vehicle_valid(uint32_t id) noexcept { return Vehicle::IsValidID(VehicleID(id)); }
inline bool RustObjectives_text_empty(const EncodedString * text) noexcept { return text->empty(); }
inline void RustObjectives_text_assign(EncodedString * to, const EncodedString * text) noexcept { *to = *text; }
inline void RustObjectives_window_data(uint16_t cls, uint32_t number, int32_t data) noexcept { InvalidateWindowData(WindowClass(cls), number, data); }
inline void RustObjectives_window_classes(uint16_t cls, int32_t data) noexcept { InvalidateWindowClassesData(WindowClass(cls), data); }
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
		.map_allocation = GetRustTileTypeAllocation,
		.goal = RustObjectives_goal,
		.goal_company = RustObjectives_goal_company,
		.goal_type = RustObjectives_goal_type,
		.goal_dst = RustObjectives_goal_dst,
		.goal_text = RustObjectives_goal_text,
		.goal_progress = RustObjectives_goal_progress,
		.goal_completed = RustObjectives_goal_completed,
		.goal_count = RustObjectives_goal_count,
		.goal_can_allocate = RustObjectives_goal_can_allocate,
		.page = RustObjectives_page,
		.page_company = RustObjectives_page_company,
		.page_title = RustObjectives_page_title,
		.page_date = RustObjectives_page_date,
		.page_count = RustObjectives_page_count,
		.page_can_allocate = RustObjectives_page_can_allocate,
		.element = RustObjectives_element,
		.element_page = RustObjectives_element_page,
		.element_type = RustObjectives_element_type,
		.element_text = RustObjectives_element_text,
		.element_referenced_id = RustObjectives_element_referenced_id,
		.element_destroy = RustObjectives_element_destroy,
		.element_count = RustObjectives_element_count,
		.element_can_allocate = RustObjectives_element_can_allocate,
		.entry = RustObjectives_entry,
		.entry_company = RustObjectives_entry_company,
		.entry_table = RustObjectives_entry_table,
		.entry_rating = RustObjectives_entry_rating,
		.entry_text = RustObjectives_entry_text,
		.entry_score = RustObjectives_entry_score,
		.entry_link_type = RustObjectives_entry_link_type,
		.entry_link_target = RustObjectives_entry_link_target,
		.entry_destroy = RustObjectives_entry_destroy,
		.entry_can_allocate = RustObjectives_entry_can_allocate,
		.goal_allocate = RustObjectives_goal_allocate,
		.page_allocate = RustObjectives_page_allocate,
		.element_allocate = RustObjectives_element_allocate,
		.element_next = RustObjectives_element_next,
		.element_id = RustObjectives_element_id,
		.page_cleaning = RustObjectives_page_cleaning,
		.table_can_allocate = RustObjectives_table_can_allocate,
		.table_allocate = RustObjectives_table_allocate,
		.entry_allocate = RustObjectives_entry_allocate,
		.own_client = RustObjectives_own_client,
		.client_valid = RustObjectives_client_valid,
		.industry_valid = RustObjectives_industry_valid,
		.town_valid = RustObjectives_town_valid,
		.vehicle_valid = RustObjectives_vehicle_valid,
		.text_empty = RustObjectives_text_empty,
		.text_assign = RustObjectives_text_assign,
		.window_data = RustObjectives_window_data,
		.window_classes = RustObjectives_window_classes,
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

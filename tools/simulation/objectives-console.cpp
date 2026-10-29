/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file objectives-console.cpp Production command results and unsaved GS event observations. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "company_func.h"
#include "company_base.h"
#include "core/backup_type.hpp"
#include "goal_cmd.h"
#include "goal_base.h"
#include "story_cmd.h"
#include "story_base.h"
#include "league_cmd.h"
#include "league_base.h"
#include "industry.h"
#include "town.h"
#include "vehicle_base.h"
#include "network/network.h"
#include "network/network_base.h"
#include "saveload/saveload.h"
#include "script/api/script_event_types.hpp"
#include "string_func.h"
#include "table/strings.h"
#include "core/random_func.hpp"
#include <thread>

static bool capture_events = false;
static unsigned event_count = 0;
extern "C" void __real__ZN4Game8NewEventEP11ScriptEvent(ScriptEvent *);
extern "C" void __wrap__ZN4Game8NewEventEP11ScriptEvent(ScriptEvent *event)
{
	if (capture_events) {
		if (auto *e = dynamic_cast<ScriptEventGoalQuestionAnswer *>(event)) fmt::print(stderr, "OBJECTIVES event {} answer {} {} {}\n", event_count++, e->GetUniqueID(), e->GetCompany(), e->GetButton());
		if (auto *e = dynamic_cast<ScriptEventStoryPageButtonClick *>(event)) fmt::print(stderr, "OBJECTIVES event {} push {} {} {}\n", event_count++, e->GetCompanyID(), e->GetStoryPageID(), e->GetElementID());
		if (auto *e = dynamic_cast<ScriptEventStoryPageTileSelect *>(event)) fmt::print(stderr, "OBJECTIVES event {} tile {} {} {} {}\n", event_count++, e->GetCompanyID(), e->GetStoryPageID(), e->GetElementID(), e->GetTile());
		if (auto *e = dynamic_cast<ScriptEventStoryPageVehicleSelect *>(event)) fmt::print(stderr, "OBJECTIVES event {} vehicle {} {} {} {}\n", event_count++, e->GetCompanyID(), e->GetStoryPageID(), e->GetElementID(), e->GetVehicleID());
	}
	__real__ZN4Game8NewEventEP11ScriptEvent(event);
}
static void Outcome(std::string_view name, const CommandCost &cost, uint32_t id = UINT32_MAX)
{
	fmt::print(stderr, "OBJECTIVES result {} {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), id);
}
template <Commands C, typename... Args> static auto Observe(std::string_view name, DoCommandFlags flags, Args &&...args)
{
	auto result = Command<C>::Do(flags, std::forward<Args>(args)...);
	if constexpr (requires { std::get<0>(result); }) Outcome(name, std::get<0>(result), std::get<1>(result).base());
	else Outcome(name, result);
	return result;
}
template <typename ID> static ID Created(const std::tuple<CommandCost, ID> &result)
{
	if (std::get<0>(result).Failed() || std::get<1>(result) == ID::Invalid()) std::abort();
	return std::get<1>(result);
}
static void State()
{
	for (const Goal *g : Goal::Iterate()) fmt::print(stderr, "OBJECTIVES goal {} {} {} {} {}\n", g->index, g->company, g->type, g->dst, g->completed);
	for (const StoryPage *p : StoryPage::Iterate()) fmt::print(stderr, "OBJECTIVES page {} {} {} {}\n", p->index, p->sort_value, p->company, p->date);
	for (const StoryPageElement *e : StoryPageElement::Iterate()) fmt::print(stderr, "OBJECTIVES element {} {} {} {} {}\n", e->index, e->sort_value, e->page, e->type, e->referenced_id);
	for (const LeagueTableElement *e : LeagueTableElement::Iterate()) fmt::print(stderr, "OBJECTIVES entry {} {} {} {} {} {}\n", e->index, e->table, e->rating, e->company, e->link.type, e->link.target);
	fmt::print(stderr, "OBJECTIVES rng {} {}\n", _random.state[0], _random.state[1]);
	fmt::print(stderr, "OBJECTIVES state-complete\n");
}
static void SortStateProbe(StoryPageID page, TileIndex tile, const EncodedString &text)
{
	bool shared = false;
	std::thread check([&] {
#ifdef WITH_RUST
		auto *state = openttd_rust_story_storage();
		shared = &state->page_next_sort == &_story_page_next_sort_value && &state->element_next_sort == &_story_page_element_next_sort_value;
#else
		shared = true;
#endif
	});
	check.join();
	fmt::print(stderr, "OBJECTIVES state-shared {}\n", shared);
	uint32_t saved_page = _story_page_next_sort_value, saved_element = _story_page_element_next_sort_value;
	const auto run = DoCommandFlag::Execute;
	_story_page_next_sort_value = UINT32_MAX;
	auto wrapped = Created(Observe<CMD_CREATE_STORY_PAGE>("page-sort-wrap", run, CompanyID::Invalid(), text));
	fmt::print(stderr, "OBJECTIVES page-wrap {} {}\n", StoryPage::Get(wrapped)->sort_value, _story_page_next_sort_value);
	Observe<CMD_REMOVE_STORY_PAGE>("page-sort-cleanup", run, wrapped);
	_story_page_next_sort_value = saved_page;
	auto primer = Created(Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("element-sort-primer", run, tile, page, SPET_TEXT, 0, text));
	_story_page_element_next_sort_value = UINT32_MAX;
	auto element = Created(Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("element-sort-wrap", run, tile, page, SPET_TEXT, 0, text));
	fmt::print(stderr, "OBJECTIVES element-wrap {} {}\n", StoryPageElement::Get(element)->sort_value, _story_page_element_next_sort_value);
	Observe<CMD_REMOVE_STORY_PAGE_ELEMENT>("element-sort-cleanup", run, element);
	Observe<CMD_REMOVE_STORY_PAGE_ELEMENT>("element-sort-primer-cleanup", run, primer);
	_story_page_element_next_sort_value = UINT32_MAX;
	auto reset = Created(Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("element-sort-reset", run, tile, page, SPET_TEXT, 0, text));
	fmt::print(stderr, "OBJECTIVES element-reset {} {}\n", StoryPageElement::Get(reset)->sort_value, _story_page_element_next_sort_value);
	Observe<CMD_REMOVE_STORY_PAGE_ELEMENT>("element-sort-reset-cleanup", run, reset);
	_story_page_element_next_sort_value = saved_element;
}
static bool ObjectivesScenario([[maybe_unused]] uint8_t argc, const char * const *argv)
{
	if (argc < 2) return false;
	if (std::string_view(argv[1]) == "inspect") {
		State();
		Backup<CompanyID> actor(_current_company, OWNER_DEITY);
		const auto text = GetEncodedString(STR_JUST_RAW_STRING, "reload");
		uint32_t page_sort = _story_page_next_sort_value, element_sort = _story_page_element_next_sort_value;
		auto page = Created(Observe<CMD_CREATE_STORY_PAGE>("reload-page", DoCommandFlag::Execute, CompanyID::Invalid(), text));
		auto element = Created(Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("reload-element", DoCommandFlag::Execute, TileXY(10, 10), page, SPET_TEXT, 0, text));
		fmt::print(stderr, "OBJECTIVES loaded-sorts {} {} {} {}\n", page_sort, StoryPage::Get(page)->sort_value, element_sort, StoryPageElement::Get(element)->sort_value);
		fmt::print(stderr, "OBJECTIVES reload-complete\n");
		actor.Restore();
		return true;
	}
	Backup<CompanyID> actor(_current_company, OWNER_DEITY);
	const auto run = DoCommandFlag::Execute;
	const CompanyID global = CompanyID::Invalid(), company = (*Company::Iterate().begin())->index;
	CompanyID other = global;
	for (const Company *c : Company::Iterate()) if (c->index != company) { other = c->index; break; }
	if (other == global) { auto *c = DoStartupNewCompany(false); if (c == nullptr) std::abort(); other = c->index; _current_company = OWNER_DEITY; }
	const EncodedString text = GetEncodedString(STR_JUST_RAW_STRING, "objectives"), empty;
	if (StoryPage::GetNumItems() == 0) _story_page_next_sort_value = UINT32_MAX;
	const StoryPageID page = Created(Observe<CMD_CREATE_STORY_PAGE>("page-global", run, global, text));
	const StoryPageID private_page = Created(Observe<CMD_CREATE_STORY_PAGE>("page-company", run, company, empty));
	const StoryPageID other_page = Created(Observe<CMD_CREATE_STORY_PAGE>("page-other", run, other, text));
	Observe<CMD_CREATE_STORY_PAGE>("page-test", {}, global, text);
	Observe<CMD_CREATE_STORY_PAGE>("page-invalid-company", run, CompanyID(254), text);
	const GoalID goal = Created(Observe<CMD_CREATE_GOAL>("goal-global", run, global, GT_STORY_PAGE, page.base(), text));
	const GoalID private_goal = Created(Observe<CMD_CREATE_GOAL>("goal-company", run, company, GT_STORY_PAGE, private_page.base(), text));
	const TileIndex tile = TileXY(10, 10);
	SortStateProbe(page, tile, text);
	for (uint8_t kind = 0; kind <= 6; ++kind) {
		uint32_t valid = 0;
		if (kind == GT_TILE) valid = tile.base();
		if (kind == GT_INDUSTRY) valid = (*Industry::Iterate().begin())->index.base();
		if (kind == GT_TOWN) valid = (*Town::Iterate().begin())->index.base();
		if (kind == GT_COMPANY) valid = company.base();
		if (kind == GT_STORY_PAGE) valid = page.base();
		for (uint32_t dest : {valid, UINT32_MAX, valid + 0x10000U}) {
			for (DoCommandFlags flags : {DoCommandFlags{}, DoCommandFlags{run}}) {
				auto result = Observe<CMD_CREATE_GOAL>(fmt::format("destination-{}-{}-{}", kind, dest, flags.base()), flags, global, GoalType(kind), dest, text);
				if (std::get<0>(result).Succeeded() && flags.Test(run)) Observe<CMD_REMOVE_GOAL>(fmt::format("destination-remove-{}-{}", kind, dest), run, std::get<1>(result));
			}
		}
	}
	Observe<CMD_CREATE_GOAL>("goal-empty", run, global, GT_NONE, 0, empty);
	Observe<CMD_CREATE_GOAL>("goal-global-private", run, global, GT_STORY_PAGE, private_page.base(), text);
	if (other != global) Observe<CMD_CREATE_GOAL>("goal-wrong-company", run, company, GT_STORY_PAGE, other_page.base(), text);
	Observe<CMD_SET_GOAL_DESTINATION>("goal-destination", run, goal, GT_TILE, tile.base());
	Observe<CMD_SET_GOAL_DESTINATION>("goal-destination-refused", run, goal, GT_STORY_PAGE, private_page.base());
	Observe<CMD_SET_GOAL_TEXT>("goal-text-test", {}, goal, text);
	Observe<CMD_SET_GOAL_TEXT>("goal-text-empty", run, goal, empty);
	Observe<CMD_SET_GOAL_TEXT>("goal-text", run, goal, text);
	Observe<CMD_SET_GOAL_PROGRESS>("goal-progress", run, goal, text);
	Observe<CMD_SET_GOAL_PROGRESS>("goal-progress-empty", run, goal, empty);
	Observe<CMD_SET_GOAL_COMPLETED>("goal-completed", run, goal, true);
	Observe<CMD_REMOVE_GOAL>("goal-remove-test", {}, private_goal);
	Observe<CMD_REMOVE_GOAL>("goal-remove-invalid", run, GoalID::Invalid());
	const auto element = [&](std::string_view name, StoryPageID p, StoryPageElementType kind, uint32_t ref) {
		return Created(Observe<CMD_CREATE_STORY_PAGE_ELEMENT>(name, run, tile, p, kind, ref, text));
	};
	const auto first = element("element-global", page, SPET_TEXT, 0);
	element("element-interleaved", private_page, SPET_GOAL, private_goal.base());
	const auto location = element("element-location", page, SPET_LOCATION, 0);
	const auto push = element("element-push", private_page, SPET_BUTTON_PUSH, 0);
	const auto tile_button = element("element-tile", page, SPET_BUTTON_TILE, 0);
	const auto vehicle_button = element("element-vehicle", page, SPET_BUTTON_VEHICLE, 0);
	Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("element-goal-refused", run, tile, page, SPET_GOAL, private_goal.base(), empty);
	for (uint8_t kind = 0; kind <= 6; ++kind) for (uint32_t ref : {UINT32_MAX, 0x03000000U, 0x0000FF00U, 0x00FE0000U}) {
		Observe<CMD_CREATE_STORY_PAGE_ELEMENT>(fmt::format("element-invalid-{}-{}", kind, ref), {}, kind == SPET_LOCATION ? INVALID_TILE : TileIndex{}, page, StoryPageElementType(kind), ref, empty);
	}
	Observe<CMD_UPDATE_STORY_PAGE_ELEMENT>("element-update-refused", run, INVALID_TILE, location, 0, text);
	Observe<CMD_UPDATE_STORY_PAGE_ELEMENT>("element-update", run, tile + 1, location, 0, text);
	Observe<CMD_SET_STORY_PAGE_TITLE>("page-title", run, page, empty);
	Observe<CMD_SET_STORY_PAGE_DATE>("page-date", run, page, TimerGameCalendar::Date(-123));
	Observe<CMD_SHOW_STORY_PAGE>("page-show-test", {}, page);
	capture_events = true;
	{
		Backup<CompanyID> player(_current_company, company);
		Observe<CMD_CREATE_GOAL>("company-refusal", run, global, GT_NONE, 0, text);
		Observe<CMD_STORY_PAGE_BUTTON>("button-push-test", {}, TileIndex{}, push, VehicleID::Invalid());
		Observe<CMD_STORY_PAGE_BUTTON>("button-push", run, TileIndex{}, push, VehicleID::Invalid());
		Observe<CMD_STORY_PAGE_BUTTON>("button-tile-invalid", run, INVALID_TILE, tile_button, VehicleID::Invalid());
		Observe<CMD_STORY_PAGE_BUTTON>("button-tile", run, tile, tile_button, VehicleID::Invalid());
		Observe<CMD_STORY_PAGE_BUTTON>("button-vehicle-invalid", run, tile, vehicle_button, VehicleID::Invalid());
		Observe<CMD_STORY_PAGE_BUTTON>("button-vehicle", run, tile, vehicle_button, (*Vehicle::Iterate().begin())->index);
		Observe<CMD_STORY_PAGE_BUTTON>("button-nonbutton", run, tile, first, VehicleID::Invalid());
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer-test", {}, 42, 0);
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer", run, 42, 17);
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer-invalid", run, 42, 18);
		Backup<bool> network(_networking, true), server(_network_server, false);
		Backup<CompanyID> local(_local_company, company);
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer-client", run, 43, 0);
		_network_server = true;
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer-server", run, 44, 1);
		_network_server = false; _local_company = global;
		Observe<CMD_GOAL_QUESTION_ANSWER>("answer-remote", run, 47, 2);
		server.Restore(); network.Restore(); local.Restore();
		if (other != global) {
			_current_company = other;
			Observe<CMD_STORY_PAGE_BUTTON>("button-wrong-company", run, tile, push, VehicleID::Invalid());
		}
		player.Restore();
	}
	Observe<CMD_GOAL_QUESTION_ANSWER>("answer-deity", run, 45, 1);
	for (uint8_t kind = 0; kind <= 4; ++kind) for (uint32_t buttons : {0U, 1U, 7U, 15U, UINT32_MAX}) {
		Observe<CMD_GOAL_QUESTION>(fmt::format("question-{}-{}", kind, buttons), {}, 42, global.base(), false, buttons, GoalQuestionType(kind), text);
	}
	{
		Backup<bool> server(_network_server, true);
		Observe<CMD_GOAL_QUESTION>("question-missing-client-test", {}, 46, UINT32_MAX, true, 1, GQT_QUESTION, text);
		/* Network replay executes after its earlier test; the client can leave in between. */
		Outcome("question-missing-client-execute", CmdGoalQuestion(run, 46, UINT32_MAX, true, 1, GQT_QUESTION, text));
		server.Restore();
	}
	capture_events = false;
	fmt::print(stderr, "OBJECTIVES events-complete {}\n", event_count);
	const auto table = Created(Observe<CMD_CREATE_LEAGUE_TABLE>("table", run, text, text, empty));
	Observe<CMD_CREATE_LEAGUE_TABLE>("table-test", {}, text, empty, text);
	Observe<CMD_CREATE_LEAGUE_TABLE>("table-empty", run, empty, text, text);
	for (int64_t rating : std::initializer_list<int64_t>{-100, 0, -100, INT64_MIN, INT64_MAX}) Created(Observe<CMD_CREATE_LEAGUE_TABLE_ELEMENT>(fmt::format("league-score-{}-{}", rating, LeagueTableElement::GetNumItems()), run, table, rating, global, text, text, LT_STORY_PAGE, page.base()));
	for (uint8_t kind = 0; kind <= 6; ++kind) Observe<CMD_CREATE_LEAGUE_TABLE_ELEMENT>(fmt::format("league-link-{}", kind), run, table, -1, company, text, text, LinkType(kind), UINT32_MAX);
	const auto entry = Created(Observe<CMD_CREATE_LEAGUE_TABLE_ELEMENT>("league-entry", run, table, -8, company, empty, empty, LT_NONE, 0));
	Observe<CMD_UPDATE_LEAGUE_TABLE_ELEMENT_DATA>("league-data-invalid", run, entry, CompanyID(254), text, LT_NONE, 1);
	Observe<CMD_UPDATE_LEAGUE_TABLE_ELEMENT_DATA>("league-data", run, entry, global, text, LT_TILE, tile.base());
	Observe<CMD_UPDATE_LEAGUE_TABLE_ELEMENT_SCORE>("league-score-test", {}, entry, 999, text);
	Observe<CMD_UPDATE_LEAGUE_TABLE_ELEMENT_SCORE>("league-score", run, entry, -100, text);
	Observe<CMD_REMOVE_LEAGUE_TABLE_ELEMENT>("league-remove-test", {}, entry);
	Observe<CMD_REMOVE_LEAGUE_TABLE_ELEMENT>("league-remove", run, entry);
	Observe<CMD_REMOVE_LEAGUE_TABLE_ELEMENT>("league-remove-invalid", run, entry);
	Observe<CMD_REMOVE_STORY_PAGE>("page-remove-test", {}, private_page);
	Observe<CMD_REMOVE_STORY_PAGE>("page-remove-interleaved", run, private_page);
	Observe<CMD_REMOVE_STORY_PAGE_ELEMENT>("element-remove", run, first);
	Observe<CMD_REMOVE_STORY_PAGE_ELEMENT>("element-remove-invalid", run, first);
	for (unsigned i = 0; i < 128; ++i) Observe<CMD_CREATE_STORY_PAGE_ELEMENT>(fmt::format("page-cap-{}", i), run, tile, other_page, SPET_TEXT, 0, text);
	Observe<CMD_CREATE_STORY_PAGE_ELEMENT>("page-cap-refused", run, tile, other_page, SPET_TEXT, 0, text);
	State();
	if (argc != 3 || SaveOrLoad(argv[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
	fmt::print(stderr, "OBJECTIVES commands-complete\n");
	actor.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("objectives_scenario", ObjectivesScenario);
}

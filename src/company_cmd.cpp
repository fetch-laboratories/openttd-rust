/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file company_cmd.cpp Handling of companies. */

#include "stdafx.h"
#include "company_base.h"
#include "company_func.h"
#include "company_gui.h"
#include "object.h"
#include "core/backup_type.hpp"
#include "town.h"
#include "news_func.h"
#include "command_func.h"
#include "network/network.h"
#include "network/network_func.h"
#include "network/network_base.h"
#include "network/network_admin.h"
#include "ai/ai.hpp"
#include "ai/ai_instance.hpp"
#include "ai/ai_config.hpp"
#include "company_manager_face.h"
#include "window_func.h"
#include "strings_func.h"
#include "sound_func.h"
#include "rail.h"
#include "core/pool_func.hpp"
#include "core/string_consumer.hpp"
#include "settings_func.h"
#include "vehicle_base.h"
#include "vehicle_func.h"
#include "smallmap_gui.h"
#include "game/game.hpp"
#include "goal_base.h"
#include "story_base.h"
#include "company_cmd.h"
#include "timer/timer.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"
#include "timer/timer_window.h"

#include "widgets/statusbar_widget.h"

#include "table/strings.h"
#include "table/company_face.h"

#include "rust/company_adapter.h"
#include "rust/company_identity_ffi.h"
#include "rust/tile_owner_services.h"
#include "rust/company_services.h"
#include "rust/command_services_ffi.h"
#include "rust/command_cost_native_services.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"

void ClearEnginesHiddenFlagOfCompany(CompanyID cid);

CompanyID _local_company;   ///< Company controlled by the human player at this client. Can also be #COMPANY_SPECTATOR.
CompanyID _current_company; ///< Company currently doing an action.
static_assert(sizeof(TypedIndexContainer<std::array<Colours, MAX_COMPANIES>, CompanyID>) == MAX_COMPANIES);
TypedIndexContainer<std::array<Colours, MAX_COMPANIES>, CompanyID> &_company_colours = *new (openttd_rust_company_colours()) TypedIndexContainer<std::array<Colours, MAX_COMPANIES>, CompanyID>{};
std::string _company_manager_face; ///< for company manager face storage in openttd.cfg

CompanyPool _company_pool("Company"); ///< Pool of companies.
INSTANTIATE_POOL_METHODS(Company)

/**
 * Constructor.
 * @param name_1 Name of the company.
 * @param is_ai  A computer program is running for this company.
 */
Company::Company(StringID name_1, bool is_ai)
{
	this->name_1 = name_1;
	this->is_ai = is_ai;
	openttd_rust_company_initialize(&this->Finances(), static_cast<uint32_t>(_settings_game.construction.terraform_frame_burst), static_cast<uint32_t>(_settings_game.construction.clear_frame_burst), static_cast<uint32_t>(_settings_game.construction.tree_frame_burst), static_cast<uint32_t>(_settings_game.construction.build_object_frame_burst));

	InvalidateWindowData(WC_PERFORMANCE_DETAIL, 0, CompanyID::Invalid());
}

/** Destructor. */
Company::~Company()
{
	if (CleaningPool()) return;

	CloseCompanyWindows(this->index);
}

/**
 * Invalidating some stuff after removing item from the pool.
 * @param index index of deleted item
 */
void Company::PostDestructor(size_t index)
{
	InvalidateWindowData(WC_GRAPH_LEGEND, 0, (int)index);
	InvalidateWindowData(WC_PERFORMANCE_DETAIL, 0, (int)index);
	InvalidateWindowData(WC_COMPANY_LEAGUE, 0, 0);
	InvalidateWindowData(WC_LINKGRAPH_LEGEND, 0);
	/* If the currently shown error message has this company in it, then close it. */
	InvalidateWindowData(WC_ERRMSG, 0);
}

/**
 * Calculate the max allowed loan for this company.
 * @return the max loan amount.
 */
Money Company::GetMaxLoan() const
{
	return openttd_rust_company_max_loan(&this->Finances());
}

/**
 * Sets the local company and updates the settings that are set on a
 * per-company basis to reflect the core's state in the GUI.
 * @param new_company the new company
 * @pre Company::IsValidID(new_company) || new_company == COMPANY_SPECTATOR || new_company == OWNER_NONE
 */
void SetLocalCompany(CompanyID new_company)
{
	/* company could also be COMPANY_SPECTATOR or OWNER_NONE */
	assert(Company::IsValidID(new_company) || new_company == COMPANY_SPECTATOR || new_company == OWNER_NONE);

	/* If actually changing to another company, several windows need closing */
	bool switching_company = _local_company != new_company;

	/* Delete the chat window, if you were team chatting. */
	if (switching_company) InvalidateWindowData(WC_SEND_NETWORK_MSG, DESTTYPE_TEAM, _local_company);

	assert(IsLocalCompany());

	_current_company = _local_company = new_company;

	if (switching_company) {
		InvalidateWindowClassesData(WC_COMPANY);
		/* Delete any construction windows... */
		CloseConstructionWindows();
	}

	/* ... and redraw the whole screen. */
	MarkWholeScreenDirty();
	InvalidateWindowClassesData(WC_SIGN_LIST, -1);
	InvalidateWindowClassesData(WC_GOALS_LIST);
	InvalidateWindowClassesData(WC_COMPANY_COLOUR, -1);
	ResetVehicleColourMap();
}

/**
 * Get the colour for DrawString-subroutines which matches the colour of the company.
 * @param company Company to get the colour of.
 * @return Colour of \a company.
 */
TextColour GetDrawStringCompanyColour(CompanyID company)
{
	if (!Company::IsValidID(company)) return GetColourGradient(COLOUR_WHITE, SHADE_NORMAL).ToTextColour();
	return GetColourGradient(_company_colours[company], SHADE_NORMAL).ToTextColour();
}

/**
 * Get the palette for recolouring with a company colour.
 * @param company Company to get the colour of.
 * @return Palette for recolouring.
 */
PaletteID GetCompanyPalette(CompanyID company)
{
	return GetColourPalette(_company_colours[company]);
}

/**
 * Draw the icon of a company.
 * @param c Company that needs its icon drawn.
 * @param x Horizontal coordinate of the icon.
 * @param y Vertical coordinate of the icon.
 */
void DrawCompanyIcon(CompanyID c, int x, int y)
{
	DrawSprite(SPR_COMPANY_ICON, GetCompanyPalette(c), x, y);
}

/**
 * Checks whether a company manager's face is a valid encoding.
 * Unused bits are not enforced to be 0.
 * @param cmf the fact to check
 * @return true if and only if the face is valid
 */

static CompanyMask _dirty_company_finances{}; ///< Bitmask of company finances that should be marked dirty.

/**
 * Mark all finance windows owned by a company as needing a refresh.
 * The actual refresh is deferred until the end of the gameloop to reduce duplicated work.
 * @param company Company that changed, and needs its windows refreshed.
 */
void InvalidateCompanyWindows(const Company *company)
{
	CompanyID cid = company->index;
	_dirty_company_finances.Set(cid);
}

/**
 * Refresh all company finance windows previously marked dirty.
 */
static const IntervalTimer<TimerWindow> invalidate_company_windows_interval(std::chrono::milliseconds(1), [](auto) {
	for (CompanyID cid : _dirty_company_finances) {
		if (cid == _local_company) SetWindowWidgetDirty(WC_STATUS_BAR, 0, WID_S_RIGHT);
		Window *w = FindWindowById(WC_FINANCES, cid);
		if (w != nullptr) {
			w->SetWidgetDirty(WID_CF_EXPS_PRICE3);
			w->SetWidgetDirty(WID_CF_OWN_VALUE);
			w->SetWidgetDirty(WID_CF_LOAN_VALUE);
			w->SetWidgetDirty(WID_CF_BALANCE_VALUE);
			w->SetWidgetDirty(WID_CF_MAXLOAN_VALUE);
		}
		SetWindowWidgetDirty(WC_COMPANY, cid, WID_C_DESC_COMPANY_VALUE);
	}
	_dirty_company_finances.Reset();
});

/**
 * Get the amount of money that a company has available, or INT64_MAX
 * if there is no such valid company.
 *
 * @param company Company to check
 * @return The available money of the company or INT64_MAX
 */
Money GetAvailableMoney(CompanyID company)
{
	const Company *c = Company::GetIfValid(company);
	return openttd_rust_company_available_money(c == nullptr ? nullptr : &c->Finances(), _settings_game.difficulty.infinite_money);
}

/**
 * This functions returns the money which can be used to execute a command.
 * This is either the money of the current company, or INT64_MAX if infinite money
 * is enabled or there is no such a company "at the moment" like the server itself.
 *
 * @return The available money of the current company or INT64_MAX
 */
Money GetAvailableMoneyForCommand()
{
	return GetAvailableMoney(_current_company);
}

/**
 * Verify whether the company can pay the bill.
 * @param[in,out] cost Money to pay, is changed to an error if the company does not have enough money.
 * @return Function returns \c true if the company has enough money or infinite money is enabled,
 * else it returns \c false.
 */
bool CheckCompanyHasMoney(CommandCost &cost)
{
	return openttd_rust_command_check_money(&GetRustCommandServices(), &GetRustCommandCostServices(), reinterpret_cast<OpenTTDCommandCost *>(&cost));
}

/**
 * Deduct costs of a command from the money of a company.
 * @param c Company to pay the bill.
 * @param cost Money to pay.
 */

/**
 * Subtract money from the #_current_company, if the company is valid.
 * @param cost Money to pay.
 */
void SubtractMoneyFromCompany(const CommandCost &cost)
{
	openttd_rust_command_subtract_money(&GetRustCommandServices(), &GetRustCommandCostServices(), reinterpret_cast<const OpenTTDCommandCost *>(&cost));
}

/**
 * Subtract money from a company, including the money fraction.
 * @param company Company paying the bill.
 * @param cst     Cost of a command.
 */
void SubtractMoneyFromCompanyFract(CompanyID company, const CommandCost &cst)
{
	openttd_rust_company_subtract_fraction(&GetRustCompanyFinanceServices(), &Company::Get(company)->Finances(), company.base(), cst.GetCost().base(), cst.GetExpensesType());
}


OpenTTDCompanyLandscaping RustLandscapingRates() noexcept
{
	const auto &cs = _settings_game.construction;
	return {
		.terraform_per_64k = cs.terraform_per_64k_frames, .clear_per_64k = cs.clear_per_64k_frames,
		.tree_per_64k = cs.tree_per_64k_frames, .object_per_64k = cs.build_object_per_64k_frames,
		.terraform_burst = cs.terraform_frame_burst, .clear_burst = cs.clear_frame_burst,
		.tree_burst = cs.tree_frame_burst, .object_burst = cs.build_object_frame_burst,
	};
}

/** Update the landscaping limits per company. */

/**
 * Set the right DParams for STR_ERROR_OWNED_BY.
 * @param owner the owner to get the name of.
 * @param tile  optional tile to get the right town.
 * @pre if tile == 0, then owner can't be OWNER_TOWN.
 */
std::array<StringParameter, 2> GetParamsForOwnedBy(Owner owner, TileIndex tile)
{
	if (owner == OWNER_TOWN) {
		assert(tile != 0);
		const Town *t = ClosestTownFromTile(tile, UINT_MAX);
		return {STR_TOWN_NAME, t->index};
	}

	if (!Company::IsValidID(owner)) {
		return {STR_COMPANY_SOMEONE, std::monostate{}};
	}

	return {STR_COMPANY_NAME, owner};
}

/**
 * Check whether the current owner owns something.
 * If that isn't the case an appropriate error will be given.
 * @param owner the owner of the thing to check.
 * @param tile  optional tile to get the right town.
 * @pre if tile == 0 then the owner can't be OWNER_TOWN.
 * @return A succeeded command iff it's owned by the current company, else a failed command.
 */
CommandCost CheckOwnership(Owner owner, TileIndex tile)
{
	assert(owner < OWNER_END);
	assert(owner != OWNER_TOWN || tile != 0);

	if (owner == _current_company) return CommandCost();

	CommandCost error{STR_ERROR_OWNED_BY};
	if (RustIsLocalCompany()) {
		RustEncodeOwnedBy(error, owner, tile);
		if (owner != OWNER_TOWN) error.SetErrorOwner(owner);
	}
	return error;
}

/**
 * Check whether the current owner owns the stuff on
 * the given tile.  If that isn't the case an
 * appropriate error will be given.
 * @param tile the tile to check.
 * @return A succeeded command iff it's owned by the current company, else a failed command.
 */
CommandCost CheckTileOwnership(TileIndex tile)
{
	return CheckOwnership(GetTileOwner(tile), tile);
}

/**
 * Generate the name of a company from the last build coordinate.
 * @param c Company to give a name.
 */


/**
 * Generate a company colour.
 * @return Generated company colour.
 */

/**
 * Generate a random president name of a company.
 * @param c Company that needs a new president name.
 */

/**
 * Reset the livery schemes to the company's primary colour.
 * This is used on loading games without livery information and on new company start up.
 * @param c Company to reset.
 */
void ResetCompanyLivery(Company *c)
{
	openttd_rust_company_reset_livery(&GetRustCompanyIdentityServices(), c);
}

/**
 * Create a new company and sets all company variables default values
 *
 * @param is_ai is an AI company?
 * @param company CompanyID to use for the new company
 * @return the company struct
 */
Company *DoStartupNewCompany(bool is_ai, CompanyID company = CompanyID::Invalid())
{
	/* Rust returns before AI::StartNew, whose script memory policy can throw. */
	OpenTTDCompanyStartup frame{.company = UINT32_MAX, .requested = company.base(), .is_ai = is_ai, .stage = 0};
	while (openttd_rust_company_startup(&GetRustCompanyServices(), &frame)) AI::StartNew(CompanyID(frame.company));
	return frame.company == UINT32_MAX ? nullptr : Company::Get(frame.company);
}

/** Start a new competitor company if possible. */
TimeoutTimer<TimerGameTick> _new_competitor_timeout({ TimerGameTick::Priority::COMPETITOR_TIMEOUT, 0 }, []() {
	const OpenTTDCompanyCompetitors competitors{
		.num_companies = Company::GetNumItems(), .interval = _settings_game.difficulty.competitors_interval,
		.menu = _game_mode == GM_MENU, .can_start = AI::CanStartNew(), .networking = _networking,
		.max_companies = _settings_client.network.max_companies, .max_competitors = _settings_game.difficulty.max_no_competitors,
	};
	/* The Post can start an AI synchronously; Rust has returned before it. */
	if (openttd_rust_company_competitor_timeout(&GetRustCompanyServices(), &competitors)) Command<CMD_COMPANY_CTRL>::Post(CCA_NEW_AI, CompanyID::Invalid(), CRR_NONE, INVALID_CLIENT_ID);
});

/** Start of a new game. */
void StartupCompanies()
{
	/* Ensure the timeout is aborted, so it doesn't fire based on information of the last game. */
	_new_competitor_timeout.Abort();
}

/** Initialize the pool of companies. */
void InitializeCompanies()
{
	_cur_company_tick_index = 0;
}

/**
 * Can company \a cbig buy company \a csmall without exceeding vehicle limits?
 * @param cbig   Company buying \a csmall.
 * @param csmall Company getting bought.
 * @return Return \c true if it is allowed.
 */
bool CheckTakeoverVehicleLimit(CompanyID cbig, CompanyID csmall)
{
	return openttd_rust_company_takeover_allowed(&GetRustCompanyServices(), cbig.base(), csmall.base());
}

/**
 * Handle the bankruptcy take over of a company.
 * Companies going bankrupt will ask the other companies in order of their
 * performance rating, so better performing companies get the 'do you want to
 * merge with Y' question earlier. The question will then stay till either the
 * company has gone bankrupt or got merged with a company.
 *
 * @param c the company that is going bankrupt.
 */

OpenTTDCompanyTick RustCompanyTick() noexcept
{
	Company *c = Company::GetIfValid(_cur_company_tick_index);
	return {
		.finances = c == nullptr ? nullptr : &c->Finances(), .num_companies = Company::GetNumItems(),
		.index = 0, .timeout = 0, .interval = _settings_game.difficulty.competitors_interval,
		.editor = _game_mode == GM_EDITOR, .named = c != nullptr && c->name_1 != 0,
		.competitor_due = _new_competitor_timeout.HasFired() && _game_mode != GM_MENU && AI::CanStartNew(),
		.networking = _networking, .max_companies = _settings_client.network.max_companies,
		.max_competitors = _settings_game.difficulty.max_no_competitors, .num_ais = 0, .stage = 0,
	};
}

/** Called every tick for updating some company info. */

OpenTTDCompanyYear RustCompanyClockYear() noexcept
{
	Company *local = Company::GetIfValid(_local_company);
	return {.local_finances = local == nullptr ? nullptr : &local->Finances(), .local = _local_company.base(),
		.show_finances = _settings_client.gui.show_finances, .new_year_sound = _settings_client.sound.new_year};
}

/**
 * A year has passed, update the economic data of all companies, and perhaps show the
 * financial overview window of the local company.
 */
static const IntervalTimer<TimerGameEconomy> _economy_companies_yearly({TimerGameEconomy::YEAR, TimerGameEconomy::Priority::COMPANY}, RustClockCallback{openttd_rust_clock_company_yearly, &GetRustClockCompanyServices()});


/**
 * Fill the CompanyNewsInformation struct with the required data.
 * @param c the current company.
 * @param other the other company (use \c nullptr if not relevant).
 */
CompanyNewsInformation::CompanyNewsInformation(StringID title, const Company *c, const Company *other)
{
	this->company_name = GetString(STR_COMPANY_NAME, c->index);

	if (other != nullptr) {
		this->other_company_name = GetString(STR_COMPANY_NAME, other->index);
		c = other;
	}

	this->president_name = GetString(STR_PRESIDENT_NAME_MANAGER, c->index);

	this->title = title;
	this->colour = c->colour;
	this->face = c->face;

}

/**
 * Called whenever company related information changes in order to notify admins.
 * @param company The company data changed of.
 */
void CompanyAdminUpdate(const Company *company)
{
	if (_network_server) NetworkAdminCompanyUpdate(company);
}

/**
 * Called whenever a company is removed in order to notify admins.
 * @param company_id The company that was removed.
 * @param reason     The reason the company was removed.
 */
void CompanyAdminRemove(CompanyID company_id, CompanyRemoveReason reason)
{
	if (_network_server) NetworkAdminCompanyRemove(company_id, (AdminCompanyRemoveReason)reason);
}

/**
 * Control the companies: add, delete, etc.
 * @param flags operation to perform
 * @param cca action to perform
 * @param company_id company to perform the action on
 * @param client_id ClientID
 * @return the cost of this operation or an error
 */
CommandCost CmdCompanyCtrl(DoCommandFlags flags, CompanyCtrlAction cca, CompanyID company_id, CompanyRemoveReason reason, ClientID client_id)
{
	OpenTTDCompanyControl frame{
		.result = {}, .startup = {}, .client = static_cast<uint32_t>(client_id), .action = static_cast<uint8_t>(cca),
		.target = company_id.base(), .reason = static_cast<uint8_t>(reason), .execute = flags.Test(DoCommandFlag::Execute), .stage = 0,
	};
	RustTileOwnerContext ownership;
	/* Rust returns before AI::StartNew, whose script memory policy can throw. */
	while (openttd_rust_company_control(&GetRustCompanyServices(), &ownership.view, &frame)) AI::StartNew(CompanyID(frame.startup.company));
	return RustCompanyCost(frame.result);
}

static bool ExecuteAllowListCtrlAction(CompanyAllowListCtrlAction action, Company *c, const std::string &public_key)
{
	switch (action) {
		case CALCA_ADD:
			return c->allow_list.Add(public_key);

		case CALCA_REMOVE:
			return c->allow_list.Remove(public_key);

		default:
			NOT_REACHED();
	}
}

/**
 * Add or remove the given public key to the allow list of this company.
 * @param flags Operation to perform.
 * @param action The action to perform.
 * @param public_key The public key of the client to add or remove.
 * @return The cost of this operation or an error.
 */
CommandCost CmdCompanyAllowListCtrl(DoCommandFlags flags, CompanyAllowListCtrlAction action, const std::string &public_key)
{
	Company *c = Company::GetIfValid(_current_company);
	if (c == nullptr) return CMD_ERROR;

	/* The public key length includes the '\0'. */
	if (public_key.size() != NETWORK_PUBLIC_KEY_LENGTH - 1) return CMD_ERROR;

	switch (action) {
		case CALCA_ADD:
		case CALCA_REMOVE:
			break;

		default:
			return CMD_ERROR;
	}

	if (flags.Test(DoCommandFlag::Execute)) {
		if (ExecuteAllowListCtrlAction(action, c, public_key)) {
			InvalidateWindowData(WC_CLIENT_LIST, 0);
			SetWindowDirty(WC_COMPANY, _current_company);
		}
	}

	return CommandCost();
}

/**
 * Change the company manager's face.
 * @param flags operation to perform
 * @param style The style of the company manager face.
 * @param bits The bits of company manager face.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetCompanyManagerFace(DoCommandFlags flags, uint style, uint32_t bits)
{
	return openttd_rust_company_manager_face(&GetRustCompanyIdentityServices(), style, bits, flags.Test(DoCommandFlag::Execute)) ? CommandCost() : CMD_ERROR;
}

/**
 * Update liveries for a company. This is called when the LS_DEFAULT scheme is changed, to update schemes with colours
 * set to default.
 * @param c Company to update.
 */
void UpdateCompanyLiveries(Company *c)
{
	openttd_rust_company_update_liveries(&GetRustCompanyIdentityServices(), c);
}

/**
 * Change the company's company-colour
 * @param flags operation to perform
 * @param scheme scheme to set
 * @param primary set first/second colour
 * @param colour new colour for vehicles, property, etc.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetCompanyColour(DoCommandFlags flags, LiveryScheme scheme, bool primary, Colours colour)
{
	return openttd_rust_company_colour(&GetRustCompanyIdentityServices(), scheme, primary, colour, flags.Test(DoCommandFlag::Execute)) ? CommandCost() : CMD_ERROR;
}

/**
 * Is the given name in use as name of a company?
 * @param name Name to search.
 * @return \c true if the name us unique (that is, not in use), else \c false.
 */

/**
 * Change the name of the company.
 * @param flags operation to perform
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameCompany(DoCommandFlags flags, const std::string &text)
{
	uint8_t result = openttd_rust_company_rename(&GetRustCompanyIdentityServices(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()}, flags.Test(DoCommandFlag::Execute));
	return result == 0 ? CommandCost() : result == 2 ? CommandCost(STR_ERROR_NAME_MUST_BE_UNIQUE) : CMD_ERROR;
}

/**
 * Is the given name in use as president name of a company?
 * @param name Name to search.
 * @return \c true if the name us unique (that is, not in use), else \c false.
 */

/**
 * Change the name of the president.
 * @param flags operation to perform
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenamePresident(DoCommandFlags flags, const std::string &text)
{
	uint8_t result = openttd_rust_president_rename(&GetRustCompanyIdentityServices(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()}, flags.Test(DoCommandFlag::Execute));
	return result == 0 ? CommandCost() : result == 2 ? CommandCost(STR_ERROR_NAME_MUST_BE_UNIQUE) : CMD_ERROR;
}

/**
 * Get the service interval for the given company and vehicle type.
 * @param c The company, or nullptr for client-default settings.
 * @param type The vehicle type to get the interval for.
 * @return The service interval.
 */
int CompanyServiceInterval(const Company *c, VehicleType type)
{
	const VehicleDefaultSettings *vds = (c == nullptr) ? &_settings_client.company.vehicle : &c->settings.vehicle;
	switch (type) {
		default: NOT_REACHED();
		case VEH_TRAIN:    return vds->servint_trains;
		case VEH_ROAD:     return vds->servint_roadveh;
		case VEH_AIRCRAFT: return vds->servint_aircraft;
		case VEH_SHIP:     return vds->servint_ships;
	}
}

/**
 * Get total sum of all owned road bits.
 * @param rtt RoadTramType to get total for.
 * @return Combined total road road bits.
 */
uint32_t CompanyInfrastructure::GetRoadTramTotal(RoadTramType rtt) const
{
	uint32_t total = 0;
	for (RoadType rt : GetMaskForRoadTramType(rtt)) {
		total += this->road[rt];
	}
	return total;
}

/**
 * Transfer funds (money) from one company to another.
 * To prevent abuse in multiplayer games you can only send money to other
 * companies if you have paid off your loan (either explicitly, or implicitly
 * given the fact that you have more money than loan).
 * @param flags operation to perform
 * @param money the amount of money to transfer; max 20.000.000
 * @param dest_company the company to transfer the money to
 * @return the cost of this operation or an error
 */
CommandCost CmdGiveMoney(DoCommandFlags flags, Money money, CompanyID dest_company)
{
	return RustCompanyCost(openttd_rust_company_give_money(&GetRustCompanyFinanceServices(), _settings_game.economy.give_money, money.base(), dest_company.base(), flags.Test(DoCommandFlag::Execute)));
}

/**
 * Get the index of the first available company. It attempts,
 *  from first to last, and as soon as the attempt succeeds,
 *  to get the index of the company:
 *  1st - get the first existing human company.
 *  2nd - get the first non-existing company.
 *  3rd - get CompanyID::Begin().
 * @return the index of the first available company.
 */
CompanyID GetFirstPlayableCompanyID()
{
	for (Company *c : Company::Iterate()) {
		if (Company::IsHumanID(c->index)) {
			return c->index;
		}
	}

	if (Company::CanAllocateItem()) {
		for (CompanyID c = CompanyID::Begin(); c < MAX_COMPANIES; ++c) {
			if (!Company::IsValidID(c)) {
				return c;
			}
		}
	}

	return CompanyID::Begin();
}

static std::vector<FaceSpec> _faces; ///< All company manager face styles.

/**
 * Reset company manager face styles to default.
 */
void ResetFaces()
{
	_faces.clear();
	_faces.assign(std::begin(_original_faces), std::end(_original_faces));
}

/**
 * Get the number of company manager face styles.
 * @return Number of face styles.
 */
uint GetNumCompanyManagerFaceStyles()
{
	return static_cast<uint>(std::size(_faces));
}

/**
 * Get the definition of a company manager face style.
 * @param style_index Face style to get.
 * @return Definition of face style.
 */
const FaceSpec *GetCompanyManagerFaceSpec(uint style_index)
{
	if (style_index < GetNumCompanyManagerFaceStyles()) return &_faces[style_index];
	return nullptr;
}

/**
 * Find a company manager face style by label.
 * @param label Label to find.
 * @return Index of face style if label is found, otherwise std::nullopt.
 */
std::optional<uint> FindCompanyManagerFaceLabel(std::string_view label)
{
	auto it = std::ranges::find(_faces, label, &FaceSpec::label);
	if (it == std::end(_faces)) return std::nullopt;

	return static_cast<uint>(std::distance(std::begin(_faces), it));
}

/**
 * Get the face variables for a face style.
 * @param style_index Face style to get variables for.
 * @return Variables for the face style.
 */
FaceVars GetCompanyManagerFaceVars(uint style)
{
	const FaceSpec *spec = GetCompanyManagerFaceSpec(style);
	if (spec == nullptr) return {};
	return spec->GetFaceVars();
}

/**
 * Set a company face style.
 * Changes both the style index and the label.
 * @param cmf The CompanyManagerFace to change.
 * @param style The style to set.
 */
void SetCompanyManagerFaceStyle(CompanyManagerFace &cmf, uint style)
{
	openttd_rust_face_style(&GetRustFaceServices(), &cmf, style);
}

/**
 * Completely randomise a company manager face, including style.
 * @note randomizer should passed be appropriate for server-side or client-side usage.
 * @param cmf The CompanyManagerFace to randomise.
 * @param randomizer The randomizer to use.
 */
void RandomiseCompanyManagerFace(CompanyManagerFace &cmf, Randomizer &randomizer)
{
	openttd_rust_face_random(&GetRustFaceServices(), &cmf, &randomizer);
}

/**
 * Mask company manager face bits to ensure they are all within range.
 * @note Does not update the CompanyManagerFace itself. Unused bits are cleared.
 * @param cmf The CompanyManagerFace.
 * @param style The face variables.
 * @return The masked face bits.
 */
uint32_t MaskCompanyManagerFaceBits(const CompanyManagerFace &cmf, FaceVars vars)
{
	return openttd_rust_face_mask(&GetRustFaceServices(), cmf.bits, {vars.data(), vars.size()});
}

/**
 * Get a face code representation of a company manager face.
 * @param cmf The company manager face.
 * @return String containing face code.
 */
std::string FormatCompanyManagerFaceCode(const CompanyManagerFace &cmf)
{
	uint32_t masked_face_bits = MaskCompanyManagerFaceBits(cmf, GetCompanyManagerFaceVars(cmf.style));
	return fmt::format("{}:{}", cmf.style_label, masked_face_bits);
}

/**
 * Parse a face code into a company manager face.
 * @param str Face code to parse.
 * @return Company manager face, or std::nullopt if it could not be parsed.
 */
std::optional<CompanyManagerFace> ParseCompanyManagerFaceCode(std::string_view str)
{
	if (str.empty()) return std::nullopt;

	CompanyManagerFace cmf;
	StringConsumer consumer{str};
	if (consumer.FindChar(':') != StringConsumer::npos) {
		auto label = consumer.ReadUntilChar(':', StringConsumer::SKIP_ONE_SEPARATOR);

		/* Read numeric part and ensure it's valid. */
		auto bits = consumer.TryReadIntegerBase<uint32_t>(10, true);
		if (!bits.has_value() || consumer.AnyBytesLeft()) return std::nullopt;

		/* Ensure style label is valid. */
		auto style = FindCompanyManagerFaceLabel(label);
		if (!style.has_value()) return std::nullopt;

		SetCompanyManagerFaceStyle(cmf, *style);
		cmf.bits = *bits;
	} else {
		/* No ':' included, treat as numeric-only. This allows old-style codes to be entered. */
		auto bits = ParseInteger(str, 10, true);
		if (!bits.has_value()) return std::nullopt;

		/* Old codes use bits 0..1 to represent face style. These map directly to the default face styles. */
		SetCompanyManagerFaceStyle(cmf, GB(*bits, 0, 2));
		cmf.bits = *bits;
	}

	/* Force the face bits to be valid. */
	FaceVars vars = GetCompanyManagerFaceVars(cmf.style);
	ScaleAllCompanyManagerFaceBits(cmf, vars);
	cmf.bits = MaskCompanyManagerFaceBits(cmf, vars);

	return cmf;
}

/* Native startup and networking operations used by the Rust company owner. */
/** Prepare company infrastructure and return whether startup needs a random face. */
bool OPENTTD_RUST_CALL RustCompanyPrepare(uint8_t id, bool is_ai) noexcept
{
	Company *c = Company::Get(id);
	c->avail_railtypes = GetCompanyRailTypes(c->index);
	c->avail_roadtypes = GetCompanyRoadTypes(c->index);
	c->inaugurated_year = TimerGameEconomy::year;
	c->inaugurated_year_calendar = TimerGameCalendar::year;

	bool randomise_face = true;
	if (!_company_manager_face.empty() && !is_ai && !_networking) {
		auto cmf = ParseCompanyManagerFaceCode(_company_manager_face);
		if (cmf.has_value()) {
			randomise_face = false;
			c->face = std::move(*cmf);
		}
	}
	return randomise_face;
}
void OPENTTD_RUST_CALL RustCompanyDefaults(uint8_t id) noexcept
{
	Company *c = Company::Get(id);

	SetDefaultCompanySettings(c->index);
	ClearEnginesHiddenFlagOfCompany(c->index);
}
void OPENTTD_RUST_CALL RustCompanyStartupWindows() noexcept
{
	SetWindowDirty(WC_GRAPH_LEGEND, 0);
	InvalidateWindowData(WC_CLIENT_LIST, 0);
	InvalidateWindowData(WC_LINKGRAPH_LEGEND, 0);
	BuildOwnerLegend();
	InvalidateWindowData(WC_SMALLMAP, 0, 1);
}
/** CCA_NEW failure: the client reverts to spectating. */
void RustCompanyNetworkSpectate(uint32_t client) noexcept
{
	NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(ClientID(client));
	if (_network_server && ci != nullptr) {
		ci->client_playas = COMPANY_SPECTATOR;
		NetworkUpdateClientInfo(ci->client_id);
	}
}
void RustCompanyNetworkNew(uint8_t id, uint32_t client, bool with_client) noexcept
{
	Company *c = Company::Get(id);
	NetworkAdminCompanyNew(c);
	NetworkServerNewCompany(c, with_client ? NetworkClientInfo::GetByClientID(ClientID(client)) : nullptr);
}
/** CCA_NEW: the requesting client takes control of its new company. */
void RustCompanyNetworkOwn(uint8_t id, uint32_t client) noexcept
{
	if (ClientID(client) != _network_own_client_id) return;
	Company *c = Company::Get(id);
	assert(_local_company == COMPANY_SPECTATOR);
	SetLocalCompany(c->index);
	if (!_company_manager_face.empty()) {
		auto cmf = ParseCompanyManagerFaceCode(_company_manager_face);
		if (cmf.has_value()) Command<CMD_SET_COMPANY_MANAGER_FACE>::SendNet(STR_NULL, c->index, cmf->style, cmf->bits);
	}
	SyncCompanySettings();
	MarkWholeScreenDirty();
}

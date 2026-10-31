/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file misc_cmd.cpp Some misc functions that are better fitted in other files, but never got moved there... */

#include "stdafx.h"
#include "command_func.h"
#include "economy_func.h"
#include "window_func.h"
#include "textbuf_gui.h"
#include "network/network.h"
#include "network/network_func.h"
#include "strings_func.h"
#include "company_func.h"
#include "company_gui.h"
#include "company_base.h"
#include "tile_map.h"
#include "texteff.hpp"
#include "core/backup_type.hpp"
#include "misc_cmd.h"

#include "table/strings.h"

#include "rust/company_adapter.h"
#include "rust/session_ffi.h"

#include "safeguards.h"

/**
 * Increase the loan of your company.
 * @param flags operation to perform
 * @param cmd when LoanCommand::Interval: loans LOAN_INTERVAL,
 *            when LoanCommand::Max: loans the maximum loan permitting money (press CTRL),
 *            when LoanCommand::Amount: loans the amount specified in \c amount
 * @param amount amount to increase the loan with, multitude of LOAN_INTERVAL. Only used when cmd == LoanCommand::Amount.
 * @return the cost of this operation or an error
 */
CommandCost CmdIncreaseLoan(DoCommandFlags flags, LoanCommand cmd, Money amount)
{
	return RustCompanyCost(openttd_rust_company_increase_loan(&GetRustCompanyFinanceServices(), static_cast<uint8_t>(cmd), amount.base(), _settings_game.difficulty.infinite_money, flags.Test(DoCommandFlag::Execute)));
}

/**
 * Decrease the loan of your company.
 * @param flags operation to perform
 * @param cmd when LoanCommand::Interval: pays back LOAN_INTERVAL,
 *            when LoanCommand::Max: pays back the maximum loan permitting money (press CTRL),
 *            when LoanCommand::Amount: pays back the amount specified in \c amount
 * @param amount amount to decrease the loan with, multitude of LOAN_INTERVAL. Only used when cmd == LoanCommand::Amount.
 * @return the cost of this operation or an error
 */
CommandCost CmdDecreaseLoan(DoCommandFlags flags, LoanCommand cmd, Money amount)
{
	return RustCompanyCost(openttd_rust_company_decrease_loan(&GetRustCompanyFinanceServices(), static_cast<uint8_t>(cmd), amount.base(), _settings_game.difficulty.infinite_money, flags.Test(DoCommandFlag::Execute)));
}

/**
 * Sets the max loan amount of your company. Does not respect the global loan setting.
 * @param company the company ID.
 * @param amount the new max loan amount, will be rounded down to the multitude of LOAN_INTERVAL. If set to COMPANY_MAX_LOAN_DEFAULT reset the max loan to default(global) value.
 * @return zero cost or an error
 */
CommandCost CmdSetCompanyMaxLoan(DoCommandFlags flags, CompanyID company, Money amount)
{
	return RustCompanyCost(openttd_rust_company_set_max_loan(&GetRustCompanyFinanceServices(), company.base(), amount.base(), flags.Test(DoCommandFlag::Execute)));
}

/**
 * In case of an unsafe unpause, we want the
 * user to confirm that it might crash.
 * @param confirmed whether the user confirmed their action
 */
static void AskUnsafeUnpauseCallback(Window *, bool confirmed)
{
	if (confirmed) {
		Command<CMD_PAUSE>::Post(PauseMode::Error, false);
	}
}

/**
 * Pause/Unpause the game (server-only).
 * Set or unset a bit in the pause mode. If pause mode is zero the game is
 * unpaused. A bitset is used instead of a boolean value/counter to have
 * more control over the game when saving/loading, etc.
 * @param flags operation to perform
 * @param mode the pause mode to change
 * @param pause true pauses, false unpauses this mode
 * @return the cost of this operation or an error
 */
CommandCost CmdPause(DoCommandFlags flags, PauseMode mode, bool pause)
{
	return openttd_rust_session_pause(&GetRustSessionServices(), mode, pause, flags.Test(DoCommandFlag::Execute)) ? CommandCost() : CMD_ERROR;
}

/**
 * Change the financial flow of your company.
 * @param amount the amount of money to receive (if positive), or spend (if negative)
 * @return the cost of this operation or an error
 */
CommandCost CmdMoneyCheat(DoCommandFlags, Money amount)
{
	return CommandCost(EXPENSES_OTHER, -amount);
}

/**
 * Change the bank bank balance of a company by inserting or removing money without affecting the loan.
 * @param flags operation to perform
 * @param tile tile to show text effect on (if not 0)
 * @param delta the amount of money to receive (if positive), or spend (if negative)
 * @param company the company ID.
 * @param expenses_type the expenses type which should register the cost/income @see ExpensesType.
 * @return zero cost or an error
 */
CommandCost CmdChangeBankBalance(DoCommandFlags flags, TileIndex tile, Money delta, CompanyID company, ExpensesType expenses_type)
{
	return RustCompanyCost(openttd_rust_company_change_bank_balance(&GetRustCompanyFinanceServices(), tile.base(), delta.base(), company.base(), expenses_type, flags.Test(DoCommandFlag::Execute)));
}

void RustAskUnsafeUnpause() noexcept
{
	ShowQuery(GetEncodedString(STR_NEWGRF_UNPAUSE_WARNING_TITLE), GetEncodedString(STR_NEWGRF_UNPAUSE_WARNING), nullptr, AskUnsafeUnpauseCallback);
}

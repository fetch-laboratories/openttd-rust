/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command.cpp Handling of commands. */

#include "stdafx.h"
#include "landscape.h"
#include "error.h"
#include "gui.h"
#include "command_func.h"
#include "network/network_type.h"
#include "network/network.h"
#include "genworld.h"
#include "strings_func.h"
#include "texteff.hpp"
#include "town.h"
#include "timer/timer_game_economy.h"
#include "company_func.h"
#include "company_base.h"
#include "signal_func.h"
#include "core/backup_type.hpp"
#include "object_base.h"
#include "autoreplace_cmd.h"
#include "company_cmd.h"
#include "depot_cmd.h"
#include "economy_cmd.h"
#include "engine_cmd.h"
#include "goal_cmd.h"
#include "group_cmd.h"
#include "industry_cmd.h"
#include "league_cmd.h"
#include "landscape_cmd.h"
#include "misc_cmd.h"
#include "news_cmd.h"
#include "object_cmd.h"
#include "order_cmd.h"
#include "rail_cmd.h"
#include "road_cmd.h"
#include "roadveh_cmd.h"
#include "settings_cmd.h"
#include "signs_cmd.h"
#include "station_cmd.h"
#include "story_cmd.h"
#include "subsidy_cmd.h"
#include "terraform_cmd.h"
#include "timetable_cmd.h"
#include "town_cmd.h"
#include "train_cmd.h"
#include "tree_cmd.h"
#include "tunnelbridge_cmd.h"
#include "vehicle_cmd.h"
#include "viewport_cmd.h"
#include "water_cmd.h"
#include "waypoint_cmd.h"
#include "misc/endian_buffer.hpp"
#include "string_func.h"

#include "table/strings.h"

#include "rust/command_services_ffi.h"
#include "rust/command_cost_services.h"
#include "rust/company_adapter.h"
#include "rust/company_services.h"
#include "rust/environment_services.h"
#include "rust/landscape_command_ffi.h"
#include "rust/newgrf_storage_ffi.h"
#include "rust/services_assert.h"
#include "rust/services_ffi.h"
#include "rust/signal_ffi.h"
#include "rust/vehicle_ffi.h"

#include "safeguards.h"


/**
 * Define a command with the flags which belongs to it.
 *
 * This struct connects a command handler function with the flags created with
 * the #CommandFlag::Auto, #CommandFlag::Offline and #CommandFlag::Server values.
 */
struct CommandInfo {
	std::string_view name; ///< A human readable name for the procedure
	CommandFlags flags; ///< The (command) flags to that apply to this command
	CommandType type;   ///< The type of command.
};
/* Helpers to generate the master command table from the command traits. */
template <typename T>
inline constexpr CommandInfo CommandFromTrait() noexcept { return { T::name, T::flags, T::type }; };

template <typename T, T... i>
inline constexpr auto MakeCommandsFromTraits(std::integer_sequence<T, i...>) noexcept {
	return std::array<CommandInfo, sizeof...(i)>{{ CommandFromTrait<CommandTraits<static_cast<Commands>(i)>>()... }};
}

/**
 * The master command table
 *
 * This table contains all possible CommandProc functions with
 * the flags which belongs to it. The indices are the same
 * as the value from the CMD_* enums.
 */
static constexpr auto _command_proc_table = MakeCommandsFromTraits(std::make_integer_sequence<std::underlying_type_t<Commands>, CMD_END>{});


/**
 * This function range-checks a cmd.
 *
 * @param cmd The integer value of a command
 * @return true if the command is valid (and got a CommandProc function)
 */
bool IsValidCommand(Commands cmd)
{
	return cmd < _command_proc_table.size();
}

/**
 * This function mask the parameter with CMD_ID_MASK and returns
 * the flags which belongs to the given command.
 *
 * @param cmd The integer value of the command
 * @return The flags for this command
 */
CommandFlags GetCommandFlags(Commands cmd)
{
	assert(IsValidCommand(cmd));

	return _command_proc_table[cmd].flags;
}

/**
 * This function mask the parameter with CMD_ID_MASK and returns
 * the name which belongs to the given command.
 *
 * @param cmd The integer value of the command
 * @return The name for this command
 */
std::string_view GetCommandName(Commands cmd)
{
	assert(IsValidCommand(cmd));

	return _command_proc_table[cmd].name;
}

/**
 * Returns whether the command is allowed while the game is paused.
 * @param cmd The command to check.
 * @return True if the command is allowed while paused, false otherwise.
 */
bool IsCommandAllowedWhilePaused(Commands cmd)
{
	assert(IsValidCommand(cmd));
	return openttd_rust_command_allowed_while_paused(&GetRustCommandServices(), to_underlying(_command_proc_table[cmd].type));
}

/** Helper to make a desync log for a command. */
void CommandHelperBase::LogCommandExecution(Commands cmd, StringID err_message, const CommandDataBuffer &args, bool failed)
{
	Debug(desync, 1, "{}: {:08x}; {:02x}; {:02x}; {:08x}; {:08x}; {} ({})", failed ? "cmdf" : "cmd", (uint32_t)TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, _current_company, cmd, err_message, FormatArrayAsHex(args), GetCommandName(cmd));
}

/** `Backup<CompanyID>` destruction while a command procedure's exception unwinds `Execute`. */
void CommandHelperBase::AbandonExecution(std::exception_ptr exception, std::source_location location)
{
	Debug(misc, 0, "{}:{}: Backed-up value was not restored!", location.file_name(), location.line());
	std::rethrow_exception(exception);
}

/* Lifecycle leaves: settings, input and presentation only. None reenters a command. */
static bool CommandInfiniteMoney() noexcept { return _settings_game.difficulty.infinite_money; }
static void CommandMoneyError(OpenTTDCommandCost *cost, int64_t amount) noexcept
{
	CommandCost &native = *reinterpret_cast<CommandCost *>(cost);
	native.MakeError(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY);
	if (IsLocalCompany()) native.SetEncodedMessage(GetEncodedString(STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY, Money(amount)));
}
static bool CommandShiftPressed() noexcept { return _shift_pressed; }
static uint8_t CommandPauseLevel() noexcept { return to_underlying(_settings_game.construction.command_pause_level); }
static void CommandLastBuild(uint8_t company, uint32_t tile) noexcept
{
	Company *c = Company::GetIfValid(CompanyID(company));
	if (c != nullptr) c->last_build_coordinate = TileIndex{tile};
}
static void CommandPausedError(uint32_t err_message, int32_t x, int32_t y) noexcept
{
	ShowErrorMessage(GetEncodedString(err_message), GetEncodedString(STR_ERROR_NOT_ALLOWED_WHILE_PAUSED), WL_INFO, x, y);
}
static void CommandErrorMessage(uint32_t err_message, int32_t x, int32_t y, const OpenTTDCommandCost *res) noexcept
{
	ShowErrorMessage(GetEncodedString(err_message), x, y, *const_cast<CommandCost *>(reinterpret_cast<const CommandCost *>(res)));
}
static void CommandEstimate(int64_t cost, int32_t x, int32_t y) noexcept { ShowEstimatedCostOrIncome(cost, x, y); }
static void CommandCostAnimation(int32_t x, int32_t y, int64_t cost) noexcept { ShowCostOrIncomeAnimation(x, y, GetSlopePixelZ(x, y), cost); }

const OpenTTDCommandServices &GetRustCommandServices() noexcept
{
	static const OpenTTDCommandServices services{
		.finance = &GetRustCompanyFinanceServices(),
		.infinite_money = CommandInfiniteMoney, .money_error = CommandMoneyError,
		.infrastructure_dirty = RustInfrastructureDirty,
		.landscape_commands = []() noexcept { return &GetRustLandscapeCommandServices(); },
		.company = []() noexcept { return &GetRustCompanyServices(); },
		.occupancy = GetRustOccupancyCommands,
		.shared = &GetRustSharedServices(),
		.cost = &GetRustCommandCostServices(),
		.rail_map = &GetRustRailMapServices(),
		.signals = &GetRustSignalServices(),
		.assert_condition = RustAssertCondition,
		.storage_discard = RustStorageDiscard,
		.shift_pressed = CommandShiftPressed,
		.pause_level = CommandPauseLevel,
		.desync_level = RustDesyncLevel,
		.last_build_coordinate = CommandLastBuild,
		.not_enough_cash = STR_ERROR_NOT_ENOUGH_CASH_REQUIRES_CURRENCY,
		.paused_error = CommandPausedError,
		.error_message = CommandErrorMessage,
		.estimate = CommandEstimate,
		.cost_animation = CommandCostAnimation,
	};
	return services;
}

/**
 * Adds the cost of the given command return value to this cost.
 * Also takes a possible error message when it is set.
 * @param ret The command to add the cost of.
 */
void CommandCost::AddCost(CommandCost &&ret)
{
	this->AddCost(ret.cost);
	if (this->success && !ret.success) {
		this->message = ret.message;
		this->encoded_message = std::move(ret.encoded_message);
		this->success = false;
	}
}

/**
 * Return an error status, with string and parameter.
 * @param str StringID of error.
 * @param value Single parameter for error.
 * @returns CommandCost representing the error.
 */
CommandCost CommandCostWithParam(StringID str, uint64_t value)
{
	CommandCost error = CommandCost(str);
	if (IsLocalCompany()) {
		error.SetEncodedMessage(GetEncodedString(str, value));
	}
	return error;
}

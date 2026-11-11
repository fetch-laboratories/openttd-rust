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
#include "rust/command_post_ffi.h"
#include "rust/company_adapter.h"
#include "rust/tile_owner_services.h"
#include "rust/session_ffi.h"
#include "network/core/tcp_game.h"

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

uint8_t GetCommandType(Commands cmd) noexcept
{
	assert(IsValidCommand(cmd));
	return to_underlying(_command_proc_table[cmd].type);
}

/** Helper to make a desync log for a command. */
void CommandHelperBase::LogCommandExecution(Commands cmd, StringID err_message, std::span<const uint8_t> args, bool failed)
{
	Debug(desync, 1, "{}: {:08x}; {:02x}; {:02x}; {:08x}; {:08x}; {} ({})", failed ? "cmdf" : "cmd", (uint32_t)TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, _current_company, cmd, err_message, FormatArrayAsHex(args), GetCommandName(cmd));
}

/** Whether a desync log line would be printed. */
bool CommandHelperBase::DesyncLogging()
{
	return _debug_desync_level >= 1;
}

/** `Backup<CompanyID>` destruction: unrestored only while a procedure's exception unwinds `Execute`. */
CommandExecutionScope::~CommandExecutionScope() noexcept
{
	if (openttd_rust_command_backup_held() != 0) {
		Debug(misc, 0, "{}:{}: Backed-up value was not restored!", this->location.file_name(), this->location.line());
		openttd_rust_command_restore(&GetRustCommandServices());
	}
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

static OpenTTDCommandData TakeCommandArguments(CommandData &&data) noexcept
{
	return std::exchange(static_cast<OpenTTDCommandData &>(data), {});
}

static_assert(sizeof(RustTileOwnerContext) <= sizeof(OpenTTDCommandControlStorage));
static_assert(alignof(RustTileOwnerContext) <= alignof(OpenTTDCommandControlStorage));
static_assert(CMD_PAUSE == 71 && CMD_SET_COMPANY_COLOUR == 54 && CMD_COMPANY_CTRL == 90);
static_assert(CCA_NEW == 0 && CCA_DELETE == 2 && CRR_NONE == 0 && CRR_BANKRUPT == 2);
const OpenTTDCommandPostServices &GetRustCommandPostServices() noexcept
{
	static const OpenTTDCommandPostServices services{
		.assign_company_cost = [](OpenTTDCommandCost *out, const OpenTTDCompanyCost *value) noexcept { *reinterpret_cast<CommandCost *>(out) = RustCompanyCost(*value); },
		.command = GetRustCommandServices,
		.control_begin = [](OpenTTDCommandControlStorage *storage) noexcept -> const OpenTTDTileOwnerContext * { return &(new (storage) RustTileOwnerContext())->view; },
		.control_end = [](OpenTTDCommandControlStorage *storage) noexcept { reinterpret_cast<RustTileOwnerContext *>(storage)->~RustTileOwnerContext(); },
		.desync_logging = []() noexcept { return CommandHelperBase::DesyncLogging(); },
		.log = [](uint8_t cmd, OpenTTDByteSpan args, bool failed) noexcept { CommandHelperBase::LogCommandExecution(static_cast<Commands>(cmd), 0, std::span(args.data, args.size), failed); },
		.network = GetRustNetworkCommandServices,
		.pack_colour = [](bool primary, uint8_t colour) noexcept {
			CommandTraits<CMD_SET_COMPANY_COLOUR>::Args args{LS_DEFAULT, primary, static_cast<Colours>(colour)};
			return TakeCommandArguments(EndianBufferWriter<CommandData>::FromValue<decltype(args), CommandData>(args));
		},
		.pack_control = [](uint8_t action, uint8_t target, uint8_t reason, uint32_t client) noexcept {
			CommandTraits<CMD_COMPANY_CTRL>::Args args{static_cast<CompanyCtrlAction>(action), CompanyID(target), static_cast<CompanyRemoveReason>(reason), static_cast<ClientID>(client)};
			return TakeCommandArguments(EndianBufferWriter<CommandData>::FromValue<decltype(args), CommandData>(args));
		},
		.pack_pause = [](PauseMode mode, bool pause) noexcept {
			CommandTraits<CMD_PAUSE>::Args args{mode, pause};
			return TakeCommandArguments(EndianBufferWriter<CommandData>::FromValue<decltype(args), CommandData>(args));
		},
		.session = GetRustSessionServices,
	};
	return services;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCommandPostServices.hpp"
bool CheckAbi_command_post() { return AbiServicesComplete(GetRustCommandPostServices()); }

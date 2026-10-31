/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_command.cpp Command handling over network connections. */

#include "../stdafx.h"
#include "network_admin.h"
#include "network_client.h"
#include "network_server.h"
#include "../command_func.h"
#include "../company_func.h"
#include "../settings_type.h"
#include "../airport_cmd.h"
#include "../aircraft_cmd.h"
#include "../autoreplace_cmd.h"
#include "../company_cmd.h"
#include "../depot_cmd.h"
#include "../dock_cmd.h"
#include "../economy_cmd.h"
#include "../engine_cmd.h"
#include "../error_func.h"
#include "../goal_cmd.h"
#include "../group_cmd.h"
#include "../industry_cmd.h"
#include "../landscape_cmd.h"
#include "../league_cmd.h"
#include "../misc_cmd.h"
#include "../news_cmd.h"
#include "../object_cmd.h"
#include "../order_cmd.h"
#include "../rail_cmd.h"
#include "../road_cmd.h"
#include "../roadveh_cmd.h"
#include "../settings_cmd.h"
#include "../signs_cmd.h"
#include "../station_cmd.h"
#include "../story_cmd.h"
#include "../subsidy_cmd.h"
#include "../terraform_cmd.h"
#include "../timetable_cmd.h"
#include "../town_cmd.h"
#include "../train_cmd.h"
#include "../tree_cmd.h"
#include "../tunnelbridge_cmd.h"
#include "../vehicle_cmd.h"
#include "../viewport_cmd.h"
#include "../water_cmd.h"
#include "../waypoint_cmd.h"
#include "../script/script_cmd.h"
#include "../rust/company_adapter.h"
#include "../rust/settings_services.h"

#include "../safeguards.h"

/** Typed list of all possible callbacks. */
static constexpr auto _callback_tuple = std::make_tuple(
	(CommandCallback *)nullptr, // Make sure this is actually a pointer-to-function.
	&CcBuildPrimaryVehicle,
	&CcBuildAirport,
	&CcBuildBridge,
	&CcPlaySound_CONSTRUCTION_WATER,
	&CcBuildDocks,
	&CcFoundTown,
	&CcBuildRoadTunnel,
	&CcBuildRailTunnel,
	&CcBuildWagon,
	&CcRoadDepot,
	&CcRailDepot,
	&CcPlaceSign,
	&CcPlaySound_EXPLOSION,
	&CcPlaySound_CONSTRUCTION_OTHER,
	&CcPlaySound_CONSTRUCTION_RAIL,
	&CcStation,
	&CcTerraform,
	&CcAI,
	&CcCloneVehicle,
	&CcCreateGroup,
	&CcFoundRandomTown,
	&CcRoadStop,
	&CcStartStopVehicle,
	&CcGame,
	&CcAddVehicleNewGroup,
	&CcMoveStationName,
	&CcMoveWaypointName
);

#ifdef SILENCE_GCC_FUNCTION_POINTER_CAST
/*
 * We cast specialized function pointers to a generic one, but don't use the
 * converted value to call the function, which is safe, except that GCC
 * helpfully thinks it is not.
 *
 * "Any pointer to function can be converted to a pointer to a different function type.
 * Calling the function through a pointer to a different function type is undefined,
 * but converting such pointer back to pointer to the original function type yields
 * the pointer to the original function." */
#	pragma GCC diagnostic push
#	pragma GCC diagnostic ignored "-Wcast-function-type"
#endif

/* Helpers to generate the callback table from the callback list. */

inline constexpr size_t _callback_tuple_size = std::tuple_size_v<decltype(_callback_tuple)>;

template <size_t... i>
inline auto MakeCallbackTable(std::index_sequence<i...>) noexcept
{
	return std::array<CommandCallback *, sizeof...(i)>{{ reinterpret_cast<CommandCallback *>(reinterpret_cast<void(*)()>(std::get<i>(_callback_tuple)))... }}; // MingW64 fails linking when casting a pointer to its own type. To work around, cast it to some other type first.
}

/** Type-erased table of callbacks. */
static const auto _callback_table = MakeCallbackTable(std::make_index_sequence<_callback_tuple_size>{});

/**
 * Helper function to ensure that callbacks used when Posting commands are actually registered for the network calls.
 * @param callback The callback to check.
 * @return \c true when the callback is in the callback table, otherwise \c false.
 */
bool IsNetworkRegisteredCallback(CommandCallback *callback)
{
	return std::ranges::find(_callback_table, callback) != _callback_table.end();
}

template <typename T> struct CallbackArgsHelper;
template <typename... Targs>
struct CallbackArgsHelper<void(*const)(Commands, const CommandCost &, Targs...)> {
	using Args = std::tuple<std::decay_t<Targs>...>;
};


/* Helpers to generate the command dispatch table from the command traits. */

template <Commands Tcmd> static CommandData SanitizeCmdStrings(std::span<const uint8_t> data);
template <Commands Tcmd, size_t cb> static void UnpackNetworkCommand(const OpenTTDCommandPacket &cp);
template <Commands Tcmd> static void NetworkReplaceCommandClientId(CommandPacket &cp, ClientID client_id);
using UnpackNetworkCommandProc = void (*)(const OpenTTDCommandPacket &);
using UnpackDispatchT = std::array<UnpackNetworkCommandProc, _callback_tuple_size>;
struct CommandDispatch {
	CommandData(*Sanitize)(std::span<const uint8_t>);
	void (*ReplaceClientId)(CommandPacket &, ClientID);
	UnpackDispatchT Unpack;
};

template <Commands Tcmd, size_t Tcb>
constexpr UnpackNetworkCommandProc MakeUnpackNetworkCommandCallback() noexcept
{
	/* Check if the callback matches with the command arguments. If not, don't generate an Unpack proc. */
	using Tcallback = std::tuple_element_t<Tcb, decltype(_callback_tuple)>;
	if constexpr (std::is_same_v<Tcallback, CommandCallback * const> || // Callback type is CommandCallback.
			std::is_same_v<Tcallback, CommandCallbackData * const> || // Callback type is CommandCallbackData.
			std::is_same_v<typename CommandTraits<Tcmd>::CbArgs, typename CallbackArgsHelper<Tcallback>::Args> || // Callback proc takes all command return values and parameters.
			(!std::is_void_v<typename CommandTraits<Tcmd>::RetTypes> && std::is_same_v<typename CallbackArgsHelper<typename CommandTraits<Tcmd>::RetCallbackProc const>::Args, typename CallbackArgsHelper<Tcallback>::Args>)) { // Callback return is more than CommandCost and the proc takes all return values.
		return &UnpackNetworkCommand<Tcmd, Tcb>;
	} else {
		return nullptr;
	}
}

template <Commands Tcmd, size_t... i>
constexpr UnpackDispatchT MakeUnpackNetworkCommand(std::index_sequence<i...>) noexcept
{
	return UnpackDispatchT{{ MakeUnpackNetworkCommandCallback<Tcmd, i>()...}};
}

template <typename T, T... i, size_t... j>
inline constexpr auto MakeDispatchTable(std::integer_sequence<T, i...>, std::index_sequence<j...>) noexcept
{
	return std::array<CommandDispatch, sizeof...(i)>{{ { &SanitizeCmdStrings<static_cast<Commands>(i)>, &NetworkReplaceCommandClientId<static_cast<Commands>(i)>, MakeUnpackNetworkCommand<static_cast<Commands>(i)>(std::make_index_sequence<_callback_tuple_size>{}) }... }};
}
/** Command dispatch table. */
static constexpr auto _cmd_dispatch = MakeDispatchTable(std::make_integer_sequence<std::underlying_type_t<Commands>, CMD_END>{}, std::make_index_sequence<_callback_tuple_size>{});

#ifdef SILENCE_GCC_FUNCTION_POINTER_CAST
#	pragma GCC diagnostic pop
#endif

/**
 * Find the callback index of a callback pointer.
 * @param callback Address of callback to search for.
 * @return Callback index or std::numeric_limits<size_t>::max() if the function wasn't found in the callback list.
 */
static size_t FindCallbackIndex(CommandCallback *callback)
{
	if (auto it = std::ranges::find(_callback_table, callback); it != std::end(_callback_table)) {
		return static_cast<size_t>(std::distance(std::begin(_callback_table), it));
	}

	return std::numeric_limits<size_t>::max();
}

static_assert(NetworkClientSocket::STATUS_MAP == 6); // network_command.rs STATUS_MAP

const OpenTTDNetworkCommandServices &GetRustNetworkCommandServices() noexcept
{
	static const OpenTTDNetworkCommandServices services{
		.company = &GetRustCompanyServices(),
		.admin_log = [](const void *owner, const OpenTTDCommandPacket *cp) noexcept { NetworkAdminCmdLogging(static_cast<const NetworkClientSocket *>(owner), *cp); },
		.allowed_while_paused = [](uint8_t cmd) noexcept { return IsCommandAllowedWhilePaused(static_cast<Commands>(cmd)); },
		.client_queue = []() noexcept -> OpenTTDCommandQueue * { return &ClientNetworkGameSocketHandler::my_client->incoming_queue; },
		.client_send = [](const OpenTTDCommandPacket *cp) noexcept { MyClient::SendCommand(*cp); },
		.commands_per_frame = RustCommandsPerFrame,
		.commands_per_frame_server = RustCommandsPerFrameServer,
		.execute = [](const OpenTTDCommandPacket *cp) noexcept {
			size_t cb_index = FindCallbackIndex(reinterpret_cast<CommandCallback *>(const_cast<void *>(cp->callback)));
			assert(cb_index < _callback_tuple_size);
			assert(_cmd_dispatch[cp->cmd].Unpack[cb_index] != nullptr);
			_cmd_dispatch[cp->cmd].Unpack[cb_index](*cp);
		},
		.execute_past = []() noexcept {
			/* If we reach here, it means for whatever reason, we've already executed
			 * past the command we need to execute. */
			FatalError("[net] Trying to execute a packet in the past!");
		},
		.frame_counter = []() noexcept { return _frame_counter; },
		.frame_counter_max = []() noexcept { return _frame_counter_max; },
		.next_client = [](size_t from, OpenTTDNetworkCommandClient *client) noexcept {
			for (NetworkClientSocket *cs : NetworkClientSocket::Iterate(from)) {
				*client = {cs, cs->index.base(), &cs->incoming_queue, &cs->outgoing_queue, static_cast<uint8_t>(cs->status)};
				return true;
			}
			return false;
		},
		.server_send = [](void *cs, const OpenTTDCommandPacket *cp) noexcept { static_cast<NetworkClientSocket *>(cs)->SendCommand(*cp); },
	};
	return services;
}

/**
 * Prepare a DoCommand to be send over the network
 * @param cmd The command to execute (a CMD_* value)
 * @param err_message Message prefix to show on error
 * @param callback A callback function to call after the command is finished
 * @param company The company that wants to send the command
 * @param cmd_data The command proc arguments.
 */
void NetworkSendCommand(Commands cmd, StringID err_message, CommandCallback *callback, CompanyID company, const CommandDataBuffer &cmd_data)
{
	openttd_rust_network_send_command(&GetRustNetworkCommandServices(), cmd, err_message, reinterpret_cast<const void *>(callback), company.base(), {cmd_data.data(), cmd_data.size()});
}

/**
 * Sync our local command queue to the command queue of the given socket.
 * @param cs The client to sync the queue to.
 */
void NetworkSyncCommandQueue(NetworkClientSocket *cs)
{
	openttd_rust_network_sync_command_queue(&cs->outgoing_queue);
}

/**
 * Execute all commands on the local command queue that ought to be executed this frame.
 */
void NetworkExecuteLocalCommandQueue()
{
	assert(IsLocalCompany());
	openttd_rust_network_execute_local_command_queue(&GetRustNetworkCommandServices());
}

/**
 * Free the local command queues.
 */
void NetworkFreeLocalCommandQueue()
{
	openttd_rust_network_free_local_command_queue();
}

/** Distribute the commands of ourself and the clients. */
void NetworkDistributeCommands()
{
	openttd_rust_network_distribute_commands(&GetRustNetworkCommandServices());
}

/**
 * Send the outgoing command queue of a client.
 * @param cs The client to send the commands to.
 */
void NetworkHandleCommandQueue(NetworkClientSocket *cs)
{
	openttd_rust_network_handle_command_queue(&GetRustNetworkCommandServices(), cs, &cs->outgoing_queue);
}

/**
 * Receives a command from the network.
 * @param p the packet to read from.
 * @param cp the struct to write the data to.
 * @return An error message, or std::nullopt there has been no error.
 */
std::optional<std::string_view> NetworkGameSocketHandler::ReceiveCommand(Packet &p, CommandPacket &cp)
{
	cp.company = p.Recv_uint8();
	/* Commands has an 8-bit underlying type; the received value narrows before validation. */
	cp.cmd     = static_cast<uint8_t>(p.Recv_uint16());
	if (!IsValidCommand(cp.Cmd())) return "invalid command";
	if (GetCommandFlags(cp.Cmd()).Test(CommandFlag::Offline)) return "single-player only command";
	cp.err_msg = p.Recv_uint16();
	cp.SetData(_cmd_dispatch[cp.cmd].Sanitize(p.Recv_buffer()));

	uint8_t callback = p.Recv_uint8();
	if (callback >= _callback_table.size() || _cmd_dispatch[cp.cmd].Unpack[callback] == nullptr) return "invalid callback";

	cp.callback = reinterpret_cast<const void *>(_callback_table[callback]);
	return std::nullopt;
}

/**
 * Sends a command over the network.
 * @param p the packet to send it in.
 * @param cp the packet to actually send.
 */
void NetworkGameSocketHandler::SendCommand(Packet &p, const OpenTTDCommandPacket &cp)
{
	p.Send_uint8(cp.company);
	p.Send_uint16(cp.cmd);
	p.Send_uint16(cp.err_msg);
	p.Send_buffer(CommandPacketData(cp));

	size_t callback = FindCallbackIndex(reinterpret_cast<CommandCallback *>(const_cast<void *>(cp.callback)));
	if (callback > UINT8_MAX || _cmd_dispatch[cp.cmd].Unpack[callback] == nullptr) {
		Debug(net, 0, "Unknown callback for command; no callback sent (command: {})", cp.cmd);
		callback = 0; // _callback_table[0] == nullptr
	}
	p.Send_uint8 ((uint8_t)callback);
}

/** Helper to process a single ClientID argument. */
template <class T>
static inline void SetClientIdHelper(T &data, [[maybe_unused]] ClientID client_id)
{
	if constexpr (std::is_same_v<ClientID, T>) {
		data = client_id;
	}
}

/** Set all invalid ClientID's to the proper value. */
template <class Ttuple, size_t... Tindices>
static inline void SetClientIds(Ttuple &values, ClientID client_id, std::index_sequence<Tindices...>)
{
	((SetClientIdHelper(std::get<Tindices>(values), client_id)), ...);
}

template <Commands Tcmd>
static void NetworkReplaceCommandClientId(CommandPacket &cp, ClientID client_id)
{
	/* Unpack command parameters. */
	auto params = EndianBufferReader::ToValue<typename CommandTraits<Tcmd>::Args>(cp.Data());

	/* Insert client id. */
	SetClientIds(params, client_id, std::make_index_sequence<std::tuple_size_v<decltype(params)>>{});

	/* Repack command parameters. */
	cp.SetData(EndianBufferWriter<CommandData>::FromValue<decltype(params), CommandData>(params));
}

/**
 * Insert a client ID into the command data in a command packet.
 * @param cp Command packet to modify.
 * @param client_id Client id to insert.
 */
void NetworkReplaceCommandClientId(CommandPacket &cp, ClientID client_id)
{
	_cmd_dispatch[cp.cmd].ReplaceClientId(cp, client_id);
}


/** Validate a single string argument coming from network. */
template <class T>
static inline void SanitizeSingleStringHelper([[maybe_unused]] CommandFlags cmd_flags, T &data)
{
	if constexpr (std::is_same_v<std::string, T>) {
		if (!_network_server && cmd_flags.Test(CommandFlag::StrCtrl)) {
			StrMakeValidInPlace(data, {StringValidationSetting::AllowControlCode, StringValidationSetting::ReplaceWithQuestionMark});
		} else {
			StrMakeValidInPlace(data, {StringValidationSetting::ReplaceWithQuestionMark});
		}
	}
}

/** Helper function to perform validation on command data strings. */
template <class Ttuple, size_t... Tindices>
static inline void SanitizeStringsHelper(CommandFlags cmd_flags, Ttuple &values, std::index_sequence<Tindices...>)
{
	((SanitizeSingleStringHelper(cmd_flags, std::get<Tindices>(values))), ...);
}

/**
 * Validate and sanitize strings in command data.
 * @tparam Tcmd Command this data belongs to.
 * @param data Command data.
 * @return Sanitized command data.
 */
template <Commands Tcmd>
CommandData SanitizeCmdStrings(std::span<const uint8_t> data)
{
	auto args = EndianBufferReader::ToValue<typename CommandTraits<Tcmd>::Args>(data);
	SanitizeStringsHelper(CommandTraits<Tcmd>::flags, args, std::make_index_sequence<std::tuple_size_v<typename CommandTraits<Tcmd>::Args>>{});
	return EndianBufferWriter<CommandData>::FromValue<decltype(args), CommandData>(args);
}

/**
 * Unpack a generic command packet into its actual typed components.
 * @tparam Tcmd Command type to be unpacked.
 * @tparam Tcb Index into the callback list.
 * @param cp Command packet to unpack.
 */
template <Commands Tcmd, size_t Tcb>
void UnpackNetworkCommand(const OpenTTDCommandPacket &cp)
{
	auto args = EndianBufferReader::ToValue<typename CommandTraits<Tcmd>::Args>(CommandPacketData(cp));
	Command<Tcmd>::PostFromNet(cp.err_msg, std::get<Tcb>(_callback_tuple), cp.my_cmd, args);
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkCommandServices.hpp"
bool CheckAbi_network_command()
{
	return AbiServicesComplete(GetRustNetworkCommandServices());
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tcp_game.cpp Basic functions to receive and send TCP packets for game purposes.
 */

#include "../../stdafx.h"

#include "../network.h"
#include "../network_internal.h"
#include "../network_client.h"
#include "../network_query.h"
#include "../network_server.h"
#include "../../debug.h"
#include "../../error.h"
#include "../../strings_func.h"
#include "../../rust/network_dispatch_ffi.h"
#include "../../rust/session_native.hpp"

#include "table/strings.h"

#include "../../safeguards.h"

void RustDeleteNetworkSocket(void *game) noexcept
{
	delete static_cast<NetworkGameSocketHandler *>(game);
}

static struct DeferredNetworkSockets {
	~DeferredNetworkSockets() { openttd_rust_network_deferred_shutdown(RustDeleteNetworkSocket); }
} _deferred_network_sockets;

/**
 * Create a new socket for the game connection.
 * @param s The socket to connect with.
 */
NetworkGameSocketHandler::NetworkGameSocketHandler(SOCKET s, uint8_t role) : NetworkTCPSocketHandler(s),
		OpenTTDNetworkSocketSync{.receive_limit = 0, .last_frame = _frame_counter, .last_frame_server = _frame_counter, .last_token_frame = 0, .lag_test = 0, .last_token = 0, .token = 0},
		OpenTTDNetworkGameSocket{.base = this, .game = this, .incoming = &this->incoming_queue, .last_packet_ns = std::chrono::steady_clock::now().time_since_epoch().count(), .outgoing = nullptr, .pending = false, .role = role, .server = nullptr, .status = 0, .sync = this, .tcp = this} {}
static_assert(std::is_same_v<std::chrono::steady_clock::duration, std::chrono::nanoseconds>);

/**
 * Functions to help ReceivePacket/SendPacket a bit
 *  A socket can make errors. When that happens this handles what to do.
 * For clients: close connection and drop back to main-menu
 * For servers: close connection and that is it
 * @return the new status
 */
NetworkRecvStatus NetworkGameSocketHandler::CloseConnection(bool)
{
	OpenTTDSessionFrame frame;
	auto result = openttd_rust_network_game_close(&GetRustNetworkFrameServices(), this, &frame.network_actions);
	frame.Rethrow();
	return static_cast<NetworkRecvStatus>(result);
}

NetworkRecvStatus NetworkGameSocketHandler::ReceivePackets()
{
	OpenTTDSessionFrame frame;
	openttd_rust_network_receive_packets(&GetRustSessionServices(), &GetRustNetworkFrameServices(), this, &frame, &frame.network_actions);
	frame.Rethrow();
	return static_cast<NetworkRecvStatus>(openttd_rust_network_actions_result(&frame.network_actions));
}

/* static */ void NetworkGameSocketHandler::ProcessDeferredDeletions()
{
	openttd_rust_network_process_deferred(&GetRustNetworkFrameServices());
}

template <typename T, NetworkRecvStatus (T::*Handler)(Packet &)>
static uint8_t ReceiveGamePacket(OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept
{
	NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
	SessionException(frame, [&] { result = (static_cast<T *>(static_cast<NetworkGameSocketHandler *>(socket->game))->*Handler)(*packet); });
	return result;
}

const OpenTTDNetworkDispatchServices &GetRustNetworkDispatchServices() noexcept
{
	static const OpenTTDNetworkDispatchServices services{
		.client_chat = ReceiveGamePacket<ServerNetworkGameSocketHandler, &ServerNetworkGameSocketHandler::Receive_CLIENT_CHAT>,
		.client_id = [](OpenTTDNetworkGameSocket *socket) noexcept -> uint32_t { return static_cast<NetworkGameSocketHandler *>(socket->game)->client_id; },
		.client_move = ReceiveGamePacket<ServerNetworkGameSocketHandler, &ServerNetworkGameSocketHandler::Receive_CLIENT_MOVE>,
		.client_rcon = ReceiveGamePacket<ServerNetworkGameSocketHandler, &ServerNetworkGameSocketHandler::Receive_CLIENT_RCON>,
		.client_set_name = ReceiveGamePacket<ServerNetworkGameSocketHandler, &ServerNetworkGameSocketHandler::Receive_CLIENT_SET_NAME>,
		.map_load_args = [](OpenTTDSessionFrame *frame) noexcept -> OpenTTDSessionLoad * { return &frame->packet_load.emplace().args; },
		.packet_end = [](OpenTTDSessionFrame *frame) noexcept { frame->packet_load.reset(); frame->packet_text.reset(); },
		.server_banned = ReceiveGamePacket<QueryNetworkGameSocketHandler, &QueryNetworkGameSocketHandler::Receive_SERVER_BANNED>,
		.server_chat = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_CHAT>,
		.server_client_info = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_CLIENT_INFO>,
		.server_config_update = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_CONFIG_UPDATE>,
		.server_error = ReceiveGamePacket<QueryNetworkGameSocketHandler, &QueryNetworkGameSocketHandler::Receive_SERVER_ERROR>,
		.server_external_chat = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_EXTERNAL_CHAT>,
		.server_full = ReceiveGamePacket<QueryNetworkGameSocketHandler, &QueryNetworkGameSocketHandler::Receive_SERVER_FULL>,
		.server_game_info = ReceiveGamePacket<QueryNetworkGameSocketHandler, &QueryNetworkGameSocketHandler::Receive_SERVER_GAME_INFO>,
		.server_move = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_MOVE>,
		.server_rcon = ReceiveGamePacket<ClientNetworkGameSocketHandler, &ClientNetworkGameSocketHandler::Receive_SERVER_RCON>,
		.text_begin = [](OpenTTDSessionFrame *frame) noexcept -> void * { return &frame->packet_text.emplace(); },
	};
	return services;
}

#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkDispatchServices.hpp"
bool CheckAbi_network_dispatch() { return AbiServicesComplete(GetRustNetworkDispatchServices()); }

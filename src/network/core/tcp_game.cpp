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

/**
 * Helper for logging receiving invalid packets.
 * @param type The received packet type.
 * @return The status the network should have, in this case: "malformed packet error".
 */
NetworkRecvStatus NetworkGameSocketHandler::ReceiveInvalidPacket(PacketGameType type)
{
	Debug(net, 0, "[tcp/game] Received illegal packet type {} from client {}", type, this->client_id);
	return NETWORK_RECV_STATUS_MALFORMED_PACKET;
}

NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_FULL(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_FULL); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_BANNED(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_BANNED); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_JOIN(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_JOIN); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_ERROR(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_ERROR); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_GAME_INFO(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_GAME_INFO); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_GAME_INFO(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_GAME_INFO); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_CLIENT_INFO(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_CLIENT_INFO); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_IDENTIFY(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_IDENTIFY); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_AUTH_REQUEST(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_AUTH_REQUEST); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_AUTH_RESPONSE(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_AUTH_RESPONSE); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_ENABLE_ENCRYPTION(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_ENABLE_ENCRYPTION); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_WELCOME(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_WELCOME); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_GETMAP(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_GETMAP); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_WAIT(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_WAIT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_MAP_BEGIN(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_MAP_BEGIN); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_MAP_SIZE(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_MAP_SIZE); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_MAP_DATA(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_MAP_DATA); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_MAP_DONE(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_MAP_DONE); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_MAP_OK(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_MAP_OK); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_JOIN(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_JOIN); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_FRAME(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_FRAME); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_SYNC(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_SYNC); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_ACK(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_ACK); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_COMMAND(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_COMMAND); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_COMMAND(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_COMMAND); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_CHAT(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_CHAT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_CHAT(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_CHAT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_EXTERNAL_CHAT(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_EXTERNAL_CHAT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_SET_NAME(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_SET_NAME); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_QUIT(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_QUIT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_ERROR(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_ERROR); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_QUIT(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_QUIT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_ERROR_QUIT(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_ERROR_QUIT); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_SHUTDOWN(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_SHUTDOWN); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_NEWGAME(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_NEWGAME); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_RCON(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_RCON); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_RCON(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_RCON); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_CHECK_NEWGRFS(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_CHECK_NEWGRFS); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_NEWGRFS_CHECKED(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_NEWGRFS_CHECKED); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_MOVE(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_MOVE); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_CLIENT_MOVE(Packet &) { return this->ReceiveInvalidPacket(PACKET_CLIENT_MOVE); }
NetworkRecvStatus NetworkGameSocketHandler::Receive_SERVER_CONFIG_UPDATE(Packet &) { return this->ReceiveInvalidPacket(PACKET_SERVER_CONFIG_UPDATE); }

void NetworkGameSocketHandler::DeferDeletion()
{
	openttd_rust_network_defer(this);
}

/* static */ void NetworkGameSocketHandler::ProcessDeferredDeletions()
{
	openttd_rust_network_process_deferred(&GetRustNetworkFrameServices());
}

const OpenTTDNetworkDispatchServices &GetRustNetworkDispatchServices() noexcept
{
	static const OpenTTDNetworkDispatchServices services{
		.client_chat = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_CLIENT_CHAT(*packet); });
			return result;
		},
		.client_command = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_CLIENT_COMMAND(*packet); });
			return result;
		},
		.client_move = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_CLIENT_MOVE(*packet); });
			return result;
		},
		.client_rcon = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_CLIENT_RCON(*packet); });
			return result;
		},
		.client_set_name = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_CLIENT_SET_NAME(*packet); });
			return result;
		},
		.illegal_packet = [](OpenTTDNetworkGameSocket *socket, uint8_t kind) noexcept { Debug(net, 0, "[tcp/game] Received illegal packet type {} from client {}", kind, static_cast<NetworkGameSocketHandler *>(socket->game)->client_id); },
		.invalid_packet = [](OpenTTDNetworkGameSocket *socket) noexcept { Debug(net, 0, "[tcp/game] Received invalid packet from client {}", static_cast<NetworkGameSocketHandler *>(socket->game)->client_id); },
		.invalid_type = [](OpenTTDNetworkGameSocket *socket, uint8_t kind) noexcept { Debug(net, 0, "[tcp/game] Received invalid packet type {} from client {}", kind, static_cast<NetworkGameSocketHandler *>(socket->game)->client_id); },
		.map_load_args = [](OpenTTDSessionFrame *frame) noexcept -> OpenTTDSessionLoad * { return &frame->packet_load.emplace().args; },
		.packet_end = [](OpenTTDSessionFrame *frame) noexcept { frame->packet_load.reset(); frame->packet_text.reset(); },
		.server_banned = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_BANNED(*packet); });
			return result;
		},
		.server_chat = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_CHAT(*packet); });
			return result;
		},
		.server_client_info = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_CLIENT_INFO(*packet); });
			return result;
		},
		.server_command = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_COMMAND(*packet); });
			return result;
		},
		.server_config_update = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_CONFIG_UPDATE(*packet); });
			return result;
		},
		.server_error = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_ERROR(*packet); });
			return result;
		},
		.server_external_chat = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_EXTERNAL_CHAT(*packet); });
			return result;
		},
		.server_full = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_FULL(*packet); });
			return result;
		},
		.server_game_info = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_GAME_INFO(*packet); });
			return result;
		},
		.server_move = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_MOVE(*packet); });
			return result;
		},
		.server_rcon = [](OpenTTDNetworkGameSocket *socket, Packet *packet, OpenTTDSessionFrame *frame) noexcept -> uint8_t {
			NetworkRecvStatus result = NETWORK_RECV_STATUS_OKAY;
			SessionException(frame, [&] { result = static_cast<NetworkGameSocketHandler *>(socket->game)->Receive_SERVER_RCON(*packet); });
			return result;
		},
		.text_begin = [](OpenTTDSessionFrame *frame) noexcept -> void * { return &frame->packet_text.emplace(); },
	};
	return services;
}

#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkDispatchServices.hpp"
bool CheckAbi_network_dispatch() { return AbiServicesComplete(GetRustNetworkDispatchServices()); }

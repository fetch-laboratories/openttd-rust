/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tcp_game.h Basic functions to receive and send TCP packets for game purposes.
 */

#ifndef NETWORK_CORE_TCP_GAME_H
#define NETWORK_CORE_TCP_GAME_H

#include "os_abstraction.h"
#include "tcp.h"
#include "../network_type.h"
#include "../../command_type.h"
#include "../../company_type.h"
#include "../../rust/network_command_ffi.h"
#include "../../rust/network_connection_ffi.h"
#include <chrono>

/**
 * Enum with all types of TCP packets.
 * For the exact meaning, look at #NetworkGameSocketHandler.
 */
enum PacketGameType : uint8_t {
	/*
	 * These first ten packets must remain in this order for backward and forward compatibility
	 * between clients that are trying to join directly. These packets can be received and/or sent
	 * by the server before the server has processed the 'join' packet from the client.
	 */

	/* Packets sent by socket accepting code without ever constructing a client socket instance. */
	PACKET_SERVER_FULL,                  ///< The server is full and has no place for you.
	PACKET_SERVER_BANNED,                ///< The server has banned you.

	/* Packets used by the client to join and an error message when the revision is wrong. */
	PACKET_CLIENT_JOIN,                  ///< The client telling the server it wants to join.
	PACKET_SERVER_ERROR,                 ///< Server sending an error message to the client.

	/* Unused packet types, formerly used for the pre-game lobby. */
	PACKET_CLIENT_UNUSED,                ///< Unused.
	PACKET_SERVER_UNUSED,                ///< Unused.

	/* Packets used to get the game info. */
	PACKET_SERVER_GAME_INFO,             ///< Information about the server.
	PACKET_CLIENT_GAME_INFO,             ///< Request information about the server.

	/* A server quitting this game. */
	PACKET_SERVER_NEWGAME,               ///< The server is preparing to start a new game.
	PACKET_SERVER_SHUTDOWN,              ///< The server is shutting down.

	/*
	 * Packets after here assume that the client
	 * and server are running the same version. As
	 * such ordering is unimportant from here on.
	 *
	 * The following is the remainder of the packets
	 * sent as part of authenticating and getting
	 * the map and other important data.
	 */

	/* After the join step, the first perform game authentication and enabling encryption. */
	PACKET_SERVER_AUTH_REQUEST,          ///< The server requests the client to authenticate using a number of methods.
	PACKET_CLIENT_AUTH_RESPONSE,         ///< The client responds to the authentication request.
	PACKET_SERVER_ENABLE_ENCRYPTION,     ///< The server tells that authentication has completed and requests to enable encryption with the keys of the last \c PACKET_CLIENT_AUTH_RESPONSE.

	/* After the authentication is done, the next step is identification. */
	PACKET_CLIENT_IDENTIFY,              ///< Client telling the server the client's name and requested company.

	/* After the identify step, the next is checking NewGRFs. */
	PACKET_SERVER_CHECK_NEWGRFS,         ///< Server sends NewGRF IDs and MD5 checksums for the client to check.
	PACKET_CLIENT_NEWGRFS_CHECKED,       ///< Client acknowledges that it has all required NewGRFs.

	/* The server welcomes the authenticated client and sends information of other clients. */
	PACKET_SERVER_WELCOME,               ///< Server welcomes you and gives you your #ClientID.
	PACKET_SERVER_CLIENT_INFO,           ///< Server sends you information about a client.

	/* Getting the savegame/map. */
	PACKET_CLIENT_GETMAP,                ///< Client requests the actual map.
	PACKET_SERVER_WAIT,                  ///< Server tells the client there are some people waiting for the map as well.
	PACKET_SERVER_MAP_BEGIN,             ///< Server tells the client that it is beginning to send the map.
	PACKET_SERVER_MAP_SIZE,              ///< Server tells the client what the (compressed) size of the map is.
	PACKET_SERVER_MAP_DATA,              ///< Server sends bits of the map to the client.
	PACKET_SERVER_MAP_DONE,              ///< Server tells it has just sent the last bits of the map to the client.
	PACKET_CLIENT_MAP_OK,                ///< Client tells the server that it received the whole map.

	PACKET_SERVER_JOIN,                  ///< Tells clients that a new client has joined.

	/*
	 * At this moment the client has the map and
	 * the client is fully authenticated. Now the
	 * normal communication starts.
	 */

	/* Game progress monitoring. */
	PACKET_SERVER_FRAME,                 ///< Server tells the client what frame it is in, and thus to where the client may progress.
	PACKET_CLIENT_ACK,                   ///< The client tells the server which frame it has executed.
	PACKET_SERVER_SYNC,                  ///< Server tells the client what the random state should be.

	/* Sending commands around. */
	PACKET_CLIENT_COMMAND,               ///< Client executed a command and sends it to the server.
	PACKET_SERVER_COMMAND,               ///< Server distributes a command to (all) the clients.

	/* Human communication! */
	PACKET_CLIENT_CHAT,                  ///< Client said something that should be distributed.
	PACKET_SERVER_CHAT,                  ///< Server distributing the message of a client (or itself).
	PACKET_SERVER_EXTERNAL_CHAT,         ///< Server distributing the message from external source.

	/* Remote console. */
	PACKET_CLIENT_RCON,                  ///< Client asks the server to execute some command.
	PACKET_SERVER_RCON,                  ///< Response of the executed command on the server.

	/* Moving a client.*/
	PACKET_CLIENT_MOVE,                  ///< A client would like to be moved to another company.
	PACKET_SERVER_MOVE,                  ///< Server tells everyone that someone is moved to another company.

	/* Configuration updates. */
	PACKET_CLIENT_SET_NAME,              ///< A client changes its name.
	PACKET_SERVER_CONFIG_UPDATE,         ///< Some network configuration important to the client changed.

	/* A client quitting. */
	PACKET_CLIENT_QUIT,                  ///< A client tells the server it is going to quit.
	PACKET_SERVER_QUIT,                  ///< A server tells that a client has quit.
	PACKET_CLIENT_ERROR,                 ///< A client reports an error to the server.
	PACKET_SERVER_ERROR_QUIT,            ///< A server tells that a client has hit an error and did quit.

	PACKET_END,                          ///< Must ALWAYS be on the end of this list!! (period)
};

/** Command parameters in Rust storage; a back-insertion container for EndianBufferWriter. */
class CommandData : public OpenTTDCommandData {
public:
	using value_type = uint8_t;

	CommandData() : OpenTTDCommandData{} {}
	CommandData(const CommandData &) = delete;
	CommandData(CommandData &&other) noexcept : OpenTTDCommandData{other} { static_cast<OpenTTDCommandData &>(other) = {}; }
	CommandData &operator=(const CommandData &) = delete;
	CommandData &operator=(CommandData &&) = delete;
	~CommandData() { openttd_rust_command_data_free(this); }

	void push_back(uint8_t byte)
	{
		if (this->size == this->capacity) openttd_rust_command_data_grow(this);
		this->data[this->size++] = byte;
	}
};

/** Command parameters of a queued packet. */
inline std::span<const uint8_t> CommandPacketData(const OpenTTDCommandPacket &cp) { return {cp.data.data, cp.data.size}; }

/**
 * A "queue" of CommandPackets, owned and ordered by Rust.
 * Not a std::queue because, when paused, some commands remain on the queue.
 * In other words, you do not always pop the first element from this queue.
 */
class CommandQueue : public OpenTTDCommandQueue {
public:
	CommandQueue() : OpenTTDCommandQueue{} {}
	CommandQueue(const CommandQueue &) = delete;
	CommandQueue &operator=(const CommandQueue &) = delete;
	~CommandQueue() { openttd_rust_command_queue_free(this); }

	size_t Count() const { return this->size; }
};

/** Base socket handler for all TCP sockets; frame and lag fields are the Rust sync record. */
class NetworkGameSocketHandler : public NetworkTCPSocketHandler, public OpenTTDNetworkSocketSync, public OpenTTDNetworkGameSocket {
/* TODO: rewrite into a proper class */
private:
	NetworkClientInfo *info = nullptr; ///< Client info related to this socket

protected:
	NetworkGameSocketHandler(SOCKET s, uint8_t role = 2);
public:
	ClientID client_id = INVALID_CLIENT_ID; ///< Client identifier
	CommandQueue incoming_queue; ///< The command-queue awaiting handling
	std::chrono::steady_clock::time_point LastPacket() const { return std::chrono::steady_clock::time_point{std::chrono::steady_clock::duration{this->last_packet_ns}}; }

	NetworkRecvStatus CloseConnection(bool error = true) override;

	/**
	 * Close the network connection due to the given status.
	 * @param status The reason the connection got closed.
	 */
	virtual NetworkRecvStatus CloseConnection(NetworkRecvStatus status) = 0;
	virtual ~NetworkGameSocketHandler() = default;

	/**
	 * Sets the client info for this socket handler.
	 * @param info The new client info.
	 */
	inline void SetInfo(NetworkClientInfo *info)
	{
		assert(info != nullptr && this->info == nullptr);
		this->info = info;
	}

	/**
	 * Gets the client info of this socket handler.
	 * @return The client info.
	 */
	inline NetworkClientInfo *GetInfo() const
	{
		return this->info;
	}

	NetworkRecvStatus ReceivePackets();

	bool IsPendingDeletion() const { return this->pending; }

	static void ProcessDeferredDeletions();
};

#endif /* NETWORK_CORE_TCP_GAME_H */

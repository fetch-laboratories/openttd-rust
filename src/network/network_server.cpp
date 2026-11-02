/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_server.cpp Server part of the network protocol. */

#include "../stdafx.h"
#include "../strings_func.h"
#include "core/network_game_info.h"
#include "network_admin.h"
#include "network_server.h"
#include "network_udp.h"
#include "network_base.h"
#include "../console_func.h"
#include "../company_base.h"
#include "../command_func.h"
#include "../saveload/saveload.h"
#include "../saveload/saveload_filter.h"
#include "../station_base.h"
#include "../genworld.h"
#include "../company_func.h"
#include "../company_gui.h"
#include "../company_cmd.h"
#include "../roadveh.h"
#include "../order_backup.h"
#include "../core/pool_func.hpp"
#include "../core/random_func.hpp"
#include "../company_cmd.h"
#include "../rev.h"
#include "../timer/timer.h"
#include "../timer/timer_game_calendar.h"
#include "../timer/timer_game_economy.h"
#include "../timer/timer_game_realtime.h"
#include <mutex>
#include <condition_variable>

#include "table/strings.h"

#include "../rust/company_services.h"
#include "../rust/settings_services.h"

#include "../safeguards.h"


/* This file handles all the server-commands */


/** Make very sure the preconditions given in network_type.h are actually followed */
static_assert(NetworkClientSocketPool::MAX_SIZE > MAX_CLIENTS);

/** The pool with clients. */
NetworkClientSocketPool _networkclientsocket_pool("NetworkClientSocket");
INSTANTIATE_POOL_METHODS(NetworkClientSocket)

/** Instantiate the listen sockets. */
template SocketList TCPListenHandler<ServerNetworkGameSocketHandler, PACKET_SERVER_FULL, PACKET_SERVER_BANNED>::sockets;

static NetworkAuthenticationDefaultPasswordProvider _password_provider(_settings_client.network.server_password); ///< Provides the password validation for the game's password.
static NetworkAuthenticationDefaultAuthorizedKeyHandler _authorized_key_handler(_settings_client.network.server_authorized_keys); ///< Provides the authorized key handling for the game authentication.
static NetworkAuthenticationDefaultAuthorizedKeyHandler _rcon_authorized_key_handler(_settings_client.network.rcon_authorized_keys); ///< Provides the authorized key validation for rcon.


/** Writing a savegame directly to a number of packets. */
struct PacketWriter : SaveFilter {
	ServerNetworkGameSocketHandler *cs; ///< Socket we are associated with.
	std::unique_ptr<Packet> current; ///< The packet we're currently writing to.
	size_t total_size;                  ///< Total size of the compressed savegame.
	std::deque<std::unique_ptr<Packet>> packets; ///< Packet queue of the savegame; send these "slowly" to the client. Cannot be a std::queue as we want to push the map size packet in front of the data packets.
	std::mutex mutex;                   ///< Mutex for making threaded saving safe.
	std::condition_variable exit_sig;   ///< Signal for threaded destruction of this packet writer.

	/**
	 * Create the packet writer.
	 * @param cs The socket handler we're making the packets for.
	 */
	PacketWriter(ServerNetworkGameSocketHandler *cs) : SaveFilter(nullptr), cs(cs), total_size(0)
	{
	}

	/** Make sure everything is cleaned up. */
	~PacketWriter()
	{
		std::unique_lock<std::mutex> lock(this->mutex);

		while (this->cs != nullptr) this->exit_sig.wait(lock);

		/* This must all wait until the Destroy function is called. */

		this->packets.clear();
		this->current = nullptr;
	}

	/**
	 * Begin the destruction of this packet writer. It can happen in two ways:
	 * in the first case the client disconnected while saving the map. In this
	 * case the saving has not finished and killed this PacketWriter. In that
	 * case we simply set cs to nullptr, triggering the appending to fail due to
	 * the connection problem and eventually triggering the destructor. In the
	 * second case the destructor is already called, and it is waiting for our
	 * signal which we will send. Only then the packets will be removed by the
	 * destructor.
	 */
	void Destroy()
	{
		std::unique_lock<std::mutex> lock(this->mutex);

		this->cs = nullptr;

		this->exit_sig.notify_all();
		lock.unlock();

		/* Make sure the saving is completely cancelled. Yes,
		 * we need to handle the save finish as well as the
		 * next connection might just be requesting a map. */
		WaitTillSaved();
	}

	/**
	 * Transfer all packets from here to the network's queue while holding
	 * the lock on our mutex.
	 * @return True iff the last packet of the map has been sent.
	 */
	bool TransferToNetworkQueue()
	{
		/* Unsafe check for the queue being empty or not. */
		if (this->packets.empty()) return false;

		std::lock_guard<std::mutex> lock(this->mutex);

		while (!this->packets.empty()) {
			bool last_packet = this->packets.front()->GetPacketType() == PACKET_SERVER_MAP_DONE;
			this->cs->SendPacket(std::move(this->packets.front()));
			this->packets.pop_front();

			if (last_packet) return true;
		}

		return false;
	}

	void Write(uint8_t *buf, size_t size) override
	{
		std::lock_guard<std::mutex> lock(this->mutex);

		/* We want to abort the saving when the socket is closed. */
		if (this->cs == nullptr) SlError(STR_NETWORK_ERROR_LOSTCONNECTION);

		if (this->current == nullptr) this->current = std::make_unique<Packet>(this->cs, PACKET_SERVER_MAP_DATA, TCP_MTU);

		std::span<const uint8_t> to_write(buf, size);
		while (!to_write.empty()) {
			to_write = this->current->Send_bytes(to_write);

			if (!this->current->CanWriteToPacket(1)) {
				this->packets.push_back(std::move(this->current));
				if (!to_write.empty()) this->current = std::make_unique<Packet>(this->cs, PACKET_SERVER_MAP_DATA, TCP_MTU);
			}
		}

		this->total_size += size;
	}

	void Finish() override
	{
		std::lock_guard<std::mutex> lock(this->mutex);

		/* We want to abort the saving when the socket is closed. */
		if (this->cs == nullptr) SlError(STR_NETWORK_ERROR_LOSTCONNECTION);

		/* Make sure the last packet is flushed. */
		if (this->current != nullptr) this->packets.push_back(std::move(this->current));

		/* Add a packet stating that this is the end to the queue. */
		this->packets.push_back(std::make_unique<Packet>(this->cs, PACKET_SERVER_MAP_DONE));

		/* Fast-track the size to the client. */
		auto p = std::make_unique<Packet>(this->cs, PACKET_SERVER_MAP_SIZE);
		p->Send_uint32((uint32_t)this->total_size);
		this->packets.push_front(std::move(p));
	}
};


/**
 * Create a new socket for the server side of the game connection.
 * @param s The socket to connect with.
 */
ServerNetworkGameSocketHandler::ServerNetworkGameSocketHandler(SOCKET s) : NetworkGameSocketHandler(s)
{
	this->client_id = static_cast<ClientID>(openttd_rust_network_next_client_id());
	this->receive_limit = _settings_client.network.bytes_per_frame_burst;

	Debug(net, 9, "client[{}] status = INACTIVE", this->client_id);

	/* The Socket and Info pools need to be the same in size. After all,
	 * each Socket will be associated with at most one Info object. As
	 * such if the Socket was allocated the Info object can as well. */
	static_assert(NetworkClientSocketPool::MAX_SIZE == NetworkClientInfoPool::MAX_SIZE);
}

/**
 * Clear everything related to this client.
 */
ServerNetworkGameSocketHandler::~ServerNetworkGameSocketHandler()
{
	delete this->GetInfo();

	if (_redirect_console_to_client == this->client_id) _redirect_console_to_client = INVALID_CLIENT_ID;
	OrderBackup::ResetUser(this->client_id);

	if (this->savegame != nullptr) {
		this->savegame->Destroy();
		this->savegame = nullptr;
	}

	InvalidateWindowData(WC_CLIENT_LIST, 0);
}

std::unique_ptr<Packet> ServerNetworkGameSocketHandler::ReceivePacket()
{
	/* Only allow receiving when we have some buffer free; this value
	 * can go negative, but eventually it will become positive again. */
	if (this->receive_limit <= 0) return nullptr;

	/* We can receive a packet, so try that and if needed account for
	 * the amount of received data. */
	std::unique_ptr<Packet> p = this->NetworkTCPSocketHandler::ReceivePacket();
	if (p != nullptr) this->receive_limit -= p->Size();
	return p;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::CloseConnection(NetworkRecvStatus status)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_server_close(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this, status));
}

/**
 * Whether an connection is allowed or not at this moment.
 * @return true if the connection is allowed.
 */
/* static */ bool ServerNetworkGameSocketHandler::AllowConnection()
{
	extern uint8_t _network_clients_connected;
	bool accept = _network_clients_connected < MAX_CLIENTS;

	/* We can't go over the MAX_CLIENTS limit here. However, the
	 * pool must have place for all clients and ourself. */
	static_assert(NetworkClientSocketPool::MAX_SIZE == MAX_CLIENTS + 1);
	assert(!accept || ServerNetworkGameSocketHandler::CanAllocateItem());
	return accept;
}

/** Send the packets for the server sockets. */
/* static */ void ServerNetworkGameSocketHandler::Send()
{
	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->writable) {
			if (cs->SendPackets() != SPS_CLOSED && cs->status == STATUS_MAP) {
				/* This client is in the middle of a map-send, call the function for that */
				cs->SendMap();
			}
		}
	}
}


/***********
 * Sending functions
 ************/

/**
 * Send the client information about a client.
 * @param ci The client to send information about.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendClientInfo(NetworkClientInfo *ci)
{
	Debug(net, 9, "client[{}] SendClientInfo(): client_id={}", this->client_id, ci->client_id);

	if (ci->client_id != INVALID_CLIENT_ID) {
		auto p = std::make_unique<Packet>(this, PACKET_SERVER_CLIENT_INFO);
		p->Send_uint32(ci->client_id);
		p->Send_uint8 (ci->client_playas);
		p->Send_string(ci->client_name);
		p->Send_string(ci->public_key);

		this->SendPacket(std::move(p));
	}
	return NETWORK_RECV_STATUS_OKAY;
}

/** Send the client information about the server. */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendGameInfo()
{
	Debug(net, 9, "client[{}] SendGameInfo()", this->client_id);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_GAME_INFO, TCP_MTU);
	SerializeNetworkGameInfo(*p, GetCurrentNetworkServerGameInfo());

	this->SendPacket(std::move(p));

	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Send an error to the client, and close its connection.
 * @param error The error to disconnect for.
 * @param reason In case of kicking a client, specifies the reason for kicking the client.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendError(NetworkErrorCode error, std::string_view reason)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_send_error(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this, error, {reinterpret_cast<const uint8_t *>(reason.data()), reason.size()}));
}


/** Request the game password. */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendAuthRequest()
{
	Debug(net, 9, "client[{}] SendAuthRequest()", this->client_id);

	/* Invalid packet when status is anything but STATUS_INACTIVE or STATUS_AUTH_GAME. */
	if (this->status != STATUS_INACTIVE && status != STATUS_AUTH_GAME) return this->CloseConnection(NETWORK_RECV_STATUS_MALFORMED_PACKET);

	Debug(net, 9, "client[{}] status = AUTH_GAME", this->client_id);
	this->status = STATUS_AUTH_GAME;

	/* Reset 'lag' counters */
	this->last_frame = this->last_frame_server = _frame_counter;

	if (this->authentication_handler == nullptr) {
		this->authentication_handler = NetworkAuthenticationServerHandler::Create(&_password_provider, &_authorized_key_handler);
	}

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_AUTH_REQUEST);
	this->authentication_handler->SendRequest(*p);

	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/** Notify the client that the authentication has completed and tell that for the remainder of this socket encryption is enabled. */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendEnableEncryption()
{
	Debug(net, 9, "client[{}] SendEnableEncryption()", this->client_id);

	/* Invalid packet when status is anything but STATUS_AUTH_GAME. */
	if (this->status != STATUS_AUTH_GAME) return this->CloseConnection(NETWORK_RECV_STATUS_MALFORMED_PACKET);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_ENABLE_ENCRYPTION);
	this->authentication_handler->SendEnableEncryption(*p);
	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}


/** This sends the map to the client */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendMap()
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_send_map(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}


/**
 * Send a command to the client to execute.
 * @param cp The command to send.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendCommand(const OpenTTDCommandPacket &cp)
{
	Debug(net, 9, "client[{}] SendCommand(): cmd={}", this->client_id, cp.cmd);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_COMMAND);

	this->NetworkGameSocketHandler::SendCommand(*p, cp);
	p->Send_uint32(cp.frame);
	p->Send_bool  (cp.my_cmd);

	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Send a chat message.
 * @param action The action associated with the message.
 * @param client_id The origin of the chat message.
 * @param self_send Whether we did send the message.
 * @param msg The actual message.
 * @param data Arbitrary extra data.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendChat(NetworkAction action, ClientID client_id, bool self_send, std::string_view msg, int64_t data)
{
	Debug(net, 9, "client[{}] SendChat(): action={}, client_id={}, self_send={}", this->client_id, action, client_id, self_send);

	if (this->status < STATUS_PRE_ACTIVE) return NETWORK_RECV_STATUS_OKAY;

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_CHAT);

	p->Send_uint8 (action);
	p->Send_uint32(client_id);
	p->Send_bool  (self_send);
	p->Send_string(msg);
	p->Send_uint64(data);

	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Send a chat message from external source.
 * @param source Name of the source this message came from.
 * @param colour TextColour to use for the message.
 * @param user Name of the user who sent the message.
 * @param msg The actual message.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendExternalChat(std::string_view source, TextColour colour, std::string_view user, std::string_view msg)
{
	Debug(net, 9, "client[{}] SendExternalChat(): source={}", this->client_id, source);

	if (this->status < STATUS_PRE_ACTIVE) return NETWORK_RECV_STATUS_OKAY;

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_EXTERNAL_CHAT);

	p->Send_string(source);
	p->Send_uint16(colour);
	p->Send_string(user);
	p->Send_string(msg);

	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}


/**
 * Send the result of a console action.
 * @param colour The colour of the result.
 * @param command The command that was executed.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendRConResult(uint16_t colour, std::string_view command)
{
	Debug(net, 9, "client[{}] SendRConResult()", this->client_id);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_RCON);

	p->Send_uint16(colour);
	p->Send_string(command);
	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Tell that a client moved to another company.
 * @param client_id The client that moved.
 * @param company_id The company the client moved to.
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendMove(ClientID client_id, CompanyID company_id)
{
	Debug(net, 9, "client[{}] SendMove(): client_id={}", this->client_id, client_id);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_MOVE);

	p->Send_uint32(client_id);
	p->Send_uint8(company_id);
	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/** Send an update about the max company/spectator counts. */
NetworkRecvStatus ServerNetworkGameSocketHandler::SendConfigUpdate()
{
	Debug(net, 9, "client[{}] SendConfigUpdate()", this->client_id);

	auto p = std::make_unique<Packet>(this, PACKET_SERVER_CONFIG_UPDATE);

	p->Send_uint8(_settings_client.network.max_companies);
	p->Send_string(_settings_client.network.server_name);
	this->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/***********
 * Receiving functions
 ************/

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_GAME_INFO(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_game_info(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_NEWGRFS_CHECKED(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_newgrfs_checked(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_JOIN(Packet &p)
{
	std::string text;
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_join(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this, &p, &text));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_IDENTIFY(Packet &p)
{
	std::string text;
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_identify(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this, &p, &text));
}

static NetworkErrorCode GetErrorForAuthenticationMethod(NetworkAuthenticationMethod method)
{
	switch (method) {
		case NetworkAuthenticationMethod::X25519_PAKE:
			return NETWORK_ERROR_WRONG_PASSWORD;
		case NetworkAuthenticationMethod::X25519_AuthorizedKey:
			return NETWORK_ERROR_NOT_ON_ALLOW_LIST;

		default:
			NOT_REACHED();
	}
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_AUTH_RESPONSE(Packet &p)
{
	if (this->status != STATUS_AUTH_GAME) {
		return this->SendError(NETWORK_ERROR_NOT_EXPECTED);
	}

	Debug(net, 9, "client[{}] Receive_CLIENT_AUTH_RESPONSE()", this->client_id);

	auto authentication_method = this->authentication_handler->GetAuthenticationMethod();
	switch (this->authentication_handler->ReceiveResponse(p)) {
		case NetworkAuthenticationServerHandler::ResponseResult::Authenticated:
			break;

		case NetworkAuthenticationServerHandler::ResponseResult::RetryNextMethod:
			return this->SendAuthRequest();

		case NetworkAuthenticationServerHandler::ResponseResult::NotAuthenticated:
		default:
			return this->SendError(GetErrorForAuthenticationMethod(authentication_method));
	}

	NetworkRecvStatus status = this->SendEnableEncryption();
	if (status != NETWORK_RECV_STATUS_OKAY) return status;

	this->peer_public_key = this->authentication_handler->GetPeerPublicKey();
	this->receive_encryption_handler = this->authentication_handler->CreateClientToServerEncryptionHandler();
	this->send_encryption_handler = this->authentication_handler->CreateServerToClientEncryptionHandler();
	this->authentication_handler = nullptr;

	Debug(net, 9, "client[{}] status = IDENTIFY", this->client_id);
	this->status = STATUS_IDENTIFY;

	/* Reset 'lag' counters */
	this->last_frame = this->last_frame_server = _frame_counter;

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_GETMAP(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_getmap(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_MAP_OK(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_map_ok(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}

/**
 * The client has done a command and wants us to handle it
 * @param p the packet in which the command was sent
 */
NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_COMMAND(Packet &p)
{
	/* The client was never joined.. so this is impossible, right?
	 *  Ignore the packet, give the client a warning, and close the connection */
	if (this->status < STATUS_DONE_MAP || this->HasClientQuit()) {
		return this->SendError(NETWORK_ERROR_NOT_EXPECTED);
	}

	if (this->incoming_queue.Count() >= _settings_client.network.max_commands_in_queue) {
		return this->SendError(NETWORK_ERROR_TOO_MANY_COMMANDS);
	}

	Debug(net, 9, "client[{}] Receive_CLIENT_COMMAND()", this->client_id);

	CommandPacket cp;
	auto err = this->ReceiveCommand(p, cp);

	if (this->HasClientQuit()) return NETWORK_RECV_STATUS_CLIENT_QUIT;

	NetworkClientInfo *ci = this->GetInfo();

	if (err.has_value()) {
		IConsolePrint(CC_WARNING, "Dropping client #{} (IP: {}) due to {}.", ci->client_id, this->GetClientIP(), *err);
		return this->SendError(NETWORK_ERROR_NOT_EXPECTED);
	}

	if (GetCommandFlags(cp.Cmd()).Test(CommandFlag::Server) && ci->client_id != CLIENT_ID_SERVER) {
		IConsolePrint(CC_WARNING, "Kicking client #{} (IP: {}) due to calling a server only command {}.", ci->client_id, this->GetClientIP(), cp.Cmd());
		return this->SendError(NETWORK_ERROR_KICKED);
	}

	if (!GetCommandFlags(cp.Cmd()).Test(CommandFlag::Spectator) && !Company::IsValidID(cp.Company()) && ci->client_id != CLIENT_ID_SERVER) {
		IConsolePrint(CC_WARNING, "Kicking client #{} (IP: {}) due to calling a non-spectator command {}.", ci->client_id, this->GetClientIP(), cp.Cmd());
		return this->SendError(NETWORK_ERROR_KICKED);
	}

	/**
	 * Only CMD_COMPANY_CTRL is always allowed, for the rest, playas needs
	 * to match the company in the packet. If it doesn't, the client has done
	 * something pretty naughty (or a bug), and will be kicked
	 */
	CompanyCtrlAction cca = cp.Cmd() == CMD_COMPANY_CTRL ? std::get<0>(EndianBufferReader::ToValue<CommandTraits<CMD_COMPANY_CTRL>::Args>(cp.Data())) : CCA_NEW;
	if (!(cp.Cmd() == CMD_COMPANY_CTRL && cca == CCA_NEW && ci->client_playas == COMPANY_NEW_COMPANY) && ci->client_playas != cp.Company()) {
		IConsolePrint(CC_WARNING, "Kicking client #{} (IP: {}) due to calling a command as another company {}.",
		               ci->client_playas + 1, this->GetClientIP(), cp.Company() + 1);
		return this->SendError(NETWORK_ERROR_COMPANY_MISMATCH);
	}

	if (cp.Cmd() == CMD_COMPANY_CTRL) {
		if (cca != CCA_NEW || cp.Company() != COMPANY_SPECTATOR) {
			return this->SendError(NETWORK_ERROR_CHEATER);
		}

		/* Check if we are full - else it's possible for spectators to send a CMD_COMPANY_CTRL and the company is created regardless of max_companies! */
		if (Company::GetNumItems() >= _settings_client.network.max_companies) {
			NetworkServerSendChat(NETWORK_ACTION_SERVER_MESSAGE, DESTTYPE_CLIENT, ci->client_id, "cannot create new company, server full", CLIENT_ID_SERVER);
			return NETWORK_RECV_STATUS_OKAY;
		}
	}

	if (cp.Cmd() == CMD_COMPANY_ALLOW_LIST_CTRL) {
		/* Maybe the client just got moved before allowing? */
		if (ci->client_id != CLIENT_ID_SERVER && ci->client_playas != cp.Company()) return NETWORK_RECV_STATUS_OKAY;

		/* Only allow clients to add/remove currently joined clients. The server owner does not go via this method, so is allowed to do more. */
		std::string public_key = std::get<1>(EndianBufferReader::ToValue<CommandTraits<CMD_COMPANY_ALLOW_LIST_CTRL>::Args>(cp.Data()));
		bool found = false;
		for (const NetworkClientInfo *info : NetworkClientInfo::Iterate()) {
			if (info->public_key == public_key) {
				found = true;
				break;
			}
		}

		/* Maybe the client just left? */
		if (!found) return NETWORK_RECV_STATUS_OKAY;
	}

	if (GetCommandFlags(cp.Cmd()).Test(CommandFlag::ClientID)) NetworkReplaceCommandClientId(cp, this->client_id);

	this->incoming_queue.Push(std::move(cp));
	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_ERROR(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_error(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this, &p));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_QUIT(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_quit(&GetRustNetworkConnectionServices(), &GetRustNetworkFrameServices(), this));
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_ACK(Packet &p)
{
	static_assert(sizeof(this->status) == sizeof(uint8_t));
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_ack(&GetRustNetworkFrameServices(), this, static_cast<NetworkGameSocketHandler *>(this), reinterpret_cast<uint8_t *>(&this->status), this, &p));
}


/**
 * Send an actual chat message.
 * @param action The action that's performed.
 * @param desttype The type of destination.
 * @param dest The actual destination index.
 * @param msg The actual message.
 * @param from_id The origin of the message.
 * @param data Arbitrary data.
 * @param from_admin Whether the origin is an admin or not.
 */
void NetworkServerSendChat(NetworkAction action, DestType desttype, int dest, std::string_view msg, ClientID from_id, int64_t data, bool from_admin)
{
	const NetworkClientInfo *ci, *ci_own, *ci_to;

	switch (desttype) {
		case DESTTYPE_CLIENT:
			/* Are we sending to the server? */
			if ((ClientID)dest == CLIENT_ID_SERVER) {
				ci = NetworkClientInfo::GetByClientID(from_id);
				/* Display the text locally, and that is it */
				if (ci != nullptr) {
					NetworkTextMessage(action, GetDrawStringCompanyColour(ci->client_playas), false, ci->client_name, msg, data);

					if (_settings_client.network.server_admin_chat) {
						NetworkAdminChat(action, desttype, from_id, msg, data, from_admin);
					}
				}
			} else {
				/* Else find the client to send the message to */
				for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
					if (cs->client_id == (ClientID)dest && cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) {
						cs->SendChat(action, from_id, false, msg, data);
						break;
					}
				}
			}

			/* Display the message locally (so you know you have sent it) */
			if (from_id != (ClientID)dest) {
				if (from_id == CLIENT_ID_SERVER) {
					ci = NetworkClientInfo::GetByClientID(from_id);
					ci_to = NetworkClientInfo::GetByClientID((ClientID)dest);
					if (ci != nullptr && ci_to != nullptr) {
						NetworkTextMessage(action, GetDrawStringCompanyColour(ci->client_playas), true, ci_to->client_name, msg, data);
					}
				} else {
					for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
						if (cs->client_id == from_id && cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) {
							cs->SendChat(action, (ClientID)dest, true, msg, data);
							break;
						}
					}
				}
			}
			break;
		case DESTTYPE_TEAM: {
			/* If this is false, the message is already displayed on the client who sent it. */
			bool show_local = true;
			/* Find all clients that belong to this company */
			ci_to = nullptr;
			for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
				ci = cs->GetInfo();
				if (ci != nullptr && ci->client_playas == (CompanyID)dest && cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) {
					cs->SendChat(action, from_id, false, msg, data);
					if (cs->client_id == from_id) show_local = false;
					ci_to = ci; // Remember a client that is in the company for company-name
				}
			}

			/* if the server can read it, let the admin network read it, too. */
			if (_local_company == (CompanyID)dest && _settings_client.network.server_admin_chat) {
				NetworkAdminChat(action, desttype, from_id, msg, data, from_admin);
			}

			ci = NetworkClientInfo::GetByClientID(from_id);
			ci_own = NetworkClientInfo::GetByClientID(CLIENT_ID_SERVER);
			if (ci != nullptr && ci_own != nullptr && ci_own->client_playas == dest) {
				NetworkTextMessage(action, GetDrawStringCompanyColour(ci->client_playas), false, ci->client_name, msg, data);
				if (from_id == CLIENT_ID_SERVER) show_local = false;
				ci_to = ci_own;
			}

			/* There is no such client */
			if (ci_to == nullptr) break;

			/* Display the message locally (so you know you have sent it) */
			if (ci != nullptr && show_local) {
				if (from_id == CLIENT_ID_SERVER) {
					StringID str = Company::IsValidID(ci_to->client_playas) ? STR_COMPANY_NAME : STR_NETWORK_SPECTATORS;
					std::string name = GetString(str, ci_to->client_playas);
					NetworkTextMessage(action, GetDrawStringCompanyColour(ci_own->client_playas), true, name, msg, data);
				} else {
					for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
						if (cs->client_id == from_id && cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) {
							cs->SendChat(action, ci_to->client_id, true, msg, data);
						}
					}
				}
			}
			break;
		}
		default:
			Debug(net, 1, "Received unknown chat destination type {}; doing broadcast instead", desttype);
			[[fallthrough]];

		case DESTTYPE_BROADCAST:
			for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
				if (cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) cs->SendChat(action, from_id, false, msg, data);
			}

			NetworkAdminChat(action, desttype, from_id, msg, data, from_admin);

			ci = NetworkClientInfo::GetByClientID(from_id);
			if (ci != nullptr) {
				NetworkTextMessage(action, GetDrawStringCompanyColour(ci->client_playas), false, ci->client_name, msg, data);
			}
			break;
	}
}

/**
 * Send a chat message from external source.
 * @param source Name of the source this message came from.
 * @param colour TextColour to use for the message.
 * @param user Name of the user who sent the message.
 * @param msg The actual message.
 */
void NetworkServerSendExternalChat(std::string_view source, TextColour colour, std::string_view user, std::string_view msg)
{
	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) cs->SendExternalChat(source, colour, user, msg);
	}
	NetworkTextMessage(NETWORK_ACTION_EXTERNAL_CHAT, colour, false, user, msg, source);
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_CHAT(Packet &p)
{
	if (this->status < STATUS_PRE_ACTIVE) {
		/* Illegal call, return error and ignore the packet */
		return this->SendError(NETWORK_ERROR_NOT_AUTHORIZED);
	}

	NetworkAction action = (NetworkAction)p.Recv_uint8();
	DestType desttype = (DestType)p.Recv_uint8();
	int dest = p.Recv_uint32();

	Debug(net, 9, "client[{}] Receive_CLIENT_CHAT(): action={}, desttype={}, dest={}", this->client_id, action, desttype, dest);

	std::string msg = p.Recv_string(NETWORK_CHAT_LENGTH);
	int64_t data = p.Recv_uint64();

	NetworkClientInfo *ci = this->GetInfo();
	switch (action) {
		case NETWORK_ACTION_CHAT:
		case NETWORK_ACTION_CHAT_CLIENT:
		case NETWORK_ACTION_CHAT_COMPANY:
			NetworkServerSendChat(action, desttype, dest, msg, this->client_id, data);
			break;
		default:
			IConsolePrint(CC_WARNING, "Kicking client #{} (IP: {}) due to unknown chact action.", ci->client_id, this->GetClientIP());
			return this->SendError(NETWORK_ERROR_NOT_EXPECTED);
	}
	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_SET_NAME(Packet &p)
{
	if (this->status != STATUS_ACTIVE) {
		/* Illegal call, return error and ignore the packet */
		return this->SendError(NETWORK_ERROR_NOT_EXPECTED);
	}

	Debug(net, 9, "client[{}] Receive_CLIENT_SET_NAME()", this->client_id);

	NetworkClientInfo *ci;

	std::string client_name = p.Recv_string(NETWORK_CLIENT_NAME_LENGTH);
	ci = this->GetInfo();

	if (this->HasClientQuit()) return NETWORK_RECV_STATUS_CLIENT_QUIT;

	if (ci != nullptr) {
		if (!NetworkIsValidClientName(client_name)) {
			/* An invalid client name was given. However, the client ensures the name
			 * is valid before it is sent over the network, so something went horribly
			 * wrong. This is probably someone trying to troll us. */
			return this->SendError(NETWORK_ERROR_INVALID_CLIENT_NAME);
		}

		/* Display change */
		if (NetworkMakeClientNameUnique(client_name)) {
			NetworkTextMessage(NETWORK_ACTION_NAME_CHANGE, CC_DEFAULT, false, ci->client_name, client_name);
			ci->client_name = std::move(client_name);
			NetworkUpdateClientInfo(ci->client_id);
		}
	}
	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_RCON(Packet &p)
{
	if (this->status != STATUS_ACTIVE) return this->SendError(NETWORK_ERROR_NOT_EXPECTED);

	Debug(net, 9, "client[{}] Receive_CLIENT_RCON()", this->client_id);

	std::string password = p.Recv_string(NETWORK_PASSWORD_LENGTH);
	std::string command = p.Recv_string(NETWORK_RCONCOMMAND_LENGTH);

	if (_rcon_authorized_key_handler.IsAllowed(this->peer_public_key)) {
		/* We are allowed, nothing more to validate. */
	} else if (_settings_client.network.rcon_password.empty()) {
		return NETWORK_RECV_STATUS_OKAY;
	} else if (_settings_client.network.rcon_password != password) {
		Debug(net, 1, "[rcon] Wrong password from client-id {}", this->client_id);
		return NETWORK_RECV_STATUS_OKAY;
	}

	Debug(net, 3, "[rcon] Client-id {} executed: {}", this->client_id, command);

	_redirect_console_to_client = this->client_id;
	IConsoleCmdExec(command);
	_redirect_console_to_client = INVALID_CLIENT_ID;
	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ServerNetworkGameSocketHandler::Receive_CLIENT_MOVE(Packet &p)
{
	if (this->status != STATUS_ACTIVE) return this->SendError(NETWORK_ERROR_NOT_EXPECTED);

	CompanyID company_id = (Owner)p.Recv_uint8();

	Debug(net, 9, "client[{}] Receive_CLIENT_MOVE(): company_id={}", this->client_id, company_id);

	/* Check if the company is valid, we don't allow moving to AI companies */
	if (company_id != COMPANY_SPECTATOR && !Company::IsValidHumanID(company_id)) return NETWORK_RECV_STATUS_OKAY;

	if (company_id != COMPANY_SPECTATOR && !Company::Get(company_id)->allow_list.Contains(this->peer_public_key)) {
		Debug(net, 2, "Wrong public key from client-id #{} for company #{}", this->client_id, company_id + 1);
		return NETWORK_RECV_STATUS_OKAY;
	}

	/* if we get here we can move the client */
	NetworkServerDoMove(this->client_id, company_id);
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Get the company stats.
 */
NetworkCompanyStatsArray NetworkGetCompanyStats()
{
	NetworkCompanyStatsArray stats = {};

	/* Go through all vehicles and count the type of vehicles */
	for (const Vehicle *v : Vehicle::Iterate()) {
		if (!Company::IsValidID(v->owner) || !v->IsPrimaryVehicle()) continue;
		uint8_t type = 0;
		switch (v->type) {
			case VEH_TRAIN: type = NETWORK_VEH_TRAIN; break;
			case VEH_ROAD: type = RoadVehicle::From(v)->IsBus() ? NETWORK_VEH_BUS : NETWORK_VEH_LORRY; break;
			case VEH_AIRCRAFT: type = NETWORK_VEH_PLANE; break;
			case VEH_SHIP: type = NETWORK_VEH_SHIP; break;
			default: continue;
		}
		stats[v->owner].num_vehicle[type]++;
	}

	/* Go through all stations and count the types of stations */
	for (const Station *s : Station::Iterate()) {
		if (Company::IsValidID(s->owner)) {
			NetworkCompanyStats *npi = &stats[s->owner];

			if (s->facilities.Test(StationFacility::Train))     npi->num_station[NETWORK_VEH_TRAIN]++;
			if (s->facilities.Test(StationFacility::TruckStop)) npi->num_station[NETWORK_VEH_LORRY]++;
			if (s->facilities.Test(StationFacility::BusStop))   npi->num_station[NETWORK_VEH_BUS]++;
			if (s->facilities.Test(StationFacility::Airport))   npi->num_station[NETWORK_VEH_PLANE]++;
			if (s->facilities.Test(StationFacility::Dock))      npi->num_station[NETWORK_VEH_SHIP]++;
		}
	}

	return stats;
}

/**
 * Send updated client info of a particular client.
 * @param client_id The client to send it for.
 */
void NetworkUpdateClientInfo(ClientID client_id)
{
	NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(client_id);

	if (ci == nullptr) return;

	Debug(desync, 1, "client: {:08x}; {:02x}; {:02x}; {:04x}", TimerGameEconomy::date, TimerGameEconomy::date_fract, ci->client_playas, client_id);

	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->status >= ServerNetworkGameSocketHandler::STATUS_AUTHORIZED) {
			cs->SendClientInfo(ci);
		}
	}

	NetworkAdminClientUpdate(ci);
}

/**
 * Remove companies that have not been used depending on the \c autoclean_companies setting
 * and values for \c autoclean_protected, which removes any company, and
 * \c autoclean_novehicles, which removes companies without vehicles.
 */
static void NetworkAutoCleanCompanies()
{
	CompanyMask has_clients{};
	CompanyMask has_vehicles{};

	if (!_settings_client.network.autoclean_companies) return;

	/* Detect the active companies */
	for (const NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (Company::IsValidID(ci->client_playas)) has_clients.Set(ci->client_playas);
	}

	if (!_network_dedicated) {
		const NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(CLIENT_ID_SERVER);
		assert(ci != nullptr);
		if (Company::IsValidID(ci->client_playas)) has_clients.Set(ci->client_playas);
	}

	if (_settings_client.network.autoclean_novehicles != 0) {
		for (const Company *c : Company::Iterate()) {
			if (std::any_of(std::begin(c->group_all), std::end(c->group_all), [](const GroupStatistics &gs) { return gs.num_vehicle != 0; })) has_vehicles.Set(c->index);
		}
	}

	/* Go through all the companies */
	for (Company *c : Company::Iterate()) {
		/* Skip the non-active once */
		if (c->is_ai) continue;

		if (!has_clients.Test(c->index)) {
			/* The company is empty for one month more */
			if (c->Finances().months_empty != std::numeric_limits<decltype(c->Finances().months_empty)>::max()) c->Finances().months_empty++;

			/* Is the company empty for autoclean_protected-months? */
			if (_settings_client.network.autoclean_protected != 0 && c->Finances().months_empty > _settings_client.network.autoclean_protected) {
				/* Shut the company down */
				Command<CMD_COMPANY_CTRL>::Post(CCA_DELETE, c->index, CRR_AUTOCLEAN, INVALID_CLIENT_ID);
				IConsolePrint(CC_INFO, "Auto-cleaned company #{}.", c->index + 1);
			}
			/* Is the company empty for autoclean_novehicles-months, and has no vehicles? */
			if (_settings_client.network.autoclean_novehicles != 0 && c->Finances().months_empty > _settings_client.network.autoclean_novehicles && !has_vehicles.Test(c->index)) {
				/* Shut the company down */
				Command<CMD_COMPANY_CTRL>::Post(CCA_DELETE, c->index, CRR_AUTOCLEAN, INVALID_CLIENT_ID);
				IConsolePrint(CC_INFO, "Auto-cleaned company #{} with no vehicles.", c->index + 1);
			}
		} else {
			/* It is not empty, reset the date */
			c->Finances().months_empty = 0;
		}
	}
}

/**
 * Check whether a name is unique, and otherwise try to make it unique.
 * @param new_name The name to check/modify.
 * @return True if an unique name was achieved.
 */
bool NetworkMakeClientNameUnique(std::string &name)
{
	bool is_name_unique = false;
	std::string original_name = name;

	for (uint number = 1; !is_name_unique && number <= MAX_CLIENTS; number++) {  // Something's really wrong when there're more names than clients
		is_name_unique = true;
		for (const NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
			if (ci->client_name == name) {
				/* Name already in use */
				is_name_unique = false;
				break;
			}
		}
		/* Check if it is the same as the server-name */
		const NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(CLIENT_ID_SERVER);
		if (ci != nullptr) {
			if (ci->client_name == name) is_name_unique = false; // name already in use
		}

		if (!is_name_unique) {
			/* Try a new name (<name> #1, <name> #2, and so on) */
			name = fmt::format("{} #{}", original_name, number);

			/* The constructed client name is larger than the limit,
			 * so... bail out as no valid name can be created. */
			if (name.size() >= NETWORK_CLIENT_NAME_LENGTH) return false;
		}
	}

	return is_name_unique;
}

/**
 * Change the client name of the given client
 * @param client_id the client to change the name of
 * @param new_name the new name for the client
 * @return true iff the name was changed
 */
bool NetworkServerChangeClientName(ClientID client_id, const std::string &new_name)
{
	/* Check if the name's already in use */
	for (NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_name == new_name) return false;
	}

	NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(client_id);
	if (ci == nullptr) return false;

	NetworkTextMessage(NETWORK_ACTION_NAME_CHANGE, CC_DEFAULT, true, ci->client_name, new_name);

	ci->client_name = new_name;

	NetworkUpdateClientInfo(client_id);
	return true;
}

/** Helper function to restart the map. */
static void NetworkRestartMap()
{
	_settings_newgame.game_creation.generation_seed = GENERATE_NEW_SEED;
	switch (_file_to_saveload.ftype.abstract) {
		case FT_SAVEGAME:
		case FT_SCENARIO:
			_switch_mode = SM_LOAD_GAME;
			break;

		case FT_HEIGHTMAP:
			_switch_mode = SM_START_HEIGHTMAP;
			break;

		default:
			_switch_mode = SM_NEWGAME;
	}
}

/** Timer to restart a network server automatically based on real-time hours played. Initialized at zero to disable until settings are loaded. */
static IntervalTimer<TimerGameRealtime> _network_restart_map_timer({std::chrono::hours::zero(), TimerGameRealtime::UNPAUSED}, [](auto)
{
	if (!_network_server) return;

	/* If setting is 0, this feature is disabled. */
	if (_settings_client.network.restart_hours == 0) return;

	Debug(net, 3, "Auto-restarting map: {} hours played", _settings_client.network.restart_hours);
	NetworkRestartMap();
});

/**
 * Reset the automatic network restart time interval.
 * @param reset Whether to reset the timer to zero.
 */
void ChangeNetworkRestartTime(bool reset)
{
	if (!_network_server) return;

	_network_restart_map_timer.SetInterval({ std::chrono::hours(_settings_client.network.restart_hours), TimerGameRealtime::UNPAUSED }, reset);
}

/** Check if we want to restart the map based on the year. */
static void NetworkCheckRestartMapYear()
{
	/* If setting is 0, this feature is disabled. */
	if (_settings_client.network.restart_game_year == 0) return;

	if (TimerGameCalendar::year >= _settings_client.network.restart_game_year) {
		Debug(net, 3, "Auto-restarting map: year {} reached", TimerGameCalendar::year);
		NetworkRestartMap();
	}
}

/** Calendar yearly "callback". Called whenever the calendar year changes. */
static const IntervalTimer<TimerGameCalendar> _calendar_network_yearly({ TimerGameCalendar::YEAR, TimerGameCalendar::Priority::NONE }, [](auto) {
	if (!_network_server) return;

	NetworkCheckRestartMapYear();
});

/** Economy yearly "callback". Called whenever the economy year changes. */
static const IntervalTimer<TimerGameEconomy> _economy_network_yearly({TimerGameEconomy::YEAR, TimerGameEconomy::Priority::NONE}, [](auto)
{
	if (!_network_server) return;

	NetworkAdminUpdate(AdminUpdateFrequency::Annually);
});

/** Quarterly "callback". Called whenever the economy quarter changes. */
static const IntervalTimer<TimerGameEconomy> _network_quarterly({TimerGameEconomy::QUARTER, TimerGameEconomy::Priority::NONE}, [](auto)
{
	if (!_network_server) return;

	NetworkAutoCleanCompanies();
	NetworkAdminUpdate(AdminUpdateFrequency::Quarterly);
});

/** Economy monthly "callback". Called whenever the economy month changes. */
static const IntervalTimer<TimerGameEconomy> _network_monthly({TimerGameEconomy::MONTH, TimerGameEconomy::Priority::NONE}, [](auto)
{
	if (!_network_server) return;

	NetworkAutoCleanCompanies();
	NetworkAdminUpdate(AdminUpdateFrequency::Monthly);
});

/** Economy weekly "callback". Called whenever the economy week changes. */
static const IntervalTimer<TimerGameEconomy> _network_weekly({TimerGameEconomy::WEEK, TimerGameEconomy::Priority::NONE}, [](auto)
{
	if (!_network_server) return;

	NetworkAdminUpdate(AdminUpdateFrequency::Weekly);
});

/** Daily "callback". Called whenever the economy date changes. */
static const IntervalTimer<TimerGameEconomy> _economy_network_daily({TimerGameEconomy::DAY, TimerGameEconomy::Priority::NONE}, [](auto)
{
	if (!_network_server) return;

	NetworkAdminUpdate(AdminUpdateFrequency::Daily);
});

/**
 * Get the IP address/hostname of the connected client.
 * @return The IP address.
 */
std::string_view ServerNetworkGameSocketHandler::GetClientIP()
{
	return this->client_address.GetHostname();
}

/** Show the status message of all clients on the console. */
void NetworkServerShowStatusToConsole()
{
	static const std::string_view stat_str[] = {
		"inactive",
		"authorizing",
		"identifying client",
		"checking NewGRFs",
		"authorized",
		"waiting",
		"loading map",
		"map done",
		"ready",
		"active"
	};
	static_assert(lengthof(stat_str) == NetworkClientSocket::STATUS_END);

	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		NetworkClientInfo *ci = cs->GetInfo();
		if (ci == nullptr) continue;
		uint lag = NetworkCalculateLag(cs);

		std::string_view status = (cs->status < (ptrdiff_t)lengthof(stat_str) ? stat_str[cs->status] : "unknown");
		IConsolePrint(CC_INFO, "Client #{}  name: '{}'  status: '{}'  frame-lag: {}  company: {}  IP: {}",
			cs->client_id, ci->client_name, status, lag,
			ci->client_playas + (Company::IsValidID(ci->client_playas) ? 1 : 0),
			cs->GetClientIP());
	}
}

/**
 * Send Config Update
 */
void NetworkServerSendConfigUpdate()
{
	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->status >= NetworkClientSocket::STATUS_PRE_ACTIVE) cs->SendConfigUpdate();
	}
}

/** Update the server's NetworkServerGameInfo due to changes in settings. */
void NetworkServerUpdateGameInfo()
{
	if (_network_server) FillStaticNetworkServerGameInfo();
}

/**
 * Handle the tid-bits of moving a client from one company to another.
 * @param client_id id of the client we want to move.
 * @param company_id id of the company we want to move the client to.
 * @return void
 */
void NetworkServerDoMove(ClientID client_id, CompanyID company_id)
{
	/* Only allow non-dedicated servers and normal clients to be moved */
	if (client_id == CLIENT_ID_SERVER && _network_dedicated) return;

	NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(client_id);
	assert(ci != nullptr);

	/* No need to waste network resources if the client is in the company already! */
	if (ci->client_playas == company_id) return;

	ci->client_playas = company_id;

	if (client_id == CLIENT_ID_SERVER) {
		SetLocalCompany(company_id);
	} else {
		NetworkClientSocket *cs = NetworkClientSocket::GetByClientID(client_id);
		/* When the company isn't authorized we can't move them yet. */
		if (cs->status < NetworkClientSocket::STATUS_AUTHORIZED) return;
		cs->SendMove(client_id, company_id);
	}

	/* Announce the client's move. */
	NetworkUpdateClientInfo(client_id);

	if (company_id == COMPANY_SPECTATOR) {
		/* The client has joined spectators. */
		NetworkServerSendChat(NETWORK_ACTION_COMPANY_SPECTATOR, DESTTYPE_BROADCAST, 0, "", client_id);
	} else {
		/* The client has joined another company. */
		std::string company_name = GetString(STR_COMPANY_NAME, company_id);
		NetworkServerSendChat(NETWORK_ACTION_COMPANY_JOIN, DESTTYPE_BROADCAST, 0, company_name, client_id);
	}

	InvalidateWindowData(WC_CLIENT_LIST, 0);
}

/**
 * Send an rcon reply to the client.
 * @param client_id The identifier of the client.
 * @param colour_code The colour of the text.
 * @param string The actual reply.
 */
void NetworkServerSendRcon(ClientID client_id, TextColour colour_code, std::string_view string)
{
	NetworkClientSocket::GetByClientID(client_id)->SendRConResult(colour_code, string);
}

/**
 * Kick a single client.
 * @param client_id The client to kick.
 * @param reason In case of kicking a client, specifies the reason for kicking the client.
 */
void NetworkServerKickClient(ClientID client_id, std::string_view reason)
{
	if (client_id == CLIENT_ID_SERVER) return;
	NetworkClientSocket::GetByClientID(client_id)->SendError(NETWORK_ERROR_KICKED, reason);
}

/**
 * Ban, or kick, everyone joined from the given client's IP.
 * @param client_id The client to check for.
 * @param ban Whether to ban or kick.
 * @param reason In case of kicking a client, specifies the reason for kicking the client.
 */
uint NetworkServerKickOrBanIP(ClientID client_id, bool ban, std::string_view reason)
{
	return NetworkServerKickOrBanIP(NetworkClientSocket::GetByClientID(client_id)->GetClientIP(), ban, reason);
}

/**
 * Kick or ban someone based on an IP address.
 * @param ip The IP address/range to ban/kick.
 * @param ban Whether to ban or just kick.
 * @param reason In case of kicking a client, specifies the reason for kicking the client.
 */
uint NetworkServerKickOrBanIP(std::string_view ip, bool ban, std::string_view reason)
{
	/* Add address to ban-list */
	if (ban) {
		bool contains = false;
		for (const auto &iter : _network_ban_list) {
			if (iter == ip) {
				contains = true;
				break;
			}
		}
		if (!contains) _network_ban_list.emplace_back(ip);
	}

	uint n = 0;

	/* There can be multiple clients with the same IP, kick them all but don't kill the server,
	 * or the client doing the rcon. The latter can't be kicked because kicking frees closes
	 * and subsequently free the connection related instances, which we would be reading from
	 * and writing to after returning. So we would read or write data from freed memory up till
	 * the segfault triggers. */
	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->client_id == CLIENT_ID_SERVER) continue;
		if (cs->client_id == _redirect_console_to_client) continue;
		if (cs->client_address.IsInNetmask(ip)) {
			NetworkServerKickClient(cs->client_id, reason);
			n++;
		}
	}

	return n;
}

/**
 * Check whether a particular company has clients.
 * @param company The company to check.
 * @return True if at least one client is joined to the company.
 */
bool NetworkCompanyHasClients(CompanyID company)
{
	for (const NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_playas == company) return true;
	}
	return false;
}


/**
 * Get the name of the client, if the user did not send it yet, Client ID is used.
 * @param client_name The variable to write the name to.
 * @param last        The pointer to the last element of the destination buffer
 */
std::string ServerNetworkGameSocketHandler::GetClientName() const
{
	const NetworkClientInfo *ci = this->GetInfo();
	if (ci != nullptr && !ci->client_name.empty()) return ci->client_name;

	return fmt::format("Client #{}", this->client_id);
}

/**
 * Print all the clients to the console
 */
void NetworkPrintClients()
{
	for (NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (_network_server) {
			IConsolePrint(CC_INFO, "Client #{}  name: '{}'  company: {}  IP: {}",
					ci->client_id,
					ci->client_name,
					ci->client_playas + (Company::IsValidID(ci->client_playas) ? 1 : 0),
					ci->client_id == CLIENT_ID_SERVER ? "server" : NetworkClientSocket::GetByClientID(ci->client_id)->GetClientIP());
		} else {
			IConsolePrint(CC_INFO, "Client #{}  name: '{}'  company: {}",
					ci->client_id,
					ci->client_name,
					ci->client_playas + (Company::IsValidID(ci->client_playas) ? 1 : 0));
		}
	}
}

/**
 * Get the public key of the client with the given id.
 * @param client_id The id of the client.
 * @return View of the public key, which is empty when the client does not exist.
 */
std::string_view NetworkGetPublicKeyOfClient(ClientID client_id)
{
	auto socket = NetworkClientSocket::GetByClientID(client_id);
	return socket == nullptr ? "" : socket->GetPeerPublicKey();
}


/**
 * Perform all the server specific administration of a new company.
 * @param c  The newly created company; can't be nullptr.
 * @param ci The client information of the client that made the company; can be nullptr.
 */
void NetworkServerNewCompany(const Company *c, NetworkClientInfo *ci)
{
	assert(c != nullptr);

	if (!_network_server) return;

	if (ci != nullptr) {
		/* ci is nullptr when replaying, or for AIs. In neither case there is a client. */
		ci->client_playas = c->index;
		NetworkUpdateClientInfo(ci->client_id);

		/*
		 * This function is called from a command, but is only called for the server.
		 * The client information is managed out-of-band from the commands, so to not have a
		 * different state/president/company name in the different clients, we need to
		 * circumvent the normal ::Post logic and go directly to sending the command.
		 */
		Command<CMD_COMPANY_ALLOW_LIST_CTRL>::SendNet(STR_NULL, c->index, CALCA_ADD, ci->public_key);
		Command<CMD_RENAME_PRESIDENT>::SendNet(STR_NULL, c->index, ci->client_name);

		NetworkServerSendChat(NETWORK_ACTION_COMPANY_NEW, DESTTYPE_BROADCAST, 0, "", ci->client_id, c->index + 1);
	}
}

static_assert(CLIENT_ID_FIRST == 2 && COMPANY_NEW_COMPANY == CompanyID(254));
static_assert(NETWORK_REVISION_LENGTH == 33 && NETWORK_CLIENT_NAME_LENGTH == 25);
static_assert(NETWORK_ERROR_WRONG_REVISION == 8 && NETWORK_ERROR_FULL == 14 && NETWORK_ERROR_COMPANY_MISMATCH == 11);
static_assert(NETWORK_ERROR_INVALID_CLIENT_NAME == 20 && NETWORK_ERROR_NAME_IN_USE == 9 && NETWORK_ERROR_KICKED == 12 && NETWORK_ERROR_ILLEGAL_PACKET == 4);
static_assert(NETWORK_RECV_STATUS_CLIENT_QUIT == 4 && NETWORK_RECV_STATUS_SERVER_ERROR == 6);
static_assert(PACKET_SERVER_ERROR == 3 && PACKET_SERVER_ERROR_QUIT == 43 && PACKET_SERVER_QUIT == 41 && PACKET_SERVER_SHUTDOWN == 9 && PACKET_SERVER_NEWGAME == 8);
static_assert(PACKET_SERVER_WELCOME == 16 && PACKET_SERVER_WAIT == 19 && PACKET_SERVER_MAP_BEGIN == 20 && PACKET_SERVER_JOIN == 25);

extern uint8_t _network_clients_connected;

const OpenTTDNetworkConnectionServices &GetRustNetworkConnectionServices() noexcept
{
	static const OpenTTDNetworkConnectionServices services{
		.admin_error = [](void *socket, uint8_t error) noexcept { NetworkAdminClientError(static_cast<NetworkClientSocket *>(socket)->client_id, static_cast<NetworkErrorCode>(error)); },
		.admin_joined = [](void *socket) noexcept { NetworkAdminClientInfo(static_cast<NetworkClientSocket *>(socket), true); },
		.admin_quit = [](void *socket) noexcept { NetworkAdminClientQuit(static_cast<NetworkClientSocket *>(socket)->client_id); },
		.allocate_info = [](void *socket, void *name, uint8_t company) noexcept {
			assert(NetworkClientInfo::CanAllocateItem());
			auto *cs = static_cast<NetworkClientSocket *>(socket);
			auto *ci = new NetworkClientInfo(cs->client_id);
			cs->SetInfo(ci);
			ci->join_date = TimerGameEconomy::date;
			ci->client_name = std::move(*static_cast<std::string *>(name));
			ci->client_playas = CompanyID(company);
			ci->public_key = cs->peer_public_key;
			Debug(desync, 1, "client: {:08x}; {:02x}; {:02x}; {:02x}", TimerGameEconomy::date, TimerGameEconomy::date_fract, ci->client_playas, ci->index);
		},
		.auth_request = [](void *socket) noexcept -> uint8_t { return static_cast<NetworkClientSocket *>(socket)->SendAuthRequest(); },
		.client_company = [](void *socket) noexcept -> uint8_t { return static_cast<NetworkClientSocket *>(socket)->GetInfo()->client_playas.base(); },
		.client_error_announcement = [](void *socket, uint8_t error) noexcept {
			std::string client_name = static_cast<NetworkClientSocket *>(socket)->GetClientName();
			StringID strid = GetNetworkErrorMsg(static_cast<NetworkErrorCode>(error));
			Debug(net, 1, "'{}' reported an error and is closing its connection: {}", client_name, GetString(strid));
			NetworkTextMessage(NETWORK_ACTION_LEAVE, CC_DEFAULT, false, client_name, "", strid);
		},
		.client_id = [](void *socket) noexcept -> uint32_t { return static_cast<NetworkClientSocket *>(socket)->client_id; },
		.clients_connected = &_network_clients_connected,
		.clients_on = &_network_game_info.clients_on,
		.company_allows_client = [](uint8_t company, void *socket) noexcept { return RustCompanyAllowsPublicKey(company, static_cast<NetworkClientSocket *>(socket)->GetPeerPublicKey()); },
		.debug_authorized = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = AUTHORIZED", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_closed = [](void *socket) noexcept { Debug(net, 3, "[{}] Client #{} closed connection", ServerNetworkGameSocketHandler::GetName(), static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_done_map = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = DONE_MAP", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_game_info = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_GAME_INFO()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_getmap = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_GETMAP()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_identify = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_IDENTIFY()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_join = [](void *socket, void *revision, uint32_t version) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_JOIN(): client_revision={}, newgrf_version={}", static_cast<NetworkClientSocket *>(socket)->client_id, *static_cast<std::string *>(revision), version); },
		.debug_map = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = MAP", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_map_first = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendMap(): first_packet", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_map_last = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendMap(): last_packet", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_map_ok = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_MAP_OK()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_map_wait = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = MAP_WAIT", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_newgrf_check = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendNewGRFCheck()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_newgrfs = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = NEWGRFS_CHECK", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_newgrfs_checked = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_NEWGRFS_CHECKED()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_next_map = [](void *socket) noexcept { Debug(net, 9, "client[{}] CheckNextClientToSendMap()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_pre_active = [](void *socket) noexcept { Debug(net, 9, "client[{}] status = PRE_ACTIVE", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_receive_error = [](void *socket, uint8_t error) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_ERROR(): errorno={}", static_cast<NetworkClientSocket *>(socket)->client_id, error); },
		.debug_receive_quit = [](void *socket) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_QUIT()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_send_error = [](void *socket, uint8_t error) noexcept { Debug(net, 9, "client[{}] SendError(): error={}", static_cast<NetworkClientSocket *>(socket)->client_id, error); },
		.debug_send_error_quit = [](void *socket, uint32_t client_id, uint8_t error) noexcept { Debug(net, 9, "client[{}] SendErrorQuit(): client_id={}, errorno={}", static_cast<NetworkClientSocket *>(socket)->client_id, client_id, error); },
		.debug_send_join = [](void *socket, uint32_t client_id) noexcept { Debug(net, 9, "client[{}] SendJoin(): client_id={}", static_cast<NetworkClientSocket *>(socket)->client_id, client_id); },
		.debug_send_newgame = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendNewGame()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_send_quit = [](void *socket, uint32_t client_id) noexcept { Debug(net, 9, "client[{}] SendQuit(): client_id={}", static_cast<NetworkClientSocket *>(socket)->client_id, client_id); },
		.debug_send_shutdown = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendShutdown()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_send_wait = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendWait()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.debug_send_welcome = [](void *socket) noexcept { Debug(net, 9, "client[{}] SendWelcome()", static_cast<NetworkClientSocket *>(socket)->client_id); },
		.error_announcement = [](void *socket, uint32_t strid, OpenTTDByteSpan reason, bool kicked) noexcept {
			std::string client_name = static_cast<NetworkClientSocket *>(socket)->GetClientName();
			Debug(net, 1, "'{}' made an error and has been disconnected: {}", client_name, GetString(strid));
			NetworkTextMessage(kicked ? NETWORK_ACTION_KICKED : NETWORK_ACTION_LEAVE, CC_DEFAULT, false, client_name, kicked ? std::string_view(reinterpret_cast<const char *>(reason.data), reason.size) : "", strid);
		},
		.error_message = [](uint8_t error) noexcept -> uint32_t { return GetNetworkErrorMsg(static_cast<NetworkErrorCode>(error)); },
		.error_unjoined = [](void *socket, uint32_t strid) noexcept { Debug(net, 1, "Client {} made an error and has been disconnected: {}", static_cast<NetworkClientSocket *>(socket)->client_id, GetString(strid)); },
		.game = [](void *socket) noexcept -> void * { return static_cast<NetworkGameSocketHandler *>(static_cast<NetworkClientSocket *>(socket)); },
		.grfs_empty = []() noexcept -> bool { return _grfconfig.empty(); },
		.grfs_packet = [](void *socket) noexcept -> void * {
			auto *p = new Packet(static_cast<NetworkClientSocket *>(socket), PACKET_SERVER_CHECK_NEWGRFS, TCP_MTU);
			uint grf_count = std::ranges::count_if(_grfconfig, [](const auto &c){ return !c->flags.Test(GRFConfigFlag::Static); });
			p->Send_uint8(grf_count);
			for (const auto &c : _grfconfig) {
				if (!c->flags.Test(GRFConfigFlag::Static)) SerializeGRFIdentifier(*p, c->ident);
			}
			return p;
		},
		.info = [](void *socket) noexcept -> void * { return static_cast<NetworkClientSocket *>(socket)->GetInfo(); },
		.join_date = [](void *socket) noexcept -> int32_t { return static_cast<NetworkClientSocket *>(socket)->GetInfo()->join_date.base(); },
		.joined = [](void *socket) noexcept {
			auto *cs = static_cast<NetworkClientSocket *>(socket);
			std::string client_name = cs->GetClientName();
			NetworkTextMessage(NETWORK_ACTION_JOIN, CC_DEFAULT, false, client_name, "", cs->client_id);
			InvalidateWindowData(WC_CLIENT_LIST, 0);
			Debug(net, 3, "[{}] Client #{} ({}) joined as {}", ServerNetworkGameSocketHandler::GetName(), cs->client_id, cs->GetClientIP(), client_name);
		},
		.lost_connection_announcement = [](void *socket) noexcept { std::string client_name = static_cast<NetworkClientSocket *>(socket)->GetClientName(); NetworkTextMessage(NETWORK_ACTION_LEAVE, CC_DEFAULT, false, client_name, "", STR_NETWORK_ERROR_CLIENT_CONNECTION_LOST); },
		.map_create = [](void *socket) noexcept { WaitTillSaved(); static_cast<NetworkClientSocket *>(socket)->savegame = std::make_shared<PacketWriter>(static_cast<NetworkClientSocket *>(socket)); },
		.map_destroy = [](void *socket) noexcept { auto *cs = static_cast<NetworkClientSocket *>(socket); cs->savegame->Destroy(); cs->savegame = nullptr; },
		.map_save = [](void *socket) noexcept { if (SaveWithFilter(static_cast<NetworkClientSocket *>(socket)->savegame, true) != SL_OK) UserError("network savedump failed"); },
		.map_transfer = [](void *socket) noexcept -> bool { return static_cast<NetworkClientSocket *>(socket)->savegame->TransferToNetworkQueue(); },
		.max_clients = RustNetworkMaxClients,
		.max_companies = RustNetworkMaxCompanies,
		.min_active_clients = RustNetworkMinActiveClients,
		.network_server = &_network_server,
		.networking = &_networking,
		.newgrf_version = []() noexcept -> uint32_t { return _openttd_newgrf_version; },
		.outgoing_queue = [](void *socket) noexcept -> OpenTTDCommandQueue * { return &static_cast<NetworkClientSocket *>(socket)->outgoing_queue; },
		.pause_on_join = RustNetworkPauseOnJoin,
		.quit_announcement = [](void *socket) noexcept { std::string client_name = static_cast<NetworkClientSocket *>(socket)->GetClientName(); NetworkTextMessage(NETWORK_ACTION_LEAVE, CC_DEFAULT, false, client_name, "", STR_NETWORK_MESSAGE_CLIENT_LEAVING); },
		.revision_compatible = [](void *revision) noexcept -> bool { return IsNetworkCompatibleVersion(*static_cast<std::string *>(revision)); },
		.send_client_info = [](void *socket, void *info) noexcept -> uint8_t { return static_cast<NetworkClientSocket *>(socket)->SendClientInfo(static_cast<NetworkClientInfo *>(info)); },
		.send_config = [](void *socket) noexcept -> uint8_t { return static_cast<NetworkClientSocket *>(socket)->SendConfigUpdate(); },
		.send_game_info = [](void *socket) noexcept -> uint8_t { return static_cast<NetworkClientSocket *>(socket)->SendGameInfo(); },
		.status = [](void *socket) noexcept -> uint8_t * { return reinterpret_cast<uint8_t *>(&static_cast<NetworkClientSocket *>(socket)->status); },
		.sync = [](void *socket) noexcept -> OpenTTDNetworkSocketSync * { return static_cast<NetworkClientSocket *>(socket); },
		.unique_name = [](void *name) noexcept -> bool { return NetworkMakeClientNameUnique(*static_cast<std::string *>(name)); },
		.valid_name = [](void *name) noexcept -> bool { return NetworkIsValidClientName(*static_cast<std::string *>(name)); },
	};
	return services;
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkConnectionServices.hpp"
bool CheckAbi_network_connection() { return AbiServicesComplete(GetRustNetworkConnectionServices()); }

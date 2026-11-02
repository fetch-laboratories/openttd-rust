/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_client.cpp Client part of the network protocol. */

#include "../stdafx.h"
#include "network_gui.h"
#include "../saveload/saveload.h"
#include "../saveload/saveload_filter.h"
#include "../command_func.h"
#include "../console_func.h"
#include "../strings_func.h"
#include "../window_func.h"
#include "../company_func.h"
#include "../company_base.h"
#include "../company_gui.h"
#include "../company_cmd.h"
#include "../core/random_func.hpp"
#include "../timer/timer_game_tick.h"
#include "../timer/timer_game_calendar.h"
#include "../gfx_func.h"
#include "../error.h"
#include "../rev.h"
#include "network.h"
#include "network_base.h"
#include "network_client.h"
#include "network_gamelist.h"
#include "../core/backup_type.hpp"
#include "../thread.h"
#include "../social_integration.h"

#include "table/strings.h"

#include "../rust/network_client_ffi.h"
#include "../rust/settings_services.h"

#include "../safeguards.h"

/* This file handles all the client-commands */

/** Read some packets, and when do use that data as initial load filter. */
struct PacketReader : LoadFilter {
	using Buffer = std::deque<uint8_t>; ///< The underlying buffer type that's being use.

	Buffer buffer; ///< Buffer with the raw save game data.
	Buffer::const_iterator iterator; ///< Buffer we're going to write to/read from.
	size_t read_bytes = 0; ///< The total number of read bytes.

	/** Initialise everything. */
	PacketReader() : LoadFilter(nullptr)
	{
	}

	/**
	 * Add a packet to this buffer.
	 * @param p The packet to add.
	 */
	void AddPacket(Packet &p)
	{
		assert(this->read_bytes == 0);
		p.TransferOut([this](std::span<const uint8_t> source) {
			std::ranges::copy(source, std::back_inserter(this->buffer));
			return source.size();
		});
	}

	size_t Read(uint8_t *rbuf, size_t size) override
	{
		/* Limit the amount to read to whatever we still have. */
		size_t read_size = std::min(this->buffer.size() - this->read_bytes, size);

		std::copy_n(this->iterator, read_size, rbuf);
		std::advance(this->iterator, read_size);
		this->read_bytes += read_size;

		return read_size;
	}

	void Reset() override
	{
		this->read_bytes = 0;
		this->iterator = this->buffer.cbegin();
	}
};


/**
 * Create an emergency savegame when the network connection is lost.
 */
void ClientNetworkEmergencySave()
{
	static FiosNumberedSaveName _netsave_ctr("netsave");
	DoAutoOrNetsave(_netsave_ctr);
}


/**
 * Create a new socket for the client side of the game connection.
 * @param s The socket to connect with.
 */
ClientNetworkGameSocketHandler::ClientNetworkGameSocketHandler(SOCKET s, std::string_view connection_string) : NetworkGameSocketHandler(s), connection_string(connection_string)
{
	assert(ClientNetworkGameSocketHandler::my_client == nullptr);
	ClientNetworkGameSocketHandler::my_client = this;
}

/** Clear whatever we assigned. */
ClientNetworkGameSocketHandler::~ClientNetworkGameSocketHandler()
{
	assert(ClientNetworkGameSocketHandler::my_client == this);
	ClientNetworkGameSocketHandler::my_client = nullptr;

	delete this->GetInfo();
}

NetworkRecvStatus ClientNetworkGameSocketHandler::CloseConnection(NetworkRecvStatus status)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_close(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), status));
}

/**
 * Handle an error coming from the client side.
 * @param res The "error" that happened.
 */
void ClientNetworkGameSocketHandler::ClientError(NetworkRecvStatus res)
{
	openttd_rust_network_client_error(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), res);
}


/**
 * Check whether we received/can send some data from/to the server and
 * when that's the case handle it appropriately.
 * @return true when everything went okay.
 */
/* static */ bool ClientNetworkGameSocketHandler::Receive()
{
	if (my_client->CanSendReceive()) {
		NetworkRecvStatus res = my_client->ReceivePackets();
		if (res != NETWORK_RECV_STATUS_OKAY) {
			/* The client made an error of which we can not recover.
			 * Close the connection and drop back to the main menu. */
			my_client->ClientError(res);
			return false;
		}
	}
	return _networking;
}

/** Send the packets of this socket handler. */
/* static */ void ClientNetworkGameSocketHandler::Send()
{
	my_client->SendPackets();
	if (my_client != nullptr) my_client->CheckConnection();
}


/** Our client's connection. */
ClientNetworkGameSocketHandler * ClientNetworkGameSocketHandler::my_client = nullptr;

/** Maximum number of companies of the currently joined server. */
static uint8_t _network_server_max_companies;
/** The current name of the server you are on. */
std::string _network_server_name;

/** Information about the game to join to. */
NetworkJoinInfo _network_join;

/***********
 * Sending functions
 ************/


NetworkRecvStatus ClientNetworkGameSocketHandler::SendIdentify()
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_send_identify(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices()));
}


/**
 * Set the game password as requested.
 * @param password The game password.
 */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendAuthResponse()
{
	Debug(net, 9, "Client::SendAuthResponse()");

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_AUTH_RESPONSE);
	my_client->authentication_handler->SendResponse(*p);
	my_client->SendPacket(std::move(p));

	return NETWORK_RECV_STATUS_OKAY;
}


/**
 * Send a command to the server.
 * @param cp The command to send.
 */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendCommand(const OpenTTDCommandPacket &cp)
{
	Debug(net, 9, "Client::SendCommand(): cmd={}", cp.cmd);

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_COMMAND);
	my_client->NetworkGameSocketHandler::SendCommand(*p, cp);

	my_client->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/** Send a chat-packet over the network */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendChat(NetworkAction action, DestType type, int dest, std::string_view msg, int64_t data)
{
	Debug(net, 9, "Client::SendChat(): action={}, type={}, dest={}", action, type, dest);

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_CHAT);

	p->Send_uint8 (action);
	p->Send_uint8 (type);
	p->Send_uint32(dest);
	p->Send_string(msg);
	p->Send_uint64(data);

	my_client->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}


/**
 * Tell the server that we like to change the name of the client.
 * @param name The new name.
 */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendSetName(const std::string &name)
{
	Debug(net, 9, "Client::SendSetName()");

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_SET_NAME);

	p->Send_string(name);
	my_client->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}


/**
 * Send a console command.
 * @param pass The password for the remote command.
 * @param command The actual command.
 */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendRCon(std::string_view pass, std::string_view command)
{
	Debug(net, 9, "Client::SendRCon()");

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_RCON);
	p->Send_string(pass);
	p->Send_string(command);
	my_client->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Ask the server to move us.
 * @param company The company to move to.
 */
NetworkRecvStatus ClientNetworkGameSocketHandler::SendMove(CompanyID company)
{
	Debug(net, 9, "Client::SendMove(): company={}", company);

	auto p = std::make_unique<Packet>(my_client, PACKET_CLIENT_MOVE);
	p->Send_uint8(company);
	my_client->SendPacket(std::move(p));
	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Check whether the client is actually connected (and in the game).
 * @return True when the client is connected.
 */
bool ClientNetworkGameSocketHandler::IsConnected()
{
	return openttd_rust_network_client_is_connected(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices());
}


/***********
 * Receiving functions
 ************/

extern bool SafeLoad(const std::string &filename, SaveLoadOperation fop, DetailedFileType dft, GameMode newgm, Subdirectory subdir, std::shared_ptr<struct LoadFilter> lf);

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_FULL(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_full(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this)));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_BANNED(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_banned(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this)));
}

/* This packet contains info about the client (playas and name)
 *  as client we save this in NetworkClientInfo, linked via 'client_id'
 *  which is always an unique number on a server. */
NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_CLIENT_INFO(Packet &p)
{
	NetworkClientInfo *ci;
	ClientID client_id = (ClientID)p.Recv_uint32();
	CompanyID playas = (CompanyID)p.Recv_uint8();

	Debug(net, 9, "Client::Receive_SERVER_CLIENT_INFO(): client_id={}, playas={}", client_id, playas);

	std::string name = p.Recv_string(NETWORK_NAME_LENGTH);
	std::string public_key = p.Recv_string(NETWORK_PUBLIC_KEY_LENGTH);

	if (this->status < STATUS_AUTHORIZED) return NETWORK_RECV_STATUS_MALFORMED_PACKET;
	if (this->HasClientQuit()) return NETWORK_RECV_STATUS_CLIENT_QUIT;
	/* The server validates the name when receiving it from clients, so when it is wrong
	 * here something went really wrong. In the best case the packet got malformed on its
	 * way too us, in the worst case the server is broken or compromised. */
	if (!NetworkIsValidClientName(name)) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	ci = NetworkClientInfo::GetByClientID(client_id);
	if (ci != nullptr) {
		if (playas == ci->client_playas && name != ci->client_name) {
			/* Client name changed, display the change */
			NetworkTextMessage(NETWORK_ACTION_NAME_CHANGE, CC_DEFAULT, false, ci->client_name, name);
		} else if (playas != ci->client_playas) {
			/* The client changed from client-player..
			 * Do not display that for now */
		}

		/* Make sure we're in the company the server tells us to be in,
		 * for the rare case that we get moved while joining. */
		if (client_id == _network_own_client_id) SetLocalCompany(!Company::IsValidID(playas) ? COMPANY_SPECTATOR : playas);

		ci->client_playas = playas;
		ci->client_name = std::move(name);
		ci->public_key = std::move(public_key);

		InvalidateWindowData(WC_CLIENT_LIST, 0);

		return NETWORK_RECV_STATUS_OKAY;
	}

	/* There are at most as many ClientInfo as ClientSocket objects in a
	 * server. Having more info than a server can have means something
	 * has gone wrong somewhere, i.e. the server has more info than it
	 * has actual clients. That means the server is feeding us an invalid
	 * state. So, bail out! This server is broken. */
	if (!NetworkClientInfo::CanAllocateItem()) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	/* We don't have this client_id yet, find an empty client_id, and put the data there */
	ci = new NetworkClientInfo(client_id);
	ci->client_playas = playas;
	if (client_id == _network_own_client_id) this->SetInfo(ci);

	ci->client_name = std::move(name);
	ci->public_key = std::move(public_key);

	InvalidateWindowData(WC_CLIENT_LIST, 0);

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_ERROR(Packet &p)
{
	std::string scratch;
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_error(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p, &scratch));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_CHECK_NEWGRFS(Packet &p)
{
	GRFIdentifier scratch;
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_grfs(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p, &scratch));
}

class ClientGamePasswordRequestHandler : public NetworkAuthenticationPasswordRequestHandler {
	virtual void SendResponse() override { MyClient::SendAuthResponse(); }
	virtual void AskUserForPassword(std::shared_ptr<NetworkAuthenticationPasswordRequest> request) override
	{
		if (!_network_join.server_password.empty()) {
			request->Reply(_network_join.server_password);
		} else {
			ShowNetworkNeedPassword(std::move(request));
		}
	}
};

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_AUTH_REQUEST(Packet &p)
{
	if (this->status != STATUS_JOIN && this->status != STATUS_AUTH_GAME) return NETWORK_RECV_STATUS_MALFORMED_PACKET;
	Debug(net, 9, "Client::status = AUTH_GAME");
	this->status = STATUS_AUTH_GAME;

	Debug(net, 9, "Client::Receive_SERVER_AUTH_REQUEST()");

	if (this->authentication_handler == nullptr) {
		this->authentication_handler = NetworkAuthenticationClientHandler::Create(std::make_shared<ClientGamePasswordRequestHandler>(),
				_settings_client.network.client_secret_key, _settings_client.network.client_public_key);
	}
	switch (this->authentication_handler->ReceiveRequest(p)) {
		case NetworkAuthenticationClientHandler::RequestResult::ReadyForResponse:
			return SendAuthResponse();

		case NetworkAuthenticationClientHandler::RequestResult::AwaitUserInput:
			return NETWORK_RECV_STATUS_OKAY;

		case NetworkAuthenticationClientHandler::RequestResult::Invalid:
		default:
			return NETWORK_RECV_STATUS_MALFORMED_PACKET;
	}
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_ENABLE_ENCRYPTION(Packet &p)
{
	if (this->status != STATUS_AUTH_GAME || this->authentication_handler == nullptr) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	Debug(net, 9, "Client::Receive_SERVER_ENABLE_ENCRYPTION()");

	if (!this->authentication_handler->ReceiveEnableEncryption(p)) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	this->receive_encryption_handler = this->authentication_handler->CreateServerToClientEncryptionHandler();
	this->send_encryption_handler = this->authentication_handler->CreateClientToServerEncryptionHandler();
	this->authentication_handler = nullptr;

	Debug(net, 9, "Client::status = ENCRYPTED");
	this->status = STATUS_ENCRYPTED;

	return this->SendIdentify();
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_WELCOME(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_welcome(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_WAIT(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_wait(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_MAP_BEGIN(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_map_begin(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_MAP_SIZE(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_map_size(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_MAP_DATA(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_map_data(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_MAP_DONE(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_map_done(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this)));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_FRAME(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_frame(&GetRustNetworkFrameServices(), this->status, this, &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_SYNC(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_receive_sync(&GetRustNetworkFrameServices(), this->status, &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_COMMAND(Packet &p)
{
	if (this->status != STATUS_ACTIVE) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	CommandPacket cp;
	auto err = this->ReceiveCommand(p, cp);
	cp.frame    = p.Recv_uint32();
	cp.my_cmd   = p.Recv_bool();

	Debug(net, 9, "Client::Receive_SERVER_COMMAND(): cmd={}, frame={}", cp.cmd, cp.frame);

	if (err.has_value()) {
		IConsolePrint(CC_WARNING, "Dropping server connection due to {}.", *err);
		return NETWORK_RECV_STATUS_MALFORMED_PACKET;
	}

	this->incoming_queue.Push(std::move(cp));

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_CHAT(Packet &p)
{
	if (this->status != STATUS_ACTIVE) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	std::string name;
	const NetworkClientInfo *ci = nullptr, *ci_to;

	NetworkAction action = (NetworkAction)p.Recv_uint8();
	ClientID client_id = (ClientID)p.Recv_uint32();
	bool self_send = p.Recv_bool();
	std::string msg = p.Recv_string(NETWORK_CHAT_LENGTH);
	int64_t data = p.Recv_uint64();

	Debug(net, 9, "Client::Receive_SERVER_CHAT(): action={}, client_id={}, self_send={}", action, client_id, self_send);

	ci_to = NetworkClientInfo::GetByClientID(client_id);
	if (ci_to == nullptr) return NETWORK_RECV_STATUS_OKAY;

	/* Did we initiate the action locally? */
	if (self_send) {
		switch (action) {
			case NETWORK_ACTION_CHAT_CLIENT:
				/* For speaking to client we need the client-name */
				name = ci_to->client_name;
				ci = NetworkClientInfo::GetByClientID(_network_own_client_id);
				break;

			/* For speaking to company, we need the company-name */
			case NETWORK_ACTION_CHAT_COMPANY: {
				StringID str = Company::IsValidID(ci_to->client_playas) ? STR_COMPANY_NAME : STR_NETWORK_SPECTATORS;

				name = GetString(str, ci_to->client_playas);
				ci = NetworkClientInfo::GetByClientID(_network_own_client_id);
				break;
			}

			default: return NETWORK_RECV_STATUS_MALFORMED_PACKET;
		}
	} else {
		/* Display message from somebody else */
		name = ci_to->client_name;
		ci = ci_to;
	}

	if (ci != nullptr) {
		NetworkTextMessage(action, GetDrawStringCompanyColour(ci->client_playas), self_send, name, msg, data);
	}
	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_EXTERNAL_CHAT(Packet &p)
{
	if (this->status != STATUS_ACTIVE) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	std::string source = p.Recv_string(NETWORK_CHAT_LENGTH);
	TextColour colour = (TextColour)p.Recv_uint16();
	std::string user = p.Recv_string(NETWORK_CHAT_LENGTH);
	std::string msg = p.Recv_string(NETWORK_CHAT_LENGTH);

	Debug(net, 9, "Client::Receive_SERVER_EXTERNAL_CHAT(): source={}", source);

	if (!IsValidConsoleColour(colour)) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	NetworkTextMessage(NETWORK_ACTION_EXTERNAL_CHAT, colour, false, user, msg, source);

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_ERROR_QUIT(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_error_quit(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_QUIT(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_quit(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_JOIN(Packet &p)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_join(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this), &p));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_SHUTDOWN(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_shutdown(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this)));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_NEWGAME(Packet &)
{
	return static_cast<NetworkRecvStatus>(openttd_rust_network_client_receive_newgame(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this)));
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_RCON(Packet &p)
{
	if (this->status < STATUS_AUTHORIZED) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	Debug(net, 9, "Client::Receive_SERVER_RCON()");

	TextColour colour_code = (TextColour)p.Recv_uint16();
	if (!IsValidConsoleColour(colour_code)) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	std::string rcon_out = p.Recv_string(NETWORK_RCONCOMMAND_LENGTH);

	IConsolePrint(colour_code, rcon_out);

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_MOVE(Packet &p)
{
	if (this->status < STATUS_AUTHORIZED) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	/* Nothing more in this packet... */
	ClientID client_id   = (ClientID)p.Recv_uint32();
	CompanyID company_id = (CompanyID)p.Recv_uint8();

	Debug(net, 9, "Client::Receive_SERVER_MOVE(): client_id={}, company_id={}", client_id, company_id);

	if (client_id == 0) {
		/* definitely an invalid client id, debug message and do nothing. */
		Debug(net, 1, "Received invalid client index = 0");
		return NETWORK_RECV_STATUS_MALFORMED_PACKET;
	}

	const NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(client_id);
	/* Just make sure we do not try to use a client_index that does not exist */
	if (ci == nullptr) return NETWORK_RECV_STATUS_OKAY;

	/* if not valid player, force spectator, else check player exists */
	if (!Company::IsValidID(company_id)) company_id = COMPANY_SPECTATOR;

	if (client_id == _network_own_client_id) {
		SetLocalCompany(company_id);
	}

	return NETWORK_RECV_STATUS_OKAY;
}

NetworkRecvStatus ClientNetworkGameSocketHandler::Receive_SERVER_CONFIG_UPDATE(Packet &p)
{
	if (this->status < STATUS_ACTIVE) return NETWORK_RECV_STATUS_MALFORMED_PACKET;

	_network_server_max_companies = p.Recv_uint8();
	_network_server_name = p.Recv_string(NETWORK_NAME_LENGTH);

	InvalidateWindowData(WC_CLIENT_LIST, 0);

	Debug(net, 9, "Client::Receive_SERVER_CONFIG_UPDATE(): max_companies={}", _network_server_max_companies);

	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * Check the connection's state, i.e. is the connection still up?
 */
void ClientNetworkGameSocketHandler::CheckConnection()
{
	openttd_rust_network_client_check_connection(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices(), static_cast<NetworkGameSocketHandler *>(this));
}


/** Is called after a client is connected to the server */
void NetworkClient_Connected()
{
	openttd_rust_network_client_connected(&GetRustNetworkClientServices(), &GetRustNetworkFrameServices());
}

/**
 * Send a remote console command.
 * @param password The password.
 * @param command The command to execute.
 */
void NetworkClientSendRcon(std::string_view password, std::string_view command)
{
	MyClient::SendRCon(password, command);
}

/**
 * Notify the server of this client wanting to be moved to another company.
 * @param company_id id of the company the client wishes to be moved to.
 */
void NetworkClientRequestMove(CompanyID company_id)
{
	MyClient::SendMove(company_id);
}

/**
 * Move the clients of a company to the spectators.
 * @param cid The company to move the clients of.
 */
void NetworkClientsToSpectators(CompanyID cid)
{
	Backup<CompanyID> cur_company(_current_company);
	/* If our company is changing owner, go to spectators */
	if (cid == _local_company) SetLocalCompany(COMPANY_SPECTATOR);

	for (NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_playas != cid) continue;
		NetworkTextMessage(NETWORK_ACTION_COMPANY_SPECTATOR, CC_DEFAULT, false, ci->client_name);
		ci->client_playas = COMPANY_SPECTATOR;
	}

	cur_company.Restore();
}

/**
 * Check whether the given client name is deemed valid for use in network games.
 * An empty name (null or '') is not valid as that is essentially no name at all.
 * A name starting with white space is not valid for tab completion purposes.
 * @param client_name The client name to check for validity.
 * @return True iff the name is valid.
 */
bool NetworkIsValidClientName(std::string_view client_name)
{
	if (client_name.empty()) return false;
	if (client_name[0] == ' ') return false;
	return true;
}

/**
 * Trim the given client name in place, i.e. remove leading and trailing spaces.
 * After the trim check whether the client name is valid. A client name is valid
 * whenever the name is not empty and does not start with spaces. This check is
 * done via \c NetworkIsValidClientName.
 * When the client name is valid, this function returns true.
 * When the client name is not valid a GUI error message is shown telling the
 * user to set the client name and this function returns false.
 *
 * This function is not suitable for ensuring a valid client name at the server
 * as the error message will then be shown to the host instead of the client.
 * @param client_name The client name to validate. It will be trimmed of leading
 *                    and trailing spaces.
 * @return True iff the client name is valid.
 */
bool NetworkValidateClientName(std::string &client_name)
{
	StrTrimInPlace(client_name);
	if (NetworkIsValidClientName(client_name)) return true;

	ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_BAD_PLAYER_NAME), {}, WL_ERROR);
	return false;
}

/**
 * Convenience method for NetworkValidateClientName on _settings_client.network.client_name.
 * It trims the client name and checks whether it is empty. When it is empty
 * an error message is shown to the GUI user.
 * See \c NetworkValidateClientName(std::string&) for details about the functionality.
 * @return True iff the client name is valid.
 */
bool NetworkValidateOurClientName()
{
	return NetworkValidateClientName(_settings_client.network.client_name);
}

/**
 * Send the server our name as callback from the setting.
 * @param newname The new client name.
 */
void NetworkUpdateClientName(const std::string &client_name)
{
	NetworkClientInfo *ci = NetworkClientInfo::GetByClientID(_network_own_client_id);
	if (ci == nullptr) return;

	/* Don't change the name if it is the same as the old name */
	if (client_name == ci->client_name) return;

	if (!_network_server) {
		MyClient::SendSetName(client_name);
	} else {
		/* Copy to a temporary buffer so no #n gets added after our name in the settings when there are duplicate names. */
		std::string temporary_name = client_name;
		if (NetworkMakeClientNameUnique(temporary_name)) {
			NetworkTextMessage(NETWORK_ACTION_NAME_CHANGE, CC_DEFAULT, false, ci->client_name, temporary_name);
			ci->client_name = std::move(temporary_name);
			NetworkUpdateClientInfo(CLIENT_ID_SERVER);
		}
	}
}

/**
 * Send a chat message.
 * @param action The action associated with the message.
 * @param type The destination type.
 * @param dest The destination index, be it a company index or client id.
 * @param msg The actual message.
 * @param data Arbitrary extra data.
 */
void NetworkClientSendChat(NetworkAction action, DestType type, int dest, std::string_view msg, int64_t data)
{
	MyClient::SendChat(action, type, dest, msg, data);
}

/**
 * Tell whether the client has team members who they can chat to.
 * @param cio client to check members of.
 * @return true if there is at least one team member.
 */
bool NetworkClientPreferTeamChat(const NetworkClientInfo *cio)
{
	/* Only companies actually playing can speak to team. Eg spectators cannot */
	if (!_settings_client.gui.prefer_teamchat || !Company::IsValidID(cio->client_playas)) return false;

	for (const NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_playas == cio->client_playas && ci != cio) return true;
	}

	return false;
}

/**
 * Get the maximum number of companies that are allowed by the server.
 * @return The number of companies allowed.
 */
uint NetworkMaxCompaniesAllowed()
{
	return _network_server ? _settings_client.network.max_companies : _network_server_max_companies;
}

/**
 * Check if max_companies has been reached on the server (local check only).
 * @return true if the max value has been reached or exceeded, false otherwise.
 */
bool NetworkMaxCompaniesReached()
{
	return Company::GetNumItems() >= NetworkMaxCompaniesAllowed();
}

static ClientNetworkGameSocketHandler *ClientFromGame(void *game) noexcept
{
	return static_cast<ClientNetworkGameSocketHandler *>(static_cast<NetworkGameSocketHandler *>(game));
}
const OpenTTDNetworkClientServices &GetRustNetworkClientServices() noexcept
{
	static_assert(ClientNetworkGameSocketHandler::STATUS_JOIN == 1 && ClientNetworkGameSocketHandler::STATUS_ENCRYPTED == 3);
	static_assert(ClientNetworkGameSocketHandler::STATUS_AUTHORIZED == 5 && ClientNetworkGameSocketHandler::STATUS_MAP_WAIT == 6);
	static_assert(ClientNetworkGameSocketHandler::STATUS_MAP == 7 && ClientNetworkGameSocketHandler::STATUS_ACTIVE == 8);
	static_assert(NETWORK_JOIN_STATUS_CONNECTING == 0 && NETWORK_JOIN_STATUS_AUTHORIZING == 1 && NETWORK_JOIN_STATUS_WAITING == 2);
	static_assert(NETWORK_JOIN_STATUS_DOWNLOADING == 3 && NETWORK_JOIN_STATUS_PROCESSING == 4 && NETWORK_JOIN_STATUS_REGISTERING == 5);
	static_assert(NETWORK_CHAT_LENGTH == 900);
	static_assert(PACKET_CLIENT_JOIN == 2 && PACKET_CLIENT_IDENTIFY == 13 && PACKET_CLIENT_NEWGRFS_CHECKED == 15);
	static_assert(PACKET_CLIENT_GETMAP == 18 && PACKET_CLIENT_MAP_OK == 24 && PACKET_CLIENT_ERROR == 42 && PACKET_CLIENT_QUIT == 40);
	static_assert(NETWORK_ERROR_GENERAL == 0 && NETWORK_ERROR_DESYNC == 1 && NETWORK_ERROR_SAVEGAME_FAILED == 2 && NETWORK_ERROR_CONNECTION_LOST == 3 && NETWORK_ERROR_NEWGRF_MISMATCH == 5);
	static_assert(NETWORK_RECV_STATUS_DESYNC == 1 && NETWORK_RECV_STATUS_NEWGRF_MISMATCH == 2 && NETWORK_RECV_STATUS_SAVEGAME == 3);
	static_assert(NETWORK_RECV_STATUS_SERVER_FULL == 7 && NETWORK_RECV_STATUS_SERVER_BANNED == 8 && NETWORK_RECV_STATUS_CLOSE_QUERY == 9);

	static const OpenTTDNetworkClientServices services{
		.check_grf = [](void *packet, void *identifier) noexcept -> bool {
			auto &c = *static_cast<GRFIdentifier *>(identifier);
			c = GRFIdentifier{};
			DeserializeGRFIdentifier(*static_cast<Packet *>(packet), c);
			const GRFConfig *f = FindGRFConfig(c.grfid, FGCM_EXACT, &c.md5sum);
			if (f != nullptr) return true;
			Debug(grf, 0, "NewGRF {:08X} not found; checksum {}", std::byteswap(c.grfid), FormatArrayAsHex(c.md5sum));
			return false;
		},
		.client_list_changed = []() noexcept { InvalidateWindowData(WC_CLIENT_LIST, 0); },
		.client_name = RustNetworkClientName,
		.debug_banned = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_BANNED()"); },
		.debug_close = [](void *game) noexcept { Debug(net, 3, "Closed client connection {}", static_cast<NetworkGameSocketHandler *>(game)->client_id); },
		.debug_connected = []() noexcept { Debug(net, 9, "Client::NetworkClient_Connected()"); },
		.debug_error = [](uint8_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_ERROR(): error={}", value); },
		.debug_error_quit = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_ERROR_QUIT(): client_id={}", value); },
		.debug_full = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_FULL()"); },
		.debug_grfs = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_CHECK_NEWGRFS(): grf_count={}", value); },
		.debug_join = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_JOIN(): client_id={}", value); },
		.debug_join_authorizing = []() noexcept { Debug(net, 9, "Client::join_status = AUTHORIZING"); },
		.debug_join_downloading = []() noexcept { Debug(net, 9, "Client::join_status = DOWNLOADING"); },
		.debug_join_processing = []() noexcept { Debug(net, 9, "Client::join_status = PROCESSING"); },
		.debug_join_registering = []() noexcept { Debug(net, 9, "Client::join_status = REGISTERING"); },
		.debug_join_waiting = []() noexcept { Debug(net, 9, "Client::join_status = WAITING"); },
		.debug_map_begin = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_MAP_BEGIN(): frame_counter={}", value); },
		.debug_map_done = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_MAP_DONE()"); },
		.debug_map_size = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_MAP_SIZE(): bytes_total={}", value); },
		.debug_newgame = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_NEWGAME()"); },
		.debug_quit = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_QUIT(): client_id={}", value); },
		.debug_send_error = [](uint8_t value) noexcept { Debug(net, 9, "Client::SendError(): errorno={}", value); },
		.debug_send_getmap = []() noexcept { Debug(net, 9, "Client::SendGetMap()"); },
		.debug_send_grfs_ok = []() noexcept { Debug(net, 9, "Client::SendNewGRFsOk()"); },
		.debug_send_identify = []() noexcept { Debug(net, 9, "Client::SendIdentify()"); },
		.debug_send_join = []() noexcept { Debug(net, 9, "Client::SendJoin()"); },
		.debug_send_map_ok = []() noexcept { Debug(net, 9, "Client::SendMapOk()"); },
		.debug_send_quit = []() noexcept { Debug(net, 9, "Client::SendQuit()"); },
		.debug_shutdown = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_SHUTDOWN()"); },
		.debug_status_active = []() noexcept { Debug(net, 9, "Client::status = ACTIVE"); },
		.debug_status_authorized = []() noexcept { Debug(net, 9, "Client::status = AUTHORIZED"); },
		.debug_status_join = []() noexcept { Debug(net, 9, "Client::status = JOIN"); },
		.debug_status_map = []() noexcept { Debug(net, 9, "Client::status = MAP"); },
		.debug_status_map_wait = []() noexcept { Debug(net, 9, "Client::status = MAP_WAIT"); },
		.debug_wait = []() noexcept { Debug(net, 9, "Client::Receive_SERVER_WAIT()"); },
		.debug_welcome = [](uint32_t value) noexcept { Debug(net, 9, "Client::Receive_SERVER_WELCOME(): client_id={}", value); },
		.delete_info = [](void *info) noexcept { delete static_cast<NetworkClientInfo *>(info); },
		.dirty_join = []() noexcept { SetWindowDirty(WC_NETWORK_STATUS_WINDOW, WN_NETWORK_STATUS_WINDOW_JOIN); },
		.emergency_save = []() noexcept { ClientNetworkEmergencySave(); },
		.enter_multiplayer = []() noexcept { SocialIntegration::EventEnterMultiplayer(Map::SizeX(), Map::SizeY()); },
		.join = [](void *info) noexcept { NetworkTextMessage(NETWORK_ACTION_JOIN, CC_DEFAULT, false, static_cast<NetworkClientInfo *>(info)->client_name); },
		.join_bytes = &_network_join_bytes,
		.join_bytes_total = &_network_join_bytes_total,
		.join_company = reinterpret_cast<const uint8_t *>(&_network_join.company),
		.join_status = reinterpret_cast<uint8_t *>(&_network_join_status),
		.join_waiting = &_network_join_waiting,
		.leave = [](void *info) noexcept { NetworkTextMessage(NETWORK_ACTION_LEAVE, CC_DEFAULT, false, static_cast<NetworkClientInfo *>(info)->client_name, "", STR_NETWORK_MESSAGE_CLIENT_LEAVING); },
		.leave_error = [](void *info, uint8_t error) noexcept { NetworkTextMessage(NETWORK_ACTION_LEAVE, CC_DEFAULT, false, static_cast<NetworkClientInfo *>(info)->client_name, "", GetNetworkErrorMsg(static_cast<NetworkErrorCode>(error))); },
		.map_add_packet = [](void *game, void *packet) noexcept -> uint32_t { auto *cs = ClientFromGame(game); cs->savegame->AddPacket(*static_cast<Packet *>(packet)); return static_cast<uint32_t>(cs->savegame->buffer.size()); },
		.map_create = [](void *game) noexcept { ClientFromGame(game)->savegame = std::make_shared<PacketReader>(); },
		.map_exists = [](void *game) noexcept -> bool { return ClientFromGame(game)->savegame != nullptr; },
		.map_load = [](void *game) noexcept -> bool {
			auto *cs = ClientFromGame(game);
			cs->savegame->Reset();
			ClearErrorMessages();
			_file_to_saveload.SetMode(FIOS_TYPE_FILE, SLO_LOAD);
			bool success = SafeLoad({}, SLO_LOAD, DFT_GAME_FILE, GM_NORMAL, NO_DIRECTORY, cs->savegame);
			cs->savegame = nullptr;
			return success;
		},
		.map_loaded_stats = []() noexcept { _game_session_stats.start_time = std::chrono::steady_clock::now(); _game_session_stats.savegame_size = std::nullopt; },
		.new_company = [](uint32_t client_id) noexcept { Command<CMD_COMPANY_CTRL>::Post(CCA_NEW, CompanyID::Invalid(), CRR_NONE, static_cast<ClientID>(client_id)); },
		.own_client_id = reinterpret_cast<uint32_t *>(&_network_own_client_id),
		.reconnect = &_network_reconnect,
		.revision = []() noexcept -> OpenTTDByteSpan { auto value = GetNetworkRevisionString(); return {reinterpret_cast<const uint8_t *>(value.data()), value.size()}; },
		.show_banned = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_SERVER_BANNED), {}, WL_CRITICAL); },
		.show_client_list = []() noexcept { ShowClientList(); },
		.show_full = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_SERVER_FULL), {}, WL_CRITICAL); },
		.show_join_window = []() noexcept { ShowJoinStatusWindow(); },
		.show_lag = [](int64_t seconds) noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_CLIENT_GUI_LOST_CONNECTION_CAPTION), GetEncodedString(STR_NETWORK_ERROR_CLIENT_GUI_LOST_CONNECTION, seconds), WL_INFO); },
		.show_newgrf_mismatch = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_NEWGRF_MISMATCH), {}, WL_CRITICAL); },
		.show_reboot = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_MESSAGE_SERVER_REBOOT), {}, WL_CRITICAL); },
		.show_savegame_error = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_SAVEGAMEERROR), {}, WL_CRITICAL); },
		.show_server_error = [](uint8_t error, void *reason) noexcept {
			static const StringID network_error_strings[] = {
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_GENERAL
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_DESYNC
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_SAVEGAME_FAILED
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_CONNECTION_LOST
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_ILLEGAL_PACKET
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_NEWGRF_MISMATCH
				STR_NETWORK_ERROR_SERVER_ERROR,        // NETWORK_ERROR_NOT_AUTHORIZED
				STR_NETWORK_ERROR_SERVER_ERROR,        // NETWORK_ERROR_NOT_EXPECTED
				STR_NETWORK_ERROR_WRONG_REVISION,      // NETWORK_ERROR_WRONG_REVISION
				STR_NETWORK_ERROR_LOSTCONNECTION,      // NETWORK_ERROR_NAME_IN_USE
				STR_NETWORK_ERROR_WRONG_PASSWORD,      // NETWORK_ERROR_WRONG_PASSWORD
				STR_NETWORK_ERROR_SERVER_ERROR,        // NETWORK_ERROR_COMPANY_MISMATCH
				STR_NETWORK_ERROR_KICKED,              // NETWORK_ERROR_KICKED
				STR_NETWORK_ERROR_CHEATER,             // NETWORK_ERROR_CHEATER
				STR_NETWORK_ERROR_SERVER_FULL,         // NETWORK_ERROR_FULL
				STR_NETWORK_ERROR_TOO_MANY_COMMANDS,   // NETWORK_ERROR_TOO_MANY_COMMANDS
				STR_NETWORK_ERROR_TIMEOUT_PASSWORD,    // NETWORK_ERROR_TIMEOUT_PASSWORD
				STR_NETWORK_ERROR_TIMEOUT_COMPUTER,    // NETWORK_ERROR_TIMEOUT_COMPUTER
				STR_NETWORK_ERROR_TIMEOUT_MAP,         // NETWORK_ERROR_TIMEOUT_MAP
				STR_NETWORK_ERROR_TIMEOUT_JOIN,        // NETWORK_ERROR_TIMEOUT_JOIN
				STR_NETWORK_ERROR_INVALID_CLIENT_NAME, // NETWORK_ERROR_INVALID_CLIENT_NAME
				STR_NETWORK_ERROR_NOT_ON_ALLOW_LIST,   // NETWORK_ERROR_NOT_ON_ALLOW_LIST
				STR_NETWORK_ERROR_SERVER_ERROR,        // NETWORK_ERROR_NO_AUTHENTICATION_METHOD_AVAILABLE
			};
			static_assert(lengthof(network_error_strings) == NETWORK_ERROR_END);

			StringID err = STR_NETWORK_ERROR_LOSTCONNECTION;
			if (error < lengthof(network_error_strings)) err = network_error_strings[error];
			if (reason != nullptr) {
				ShowErrorMessage(GetEncodedString(err), GetEncodedString(STR_NETWORK_ERROR_KICK_MESSAGE, *static_cast<std::string *>(reason)), WL_CRITICAL);
			} else {
				ShowErrorMessage(GetEncodedString(err), {}, WL_CRITICAL);
			}
		},
		.show_shutdown = []() noexcept { ShowErrorMessage(GetEncodedString(STR_NETWORK_MESSAGE_SERVER_SHUTDOWN), {}, WL_CRITICAL); },
		.status = [](void *game) noexcept -> uint8_t * { return reinterpret_cast<uint8_t *>(&ClientFromGame(game)->status); },
		.unknown_quit = [](uint32_t client_id) noexcept { Debug(net, 1, "Unknown client ({}) is leaving the game", client_id); },
		.wait_for_quit_packets = []() noexcept { CSleep(3 * MILLISECONDS_PER_TICK); },
	};
	return services;
}
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkClientServices.hpp"
bool CheckAbi_network_client() { return AbiServicesComplete(GetRustNetworkClientServices()); }

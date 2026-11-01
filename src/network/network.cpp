/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network.cpp Base functions for networking support. */

#include "../stdafx.h"

#include "../strings_func.h"
#include "../command_func.h"
#include "../timer/timer_game_tick.h"
#include "../timer/timer_game_economy.h"
#include "network_admin.h"
#include "network_client.h"
#include "network_query.h"
#include "network_server.h"
#include "network_content.h"
#include "network_udp.h"
#include "network_gamelist.h"
#include "network_base.h"
#include "network_coordinator.h"
#include "core/udp.h"
#include "core/host.h"
#include "network_gui.h"
#include "../console_func.h"
#include "../3rdparty/md5/md5.h"
#include "../core/random_func.hpp"
#include "../window_func.h"
#include "../company_func.h"
#include "../company_base.h"
#include "../landscape_type.h"
#include "../rev.h"
#include "../core/pool_func.hpp"
#include "../gfx_func.h"
#include "../error.h"
#include "../misc_cmd.h"
#include "../core/string_builder.hpp"
#include "../rust/session_native.hpp"
#include "../rust/settings_services.h"
#ifdef DEBUG_DUMP_COMMANDS
#	include "../fileio_func.h"
#	include "../core/string_consumer.hpp"
#endif
#include <charconv>

#include "table/strings.h"

#include "../safeguards.h"

#ifdef DEBUG_DUMP_COMMANDS
/** Helper variable to make the dedicated server go fast until the (first) join.
 * Used to load the desync debug logs, i.e. for reproducing a desync.
 * There's basically no need to ever enable this, unless you really know what
 * you are doing, i.e. debugging a desync.
 * See docs/desync.md for details. */
bool _ddc_fastforward = true;
#endif /* DEBUG_DUMP_COMMANDS */

/** Make sure both pools have the same size. */
static_assert(NetworkClientInfoPool::MAX_SIZE == NetworkClientSocketPool::MAX_SIZE);

/** The pool with client information. */
NetworkClientInfoPool _networkclientinfo_pool("NetworkClientInfo");
INSTANTIATE_POOL_METHODS(NetworkClientInfo)

bool _networking;         ///< are we in networking mode?
bool _network_server;     ///< network-server is active
bool _network_available;  ///< is network mode available?
bool _network_dedicated;  ///< are we a dedicated server?
bool _is_network_server;  ///< Does this client wants to be a network-server?
ClientID _network_own_client_id;      ///< Our client identifier.
ClientID _redirect_console_to_client; ///< If not invalid, redirect the console output to a client.
uint8_t _network_reconnect;             ///< Reconnect timeout
StringList _network_bind_list;        ///< The addresses to bind on.
StringList _network_host_list;        ///< The servers we know.
StringList _network_ban_list;         ///< The banned clients.
NetworkAddressList _broadcast_list;   ///< List of broadcast addresses.

/** The amount of clients connected */
uint8_t _network_clients_connected = 0;

extern std::string GenerateUid(std::string_view subject);

/**
 * Return whether there is any client connected or trying to connect at all.
 * @return whether we have any client activity
 */
bool HasClients()
{
	return !NetworkClientSocket::Iterate().empty();
}

/**
 * Basically a client is leaving us right now.
 */
NetworkClientInfo::~NetworkClientInfo()
{
	/* Delete the chat window, if you were chatting with this client. */
	InvalidateWindowData(WC_SEND_NETWORK_MSG, DESTTYPE_CLIENT, this->client_id);
}

/**
 * Return the CI given it's client-identifier
 * @param client_id the ClientID to search for
 * @return return a pointer to the corresponding NetworkClientInfo struct or nullptr when not found
 */
/* static */ NetworkClientInfo *NetworkClientInfo::GetByClientID(ClientID client_id)
{
	for (NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_id == client_id) return ci;
	}

	return nullptr;
}

/**
 * Returns whether the given company can be joined by this client.
 * @param company_id The id of the company.
 * @return \c true when this company is allowed to join, otherwise \c false.
 */
bool NetworkClientInfo::CanJoinCompany(CompanyID company_id) const
{
	Company *c = Company::GetIfValid(company_id);
	return c != nullptr && c->allow_list.Contains(this->public_key);
}

/**
 * Returns whether the given company can be joined by this client.
 * @param company_id The id of the company.
 * @return \c true when this company is allowed to join, otherwise \c false.
 */
bool NetworkCanJoinCompany(CompanyID company_id)
{
	NetworkClientInfo *info = NetworkClientInfo::GetByClientID(_network_own_client_id);
	return info != nullptr && info->CanJoinCompany(company_id);
}

/**
 * Return the client state given it's client-identifier
 * @param client_id the ClientID to search for
 * @return return a pointer to the corresponding NetworkClientSocket struct or nullptr when not found
 */
/* static */ ServerNetworkGameSocketHandler *ServerNetworkGameSocketHandler::GetByClientID(ClientID client_id)
{
	for (NetworkClientSocket *cs : NetworkClientSocket::Iterate()) {
		if (cs->client_id == client_id) return cs;
	}

	return nullptr;
}


/**
 * Simple helper to find the location of the given authorized key in the authorized keys.
 * @param authorized_keys The keys to look through.
 * @param authorized_key The key to look for.
 * @return The iterator to the location of the authorized key, or \c authorized_keys.end().
 */
static auto FindKey(auto *authorized_keys, std::string_view authorized_key)
{
	return std::ranges::find_if(*authorized_keys, [authorized_key](auto &value) { return StrEqualsIgnoreCase(value, authorized_key); });
}

/**
 * Check whether the given key is contains in these authorized keys.
 * @param key The key to look for.
 * @return \c true when the key has been found, otherwise \c false.
 */
bool NetworkAuthorizedKeys::Contains(std::string_view key) const
{
	return FindKey(this, key) != this->end();
}

/**
 * Add the given key to the authorized keys, when it is not already contained.
 * @param key The key to add.
 * @return \c true when the key was added, \c false when the key already existed or the key was empty.
 */
bool NetworkAuthorizedKeys::Add(std::string_view key)
{
	if (key.empty()) return false;

	auto iter = FindKey(this, key);
	if (iter != this->end()) return false;

	this->emplace_back(key);
	return true;
}

/**
 * Remove the given key from the authorized keys, when it is exists.
 * @param key The key to remove.
 * @return \c true when the key was removed, \c false when the key did not exist.
 */
bool NetworkAuthorizedKeys::Remove(std::string_view key)
{
	auto iter = FindKey(this, key);
	if (iter == this->end()) return false;

	this->erase(iter);
	return true;
}


uint8_t NetworkSpectatorCount()
{
	uint8_t count = 0;

	for (const NetworkClientInfo *ci : NetworkClientInfo::Iterate()) {
		if (ci->client_playas == COMPANY_SPECTATOR) count++;
	}

	/* Don't count a dedicated server as spectator */
	if (_network_dedicated) count--;

	return count;
}


/* This puts a text-message to the console, or in the future, the chat-box,
 *  (to keep it all a bit more general)
 * If 'self_send' is true, this is the client who is sending the message */
void NetworkTextMessage(NetworkAction action, TextColour colour, bool self_send, std::string_view name, std::string_view str, StringParameter &&data)
{
	std::string message;
	StringBuilder builder(message);

	/* All of these strings start with "***". These characters are interpreted as both left-to-right and
	 * right-to-left characters depending on the context. As the next text might be an user's name, the
	 * user name's characters will influence the direction of the "***" instead of the language setting
	 * of the game. Manually set the direction of the "***" by inserting a text-direction marker. */
	builder.PutUtf8(_current_text_dir == TD_LTR ? CHAR_TD_LRM : CHAR_TD_RLM);

	switch (action) {
		case NETWORK_ACTION_SERVER_MESSAGE:
			/* Ignore invalid messages */
			builder += GetString(STR_NETWORK_SERVER_MESSAGE, str);
			colour = CC_DEFAULT;
			break;
		case NETWORK_ACTION_COMPANY_SPECTATOR:
			colour = CC_DEFAULT;
			builder += GetString(STR_NETWORK_MESSAGE_CLIENT_COMPANY_SPECTATE, name);
			break;
		case NETWORK_ACTION_COMPANY_JOIN:
			colour = CC_DEFAULT;
			builder += GetString(STR_NETWORK_MESSAGE_CLIENT_COMPANY_JOIN, name, str);
			break;
		case NETWORK_ACTION_COMPANY_NEW:
			colour = CC_DEFAULT;
			builder += GetString(STR_NETWORK_MESSAGE_CLIENT_COMPANY_NEW, name, std::move(data));
			break;
		case NETWORK_ACTION_JOIN:
			/* Show the Client ID for the server but not for the client. */
			builder += _network_server ?
					GetString(STR_NETWORK_MESSAGE_CLIENT_JOINED_ID, name, std::move(data)) :
					GetString(STR_NETWORK_MESSAGE_CLIENT_JOINED, name);
			break;
		case NETWORK_ACTION_LEAVE:          builder += GetString(STR_NETWORK_MESSAGE_CLIENT_LEFT, name, std::move(data)); break;
		case NETWORK_ACTION_NAME_CHANGE:    builder += GetString(STR_NETWORK_MESSAGE_NAME_CHANGE, name, str); break;
		case NETWORK_ACTION_GIVE_MONEY:     builder += GetString(STR_NETWORK_MESSAGE_GIVE_MONEY, name, std::move(data), str); break;
		case NETWORK_ACTION_KICKED:         builder += GetString(STR_NETWORK_MESSAGE_KICKED, name, str); break;
		case NETWORK_ACTION_CHAT_COMPANY:   builder += GetString(self_send ? STR_NETWORK_CHAT_TO_COMPANY : STR_NETWORK_CHAT_COMPANY, name, str); break;
		case NETWORK_ACTION_CHAT_CLIENT:    builder += GetString(self_send ? STR_NETWORK_CHAT_TO_CLIENT  : STR_NETWORK_CHAT_CLIENT, name, str);  break;
		case NETWORK_ACTION_EXTERNAL_CHAT:  builder += GetString(STR_NETWORK_CHAT_EXTERNAL, std::move(data), name, str); break;
		default:                            builder += GetString(STR_NETWORK_CHAT_ALL, name, str); break;
	}

	Debug(desync, 1, "msg: {:08x}; {:02x}; {}", TimerGameEconomy::date, TimerGameEconomy::date_fract, message);
	IConsolePrint(colour, message);
	NetworkAddChatMessage(colour, _settings_client.gui.network_chat_timeout, message);
}

/* Calculate the frame-lag of a client */
uint NetworkCalculateLag(const NetworkClientSocket *cs)
{
	return openttd_rust_network_calculate_lag(&GetRustNetworkFrameServices(), cs);
}


/* There was a non-recoverable error, drop back to the main menu with a nice
 *  error */
void ShowNetworkError(StringID error_string)
{
	_switch_mode = SM_MENU;
	ShowErrorMessage(GetEncodedString(error_string), {}, WL_CRITICAL);
}

/**
 * Retrieve the string id of an internal error number
 * @param err NetworkErrorCode
 * @return the StringID
 */
StringID GetNetworkErrorMsg(NetworkErrorCode err)
{
	/* List of possible network errors, used by
	 * PACKET_SERVER_ERROR and PACKET_CLIENT_ERROR */
	static const StringID network_error_strings[] = {
		STR_NETWORK_ERROR_CLIENT_GENERAL,
		STR_NETWORK_ERROR_CLIENT_DESYNC,
		STR_NETWORK_ERROR_CLIENT_SAVEGAME,
		STR_NETWORK_ERROR_CLIENT_CONNECTION_LOST,
		STR_NETWORK_ERROR_CLIENT_PROTOCOL_ERROR,
		STR_NETWORK_ERROR_CLIENT_NEWGRF_MISMATCH,
		STR_NETWORK_ERROR_CLIENT_NOT_AUTHORIZED,
		STR_NETWORK_ERROR_CLIENT_NOT_EXPECTED,
		STR_NETWORK_ERROR_CLIENT_WRONG_REVISION,
		STR_NETWORK_ERROR_CLIENT_NAME_IN_USE,
		STR_NETWORK_ERROR_CLIENT_WRONG_PASSWORD,
		STR_NETWORK_ERROR_CLIENT_COMPANY_MISMATCH,
		STR_NETWORK_ERROR_CLIENT_KICKED,
		STR_NETWORK_ERROR_CLIENT_CHEATER,
		STR_NETWORK_ERROR_CLIENT_SERVER_FULL,
		STR_NETWORK_ERROR_CLIENT_TOO_MANY_COMMANDS,
		STR_NETWORK_ERROR_CLIENT_TIMEOUT_PASSWORD,
		STR_NETWORK_ERROR_CLIENT_TIMEOUT_COMPUTER,
		STR_NETWORK_ERROR_CLIENT_TIMEOUT_MAP,
		STR_NETWORK_ERROR_CLIENT_TIMEOUT_JOIN,
		STR_NETWORK_ERROR_CLIENT_INVALID_CLIENT_NAME,
		STR_NETWORK_ERROR_CLIENT_NOT_ON_ALLOW_LIST,
		STR_NETWORK_ERROR_CLIENT_NO_AUTHENTICATION_METHOD_AVAILABLE,
	};
	static_assert(lengthof(network_error_strings) == NETWORK_ERROR_END);

	if (err >= (ptrdiff_t)lengthof(network_error_strings)) err = NETWORK_ERROR_GENERAL;

	return network_error_strings[err];
}

/**
 * Handle the pause mode change so we send the right messages to the chat.
 * @param prev_mode The previous pause mode.
 * @param changed_mode The pause mode that got changed.
 */
void NetworkHandlePauseChange(PauseModes prev_mode, PauseMode changed_mode)
{
	if (!_networking) return;

	switch (changed_mode) {
		case PauseMode::Normal:
		case PauseMode::Join:
		case PauseMode::GameScript:
		case PauseMode::ActiveClients:
		case PauseMode::LinkGraph: {
			bool changed = _pause_mode.None() != prev_mode.None();
			bool paused = _pause_mode.Any();
			if (!paused && !changed) return;

			std::string str;
			if (!changed) {
				std::array<StringParameter, 5> params{};
				auto it = params.begin();
				if (_pause_mode.Test(PauseMode::Normal))        *it++ = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_MANUAL;
				if (_pause_mode.Test(PauseMode::Join))          *it++ = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_CONNECTING_CLIENTS;
				if (_pause_mode.Test(PauseMode::GameScript))    *it++ = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_GAME_SCRIPT;
				if (_pause_mode.Test(PauseMode::ActiveClients)) *it++ = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_NOT_ENOUGH_PLAYERS;
				if (_pause_mode.Test(PauseMode::LinkGraph))     *it++ = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_LINK_GRAPH;
				str = GetStringWithArgs(STR_NETWORK_SERVER_MESSAGE_GAME_STILL_PAUSED_1 + std::distance(params.begin(), it) - 1, {params.begin(), it});
			} else {
				StringID reason;
				switch (changed_mode) {
					case PauseMode::Normal:        reason = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_MANUAL; break;
					case PauseMode::Join:          reason = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_CONNECTING_CLIENTS; break;
					case PauseMode::GameScript:    reason = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_GAME_SCRIPT; break;
					case PauseMode::ActiveClients: reason = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_NOT_ENOUGH_PLAYERS; break;
					case PauseMode::LinkGraph:     reason = STR_NETWORK_SERVER_MESSAGE_GAME_REASON_LINK_GRAPH; break;
					default: NOT_REACHED();
				}
				str = GetString(paused ? STR_NETWORK_SERVER_MESSAGE_GAME_PAUSED : STR_NETWORK_SERVER_MESSAGE_GAME_UNPAUSED, reason);
			}

			NetworkTextMessage(NETWORK_ACTION_SERVER_MESSAGE, CC_DEFAULT, false, "", str);
			break;
		}

		default:
			return;
	}
}


/**
 * Parse the company part ("#company" postfix) of a connecting string.
 * @param connection_string The string with the connection data.
 * @param company_id        The company ID to set, if available.
 * @return A std::string_view into the connection string without the company part.
 */
std::string_view ParseCompanyFromConnectionString(std::string_view connection_string, CompanyID *company_id)
{
	std::string_view ip = connection_string;
	if (company_id == nullptr) return ip;

	size_t offset = ip.find_last_of('#');
	if (offset != std::string::npos) {
		std::string_view company_string = ip.substr(offset + 1);
		ip = ip.substr(0, offset);

		uint8_t company_value;
		auto [_, err] = std::from_chars(company_string.data(), company_string.data() + company_string.size(), company_value);
		if (err == std::errc()) {
			if (company_value != COMPANY_NEW_COMPANY && company_value != COMPANY_SPECTATOR) {
				if (company_value > MAX_COMPANIES || company_value == 0) {
					*company_id = COMPANY_SPECTATOR;
				} else {
					/* "#1" means the first company, which has index 0. */
					*company_id = (CompanyID)(company_value - 1);
				}
			} else {
				*company_id = (CompanyID)company_value;
			}
		}
	}

	return ip;
}

/**
 * Converts a string to ip/port/company
 *  Format: IP:port#company
 *
 * Returns the IP part as a string view into the passed string. This view is
 * valid as long the passed connection string is valid. If there is no port
 * present in the connection string, the port reference will not be touched.
 * When there is no company ID present in the connection string or company_id
 * is nullptr, then company ID will not be touched.
 *
 * @param connection_string The string with the connection data.
 * @param port              The port reference to set.
 * @param company_id        The company ID to set, if available.
 * @return A std::string_view into the connection string with the (IP) address part.
 */
std::string_view ParseFullConnectionString(std::string_view connection_string, uint16_t &port, CompanyID *company_id)
{
	std::string_view ip = ParseCompanyFromConnectionString(connection_string, company_id);

	size_t port_offset = ip.find_last_of(':');
	size_t ipv6_close = ip.find_last_of(']');
	if (port_offset != std::string::npos && (ipv6_close == std::string::npos || ipv6_close < port_offset)) {
		std::string_view port_string = ip.substr(port_offset + 1);
		ip = ip.substr(0, port_offset);
		std::from_chars(port_string.data(), port_string.data() + port_string.size(), port);
	}
	return ip;
}

/**
 * Normalize a connection string. That is, ensure there is a port in the string.
 * @param connection_string The connection string to normalize.
 * @param default_port The port to use if none is given.
 * @return The normalized connection string.
 */
std::string NormalizeConnectionString(std::string_view connection_string, uint16_t default_port)
{
	uint16_t port = default_port;
	std::string_view ip = ParseFullConnectionString(connection_string, port);
	return fmt::format("{}:{}", ip, port);
}

/**
 * Convert a string containing either "hostname" or "hostname:ip" to a
 * NetworkAddress.
 *
 * @param connection_string The string to parse.
 * @param default_port The default port to set port to if not in connection_string.
 * @return A valid NetworkAddress of the parsed information.
 */
NetworkAddress ParseConnectionString(std::string_view connection_string, uint16_t default_port)
{
	uint16_t port = default_port;
	std::string_view ip = ParseFullConnectionString(connection_string, port);
	return NetworkAddress(ip, port);
}

/**
 * Handle the accepting of a connection to the server.
 * @param s The socket of the new connection.
 * @param address The address of the peer.
 */
/* static */ void ServerNetworkGameSocketHandler::AcceptConnection(SOCKET s, const NetworkAddress &address)
{
	/* Register the login */
	_network_clients_connected++;

	ServerNetworkGameSocketHandler *cs = new ServerNetworkGameSocketHandler(s);
	cs->client_address = address; // Save the IP of the client

	InvalidateWindowData(WC_CLIENT_LIST, 0);
}

/**
 * Resets the pools used for network clients, and the admin pool if needed.
 * @param close_admins Whether the admin pool has to be cleared as well.
 */
static void InitializeNetworkPools(bool close_admins = true)
{
	PoolTypes to_clean{PoolType::NetworkClient};
	if (close_admins) to_clean.Set(PoolType::NetworkAdmin);
	PoolBase::Clean(to_clean);
}


/* Initializes the network (cleans sockets and stuff) */
static void NetworkInitialize(bool close_admins = true)
{
	openttd_rust_network_initialize(&GetRustNetworkLifecycleServices(), &GetRustNetworkFrameServices(), close_admins);
}

/** Non blocking connection to query servers for their game info. */
class TCPQueryConnecter : public TCPServerConnecter {
private:
	std::string connection_string;

public:
	TCPQueryConnecter(std::string_view connection_string) : TCPServerConnecter(connection_string, NETWORK_DEFAULT_PORT), connection_string(connection_string) {}

	void OnFailure() override
	{
		Debug(net, 9, "Query::OnFailure(): connection_string={}", this->connection_string);

		NetworkGame *item = NetworkGameListAddItem(connection_string);
		item->status = NGLS_OFFLINE;
		item->refreshing = false;

		UpdateNetworkGameWindow();
	}

	void OnConnect(SOCKET s) override
	{
		Debug(net, 9, "Query::OnConnect(): connection_string={}", this->connection_string);

		QueryNetworkGameSocketHandler::QueryServer(s, this->connection_string);
	}
};

/**
 * Query a server to fetch the game-info.
 * @param connection_string the address to query.
 */
void NetworkQueryServer(std::string_view connection_string)
{
	if (!_network_available) return;

	Debug(net, 9, "NetworkQueryServer(): connection_string={}", connection_string);

	/* Mark the entry as refreshing, so the GUI can show the refresh is pending. */
	NetworkGame *item = NetworkGameListAddItem(connection_string);
	item->refreshing = true;

	TCPConnecter::Create<TCPQueryConnecter>(connection_string);
}

/**
 * Validates an address entered as a string and adds the server to
 * the list. If you use this function, the games will be marked
 * as manually added.
 * @param connection_string The IP:port of the server to add.
 * @param manually Whether the enter should be marked as manual added.
 * @param never_expire Whether the entry can expire (removed when no longer found in the public listing).
 * @return The entry on the game list.
 */
NetworkGame *NetworkAddServer(std::string_view connection_string, bool manually, bool never_expire)
{
	if (connection_string.empty()) return nullptr;

	/* Ensure the item already exists in the list */
	NetworkGame *item = NetworkGameListAddItem(connection_string);
	if (item->info.server_name.empty()) {
		ClearGRFConfigList(item->info.grfconfig);
		item->info.server_name = connection_string;

		UpdateNetworkGameWindow();

		NetworkQueryServer(connection_string);
	}

	if (manually) item->manually = true;
	if (never_expire) item->version = INT32_MAX;

	return item;
}

/**
 * Get the addresses to bind to.
 * @param addresses the list to write to.
 * @param port the port to bind to.
 */
void GetBindAddresses(NetworkAddressList *addresses, uint16_t port)
{
	for (const auto &iter : _network_bind_list) {
		addresses->emplace_back(iter, port);
	}

	/* No address, so bind to everything. */
	if (addresses->empty()) {
		addresses->emplace_back("", port);
	}
}

/* Generates the list of manually added hosts from NetworkGame and
 * dumps them into the array _network_host_list. This array is needed
 * by the function that generates the config file. */
void NetworkRebuildHostList()
{
	_network_host_list.clear();

	for (const auto &item : _network_game_list) {
		if (item->manually) _network_host_list.emplace_back(item->connection_string);
	}
}

/** Non blocking connection create to actually connect to servers */
class TCPClientConnecter : public TCPServerConnecter {
private:
	std::string connection_string;

public:
	TCPClientConnecter(std::string_view connection_string) : TCPServerConnecter(connection_string, NETWORK_DEFAULT_PORT), connection_string(connection_string) {}

	void OnFailure() override
	{
		Debug(net, 9, "Client::OnFailure(): connection_string={}", this->connection_string);

		ShowNetworkError(STR_NETWORK_ERROR_NOCONNECTION);
	}

	void OnConnect(SOCKET s) override
	{
		Debug(net, 9, "Client::OnConnect(): connection_string={}", this->connection_string);

		_networking = true;
		_network_own_client_id = ClientID{};
		new ClientNetworkGameSocketHandler(s, this->connection_string);
		IConsoleCmdExec("exec scripts/on_client.scr 0");
		NetworkClient_Connected();
	}
};

/**
 * Join a client to the server at with the given connection string.
 * The default for the passwords is \c nullptr. When the server needs a
 * password and none is given, the user is asked to enter the password in the GUI.
 * This function will return false whenever some information required to join is not
 * correct such as the company number or the client's name, or when there is not
 * networking avalabile at all. If the function returns false the connection with
 * the existing server is not disconnected.
 * It will return true when it starts the actual join process, i.e. when it
 * actually shows the join status window.
 *
 * @param connection_string     The IP address, port and company number to join as.
 * @param default_company       The company number to join as when none is given.
 * @param join_server_password  The password for the server.
 * @return Whether the join has started.
 */
bool NetworkClientConnectGame(std::string_view connection_string, CompanyID default_company, const std::string &join_server_password)
{
	std::string resolved;
	return openttd_rust_network_connect_game(&GetRustNetworkLifecycleServices(), &GetRustNetworkFrameServices(), &GetRustSessionServices(), {reinterpret_cast<const uint8_t *>(connection_string.data()), connection_string.size()}, default_company.base(), &join_server_password, &resolved);
}


/**
 * Trim the given server name in place, i.e. remove leading and trailing spaces.
 * After the trim check whether the server name is not empty.
 * When the server name is empty a GUI error message is shown telling the
 * user to set the servername and this function returns false.
 *
 * @param server_name The server name to validate. It will be trimmed of leading
 *                    and trailing spaces.
 * @return True iff the server name is valid.
 */
bool NetworkValidateServerName(std::string &server_name)
{
	StrTrimInPlace(server_name);
	if (!server_name.empty()) return true;

	ShowErrorMessage(GetEncodedString(STR_NETWORK_ERROR_BAD_SERVER_NAME), {}, WL_ERROR);
	return false;
}

/**
 * Check whether the client and server name are set, for a dedicated server and if not set them to some default
 * value and tell the user to change this as soon as possible.
 * If the saved name is the default value, then the user is told to override  this value too.
 * This is only meant dedicated servers, as for the other servers the GUI ensures a name has been entered.
 */
static void CheckClientAndServerName()
{
	static const std::string fallback_client_name = "Unnamed Client";
	StrTrimInPlace(_settings_client.network.client_name);
	if (_settings_client.network.client_name.empty() || _settings_client.network.client_name == fallback_client_name) {
		Debug(net, 1, "No \"client_name\" has been set, using \"{}\" instead. Please set this now using the \"name <new name>\" command", fallback_client_name);
		_settings_client.network.client_name = fallback_client_name;
	}

	static const std::string fallback_server_name = "Unnamed Server";
	StrTrimInPlace(_settings_client.network.server_name);
	if (_settings_client.network.server_name.empty() || _settings_client.network.server_name == fallback_server_name) {
		Debug(net, 1, "No \"server_name\" has been set, using \"{}\" instead. Please set this now using the \"server_name <new name>\" command", fallback_server_name);
		_settings_client.network.server_name = fallback_server_name;
	}
}


/**
 * We want to disconnect from the host/clients.
 * @param close_admins Whether the admin sockets need to be closed as well.
 */
void NetworkDisconnect(bool close_admins)
{
	openttd_rust_network_disconnect(&GetRustNetworkLifecycleServices(), &GetRustNetworkFrameServices(), close_admins);
}

/**
 * The setting server_game_type was updated; possibly we need to take some
 * action.
 */
void NetworkUpdateServerGameType()
{
	if (!_networking) return;

	switch (_settings_client.network.server_game_type) {
		case SERVER_GAME_TYPE_LOCAL:
			_network_coordinator_client.CloseConnection();
			break;

		case SERVER_GAME_TYPE_INVITE_ONLY:
		case SERVER_GAME_TYPE_PUBLIC:
			_network_coordinator_client.Register();
			break;

		default:
			NOT_REACHED();
	}
}

/**
 * Receives something from the network.
 * @return true if everything went fine, false when the connection got closed.
 */
static bool NetworkReceive()
{
	bool result;
	if (_network_server) {
		ServerNetworkAdminSocketHandler::Receive();
		result = ServerNetworkGameSocketHandler::Receive();
	} else {
		result = ClientNetworkGameSocketHandler::Receive();
	}
	NetworkGameSocketHandler::ProcessDeferredDeletions();
	return result;
}

/* This sends all buffered commands (if possible) */
static void NetworkSend()
{
	if (_network_server) {
		ServerNetworkAdminSocketHandler::Send();
		ServerNetworkGameSocketHandler::Send();
	} else {
		ClientNetworkGameSocketHandler::Send();
	}
	NetworkGameSocketHandler::ProcessDeferredDeletions();
}

/**
 * We have to do some (simple) background stuff that runs normally,
 * even when we are not in multiplayer. For example stuff needed
 * for finding servers or downloading content.
 */
void NetworkBackgroundLoop()
{
	_network_content_client.SendReceive();
	_network_coordinator_client.SendReceive();
	TCPConnecter::CheckCallbacks();
	NetworkHTTPSocketHandler::HTTPReceive();
	QueryNetworkGameSocketHandler::SendReceive();
	NetworkGameSocketHandler::ProcessDeferredDeletions();

	NetworkBackgroundUDPLoop();
}

/** Log the sync state to check for in-syncedness of replays. */
static void NetworkSyncLog()
{
	if (TimerGameEconomy::date_fract == 0) {
		/* We don't want to log multiple times if paused. */
		static TimerGameEconomy::Date last_log;
		if (last_log != TimerGameEconomy::date) {
			Debug(desync, 1, "sync: {:08x}; {:02x}; {:08x}; {:08x}", TimerGameEconomy::date, TimerGameEconomy::date_fract, _random.state[0], _random.state[1]);
			last_log = TimerGameEconomy::date;
		}
	}
}

/** Replay commands.log; only with DEBUG_DUMP_COMMANDS. */
static void NetworkReplayCommands()
{
#ifdef DEBUG_DUMP_COMMANDS
	/* Loading of the debug commands from -ddesync>=1 */
	static auto f = FioFOpenFile("commands.log", "rb", SAVE_DIR);
	static TimerGameEconomy::Date next_date(0);
	static uint32_t next_date_fract;
	/** A replayed command, injected through NetworkSendCommand. */
	struct DumpedCommand {
		CompanyID company = CompanyID::Invalid();
		Commands cmd{};
		StringID err_msg{};
		CommandDataBuffer data{};
	};
	static DumpedCommand *cp = nullptr;
	static bool check_sync_state = false;
	static uint32_t sync_state[2];
	if (!f.has_value() && next_date == 0) {
		Debug(desync, 0, "Cannot open commands.log");
		next_date = TimerGameEconomy::Date(1);
	}

	while (f.has_value() && !feof(*f)) {
		if (TimerGameEconomy::date == next_date && TimerGameEconomy::date_fract == next_date_fract) {
			if (cp != nullptr) {
				NetworkSendCommand(cp->cmd, cp->err_msg, nullptr, cp->company, cp->data);
				Debug(desync, 0, "Injecting: {:08x}; {:02x}; {:02x}; {:08x}; {} ({})", TimerGameEconomy::date, TimerGameEconomy::date_fract, _current_company.base(), cp->cmd, FormatArrayAsHex(cp->data), GetCommandName(cp->cmd));
				delete cp;
				cp = nullptr;
			}
			if (check_sync_state) {
				if (sync_state[0] == _random.state[0] && sync_state[1] == _random.state[1]) {
					Debug(desync, 0, "Sync check: {:08x}; {:02x}; match", TimerGameEconomy::date, TimerGameEconomy::date_fract);
				} else {
					Debug(desync, 0, "Sync check: {:08x}; {:02x}; mismatch expected {{{:08x}, {:08x}}}, got {{{:08x}, {:08x}}}",
								TimerGameEconomy::date, TimerGameEconomy::date_fract, sync_state[0], sync_state[1], _random.state[0], _random.state[1]);
					NOT_REACHED();
				}
				check_sync_state = false;
			}
		}

		/* Skip all entries in the command-log till we caught up with the current game again. */
		if (TimerGameEconomy::date > next_date || (TimerGameEconomy::date == next_date && TimerGameEconomy::date_fract > next_date_fract)) {
			Debug(desync, 0, "Skipping to next command at {:08x}:{:02x}", next_date, next_date_fract);
			if (cp != nullptr) {
				delete cp;
				cp = nullptr;
			}
			check_sync_state = false;
		}

		if (cp != nullptr || check_sync_state) break;

		char buff[4096];
		if (fgets(buff, lengthof(buff), *f) == nullptr) break;

		StringConsumer consumer{std::string_view{buff}};
		/* Ignore the "[date time] " part of the message */
		if (consumer.ReadCharIf('[')) {
			consumer.SkipUntilChar(']', StringConsumer::SKIP_ONE_SEPARATOR);
			consumer.SkipCharIf(' ');
		}

		if (consumer.ReadIf("cmd: ")
#ifdef DEBUG_FAILED_DUMP_COMMANDS
			|| consumer.ReadIf("cmdf: ")
#endif
			) {
			cp = new DumpedCommand();
			next_date = TimerGameEconomy::Date(consumer.ReadIntegerBase<uint32_t>(16));
			bool valid = consumer.ReadIf("; ");
			next_date_fract = consumer.ReadIntegerBase<uint32_t>(16);
			valid &= consumer.ReadIf("; ");
			cp->company = static_cast<CompanyID>(consumer.ReadIntegerBase<uint16_t>(16));
			valid &= consumer.ReadIf("; ");
			cp->cmd = static_cast<Commands>(consumer.ReadIntegerBase<uint32_t>(16));
			valid &= consumer.ReadIf("; ");
			cp->err_msg = consumer.ReadIntegerBase<uint32_t>(16);
			valid &= consumer.ReadIf("; ");
			auto args = consumer.ReadUntilChar(' ', StringConsumer::SKIP_ONE_SEPARATOR);
			assert(valid);

			/* Parse command data. */
			cp->data.clear();
			for (size_t i = 0; i + 1 < args.size(); i += 2) {
				uint8_t e = 0;
				std::from_chars(args.data() + i, args.data() + i + 2, e, 16);
				cp->data.push_back(e);
			}
		} else if (consumer.ReadIf("join: ")) {
			/* Manually insert a pause when joining; this way the client can join at the exact right time. */
			next_date = TimerGameEconomy::Date(consumer.ReadIntegerBase<uint32_t>(16));
			bool valid = consumer.ReadIf("; ");
			next_date_fract = consumer.ReadIntegerBase<uint32_t>(16);
			assert(valid);
			Debug(desync, 0, "Injecting pause for join at {:08x}:{:02x}; please join when paused", next_date, next_date_fract);
			cp = new DumpedCommand();
			cp->company = COMPANY_SPECTATOR;
			cp->cmd = CMD_PAUSE;
			cp->data = EndianBufferWriter<>::FromValue(CommandTraits<CMD_PAUSE>::Args{ PauseMode::Normal, true });
			_ddc_fastforward = false;
		} else if (consumer.ReadIf("sync: ")) {
			next_date = TimerGameEconomy::Date(consumer.ReadIntegerBase<uint32_t>(16));
			bool valid = consumer.ReadIf("; ");
			next_date_fract = consumer.ReadIntegerBase<uint32_t>(16);
			valid &= consumer.ReadIf("; ");
			sync_state[0] = consumer.ReadIntegerBase<uint32_t>(16);
			valid &= consumer.ReadIf("; ");
			sync_state[1] = consumer.ReadIntegerBase<uint32_t>(16);
			assert(valid);
			check_sync_state = true;
		} else if (consumer.ReadIf("msg: ") || consumer.ReadIf("client: ") ||
					consumer.ReadIf("load: ") || consumer.ReadIf("save: ") ||
					consumer.ReadIf("warning: ")) {
			/* A message that is not very important to the log playback, but part of the log. */
#ifndef DEBUG_FAILED_DUMP_COMMANDS
		} else if (consumer.ReadIf("cmdf: ")) {
			Debug(desync, 0, "Skipping replay of failed command: {}", consumer.Read(StringConsumer::npos));
#endif
		} else {
			/* Can't parse a line; what's wrong here? */
			Debug(desync, 0, "Trying to parse: {}", consumer.Read(StringConsumer::npos));
			NOT_REACHED();
		}
	}
	if (f.has_value() && feof(*f)) {
		Debug(desync, 0, "End of commands.log");
		f.reset();
	}
#endif /* DEBUG_DUMP_COMMANDS */
}

static_assert(std::is_same_v<std::chrono::steady_clock::duration, std::chrono::nanoseconds>); // network_frame.rs now
static_assert(PACKET_SERVER_FRAME == 26 && PACKET_CLIENT_ACK == 27 && PACKET_SERVER_SYNC == 28);
static_assert(NetworkClientSocket::STATUS_INACTIVE == 0 && NetworkClientSocket::STATUS_AUTH_GAME == 1 && NetworkClientSocket::STATUS_IDENTIFY == 2);
static_assert(NetworkClientSocket::STATUS_NEWGRFS_CHECK == 3 && NetworkClientSocket::STATUS_AUTHORIZED == 4 && NetworkClientSocket::STATUS_MAP_WAIT == 5);
static_assert(NetworkClientSocket::STATUS_MAP == 6 && NetworkClientSocket::STATUS_DONE_MAP == 7 && NetworkClientSocket::STATUS_PRE_ACTIVE == 8);
static_assert(NetworkClientSocket::STATUS_ACTIVE == 9 && NetworkClientSocket::STATUS_END == 10);
static_assert(NETWORK_ERROR_NOT_AUTHORIZED == 6 && NETWORK_ERROR_TIMEOUT_PASSWORD == 16 && NETWORK_ERROR_TIMEOUT_COMPUTER == 17);
static_assert(NETWORK_ERROR_TIMEOUT_MAP == 18 && NETWORK_ERROR_TIMEOUT_JOIN == 19);
static_assert(NETWORK_RECV_STATUS_OKAY == 0 && NETWORK_RECV_STATUS_MALFORMED_PACKET == 5);

const OpenTTDNetworkFrameServices &GetRustNetworkFrameServices() noexcept
{
	static const OpenTTDNetworkFrameServices services{
		.bytes_per_frame = RustNetworkBytesPerFrame,
		.bytes_per_frame_burst = RustNetworkBytesPerFrameBurst,
		.client = &GetRustNetworkClientServices(),
		.client_desync = []() noexcept {
			ShowNetworkError(STR_NETWORK_ERROR_DESYNC);
			Debug(desync, 1, "sync_err: {:08x}; {:02x}", TimerGameEconomy::date, TimerGameEconomy::date_fract);
			Debug(net, 0, "Sync error detected");
		},
		.client_game = []() noexcept -> void * { return static_cast<NetworkGameSocketHandler *>(ClientNetworkGameSocketHandler::my_client); },
		.client_sync = []() noexcept -> OpenTTDNetworkSocketSync * { return ClientNetworkGameSocketHandler::my_client; },
		.command = &GetRustNetworkCommandServices(),
		.connection = &GetRustNetworkConnectionServices(),
		.console_bad_acks = [](void *cs) noexcept {
			IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because it fails to send valid acks.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP());
		},
		.console_download_timeout = [](void *cs) noexcept {
			IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because it took longer than {} ticks to download the map.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), RustNetworkMaxDownloadTime());
		},
		.console_init_timeout = [](void *cs) noexcept {
			IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because it took longer than {} ticks to start the joining process.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), RustNetworkMaxInitTime());
		},
		.console_join_timeout = [](void *cs) noexcept {
			IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because it took longer than {} ticks to join.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), RustNetworkMaxJoinTime());
		},
		.console_lagging = [](void *cs, uint32_t lag, bool recent) noexcept {
			if (recent) {
				/* A packet was received in the last three game days, so the client is likely lagging behind. */
				IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because the client's game state is more than {} ticks behind.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), lag);
			} else {
				/* No packet was received in the last three game days; sounds like a lost connection. */
				IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because the client did not respond for more than {} ticks.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), lag);
			}
		},
		.console_password_timeout = [](void *cs) noexcept {
			IConsolePrint(CC_WARNING, "Client #{} (IP: {}) is dropped because it took longer than {} ticks to enter the password.", static_cast<NetworkClientSocket *>(cs)->client_id, static_cast<NetworkClientSocket *>(cs)->GetClientIP(), RustNetworkMaxPasswordTime());
		},
		.console_slow = [](void *cs, uint32_t frame) noexcept {
			IConsolePrint(CC_WARNING, "[{}] Client #{} is slow, try increasing [network.]frame_freq to a higher value!", frame, static_cast<NetworkClientSocket *>(cs)->client_id);
		},
		.debug_ack_sent = [](uint32_t frame) noexcept { Debug(net, 7, "Sent ACK at {}", frame); },
		.debug_active = [](void *game) noexcept { Debug(net, 9, "client[{}] status = ACTIVE", static_cast<NetworkGameSocketHandler *>(game)->client_id); },
		.debug_missed_sync = [](uint32_t sync_frame, uint32_t frame) noexcept { Debug(net, 1, "Missed frame for sync-test: {} / {}", sync_frame, frame); },
		.debug_receive_ack = [](void *game, uint32_t frame) noexcept { Debug(net, 9, "client[{}] Receive_CLIENT_ACK(): frame={}", static_cast<NetworkGameSocketHandler *>(game)->client_id, frame); },
		.debug_receive_sync = [](uint32_t sync_frame, uint32_t seed) noexcept { Debug(net, 9, "Client::Receive_SERVER_SYNC(): sync_frame={}, sync_seed_1={}", sync_frame, seed); },
		.debug_send_ack = []() noexcept { Debug(net, 9, "Client::SendAck()"); },
		.debug_send_sync = [](void *game, uint32_t frame, uint32_t seed) noexcept { Debug(net, 9, "client[{}] SendSync(), frame_counter={}, sync_seed_1={}", static_cast<NetworkGameSocketHandler *>(game)->client_id, frame, seed); },
		.frame_freq = RustNetworkFrameFreq,
		.interactive_random_range = [](uint32_t limit) noexcept { return InteractiveRandomRange(limit); },
		.lifecycle = &GetRustNetworkLifecycleServices(),
		.max_download_time = RustNetworkMaxDownloadTime,
		.max_init_time = RustNetworkMaxInitTime,
		.max_join_time = RustNetworkMaxJoinTime,
		.max_lag_time = RustNetworkMaxLagTime,
		.max_password_time = RustNetworkMaxPasswordTime,
		.now = []() noexcept -> int64_t { return std::chrono::steady_clock::now().time_since_epoch().count(); },
		/* Execute script for, e.g. MOTD */
		.on_server_connect = []() noexcept { IConsoleCmdExec("exec scripts/on_server_connect.scr 0"); },
		.packet_can_read = [](void *p, size_t size) noexcept { return static_cast<Packet *>(p)->CanReadFromPacket(size); },
		.packet_new = [](void *game, uint8_t type) noexcept -> void * { return new Packet(static_cast<NetworkGameSocketHandler *>(game), static_cast<PacketType>(type)); },
		.packet_recv_string = [](void *packet, void *text, size_t limit) noexcept { *static_cast<std::string *>(text) = static_cast<Packet *>(packet)->Recv_string(limit); },
		.packet_recv_uint32 = [](void *p) noexcept { return static_cast<Packet *>(p)->Recv_uint32(); },
		.packet_recv_uint8 = [](void *p) noexcept { return static_cast<Packet *>(p)->Recv_uint8(); },
		.packet_send = [](void *game, void *p) noexcept { static_cast<NetworkGameSocketHandler *>(game)->SendPacket(std::unique_ptr<Packet>(static_cast<Packet *>(p))); },
		.packet_send_string = [](void *packet, OpenTTDByteSpan text) noexcept { static_cast<Packet *>(packet)->Send_string(std::string_view(reinterpret_cast<const char *>(text.data), text.size)); },
		.packet_send_uint32 = [](void *p, uint32_t value) noexcept { static_cast<Packet *>(p)->Send_uint32(value); },
		.packet_send_uint8 = [](void *p, uint8_t value) noexcept { static_cast<Packet *>(p)->Send_uint8(value); },
		.random = _random.state,
		.receive = [](OpenTTDSessionFrame *f, bool *received) noexcept { return SessionException(f, [=] { *received = NetworkReceive(); }); },
		.replay_commands = []() noexcept { NetworkReplayCommands(); },
		.send = [](OpenTTDSessionFrame *f) noexcept { return SessionException(f, [] { NetworkSend(); }); },
		.socket_close = [](void *game) noexcept { static_cast<NetworkGameSocketHandler *>(game)->NetworkGameSocketHandler::CloseConnection(); },
		.socket_defer_deletion = [](void *game) noexcept { static_cast<NetworkGameSocketHandler *>(game)->DeferDeletion(); },
		.socket_has_quit = [](void *game) noexcept -> bool { return static_cast<NetworkGameSocketHandler *>(game)->HasClientQuit(); },
		.socket_last_packet = [](void *game) noexcept -> int64_t { return static_cast<NetworkGameSocketHandler *>(game)->last_packet.time_since_epoch().count(); },
		.socket_mark_closed = [](void *game) noexcept { static_cast<NetworkGameSocketHandler *>(game)->MarkClosed(); },
		.socket_pending_deletion = [](void *game) noexcept -> bool { return static_cast<NetworkGameSocketHandler *>(game)->IsPendingDeletion(); },
		.socket_send_packets = [](void *game, bool closing) noexcept { static_cast<NetworkGameSocketHandler *>(game)->SendPackets(closing); },
		.socket_set_last_packet_now = [](void *game) noexcept { static_cast<NetworkGameSocketHandler *>(game)->last_packet = std::chrono::steady_clock::now(); },
		.socket_valid = [](void *game) noexcept -> bool { return static_cast<NetworkGameSocketHandler *>(game)->sock != INVALID_SOCKET; },
		.sync_freq = RustNetworkSyncFreq,
		.sync_log = []() noexcept { NetworkSyncLog(); },
	};
	return services;
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkFrameServices.hpp"
bool CheckAbi_network_frame()
{
	return AbiServicesComplete(GetRustNetworkFrameServices());
}

/** This tries to launch the network for a given OS */
void NetworkStartUp()
{
	Debug(net, 3, "Starting network");

	/* Network is available */
	_network_available = NetworkCoreInitialize();
	_network_dedicated = false;

	_network_game_info = {};

	NetworkInitialize();
	NetworkUDPInitialize();
	Debug(net, 3, "Network online, multiplayer available");
	NetworkFindBroadcastIPs(&_broadcast_list);
	NetworkHTTPInitialize();
}

/** This shuts the network down */
void NetworkShutDown()
{
	NetworkDisconnect();
	NetworkHTTPUninitialize();
	NetworkUDPClose();

	Debug(net, 3, "Shutting down network");

	_network_available = false;

	NetworkCoreShutdown();
}

#ifdef __EMSCRIPTEN__
extern "C" {

void CDECL em_openttd_add_server(const char *connection_string)
{
	NetworkAddServer(connection_string, false, true);
}

}
#endif

const OpenTTDNetworkLifecycleServices &GetRustNetworkLifecycleServices() noexcept
{
	static_assert(SERVER_GAME_TYPE_LOCAL == 0 && CLIENT_ID_SERVER == 1);
	static const OpenTTDNetworkLifecycleServices services{
		.admin_auth_configured = RustNetworkAdminAuthenticationConfigured,
		.admin_port = RustNetworkServerAdminPort,
		.admins_close = []() noexcept {
			for (ServerNetworkAdminSocketHandler *as : ServerNetworkAdminSocketHandler::Iterate()) {
				as->CloseConnection(true);
			}
		},
		.admins_newgame = []() noexcept {
			for (ServerNetworkAdminSocketHandler *as : ServerNetworkAdminSocketHandler::IterateActive()) {
				as->SendNewGame();
				as->SendPackets();
			}
		},
		.admins_shutdown = []() noexcept {
			for (ServerNetworkAdminSocketHandler *as : ServerNetworkAdminSocketHandler::IterateActive()) {
				as->SendShutdown();
				as->SendPackets();
			}
		},
		.admins_welcome = []() noexcept { ServerNetworkAdminSocketHandler::WelcomeAll(); },
		.allocate_server_info = []() noexcept -> void * {
			assert(NetworkClientInfo::CanAllocateItem());
			auto *ci = new NetworkClientInfo(CLIENT_ID_SERVER);
			ci->client_playas = COMPANY_SPECTATOR;
			ci->client_name = _settings_client.network.client_name;
			return ci;
		},
		.auth_client_key = []() noexcept { NetworkAuthenticationClientHandler::EnsureValidSecretKeyAndUpdatePublicKey(_settings_client.network.client_secret_key, _settings_client.network.client_public_key); },
		.check_names = []() noexcept { CheckClientAndServerName(); },
		.close_admin_listeners = []() noexcept { ServerNetworkAdminSocketHandler::CloseListeners(); },
		.close_client_listeners = []() noexcept { ServerNetworkGameSocketHandler::CloseListeners(); },
		.connect_debug = [](OpenTTDByteSpan connection) noexcept { Debug(net, 9, "NetworkClientConnectGame(): connection_string={}", std::string_view(reinterpret_cast<const char *>(connection.data), connection.size)); },
		.connect_text_begin = [](void *storage) noexcept -> void * {
			static_assert(sizeof(std::string) <= 8 * sizeof(uint64_t));
			static_assert(alignof(std::string) <= alignof(uint64_t));
			return new (storage) std::string;
		},
		.connect_text_end = [](void *text) noexcept { std::destroy_at<std::string>(static_cast<std::string *>(text)); },
		.coordinator_close_client = []() noexcept { _network_coordinator_client.CloseAllConnections(); },
		.coordinator_close_server = []() noexcept { _network_coordinator_client.CloseConnection(); },
		.coordinator_register = []() noexcept { _network_coordinator_client.Register(); },
		.create_client_connecter = []() noexcept { TCPConnecter::Create<TCPClientConnecter>(_network_join.connection_string); },
		.debug_connecting = []() noexcept { Debug(net, 9, "status = CONNECTING"); },
		.debug_listen_admins = []() noexcept { Debug(net, 5, "Starting listeners for admins"); },
		.debug_listen_clients = []() noexcept { Debug(net, 5, "Starting listeners for clients"); },
		.debug_listen_queries = []() noexcept { Debug(net, 5, "Starting listeners for incoming server queries"); },
		.deferred_deletions = []() noexcept { NetworkGameSocketHandler::ProcessDeferredDeletions(); },
		.fill_game_info = []() noexcept { FillStaticNetworkServerGameInfo(); },
		.initialize_pools = [](bool close_admins) noexcept { InitializeNetworkPools(close_admins); },
		.join_connection = []() noexcept -> OpenTTDByteSpan { const auto &connection = _network_join.connection_string; return {reinterpret_cast<const uint8_t *>(connection.data()), connection.size()}; },
		.kill_connecters = []() noexcept { TCPConnecter::KillAll(); },
		.last_joined = RustNetworkLastJoined,
		.listen_admins = [](uint16_t port) noexcept -> bool { return ServerNetworkAdminSocketHandler::Listen(port); },
		.listen_clients = [](uint16_t port) noexcept -> bool { return ServerNetworkGameSocketHandler::Listen(port); },
		.new_company = [](const Company *company, void *info) noexcept { NetworkServerNewCompany(company, static_cast<NetworkClientInfo *>(info)); },
		.on_dedicated = []() noexcept { IConsoleCmdExec("exec scripts/on_dedicated.scr 0"); },
		.on_server = []() noexcept { IConsoleCmdExec("exec scripts/on_server.scr 0"); },
		.parse_connection = [](OpenTTDByteSpan connection, uint8_t *company, void *resolved) noexcept {
			CompanyID join_as(*company);
			*static_cast<std::string *>(resolved) = ServerAddress::Parse(std::string_view(reinterpret_cast<const char *>(connection.data), connection.size), NETWORK_DEFAULT_PORT, &join_as).connection_string;
			*company = join_as.base();
		},
		.pre_dedicated = []() noexcept { IConsoleCmdExec("exec scripts/pre_dedicated.scr 0"); },
		.pre_server = []() noexcept { IConsoleCmdExec("exec scripts/pre_server.scr 0"); },
		.remember_joined_server = RustNetworkSetLastJoined,
		.reset_restart_time = []() noexcept { ChangeNetworkRestartTime(true); },
		.server_game_type = RustNetworkServerGameType,
		.server_port = RustNetworkServerPort,
		.set_info_company = [](void *info, uint8_t company) noexcept { static_cast<NetworkClientInfo *>(info)->client_playas = CompanyID(company); },
		.set_join_info = [](void *resolved, uint8_t company, const void *password) noexcept {
			_network_join.connection_string = std::move(*static_cast<std::string *>(resolved));
			_network_join.company = CompanyID(company);
			_network_join.server_password = *static_cast<const std::string *>(password);
		},
		.set_server_info_key = [](void *info) noexcept { static_cast<NetworkClientInfo *>(info)->public_key = _settings_client.network.client_public_key; },
		.udp_initialize = []() noexcept { NetworkUDPInitialize(); },
		.udp_listen = []() noexcept { NetworkUDPServerListen(); },
		.update_game_info = []() noexcept { NetworkServerUpdateGameInfo(); },
		.validate_client_name = []() noexcept -> bool { return NetworkValidateOurClientName(); },
	};
	return services;
}
#include "abi-visit-OpenTTDNetworkLifecycleServices.hpp"
bool CheckAbi_network_lifecycle() { return AbiServicesComplete(GetRustNetworkLifecycleServices()); }

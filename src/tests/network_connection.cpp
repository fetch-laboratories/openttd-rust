/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection.cpp Admission behavior at the native packet boundary. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../network/core/packet.h"
#include "../newgrf_config.h"
#include "../rust/network_client_ffi.h"
#include "../rust/network_connection_ffi.h"
#include "../rust/company_services.h"
#include "../rust/session_native.hpp"

TEST_CASE("Network admission - each NewGRF checksum starts empty")
{
	struct RestoreConfigs {
		GRFConfigList saved = std::move(_all_grfs);
		~RestoreConfigs() { _all_grfs = std::move(this->saved); }
	} configs;
	_all_grfs.clear();
	auto config = std::make_unique<GRFConfig>();
	config->ident.grfid = 0;
	config->ident.md5sum.fill(42);
	_all_grfs.push_back(std::move(config));

	class Socket : public NetworkSocketHandler {} socket;
	Packet source(&socket, PacketType{});
	source.Send_uint8(2);
	source.Send_uint32(0);
	for (uint i = 0; i < MD5_HASH_BYTES; ++i) source.Send_uint8(42);
	source.Send_uint32(0); // The second identifier has no checksum bytes.
	source.PrepareToSend();
	Packet packet(&socket, COMPAT_MTU, source.Size());
	packet.TransferIn([&](std::span<uint8_t> destination) {
		return source.TransferOut([&](std::span<const uint8_t> bytes) {
			std::ranges::copy(bytes, destination.begin());
			return bytes.size();
		});
	});
	REQUIRE(packet.PrepareToRead());
	packet.Recv_uint8(); // Packet type.

	auto services = GetRustNetworkClientServices();
	services.status = [](void *game) noexcept { return static_cast<uint8_t *>(game); };
	services.debug_grfs = [](uint32_t) noexcept {};
	services.show_newgrf_mismatch = []() noexcept {};
	uint8_t status = 3; // Client STATUS_ENCRYPTED.
	CHECK(openttd_rust_network_client_receive_grfs(&services, &GetRustNetworkFrameServices(), &status, packet.Record()) == NETWORK_RECV_STATUS_NEWGRF_MISMATCH);
	CHECK(packet.Record()->position == packet.Size());
}

static uint8_t map_status;
TEST_CASE("Network map load - recorded exception stops cleanup and reaches native caller")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	map_status = 7; // Client STATUS_MAP.
	OpenTTDNetworkClientServices client{};
	client.status = [](void *) noexcept { return &map_status; };
	client.map_exists = [](void *) noexcept { return true; };
	client.debug_map_done = []() noexcept {};
	client.debug_join_processing = []() noexcept {};
	uint8_t join_status = 0;
	client.join_status = &join_status;
	client.dirty_join = []() noexcept {};
	client.map_load_prepare = [](void *, OpenTTDSessionLoad *) noexcept {};
	auto session = GetRustSessionServices();
	session.load = [](OpenTTDSessionFrame *frame, OpenTTDSessionLoad *) noexcept {
		SessionException(frame, [] { throw 574; });
		return SL_ERROR;
	};
	OpenTTDNetworkFrameServices network{};
	OpenTTDSessionFrame frame;
	const std::string filename;
	OpenTTDSessionLoad load{filename, SLO_LOAD, DFT_GAME_FILE, NO_DIRECTORY, {}};
	CHECK(openttd_rust_network_client_receive_map_done(&client, &network, &session, nullptr, &frame, &load) == 0);
	CHECK(_game_mode == GM_NORMAL);
	CHECK_THROWS_AS(frame.Rethrow(), int);
}

TEST_CASE("Network ack - console exception stops before token consumption")
{
	OpenTTDNetworkFrameServices network{};
	network.packet = &Packet::Services();
	class Socket : public NetworkSocketHandler {} socket;
	Packet packet(&socket, PacketType{});
	packet.Send_uint32(GetRustNetworkFrameState().frame_counter);
	packet.Record()->position = Packet::EncodedLengthOfPacketSize() + Packet::EncodedLengthOfPacketType();
	network.debug_receive_ack = [](void *, uint32_t) noexcept {};
	network.debug_active = [](void *) noexcept {};
	network.on_server_connect = [](OpenTTDSessionFrame *frame) noexcept {
		return SessionException(frame, [] { throw 574; });
	};
	uint8_t status = 8; // Server STATUS_PRE_ACTIVE.
	OpenTTDNetworkSocketSync sync{};
	sync.last_frame = 123;
	sync.last_frame_server = 456;
	OpenTTDSessionFrame frame;
	CHECK(openttd_rust_network_receive_ack(&network, nullptr, nullptr, &status, &sync, packet.Record(), &frame, &frame.network_actions) == 0);
	CHECK(status == 9); // Server STATUS_ACTIVE.
	CHECK(sync.last_token_frame == GetRustNetworkFrameState().frame_counter);
	CHECK(sync.last_frame == 123);
	CHECK(sync.last_frame_server == 456);
	CHECK_THROWS_AS(frame.Rethrow(), int);
}

struct MapClient {
	uint8_t status;
	OpenTTDCommandQueue incoming{};
	OpenTTDCommandQueue outgoing{};
	OpenTTDNetworkSocketSync sync{};
};
static std::array<MapClient, 4> map_clients;
static std::vector<std::pair<int, int>> map_events;
static int MapIndex(void *socket) { return static_cast<MapClient *>(socket) - map_clients.data(); }
static void MapEvent(int event, void *socket) { map_events.emplace_back(event, MapIndex(socket)); }
TEST_CASE("Network map save - native actions preserve nested waits and close tail order")
{
	map_clients = {};
	map_events.clear();
	map_clients[0].status = 6; // Server STATUS_MAP.
	for (int i = 1; i < 4; i++) map_clients[i].status = 5; // Server STATUS_MAP_WAIT.
	OpenTTDNetworkCommandServices command{};
	command.company = &GetRustCompanyServices();
	command.next_client = [](size_t from, OpenTTDNetworkCommandClient *client) noexcept {
		if (from >= map_clients.size()) return false;
		auto *cs = &map_clients[from];
		*client = {cs, cs, from, &cs->incoming, &cs->outgoing, &cs->sync, 0, cs->status};
		return true;
	};
	OpenTTDNetworkConnectionServices connection{};
	connection.game = [](void *socket) noexcept { return socket; };
	connection.status = [](void *socket) noexcept { return &static_cast<MapClient *>(socket)->status; };
	connection.sync = [](void *socket) noexcept { return &static_cast<MapClient *>(socket)->sync; };
	connection.outgoing_queue = [](void *socket) noexcept { return &static_cast<MapClient *>(socket)->outgoing; };
	connection.join_date = [](void *) noexcept { return 0; };
	connection.client_id = [](void *socket) noexcept { return static_cast<uint32_t>(MapIndex(socket)); };
	connection.debug_next_map = [](void *) noexcept {};
	connection.debug_map_first = [](void *) noexcept {};
	connection.debug_map = [](void *) noexcept {};
	connection.debug_map_last = [](void *) noexcept {};
	connection.debug_done_map = [](void *) noexcept {};
	connection.debug_send_wait = [](void *socket) noexcept { MapEvent(7, socket); };
	connection.debug_closed = [](void *) noexcept {};
	connection.map_create = [](void *socket) noexcept { MapEvent(2, socket); };
	connection.map_destroy = [](void *socket) noexcept { MapEvent(1, socket); };
	connection.map_transfer = [](void *socket) noexcept { MapEvent(4, socket); return MapIndex(socket) == 1; };
	connection.admin_error = [](void *socket, uint8_t) noexcept { MapEvent(8, socket); };
	uint8_t on = 4, connected = 4;
	connection.clients_on = &on;
	connection.clients_connected = &connected;
	OpenTTDNetworkFrameServices network{};
	network.command = &command;
	network.connection = &connection;
	network.socket_pending_deletion = [](void *) noexcept { return false; };
	network.socket_valid = [](void *) noexcept { return true; };
	network.socket_send_packets = [](void *socket, bool) noexcept { MapEvent(9, socket); };
	network.socket_defer_deletion = [](void *socket) noexcept { MapEvent(10, socket); };
	auto packet_services = Packet::Services();
	packet_services.has_quit = [](const NetworkSocketHandler *) noexcept { return false; };
	network.packet = &packet_services;
	network.packet_socket = [](void *) noexcept -> NetworkSocketHandler * { return nullptr; };
	network.packet_enqueue = [](void *, OpenTTDPacketState *state) noexcept { Packet packet(*state); };
	OpenTTDSessionFrame frame;
	CHECK(openttd_rust_network_server_close(&connection, &network, &map_clients[0], NETWORK_RECV_STATUS_CLIENT_QUIT, &frame.network_actions) == NETWORK_RECV_STATUS_CLIENT_QUIT);
	CHECK(on == 4);
	CHECK(connected == 4);
	REQUIRE(openttd_rust_network_actions_save(&frame.network_actions) == &map_clients[1]);
	MapEvent(3, &map_clients[1]); // The native caller saves before resuming Rust.
	openttd_rust_network_actions_resume(&frame.network_actions);
	CHECK(on == 4);
	REQUIRE(openttd_rust_network_actions_save(&frame.network_actions) == &map_clients[2]);
	MapEvent(3, &map_clients[2]);
	openttd_rust_network_actions_resume(&frame.network_actions);
	CHECK(openttd_rust_network_actions_save(&frame.network_actions) == nullptr);
	CHECK(on == 3);
	CHECK(connected == 3);
	const std::vector<std::pair<int, int>> expected{
		{1, 0}, {2, 1}, {3, 1}, {4, 1}, {1, 1}, {2, 2}, {3, 2}, {4, 2},
		{7, 3}, {7, 3}, {8, 0}, {9, 0}, {10, 0},
	};
	CHECK(map_events == expected);
}

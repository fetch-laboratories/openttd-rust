/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection.cpp Admission behavior at the native packet boundary. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../network/core/packet.h"
#include "../network/core/tcp.h"
#include "../rust/services_assert.h"
#include "../newgrf_config.h"
#include "../rust/network_client_ffi.h"
#include "../rust/network_tcp_ffi.h"
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
	services.debug_grfs = [](uint32_t) noexcept {};
	services.show_newgrf_mismatch = []() noexcept {};
	OpenTTDNetworkGameSocket game{};
	game.base = &socket;
	game.status = 3; // Client STATUS_ENCRYPTED.
	CHECK(openttd_rust_network_client_receive_grfs(&services, &GetRustNetworkFrameServices(), &game, packet.Record()) == NETWORK_RECV_STATUS_NEWGRF_MISMATCH);
	CHECK(packet.Record()->position == packet.Size());
}

TEST_CASE("Network map load - recorded exception stops cleanup and reaches native caller")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	OpenTTDNetworkGameSocket game{};
	game.status = 7; // Client STATUS_MAP.
	OpenTTDNetworkClientServices client{};
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
	CHECK(openttd_rust_network_client_receive_map_done(&client, &network, &session, &game, &frame, &load) == 0);
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
	OpenTTDNetworkGameSocket game{};
	game.status = 8; // Server STATUS_PRE_ACTIVE.
	game.base = &socket;
	OpenTTDNetworkSocketSync sync{};
	game.sync = &sync;
	sync.last_frame = 123;
	sync.last_frame_server = 456;
	OpenTTDSessionFrame frame;
	CHECK(openttd_rust_network_receive_ack(&network, &game, packet.Record(), &frame, &frame.network_actions) == 0);
	CHECK(game.status == 9); // Server STATUS_ACTIVE.
	CHECK(sync.last_token_frame == GetRustNetworkFrameState().frame_counter);
	CHECK(sync.last_frame == 123);
	CHECK(sync.last_frame_server == 456);
	CHECK_THROWS_AS(frame.Rethrow(), int);
}

struct MapClient {
	OpenTTDNetworkGameSocket record{};
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
	for (auto &client : map_clients) {
		client.record.game = &client;
		client.record.tcp = reinterpret_cast<NetworkTCPSocketHandler *>(&client);
		client.record.server = &client;
		client.record.incoming = &client.incoming;
		client.record.outgoing = &client.outgoing;
		client.record.sync = &client.sync;
	}
	map_clients[0].record.status = 6; // Server STATUS_MAP.
	for (int i = 1; i < 4; i++) map_clients[i].record.status = 5; // Server STATUS_MAP_WAIT.
	OpenTTDNetworkCommandServices command{};
	command.company = &GetRustCompanyServices();
	command.next_client = [](size_t from, OpenTTDNetworkCommandClient *client) noexcept {
		if (from >= map_clients.size()) return false;
		auto *cs = &map_clients[from];
		*client = {&cs->record, from};
		return true;
	};
	OpenTTDNetworkConnectionServices connection{};
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
	OpenTTDNetworkTcpServices tcp{};
	tcp.valid = [](const NetworkTCPSocketHandler *) noexcept { return true; };
	tcp.writable = [](const NetworkTCPSocketHandler *socket) noexcept { MapEvent(9, const_cast<void *>(static_cast<const void *>(socket))); return true; };
	tcp.send_front = [](NetworkTCPSocketHandler *) noexcept -> OpenTTDPacketState * { return nullptr; };
	network.tcp = &tcp;
	network.socket_delete = [](void *socket) noexcept { MapEvent(10, socket); };
	auto packet_services = Packet::Services();
	packet_services.has_quit = [](const NetworkSocketHandler *) noexcept { return false; };
	network.packet = &packet_services;
	network.packet_enqueue = [](void *, OpenTTDPacketState *state) noexcept { Packet packet(*state); };
	OpenTTDSessionFrame frame;
	CHECK(openttd_rust_network_server_close(&connection, &network, &map_clients[0].record, NETWORK_RECV_STATUS_CLIENT_QUIT, &frame.network_actions) == NETWORK_RECV_STATUS_CLIENT_QUIT);
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
	CHECK(map_clients[0].record.pending);
	CHECK(openttd_rust_network_server_close(&connection, &network, &map_clients[0].record, NETWORK_RECV_STATUS_CLIENT_QUIT, &frame.network_actions) == NETWORK_RECV_STATUS_CLIENT_QUIT);
	CHECK(on == 3);
	CHECK(connected == 3);
	openttd_rust_network_process_deferred(&network);
	const std::vector<std::pair<int, int>> expected{
		{1, 0}, {2, 1}, {3, 1}, {4, 1}, {1, 1}, {2, 2}, {3, 2}, {4, 2},
		{7, 3}, {7, 3}, {8, 0}, {9, 0}, {10, 0},
	};
	CHECK(map_events == expected);
}

static OpenTTDPacketState *tcp_packet;
static intptr_t tcp_transfer_result;
static bool tcp_valid, tcp_writable, tcp_blocked;
static uint tcp_calls, tcp_logs, tcp_pops;
static std::vector<uint8_t> tcp_output;
TEST_CASE("Network TCP - partial writes and close requests preserve cursor and closing policy")
{
	struct Case { intptr_t result; bool closing, blocked, valid, writable; OpenTTDTcpSend expected; size_t cursor; uint calls, logs, pops; };
	const Case cases[]{
		{-2, false, false, true, true, OpenTTDTcpSend::All, 0, 1, 0, 1},
		{2, false, false, true, true, OpenTTDTcpSend::Partly, 2, 1, 0, 0},
		{-1, false, true, true, true, OpenTTDTcpSend::Partly, 0, 1, 0, 0},
		{-1, false, false, true, true, OpenTTDTcpSend::Close, 0, 1, 1, 0},
		{-1, true, false, true, true, OpenTTDTcpSend::Closed, 0, 1, 0, 0},
		{0, false, false, true, true, OpenTTDTcpSend::Close, 0, 1, 0, 0},
		{0, true, false, true, true, OpenTTDTcpSend::Closed, 0, 1, 0, 0},
		{-2, false, false, false, true, OpenTTDTcpSend::Closed, 0, 0, 0, 0},
		{-2, false, false, true, false, OpenTTDTcpSend::None, 0, 0, 0, 0},
	};
	auto services = GetRustNetworkTcpServices();
	services.valid = [](const NetworkTCPSocketHandler *) noexcept { return tcp_valid; };
	services.writable = [](const NetworkTCPSocketHandler *) noexcept { return tcp_writable; };
	services.send_front = [](NetworkTCPSocketHandler *) noexcept { return tcp_packet; };
	services.send_bytes = [](NetworkTCPSocketHandler *, const uint8_t *data, size_t size) noexcept -> intptr_t {
		tcp_calls++;
		auto result = tcp_transfer_result == -2 ? static_cast<intptr_t>(size) : tcp_transfer_result;
		if (result > 0) tcp_output.insert(tcp_output.end(), data, data + result);
		return result;
	};
	services.error_would_block = [](const NetworkError *) noexcept { return tcp_blocked; };
	services.error_send = [](const NetworkError *) noexcept { tcp_logs++; };
	services.send_pop = [](NetworkTCPSocketHandler *) noexcept { *tcp_packet = {}; tcp_packet = nullptr; tcp_pops++; };
	for (const auto &test : cases) {
		CAPTURE(test.result, test.closing, test.blocked, test.valid, test.writable);
		Packet packet(nullptr, PacketType{9}); packet.Send_uint32(0x04030201); packet.PrepareToSend();
		tcp_packet = packet.Record(); tcp_transfer_result = test.result;
		tcp_valid = test.valid; tcp_writable = test.writable; tcp_blocked = test.blocked;
		tcp_calls = tcp_logs = tcp_pops = 0; tcp_output.clear();
		CHECK(openttd_rust_tcp_send(&services, nullptr, test.closing, RustAssertCondition) == test.expected);
		CHECK(tcp_calls == test.calls); CHECK(tcp_logs == test.logs); CHECK(tcp_pops == test.pops);
		if (tcp_packet != nullptr) CHECK(tcp_packet->position == test.cursor);
		const std::array<uint8_t, 7> expected{7, 0, 9, 1, 2, 3, 4};
		CHECK(std::equal(tcp_output.begin(), tcp_output.end(), expected.begin()));
	}
}

static Packet *tcp_receive_packet;
static std::vector<uint8_t> tcp_input;
static size_t tcp_read_position, tcp_read_limit;
static intptr_t tcp_read_end;
TEST_CASE("Network TCP - fragmented size headers resume and invalid sizes request close")
{
	auto services = GetRustNetworkTcpServices();
	services.valid = [](const NetworkTCPSocketHandler *) noexcept { return true; };
	class Socket : public NetworkSocketHandler {} socket;
	static NetworkSocketHandler *base;
	base = &socket;
	services.base_socket = [](NetworkTCPSocketHandler *) noexcept { return base; };
	services.receive_packet = [](NetworkTCPSocketHandler *) noexcept -> OpenTTDPacketState * { return tcp_receive_packet == nullptr ? nullptr : tcp_receive_packet->Record(); };
	services.receive_create = [](NetworkTCPSocketHandler *, OpenTTDPacketState *state) noexcept { tcp_receive_packet = new Packet(*state); return tcp_receive_packet->Record(); };
	services.receive_bytes = [](NetworkTCPSocketHandler *, uint8_t *data, size_t size) noexcept -> intptr_t {
		if (tcp_read_position == tcp_read_limit) return tcp_read_end;
		size_t count = std::min(size, tcp_read_limit - tcp_read_position);
		std::copy_n(tcp_input.data() + tcp_read_position, count, data);
		tcp_read_position += count;
		return count;
	};
	services.error_would_block = [](const NetworkError *) noexcept { return true; };
	tcp_receive_packet = nullptr; tcp_input = {7, 0, 9, 1, 2, 3, 4};
	tcp_read_position = 0; tcp_read_limit = 1; tcp_read_end = -1;
	CHECK(openttd_rust_tcp_receive(&services, nullptr, RustAssertCondition) == OpenTTDTcpReceive::None);
	REQUIRE(tcp_receive_packet != nullptr); CHECK(tcp_receive_packet->Record()->position == 1);
	tcp_read_limit = tcp_input.size();
	CHECK(openttd_rust_tcp_receive(&services, nullptr, RustAssertCondition) == OpenTTDTcpReceive::Ready);
	CHECK(tcp_receive_packet->Record()->position == 7);
	CHECK(std::equal(tcp_input.begin(), tcp_input.end(), tcp_receive_packet->Record()->data));
	delete std::exchange(tcp_receive_packet, nullptr);
	for (const auto &wire : {std::vector<uint8_t>{1, 0}, std::vector<uint8_t>{0, 128}, std::vector<uint8_t>{7, 0}}) {
		tcp_input = wire; tcp_read_position = 0; tcp_read_limit = wire.size(); tcp_read_end = 0;
		CHECK(openttd_rust_tcp_receive(&services, nullptr, RustAssertCondition) == OpenTTDTcpReceive::Close);
		delete std::exchange(tcp_receive_packet, nullptr);
	}
}

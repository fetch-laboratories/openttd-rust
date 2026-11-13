/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection.cpp Admission behavior at the native packet boundary. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../network/core/packet.h"
#include "../network/core/tcp.h"
#include "../network/core/tcp_game.h"
#include "../network/network_crypto_internal.h"
#include "../string_func.h"
#include "../rust/network_client_ffi.h"
#include "../rust/network_dispatch_ffi.h"
#include "../rust/network_tcp_ffi.h"
#include "../rust/network_connection_ffi.h"
#include "../rust/company_services.h"
#include "../rust/session_native.hpp"

/* Tests supply transport bytes, while production framing, dispatch and cleanup run. */
static std::span<const uint8_t> fixture_receive_wire;
static NetworkSocketHandler *fixture_receive_base;
static Packet *fixture_receive_pending;
static uint fixture_receive_position;
static uint8_t ReceiveFixturePacket(const OpenTTDSessionServices &session, OpenTTDNetworkFrameServices network, OpenTTDNetworkGameSocket &game, OpenTTDSessionFrame &frame, Packet &source)
{
	source.PrepareToSend();
	fixture_receive_wire = {source.Record()->data, source.Size()};
	fixture_receive_base = game.base;
	fixture_receive_pending = nullptr;
	fixture_receive_position = 0;
	struct Pending { ~Pending() { delete std::exchange(fixture_receive_pending, nullptr); } } pending;
	auto tcp = GetRustNetworkTcpServices();
	tcp.valid = [](const NetworkTCPSocketHandler *) noexcept { return true; };
	tcp.base_socket = [](NetworkTCPSocketHandler *) noexcept { return fixture_receive_base; };
	tcp.receive_packet = [](NetworkTCPSocketHandler *) noexcept -> OpenTTDPacketState * { return fixture_receive_pending ? fixture_receive_pending->Record() : nullptr; };
	tcp.receive_create = [](NetworkTCPSocketHandler *, OpenTTDPacketState *record) noexcept { fixture_receive_pending = new Packet(*record); return fixture_receive_pending->Record(); };
	tcp.receive_bytes = [](NetworkTCPSocketHandler *, uint8_t *data, size_t size) noexcept -> intptr_t {
		if (fixture_receive_wire.empty()) return -1;
		size_t count = std::min(size, fixture_receive_wire.size());
		std::copy_n(fixture_receive_wire.data(), count, data);
		fixture_receive_wire = fixture_receive_wire.subspan(count);
		return count;
	};
	tcp.receive_take = [](NetworkTCPSocketHandler *) noexcept { return std::exchange(fixture_receive_pending, nullptr); };
	tcp.error_would_block = [](const NetworkError *) noexcept { return true; };
	tcp.packet_record = [](Packet *packet) noexcept { fixture_receive_position = packet->Record()->position; return packet->Record(); };
	network.tcp = &tcp;
	network.packet = &Packet::Services();
	if (network.command == nullptr) network.command = &GetRustNetworkCommandServices();
	if (network.dispatch == nullptr) network.dispatch = &GetRustNetworkDispatchServices();
	tcp.now = []() noexcept -> int64_t { return 0; };
	openttd_rust_network_receive_packets(&session, &network, &game, &frame, &frame.network_actions);
	return openttd_rust_network_actions_result(&frame.network_actions);
}

TEST_CASE("Network map load - recorded exception stops cleanup and reaches native caller")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	OpenTTDNetworkGameSocket game{};
	game.status = 7; // Client STATUS_MAP.
	OpenTTDNetworkClientServices client{};
	client.map_exists = [](void *) noexcept { return true; };
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
	network.client = &client;
	NetworkSocketHandler socket;
	game.base = &socket;
	Packet source(&socket, PACKET_SERVER_MAP_DONE);
	CHECK(ReceiveFixturePacket(session, network, game, frame, source) == 0);
	CHECK(_game_mode == GM_NORMAL);
	CHECK_THROWS_AS(frame.Rethrow(), int);
}

TEST_CASE("Network ack - console exception stops before token consumption")
{
	OpenTTDNetworkFrameServices network{};
	network.packet = &Packet::Services();
	class Socket : public NetworkSocketHandler {} socket;
	Packet packet(&socket, PACKET_CLIENT_ACK);
	packet.Send_uint32(GetRustNetworkFrameState().frame_counter);
	network.on_server_connect = [](OpenTTDSessionFrame *frame) noexcept {
		return SessionException(frame, [] { throw 574; });
	};
	OpenTTDNetworkGameSocket game{};
	game.status = 8; // Server STATUS_PRE_ACTIVE.
	game.role = 1;
	game.base = &socket;
	OpenTTDNetworkSocketSync sync{};
	game.sync = &sync;
	sync.receive_limit = COMPAT_MTU;
	sync.last_frame = 123;
	sync.last_frame_server = 456;
	OpenTTDSessionFrame frame;
	CHECK(ReceiveFixturePacket(GetRustSessionServices(), network, game, frame, packet) == 0);
	CHECK(fixture_receive_position == 7); // The missing token was never consumed.
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
static Packet *map_done;
static int MapIndex(void *socket) { return static_cast<MapClient *>(socket) - map_clients.data(); }
static void MapEvent(int event, void *socket) { map_events.emplace_back(event, MapIndex(socket)); }
TEST_CASE("Network map save - native actions preserve nested waits and close tail order")
{
	map_clients = {};
	map_events.clear();
	Packet done(nullptr, static_cast<PacketType>(23)); // SERVER_MAP_DONE.
	map_done = &done;
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
	connection.map_create = [](void *socket) noexcept { MapEvent(2, socket); };
	connection.map_destroy = [](void *socket) noexcept { MapEvent(1, socket); };
	connection.map_begin = [](void *socket) noexcept { MapEvent(4, socket); return reinterpret_cast<PacketWriter *>(socket); };
	connection.map_end = [](PacketWriter *) noexcept {};
	connection.map_enqueue = [](PacketWriter *) noexcept {};
	connection.map_front = [](PacketWriter *writer) noexcept -> Packet * { return MapIndex(writer) == 1 ? map_done : nullptr; };
	connection.admin_error = [](void *socket, uint8_t) noexcept { MapEvent(8, socket); };
	uint8_t on = 4, connected = 4;
	connection.clients_on = &on;
	connection.clients_connected = &connected;
	OpenTTDNetworkFrameServices network{};
	network.command = &command;
	network.connection = &connection;
	auto dispatch = GetRustNetworkDispatchServices();
	dispatch.client_id = [](OpenTTDNetworkGameSocket *socket) noexcept { return static_cast<uint32_t>(MapIndex(socket->game)); };
	network.dispatch = &dispatch;
	OpenTTDNetworkTcpServices tcp{};
	tcp.valid = [](const NetworkTCPSocketHandler *) noexcept { return true; };
	tcp.writable = [](const NetworkTCPSocketHandler *socket) noexcept { MapEvent(9, const_cast<void *>(static_cast<const void *>(socket))); return true; };
	tcp.packet_record = [](Packet *packet) noexcept { return packet->Record(); };
	tcp.send_front = [](NetworkTCPSocketHandler *) noexcept -> OpenTTDPacketState * { return nullptr; };
	network.tcp = &tcp;
	network.socket_delete = [](void *socket) noexcept { MapEvent(10, socket); };
	auto packet_services = Packet::Services();
	packet_services.has_quit = [](const NetworkSocketHandler *) noexcept { return false; };
	network.packet = &packet_services;
	/* SendWait is observed at its enqueue; no other step records an event in between. */
	tcp.packet_enqueue = [](NetworkTCPSocketHandler *socket, OpenTTDPacketState *state) noexcept { Packet packet(*state); if (packet.GetPacketType() == PACKET_SERVER_WAIT) MapEvent(7, socket); };
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

static Packet *tcp_receive_packet;
static std::vector<uint8_t> tcp_input;
static size_t tcp_read_position;

TEST_CASE("Network authentication - server adapters report usability and the active method name")
{
	/* #736/#730: the server handler's CanBeUsed and GetName adapters, which network_admin.cpp's secure
	 * join reaches. Expectations are the original's `usable` and `state` results in auth/endpoints-oracle.txt. */
	struct Password : NetworkAuthenticationPasswordProvider {
		std::string_view GetPassword() const override { return "password"; }
	} password;
	struct Keys : NetworkAuthenticationAuthorizedKeyHandler {
		bool CanBeUsed() const override { return true; }
		bool IsAllowed(std::string_view) const override { return false; }
	} keys;
	struct Case { bool password, keys; uint16_t mask; bool usable; std::string_view name; };
	const Case cases[]{
		{false, false, 7, true, "X25519-KeyExchangeOnly-server"}, // no-password
		{true, true, 7, true, "X25519-AuthorizedKey-server"}, // key-before-password
		{true, true, 0, false, "Unknown"}, // no-supported-methods
		{true, true, 2, true, "X25519-PAKE-server"}, // only-password-supported
	};
	for (const auto &test : cases) {
		auto server = NetworkAuthenticationServerHandler::Create(test.password ? &password : nullptr, test.keys ? &keys : nullptr, NetworkAuthenticationMethodMask(test.mask));
		CHECK(server->CanBeUsed() == test.usable);
		CHECK(server->GetName() == test.name);
	}
}

static NetworkSocketHandler *dispatch_base;
static uint dispatch_creates, dispatch_deletes, dispatch_closes, dispatch_position;
static bool dispatch_ended, dispatch_cleanup_order, dispatch_previous_live;
TEST_CASE("Network game receive - encrypted dispatch preserves consumption and native unwind")
{
	NetworkSocketHandler handshake;
	/* #658: the key-exchange-only handlers come from the production factories with a one-method mask. */
	X25519SecretKey client_secret{};
	client_secret[0] = 1;
	std::string client_secret_text = FormatArrayAsHex(client_secret), client_public_text;
	auto client_handler = NetworkAuthenticationClientHandler::Create({}, client_secret_text, client_public_text);
	auto server_handler = NetworkAuthenticationServerHandler::Create(nullptr, nullptr, NetworkAuthenticationMethodMask{NetworkAuthenticationMethod::X25519_KeyExchangeOnly});
	auto &client_auth = *client_handler;
	auto &server_auth = *server_handler;
	Packet request(&handshake, PacketType{});
	server_auth.SendRequest(request);
	request.PrepareToSend(); REQUIRE(request.PrepareToRead()); request.Recv_uint8();
	REQUIRE(client_auth.ReceiveRequest(request) == NetworkAuthenticationClientHandler::RequestResult::ReadyForResponse);
	Packet response(&handshake, PacketType{});
	REQUIRE(client_auth.SendResponse(response));
	response.PrepareToSend(); REQUIRE(response.PrepareToRead()); response.Recv_uint8();
	REQUIRE(server_auth.ReceiveResponse(response) == NetworkAuthenticationServerHandler::ResponseResult::Authenticated);
	Packet enable(&handshake, PacketType{});
	server_auth.SendEnableEncryption(enable);
	enable.PrepareToSend(); REQUIRE(enable.PrepareToRead()); enable.Recv_uint8();
	REQUIRE(client_auth.ReceiveEnableEncryption(enable));

	struct Case { uint8_t type, status, role; bool quit, throws; uint8_t result, consumed, waiting, closes; int64_t timestamp; size_t limit, remaining; uint16_t command = 71; uint8_t callback = 0; size_t queued = 0; };
	const Case cases[]{
		{PACKET_SERVER_WAIT, 6, 0, false, false, 0, 20, 42, 0, 321, 0, 0},
		{PACKET_SERVER_COMMAND, 8, 0, false, false, 0, 34, 17, 0, 321, 0, 0, 327, 0, 1}, // command 327 narrows to CMD_PAUSE
		{PACKET_SERVER_COMMAND, 0, 0, false, false, 5, 19, 17, 0, 321, 0, 0},
		{PACKET_SERVER_COMMAND, 8, 0, false, false, 5, 27, 17, 0, 321, 0, 0, 255}, // invalid ID still consumes frame/my_cmd
		{PACKET_SERVER_COMMAND, 8, 0, false, false, 5, 34, 17, 0, 321, 0, 0, 71, 255},
		{PACKET_SERVER_WAIT, 0, 0, false, false, 5, 19, 17, 0, 321, 0, 0},
		{255, 0, 2, false, false, 5, 19, 17, 1, 321, 0, 0},
		{PACKET_SERVER_WAIT, 0, 2, true, false, 5, 18, 17, 1, 77, 0, 0},
		{PACKET_SERVER_RCON, 8, 0, false, true, 0, 19, 17, 0, 321, 0, 0},
		{PACKET_SERVER_WAIT, 0, 1, false, false, 0, 0, 17, 0, 77, 0, 0},
		{PACKET_SERVER_WAIT, 0, 1, false, false, 5, 19, 17, 0, 321, 1, std::numeric_limits<size_t>::max() - 18},
	};
	for (const auto &test : cases) {
		CAPTURE(test.type, test.status, test.quit, test.throws);
		class SendingSocket : public NetworkSocketHandler {
		public:
			void Send(std::unique_ptr<NetworkEncryptionHandler> handler) { this->send_encryption_handler = std::move(handler); }
		} sender;
		class ReceivingSocket : public NetworkGameSocketHandler {
		public:
			ReceivingSocket() : NetworkGameSocketHandler(INVALID_SOCKET, 0) {}
			NetworkRecvStatus CloseConnection(NetworkRecvStatus result) override { return result; }
			void Receive(std::unique_ptr<NetworkEncryptionHandler> handler) { this->receive_encryption_handler = std::move(handler); }
		} receiver;
		sender.Send(server_auth.CreateServerToClientEncryptionHandler());
		receiver.Receive(client_auth.CreateServerToClientEncryptionHandler());
		if (test.quit) receiver.MarkClosed();
		dispatch_base = &receiver;
		Packet source(&sender, test.type);
		if (test.type == PACKET_SERVER_COMMAND) {
			source.Send_uint8(255);
			source.Send_uint16(test.command);
			source.Send_uint16(0x1234);
			const uint8_t args[]{0, 1}; // CMD_PAUSE(PauseMode::Normal, true)
			source.Send_buffer(args);
			source.Send_uint8(test.callback);
			source.Send_uint32(UINT32_MAX);
			source.Send_bool(true);
		} else {
			source.Send_uint8(42);
		}
		source.PrepareToSend();
		tcp_input.assign(source.Record()->data, source.Record()->data + source.Size());
		tcp_read_position = 0;
		tcp_receive_packet = nullptr;
		dispatch_creates = dispatch_deletes = dispatch_closes = dispatch_position = 0;
		dispatch_ended = dispatch_previous_live = false;
		dispatch_cleanup_order = true;
		struct PendingPacket { ~PendingPacket() { delete std::exchange(tcp_receive_packet, nullptr); } } pending;
		auto tcp = GetRustNetworkTcpServices();
		tcp.valid = [](const NetworkTCPSocketHandler *) noexcept { return true; };
		tcp.base_socket = [](NetworkTCPSocketHandler *) noexcept { return dispatch_base; };
		tcp.receive_packet = [](NetworkTCPSocketHandler *) noexcept -> OpenTTDPacketState * { return tcp_receive_packet ? tcp_receive_packet->Record() : nullptr; };
		tcp.receive_create = [](NetworkTCPSocketHandler *, OpenTTDPacketState *record) noexcept {
			if (++dispatch_creates == 2) dispatch_previous_live = dispatch_deletes == 0;
			tcp_receive_packet = new Packet(*record);
			return tcp_receive_packet->Record();
		};
		tcp.receive_bytes = [](NetworkTCPSocketHandler *, uint8_t *data, size_t size) noexcept -> intptr_t {
			if (tcp_read_position == tcp_input.size()) return -1;
			size_t count = std::min(size, tcp_input.size() - tcp_read_position);
			std::copy_n(tcp_input.data() + tcp_read_position, count, data);
			tcp_read_position += count;
			return count;
		};
		tcp.receive_take = [](NetworkTCPSocketHandler *) noexcept { return std::exchange(tcp_receive_packet, nullptr); };
		tcp.error_would_block = [](const NetworkError *) noexcept { return true; };
		tcp.packet_record = [](Packet *packet) noexcept { dispatch_position = packet->Record()->position; return packet->Record(); };
		tcp.packet_delete_shell = [](Packet *packet) noexcept { dispatch_deletes++; dispatch_cleanup_order &= dispatch_ended; Packet::DeleteShell(packet); };
		auto dispatch = GetRustNetworkDispatchServices();
		dispatch.server_rcon = [](OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *frame) noexcept -> uint8_t { SessionException(frame, [] { throw 574; }); return NETWORK_RECV_STATUS_OKAY; };
		dispatch.packet_end = [](OpenTTDSessionFrame *frame) noexcept { frame->packet_load.reset(); frame->packet_text.reset(); dispatch_ended = true; };
		OpenTTDNetworkClientServices client{};
		client.dirty_join = []() noexcept {};
		uint8_t join_status = 0, waiting = 17;
		client.join_status = &join_status;
		client.join_waiting = &waiting;
		auto company = GetRustCompanyServices();
		company.network_server = []() noexcept { return false; };
		auto command = GetRustNetworkCommandServices();
		command.company = &company;
		command.console_bad_server = [](OpenTTDNetworkCommandDecodeError) noexcept {};
		OpenTTDNetworkConnectionServices connection{};
		auto network = GetRustNetworkFrameServices();
		network.client = &client;
		network.command = &command;
		network.connection = &connection;
		network.dispatch = &dispatch;
		network.tcp = &tcp;
		tcp.now = []() noexcept -> int64_t { return 321; };
		network.query_close = [](void *, uint8_t result) noexcept { dispatch_closes++; return result; };
		auto &game = static_cast<OpenTTDNetworkGameSocket &>(receiver);
		game.status = test.status;
		game.role = test.role;
		game.last_packet_ns = 77;
		game.sync->receive_limit = test.limit;
		OpenTTDSessionFrame frame;
		openttd_rust_network_receive_packets(&GetRustSessionServices(), &network, &game, &frame, &frame.network_actions);
		if (test.throws) { CHECK_THROWS_AS(frame.Rethrow(), int); } else { CHECK_NOTHROW(frame.Rethrow()); }
		CHECK(openttd_rust_network_actions_result(&frame.network_actions) == test.result);
		CHECK(dispatch_position == test.consumed);
		CHECK(receiver.incoming_queue.Count() == test.queued);
		if (test.queued != 0) {
			const auto &cp = receiver.incoming_queue.data[0];
			CHECK(cp.cmd == 71);
			CHECK(cp.company == 255);
			CHECK(cp.err_msg == 0x1234);
			CHECK(cp.frame == UINT32_MAX);
			CHECK(cp.my_cmd);
			CHECK(cp.callback == nullptr);
			CHECK(std::vector<uint8_t>(cp.data.data, cp.data.data + cp.data.size) == std::vector<uint8_t>{0, 1});
		}
		CHECK(waiting == test.waiting);
		CHECK(dispatch_closes == test.closes);
		CHECK(game.last_packet_ns == test.timestamp);
		CHECK(dispatch_deletes == (test.consumed == 0 ? 0 : 1));
		CHECK(game.sync->receive_limit == test.remaining);
		CHECK(dispatch_cleanup_order);
		if (test.result == 0 && !test.throws && test.consumed != 0) CHECK(dispatch_previous_live);
	}
}

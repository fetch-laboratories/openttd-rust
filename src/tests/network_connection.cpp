/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection.cpp Admission behavior at the native packet boundary. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../network/core/packet.h"
#include "../newgrf_config.h"
#include "../rust/network_client_ffi.h"
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

	GRFIdentifier identifier;
	const auto &services = GetRustNetworkClientServices();
	CHECK(services.check_grf(&packet, &identifier));
	CHECK_FALSE(services.check_grf(&packet, &identifier));
	CHECK(identifier.md5sum == MD5Hash{});
}

static uint8_t map_status;
TEST_CASE("Network map load - recorded exception stops cleanup and reaches native caller")
{
	AutoRestoreBackup<GameMode> mode(_game_mode, GM_MENU);
	map_status = 6; // Client STATUS_MAP.
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
	network.packet_recv_uint32 = [](void *) noexcept { return GetRustNetworkFrameState().frame_counter; };
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
	CHECK(openttd_rust_network_receive_ack(&network, nullptr, nullptr, &status, &sync, nullptr, &frame) == 0);
	CHECK(status == 9); // Server STATUS_ACTIVE.
	CHECK(sync.last_token_frame == GetRustNetworkFrameState().frame_counter);
	CHECK(sync.last_frame == 123);
	CHECK(sync.last_frame_server == 456);
	CHECK_THROWS_AS(frame.Rethrow(), int);
}

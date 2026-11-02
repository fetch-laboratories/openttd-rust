/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection.cpp Admission behavior at the native packet boundary. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../network/core/packet.h"
#include "../newgrf_config.h"
#include "../rust/network_client_ffi.h"

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

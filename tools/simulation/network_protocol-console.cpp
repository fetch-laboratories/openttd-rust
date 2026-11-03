/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_protocol-console.cpp Loopback observation after production packet decryption. */
#include "stdafx.h"
#include "command_func.h"
#include "company_cmd.h"
#include "console_internal.h"
#include "console_func.h"
#include "core/random_func.hpp"
#include "core/string_consumer.hpp"
#include "network/network_client.h"
#include "network/network_server.h"
#include "road_cmd.h"
#include "settings_type.h"
#include "timer/timer.h"
#include "timer/timer_game_realtime.h"
#include "timer/timer_game_tick.h"
#include <fstream>

static bool armed, held_map, saved;
static uint connections;
static uint64_t stop_tick;

static void ObservePacket(NetworkTCPSocketHandler *socket, Packet *packet)
{
	if (packet == nullptr) return;
	auto *game = dynamic_cast<NetworkGameSocketHandler *>(socket);
	if (game == nullptr) return;
	Packet copy(*packet);
	std::vector<uint8_t> payload(copy.RemainingBytesToTransfer());
	copy.Recv_bytes(payload);
	if (payload.empty() || payload[0] == PACKET_SERVER_MAP_DATA) return;
	std::string hex;
	for (auto byte : payload) fmt::format_to(std::back_inserter(hex), "{:02x}", byte);
	fmt::print(stderr, "NET574 packet {} {} {} {} {}\n", _network_server, game->client_id, _frame_counter, payload[0], hex);
	if (!held_map && payload[0] == PACKET_SERVER_MAP_DONE && std::ifstream("hold-map").good()) {
		held_map = true;
		/* The peer keeps receiving commands while this joiner awaits its map completion. */
		std::ifstream release("map-release");
		std::string line;
		if (!std::getline(release, line) || line != "release") std::abort();
	}
}

#ifdef WITH_RUST
#include "rust/network_tcp_ffi.h"
extern "C" const OpenTTDNetworkTcpServices &__real__Z25GetRustNetworkTcpServicesv();
extern "C" const OpenTTDNetworkTcpServices &__wrap__Z25GetRustNetworkTcpServicesv()
{
	static const auto &original = __real__Z25GetRustNetworkTcpServicesv();
	static const auto services = [] {
		auto result = original;
		result.receive_take = [](NetworkTCPSocketHandler *socket) noexcept {
			auto *packet = original.receive_take(socket);
			ObservePacket(socket, packet);
			return packet;
		};
		return result;
	}();
	return services;
}
#else
extern "C" std::unique_ptr<Packet> __real__ZN23NetworkTCPSocketHandler13ReceivePacketEv(NetworkTCPSocketHandler *);
extern "C" std::unique_ptr<Packet> __wrap__ZN23NetworkTCPSocketHandler13ReceivePacketEv(NetworkTCPSocketHandler *socket)
{
	auto packet = __real__ZN23NetworkTCPSocketHandler13ReceivePacketEv(socket);
	ObservePacket(socket, packet.get());
	return packet;
}
#endif

extern "C" void __real__Z30CcPlaySound_CONSTRUCTION_OTHER8CommandsRK11CommandCostN10StrongType7TypedefIj12TileIndexTagJNS3_7CompareENS3_7IntegerENS3_10CompatibleIiEENS8_IlEEEEE(Commands, const CommandCost &, TileIndex);
extern "C" void __wrap__Z30CcPlaySound_CONSTRUCTION_OTHER8CommandsRK11CommandCostN10StrongType7TypedefIj12TileIndexTagJNS3_7CompareENS3_7IntegerENS3_10CompatibleIiEENS8_IlEEEEE(Commands cmd, const CommandCost &result, TileIndex tile)
{
	fmt::print(stderr, "NET574 callback {} {} {} {}\n", _network_own_client_id, cmd, result.Succeeded(), result.GetCost());
	__real__Z30CcPlaySound_CONSTRUCTION_OTHER8CommandsRK11CommandCostN10StrongType7TypedefIj12TileIndexTagJNS3_7CompareENS3_7IntegerENS3_10CompatibleIiEENS8_IlEEEEE(cmd, result, tile);
}

static IntervalTimer<TimerGameRealtime> control_timer({std::chrono::milliseconds{1}, TimerGameRealtime::ALWAYS}, [](auto) {
	if (!armed) return;
	std::ifstream trigger("trigger.txt");
	std::string action;
	if (!std::getline(trigger, action)) return;
	trigger.close();
	std::remove("trigger.txt");
	if (action == "issue") {
		for (uint i = 0; i < 24; i++) Command<CMD_RENAME_PRESIDENT>::Post(STR_NULL, CcPlaySound_CONSTRUCTION_OTHER, fmt::format("Protocol574_{}", i));
	} else {
		IConsoleCmdExec(action);
	}
	fmt::print(stderr, "NET574 action {}\n", action);
});

static IntervalTimer<TimerGameTick> stop_timer({TimerGameTick::NONE, 1}, [](auto) {
	if (!armed || saved || TimerGameTick::counter < stop_tick) return;
	saved = true;
	IConsoleCmdExec("save stop");
	fmt::print(stderr, "NET574 saved {} {} {}\n", TimerGameTick::counter, _random.state[0], _random.state[1]);
});

static bool ProtocolCommand(std::span<std::string_view> argv)
{
	if (argv.size() != 2) return false;
	armed = true;
	stop_tick = ParseInteger<uint64_t>(std::getenv("OPENTTD_PROTOCOL_STOP")).value();
	if (_network_server) {
		_settings_client.network.commands_per_frame = 1;
		_settings_client.network.commands_per_frame_server = 1;
		_settings_client.network.frame_freq = 0;
	}
	fmt::print(stderr, "NET574 ready {} {} {} {}\n", argv[1], ++connections, _network_own_client_id, TimerGameTick::counter);
	return true;
}

extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("network574", ProtocolCommand);
}

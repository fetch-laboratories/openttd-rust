/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_protocol-console.cpp Loopback observation after production packet decryption. */
#include "stdafx.h"
#include "command_func.h"
#include "company_cmd.h"
#include "company_base.h"
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
#include "viewport_func.h"
#include "window_func.h"
#include "window_gui.h"
#include <fstream>

static bool armed, held_map, saved, awaiting_key;
static uint connections;
static uint64_t stop_tick;

static void ObservePayload(NetworkGameSocketHandler *game, std::span<const uint8_t> payload)
{
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
		/* The I/O leaf identifies a completed packet. The dispatch clock leaf runs
		 * after production decryption and type consumption, before handler effects. */
		static thread_local OpenTTDNetworkTcpSocket *receiving;
		static thread_local Packet *completed;
		result.receive_bytes = [](OpenTTDNetworkTcpSocket *socket, uint8_t *data, size_t size) noexcept {
			auto count = original.receive_bytes(socket, data, size);
			auto *state = reinterpret_cast<OpenTTDPacketState *>(socket->receive);
			if (count > 0 && state->size > Packet::EncodedLengthOfPacketSize() && state->position + static_cast<size_t>(count) == state->size) {
				receiving = socket;
				completed = socket->receive;
			}
			return count;
		};
		result.packet_delete_shell = [](Packet *packet) noexcept {
			if (completed == packet) completed = nullptr;
			original.packet_delete_shell(packet);
		};
		result.now = []() noexcept {
			if (completed != nullptr && receiving->receive == nullptr) {
				auto *packet = std::exchange(completed, nullptr);
				NetworkGameSocketHandler *game = nullptr;
				if (auto *client = ClientNetworkGameSocketHandler::my_client; client != nullptr && client->TcpRecord() == receiving) game = client;
				for (auto *client : NetworkClientSocket::Iterate()) {
					if (client->TcpRecord() == receiving) game = client;
				}
				if (game != nullptr) {
					auto *state = reinterpret_cast<const OpenTTDPacketState *>(packet);
					/* HandlePacket consumed only the type before requesting its timestamp. */
					size_t position = state->position - Packet::EncodedLengthOfPacketType();
					ObservePayload(game, {state->data + position, state->size - position});
				}
			}
			return original.now();
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
	if (auto *game = dynamic_cast<NetworkGameSocketHandler *>(socket); game != nullptr && packet != nullptr) {
		Packet copy(*packet);
		std::vector<uint8_t> payload(copy.RemainingBytesToTransfer());
		copy.Recv_bytes(payload);
		ObservePayload(game, payload);
	}
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
	if (awaiting_key && Company::Get(CompanyID{0})->allow_list.Contains(std::getenv("OPENTTD_PROTOCOL_KEY"))) {
		awaiting_key = false;
		fmt::print(stderr, "NET574 allowed\n");
	}
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
	if (!armed || saved || !_networking || (!_network_server && !MyClient::IsConnected()) || TimerGameTick::counter < stop_tick) return;
	saved = true;
	/* Equal local camera inputs retain the full VIEW chunk comparison. */
	auto *window = GetMainWindow();
	assert(window != nullptr && window->viewport != nullptr);
	window->viewport->scrollpos_x = window->viewport->scrollpos_y = 0;
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
		Backup<CompanyID> current(_current_company, CompanyID{0});
		Command<CMD_COMPANY_ALLOW_LIST_CTRL>::Post(STR_NULL, CALCA_ADD, std::string(std::getenv("OPENTTD_PROTOCOL_KEY")));
		awaiting_key = true;
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

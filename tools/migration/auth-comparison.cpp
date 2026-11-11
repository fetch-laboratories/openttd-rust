/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth-comparison.cpp Prescribed-entropy public authentication endpoints. */
#include "stdafx.h"
#include "debug.h"
#include "core/random_func.hpp"
#include <charconv>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include "network/network_crypto.cpp"

int _debug_net_level = 9;
static std::vector<size_t> random_sizes;
static uint8_t random_seed;
static size_t logs, asks, replies, password_reads, authorized_checks, allowed_checks;
static std::string last_log;
void RandomBytesWithFallback(std::span<uint8_t> output)
{
	for (size_t i = 0; i < output.size(); ++i) output[i] = static_cast<uint8_t>(random_seed + i + random_sizes.size() * 31);
	random_sizes.push_back(output.size());
}
void DebugPrint(std::string_view category, int level, std::string &&message) { ++logs; last_log = fmt::format("{}:{}:{}", category, level, message); }
[[noreturn]] void NOT_REACHED(const std::source_location) { std::abort(); }
#if __has_include("rust/debug_ffi.h")
#include "rust/debug_ffi.h"
/* The candidate's Rust Debug reads this harness's net level and prints through its DebugPrint; this standalone entry also binds native assertions. */
static const int _debug_other_level = 0;
/* The harness loads no NewGRF, so no GRF message can reach this. */
[[noreturn]] static void UnreachableGrfMsg(int, const char *, size_t) noexcept { std::abort(); }
static const OpenTTDDebugServices _debug_services{
	.console = &_debug_other_level, .desync = &_debug_other_level, .driver = &_debug_other_level, .fontcache = &_debug_other_level,
	.gamelog = &_debug_other_level, .grf = &_debug_other_level, .grf_msg = &UnreachableGrfMsg,
	.map = &_debug_other_level, .misc = &_debug_other_level, .net = &_debug_net_level, .oldloader = &_debug_other_level,
	.print = [](const char *category, size_t category_size, int level, const char *message, size_t size) noexcept { DebugPrint({category, category_size}, level, std::string(message, size)); },
	.script = &_debug_other_level, .sl = &_debug_other_level, .sprite = &_debug_other_level, .yapf = &_debug_other_level,
};
[[maybe_unused]] static const bool _rust_bound = (openttd_rust_assert_bind(RustAssertCondition, nullptr), openttd_rust_debug_bind(&_debug_services), true);
#endif
[[noreturn]] void AssertFailedError(std::string_view, const std::source_location) { std::abort(); }

static unsigned Number(std::string_view input, int base = 10)
{
	unsigned result;
	if (std::from_chars(input.data(), input.data() + input.size(), result, base).ec != std::errc{}) throw std::runtime_error("number");
	return result;
}
static std::vector<uint8_t> Unhex(std::string_view input)
{
	if (input == "-") return {};
	std::vector<uint8_t> bytes;
	for (size_t i = 0; i < input.size(); i += 2) bytes.push_back(static_cast<uint8_t>(Number(input.substr(i, 2), 16)));
	return bytes;
}
static std::string Text(std::string_view input)
{
	auto bytes = Unhex(input);
	return std::string(bytes.begin(), bytes.end());
}
static std::string Hex(std::span<const uint8_t> input)
{
	static constexpr char alphabet[] = "0123456789abcdef";
	std::string result;
	for (auto byte : input) { result.push_back(alphabet[byte >> 4]); result.push_back(alphabet[byte & 15]); }
	return result.empty() ? "-" : result;
}
static NetworkSocketHandler socket_handler;
static std::string Wire(Packet &packet)
{
	packet.PrepareToSend();
	std::vector<uint8_t> bytes;
	packet.TransferOut([&](std::span<const uint8_t> src) { bytes.assign(src.begin(), src.end()); return src.size(); });
	return Hex(bytes);
}
static Packet Read(std::string_view hex)
{
	auto bytes = Unhex(hex);
	Packet packet(&socket_handler, COMPAT_MTU, bytes.size());
	packet.TransferIn([&](std::span<uint8_t> dest) { std::ranges::copy(bytes, dest.begin()); return bytes.size(); });
	if (!packet.PrepareToRead()) throw std::runtime_error("invalid wire");
	packet.Recv_uint8();
	return packet;
}

static std::unique_ptr<NetworkAuthenticationClientHandler> client;
static std::unique_ptr<NetworkAuthenticationServerHandler> server;
static std::string server_password, authorized_key, reentrant_response;
static bool reenter;
struct PasswordProvider : NetworkAuthenticationPasswordProvider {
	std::string_view GetPassword() const override { ++password_reads; return server_password; }
};
struct AuthorizedKeys : NetworkAuthenticationAuthorizedKeyHandler {
	bool CanBeUsed() const override { ++authorized_checks; return !authorized_key.empty(); }
	bool IsAllowed(std::string_view key) const override { ++allowed_checks; return key == authorized_key; }
};
struct PasswordRequest : NetworkAuthenticationPasswordRequestHandler {
	std::string reply;
	void AskUserForPassword(std::shared_ptr<NetworkAuthenticationPasswordRequest> request) override { ++asks; request->Reply(this->reply); }
	void SendResponse() override
	{
		++replies;
		if (reenter) {
			Packet packet(&socket_handler, PacketType{});
			if (!client->SendResponse(packet)) throw std::runtime_error("reentrant response");
			reentrant_response = Wire(packet);
		}
	}
};

int main()
{
	PasswordProvider provider;
	AuthorizedKeys authorized;
	std::shared_ptr<PasswordRequest> password;
	std::unique_ptr<NetworkEncryptionHandler> stream;
	std::string line;
	while (std::getline(std::cin, line)) {
		std::istringstream input(line);
		std::string command, a, b, c, d;
		input >> command >> a >> b >> c >> d;
		try {
			if (command == "server" || command == "client") {
				stream.reset(); server.reset(); client.reset(); password.reset();
				random_sizes.clear(); logs = asks = replies = password_reads = authorized_checks = allowed_checks = 0; last_log.clear(); reentrant_response.clear();
				if (command == "server") {
					server_password = a == "none" ? "" : Text(a);
					authorized_key = b == "none" || b == "-" ? "" : b;
					random_seed = static_cast<uint8_t>(Number(d));
					server = NetworkAuthenticationServerHandler::Create(a == "none" ? nullptr : &provider, b == "none" ? nullptr : &authorized, NetworkAuthenticationMethodMask(static_cast<uint16_t>(Number(c))));
				} else {
					std::string secret = a, public_key;
					random_seed = static_cast<uint8_t>(Number(c));
					reenter = d == "1";
					password = std::make_shared<PasswordRequest>(); password->reply = Text(b);
					client = NetworkAuthenticationClientHandler::Create(password, secret, public_key);
				}
				std::cout << "ok";
			} else if (command == "public") {
				X25519SecretKey secret; auto bytes = Unhex(a); std::ranges::copy(bytes, secret.begin());
				std::cout << FormatArrayAsHex(secret.CreatePublicKey());
			} else if (command == "request" || command == "response" || command == "nonce") {
				Packet packet(&socket_handler, PacketType{});
				bool ok = true;
				if (command == "request") server->SendRequest(packet);
				if (command == "response") ok = client->SendResponse(packet);
				if (command == "nonce") server->SendEnableEncryption(packet);
				std::cout << ok << ' ' << Wire(packet);
			} else if (command == "recv-request" || command == "recv-response" || command == "recv-nonce") {
				auto packet = Read(a);
				int result;
				if (command == "recv-request") result = static_cast<int>(client->ReceiveRequest(packet));
				else if (command == "recv-response") result = static_cast<int>(server->ReceiveResponse(packet));
				else result = client->ReceiveEnableEncryption(packet);
				std::cout << result << ' ' << packet.RemainingBytesToTransfer();
			} else if (command == "state") {
				auto *handler = client ? static_cast<NetworkAuthenticationHandler *>(client.get()) : static_cast<NetworkAuthenticationHandler *>(server.get());
				std::cout << handler->GetName() << ' ' << static_cast<int>(handler->GetAuthenticationMethod()) << ' ' << logs << ' ' << asks << ' ' << replies << ' ' << password_reads << ' ' << authorized_checks << ' ' << allowed_checks << ' ' << Hex(std::span(reinterpret_cast<const uint8_t *>(last_log.data()), last_log.size())) << ' ';
				for (auto size : random_sizes) std::cout << size << ',';
			} else if (command == "usable") {
				std::cout << server->CanBeUsed();
			} else if (command == "peer") {
				std::cout << server->GetPeerPublicKey();
			} else if (command == "reentrant") {
				std::cout << (reentrant_response.empty() ? "-" : reentrant_response);
			} else if (command == "init-stream") {
				auto *handler = client ? static_cast<NetworkAuthenticationHandler *>(client.get()) : static_cast<NetworkAuthenticationHandler *>(server.get());
				stream = a == "0" ? handler->CreateClientToServerEncryptionHandler() : handler->CreateServerToClientEncryptionHandler();
				std::cout << stream->MACSize();
			} else if (command == "encrypt" || command == "decrypt") {
				auto message = Unhex(a);
				std::array<uint8_t, 16> mac{};
				bool ok = true;
				if (command == "encrypt") stream->Encrypt(mac, message);
				else { auto raw_mac = Unhex(b); std::ranges::copy(raw_mac, mac.begin()); ok = stream->Decrypt(mac, message); }
				std::cout << ok << ' ' << Hex(mac) << ' ' << Hex(message);
			} else throw std::runtime_error("unknown command");
			std::cout << '\n' << std::flush;
		} catch (const std::exception &error) { std::cout << "error " << error.what() << '\n' << std::flush; }
	}
}

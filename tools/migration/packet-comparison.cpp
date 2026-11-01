/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file packet-comparison.cpp Bounded actual Packet byte/callback/state observations. */
#include "stdafx.h"
#include "network/core/packet.h"
#include <cstdio>
#include <cstdlib>
#include <stdexcept>

[[noreturn]] void NOT_REACHED(const std::source_location) { throw std::runtime_error("NOT_REACHED"); }
[[noreturn]] void AssertFailedError(std::string_view, const std::source_location) { throw std::runtime_error("assertion"); }

struct Socket : NetworkSocketHandler {
	using NetworkSocketHandler::receive_encryption_handler;
	using NetworkSocketHandler::send_encryption_handler;
	unsigned derived_closed = 0;
	void MarkClosed() { ++derived_closed; }
};
static void Bytes(std::span<const uint8_t> bytes)
{
	for (uint8_t byte : bytes) std::printf("%02x", byte);
	std::printf("/");
}
static void State(const char *name, Packet &packet, Socket &socket)
{
	std::printf("%s %zu %zu %d %d %u ", name, packet.Size(), packet.RemainingBytesToTransfer(), packet.HasPacketSizeData(), socket.HasClientQuit(), socket.derived_closed);
	Packet copy = packet;
	copy.TransferOut([](std::span<const uint8_t> bytes) { Bytes(bytes); return static_cast<ssize_t>(bytes.size()); });
	std::printf("\n");
}
static Packet Read(Socket &socket, std::span<const uint8_t> bytes, size_t limit = 70000)
{
	Packet packet(&socket, limit, bytes.size());
	packet.TransferIn([&](std::span<uint8_t> output) { std::ranges::copy(bytes, output.begin()); return static_cast<ssize_t>(bytes.size()); });
	(void)packet.PrepareToRead();
	return packet;
}
struct Encryption : NetworkEncryptionHandler {
	Packet *packet = nullptr;
	unsigned mode = 0;
	size_t mac_size = 2;
	size_t MACSize() const override
	{
		std::printf("mac %u %zu\n", mode, mac_size);
		if (mode == 4) throw std::runtime_error("MACSize");
		if (mode == 5) packet->Recv_uint8();
		return mac_size;
	}
	void Encrypt(std::span<uint8_t> mac, std::span<uint8_t> message) override
	{
		std::printf("encrypt %zu %zu ", mac.size(), message.size()); Bytes(message); std::printf("\n");
		if (!mac.empty()) mac[0] = 0xA1;
		if (!message.empty()) message[0] = 0xB2;
		if (mode == 2 || mode == 3) packet->Recv_uint8();
		if (mode == 1 || mode == 3) throw std::runtime_error("Encrypt");
	}
	bool Decrypt(std::span<uint8_t> mac, std::span<uint8_t> message) override
	{
		std::printf("decrypt %zu %zu ", mac.size(), message.size()); Bytes(message); std::printf("\n");
		if (!mac.empty()) mac[0] = 0xC3;
		if (!message.empty()) message[0] = 0xD4;
		if (mode == 2 || mode == 3) packet->Recv_uint8();
		if (mode == 1 || mode == 3) throw std::runtime_error("Decrypt");
		return mode != 0;
	}
};
int main()
{
	/* All scalar widths, canonical/noncanonical boolean and copy independence. */
	for (uint64_t value : {UINT64_C(0), UINT64_C(1), UINT64_C(0xFEDCBA9876543210), UINT64_MAX}) {
		Socket socket;
		Packet packet(&socket, PacketType{7}, 100);
		packet.Send_bool(false); packet.Send_bool(true); packet.Send_uint8(128);
		packet.Send_uint8(static_cast<uint8_t>(value)); packet.Send_uint16(static_cast<uint16_t>(value)); packet.Send_uint32(static_cast<uint32_t>(value)); packet.Send_uint64(value);
		packet.PrepareToSend(); State("binary-wire", packet, socket);
		Packet copy = packet; Packet assigned(&socket, PacketType{0}, 100); assigned = packet;
		(void)copy.PrepareToRead(); (void)assigned.PrepareToRead();
		auto type = copy.Recv_uint8(); bool first = copy.Recv_bool(), second = copy.Recv_bool(), third = copy.Recv_bool();
		auto byte = copy.Recv_uint8(); auto word = copy.Recv_uint16(); auto dword = copy.Recv_uint32(); auto qword = copy.Recv_uint64();
		std::printf("binary-values %u %d %d %d %u %u %u %llu\n", type, first, second, third, byte, word, dword, static_cast<unsigned long long>(qword));
		State("original-independent", packet, socket); State("copy-read", copy, socket); State("assignment", assigned, socket);
	}
	for (size_t length : {size_t{0}, size_t{1}, size_t{8}, size_t{257}}) {
		Socket socket; std::vector<uint8_t> bytes(length, 0xA5);
		Packet packet(&socket, PacketType{3}, 1000); packet.Send_buffer(bytes); packet.PrepareToSend(); (void)packet.PrepareToRead(); packet.Recv_uint8();
		auto result = packet.Recv_buffer(); std::printf("buffer %zu ", length); Bytes(result); std::printf("\n"); State("buffer-state", packet, socket);
	}
	for (size_t size : {size_t{3}, size_t{4}, size_t{5}, size_t{6}}) {
		Socket socket; std::array<uint8_t, 6> bytes{6, 0, 3, 2, 0, 0xAB};
		auto packet = Read(socket, std::span(bytes).first(size)); packet.Recv_uint8(); auto result = packet.Recv_buffer();
		std::printf("truncated %zu ", size); Bytes(result); std::printf("\n"); State("truncated-state", packet, socket);
	}
	for (size_t limit : {size_t{3}, size_t{4}, size_t{8}}) {
		Socket socket; Packet packet(&socket, PacketType{0}, limit); std::array<uint8_t, 8> bytes{1,2,3,4,5,6,7,8};
		auto suffix = packet.Send_bytes(bytes); std::printf("send-bytes %zu %zu %td\n", limit, suffix.size(), suffix.data() - bytes.data()); State("send-bytes-state", packet, socket);
	}
	/* Truncation can split UTF-8; drain the terminator before reading the next field. */
	for (std::string_view input : {"a\xC3\xA9z", "a\xFFz", "a\nb\r\nc\t"}) {
		for (size_t length : {size_t{2}, size_t{3}, size_t{4}, size_t{20}}) {
			for (StringValidationSettings settings : {StringValidationSetting::ReplaceWithQuestionMark, StringValidationSetting::AllowNewline, StringValidationSetting::ReplaceTabCrNlWithSpace}) {
				Socket socket; Packet packet(&socket, PacketType{1}, 100);
				packet.Send_string(input); packet.Send_uint8(0xED); packet.PrepareToSend(); (void)packet.PrepareToRead(); packet.Recv_uint8();
				auto text = packet.Recv_string(length, settings);
				std::printf("string %zu %u ", length, settings.base()); Bytes({reinterpret_cast<const uint8_t *>(text.data()), text.size()});
				auto next = packet.Recv_uint8();
				std::printf("%u %d %zu\n", next, socket.HasClientQuit(), packet.RemainingBytesToTransfer());
			}
		}
	}
	{
		Socket socket; std::array<uint8_t, 4> bytes{4, 0, 'x', 'y'}; auto packet = Read(socket, bytes);
		auto text = packet.Recv_string(20);
		std::printf("unterminated-string "); Bytes({reinterpret_cast<const uint8_t *>(text.data()), text.size()});
		std::printf("%d %zu\n", socket.HasClientQuit(), packet.RemainingBytesToTransfer());
	}
	{
		Socket socket; Packet source(&socket, PacketType{7}, 100); source.Send_uint32(0xABCDEF01); (void)source.PrepareToRead(); source.Recv_uint8();
		Packet moved(std::move(source));
		std::printf("move-source %zu %d %zu\n", source.Size(), source.HasPacketSizeData(), source.RemainingBytesToTransfer());
		Packet assigned(&socket, PacketType{0}, 100); assigned = std::move(moved);
		std::printf("move-result %u %zu %d %zu\n", assigned.Recv_uint32(), moved.Size(), moved.HasPacketSizeData(), moved.RemainingBytesToTransfer());
	}
	/* TCP header-first / UDP full initial storage, rejected framing. */
	for (size_t initial : {size_t{2}, size_t{8}}) for (uint16_t size : {uint16_t{0}, uint16_t{2}, uint16_t{3}, uint16_t{8}, uint16_t{9}}) {
		Socket socket; Packet packet(&socket, size_t{8}, initial);
		packet.TransferIn([&](std::span<uint8_t> bytes) { std::fill(bytes.begin(), bytes.end(), 0xAB); bytes[0] = static_cast<uint8_t>(size); bytes[1] = static_cast<uint8_t>(size >> 8); return ssize_t{2}; });
		bool parsed = packet.ParsePacketSize();
		if (parsed) packet.TransferIn([](std::span<uint8_t> body) { std::fill(body.begin(), body.end(), 0xCC); return static_cast<ssize_t>(body.size()); });
		std::printf("parse %zu %u %d\n", initial, size, parsed); State("parse-state", packet, socket);
	}
	/* Transfer return, exception and reentrant live-cursor commit. */
	for (unsigned mode = 0; mode < 7; ++mode) for (bool incoming : {false,true}) {
		Socket socket; Packet packet(&socket, size_t{20}, size_t{10}); unsigned calls = 0; ssize_t result = 99; bool threw = false;
		auto callback = [&](auto bytes) -> ssize_t {
			++calls; std::printf("transfer-call %u %d %zu\n", mode, incoming, bytes.size());
			if constexpr (!std::is_const_v<typename decltype(bytes)::element_type>) bytes[0] = 0xEE;
			if (mode == 3) throw std::runtime_error("transfer");
			if (mode == 4 || mode == 5) { (void)packet.PrepareToRead(); packet.Recv_uint8(); }
			if (mode == 5) { Packet other(&socket, size_t{20}, size_t{4}); other.TransferIn([](auto) { return ssize_t{1}; }); }
			return mode == 0 ? 0 : mode == 1 ? -1 : 2;
		};
		try { result = incoming ? packet.TransferIn(callback) : packet.TransferOutWithLimit(callback, mode == 6 ? 0 : 4); } catch (const std::runtime_error &) { threw = true; }
		std::printf("transfer-result %u %d %zd %u %d\n", mode, incoming, result, calls, threw); State("transfer-state", packet, socket);
	}
	/* Persistent uint16 narrowing after every byte, even in larger native buffers. */
	for (unsigned width : {1U,2U,4U,8U}) {
		Socket socket; Packet packet(&socket, size_t{70000}, size_t{65543});
		packet.TransferIn([](std::span<uint8_t> bytes) { for (size_t i=0;i<bytes.size();++i) bytes[i]=static_cast<uint8_t>(i); return ssize_t{65535}; });
		uint64_t value = width == 1 ? packet.Recv_uint8() : width == 2 ? packet.Recv_uint16() : width == 4 ? packet.Recv_uint32() : packet.Recv_uint64();
		std::printf("wrap %u %llu %zu\n", width, static_cast<unsigned long long>(value), packet.RemainingBytesToTransfer());
	}
	for (unsigned mode=0;mode<6;++mode) for (bool sending : {false,true}) {
		Socket socket; auto handler=std::make_unique<Encryption>(); auto *observed=handler.get();
		if (sending) socket.send_encryption_handler=std::move(handler); else socket.receive_encryption_handler=std::move(handler);
		Packet packet(&socket, PacketType{7}, 100); packet.Send_uint8(9); packet.Send_uint8(10); packet.Send_uint8(11);
		observed->packet=&packet; observed->mode=mode; bool threw=false,valid=false;
		try { if(sending) packet.PrepareToSend(); else valid=packet.PrepareToRead(); } catch(const std::runtime_error &) { threw=true; }
		std::printf("crypto-result %u %d %d %d\n",mode,sending,valid,threw); State("crypto-state",packet,socket);
	}
	for (size_t mac : {size_t{0},size_t{3},size_t{4}}) {
		Socket socket; auto handler=std::make_unique<Encryption>(); handler->mac_size=mac; socket.receive_encryption_handler=std::move(handler);
		Packet packet(&socket, PacketType{1},100); packet.Send_uint8(2); packet.Send_uint8(3);
		std::printf("strict-mac %zu %d\n",mac,packet.PrepareToRead()); State("strict-mac-state",packet,socket);
	}
	{
		Socket socket; Packet packet(&socket, size_t{10}, size_t{3}); (void)packet.PrepareToRead();
		bool wrapped = packet.CanReadFromPacket(SIZE_MAX);
		bool first = packet.CanReadFromPacket(2), first_closed = socket.HasClientQuit();
		bool second = packet.CanReadFromPacket(2, true), second_closed = socket.HasClientQuit();
		bool third = packet.CanReadFromPacket(0);
		std::printf("bounds %d %d %d %d %d %d\n", wrapped, first, first_closed, second, second_closed, third); State("bounds-state", packet, socket);
	}
	/* Defined Recv_bytes domains only: empty source or destination <= remaining. */
	for (size_t length : {size_t{0},size_t{1},size_t{3}}) {
		Socket socket; std::array<uint8_t,5> wire{5,0,0xA1,0xA2,0xA3}; auto packet=Read(socket,wire); std::array<uint8_t,3> result{};
		std::printf("recv-bytes %zu %zu ",length,packet.Recv_bytes(std::span(result).first(length))); Bytes(result); std::printf("\n"); State("recv-bytes-state",packet,socket);
	}
	{
		Socket socket; std::array<uint8_t, 6> bytes{6,0,0xAA,0xBB,0xCC,0xDD}; auto packet = Read(socket, bytes);
		auto send = std::make_unique<Encryption>(); send->mac_size = 1; send->packet = &packet; auto *observed = send.get();
		auto receive = std::make_unique<Encryption>(); receive->mac_size = 2;
		socket.send_encryption_handler = std::move(send); socket.receive_encryption_handler = std::move(receive);
		std::printf("type-send-handler %u\n", packet.GetPacketType());
		observed->mode = 4; bool threw = false; try { packet.GetPacketType(); } catch (const std::runtime_error &) { threw = true; }
		std::printf("type-throw %d\n", threw);
	}
	{
		Socket socket; std::vector<uint8_t> bytes(65537, 0xA5); Packet packet(&socket, PacketType{1}, 70000);
		packet.Send_buffer(bytes); (void)packet.PrepareToRead(); packet.Recv_uint8();
		auto result = packet.Recv_buffer(); std::printf("narrow-prefix %zu %zu %zu\n", packet.Size(), result.size(), packet.RemainingBytesToTransfer());
		Packet empty(&socket, size_t{10}, size_t{0}); std::array<uint8_t, 1> destination{};
		std::printf("empty-source %zu %u\n", empty.Recv_bytes(destination), destination[0]);
	}

	std::printf("packet companion passed\n");
}

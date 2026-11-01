/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file packet.cpp Basic functions to create, fill and read packets.
 */

#include "../../stdafx.h"
#include "packet.h"
#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDPacketServices.hpp"

#include "../../safeguards.h"

/**
 * Create a packet that is used to read from a network socket.
 * @param cs                The socket handler associated with the socket we are reading from.
 * @param limit             The maximum size of packets to accept.
 * @param initial_read_size The initial amount of data to transfer from the socket into the
 *                          packet. This defaults to just the required bytes to determine the
 *                          packet's size. That default is the wanted for streams such as TCP
 *                          as you do not want to read data of the next packet yet. For UDP
 *                          you need to read the whole packet at once otherwise you might
 *                          loose some the data of the packet, so there you pass the maximum
 *                          size for the packet you expect from the network.
 */
Packet::Packet(NetworkSocketHandler *cs, size_t limit, size_t initial_read_size)
{
	openttd_rust_packet_init_receive(&this->state, cs, limit, initial_read_size);
}

/**
 * Creates a packet to send
 * @param cs    The socket handler associated with the socket we are writing to; could be \c nullptr.
 * @param type  The type of the packet to send
 * @param limit The maximum number of bytes the packet may have. Default is COMPAT_MTU.
 *              Be careful of compatibility with older clients/servers when changing
 *              the limit as it might break things if the other side is not expecting
 *              much larger packets than what they support.
 */
Packet::Packet(NetworkSocketHandler *cs, PacketType type, size_t limit)
{
	if (cs == nullptr || cs->send_encryption_handler == nullptr || cs->send_encryption_handler->GetRustStream() != nullptr) {
		openttd_rust_packet_init_outgoing(&this->state, cs, &Services(), type, limit);
	} else {
		openttd_rust_packet_init_send(&this->state, cs, type, limit, cs->send_encryption_handler->MACSize());
	}
}

/**
 * Writes the packet size from the raw packet from packet->size
 */
void Packet::PrepareToSend()
{
	auto *cs = this->state.socket;
	if (cs == nullptr || cs->send_encryption_handler == nullptr || cs->send_encryption_handler->GetRustStream() != nullptr) {
		openttd_rust_packet_prepare_to_send(&this->state, &Services());
		return;
	}
	/* Native virtual implementations can throw; their frame stays in C++. */
	openttd_rust_packet_begin_send(&this->state);
	size_t mac = cs->send_encryption_handler->MACSize();
	OpenTTDPacketFrame frame;
	openttd_rust_packet_frame(2, this->Size(), mac, &frame);
	cs->send_encryption_handler->Encrypt({this->state.data + 2, mac}, {this->state.data + frame.message, frame.payload});
	openttd_rust_packet_finish_send(&this->state);
}

/**
 * Is it safe to write to the packet, i.e. didn't we run over the buffer?
 * @param bytes_to_write The amount of bytes we want to try to write.
 * @return True iff the given amount of bytes can be written to the packet.
 */
bool Packet::CanWriteToPacket(size_t bytes_to_write)
{
	return openttd_rust_packet_can_write(&this->state, bytes_to_write);
}

/*
 * The next couple of functions make sure we can send
 *  uint8_t, uint16_t, uint32_t and uint64_t endian-safe
 *  over the network. The least significant bytes are
 *  sent first.
 *
 *  So 0x01234567 would be sent as 67 45 23 01.
 *
 * A bool is sent as a uint8_t where zero means false
 *  and non-zero means true.
 */

/**
 * Package a boolean in the packet.
 * @param data The data to send.
 */
void Packet::Send_bool(bool data)
{
	openttd_rust_packet_send_bool(&this->state, data);
}

/**
 * Package a 8 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint8(uint8_t data)
{
	openttd_rust_packet_send_uint8(&this->state, data);
}

/**
 * Package a 16 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint16(uint16_t data)
{
	openttd_rust_packet_send_uint16(&this->state, data);
}

/**
 * Package a 32 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint32(uint32_t data)
{
	openttd_rust_packet_send_uint32(&this->state, data);
}

/**
 * Package a 64 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint64(uint64_t data)
{
	openttd_rust_packet_send_uint64(&this->state, data);
}

/**
 * Sends a string over the network. It sends out
 * the string + '\0'. No size-byte or something.
 * @param data The string to send
 */
void Packet::Send_string(std::string_view data)
{
	openttd_rust_packet_send_string(&this->state, reinterpret_cast<const uint8_t *>(data.data()), data.size());
}

/**
 * Copy a sized byte buffer into the packet.
 * @param data The data to send.
 */
void Packet::Send_buffer(const std::vector<uint8_t> &data)
{
	openttd_rust_packet_send_buffer(&this->state, data.data(), data.size());
}

/**
 * Send as many of the bytes as possible in the packet. This can mean
 * that it is possible that not all bytes are sent. To cope with this
 * the function returns the span of bytes that were not sent.
 * @param span The span describing the range of bytes to send.
 * @return The span of bytes that were not written.
 */
std::span<const uint8_t> Packet::Send_bytes(const std::span<const uint8_t> span)
{
	return span.subspan(openttd_rust_packet_send_bytes(&this->state, span.data(), span.size()));
}

/*
 * Receiving commands
 * Again, the next couple of functions are endian-safe
 *  see the comment before Send_bool for more info.
 */


/**
 * Is it safe to read from the packet, i.e. didn't we run over the buffer?
 * In case \c close_connection is true, the connection will be closed when one would
 * overrun the buffer. When it is false, the connection remains untouched.
 * @param bytes_to_read    The amount of bytes we want to try to read.
 * @param close_connection Whether to close the connection if one cannot read that amount.
 * @return True if that is safe, otherwise false.
 */
bool Packet::CanReadFromPacket(size_t bytes_to_read, bool close_connection)
{
	return openttd_rust_packet_can_read(&this->state, &Services(), bytes_to_read, close_connection);
}

/**
 * Check whether the packet, given the position of the "write" pointer, has read
 * enough of the packet to contain its size.
 * @return True iff there is enough data in the packet to contain the packet's size.
 */
bool Packet::HasPacketSizeData() const
{
	return this->state.position >= EncodedLengthOfPacketSize();
}

/**
 * Get the number of bytes in the packet.
 * When sending a packet this is the size of the data up to that moment.
 * When receiving a packet (before PrepareToRead) this is the allocated size for the data to be read.
 * When reading a packet (after PrepareToRead) this is the full size of the packet.
 * @return The packet's size.
 */
size_t Packet::Size() const
{
	return this->state.size;
}

/**
 * Reads the packet size from the raw packet and stores it in the packet->size
 * @return True iff the packet size seems plausible.
 */
bool Packet::ParsePacketSize()
{
	return openttd_rust_packet_parse_size(&this->state);
}

/**
 * Prepares the packet so it can be read
 * @return True when the packet was valid, otherwise false.
 */
bool Packet::PrepareToRead()
{
	auto *cs = this->state.socket;
	if (cs == nullptr || cs->receive_encryption_handler == nullptr || cs->receive_encryption_handler->GetRustStream() != nullptr) return openttd_rust_packet_prepare_to_read(&this->state, &Services());
	openttd_rust_packet_begin_read(&this->state);
	size_t mac = cs->receive_encryption_handler->MACSize();
	OpenTTDPacketFrame frame;
	if (!openttd_rust_packet_frame(this->Position(), this->Size(), mac, &frame)) return false;
	bool valid = cs->receive_encryption_handler->Decrypt({this->state.data + this->Position(), mac}, {this->state.data + frame.message, frame.payload});
	openttd_rust_packet_finish_read(&this->state, mac);
	return valid;
}

/**
 * Get the \c PacketType from this packet.
 * @return The packet type.
 */
PacketType Packet::GetPacketType() const
{
	auto *cs = this->state.socket;
	assert(this->Size() >= EncodedLengthOfPacketSize() + EncodedLengthOfPacketType());
	if (cs == nullptr || cs->send_encryption_handler == nullptr || cs->send_encryption_handler->GetRustStream() != nullptr) return openttd_rust_packet_get_type(&this->state, &Services());
	size_t mac = cs != nullptr && cs->send_encryption_handler != nullptr ? cs->send_encryption_handler->MACSize() : 0;
	return openttd_rust_packet_packet_type(&this->state, mac);
}

/**
 * Read a boolean from the packet.
 * @return The read data.
 */
bool Packet::Recv_bool()
{
	return openttd_rust_packet_recv_bool(&this->state, &Services());
}

/**
 * Read a 8 bits integer from the packet.
 * @return The read data.
 */
uint8_t Packet::Recv_uint8()
{
	return openttd_rust_packet_recv_uint8(&this->state, &Services());
}

/**
 * Read a 16 bits integer from the packet.
 * @return The read data.
 */
uint16_t Packet::Recv_uint16()
{
	return openttd_rust_packet_recv_uint16(&this->state, &Services());
}

/**
 * Read a 32 bits integer from the packet.
 * @return The read data.
 */
uint32_t Packet::Recv_uint32()
{
	return openttd_rust_packet_recv_uint32(&this->state, &Services());
}

/**
 * Read a 64 bits integer from the packet.
 * @return The read data.
 */
uint64_t Packet::Recv_uint64()
{
	return openttd_rust_packet_recv_uint64(&this->state, &Services());
}

/**
 * Extract a sized byte buffer from the packet.
 * @return The extracted buffer.
 */
std::vector<uint8_t> Packet::Recv_buffer()
{
	std::vector<uint8_t> result(openttd_rust_packet_recv_buffer_size(&this->state, &Services()));
	openttd_rust_packet_recv_buffer_data(&this->state, result.data(), result.size());
	return result;
}

/**
 * Extract at most the length of the span bytes from the packet into the span.
 * @param destination The span to write the bytes to.
 * @return The number of bytes that were actually read.
 */
size_t Packet::Recv_bytes(std::span<uint8_t> destination)
{
	return openttd_rust_packet_recv_bytes(&this->state, destination.data(), destination.size());
}

/**
 * Reads characters (bytes) from the packet until it finds a '\0', or reaches a
 * maximum of \c length characters.
 * When the '\0' has not been reached in the first \c length read characters,
 * more characters are read from the packet until '\0' has been reached. However,
 * these characters will not end up in the returned string.
 * The length of the returned string will be at most \c length - 1 characters.
 * @param length   The maximum length of the string including '\0'.
 * @param settings The string validation settings.
 * @return The validated string.
 */
std::string Packet::Recv_string(size_t length, StringValidationSettings settings)
{
	std::string result;
	openttd_rust_packet_recv_string(&this->state, &Services(), length, settings.base(), &result, [](void *output, const uint8_t *data, size_t size) noexcept {
		static_cast<std::string *>(output)->append(reinterpret_cast<const char *>(data), size);
	});
	return result;
}

/**
 * Get the amount of bytes that are still available for the Transfer functions.
 * @return The number of bytes that still have to be transferred.
 */
size_t Packet::RemainingBytesToTransfer() const
{
	return this->state.size - this->state.position;
}

const OpenTTDPacketServices &Packet::Services()
{
	static const OpenTTDPacketServices services{
		.decrypt = [](NetworkEncryptionHandler *handler, uint8_t *mac, size_t mac_size, uint8_t *message, size_t size) noexcept { return handler->Decrypt({mac, mac_size}, {message, size}); },
		.encrypt = [](NetworkEncryptionHandler *handler, uint8_t *mac, size_t mac_size, uint8_t *message, size_t size) noexcept { handler->Encrypt({mac, mac_size}, {message, size}); },
		.has_quit = [](const NetworkSocketHandler *socket) noexcept { return socket->HasClientQuit(); },
		.mac_size = [](const NetworkEncryptionHandler *handler) noexcept { return handler->MACSize(); },
		.mark_closed = [](NetworkSocketHandler *socket) noexcept { socket->NetworkSocketHandler::MarkClosed(); },
		.receive_handler = [](const NetworkSocketHandler *socket) noexcept { return socket->receive_encryption_handler.get(); },
		.send_handler = [](const NetworkSocketHandler *socket) noexcept { return socket->send_encryption_handler.get(); },
		.stream = [](const NetworkEncryptionHandler *handler) noexcept { return handler->GetRustStream(); },
	};
	return services;
}

bool CheckAbi_packet() { return AbiServicesComplete(Packet::Services()); }

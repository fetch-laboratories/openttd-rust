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
#include "../../string_func.h"

#include "packet.h"

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
	: state{}
{
	openttd_rust_packet_init(&this->state, limit);
	assert(cs != nullptr);

	this->cs = cs;
	openttd_rust_packet_resize(&this->state, initial_read_size);
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
	: state{}, cs(cs)
{
	openttd_rust_packet_init(&this->state, limit);
	/* Allocate space for the the size so we can write that in just before sending the packet. */
	size_t size = EncodedLengthOfPacketSize();
	if (cs != nullptr && cs->send_encryption_handler != nullptr) {
		/* Allocate some space for the message authentication code of the encryption. */
		size_t mac_size = cs->send_encryption_handler->MACSize();
		OpenTTDPacketFrame frame;
		openttd_rust_packet_frame(static_cast<PacketSize>(size), this->Size(), mac_size, &frame);
		size = frame.message;
	}
	assert(this->CanWriteToPacket(size));
	openttd_rust_packet_resize(&this->state, size);

	this->Send_uint8(type);
}


/**
 * Writes the packet size from the raw packet from packet->size
 */
void Packet::PrepareToSend()
{
	/* Header writes precede handler queries and encryption, even when those throw. */
	assert(this->state.data[0] == 0 && this->state.data[1] == 0);
	openttd_rust_packet_write_header(this->state.data, this->Size());
	if (cs != nullptr && cs->send_encryption_handler != nullptr) {
		size_t mac_size = cs->send_encryption_handler->MACSize();
		OpenTTDPacketFrame frame;
		openttd_rust_packet_frame(static_cast<PacketSize>(EncodedLengthOfPacketSize()), this->Size(), mac_size, &frame);
		cs->send_encryption_handler->Encrypt(std::span(&this->state.data[EncodedLengthOfPacketSize()], mac_size), std::span(&this->state.data[frame.message], frame.payload));
	}
	openttd_rust_packet_send_reset(&this->state);
	openttd_rust_packet_shrink(&this->state);
}

/**
 * Is it safe to write to the packet, i.e. didn't we run over the buffer?
 * @param bytes_to_write The amount of bytes we want to try to write.
 * @return True iff the given amount of bytes can be written to the packet.
 */
bool Packet::CanWriteToPacket(size_t bytes_to_write)
{
	return openttd_rust_packet_can_write(&this->state, this->Size(), bytes_to_write) != 0;
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
	this->Send_uint8(openttd_rust_packet_boolean(static_cast<uint8_t>(data)));
}

/**
 * Package a 8 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint8(uint8_t data)
{
	assert(this->CanWriteToPacket(sizeof(data)));
	OpenTTDRustLittleEndian encoded = openttd_rust_encode_uint_le(data);
	/* Keep each original append and its allocation/partial-write order. */
	for (size_t byte = 0; byte < sizeof(data); ++byte) openttd_rust_packet_append(&this->state, &encoded.bytes[byte], 1);
}

/**
 * Package a 16 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint16(uint16_t data)
{
	assert(this->CanWriteToPacket(sizeof(data)));
	OpenTTDRustLittleEndian encoded = openttd_rust_encode_uint_le(data);
	/* Keep each original append and its allocation/partial-write order. */
	for (size_t byte = 0; byte < sizeof(data); ++byte) openttd_rust_packet_append(&this->state, &encoded.bytes[byte], 1);
}

/**
 * Package a 32 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint32(uint32_t data)
{
	assert(this->CanWriteToPacket(sizeof(data)));
	OpenTTDRustLittleEndian encoded = openttd_rust_encode_uint_le(data);
	/* Keep each original append and its allocation/partial-write order. */
	for (size_t byte = 0; byte < sizeof(data); ++byte) openttd_rust_packet_append(&this->state, &encoded.bytes[byte], 1);
}

/**
 * Package a 64 bits integer in the packet.
 * @param data The data to send.
 */
void Packet::Send_uint64(uint64_t data)
{
	assert(this->CanWriteToPacket(sizeof(data)));
	OpenTTDRustLittleEndian encoded = openttd_rust_encode_uint_le(data);
	/* Keep each original append and its allocation/partial-write order. */
	for (size_t byte = 0; byte < sizeof(data); ++byte) openttd_rust_packet_append(&this->state, &encoded.bytes[byte], 1);
}

/**
 * Sends a string over the network. It sends out
 * the string + '\0'. No size-byte or something.
 * @param data The string to send
 */
void Packet::Send_string(std::string_view data)
{
	assert(this->CanWriteToPacket(data.size() + 1));
	openttd_rust_packet_append(&this->state, reinterpret_cast<const uint8_t *>(data.data()), data.size());
	const uint8_t zero = 0;
	openttd_rust_packet_append(&this->state, &zero, 1);
}

/**
 * Copy a sized byte buffer into the packet.
 * @param data The data to send.
 */
void Packet::Send_buffer(const std::vector<uint8_t> &data)
{
	assert(this->CanWriteToPacket(openttd_rust_packet_buffer_size(data.size())));
	this->Send_uint16(openttd_rust_packet_prefix(data.size()));
	openttd_rust_packet_append(&this->state, reinterpret_cast<const uint8_t *>(data.data()), data.size());
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
	size_t amount = openttd_rust_packet_send_amount(&this->state, this->Size(), span.size());
	openttd_rust_packet_append(&this->state, span.data(), amount);
	return span.subspan(amount);
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
	/* Don't allow reading from a quit client/client who send bad data */
	if (this->cs->HasClientQuit()) return false;

	/* Check if variable is within packet-size */
	if (openttd_rust_packet_can_read(&this->state, this->Size(), bytes_to_read) == 0) {
		if (close_connection) this->cs->NetworkSocketHandler::MarkClosed();
		return false;
	}

	return true;
}

/**
 * Check whether the packet, given the position of the "write" pointer, has read
 * enough of the packet to contain its size.
 * @return True iff there is enough data in the packet to contain the packet's size.
 */
bool Packet::HasPacketSizeData() const
{
	return openttd_rust_packet_has_size(&this->state) != 0;
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
	PacketSize size = openttd_rust_packet_parse_size(&this->state, this->state.data, this->Size());
	if (size == 0) return false;
	/* Resize may throw; the cursor commits only after it succeeds. */
	openttd_rust_packet_resize(&this->state, size);
	openttd_rust_packet_read_start(&this->state);
	return true;
}

/**
 * Prepares the packet so it can be read
 * @return True when the packet was valid, otherwise false.
 */
bool Packet::PrepareToRead()
{
	openttd_rust_packet_read_start(&this->state);
	if (cs == nullptr || cs->receive_encryption_handler == nullptr) return true;

	size_t mac_size = cs->receive_encryption_handler->MACSize();
	OpenTTDPacketFrame frame;
	if (openttd_rust_packet_frame(this->Position(), this->Size(), mac_size, &frame) == 0) return false;
	bool valid = cs->receive_encryption_handler->Decrypt(std::span(&this->state.data[this->Position()], mac_size), std::span(&this->state.data[frame.message], frame.payload));
	/* False consumes the MAC; a thrown callback does not. Use its live cursor. */
	openttd_rust_packet_skip_mac(&this->state, mac_size);
	return valid;
}

/**
 * Get the \c PacketType from this packet.
 * @return The packet type.
 */
PacketType Packet::GetPacketType() const
{
	assert(this->Size() >= EncodedLengthOfPacketSize() + EncodedLengthOfPacketType());
	/* The original deliberately queries the send handler here. */
	size_t mac_size = 0;
	if (cs != nullptr && cs->send_encryption_handler != nullptr) mac_size = cs->send_encryption_handler->MACSize();
	OpenTTDPacketFrame frame;
	openttd_rust_packet_frame(static_cast<PacketSize>(EncodedLengthOfPacketSize()), this->Size(), mac_size, &frame);
	return static_cast<PacketType>(this->state.data[frame.message]);
}

/**
 * Read a boolean from the packet.
 * @return The read data.
 */
bool Packet::Recv_bool()
{
	return openttd_rust_packet_boolean(this->Recv_uint8()) != 0;
}

/**
 * Read a 8 bits integer from the packet.
 * @return The read data.
 */
uint8_t Packet::Recv_uint8()
{
	if (!this->CanReadFromPacket(sizeof(uint8_t), true)) return 0;
	return static_cast<uint8_t>(openttd_rust_packet_recv(&this->state, this->state.data, this->Size(), sizeof(uint8_t)));
}

/**
 * Read a 16 bits integer from the packet.
 * @return The read data.
 */
uint16_t Packet::Recv_uint16()
{
	if (!this->CanReadFromPacket(sizeof(uint16_t), true)) return 0;
	return static_cast<uint16_t>(openttd_rust_packet_recv(&this->state, this->state.data, this->Size(), sizeof(uint16_t)));
}

/**
 * Read a 32 bits integer from the packet.
 * @return The read data.
 */
uint32_t Packet::Recv_uint32()
{
	if (!this->CanReadFromPacket(sizeof(uint32_t), true)) return 0;
	return static_cast<uint32_t>(openttd_rust_packet_recv(&this->state, this->state.data, this->Size(), sizeof(uint32_t)));
}

/**
 * Read a 64 bits integer from the packet.
 * @return The read data.
 */
uint64_t Packet::Recv_uint64()
{
	if (!this->CanReadFromPacket(sizeof(uint64_t), true)) return 0;
	return static_cast<uint64_t>(openttd_rust_packet_recv(&this->state, this->state.data, this->Size(), sizeof(uint64_t)));
}

/**
 * Extract a sized byte buffer from the packet.
 * @return The extracted buffer.
 */
std::vector<uint8_t> Packet::Recv_buffer()
{
	uint16_t size = this->Recv_uint16();
	if (size == 0 || !this->CanReadFromPacket(size, true)) return {};

	std::vector<uint8_t> data;
	uint8_t byte;
	/* Consume each byte before its potentially throwing result allocation. */
	while (openttd_rust_packet_buffer_next(&this->state, this->state.data, this->Size(), &size, &byte) != 0) data.push_back(byte);
	return data;
}

/**
 * Extract at most the length of the span bytes from the packet into the span.
 * @param destination The span to write the bytes to.
 * @return The number of bytes that were actually read.
 */
size_t Packet::Recv_bytes(std::span<uint8_t> destination)
{
	auto transfer_to_span = [&destination](std::span<const uint8_t> source) {
		auto to_copy = source.subspan(0, destination.size());
		std::ranges::copy(to_copy, destination.begin());
		return to_copy.size();
	};

	return this->TransferOut(transfer_to_span);
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
	assert(length > 1);

	/* Both loops with Recv_uint8 terminate when reading past the end of the
	 * packet as Recv_uint8 then closes the connection and returns 0. */
	std::string str;
	char character;
	while (--length > 0 && (character = this->Recv_uint8()) != '\0') str.push_back(character);

	if (length == 0) {
		/* The string in the packet was longer. Read until the termination. */
		while (this->Recv_uint8() != '\0') {}
	}

	return StrMakeValid(str, settings);
}

/**
 * Get the amount of bytes that are still available for the Transfer functions.
 * @return The number of bytes that still have to be transferred.
 */
size_t Packet::RemainingBytesToTransfer() const
{
	return openttd_rust_packet_remaining(&this->state, this->Size());
}

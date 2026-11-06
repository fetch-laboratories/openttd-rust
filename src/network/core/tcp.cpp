/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tcp.cpp Basic functions to receive and send TCP packets.
 */

#include "../../stdafx.h"
#include "../../debug.h"

#include "tcp.h"
#include "../../rust/services_assert.h"

#include "../../safeguards.h"

NetworkTCPSocketHandler::~NetworkTCPSocketHandler()
{
	this->CloseSocket();
	openttd_rust_tcp_drop(&GetRustNetworkTcpServices(), this->record);
}

/**
 * Close the actual socket of the connection.
 * Please make sure CloseConnection is called before CloseSocket, as
 * otherwise not all resources might be released.
 */
void NetworkTCPSocketHandler::CloseSocket()
{
	if (this->record->socket != INVALID_SOCKET) closesocket(this->record->socket);
	this->record->socket = INVALID_SOCKET;
}

/**
 * This will put this socket handler in a close state. It will not
 * actually close the OS socket; use CloseSocket for this.
 * @param error Whether we quit under an error condition or not.
 * @return new status of the connection.
 */
NetworkRecvStatus NetworkTCPSocketHandler::CloseConnection([[maybe_unused]] bool error)
{
	openttd_rust_tcp_close(&GetRustNetworkTcpServices(), this->record);

	return NETWORK_RECV_STATUS_OKAY;
}

/**
 * This function puts the packet in the send-queue and it is send as
 * soon as possible. This is the next tick, or maybe one tick later
 * if the OS-network-buffer is full)
 * @param packet the packet to send
 */
void NetworkTCPSocketHandler::SendPacket(std::unique_ptr<Packet> &&packet)
{
	assert(packet != nullptr);

	packet->PrepareToSend();
	openttd_rust_tcp_enqueue(this->record, packet.release());
}

/**
 * Sends all the buffered packets out for this client. It stops when:
 *   1) all packets are send (queue is empty)
 *   2) the OS reports back that it can not send any more
 *      data right now (full network-buffer, it happens ;))
 *   3) sending took too long
 * @param closing_down Whether we are closing down the connection.
 * @return \c true if a (part of a) packet could be sent and
 *         the connection is not closed yet.
 */
SendPacketsState NetworkTCPSocketHandler::SendPackets(bool closing_down)
{
	auto result = openttd_rust_tcp_send(&GetRustNetworkTcpServices(), this->record, closing_down, RustAssertCondition);
	if (result == OpenTTDTcpSend::Close) {
		this->CloseConnection();
		return SPS_CLOSED;
	}
	return static_cast<SendPacketsState>(result);
}

/**
 * Receives a packet for the given client
 * @return The received packet (or nullptr when it didn't receive one)
 */
std::unique_ptr<Packet> NetworkTCPSocketHandler::ReceivePacket()
{
	auto result = openttd_rust_tcp_receive(&GetRustNetworkTcpServices(), this->record, RustAssertCondition);
	if (result == OpenTTDTcpReceive::Close) {
		this->CloseConnection();
		return nullptr;
	}
	if (result == OpenTTDTcpReceive::None) return nullptr;
	if (openttd_rust_tcp_receive_prepared(this->record->receive->PrepareToRead()) == OpenTTDTcpReceive::Close) {
		this->CloseConnection();
		return nullptr;
	}
	return std::unique_ptr<Packet>(std::exchange(this->record->receive, nullptr));
}

/**
 * Check whether this socket can send or receive something.
 * @return \c true when there is something to receive.
 * @note Sets #writable if more data can be sent.
 */
bool NetworkTCPSocketHandler::CanSendReceive()
{
	return openttd_rust_tcp_can_receive(&GetRustNetworkTcpServices(), this->record, RustAssertCondition) != 0;
}

static_assert(sizeof(NetworkError) <= sizeof(OpenTTDTcpErrorStorage));
static_assert(alignof(NetworkError) <= alignof(OpenTTDTcpErrorStorage));
const OpenTTDNetworkTcpServices &GetRustNetworkTcpServices() noexcept
{
	static const OpenTTDNetworkTcpServices services{
		.error_begin = [](OpenTTDTcpErrorStorage *p) noexcept { return new (p) NetworkError(NetworkError::GetLast()); },
		.error_end = [](NetworkError *error) noexcept { error->~NetworkError(); },
		.error_reset = [](const NetworkError *error) noexcept { return error->IsConnectionReset(); },
		.error_text = [](const NetworkError *error) noexcept { auto text = error->AsString(); return OpenTTDByteSpan{reinterpret_cast<const uint8_t *>(text.data()), text.size()}; },
		.error_would_block = [](const NetworkError *error) noexcept { return error->WouldBlock(); },
		.now = []() noexcept -> int64_t { return std::chrono::steady_clock::now().time_since_epoch().count(); },
		.packet = &Packet::Services(),
		.packet_delete_shell = Packet::DeleteShell,
		.packet_new_shell = [](OpenTTDPacketState *state) noexcept { return new Packet(*state); },
		.poll = [](OpenTTDNetworkTcpSocket *s) noexcept -> int32_t {
			fd_set read_fd, write_fd;
			FD_ZERO(&read_fd);
			FD_ZERO(&write_fd);
			FD_SET(s->socket, &read_fd);
			FD_SET(s->socket, &write_fd);
			struct timeval tv{};
			if (select(FD_SETSIZE, &read_fd, &write_fd, nullptr, &tv) < 0) return -1;
			return (FD_ISSET(s->socket, &read_fd) != 0 ? 1 : 0) | (FD_ISSET(s->socket, &write_fd) != 0 ? 2 : 0);
		},
		.receive_bytes = [](OpenTTDNetworkTcpSocket *s, uint8_t *data, size_t size) noexcept -> intptr_t { return SocketReceiver{s->socket}(std::span(data, size)); },
		.send_bytes = [](OpenTTDNetworkTcpSocket *s, const uint8_t *data, size_t size) noexcept -> intptr_t { return SocketSender{s->socket}(std::span(data, size)); },
	};
	return services;
}
#include "../../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDNetworkTcpServices.hpp"
bool CheckAbi_network_tcp() { return AbiServicesComplete(GetRustNetworkTcpServices()); }

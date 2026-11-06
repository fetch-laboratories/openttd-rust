/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_tcp_ffi.h Common TCP transfer leaves and native continuation results. */
#ifndef RUST_NETWORK_TCP_FFI_H
#define RUST_NETWORK_TCP_FFI_H
#include "byte_span_ffi.h"
#include "packet_ffi.h"
#include "../newgrf_badge_type.h"
#include "abi-record-probe.h"
class NetworkTCPSocketHandler;
class NetworkError;
struct Packet;
struct OpenTTDNetworkTcpServices {
	NetworkSocketHandler *(*base_socket)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(base_socket);
	bool (*can_send_receive)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(can_send_receive);
	NetworkError *(*error_begin)(OpenTTDTcpErrorStorage *) noexcept OPENTTD_SERVICE_DEFAULT(error_begin);
	void (*error_end)(NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_end);
	bool (*error_reset)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_reset);
	OpenTTDByteSpan (*error_text)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_text);
	bool (*error_would_block)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_would_block);
	const OpenTTDPacketServices *packet;
	void (*packet_delete_shell)(Packet *) noexcept OPENTTD_SERVICE_DEFAULT(packet_delete_shell);
	OpenTTDPacketState *(*packet_record)(Packet *) noexcept OPENTTD_SERVICE_DEFAULT(packet_record);
	intptr_t (*receive_bytes)(NetworkTCPSocketHandler *, uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(receive_bytes);
	OpenTTDPacketState *(*receive_create)(NetworkTCPSocketHandler *, OpenTTDPacketState *) noexcept OPENTTD_SERVICE_DEFAULT(receive_create);
	OpenTTDPacketState *(*receive_packet)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(receive_packet);
	Packet *(*receive_take)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(receive_take);
	intptr_t (*send_bytes)(NetworkTCPSocketHandler *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(send_bytes);
	OpenTTDPacketState *(*send_front)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(send_front);
	void (*send_pop)(NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(send_pop);
	bool (*valid)(const NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(valid);
	bool (*writable)(const NetworkTCPSocketHandler *) noexcept OPENTTD_SERVICE_DEFAULT(writable);
};
enum class OpenTTDTcpSend : uint8_t { Closed, None, Partly, All, Close };
enum class OpenTTDTcpReceive : uint8_t { None, Close, Ready };
const OpenTTDNetworkTcpServices &GetRustNetworkTcpServices() noexcept;
extern "C" {
OpenTTDTcpSend openttd_rust_tcp_send(const OpenTTDNetworkTcpServices *, NetworkTCPSocketHandler *, bool, void (*)(bool) noexcept);
OpenTTDTcpReceive openttd_rust_tcp_receive(const OpenTTDNetworkTcpServices *, NetworkTCPSocketHandler *, void (*)(bool) noexcept);
}
#endif /* RUST_NETWORK_TCP_FFI_H */

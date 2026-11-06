/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_tcp_ffi.h Common TCP transfer leaves and native continuation results. */
#ifndef RUST_NETWORK_TCP_FFI_H
#define RUST_NETWORK_TCP_FFI_H
#include "byte_span_ffi.h"
#include "packet_ffi.h"
#include "../newgrf_badge_type.h"
#include "abi-record-probe.h"
#include "../network/core/os_abstraction.h"
#include "rust_vector.h"
struct Packet;
#include "abi-record-network_tcp.h"
class NetworkError;
struct OpenTTDNetworkTcpServices {
	NetworkError *(*error_begin)(OpenTTDTcpErrorStorage *) noexcept OPENTTD_SERVICE_DEFAULT(error_begin);
	void (*error_end)(NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_end);
	bool (*error_reset)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_reset);
	OpenTTDByteSpan (*error_text)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_text);
	bool (*error_would_block)(const NetworkError *) noexcept OPENTTD_SERVICE_DEFAULT(error_would_block);
	int64_t (*now)() noexcept OPENTTD_SERVICE_DEFAULT(now);
	const OpenTTDPacketServices *packet;
	void (*packet_delete_shell)(Packet *) noexcept OPENTTD_SERVICE_DEFAULT(packet_delete_shell);
	Packet *(*packet_new_shell)(OpenTTDPacketState *) noexcept OPENTTD_SERVICE_DEFAULT(packet_new_shell);
	int32_t (*poll)(OpenTTDNetworkTcpSocket *) noexcept OPENTTD_SERVICE_DEFAULT(poll);
	intptr_t (*receive_bytes)(OpenTTDNetworkTcpSocket *, uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(receive_bytes);
	intptr_t (*send_bytes)(OpenTTDNetworkTcpSocket *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(send_bytes);
};
enum class OpenTTDTcpSend : uint8_t { Closed, None, Partly, All, Close };
enum class OpenTTDTcpReceive : uint8_t { None, Close, Ready };
const OpenTTDNetworkTcpServices &GetRustNetworkTcpServices() noexcept;
extern "C" {
uint8_t openttd_rust_tcp_can_receive(const OpenTTDNetworkTcpServices *, OpenTTDNetworkTcpSocket *, void (*)(bool) noexcept);
OpenTTDNetworkTcpSocket *openttd_rust_tcp_new(OpenTTDNetworkSocket *, SOCKET);
void openttd_rust_tcp_drop(const OpenTTDNetworkTcpServices *, OpenTTDNetworkTcpSocket *);
void openttd_rust_tcp_close(const OpenTTDNetworkTcpServices *, OpenTTDNetworkTcpSocket *);
void openttd_rust_tcp_enqueue(OpenTTDNetworkTcpSocket *, Packet *);
OpenTTDTcpSend openttd_rust_tcp_send(const OpenTTDNetworkTcpServices *, OpenTTDNetworkTcpSocket *, bool, void (*)(bool) noexcept);
OpenTTDTcpReceive openttd_rust_tcp_receive_prepared(bool);
OpenTTDTcpReceive openttd_rust_tcp_receive(const OpenTTDNetworkTcpServices *, OpenTTDNetworkTcpSocket *, void (*)(bool) noexcept);
}
#endif /* RUST_NETWORK_TCP_FFI_H */

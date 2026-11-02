/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_command_ffi.h Rust network command queues; argument bytes stay encoded by the native command traits. */
#ifndef RUST_NETWORK_COMMAND_FFI_H
#define RUST_NETWORK_COMMAND_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
struct OpenTTDCompanyServices;
struct OpenTTDNetworkSocketSync;
#include "abi-record-probe.h"
/** Native leaves of the command queues; packet pointers are valid only during the call. */
struct OpenTTDNetworkCommandServices {
	const OpenTTDCompanyServices *company;
	void (*admin_log)(const void *, const OpenTTDCommandPacket *) noexcept OPENTTD_SERVICE_DEFAULT(admin_log);
	bool (*allowed_while_paused)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(allowed_while_paused);
	OpenTTDCommandQueue *(*client_queue)() noexcept OPENTTD_SERVICE_DEFAULT(client_queue);
	void (*client_send)(const OpenTTDCommandPacket *) noexcept OPENTTD_SERVICE_DEFAULT(client_send);
	uint16_t (*commands_per_frame)() noexcept OPENTTD_SERVICE_DEFAULT(commands_per_frame);
	uint16_t (*commands_per_frame_server)() noexcept OPENTTD_SERVICE_DEFAULT(commands_per_frame_server);
	void (*execute)(const OpenTTDCommandPacket *) noexcept OPENTTD_SERVICE_DEFAULT(execute);
	void (*execute_past)() noexcept OPENTTD_SERVICE_DEFAULT(execute_past);
	bool (*next_client)(size_t, OpenTTDNetworkCommandClient *) noexcept OPENTTD_SERVICE_DEFAULT(next_client);
	void (*server_send)(void *, const OpenTTDCommandPacket *) noexcept OPENTTD_SERVICE_DEFAULT(server_send);
};
const OpenTTDNetworkCommandServices &GetRustNetworkCommandServices() noexcept;
extern "C" {
void openttd_rust_network_send_command(const OpenTTDNetworkCommandServices *, uint8_t cmd, uint32_t err_msg, const void *callback, uint8_t company, OpenTTDByteSpan data);
void openttd_rust_command_data_grow(OpenTTDCommandData *);
void openttd_rust_command_data_free(OpenTTDCommandData *);
void openttd_rust_command_queue_push(OpenTTDCommandQueue *, OpenTTDCommandPacket *);
void openttd_rust_command_queue_free(OpenTTDCommandQueue *);
}
#endif /* RUST_NETWORK_COMMAND_FFI_H */

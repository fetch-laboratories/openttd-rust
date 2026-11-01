/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_command_ffi.h Rust network command queues; argument bytes stay encoded by the native command traits. */
#ifndef RUST_NETWORK_COMMAND_FFI_H
#define RUST_NETWORK_COMMAND_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
struct OpenTTDCompanyServices;
struct OpenTTDNetworkSocketSync;
/** Raw parts of a Rust byte vector; zeroes when never allocated. Only Rust allocates or frees it. */
struct OpenTTDCommandData {
	uint8_t *data;
	size_t size;
	size_t capacity;
};
/** A queued command; the callback is a `CommandCallback *` that only native leaves compare or call. */
struct OpenTTDCommandPacket {
	OpenTTDCommandData data;
	const void *callback;
	uint32_t frame;
	uint32_t err_msg;
	uint8_t company;
	uint8_t cmd;
	bool my_cmd;
};
/** Raw parts of a Rust packet vector; zeroes when never allocated. Only Rust allocates or frees it. */
struct OpenTTDCommandQueue {
	OpenTTDCommandPacket *data;
	size_t size;
	size_t capacity;
};
/** One client socket and its in-place queues, valid until the socket is deleted. */
struct OpenTTDNetworkCommandClient {
	void *socket;
	void *game; ///< The same socket as a NetworkGameSocketHandler.
	size_t index;
	OpenTTDCommandQueue *incoming;
	OpenTTDCommandQueue *outgoing;
	OpenTTDNetworkSocketSync *sync;
	int64_t last_packet; ///< Steady-clock nanoseconds when visited.
	uint8_t status;
};
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
void openttd_rust_network_sync_command_queue(OpenTTDCommandQueue *outgoing);
void openttd_rust_network_free_local_command_queue();
void openttd_rust_network_handle_command_queue(const OpenTTDNetworkCommandServices *, void *socket, OpenTTDCommandQueue *outgoing);
void openttd_rust_command_data_grow(OpenTTDCommandData *);
void openttd_rust_command_data_free(OpenTTDCommandData *);
void openttd_rust_command_queue_push(OpenTTDCommandQueue *, OpenTTDCommandPacket *);
void openttd_rust_command_queue_free(OpenTTDCommandQueue *);
}
#endif /* RUST_NETWORK_COMMAND_FFI_H */

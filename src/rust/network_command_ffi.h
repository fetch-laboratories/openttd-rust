/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_command_ffi.h Rust network command queues; argument bytes stay encoded by the native command traits. */
#ifndef RUST_NETWORK_COMMAND_FFI_H
#define RUST_NETWORK_COMMAND_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
struct OpenTTDCompanyServices;
struct OpenTTDCommandServices;
struct OpenTTDNetworkFrameServices;
enum class OpenTTDNetworkCommandDecodeError : uint8_t { None, InvalidCommand, Offline, InvalidCallback };
struct OpenTTDNetworkSocketSync;
#include "abi-record-probe.h"
/** Native leaves of the command queues; packet pointers are valid only during the call. */
struct OpenTTDNetworkCommandServices {
	void (*admin_log)(const void *, const OpenTTDCommandPacket *) noexcept OPENTTD_SERVICE_DEFAULT(admin_log);
	void (*allow_list_key)(OpenTTDByteSpan, void *) noexcept OPENTTD_SERVICE_DEFAULT(allow_list_key);
	const void *(*callback)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	size_t callback_count;
	uint16_t (*commands_per_frame)() noexcept OPENTTD_SERVICE_DEFAULT(commands_per_frame);
	uint16_t (*commands_per_frame_server)() noexcept OPENTTD_SERVICE_DEFAULT(commands_per_frame_server);
	const OpenTTDCompanyServices *company;
	void (*console_bad_client)(void *, OpenTTDNetworkCommandDecodeError) noexcept OPENTTD_SERVICE_DEFAULT(console_bad_client);
	void (*console_bad_server)(OpenTTDNetworkCommandDecodeError) noexcept OPENTTD_SERVICE_DEFAULT(console_bad_server);
	void (*console_company_mismatch)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(console_company_mismatch);
	void (*console_server_only)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(console_server_only);
	void (*console_spectator)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(console_spectator);
	uint8_t (*control_action)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(control_action);
	void (*debug_receive_client)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_client);
	void (*debug_receive_server)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_server);
	void (*debug_send_client)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_client);
	void (*debug_send_server)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_server);
	void (*debug_unknown_callback)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_unknown_callback);
	void (*execute)(const OpenTTDCommandPacket *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(execute);
	void (*execute_past)() noexcept OPENTTD_SERVICE_DEFAULT(execute_past);
	bool (*has_unpack)(uint8_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(has_unpack);
	bool (*info_key_equals)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(info_key_equals);
	uint16_t (*max_commands_in_queue)() noexcept OPENTTD_SERVICE_DEFAULT(max_commands_in_queue);
	const OpenTTDNetworkFrameServices &(*network)() noexcept OPENTTD_SERVICE_DEFAULT(network);
	bool (*next_client)(size_t, OpenTTDNetworkCommandClient *) noexcept OPENTTD_SERVICE_DEFAULT(next_client);
	void *(*next_info)(uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(next_info);
	OpenTTDCommandData (*replace_client_id)(uint8_t, OpenTTDByteSpan, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(replace_client_id);
	OpenTTDCommandData (*sanitize_strings)(uint8_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(sanitize_strings);
	void (*server_full)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(server_full);

};
const OpenTTDNetworkCommandServices &GetRustNetworkCommandServices() noexcept;
extern "C" {
uint8_t openttd_rust_network_registered_callback(const OpenTTDNetworkCommandServices *, const void *);
void openttd_rust_network_send_command(const OpenTTDNetworkCommandServices *, uint8_t cmd, uint32_t err_msg, const void *callback, uint8_t company, OpenTTDByteSpan data);
void openttd_rust_command_data_grow(OpenTTDCommandData *);
void openttd_rust_command_data_free(OpenTTDCommandData *);
void openttd_rust_command_queue_free(OpenTTDCommandQueue *);
}
#endif /* RUST_NETWORK_COMMAND_FFI_H */

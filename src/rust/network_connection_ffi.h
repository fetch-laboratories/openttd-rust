/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection_ffi.h Admission and map protocol native services. */
#ifndef RUST_NETWORK_CONNECTION_FFI_H
#define RUST_NETWORK_CONNECTION_FFI_H
#include "network_frame_ffi.h"
#include "auth_ffi.h"
class NetworkAuthenticationHandler;
struct OpenTTDGrfLoadingServices;
struct Packet;
struct PacketWriter;
struct OpenTTDNetworkConnectionServices {
	void (*admin_error)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(admin_error);
	void (*admin_joined)(void *) noexcept OPENTTD_SERVICE_DEFAULT(admin_joined);
	void (*admin_quit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(admin_quit);
	void (*allocate_info)(void *, void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(allocate_info);
	const NetworkAuthenticationHandler *(*auth)(void *) noexcept OPENTTD_SERVICE_DEFAULT(auth);
	void (*auth_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(auth_create);
	uint32_t (*client_id)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(client_id);
	OpenTTDByteSpan (*client_name)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(client_name);
	uint8_t (*client_playas)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(client_playas);
	uint8_t *clients_connected;
	uint8_t *clients_on;
	bool (*company_allows_client)(uint8_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(company_allows_client);
	void (*enable_encryption)(void *, const uint8_t *, OpenTTDAuthStream *, OpenTTDAuthStream *) noexcept OPENTTD_SERVICE_DEFAULT(enable_encryption);
	OpenTTDByteSpan (*get_client_ip)(void *) noexcept OPENTTD_SERVICE_DEFAULT(get_client_ip);
	const OpenTTDGrfLoadingServices *grf;
	void * (*info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(info);
	int32_t (*join_date)(void *) noexcept OPENTTD_SERVICE_DEFAULT(join_date);
	PacketWriter *(*map_begin)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_begin);
	void (*map_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_create);
	void (*map_destroy)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_destroy);
	void (*map_end)(PacketWriter *) noexcept OPENTTD_SERVICE_DEFAULT(map_end);
	void (*map_enqueue)(PacketWriter *) noexcept OPENTTD_SERVICE_DEFAULT(map_enqueue);
	Packet *(*map_front)(PacketWriter *) noexcept OPENTTD_SERVICE_DEFAULT(map_front);
	uint8_t (*max_clients)() noexcept OPENTTD_SERVICE_DEFAULT(max_clients);
	uint8_t (*max_companies)() noexcept OPENTTD_SERVICE_DEFAULT(max_companies);
	uint8_t (*min_active_clients)() noexcept OPENTTD_SERVICE_DEFAULT(min_active_clients);
	uint32_t (*newgrf_version)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf_version);
	bool (*pause_on_join)() noexcept OPENTTD_SERVICE_DEFAULT(pause_on_join);
	bool (*revision_compatible)(void *) noexcept OPENTTD_SERVICE_DEFAULT(revision_compatible);
	uint8_t (*send_client_info)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(send_client_info);
	uint8_t (*send_config)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_config);
	uint8_t (*send_game_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_game_info);
	bool (*unique_name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(unique_name);
	bool (*valid_name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(valid_name);
};
const OpenTTDNetworkConnectionServices &GetRustNetworkConnectionServices() noexcept;
extern "C" {
uint32_t openttd_rust_network_next_client_id();
uint8_t openttd_rust_network_send_error(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *socket, uint8_t error, OpenTTDByteSpan reason, OpenTTDNetworkActionsStorage *);
uint8_t openttd_rust_network_server_close(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *socket, uint8_t result, OpenTTDNetworkActionsStorage *);
}
#endif

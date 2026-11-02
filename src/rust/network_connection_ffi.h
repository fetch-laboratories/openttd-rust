/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection_ffi.h Admission and map protocol native services. */
#ifndef RUST_NETWORK_CONNECTION_FFI_H
#define RUST_NETWORK_CONNECTION_FFI_H
#include "network_frame_ffi.h"
struct OpenTTDNetworkConnectionServices {
	void (*admin_error)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(admin_error);
	void (*admin_joined)(void *) noexcept OPENTTD_SERVICE_DEFAULT(admin_joined);
	void (*admin_quit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(admin_quit);
	void (*allocate_info)(void *, void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(allocate_info);
	uint8_t (*auth_request)(void *) noexcept OPENTTD_SERVICE_DEFAULT(auth_request);
	uint8_t (*client_company)(void *) noexcept OPENTTD_SERVICE_DEFAULT(client_company);
	void (*client_error_announcement)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(client_error_announcement);
	uint32_t (*client_id)(void *) noexcept OPENTTD_SERVICE_DEFAULT(client_id);
	uint8_t *clients_connected;
	uint8_t *clients_on;
	bool (*company_allows_client)(uint8_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(company_allows_client);
	void (*debug_authorized)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_authorized);
	void (*debug_closed)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_closed);
	void (*debug_done_map)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_done_map);
	void (*debug_game_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_game_info);
	void (*debug_getmap)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_getmap);
	void (*debug_identify)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_identify);
	void (*debug_join)(void *, void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_join);
	void (*debug_map)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_map);
	void (*debug_map_first)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_map_first);
	void (*debug_map_last)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_map_last);
	void (*debug_map_ok)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_map_ok);
	void (*debug_map_wait)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_map_wait);
	void (*debug_newgrf_check)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_newgrf_check);
	void (*debug_newgrfs)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_newgrfs);
	void (*debug_newgrfs_checked)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_newgrfs_checked);
	void (*debug_next_map)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_next_map);
	void (*debug_pre_active)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_pre_active);
	void (*debug_receive_error)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_error);
	void (*debug_receive_quit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_quit);
	void (*debug_send_error)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_error);
	void (*debug_send_error_quit)(void *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_error_quit);
	void (*debug_send_join)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_join);
	void (*debug_send_newgame)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_newgame);
	void (*debug_send_quit)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_quit);
	void (*debug_send_shutdown)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_shutdown);
	void (*debug_send_wait)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_wait);
	void (*debug_send_welcome)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_welcome);
	void (*error_announcement)(void *, uint32_t, OpenTTDByteSpan, bool) noexcept OPENTTD_SERVICE_DEFAULT(error_announcement);
	uint32_t (*error_message)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(error_message);
	void (*error_unjoined)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(error_unjoined);
	void * (*game)(void *) noexcept OPENTTD_SERVICE_DEFAULT(game);
	bool (*grfs_empty)() noexcept OPENTTD_SERVICE_DEFAULT(grfs_empty);
	void * (*grfs_packet)(void *) noexcept OPENTTD_SERVICE_DEFAULT(grfs_packet);
	void * (*info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(info);
	void * (*info_by_id)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(info_by_id);
	int32_t (*join_date)(void *) noexcept OPENTTD_SERVICE_DEFAULT(join_date);
	void (*joined)(void *) noexcept OPENTTD_SERVICE_DEFAULT(joined);
	void (*lost_connection_announcement)(void *) noexcept OPENTTD_SERVICE_DEFAULT(lost_connection_announcement);
	void (*map_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_create);
	void (*map_destroy)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_destroy);
	void (*map_save)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_save);
	bool (*map_transfer)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_transfer);
	uint8_t (*max_clients)() noexcept OPENTTD_SERVICE_DEFAULT(max_clients);
	uint8_t (*max_companies)() noexcept OPENTTD_SERVICE_DEFAULT(max_companies);
	uint8_t (*min_active_clients)() noexcept OPENTTD_SERVICE_DEFAULT(min_active_clients);
	bool *network_server;
	bool *networking;
	uint32_t (*newgrf_version)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf_version);
	OpenTTDCommandQueue * (*outgoing_queue)(void *) noexcept OPENTTD_SERVICE_DEFAULT(outgoing_queue);
	bool (*pause_on_join)() noexcept OPENTTD_SERVICE_DEFAULT(pause_on_join);
	void (*quit_announcement)(void *) noexcept OPENTTD_SERVICE_DEFAULT(quit_announcement);
	bool (*revision_compatible)(void *) noexcept OPENTTD_SERVICE_DEFAULT(revision_compatible);
	uint8_t (*send_client_info)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(send_client_info);
	uint8_t (*send_config)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_config);
	uint8_t (*send_game_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_game_info);
	uint8_t * (*status)(void *) noexcept OPENTTD_SERVICE_DEFAULT(status);
	OpenTTDNetworkSocketSync * (*sync)(void *) noexcept OPENTTD_SERVICE_DEFAULT(sync);
	bool (*unique_name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(unique_name);
	bool (*valid_name)(void *) noexcept OPENTTD_SERVICE_DEFAULT(valid_name);
};
const OpenTTDNetworkConnectionServices &GetRustNetworkConnectionServices() noexcept;
extern "C" {
uint32_t openttd_rust_network_next_client_id();
uint8_t openttd_rust_network_send_map(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_receive_newgrfs_checked(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_receive_getmap(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_receive_map_ok(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_receive_game_info(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_receive_join(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket, void *packet, void *text);
uint8_t openttd_rust_network_receive_identify(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket, void *packet, void *text);
uint8_t openttd_rust_network_send_error(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket, uint8_t error, OpenTTDByteSpan reason);
uint8_t openttd_rust_network_receive_error(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket, void *packet);
uint8_t openttd_rust_network_receive_quit(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket);
uint8_t openttd_rust_network_server_close(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket, uint8_t result);
}
#endif

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_connection_ffi.h Admission and map protocol native services. */
#ifndef RUST_NETWORK_CONNECTION_FFI_H
#define RUST_NETWORK_CONNECTION_FFI_H
#include "network_frame_ffi.h"
struct OpenTTDNetworkConnectionServices {
	uint8_t *clients_on;
	void (*admin_joined)(void *) noexcept OPENTTD_SERVICE_DEFAULT(admin_joined);
	uint8_t (*client_company)(void *) noexcept OPENTTD_SERVICE_DEFAULT(client_company);
	uint32_t (*client_id)(void *) noexcept OPENTTD_SERVICE_DEFAULT(client_id);
	uint8_t (*close)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(close);
	void (*debug_authorized)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_authorized);
	void (*debug_done_map)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_done_map);
	void (*debug_game_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_game_info);
	void (*debug_getmap)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_getmap);
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
	void (*debug_send_join)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_join);
	void (*debug_send_wait)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_wait);
	void (*debug_send_welcome)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_welcome);
	void * (*game)(void *) noexcept OPENTTD_SERVICE_DEFAULT(game);
	bool (*grfs_empty)() noexcept OPENTTD_SERVICE_DEFAULT(grfs_empty);
	void * (*grfs_packet)(void *) noexcept OPENTTD_SERVICE_DEFAULT(grfs_packet);
	bool (*has_quit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(has_quit);
	void * (*info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(info);
	int32_t (*join_date)(void *) noexcept OPENTTD_SERVICE_DEFAULT(join_date);
	void (*joined)(void *) noexcept OPENTTD_SERVICE_DEFAULT(joined);
	void (*map_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_create);
	void (*map_destroy)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_destroy);
	void (*map_save)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_save);
	bool (*map_transfer)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_transfer);
	uint8_t (*min_active_clients)() noexcept OPENTTD_SERVICE_DEFAULT(min_active_clients);
	OpenTTDCommandQueue * (*outgoing_queue)(void *) noexcept OPENTTD_SERVICE_DEFAULT(outgoing_queue);
	bool (*pause_on_join)() noexcept OPENTTD_SERVICE_DEFAULT(pause_on_join);
	uint8_t (*send_client_info)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(send_client_info);
	uint8_t (*send_config)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_config);
	uint8_t (*send_game_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(send_game_info);
	void * (*server_info)() noexcept OPENTTD_SERVICE_DEFAULT(server_info);
	uint8_t * (*status)(void *) noexcept OPENTTD_SERVICE_DEFAULT(status);
	OpenTTDNetworkSocketSync * (*sync)(void *) noexcept OPENTTD_SERVICE_DEFAULT(sync);
};
const OpenTTDNetworkConnectionServices &GetRustNetworkConnectionServices() noexcept;
extern "C" {
uint8_t openttd_rust_network_send_welcome(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_send_wait(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_send_map(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_send_newgrf_check(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_receive_newgrfs_checked(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_receive_getmap(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_receive_map_ok(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
uint8_t openttd_rust_network_receive_game_info(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void * socket);
void openttd_rust_network_next_map(const OpenTTDNetworkConnectionServices *, const OpenTTDNetworkFrameServices *, void *socket, void *ignore);
}
#endif

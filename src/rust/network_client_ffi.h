/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_client_ffi.h Client admission, map and connection native services. */
#ifndef RUST_NETWORK_CLIENT_FFI_H
#define RUST_NETWORK_CLIENT_FFI_H
#include "network_frame_ffi.h"
#include "auth_ffi.h"
class NetworkAuthenticationHandler;
struct OpenTTDSessionServices;
struct OpenTTDSessionLoad;
struct OpenTTDNetworkClientServices {
	const NetworkAuthenticationHandler *(*auth)(void *) noexcept OPENTTD_SERVICE_DEFAULT(auth);
	void (*auth_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(auth_create);
	bool (*check_grf)(uint32_t, const uint8_t *) noexcept OPENTTD_SERVICE_DEFAULT(check_grf);
	OpenTTDByteSpan (*client_name)() noexcept OPENTTD_SERVICE_DEFAULT(client_name);
	void (*delete_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(delete_info);
	void (*dirty_join)() noexcept OPENTTD_SERVICE_DEFAULT(dirty_join);
	void (*emergency_save)() noexcept OPENTTD_SERVICE_DEFAULT(emergency_save);
	void (*enable_encryption)(void *, OpenTTDAuthStream *, OpenTTDAuthStream *) noexcept OPENTTD_SERVICE_DEFAULT(enable_encryption);
	void (*enter_multiplayer)() noexcept OPENTTD_SERVICE_DEFAULT(enter_multiplayer);
	void (*join)(void *) noexcept OPENTTD_SERVICE_DEFAULT(join);
	uint32_t *join_bytes;
	uint32_t *join_bytes_total;
	const uint8_t *join_company;
	uint8_t *join_status;
	uint8_t *join_waiting;
	void (*leave)(void *) noexcept OPENTTD_SERVICE_DEFAULT(leave);
	void (*leave_error)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(leave_error);
	uint32_t (*map_append_data)(void *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(map_append_data);
	void (*map_create)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_create);
	bool (*map_exists)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_exists);
	void (*map_load_prepare)(void *, OpenTTDSessionLoad *) noexcept OPENTTD_SERVICE_DEFAULT(map_load_prepare);
	void (*map_release)(void *) noexcept OPENTTD_SERVICE_DEFAULT(map_release);
	uint32_t *own_client_id;
	uint8_t *reconnect;
	OpenTTDByteSpan (*revision)() noexcept OPENTTD_SERVICE_DEFAULT(revision);
	void (*show_banned)() noexcept OPENTTD_SERVICE_DEFAULT(show_banned);
	void (*show_client_list)() noexcept OPENTTD_SERVICE_DEFAULT(show_client_list);
	void (*show_full)() noexcept OPENTTD_SERVICE_DEFAULT(show_full);
	void (*show_join_window)() noexcept OPENTTD_SERVICE_DEFAULT(show_join_window);
	void (*show_lag)(int64_t) noexcept OPENTTD_SERVICE_DEFAULT(show_lag);
	void (*show_newgrf_mismatch)() noexcept OPENTTD_SERVICE_DEFAULT(show_newgrf_mismatch);
	void (*show_reboot)() noexcept OPENTTD_SERVICE_DEFAULT(show_reboot);
	void (*show_savegame_error)() noexcept OPENTTD_SERVICE_DEFAULT(show_savegame_error);
	void (*show_server_error)(uint8_t, void *) noexcept OPENTTD_SERVICE_DEFAULT(show_server_error);
	void (*show_shutdown)() noexcept OPENTTD_SERVICE_DEFAULT(show_shutdown);
	void (*wait_for_quit_packets)() noexcept OPENTTD_SERVICE_DEFAULT(wait_for_quit_packets);
};
const OpenTTDNetworkClientServices &GetRustNetworkClientServices() noexcept;
extern "C" {
void openttd_rust_network_client_connected(const OpenTTDNetworkClientServices *, const OpenTTDNetworkFrameServices *);
uint8_t openttd_rust_network_client_close(const OpenTTDNetworkClientServices *, const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *game, uint8_t result);
uint8_t openttd_rust_network_client_is_connected_abi(const OpenTTDNetworkClientServices *, const OpenTTDNetworkFrameServices *);
extern "C++" inline bool openttd_rust_network_client_is_connected(const OpenTTDNetworkClientServices *a0, const OpenTTDNetworkFrameServices *a1) { return openttd_rust_network_client_is_connected_abi(a0, a1) != 0; }
}
#endif /* RUST_NETWORK_CLIENT_FFI_H */

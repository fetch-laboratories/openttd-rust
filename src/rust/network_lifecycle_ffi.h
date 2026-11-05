/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_lifecycle_ffi.h Session connection and server lifecycle native services. */
#ifndef RUST_NETWORK_LIFECYCLE_FFI_H
#define RUST_NETWORK_LIFECYCLE_FFI_H
#include "network_frame_ffi.h"
struct OpenTTDSessionServices;
struct Company;
struct OpenTTDNetworkLifecycleServices {
	bool (*admin_auth_configured)() noexcept OPENTTD_SERVICE_DEFAULT(admin_auth_configured);
	uint16_t (*admin_port)() noexcept OPENTTD_SERVICE_DEFAULT(admin_port);
	void (*admins_close)() noexcept OPENTTD_SERVICE_DEFAULT(admins_close);
	void (*admins_newgame)() noexcept OPENTTD_SERVICE_DEFAULT(admins_newgame);
	void (*admins_shutdown)() noexcept OPENTTD_SERVICE_DEFAULT(admins_shutdown);
	void (*admins_welcome)() noexcept OPENTTD_SERVICE_DEFAULT(admins_welcome);
	void * (*allocate_server_info)() noexcept OPENTTD_SERVICE_DEFAULT(allocate_server_info);
	void (*announce_company)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(announce_company);
	void (*auth_client_key)() noexcept OPENTTD_SERVICE_DEFAULT(auth_client_key);
	void (*check_names)() noexcept OPENTTD_SERVICE_DEFAULT(check_names);
	void (*close_admin_listeners)() noexcept OPENTTD_SERVICE_DEFAULT(close_admin_listeners);
	void (*close_client_listeners)() noexcept OPENTTD_SERVICE_DEFAULT(close_client_listeners);
	void * (*connect_text_begin)(OpenTTDSessionFrame *, bool) noexcept OPENTTD_SERVICE_DEFAULT(connect_text_begin);
	void (*connect_text_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(connect_text_end);
	void (*coordinator_close_client)() noexcept OPENTTD_SERVICE_DEFAULT(coordinator_close_client);
	void (*coordinator_close_server)() noexcept OPENTTD_SERVICE_DEFAULT(coordinator_close_server);
	void (*coordinator_register)() noexcept OPENTTD_SERVICE_DEFAULT(coordinator_register);
	void (*create_client_connecter)() noexcept OPENTTD_SERVICE_DEFAULT(create_client_connecter);
	void (*fill_game_info)() noexcept OPENTTD_SERVICE_DEFAULT(fill_game_info);
	void (*initialize_pools)(bool) noexcept OPENTTD_SERVICE_DEFAULT(initialize_pools);
	OpenTTDByteSpan (*join_connection)() noexcept OPENTTD_SERVICE_DEFAULT(join_connection);
	void (*kill_connecters)() noexcept OPENTTD_SERVICE_DEFAULT(kill_connecters);
	OpenTTDByteSpan (*last_joined)() noexcept OPENTTD_SERVICE_DEFAULT(last_joined);
	bool (*listen_admins)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(listen_admins);
	bool (*listen_clients)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(listen_clients);
	bool (*on_dedicated)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(on_dedicated);
	bool (*on_server)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(on_server);
	OpenTTDCommandData (*pack_company_allow)(void *) noexcept OPENTTD_SERVICE_DEFAULT(pack_company_allow);
	OpenTTDCommandData (*pack_president)(void *) noexcept OPENTTD_SERVICE_DEFAULT(pack_president);
	void (*parse_connection)(OpenTTDByteSpan, uint8_t *, void *) noexcept OPENTTD_SERVICE_DEFAULT(parse_connection);
	bool (*pre_dedicated)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(pre_dedicated);
	bool (*pre_server)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(pre_server);
	void (*remember_joined_server)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(remember_joined_server);
	void (*reset_restart_time)() noexcept OPENTTD_SERVICE_DEFAULT(reset_restart_time);
	uint8_t (*server_game_type)() noexcept OPENTTD_SERVICE_DEFAULT(server_game_type);
	uint16_t (*server_port)() noexcept OPENTTD_SERVICE_DEFAULT(server_port);
	void (*set_info_company)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_info_company);
	void (*set_join_info)(void *, uint8_t, const void *) noexcept OPENTTD_SERVICE_DEFAULT(set_join_info);
	void (*set_network_server)(bool) noexcept OPENTTD_SERVICE_DEFAULT(set_network_server);
	void (*set_networking)(bool) noexcept OPENTTD_SERVICE_DEFAULT(set_networking);
	void (*set_server_info_key)(void *) noexcept OPENTTD_SERVICE_DEFAULT(set_server_info_key);
	void (*udp_initialize)() noexcept OPENTTD_SERVICE_DEFAULT(udp_initialize);
	void (*udp_listen)() noexcept OPENTTD_SERVICE_DEFAULT(udp_listen);
	void (*update_client_info)(void *) noexcept OPENTTD_SERVICE_DEFAULT(update_client_info);
	void (*update_game_info)() noexcept OPENTTD_SERVICE_DEFAULT(update_game_info);
	bool (*validate_client_name)() noexcept OPENTTD_SERVICE_DEFAULT(validate_client_name);
};
const OpenTTDNetworkLifecycleServices &GetRustNetworkLifecycleServices() noexcept;
extern "C" {
void openttd_rust_network_initialize(const OpenTTDNetworkLifecycleServices *, const OpenTTDNetworkFrameServices *, bool close_admins);
void openttd_rust_network_disconnect(const OpenTTDNetworkLifecycleServices *, const OpenTTDNetworkFrameServices *, bool close_admins, OpenTTDNetworkActionsStorage *);
uint8_t openttd_rust_network_connect_game(const OpenTTDNetworkLifecycleServices *, const OpenTTDNetworkFrameServices *, const OpenTTDSessionServices * session, OpenTTDByteSpan connection, uint8_t company, const void * password, void * resolved, OpenTTDNetworkActionsStorage *);
}
#endif /* RUST_NETWORK_LIFECYCLE_FFI_H */

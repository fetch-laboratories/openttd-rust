/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_dispatch_ffi.h Native retained game handlers and packet frame resources. */
#ifndef RUST_NETWORK_DISPATCH_FFI_H
#define RUST_NETWORK_DISPATCH_FFI_H
#include "network_frame_ffi.h"
struct OpenTTDSessionServices;
struct OpenTTDSessionLoad;
struct Packet;
struct OpenTTDNetworkDispatchServices {
	uint8_t (*client_chat)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(client_chat);
	uint8_t (*client_move)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(client_move);
	uint8_t (*client_rcon)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(client_rcon);
	uint8_t (*client_set_name)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(client_set_name);
	OpenTTDSessionLoad *(*map_load_args)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(map_load_args);
	void (*packet_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(packet_end);
	uint8_t (*server_banned)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_banned);
	uint8_t (*server_chat)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_chat);
	uint8_t (*server_client_info)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_client_info);
	uint8_t (*server_config_update)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_config_update);
	uint8_t (*server_error)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_error);
	uint8_t (*server_external_chat)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_external_chat);
	uint8_t (*server_full)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_full);
	uint8_t (*server_game_info)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_game_info);
	uint8_t (*server_move)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_move);
	uint8_t (*server_rcon)(OpenTTDNetworkGameSocket *, Packet *, OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_rcon);
	void *(*text_begin)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(text_begin);
};
const OpenTTDNetworkDispatchServices &GetRustNetworkDispatchServices() noexcept;
extern "C" {
void openttd_rust_network_receive_packets(const OpenTTDSessionServices *, const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *, OpenTTDSessionFrame *, OpenTTDNetworkActionsStorage *);
}
#endif

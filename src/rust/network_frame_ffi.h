/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_frame_ffi.h Rust network frame, sync, ack and lag decisions; packets and sockets stay native. */
#ifndef RUST_NETWORK_FRAME_FFI_H
#define RUST_NETWORK_FRAME_FFI_H
#include "service_defaults.hpp"
#include "packet_ffi.h"
#include "network_command_ffi.h"
struct OpenTTDSessionFrame;
struct OpenTTDAuthHandler;
class NetworkAuthenticationHandler;
struct OpenTTDNetworkTcpServices;
struct OpenTTDNetworkDispatchServices;
struct OpenTTDNetworkConnectionServices;
struct OpenTTDNetworkClientServices;
struct OpenTTDNetworkLifecycleServices;
#include "../newgrf_badge_type.h"
#include "abi-record-probe.h"
enum class OpenTTDNetworkPoll : uint8_t { Error, Ready, Exception };
/** Native leaves of the frame protocol: `game` is a NetworkGameSocketHandler, `socket` a NetworkClientSocket, packets are owned until sent. */
struct OpenTTDNetworkFrameServices {
	bool (*admins_receive)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(admins_receive);
	bool (*admins_send)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(admins_send);
	OpenTTDAuthHandler *(*auth_owner)(const NetworkAuthenticationHandler *) noexcept OPENTTD_SERVICE_DEFAULT(auth_owner);
	uint16_t (*bytes_per_frame)() noexcept OPENTTD_SERVICE_DEFAULT(bytes_per_frame);
	uint16_t (*bytes_per_frame_burst)() noexcept OPENTTD_SERVICE_DEFAULT(bytes_per_frame_burst);
	const OpenTTDNetworkClientServices *client;
	void (*client_desync)() noexcept OPENTTD_SERVICE_DEFAULT(client_desync);
	OpenTTDNetworkGameSocket *(*client_game)() noexcept OPENTTD_SERVICE_DEFAULT(client_game);
	const OpenTTDNetworkCommandServices *command;
	const OpenTTDNetworkConnectionServices *connection;
	void (*console_bad_acks)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_bad_acks);
	void (*console_download_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_download_timeout);
	void (*console_init_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_init_timeout);
	void (*console_join_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_join_timeout);
	void (*console_lagging)(void *, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(console_lagging);
	void (*console_password_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_password_timeout);
	void (*console_slow)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(console_slow);
	const OpenTTDNetworkDispatchServices *dispatch;
	uint8_t (*frame_freq)() noexcept OPENTTD_SERVICE_DEFAULT(frame_freq);
	uint32_t (*interactive_random_range)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(interactive_random_range);
	const OpenTTDNetworkLifecycleServices *lifecycle;
	uint16_t (*max_download_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_download_time);
	uint16_t (*max_init_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_init_time);
	uint16_t (*max_join_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_join_time);
	uint16_t (*max_lag_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_lag_time);
	uint16_t (*max_password_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_password_time);
	bool (*on_server_connect)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(on_server_connect);
	const OpenTTDPacketServices *packet;
	uint8_t (*query_close)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(query_close);
	const uint32_t *random;
	void (*replay_commands)() noexcept OPENTTD_SERVICE_DEFAULT(replay_commands);
	bool (*server_accept)(OpenTTDSessionFrame *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(server_accept);
	OpenTTDNetworkPoll (*server_accept_next)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_accept_next);
	OpenTTDNetworkPoll (*server_poll)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_poll);
	bool (*server_poll_client)(OpenTTDSessionFrame *, OpenTTDNetworkGameSocket *) noexcept OPENTTD_SERVICE_DEFAULT(server_poll_client);
	void (*server_poll_end)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(server_poll_end);
	void (*show_lost_connection)() noexcept OPENTTD_SERVICE_DEFAULT(show_lost_connection);
	void (*socket_delete)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_delete);
	uint16_t (*sync_freq)() noexcept OPENTTD_SERVICE_DEFAULT(sync_freq);
	void (*sync_log)() noexcept OPENTTD_SERVICE_DEFAULT(sync_log);
	const OpenTTDNetworkTcpServices *tcp;
	void (*text_append)(void *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(text_append);
	void (*text_reset)(void *) noexcept OPENTTD_SERVICE_DEFAULT(text_reset);
};
const OpenTTDNetworkFrameServices &GetRustNetworkFrameServices() noexcept;
void RustNetworkCompleteActions(OpenTTDSessionFrame *);
extern "C" {
uint8_t openttd_rust_network_game_close(const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *, OpenTTDNetworkActionsStorage *);
void openttd_rust_network_process_deferred(const OpenTTDNetworkFrameServices *);
void RustDeleteNetworkSocket(void *) noexcept;
void openttd_rust_network_deferred_shutdown(void (*)(void *) noexcept);
void openttd_rust_network_actions_init(OpenTTDNetworkActionsStorage *);
void *openttd_rust_network_actions_save(OpenTTDNetworkActionsStorage *);
void openttd_rust_network_actions_resume(OpenTTDNetworkActionsStorage *);
void openttd_rust_network_actions_drop(OpenTTDNetworkActionsStorage *);
uint8_t openttd_rust_network_actions_result(const OpenTTDNetworkActionsStorage *);
OpenTTDNetworkFrameState *openttd_rust_network_frame_storage();
uint32_t openttd_rust_network_calculate_lag(const OpenTTDNetworkFrameServices *, const OpenTTDNetworkSocketSync *sync);
}
/** The Rust frame state. */
inline OpenTTDNetworkFrameState &GetRustNetworkFrameState() noexcept
{
	static auto *state = openttd_rust_network_frame_storage();
	return *state;
}
#endif /* RUST_NETWORK_FRAME_FFI_H */

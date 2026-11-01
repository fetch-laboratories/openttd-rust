/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_frame_ffi.h Rust network frame, sync, ack and lag decisions; packets and sockets stay native. */
#ifndef RUST_NETWORK_FRAME_FFI_H
#define RUST_NETWORK_FRAME_FFI_H
#include "service_defaults.hpp"
#include "network_command_ffi.h"
struct OpenTTDSessionFrame;
struct OpenTTDNetworkConnectionServices;
struct OpenTTDNetworkClientServices;
struct OpenTTDNetworkLifecycleServices;
/** The frame globals of the network game loop; C++ aliases each field in place. */
struct OpenTTDNetworkFrameState {
	uint32_t frame_counter_server;
	uint32_t frame_counter_max;
	uint32_t frame_counter;
	uint32_t last_sync_frame;
	uint32_t sync_seed_1;
	uint32_t sync_seed_2;
	uint32_t sync_frame;
	uint32_t last_ack_frame;
	bool network_first_time;
};
/** Frame and lag fields of a game socket, held in place as a base of the socket shell. */
struct OpenTTDNetworkSocketSync {
	size_t receive_limit; ///< Amount of bytes that we can receive at this moment
	uint32_t last_frame; ///< Last frame we have executed
	uint32_t last_frame_server; ///< Last frame the server has executed
	uint32_t last_token_frame; ///< The last frame we received the right token
	uint8_t lag_test; ///< Byte used for lag-testing the client
	uint8_t last_token; ///< The last random token we did send to verify the client is listening
	uint8_t token; ///< The token we need to send back to the server to prove we're the right client.
};
/** Native leaves of the frame protocol: `game` is a NetworkGameSocketHandler, `socket` a NetworkClientSocket, packets are owned until sent. */
struct OpenTTDNetworkFrameServices {
	uint16_t (*bytes_per_frame)() noexcept OPENTTD_SERVICE_DEFAULT(bytes_per_frame);
	uint16_t (*bytes_per_frame_burst)() noexcept OPENTTD_SERVICE_DEFAULT(bytes_per_frame_burst);
	const OpenTTDNetworkClientServices *client;
	void (*client_desync)() noexcept OPENTTD_SERVICE_DEFAULT(client_desync);
	void *(*client_game)() noexcept OPENTTD_SERVICE_DEFAULT(client_game);
	OpenTTDNetworkSocketSync *(*client_sync)() noexcept OPENTTD_SERVICE_DEFAULT(client_sync);
	const OpenTTDNetworkCommandServices *command;
	const OpenTTDNetworkConnectionServices *connection;
	void (*console_bad_acks)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_bad_acks);
	void (*console_download_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_download_timeout);
	void (*console_init_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_init_timeout);
	void (*console_join_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_join_timeout);
	void (*console_lagging)(void *, uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(console_lagging);
	void (*console_password_timeout)(void *) noexcept OPENTTD_SERVICE_DEFAULT(console_password_timeout);
	void (*console_slow)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(console_slow);
	void (*debug_ack_sent)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_ack_sent);
	void (*debug_active)(void *) noexcept OPENTTD_SERVICE_DEFAULT(debug_active);
	void (*debug_missed_sync)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_missed_sync);
	void (*debug_receive_ack)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_ack);
	void (*debug_receive_sync)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_receive_sync);
	void (*debug_send_ack)() noexcept OPENTTD_SERVICE_DEFAULT(debug_send_ack);
	void (*debug_send_sync)(void *, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(debug_send_sync);
	uint8_t (*frame_freq)() noexcept OPENTTD_SERVICE_DEFAULT(frame_freq);
	uint32_t (*interactive_random_range)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(interactive_random_range);
	const OpenTTDNetworkLifecycleServices *lifecycle;
	uint16_t (*max_download_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_download_time);
	uint16_t (*max_init_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_init_time);
	uint16_t (*max_join_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_join_time);
	uint16_t (*max_lag_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_lag_time);
	uint16_t (*max_password_time)() noexcept OPENTTD_SERVICE_DEFAULT(max_password_time);
	int64_t (*now)() noexcept OPENTTD_SERVICE_DEFAULT(now);
	void (*on_server_connect)() noexcept OPENTTD_SERVICE_DEFAULT(on_server_connect);
	bool (*packet_can_read)(void *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(packet_can_read);
	void *(*packet_new)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(packet_new);
	void (*packet_recv_string)(void *, void *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(packet_recv_string);
	uint32_t (*packet_recv_uint32)(void *) noexcept OPENTTD_SERVICE_DEFAULT(packet_recv_uint32);
	uint8_t (*packet_recv_uint8)(void *) noexcept OPENTTD_SERVICE_DEFAULT(packet_recv_uint8);
	void (*packet_send)(void *, void *) noexcept OPENTTD_SERVICE_DEFAULT(packet_send);
	void (*packet_send_string)(void *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(packet_send_string);
	void (*packet_send_uint32)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(packet_send_uint32);
	void (*packet_send_uint8)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(packet_send_uint8);
	const uint32_t *random;
	bool (*receive)(OpenTTDSessionFrame *, bool *) noexcept OPENTTD_SERVICE_DEFAULT(receive);
	void (*replay_commands)() noexcept OPENTTD_SERVICE_DEFAULT(replay_commands);
	bool (*send)(OpenTTDSessionFrame *) noexcept OPENTTD_SERVICE_DEFAULT(send);
	void (*socket_close)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_close);
	void (*socket_defer_deletion)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_defer_deletion);
	bool (*socket_has_quit)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_has_quit);
	int64_t (*socket_last_packet)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_last_packet);
	void (*socket_mark_closed)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_mark_closed);
	bool (*socket_pending_deletion)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_pending_deletion);
	void (*socket_send_packets)(void *, bool) noexcept OPENTTD_SERVICE_DEFAULT(socket_send_packets);
	void (*socket_set_last_packet_now)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_set_last_packet_now);
	bool (*socket_valid)(void *) noexcept OPENTTD_SERVICE_DEFAULT(socket_valid);
	uint16_t (*sync_freq)() noexcept OPENTTD_SERVICE_DEFAULT(sync_freq);
	void (*sync_log)() noexcept OPENTTD_SERVICE_DEFAULT(sync_log);
};
const OpenTTDNetworkFrameServices &GetRustNetworkFrameServices() noexcept;
extern "C" {
OpenTTDNetworkFrameState *openttd_rust_network_frame_storage();
uint32_t openttd_rust_network_calculate_lag(const OpenTTDNetworkFrameServices *, const OpenTTDNetworkSocketSync *sync);
uint8_t openttd_rust_network_receive_ack(const OpenTTDNetworkFrameServices *, void *socket, void *game, uint8_t *status, OpenTTDNetworkSocketSync *sync, void *packet);
uint8_t openttd_rust_network_receive_frame(const OpenTTDNetworkFrameServices *, uint8_t status, OpenTTDNetworkSocketSync *sync, void *packet);
uint8_t openttd_rust_network_receive_sync(const OpenTTDNetworkFrameServices *, uint8_t status, void *packet);
}
/** The Rust frame state. */
inline OpenTTDNetworkFrameState &GetRustNetworkFrameState() noexcept
{
	static auto *state = openttd_rust_network_frame_storage();
	return *state;
}
#endif /* RUST_NETWORK_FRAME_FFI_H */

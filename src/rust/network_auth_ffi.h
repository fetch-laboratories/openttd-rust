/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_auth_ffi.h Game protocol authentication transitions. */
#ifndef RUST_NETWORK_AUTH_FFI_H
#define RUST_NETWORK_AUTH_FFI_H
#include "network_frame_ffi.h"
extern "C" {
uint8_t openttd_rust_network_auth_send_response(const OpenTTDNetworkFrameServices *);
uint8_t openttd_rust_network_auth_receive_request(const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *, OpenTTDPacketState *);
uint8_t openttd_rust_network_auth_receive_enable(const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *, OpenTTDPacketState *);
uint8_t openttd_rust_network_auth_receive_response(const OpenTTDNetworkFrameServices *, OpenTTDNetworkGameSocket *, OpenTTDPacketState *, OpenTTDNetworkActionsStorage *);
}
#endif

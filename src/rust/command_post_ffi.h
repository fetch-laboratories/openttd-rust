/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file command_post_ffi.h Typed argument codecs and native scratch for Rust posts. */
#ifndef RUST_COMMAND_POST_FFI_H
#define RUST_COMMAND_POST_FFI_H
#include "command_services_ffi.h"
#include "network_command_ffi.h"
#include "../openttd.h"
struct OpenTTDSessionServices;
struct OpenTTDTileOwnerContext;
struct OpenTTDCompanyCost;
struct OpenTTDCommandPostServices {
	void (*assign_company_cost)(OpenTTDCommandCost *, const OpenTTDCompanyCost *) noexcept OPENTTD_SERVICE_DEFAULT(assign_company_cost);
	const OpenTTDCommandServices &(*command)() noexcept OPENTTD_SERVICE_DEFAULT(command);
	const OpenTTDTileOwnerContext *(*control_begin)(OpenTTDCommandControlStorage *) noexcept OPENTTD_SERVICE_DEFAULT(control_begin);
	void (*control_end)(OpenTTDCommandControlStorage *) noexcept OPENTTD_SERVICE_DEFAULT(control_end);
	bool (*desync_logging)() noexcept OPENTTD_SERVICE_DEFAULT(desync_logging);
	void (*log)(uint8_t, OpenTTDByteSpan, bool) noexcept OPENTTD_SERVICE_DEFAULT(log);
	const OpenTTDNetworkCommandServices &(*network)() noexcept OPENTTD_SERVICE_DEFAULT(network);
	OpenTTDCommandData (*pack_colour)(bool, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(pack_colour);
	OpenTTDCommandData (*pack_control)(uint8_t, uint8_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(pack_control);
	OpenTTDCommandData (*pack_pause)(PauseMode, bool) noexcept OPENTTD_SERVICE_DEFAULT(pack_pause);
	const OpenTTDSessionServices &(*session)() noexcept OPENTTD_SERVICE_DEFAULT(session);
};
const OpenTTDCommandPostServices &GetRustCommandPostServices() noexcept;
#endif

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_host_ffi.h Rust-owned script host state, lifecycle steps, command continuation and events. */
#ifndef OPENTTD_RUST_SCRIPT_HOST_FFI_H
#define OPENTTD_RUST_SCRIPT_HOST_FFI_H
#include <stddef.h>
#include <stdint.h>
#include "call_ffi.h"
#include "service_defaults.hpp"
#include "abi-record-script.h"
struct OpenTTDCommandCost;
struct OpenTTDCommandServices;
class ScriptEvent;
struct OpenTTDScriptHostServices {
	const OpenTTDCommandServices *command;
	void (*release_event)(ScriptEvent *) noexcept OPENTTD_SERVICE_DEFAULT(release_event);
	uint32_t (*string_to_error)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(string_to_error);
};
/** `OpenTTDScriptHost::mode` and `async_mode`; see `script_host.rs`. */
enum OpenTTDScriptMode : uint8_t {
	SCRIPT_MODE_NONE = 0,
	SCRIPT_MODE_EXEC = 1, ///< ScriptExecMode, or ScriptAsyncMode(true).
	SCRIPT_MODE_TEST = 2, ///< ScriptTestMode, or ScriptAsyncMode(false).
};
/** Next `ScriptInstance::GameLoop` step; see `script_host.rs` `Step`. */
enum class OpenTTDScriptStep : uint8_t {
	Idle,
	Callback,
	PopCallback,
	Start,
	Resume,
	PopResume,
};
/** `ScriptObject::DoCommandPrep` decisions; see `script_host.rs` `Prep`. */
struct OpenTTDScriptPrep {
	bool rejected;
	bool estimate_only;
	bool asynchronous;
	bool networking;
};
/** `ScriptObject::DoCommandProcessResult` decision; see `script_host.rs` `Outcome`. */
enum class OpenTTDScriptOutcome : uint8_t {
	Failed,
	Done,
	Callback,
	ChargedCallback,
	Suspend,
};
struct OpenTTDScriptProcessed {
	OpenTTDScriptOutcome outcome;
	int32_t suspend;
};
const OpenTTDScriptHostServices &GetRustScriptHostServices() noexcept;
/* Serialized by the game-state mutex. VM calls, callbacks and throws run on the native
 * caller's frame between these steps; no Rust borrow spans one. Predicates return 0 or 1. */
extern "C" {
OpenTTDScriptContext *OPENTTD_RUST_CALL openttd_rust_script_context();
OpenTTDScriptHost *OPENTTD_RUST_CALL openttd_rust_script_host_new(ScriptInstance *);
void OPENTTD_RUST_CALL openttd_rust_script_host_free(OpenTTDScriptHost *, const OpenTTDScriptHostServices *);
OpenTTDScriptStep OPENTTD_RUST_CALL openttd_rust_script_loop(OpenTTDScriptHost *, const OpenTTDScriptHostServices *);
OpenTTDScriptStep OPENTTD_RUST_CALL openttd_rust_script_callback_done(OpenTTDScriptHost *);
void OPENTTD_RUST_CALL openttd_rust_script_suspend(OpenTTDScriptHost *, int32_t, ScriptCallback *);
void OPENTTD_RUST_CALL openttd_rust_script_continue(OpenTTDScriptHost *, const OpenTTDScriptHostServices *);
void OPENTTD_RUST_CALL openttd_rust_script_set_company(OpenTTDScriptHost *, const OpenTTDScriptHostServices *, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_script_set_callback_value(OpenTTDScriptHost *, int32_t, int32_t);
void OPENTTD_RUST_CALL openttd_rust_script_set_last_command(OpenTTDScriptHost *, const uint8_t *, size_t, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_script_set_result(OpenTTDScriptHost *, const uint8_t *, size_t);
OpenTTDScriptPrep OPENTTD_RUST_CALL openttd_rust_script_command_prep(OpenTTDScriptHost *, const OpenTTDScriptHostServices *);
OpenTTDScriptProcessed OPENTTD_RUST_CALL openttd_rust_script_command_result(OpenTTDScriptHost *, const OpenTTDScriptHostServices *, const OpenTTDCommandCost *, bool, bool);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_command_done(OpenTTDScriptHost *, const OpenTTDScriptHostServices *, const OpenTTDCommandCost *, const uint8_t *, size_t, const uint8_t *, size_t, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_script_event_push(OpenTTDScriptHost *, ScriptEvent *);
ScriptEvent *OPENTTD_RUST_CALL openttd_rust_script_event_pop(OpenTTDScriptHost *);
uint8_t OPENTTD_RUST_CALL openttd_rust_script_event_waiting(OpenTTDScriptHost *);
}
/** The process-wide active-instance context, accessed in place. */
inline OpenTTDScriptContext &GetRustScriptContext() noexcept
{
	static OpenTTDScriptContext &context = *openttd_rust_script_context();
	return context;
}
#endif /* OPENTTD_RUST_SCRIPT_HOST_FFI_H */

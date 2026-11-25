/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_global_ffi.h Shared Action2/7/9/D global-variable observations. */
#ifndef RUST_NEWGRF_GLOBAL_FFI_H
#define RUST_NEWGRF_GLOBAL_FFI_H
#include "services_ffi.h"
struct GRFConfig;
struct GRFFile;
enum GameMode : uint8_t;
#include "cargo_label_ffi.h"
struct OpenTTDGlobalVariableServices {
	GRFConfig *(*get_grf_config)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(get_grf_config);
	uint8_t (*misc_grf_features)() noexcept OPENTTD_SERVICE_DEFAULT(misc_grf_features);
	uint32_t (*openttd_newgrf_version)() noexcept OPENTTD_SERVICE_DEFAULT(openttd_newgrf_version);
	uint8_t (*palette)(const GRFConfig *) noexcept OPENTTD_SERVICE_DEFAULT(palette);
	const OpenTTDSharedServices *shared;
};
const OpenTTDGlobalVariableServices &GetRustGlobalVariableServices() noexcept;
extern "C" uint8_t openttd_rust_global_variable_abi(const OpenTTDGlobalVariableServices *, uint8_t, uint32_t *, const GRFFile *);
extern "C++" inline bool openttd_rust_global_variable(const OpenTTDGlobalVariableServices *a0, uint8_t a1, uint32_t *a2, const GRFFile *a3) { return openttd_rust_global_variable_abi(a0, a1, a2, a3) != 0; }
#endif

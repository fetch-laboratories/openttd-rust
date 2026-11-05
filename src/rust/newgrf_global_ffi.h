/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_global_ffi.h Shared Action2/7/9/D global-variable observations. */
#ifndef RUST_NEWGRF_GLOBAL_FFI_H
#define RUST_NEWGRF_GLOBAL_FFI_H
#include "services_ffi.h"
struct GRFFile;
enum GameMode : uint8_t;
struct OpenTTDNewGRFCargoLabel { uint32_t value; bool mixed; };
struct OpenTTDGlobalVariableServices {
	uint8_t (*misc_bits)() noexcept OPENTTD_SERVICE_DEFAULT(misc_bits);
	uint32_t (*newgrf_version)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf_version);
	uint8_t (*palette)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(palette);
	int32_t (*pitch)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(pitch);
	uint16_t (*rail_cost)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_cost);
	uint8_t (*road_side)() noexcept OPENTTD_SERVICE_DEFAULT(road_side);
	const OpenTTDSharedServices *shared;
	uint32_t (*width)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(width);
};
const OpenTTDGlobalVariableServices &GetRustGlobalVariableServices() noexcept;
extern "C" uint8_t openttd_rust_global_variable_abi(const OpenTTDGlobalVariableServices *, uint8_t, uint32_t *, const GRFFile *);
extern "C++" inline bool openttd_rust_global_variable(const OpenTTDGlobalVariableServices *a0, uint8_t a1, uint32_t *a2, const GRFFile *a3) { return openttd_rust_global_variable_abi(a0, a1, a2, a3) != 0; }
#endif

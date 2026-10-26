/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_global_ffi.h Shared Action2/7/9/D global-variable observations. */
#ifndef RUST_NEWGRF_GLOBAL_FFI_H
#define RUST_NEWGRF_GLOBAL_FFI_H
#include "services_ffi.h"
struct GRFFile;
enum GameMode : uint8_t;
struct OpenTTDNewGRFCargoLabel { uint32_t value; bool mixed; };
extern "C" uint32_t openttd_rust_newgrf_active_cargo_label(uint8_t (*)(uint32_t) noexcept, OpenTTDNewGRFCargoLabel);
struct OpenTTDGlobalVariableServices {
	uint8_t (*climate)() noexcept OPENTTD_SERVICE_DEFAULT(climate);
	uint16_t (*date_fraction)() noexcept OPENTTD_SERVICE_DEFAULT(date_fraction);
	OpenTTDCalendarYMD (*date_to_ymd)(int32_t) noexcept OPENTTD_SERVICE_DEFAULT(date_to_ymd);
	bool (*disable_elrails)() noexcept OPENTTD_SERVICE_DEFAULT(disable_elrails);
	uint8_t (*file_version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(file_version);
	GameMode (*game_mode)() noexcept OPENTTD_SERVICE_DEFAULT(game_mode);
	uint8_t (*height_limit)() noexcept OPENTTD_SERVICE_DEFAULT(height_limit);
	uint8_t (*misc_bits)() noexcept OPENTTD_SERVICE_DEFAULT(misc_bits);
	uint32_t (*newgrf_version)() noexcept OPENTTD_SERVICE_DEFAULT(newgrf_version);
	uint8_t (*palette)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(palette);
	int32_t (*pitch)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(pitch);
	uint16_t (*rail_cost)(RailType) noexcept OPENTTD_SERVICE_DEFAULT(rail_cost);
	uint8_t (*road_side)() noexcept OPENTTD_SERVICE_DEFAULT(road_side);
	const OpenTTDSharedServices *shared;
	uint32_t (*width)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(width);
	int32_t (*ymd_to_date)(int32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ymd_to_date);
};
const OpenTTDGlobalVariableServices &GetRustGlobalVariableServices() noexcept;
extern "C" bool openttd_rust_global_variable(const OpenTTDGlobalVariableServices *, uint8_t, uint32_t *, const GRFFile *);
#endif

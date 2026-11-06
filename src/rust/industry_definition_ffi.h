/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_definition_ffi.h Native inputs of the Rust industry definitions. */
#ifndef RUST_INDUSTRY_DEFINITION_FFI_H
#define RUST_INDUSTRY_DEFINITION_FFI_H
#include "service_defaults.hpp"
#include "newgrf_bytereader_ffi.h"
#include "../industrytype.h"
struct OpenTTDGrfLoadingServices;
struct OpenTTDSharedServices;
/** Rust owns the specs; the original tables and string ids stay native. Calls hold the game-state mutex. */
struct OpenTTDIndustryDefinitionServices {
	const OpenTTDSharedServices *shared;
	const IndustrySpec *origin;
	const IndustryTileSpec *origin_tiles;
	uint32_t invalid_name;
	void (*map_string)(uint16_t, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(map_string);
};
const OpenTTDIndustryDefinitionServices &GetIndustryDefinitionServices() noexcept;
struct OpenTTDIndustryPropertyContext {
	GRFFile *file;
	uint32_t grfid;
	uint8_t version, climate;
};
extern "C" {
uint8_t openttd_rust_industry_properties_abi(uint32_t, uint32_t, int32_t, ByteReader *, const OpenTTDIndustryPropertyContext *, const OpenTTDByteReaderServices *, const OpenTTDGrfLoadingServices *, int32_t *);
extern "C++" inline bool openttd_rust_industry_properties(uint32_t a0, uint32_t a1, int32_t a2, ByteReader *a3, const OpenTTDIndustryPropertyContext *a4, const OpenTTDByteReaderServices *a5, const OpenTTDGrfLoadingServices *a6, int32_t *a7) { return openttd_rust_industry_properties_abi(a0, a1, a2, a3, a4, a5, a6, a7) != 0; }
uint8_t openttd_rust_industry_tile_properties_abi(uint32_t, uint32_t, int32_t, ByteReader *, const OpenTTDIndustryPropertyContext *, const OpenTTDByteReaderServices *, const OpenTTDGrfLoadingServices *, int32_t *);
extern "C++" inline bool openttd_rust_industry_tile_properties(uint32_t a0, uint32_t a1, int32_t a2, ByteReader *a3, const OpenTTDIndustryPropertyContext *a4, const OpenTTDByteReaderServices *a5, const OpenTTDGrfLoadingServices *a6, int32_t *a7) { return openttd_rust_industry_tile_properties_abi(a0, a1, a2, a3, a4, a5, a6, a7) != 0; }
uint8_t openttd_rust_industry_file_badge_abi(const GRFFile *, bool, uint32_t);
extern "C++" inline bool openttd_rust_industry_file_badge(const GRFFile *a0, bool a1, BadgeID a2) { return openttd_rust_industry_file_badge_abi(a0, a1, a2.base()) != 0; }
}
#endif

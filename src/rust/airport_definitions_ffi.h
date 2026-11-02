/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport_definitions_ffi.h Canonical native airport definition payloads. */
#ifndef RUST_AIRPORT_DEFINITIONS_FFI_H
#define RUST_AIRPORT_DEFINITIONS_FFI_H
#include "newgrf_bytereader_ffi.h"
#include "override_manager_ffi.h"
#include "spec_identity.h"
#include "../newgrf_airport.h"
struct AirportSpec;
struct AirportTileSpec;
struct AirportTileTable;
struct OpenTTDSharedServices;
struct OpenTTDAirportDefinitionServices {
	const OpenTTDByteReaderServices *reader;
	const OpenTTDSharedServices *shared;
	size_t (*airport_count)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(airport_count);
	void (*airport_resize)(GRFFile *file, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(airport_resize);
	AirportSpec * (*airport_get)(GRFFile *file, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(airport_get);
	AirportSpec * (*airport_new)(GRFFile *file, size_t index, const AirportSpec *source) noexcept OPENTTD_SERVICE_DEFAULT(airport_new);
	void (*airport_set_substitute)(AirportSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_set_substitute);
	size_t (*tile_count)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(tile_count);
	void (*tile_resize)(GRFFile *file, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(tile_resize);
	AirportTileSpec * (*tile_get)(GRFFile *file, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(tile_get);
	AirportTileSpec * (*tile_new)(GRFFile *file, size_t index, const AirportTileSpec *source) noexcept OPENTTD_SERVICE_DEFAULT(tile_new);
	void (*tile_set_substitute)(AirportTileSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(tile_set_substitute);
	AirportSpec * (*airport_base)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(airport_base);
	AirportTileSpec * (*tile_base)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_base);
	void (*airport_default)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(airport_default);
	void (*airport_clear)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(airport_clear);
	void (*tile_default)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_default);
	void (*tile_clear)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_clear);
	void (*airport_publish)(uint8_t id, AirportSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(airport_publish);
	void (*tile_publish)(StationGfx id, AirportTileSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(tile_publish);
	void (*airport_file_clear)(GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(airport_file_clear);
	void (*airport_file_shrink)(GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(airport_file_shrink);
	void (*tile_file_clear)(GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(tile_file_clear);
	void (*tile_file_shrink)(GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(tile_file_shrink);
	void (*airport_min_year)(AirportSpec *spec, int32_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_min_year);
	void (*airport_max_year)(AirportSpec *spec, int32_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_max_year);
	void (*airport_ttdp_type)(AirportSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_ttdp_type);
	void (*airport_catchment)(AirportSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_catchment);
	void (*airport_noise)(AirportSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_noise);
	void (*airport_maintenance)(AirportSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_maintenance);
	void (*airport_name)(AirportSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(airport_name);
	void (*tile_clear_animation)(AirportTileSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(tile_clear_animation);
	void (*tile_callbacks)(AirportTileSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(tile_callbacks);
	void (*layouts_resize)(AirportSpec *spec, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(layouts_resize);
	void (*layout_assign)(AirportSpec *spec, size_t index, uint8_t rotation, const AirportTileTable *tiles, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(layout_assign);
	void (*dimensions)(AirportSpec *spec, uint8_t x, uint8_t y) noexcept OPENTTD_SERVICE_DEFAULT(dimensions);
	void (*airport_invalid)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(airport_invalid);
	void (*airport_undefined)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(airport_undefined);
	void (*airport_substitute_invalid)(uint8_t subs_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(airport_substitute_invalid);
	void (*layout_tile_undefined)(uint16_t local_tile_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(layout_tile_undefined);
	void (*airport_full)() noexcept OPENTTD_SERVICE_DEFAULT(airport_full);
	void (*tile_invalid)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(tile_invalid);
	void (*tile_undefined)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_undefined);
	void (*tile_substitute_invalid)(uint8_t subs_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_substitute_invalid);
	void (*tile_override_invalid)(uint8_t override_id, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(tile_override_invalid);
	void (*tile_full)() noexcept OPENTTD_SERVICE_DEFAULT(tile_full);
};
void RustAirportTileDefault(uint16_t id) noexcept;
const OpenTTDAirportDefinitionServices &GetAirportDefinitionServices() noexcept;
extern "C" {
bool openttd_rust_airport_change_info(const OpenTTDAirportDefinitionServices *, GRFFile *, uint32_t, uint32_t, uint32_t, ByteReader *, uint8_t *);
bool openttd_rust_airport_tile_change_info(const OpenTTDAirportDefinitionServices *, GRFFile *, uint32_t, uint32_t, uint32_t, ByteReader *, uint8_t *);
}
#endif

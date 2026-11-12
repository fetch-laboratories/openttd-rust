/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport_definitions.cpp Native storage leaves for airport loading. */
#include "../stdafx.h"
#include "airport_definitions_ffi.h"
#include "../newgrf_airport.h"
#include "../newgrf_airporttiles.h"
#include "newgrf_bytereader_services.h"
#include "services_newgrf.h"
#include "services_station.h"
#include "../safeguards.h"
const OpenTTDAirportDefinitionServices &GetAirportDefinitionServices() noexcept
{
	static const OpenTTDAirportDefinitionServices services{
		.reader = &GetRustByteReaderServices(),
		.shared = &GetRustSharedServices(),
		.airport_count = [](const GRFFile *file) noexcept -> size_t { return file->airportspec.size(); },
		.airport_resize = [](GRFFile *file, size_t count) noexcept -> void { file->airportspec.resize(count); },
		.airport_get = [](GRFFile *file, size_t index) noexcept -> AirportSpec * { return file->airportspec[index].get(); },
		.airport_new = [](GRFFile *file, size_t index, const AirportSpec *source) noexcept -> AirportSpec * { file->airportspec[index] = std::make_unique<AirportSpec>(*source); return file->airportspec[index].get(); },
		.airport_set_substitute = RustSpecSetSubstitute<AirportSpec>,
		.tile_count = [](const GRFFile *file) noexcept -> size_t { return file->airtspec.size(); },
		.tile_resize = [](GRFFile *file, size_t count) noexcept -> void { file->airtspec.resize(count); },
		.tile_get = [](GRFFile *file, size_t index) noexcept -> AirportTileSpec * { return file->airtspec[index].get(); },
		.tile_new = [](GRFFile *file, size_t index, const AirportTileSpec *source) noexcept -> AirportTileSpec * { file->airtspec[index] = std::make_unique<AirportTileSpec>(*source); return file->airtspec[index].get(); },
		.tile_set_substitute = RustSpecSetSubstitute<AirportTileSpec>,
		.airport_base = RustAirportSpec<AirportSpec *>,
		.tile_base = RustAirportTileSpec<AirportTileSpec *>,
		.airport_default = [](uint16_t id) noexcept -> void { extern const AirportSpec _origin_airport_specs[NEW_AIRPORT_OFFSET]; *AirportSpec::RustRawSpec(id) = _origin_airport_specs[id]; },
		.airport_clear = [](uint16_t id) noexcept -> void { *AirportSpec::RustRawSpec(id) = AirportSpec{}; },
		.tile_default = RustAirportTileDefault,
		.tile_clear = [](uint16_t id) noexcept -> void { *const_cast<AirportTileSpec *>(AirportTileSpec::Get(id)) = AirportTileSpec{}; },
		.airport_publish = [](uint8_t id, AirportSpec *spec) noexcept -> void { *AirportSpec::RustRawSpec(id) = std::move(*spec); },
		.tile_publish = [](StationGfx id, AirportTileSpec *spec) noexcept -> void { *const_cast<AirportTileSpec *>(AirportTileSpec::Get(id)) = std::move(*spec); },
		.airport_file_clear = [](GRFFile *file) noexcept -> void { file->airportspec.clear(); },
		.airport_file_shrink = [](GRFFile *file) noexcept -> void { file->airportspec.shrink_to_fit(); },
		.tile_file_clear = [](GRFFile *file) noexcept -> void { file->airtspec.clear(); },
		.tile_file_shrink = [](GRFFile *file) noexcept -> void { file->airtspec.shrink_to_fit(); },
		.airport_min_year = [](AirportSpec *spec, int32_t value) noexcept -> void { spec->min_year = TimerGameCalendar::Year{value}; },
		.airport_max_year = [](AirportSpec *spec, int32_t value) noexcept -> void { spec->max_year = TimerGameCalendar::Year{value}; },
		.airport_ttdp_type = [](AirportSpec *spec, uint8_t value) noexcept -> void { spec->ttd_airport_type = static_cast<TTDPAirportType>(value); },
		.airport_catchment = [](AirportSpec *spec, uint8_t value) noexcept -> void { spec->catchment = value; },
		.airport_noise = [](AirportSpec *spec, uint8_t value) noexcept -> void { spec->noise_level = value; },
		.airport_maintenance = [](AirportSpec *spec, uint16_t value) noexcept -> void { spec->maintenance_cost = value; },
		.airport_name = RustSpecRegisterName<AirportSpec>,
		.tile_clear_animation = RustSpecClearAnimation<AirportTileSpec>,
		.tile_callbacks = [](AirportTileSpec *spec, uint8_t value) noexcept -> void { spec->callback_mask = static_cast<AirportTileCallbackMasks>(value); },
		.layouts_resize = [](AirportSpec *spec, size_t count) noexcept -> void { spec->layouts.clear(); spec->layouts.resize(count); },
		.layout_assign = [](AirportSpec *spec, size_t index, uint8_t rotation, const AirportTileTable *tiles, size_t count) noexcept -> void { auto &layout = spec->layouts[index]; layout.rotation = static_cast<Direction>(rotation); layout.tiles.assign(tiles, tiles + count); },
		.airport_size_x = [](AirportSpec *spec, uint8_t value) noexcept -> void { spec->size_x = value; },
		.airport_size_y = [](AirportSpec *spec, uint8_t value) noexcept -> void { spec->size_y = value; },
	};
	return services;
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file airport_definitions.cpp Native storage leaves for airport loading. */
#include "../stdafx.h"
#ifdef WITH_RUST
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
		.file = RustGRFFileAt,
		.file_id = RustGRFFileID,
		.airport_count = [](const GRFFile *file) noexcept -> size_t { return file->airportspec.size(); },
		.airport_resize = [](GRFFile *file, size_t count) noexcept -> void { file->airportspec.resize(count); },
		.airport_get = [](GRFFile *file, size_t index) noexcept -> AirportSpec * { return file->airportspec[index].get(); },
		.airport_new = [](GRFFile *file, size_t index, const AirportSpec *source) noexcept -> AirportSpec * { file->airportspec[index] = std::make_unique<AirportSpec>(*source); return file->airportspec[index].get(); },
		.airport_enabled = RustSpecEnabled<AirportSpec>,
		.airport_set_enabled = RustSpecSetEnabled<AirportSpec>,
		.airport_identity = RustSpecSubstituteIdentity<AirportSpec>,
		.airport_set_local = RustSpecSetLocalID<AirportSpec>,
		.airport_set_substitute = RustSpecSetSubstitute<AirportSpec>,
		.airport_set_file = RustSpecSetGRFFile<AirportSpec>,
		.airport_set_override = RustSpecSetOverride<AirportSpec>,
		.airport_badges = RustSpecBadgeList<AirportSpec>,
		.tile_count = [](const GRFFile *file) noexcept -> size_t { return file->airtspec.size(); },
		.tile_resize = [](GRFFile *file, size_t count) noexcept -> void { file->airtspec.resize(count); },
		.tile_get = [](GRFFile *file, size_t index) noexcept -> AirportTileSpec * { return file->airtspec[index].get(); },
		.tile_new = [](GRFFile *file, size_t index, const AirportTileSpec *source) noexcept -> AirportTileSpec * { file->airtspec[index] = std::make_unique<AirportTileSpec>(*source); return file->airtspec[index].get(); },
		.tile_enabled = RustSpecEnabled<AirportTileSpec>,
		.tile_set_enabled = RustSpecSetEnabled<AirportTileSpec>,
		.tile_identity = RustSpecSubstituteIdentity<AirportTileSpec>,
		.tile_set_local = RustSpecSetLocalID<AirportTileSpec>,
		.tile_set_substitute = RustSpecSetSubstitute<AirportTileSpec>,
		.tile_set_file = RustSpecSetGRFFile<AirportTileSpec>,
		.tile_set_override = RustSpecSetOverride<AirportTileSpec>,
		.tile_badges = RustSpecBadgeList<AirportTileSpec>,
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
		.tile_frames = RustSpecAnimationFrames<AirportTileSpec>,
		.tile_status = RustSpecAnimationStatus<AirportTileSpec>,
		.tile_speed = RustSpecAnimationSpeed<AirportTileSpec>,
		.tile_triggers = RustSpecAnimationTriggers<AirportTileSpec>,
		.layouts_resize = [](AirportSpec *spec, size_t count) noexcept -> void { spec->layouts.clear(); spec->layouts.resize(count); },
		.layout_assign = [](AirportSpec *spec, size_t index, uint8_t rotation, const AirportTileTable *tiles, size_t count) noexcept -> void { auto &layout = spec->layouts[index]; layout.rotation = static_cast<Direction>(rotation); layout.tiles.assign(tiles, tiles + count); },
		.dimensions = [](AirportSpec *spec, uint8_t x, uint8_t y) noexcept -> void { spec->size_x = x; spec->size_y = y; },
		.airport_invalid = [](uint32_t last) noexcept -> void { GrfMsg(1, "AirportChangeInfo: Too many airports, trying id ({}), max ({}). Ignoring.", last, NUM_AIRPORTS_PER_GRF); },
		.airport_undefined = [](uint32_t id) noexcept -> void { GrfMsg(2, "AirportChangeInfo: Attempt to modify undefined airport {}, ignoring", id); },
		.airport_substitute_invalid = [](uint8_t subs_id, uint32_t id) noexcept -> void { GrfMsg(2, "AirportChangeInfo: Attempt to use new airport {} as substitute airport for {}. Ignoring.", subs_id, id); },
		.layout_tile_undefined = [](uint16_t local_tile_id, uint32_t id) noexcept -> void { GrfMsg(2, "AirportChangeInfo: Attempt to use airport tile {} with airport id {}, not yet defined. Ignoring.", local_tile_id, id); },
		.airport_full = []() noexcept -> void { GrfMsg(1, "Airport.SetEntitySpec: Too many airports allocated. Ignoring."); },
		.tile_invalid = [](uint32_t last) noexcept -> void { GrfMsg(1, "AirportTileChangeInfo: Too many airport tiles loaded ({}), max ({}). Ignoring.", last, NUM_AIRPORTTILES_PER_GRF); },
		.tile_undefined = [](uint32_t id) noexcept -> void { GrfMsg(2, "AirportTileChangeInfo: Attempt to modify undefined airport tile {}. Ignoring.", id); },
		.tile_substitute_invalid = [](uint8_t subs_id, uint32_t id) noexcept -> void { GrfMsg(2, "AirportTileChangeInfo: Attempt to use new airport tile {} as substitute airport tile for {}. Ignoring.", subs_id, id); },
		.tile_override_invalid = [](uint8_t override_id, uint32_t id) noexcept -> void { GrfMsg(2, "AirportTileChangeInfo: Attempt to override new airport tile {} with airport tile id {}. Ignoring.", override_id, id); },
		.tile_full = []() noexcept -> void { GrfMsg(1, "AirportTile.SetEntitySpec: Too many airport tiles allocated. Ignoring."); },
	};
	return services;
}
#endif

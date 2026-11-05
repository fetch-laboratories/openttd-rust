/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_definitions.cpp Typed native storage leaves for station definitions. */
#include "../stdafx.h"
#include "station_definitions_ffi.h"
#include "newgrf_bytereader_services.h"
#include "../newgrf_station.h"
#include "../newgrf_roadstop.h"
#include "../newgrf/newgrf_stringmapping.h"
#include "services_newgrf.h"
#include "services_station.h"
#include "../table/strings.h"
#include "../safeguards.h"
const OpenTTDStationDefinitionServices &GetStationDefinitionServices() noexcept
{
	static const OpenTTDStationDefinitionServices services{
		.empty_string = STR_EMPTY,
		.reader = &GetRustByteReaderServices(),
		.shared = &GetRustSharedServices(),
		.station_resize = [](GRFFile *file, uint32_t count) noexcept -> void { file->stations.resize(count); },
		.station_count = [](const GRFFile *file) noexcept -> size_t { return file->stations.size(); },
		.station_get = [](GRFFile *file, uint32_t id) noexcept -> StationSpec * { return id < file->stations.size() ? file->stations[id].get() : nullptr; },
		.station_new = [](GRFFile *file, uint32_t id) noexcept -> StationSpec * { file->stations[id] = std::make_unique<StationSpec>(); return file->stations[id].get(); },
		.station_class = [](StationSpec *spec, uint16_t value) noexcept -> void { spec->class_index = static_cast<StationClassID>(value); },
		.station_name = RustSpecRegisterName<StationSpec>,
		.station_class_name = RustSpecRegisterClassName<StationSpec, openttd_rust_station_class_set_name>,
		.station_cargo_triggers = [](StationSpec *spec, CargoTypes value) noexcept -> void { spec->cargo_triggers = value; },
		.station_callback_mask = [](StationSpec *spec, uint8_t value) noexcept -> void { spec->callback_mask = static_cast<StationCallbackMasks>(value); },
		.station_flags = [](StationSpec *spec, uint8_t value) noexcept -> void { spec->flags = StationSpecFlags{static_cast<uint8_t>(value)}; },
		.station_set_group = RustSpecSetCargoGroup<StationSpec>,
		.road_resize = [](GRFFile *file, uint32_t count) noexcept -> void { file->roadstops.resize(count); },
		.road_count = [](const GRFFile *file) noexcept -> size_t { return file->roadstops.size(); },
		.road_get = [](GRFFile *file, uint32_t id) noexcept -> RoadStopSpec * { return id < file->roadstops.size() ? file->roadstops[id].get() : nullptr; },
		.road_new = [](GRFFile *file, uint32_t id) noexcept -> RoadStopSpec * { file->roadstops[id] = std::make_unique<RoadStopSpec>(); return file->roadstops[id].get(); },
		.road_class = [](RoadStopSpec *spec, uint16_t value) noexcept -> void { spec->class_index = static_cast<RoadStopClassID>(value); },
		.road_name = RustSpecRegisterName<RoadStopSpec>,
		.road_class_name = RustSpecRegisterClassName<RoadStopSpec, openttd_rust_roadstop_class_set_name>,
		.road_cargo_triggers = [](RoadStopSpec *spec, CargoTypes value) noexcept -> void { spec->cargo_triggers = value; },
		.road_callback_mask = [](RoadStopSpec *spec, uint8_t value) noexcept -> void { spec->callback_mask = static_cast<RoadStopCallbackMasks>(value); },
		.road_flags = [](RoadStopSpec *spec, uint32_t value) noexcept -> void { spec->flags = RoadStopSpecFlags{static_cast<uint8_t>(value)}; },
		.road_set_group = RustSpecSetCargoGroup<RoadStopSpec>,
		.station_disallowed_platforms = [](StationSpec *spec, uint8_t value) noexcept -> void { spec->disallowed_platforms = value; },
		.station_disallowed_lengths = [](StationSpec *spec, uint8_t value) noexcept -> void { spec->disallowed_lengths = value; },
		.station_cargo_threshold = [](StationSpec *spec, uint16_t value) noexcept -> void { spec->cargo_threshold = value; },
		.road_stop_type = [](RoadStopSpec *spec, uint8_t value) noexcept -> void { spec->stop_type = static_cast<RoadStopAvailabilityType>(value); },
		.road_draw_mode = [](RoadStopSpec *spec, uint8_t value) noexcept -> void { spec->draw_mode = RoadStopDrawModes(value); },
		.road_build_cost = [](RoadStopSpec *spec, uint8_t value) noexcept -> void { spec->build_cost_multiplier = value; },
		.road_clear_cost = [](RoadStopSpec *spec, uint8_t value) noexcept -> void { spec->clear_cost_multiplier = value; },
		.station_render_clear = [](StationSpec *spec) noexcept -> void { spec->renderdata.clear(); },
		.station_render_reserve = [](StationSpec *spec, uint16_t count) noexcept -> void { spec->renderdata.reserve(count); },
		.station_render_size = [](const StationSpec *spec) noexcept -> size_t { return spec->renderdata.size(); },
		.station_render_append = [](StationSpec *spec) noexcept -> NewGRFSpriteLayout * { return &spec->renderdata.emplace_back(); },
		.station_render_pop = [](StationSpec *spec) noexcept -> void { spec->renderdata.pop_back(); },
		.station_render_copy_append = [](StationSpec *spec, const StationSpec *source, size_t index) noexcept -> void { spec->renderdata.emplace_back(source->renderdata[index]); },
		.sequence_reset = [](OpenTTDStationSequence *sequence) noexcept { OpenTTDStationSequence{}.swap(*sequence); },
		.sequence_append = [](OpenTTDStationSequence *sequence) noexcept -> DrawTileSeqStruct * { return &sequence->emplace_back(); },
		.layout_sequence_move = [](NewGRFSpriteLayout *layout, OpenTTDStationSequence *sequence) noexcept { layout->seq = std::move(*sequence); },
		.layout_original = [](NewGRFSpriteLayout *layout, uint32_t index) noexcept -> void { extern const DrawTileSpriteSpan _station_display_datas_rail[8]; const auto &span = _station_display_datas_rail[index]; layout->ground = span.ground; layout->seq.insert(layout->seq.end(), span.GetSequence().begin(), span.GetSequence().end()); },
		.station_custom_assign = [](StationSpec *spec, uint16_t key, const uint8_t *data, size_t count) noexcept -> void { spec->layouts[key].assign(data, data + count); },
		.station_custom_tile = [](StationSpec *spec, uint16_t key, size_t index, uint8_t value) noexcept -> void { spec->layouts[key][index] = value; },
		.station_custom_copy = [](StationSpec *spec, const StationSpec *source) noexcept -> void { spec->layouts = source->layouts; },
		.station_tileflags_size = RustStationTileFlagCount,
		.station_tileflags_resize = [](StationSpec *spec, size_t count) noexcept -> void { spec->tileflags.resize(count); },
		.station_pylons = [](StationSpec *spec, uint8_t index, bool set) noexcept -> void { spec->tileflags[index].Set(StationSpec::TileFlag::Pylons, set); },
		.station_wires = [](StationSpec *spec, uint8_t index, bool set) noexcept -> void { spec->tileflags[index].Set(StationSpec::TileFlag::NoWires, set); },
		.station_blocked = [](StationSpec *spec, uint8_t index, bool set) noexcept -> void { spec->tileflags[index].Set(StationSpec::TileFlag::Blocked, set); },
		.station_tileflags_assign = [](StationSpec *spec, const uint8_t *data, size_t count) noexcept -> void { auto flags = reinterpret_cast<const StationSpec::TileFlags *>(data); spec->tileflags.assign(flags, flags + count); },
		.station_bridge_size = RustStationBridgeInfoCount<StationSpec>,
		.station_bridge_resize = [](StationSpec *spec, size_t count) noexcept -> void { spec->bridgeable_info.resize(count); },
		.road_bridge_size = RustStationBridgeInfoCount<RoadStopSpec>,
		.station_bridge_height = [](StationSpec *spec, size_t index, uint8_t value) noexcept -> void { spec->bridgeable_info[index].height = value; },
		.station_bridge_pillars = [](StationSpec *spec, size_t index, uint8_t value) noexcept -> void { spec->bridgeable_info[index].disallowed_pillars = BridgePillarFlags{value}; },
		.road_bridge_height = [](RoadStopSpec *spec, size_t index, uint8_t value) noexcept -> void { spec->bridgeable_info[index].height = value; },
		.road_bridge_pillars = [](RoadStopSpec *spec, size_t index, uint8_t value) noexcept -> void { spec->bridgeable_info[index].disallowed_pillars = BridgePillarFlags{value}; },
	};
	return services;
}

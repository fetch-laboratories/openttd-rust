/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_definitions_ffi.h Native spec storage and canonical payload operations. */
#ifndef RUST_STATION_DEFINITIONS_FFI_H
#define RUST_STATION_DEFINITIONS_FFI_H
#include "service_defaults.hpp"
#include "newgrf_bytereader_ffi.h"
#include "../newgrf_commons.h"
struct StationSpec;
struct RoadStopSpec;
struct GRFFile;
struct SpriteGroup;
using OpenTTDStationSequence = std::vector<DrawTileSeqStruct>;
struct OpenTTDStationSequenceFields {
	int8_t *origin_x, *origin_y, *origin_z;
	uint8_t *extent_x, *extent_y, *extent_z;
	PalSpriteID *image;
};
struct OpenTTDAct3CargoServices;
struct OpenTTDStationDefinitionServices {
	StringID empty_string;
	const OpenTTDAct3CargoServices *cargo;
	const OpenTTDByteReaderServices *reader;
	uint8_t (*file_version)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(file_version);
	void (*station_resize)(GRFFile *file, uint32_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_resize);
	size_t (*station_count)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(station_count);
	StationSpec * (*station_get)(GRFFile *file, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(station_get);
	StationSpec * (*station_new)(GRFFile *file, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(station_new);
	void (*station_class)(StationSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_class);
	void (*station_name)(StationSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_name);
	void (*station_class_name)(StationSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_class_name);
	std::vector<BadgeID> * (*station_badges)(StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_badges);
	void (*station_cargo_triggers)(StationSpec *spec, CargoTypes value) noexcept OPENTTD_SERVICE_DEFAULT(station_cargo_triggers);
	void (*station_frames)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_frames);
	void (*station_status)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_status);
	void (*station_speed)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_speed);
	void (*station_triggers)(StationSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_triggers);
	void (*station_callback_mask)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_callback_mask);
	void (*station_flags)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_flags);
	bool (*station_has_file)(const StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_has_file);
	void (*station_set_group)(StationSpec *spec, CargoType cargo, const SpriteGroup *group) noexcept OPENTTD_SERVICE_DEFAULT(station_set_group);
	void (*station_set_file)(StationSpec *spec, GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(station_set_file);
	void (*station_set_local)(StationSpec *spec, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(station_set_local);
	void (*road_resize)(GRFFile *file, uint32_t count) noexcept OPENTTD_SERVICE_DEFAULT(road_resize);
	size_t (*road_count)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(road_count);
	RoadStopSpec * (*road_get)(GRFFile *file, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(road_get);
	RoadStopSpec * (*road_new)(GRFFile *file, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(road_new);
	void (*road_class)(RoadStopSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_class);
	void (*road_name)(RoadStopSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_name);
	void (*road_class_name)(RoadStopSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_class_name);
	std::vector<BadgeID> * (*road_badges)(RoadStopSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(road_badges);
	void (*road_cargo_triggers)(RoadStopSpec *spec, CargoTypes value) noexcept OPENTTD_SERVICE_DEFAULT(road_cargo_triggers);
	void (*road_frames)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_frames);
	void (*road_status)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_status);
	void (*road_speed)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_speed);
	void (*road_triggers)(RoadStopSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_triggers);
	void (*road_callback_mask)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_callback_mask);
	void (*road_flags)(RoadStopSpec *spec, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_flags);
	bool (*road_has_file)(const RoadStopSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(road_has_file);
	void (*road_set_group)(RoadStopSpec *spec, CargoType cargo, const SpriteGroup *group) noexcept OPENTTD_SERVICE_DEFAULT(road_set_group);
	void (*road_set_file)(RoadStopSpec *spec, GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(road_set_file);
	void (*road_set_local)(RoadStopSpec *spec, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(road_set_local);
	void (*station_disallowed_platforms)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_disallowed_platforms);
	void (*station_disallowed_lengths)(StationSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_disallowed_lengths);
	void (*station_cargo_threshold)(StationSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_cargo_threshold);
	void (*road_stop_type)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_stop_type);
	void (*road_draw_mode)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_draw_mode);
	void (*road_build_cost)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_build_cost);
	void (*road_clear_cost)(RoadStopSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_clear_cost);
	void (*station_render_clear)(StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_render_clear);
	void (*station_render_reserve)(StationSpec *spec, uint16_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_render_reserve);
	size_t (*station_render_size)(const StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_render_size);
	NewGRFSpriteLayout * (*station_render_append)(StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_render_append);
	void (*station_render_pop)(StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_render_pop);
	void (*station_render_copy_append)(StationSpec *spec, const StationSpec *source, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(station_render_copy_append);
	void (*layout_unlimited)(NewGRFSpriteLayout *layout) noexcept OPENTTD_SERVICE_DEFAULT(layout_unlimited);
	PalSpriteID * (*layout_ground)(NewGRFSpriteLayout *layout) noexcept OPENTTD_SERVICE_DEFAULT(layout_ground);
	void (*sequence_reset)(OpenTTDStationSequence *) noexcept OPENTTD_SERVICE_DEFAULT(sequence_reset);
	OpenTTDStationSequenceFields (*sequence_append)(OpenTTDStationSequence *) noexcept OPENTTD_SERVICE_DEFAULT(sequence_append);
	void (*layout_sequence_move)(NewGRFSpriteLayout *, OpenTTDStationSequence *) noexcept OPENTTD_SERVICE_DEFAULT(layout_sequence_move);
	void (*layout_original)(NewGRFSpriteLayout *layout, uint32_t index) noexcept OPENTTD_SERVICE_DEFAULT(layout_original);
	void (*station_custom_assign)(StationSpec *spec, uint16_t key, const uint8_t *data, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_custom_assign);
	void (*station_custom_tile)(StationSpec *spec, uint16_t key, size_t index, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_custom_tile);
	void (*station_custom_copy)(StationSpec *spec, const StationSpec *source) noexcept OPENTTD_SERVICE_DEFAULT(station_custom_copy);
	size_t (*station_tileflags_size)(const StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_tileflags_size);
	void (*station_tileflags_resize)(StationSpec *spec, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_tileflags_resize);
	void (*station_pylons)(StationSpec *spec, uint8_t index, bool set) noexcept OPENTTD_SERVICE_DEFAULT(station_pylons);
	void (*station_wires)(StationSpec *spec, uint8_t index, bool set) noexcept OPENTTD_SERVICE_DEFAULT(station_wires);
	void (*station_blocked)(StationSpec *spec, uint8_t index, bool set) noexcept OPENTTD_SERVICE_DEFAULT(station_blocked);
	void (*station_tileflags_assign)(StationSpec *spec, const uint8_t *data, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_tileflags_assign);
	size_t (*station_bridge_size)(const StationSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(station_bridge_size);
	void (*station_bridge_resize)(StationSpec *spec, size_t count) noexcept OPENTTD_SERVICE_DEFAULT(station_bridge_resize);
	size_t (*road_bridge_size)(const RoadStopSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(road_bridge_size);
	void (*station_bridge_height)(StationSpec *spec, size_t index, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_bridge_height);
	void (*station_bridge_pillars)(StationSpec *spec, size_t index, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(station_bridge_pillars);
	void (*road_bridge_height)(RoadStopSpec *spec, size_t index, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_bridge_height);
	void (*road_bridge_pillars)(RoadStopSpec *spec, size_t index, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(road_bridge_pillars);
	void (*station_invalid)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(station_invalid);
	void (*road_invalid)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(road_invalid);
	void (*station_undefined)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(station_undefined);
	void (*road_undefined)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(road_undefined);
	void (*odd_layout)(uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(odd_layout);
	void (*missing_sprite)(uint16_t source, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(missing_sprite);
	void (*missing_custom)(uint16_t source, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(missing_custom);
	void (*invalid_tile)(uint8_t tile, uint8_t length, uint8_t number) noexcept OPENTTD_SERVICE_DEFAULT(invalid_tile);
	void (*undefined_publication)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(undefined_publication);
	void (*duplicate_publication)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(duplicate_publication);
	bool (*read_layout_sprite)(ByteReader *reader, bool flags, bool invert, bool current, GrfSpecFeature feature, PalSpriteID *sprite, uint8_t *result) noexcept OPENTTD_SERVICE_DEFAULT(read_layout_sprite);
	bool (*read_layout)(ByteReader *reader, uint32_t count, bool current, GrfSpecFeature feature, bool var10, bool no_z, NewGRFSpriteLayout *layout, bool *disabled) noexcept OPENTTD_SERVICE_DEFAULT(read_layout);
};
const OpenTTDStationDefinitionServices &GetStationDefinitionServices() noexcept;
extern "C" {
bool openttd_rust_station_change_info(const OpenTTDStationDefinitionServices *, GRFFile *, uint32_t, uint32_t, uint32_t, ByteReader *, OpenTTDStationSequence *, uint8_t *);
bool openttd_rust_roadstop_change_info(const OpenTTDStationDefinitionServices *, GRFFile *, uint32_t, uint32_t, uint32_t, ByteReader *, uint8_t *);
void openttd_rust_station_map_specific(const OpenTTDStationDefinitionServices *, GRFFile *, uint16_t, uint8_t, const SpriteGroup *);
void openttd_rust_station_map_default(const OpenTTDStationDefinitionServices *, GRFFile *, uint16_t, const SpriteGroup *);
void openttd_rust_roadstop_map_specific(const OpenTTDStationDefinitionServices *, GRFFile *, uint16_t, uint8_t, const SpriteGroup *);
void openttd_rust_roadstop_map_default(const OpenTTDStationDefinitionServices *, GRFFile *, uint16_t, const SpriteGroup *);
}
#endif

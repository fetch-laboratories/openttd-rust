/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file house_definition_ffi.h Native shells and typed leaves for house definitions. */
#ifndef RUST_HOUSE_DEFINITION_FFI_H
#define RUST_HOUSE_DEFINITION_FFI_H
#include "newgrf_bytereader_ffi.h"
#include "../house.h"
struct OpenTTDHouseDefinitionIdentity { uint32_t grfid; uint16_t local, substitute; };
struct OpenTTDHouseDefinitionValidity { uint16_t availability, substitute; uint8_t flags; };
struct OpenTTDHouseDefinitionPart { uint8_t flags, population; bool enabled; };
struct OpenTTDHouseDefinitionEarly { int32_t year; uint16_t availability; bool enabled; };
struct OpenTTDHouseDefinitionCargo { uint32_t label; CargoType cargo; };
struct OpenTTDHouseResolverServices;
struct OpenTTDGrfLoadingServices;
struct OpenTTDHouseDefinitionServices {
	const OpenTTDHouseResolverServices *resolver;
	const HouseSpec *(*get)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(get);
	size_t (*count)() noexcept OPENTTD_SERVICE_DEFAULT(count);
	void (*clear_specs)() noexcept OPENTTD_SERVICE_DEFAULT(clear_specs);
	void (*reserve_specs)(size_t count) noexcept OPENTTD_SERVICE_DEFAULT(reserve_specs);
	void (*append_default)(uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(append_default);
	size_t (*file_count)() noexcept OPENTTD_SERVICE_DEFAULT(file_count);
	GRFFile *(*file_at)(size_t index) noexcept OPENTTD_SERVICE_DEFAULT(file_at);
	size_t (*local_count)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(local_count);
	bool (*has_filename)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(has_filename);
	void (*resize_local)(GRFFile *file, size_t size) noexcept OPENTTD_SERVICE_DEFAULT(resize_local);
	const HouseSpec *(*local)(const GRFFile *file, size_t index) noexcept OPENTTD_SERVICE_DEFAULT(local);
	void (*clear_local)(GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(clear_local);
	const HouseSpec *(*copy_substitute)(GRFFile *file, uint16_t id, const HouseSpec *source) noexcept OPENTTD_SERVICE_DEFAULT(copy_substitute);
	void (*set_identity)(const HouseSpec *spec, GRFFile *file, uint16_t local, uint16_t substitute) noexcept OPENTTD_SERVICE_DEFAULT(set_identity);
	OpenTTDHouseDefinitionIdentity (*identity)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	OpenTTDHouseDefinitionValidity (*validity)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(validity);
	OpenTTDHouseDefinitionPart (*part)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(part);
	uint8_t (*flags)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(flags);
	OpenTTDHouseDefinitionEarly (*early)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(early);
	OpenTTDHouseDefinitionCargo (*cargo)(const HouseSpec *spec, uint8_t index) noexcept OPENTTD_SERVICE_DEFAULT(cargo);
	void (*set_cargo)(const HouseSpec *spec, uint8_t index, CargoType cargo) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo);
	void (*set_label)(const HouseSpec *spec, uint8_t index, uint32_t label) noexcept OPENTTD_SERVICE_DEFAULT(set_label);
	void (*set_acceptance)(const HouseSpec *spec, uint8_t index, uint8_t amount) noexcept OPENTTD_SERVICE_DEFAULT(set_acceptance);
	void (*set_colour)(const HouseSpec *spec, uint8_t index, uint8_t colour) noexcept OPENTTD_SERVICE_DEFAULT(set_colour);
	uint16_t (*callbacks)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	uint64_t (*watched)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(watched);
	void (*map_string)(const HouseSpec *spec, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(map_string);
	void (*set_animation)(const HouseSpec *spec, uint8_t frames, AnimationStatus status) noexcept OPENTTD_SERVICE_DEFAULT(set_animation);
	std::vector<BadgeID> *(*badges)(const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	uint32_t (*file_id)(const GRFFile *file) noexcept OPENTTD_SERVICE_DEFAULT(file_id);
	void (*resize_specs)(size_t count) noexcept OPENTTD_SERVICE_DEFAULT(resize_specs);
	void (*move_spec)(const HouseSpec *spec, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(move_spec);
	void (*set_override)(const HouseSpec *spec, uint16_t id) noexcept OPENTTD_SERVICE_DEFAULT(set_override);
	uint16_t (*climate_mask)() noexcept OPENTTD_SERVICE_DEFAULT(climate_mask);
	LandscapeType (*landscape)() noexcept OPENTTD_SERVICE_DEFAULT(landscape);
	void (*too_many)(uint32_t last) noexcept OPENTTD_SERVICE_DEFAULT(too_many);
	void (*invalid_substitute)(uint8_t substitute, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(invalid_substitute);
	void (*invalid_override)(uint8_t target, uint32_t id) noexcept OPENTTD_SERVICE_DEFAULT(invalid_override);
	void (*allocation_failed)() noexcept OPENTTD_SERVICE_DEFAULT(allocation_failed);
	void (*invalid_missing)(const GRFFile *file, const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(invalid_missing);
	void (*invalid_population)(const GRFFile *file, const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(invalid_population);
	void (*invalid_size)(const GRFFile *file, const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(invalid_size);
	void (*invalid_availability)(const GRFFile *file, const HouseSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(invalid_availability);
	void (*set_enabled)(const HouseSpec *spec, bool value) noexcept OPENTTD_SERVICE_DEFAULT(set_enabled);
	void (*set_flags)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_flags);
	void (*set_min_year)(const HouseSpec *spec, int32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_min_year);
	void (*set_max_year)(const HouseSpec *spec, int32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_max_year);
	void (*set_population)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_population);
	void (*set_mail)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_mail);
	void (*set_rating)(const HouseSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_rating);
	void (*set_removal)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_removal);
	void (*set_availability)(const HouseSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_availability);
	void (*set_callbacks)(const HouseSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_callbacks);
	void (*set_processing)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_processing);
	void (*set_probability)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_probability);
	void (*set_extra)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_extra);
	void (*set_animation_speed)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_animation_speed);
	void (*set_class_id)(const HouseSpec *spec, uint16_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_class_id);
	void (*set_minimum_life)(const HouseSpec *spec, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_minimum_life);
	void (*set_watched)(const HouseSpec *spec, uint64_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_watched);
};
const OpenTTDHouseDefinitionServices &GetHouseDefinitionServices() noexcept;
extern "C" {
uint8_t openttd_rust_house_definition_property(const OpenTTDHouseDefinitionServices *, const OpenTTDByteReaderServices *, const OpenTTDGrfLoadingServices *, GRFFile *, uint32_t, uint32_t, int, ByteReader *);
}
#endif

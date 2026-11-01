/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_definition_ffi.h Industry definition storage leaves. */
#ifndef RUST_INDUSTRY_DEFINITION_FFI_H
#define RUST_INDUSTRY_DEFINITION_FFI_H
#include "service_defaults.hpp"
#include "spec_identity.h"
#include "newgrf_global_ffi.h"
#include "../industry_type.h"
#include <cstdint>
#include <cstddef>
struct OpenTTDIndustryGenerationServices;
struct OpenTTDOverrideManager;
struct IndustrySpec;
struct IndustryTileSpec;
struct GRFFile;
struct OpenTTDIndustryDefinitionAvailability { bool enabled, has_file; };
struct OpenTTDIndustryProducedSlots { uint8_t *cargo; OpenTTDNewGRFCargoLabel labels[INDUSTRY_ORIGINAL_NUM_OUTPUTS]; };
struct OpenTTDIndustryAcceptedSlots { uint8_t *cargo; OpenTTDNewGRFCargoLabel labels[INDUSTRY_ORIGINAL_NUM_INPUTS]; };
struct OpenTTDIndustryDefinitionCargo { OpenTTDIndustryProducedSlots produced; OpenTTDIndustryAcceptedSlots accepted; };
/** Native specs remain canonical. Load/reset is serialized by the game-state mutex;
 * leaves never yield or reset definitions. No borrowed spec survives vector cleanup. */
struct OpenTTDIndustryDefinitionServices {
	const OpenTTDIndustryGenerationServices *generation;
	const OpenTTDSharedServices *shared;
	IndustrySpec *(*industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	IndustryTileSpec *(*tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	OpenTTDIndustryDefinitionAvailability (*availability)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(availability);
	void (*publish)(uint16_t, IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(publish);
	void (*publish_tile)(uint16_t, IndustryTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(publish_tile);
	void (*industry_full)() noexcept OPENTTD_SERVICE_DEFAULT(industry_full);
	void (*tile_full)() noexcept OPENTTD_SERVICE_DEFAULT(tile_full);
	void (*reset_industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_industry);
	void (*clear_industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_industry);
	bool (*climate_available)(const IndustrySpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(climate_available);
	void (*reset_tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_tile);
	void (*clear_tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_tile);
	size_t (*industry_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(industry_count);
	size_t (*tile_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(tile_count);
	IndustrySpec *(*file_industry)(GRFFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(file_industry);
	IndustryTileSpec *(*file_tile)(GRFFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(file_tile);
	void (*clear_file)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(clear_file);
	uint8_t *(*conflicting)(IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(conflicting);
	void (*invalid_name)(IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(invalid_name);
	OpenTTDIndustryDefinitionCargo (*cargo)(IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(cargo);
	OpenTTDIndustryAcceptedSlots (*tile_cargo)(IndustryTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_cargo);
	void (*set_produced)(IndustrySpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_produced);
	void (*set_accepted)(IndustrySpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_accepted);
	void (*set_tile_accepted)(IndustryTileSpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_accepted);
};
const OpenTTDIndustryDefinitionServices &GetIndustryDefinitionServices() noexcept;
#endif

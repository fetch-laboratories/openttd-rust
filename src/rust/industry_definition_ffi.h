/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_definition_ffi.h Industry definition storage leaves. */
#ifndef RUST_INDUSTRY_DEFINITION_FFI_H
#define RUST_INDUSTRY_DEFINITION_FFI_H
#include "service_defaults.hpp"
#include "spec_identity.h"
#include "newgrf_global_ffi.h"
#include <cstdint>
#include <cstddef>
struct OpenTTDIndustryGenerationServices;
struct OpenTTDOverrideManager;
struct IndustrySpec;
struct IndustryTileSpec;
struct GRFFile;
struct OpenTTDIndustryDefinitionAvailability { bool enabled, has_file; };
struct OpenTTDIndustryDefinitionCargo { OpenTTDNewGRFCargoLabel label; uint8_t cargo; };
/** Native specs remain canonical. Load/reset is serialized by the game-state mutex;
 * leaves never yield or reset definitions. No borrowed spec survives vector cleanup. */
struct OpenTTDIndustryDefinitionServices {
	const OpenTTDIndustryGenerationServices *generation;
	IndustrySpec *(*industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	IndustryTileSpec *(*tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(tile);
	uint32_t (*grfid)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	OpenTTDSubstituteSpecIdentity (*identity)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	OpenTTDSubstituteSpecIdentity (*tile_identity)(const IndustryTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_identity);
	OpenTTDIndustryDefinitionAvailability (*availability)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(availability);
	void (*set_enabled)(IndustrySpec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_enabled);
	void (*set_override)(IndustrySpec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_override);
	void (*set_tile_override)(IndustryTileSpec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_override);
	void (*set_tile_enabled)(IndustryTileSpec *, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_enabled);
	void (*publish)(uint16_t, IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(publish);
	void (*publish_tile)(uint16_t, IndustryTileSpec *) noexcept OPENTTD_SERVICE_DEFAULT(publish_tile);
	void (*industry_full)() noexcept OPENTTD_SERVICE_DEFAULT(industry_full);
	void (*tile_full)() noexcept OPENTTD_SERVICE_DEFAULT(tile_full);
	void (*reset_industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_industry);
	void (*clear_industry)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_industry);
	bool (*climate_available)(const IndustrySpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(climate_available);
	void (*reset_tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_tile);
	void (*clear_tile)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(clear_tile);
	GRFFile *(*file)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(file);
	size_t (*industry_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(industry_count);
	size_t (*tile_count)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(tile_count);
	IndustrySpec *(*file_industry)(GRFFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(file_industry);
	IndustryTileSpec *(*file_tile)(GRFFile *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(file_tile);
	void (*clear_file)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(clear_file);
	uint8_t *(*conflicting)(IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(conflicting);
	void (*invalid_name)(IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(invalid_name);
	OpenTTDIndustryDefinitionCargo (*produced)(const IndustrySpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(produced);
	OpenTTDIndustryDefinitionCargo (*accepted)(const IndustrySpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(accepted);
	OpenTTDIndustryDefinitionCargo (*tile_accepted)(const IndustryTileSpec *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_accepted);
	void (*set_produced)(IndustrySpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_produced);
	void (*set_accepted)(IndustrySpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_accepted);
	void (*set_tile_accepted)(IndustryTileSpec *, size_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_accepted);
};
const OpenTTDIndustryDefinitionServices &GetIndustryDefinitionServices() noexcept;
extern "C" {
uint16_t openttd_rust_industry_definition_allocate(OpenTTDOverrideManager *, uint16_t, uint32_t, uint16_t, const OpenTTDIndustryDefinitionServices *);
void openttd_rust_industry_definition_reset(uint8_t, const OpenTTDIndustryDefinitionServices *);
}
#endif

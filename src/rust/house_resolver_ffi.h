/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file house_resolver_ffi.h Stack contexts for house and shared-town resolution. */
#ifndef RUST_HOUSE_RESOLVER_FFI_H
#define RUST_HOUSE_RESOLVER_FFI_H
#include "town_ffi.h"
#include "newgrf_storage_ffi.h"
struct ResolverObject;
struct GRFFile;
struct Station;
struct OpenTTDTownScope {
	Town *town;
	OpenTTDTownState *owner;
	uint16_t id;
	bool readonly;
};
struct OpenTTDHouseScope {
	OpenTTDTownScope town;
	uint64_t watched;
	uint32_t tile;
	int32_t view;
	uint16_t house;
	uint16_t initial_random;
	bool prospective;
};
struct OpenTTDHouseSearchSpec { uint32_t grfid; uint16_t local, house_class; bool has_file; };
struct OpenTTDHouseNearbySpec { const GRFFile *file; uint16_t local, house_class; };
struct OpenTTDHouseResolverServices {
	const OpenTTDTownLeaves *towns;
	const OpenTTDSharedServices *shared;
	const OpenTTDStorageServices *storage;
	uint8_t (*larger_towns)() noexcept OPENTTD_SERVICE_DEFAULT(larger_towns);
	uint8_t (*cargo_by_label)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_by_label);
	int32_t (*get_register)(const ResolverObject *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(get_register);
	const GRFFile *(*file)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(file);
	uint8_t (*version)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(version);
	uint8_t (*cargo_translation)(uint8_t, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_translation);
	uint32_t (*nearby)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby);
	uint32_t (*nearby_info)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(nearby_info);
	uint32_t (*terrain)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(terrain);
	uint8_t (*stage)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(stage);
	uint8_t (*age)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(age);
	uint8_t (*random_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	uint8_t (*random_triggers)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_triggers);
	uint8_t (*animation)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(animation);
	OpenTTDHouseSearchSpec (*search_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(search_spec);
	OpenTTDHouseNearbySpec (*nearby_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(nearby_spec);
	uint16_t (*house_id)(uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(house_id);
	uint32_t (*house_grfid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(house_grfid);
	uint32_t (*badges)(const ResolverObject *, uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	void (*stations)(uint32_t, void *, void (*)(void *, const Station *)) noexcept OPENTTD_SERVICE_DEFAULT(stations);
	void (*unhandled_house)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled_house);
	void (*unhandled_town)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled_town);
};
const OpenTTDHouseResolverServices &GetHouseResolverServices() noexcept;
extern "C" {
void openttd_rust_house_classes_reset();
uint16_t openttd_rust_house_class_allocate(uint8_t, uint32_t);
void openttd_rust_house_counts_init(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDTownState *, size_t);
uint32_t openttd_rust_house_variable(const OpenTTDHouseResolverServices *, const OpenTTDHouseScope *, const ResolverObject *, uint8_t, uint32_t, bool *);
uint32_t openttd_rust_house_random(const OpenTTDHouseResolverServices *, const OpenTTDHouseScope *, bool);
uint32_t openttd_rust_town_variable(const OpenTTDHouseResolverServices *, const OpenTTDTownScope *, const ResolverObject *, uint8_t, uint32_t, bool *);
void openttd_rust_town_scope_store(const OpenTTDHouseResolverServices *, const OpenTTDTownScope *, const ResolverObject *, uint32_t, int32_t);
}
#endif

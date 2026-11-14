/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file subsidy_ffi.h Canonical subsidy scalars and synchronous shared leaves. */
#ifndef RUST_SUBSIDY_FFI_H
#define RUST_SUBSIDY_FFI_H
#ifndef OPENTTD_SUBSIDY_FIXTURE_TYPES
#include "../source_type.h"
#include "../subsidy_type.h"
#include "../cargo_type.h"
#endif
#include "service_defaults.hpp"
#include "town_ffi.h"
struct OpenTTDIndustry;
struct Industry;
struct OpenTTDIndustryServices;
/** Native aliases address this sole Rust allocation. Calls are serialized by the
 * game-state mutex. No owner borrow survives allocation, news, events or deletion;
 * noexcept shared leaves terminate environmental failures and never run a VM. */
struct OpenTTDSubsidyState { CargoType cargo_type; uint16_t remaining; CompanyID awarded; Source src, dst; };
/** The contiguous scalar data of a native stack CargoArray. */
struct OpenTTDSubsidyCargo { uint32_t *data; };
struct OpenTTDSourceFormatStrings { uint32_t industry_name, town_name; };
struct OpenTTDSubsidyStrings { uint32_t company_name, offer_of_subsidy_expired, service_subsidy_awarded_half, service_subsidy_offered; OpenTTDSourceFormatStrings source; uint32_t subsidy_withdrawn_service; };
struct OpenTTDSubsidyServices {
	const OpenTTDSharedServices * shared;
	const OpenTTDTownLeaves * town;
	const OpenTTDIndustryServices * industry;
	OpenTTDIndustry *(*industry_owner)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_owner);
	uint16_t (*industry_id)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_id);
	uint32_t (*next_industry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_industry);
	bool (*has_source_flag)(Source, PartOfSubsidy) noexcept OPENTTD_SERVICE_DEFAULT(has_source_flag);
	void (*set_source_flag)(Source, PartOfSubsidy) noexcept OPENTTD_SERVICE_DEFAULT(set_source_flag);
	void (*clear_source_flags)(Source) noexcept OPENTTD_SERVICE_DEFAULT(clear_source_flags);
	bool (*valid_source)(Source) noexcept OPENTTD_SERVICE_DEFAULT(valid_source);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	uint16_t (*allocate)(CargoType, Source, Source, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	void (*destroy)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	uint16_t (*duration)() noexcept OPENTTD_SERVICE_DEFAULT(duration);
	uint8_t (*multiplier)() noexcept OPENTTD_SERVICE_DEFAULT(multiplier);
	bool (*cargo_valid)(CargoType) noexcept OPENTTD_SERVICE_DEFAULT(cargo_valid);
	bool (*manual)(CargoType) noexcept OPENTTD_SERVICE_DEFAULT(manual);
	bool (*manual_pax)() noexcept OPENTTD_SERVICE_DEFAULT(manual_pax);
	bool (*any_manual)() noexcept OPENTTD_SERVICE_DEFAULT(any_manual);
	OpenTTDSubsidyStrings strings;
	void (*dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	const Station *(*station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	void (*assert_destination)(Source) noexcept OPENTTD_SERVICE_DEFAULT(assert_destination);
};
extern "C" {
OpenTTDSubsidyState *openttd_rust_subsidy_new(uint16_t);
void openttd_rust_subsidy_destroy(uint16_t, OpenTTDSubsidyState *);
void openttd_rust_subsidy_rebuild(const OpenTTDSubsidyServices *);
void openttd_rust_subsidy_delete_with(const OpenTTDSubsidyServices *, Source);
uint8_t openttd_rust_subsidy_command_abi(const OpenTTDSubsidyServices *, uint16_t, CargoType, Source, Source);
extern "C++" inline bool openttd_rust_subsidy_command(const OpenTTDSubsidyServices *a0, uint16_t a1, CargoType a2, Source a3, Source a4) { return openttd_rust_subsidy_command_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_subsidy_check_abi(const OpenTTDSubsidyServices *, CargoType, uint8_t, Source, uint16_t);
extern "C++" inline bool openttd_rust_subsidy_check(const OpenTTDSubsidyServices *a0, CargoType a1, uint8_t a2, Source a3, uint16_t a4) { return openttd_rust_subsidy_check_abi(a0, a1, a2, a3, a4) != 0; }
}
#endif /* RUST_SUBSIDY_FFI_H */

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
struct OpenTTDIndustryServices;
/** Native aliases address this sole Rust allocation. Calls are serialized by the
 * game-state mutex. No owner borrow survives allocation, news, events or deletion;
 * noexcept shared leaves terminate environmental failures and never run a VM. */
struct OpenTTDSubsidyState { CargoType cargo_type; uint16_t remaining; CompanyID awarded; Source src, dst; };
/** Native stack CargoArray and the actual contiguous scalar data it owns. */
struct OpenTTDSubsidyCargo { void *object; uint32_t *data; };
struct OpenTTDSubsidyServices {
	const OpenTTDSharedServices * shared;
	const OpenTTDTownLeaves * town;
	const OpenTTDIndustryServices * industry;
	OpenTTDTownRef (*random_town)() noexcept OPENTTD_SERVICE_DEFAULT(random_town);
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
	uint8_t (*local_company)() noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	bool (*cargo_valid)(CargoType) noexcept OPENTTD_SERVICE_DEFAULT(cargo_valid);
	bool (*manual)(CargoType) noexcept OPENTTD_SERVICE_DEFAULT(manual);
	bool (*manual_pax)() noexcept OPENTTD_SERVICE_DEFAULT(manual_pax);
	bool (*any_manual)() noexcept OPENTTD_SERVICE_DEFAULT(any_manual);
	uint32_t (*passenger_count)() noexcept OPENTTD_SERVICE_DEFAULT(passenger_count);
	CargoType (*passenger_at)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(passenger_at);
	void (*offered_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(offered_news);
	void (*awarded_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(awarded_news);
	void (*offer_expired_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(offer_expired_news);
	void (*withdrawn_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(withdrawn_news);
	void (*offer_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(offer_events);
	void (*award_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(award_events);
	void (*offer_expired_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(offer_expired_events);
	void (*expired_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(expired_events);
	void (*dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	const Station *(*station)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station);
	bool (*station_empty)(const Station *) noexcept OPENTTD_SERVICE_DEFAULT(station_empty);
	void (*catchment)(const Station *, void *, void (*)(void *, uint32_t)) noexcept OPENTTD_SERVICE_DEFAULT(catchment);
	void (*near_industries)(const Station *, void *, void (*)(void *, uint16_t)) noexcept OPENTTD_SERVICE_DEFAULT(near_industries);
	void (*assert_destination)(Source) noexcept OPENTTD_SERVICE_DEFAULT(assert_destination);
};
extern "C" {
OpenTTDSubsidyState *openttd_rust_subsidy_new(uint16_t);
void openttd_rust_subsidy_destroy(uint16_t, OpenTTDSubsidyState *);
void openttd_rust_subsidy_rebuild(const OpenTTDSubsidyServices *);
void openttd_rust_subsidy_delete_with(const OpenTTDSubsidyServices *, Source);
bool openttd_rust_subsidy_command(const OpenTTDSubsidyServices *, uint16_t, CargoType, Source, Source);
void openttd_rust_subsidy_monthly(const OpenTTDSubsidyServices *, OpenTTDSubsidyCargo);
bool openttd_rust_subsidy_check(const OpenTTDSubsidyServices *, CargoType, uint8_t, Source, uint16_t);
}
#endif /* RUST_SUBSIDY_FFI_H */

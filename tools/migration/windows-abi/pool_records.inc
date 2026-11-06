/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#pragma once
#include "town.h"
#include "industry.h"
#include "subsidy_base.h"
/* The town, industry and subsidy records hold native class-typed fields, so offsetof
 * is conditionally supported; every offset and width is checked under each ABI. */
static void PoolRecordLayouts()
{
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Winvalid-offsetof"
#endif
#include "abi-layout-town_record.inc"
#include "abi-layout-industry_record.inc"
#include "abi-layout-subsidy_record.inc"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
	/* Each record is the first member of its Rust owner, so the owner's address is the record's. */
	OpenTTDTownState *town = openttd_rust_town_new(nullptr);
	CHECK(reinterpret_cast<OpenTTDTownRecord *>(town)->xy == INVALID_TILE);
	CHECK(reinterpret_cast<OpenTTDTownRecord *>(town)->exclusivity == CompanyID::Invalid());
	openttd_rust_town_destroy(town);
	OpenTTDIndustry *industry = openttd_rust_industry_new(nullptr);
	CHECK(reinterpret_cast<OpenTTDIndustryRecord *>(industry)->location.tile == INVALID_TILE);
	CHECK(reinterpret_cast<OpenTTDIndustryRecord *>(industry)->owner == INVALID_OWNER);
	openttd_rust_industry_destroy(industry);
	std::printf("town, industry and subsidy record layouts and owner defaults passed\n");
}
ABI_PROBE(PoolRecordLayouts);

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_services.cpp Native pool identity and feature associations. */
#include "../stdafx.h"
#include "../newgrf_storage.h"
#include "../industry.h"
#include "../newgrf_industries.h"
#include "../station_base.h"
#include "../town.h"
#include "services_newgrf.h"
#include "town_services.h"
#include "../safeguards.h"

static PersistentStorage *StorageCreate(uint32_t grfid, uint8_t feature, uint32_t tile) noexcept
{
	assert(PersistentStorage::CanAllocateItem());
	return new PersistentStorage(grfid, static_cast<GrfSpecFeature>(feature), TileIndex(tile));
}
static void StorageDestroy(PersistentStorage *storage) noexcept { delete storage; }
/** One serialized canonical owner resolution; null-in/null-out is required. */
static OpenTTDPersistentStorageRecord *StorageRecord(const PersistentStorage *storage) noexcept
{
	return storage == nullptr ? nullptr : const_cast<OpenTTDPersistentStorageRecord *>(&storage->Record());
}
OpenTTDStorage256 *RustIndustryStorageOwner(Industry *industry) noexcept
{
	return industry->psa == nullptr ? nullptr : industry->psa->rust_storage;
}
static OpenTTDIndustryStorage StorageIndustry(Industry *industry) noexcept
{
	if (industry->index == IndustryID::Invalid()) return {nullptr, false};
	return {RustIndustryStorageOwner(industry), true};
}
static OpenTTDStorage256 *StorageCreateIndustry(Industry *industry) noexcept
{
	const IndustrySpec *spec = GetIndustrySpec(industry->type);
	industry->psa = StorageCreate(spec->grf_prop.grfid, GSF_INDUSTRIES, industry->location.tile.base());
	return industry->psa->rust_storage;
}
static OpenTTDStorage256 *StorageAirport(Station *station) noexcept { return station->airport.psa == nullptr ? nullptr : station->airport.psa->rust_storage; }
static OpenTTDStorage256 *StorageCreateAirport(Station *station, uint32_t grfid) noexcept
{
	station->airport.psa = StorageCreate(grfid, GSF_AIRPORTS, station->airport.tile.base());
	return station->airport.psa->rust_storage;
}
static OpenTTDPersistentStorageRecord *StorageTown(const Town *town, size_t index) noexcept
{
	return StorageRecord(town->psa_list[index]);
}
static OpenTTDStorage256 *StorageCreateTown(Town *town, uint32_t grfid) noexcept
{
	return StorageCreate(grfid, GSF_FAKE_TOWNS, town->xy.base())->rust_storage;
}
static void StorageAppendTown(Town *town, BasePersistentStorageArray *storage) noexcept { town->psa_list.push_back(static_cast<PersistentStorage *>(storage)); }
const OpenTTDStorageServices &GetRustStorageServices() noexcept
{
	static const OpenTTDStorageServices services{
		.shared = &GetRustSharedServices(),
		.check = RustAssertCondition,
		.create = StorageCreate,
		.destroy = StorageDestroy,
		.record = StorageRecord,
		.industry = StorageIndustry,
		.create_industry = StorageCreateIndustry,
		.airport = StorageAirport,
		.create_airport = StorageCreateAirport,
		.town_count = RustTownStorageCount<const Town>,
		.town_storage = StorageTown,
		.create_town = StorageCreateTown,
		.append_town = StorageAppendTown,
	};
	return services;
}
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDStorageServices.hpp"
bool CheckAbi_storage() { return AbiServicesComplete(GetRustStorageServices()); }

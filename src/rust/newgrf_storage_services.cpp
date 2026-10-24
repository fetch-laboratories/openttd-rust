/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_services.cpp Native pool identity and feature associations. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../newgrf_storage.h"
#include "../industry.h"
#include "../newgrf_industries.h"
#include "../station_base.h"
#include "../town.h"
#include "services_newgrf.h"
#include "town_services.h"
#include "../safeguards.h"

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
	assert(PersistentStorage::CanAllocateItem());
	industry->psa = new PersistentStorage(spec->grf_prop.grfid, GSF_INDUSTRIES, industry->location.tile);
	return industry->psa->rust_storage;
}
static OpenTTDStorage256 *StorageAirport(Station *station) noexcept { return station->airport.psa == nullptr ? nullptr : station->airport.psa->rust_storage; }
static OpenTTDStorage256 *StorageCreateAirport(Station *station, uint32_t grfid) noexcept
{
	assert(PersistentStorage::CanAllocateItem());
	station->airport.psa = new PersistentStorage(grfid, GSF_AIRPORTS, station->airport.tile);
	return station->airport.psa->rust_storage;
}
static OpenTTDStorageReference StorageTown(const Town *town, size_t index) noexcept
{
	const PersistentStorage *storage = town->psa_list[index];
	return {storage->rust_storage, storage->grfid};
}
static OpenTTDStorage256 *StorageCreateTown(Town *town, uint32_t grfid) noexcept
{
	assert(PersistentStorage::CanAllocateItem());
	return (new PersistentStorage(grfid, GSF_FAKE_TOWNS, town->xy))->rust_storage;
}
static void StorageAppendTown(Town *town, BasePersistentStorageArray *storage) noexcept { town->psa_list.push_back(static_cast<PersistentStorage *>(storage)); }
const OpenTTDStorageServices &GetRustStorageServices() noexcept
{
	static const OpenTTDStorageServices services{
		.check = RustAssertCondition,
		.grfid = RustGRFFileID,
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
#endif /* WITH_RUST */

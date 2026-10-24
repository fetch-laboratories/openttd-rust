/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Real resolver facades and native associations, without an external GRF program. */
#include "newgrf_town.h"
#include "newgrf_industries.h"
#include "newgrf_airport.h"
#include "industry.h"
#include "industrytype.h"
static bool StorageScenario(std::span<std::string_view> args)
{
	auto Require = [](bool good) { if (!good) std::abort(); };
	if (args[1] == "storage-load") {
		Require(PersistentStorage::GetNumItems() == 0);
		auto file = FileHandle::Open(args[2], "rb"); Require(file.has_value());
		_sl.reader = std::make_unique<ReadBuffer>(std::make_shared<FileReader>(std::move(*file)));
		_sl.action = SLA_LOAD; _sl_version = args[3] == "legacy16" ? SLV_EXTEND_RAILTYPES : SAVEGAME_VERSION; _next_offs = 0;
		bool failed = false;
		try { SlLoadChunk(*SlFindChunkHandler('PSAC')); } catch (const std::exception &) { failed = true; }
		if (args[3] == "header") {
			Require(failed && PersistentStorage::GetNumItems() == 0);
		} else if (args[3] == "partial") {
			Require(failed && PersistentStorage::GetNumItems() == 2);
			auto *first = PersistentStorage::Get(PersistentStorageID{0}), *partial = PersistentStorage::Get(PersistentStorageID{1});
			Require(first->grfid == 0x39312345 && first->GetValue(0) == INT32_MIN && first->GetValue(255) == INT32_MAX);
			Require(partial->grfid == 0xAABBCCDD && partial->GetValue(0) == -7 && partial->GetValue(1) == 0);
		} else {
			Require(!failed && PersistentStorage::GetNumItems() == 1);
			auto *legacy = PersistentStorage::Get(PersistentStorageID{0});
			Require(legacy->grfid == 0x39312345 && legacy->GetValue(0) == INT32_MIN && legacy->GetValue(15) == -393 && legacy->GetValue(16) == 0 && legacy->GetValue(255) == 0);
		}
		fmt::print(stderr, "STORAGE load {} true\n", args[3]); std::fflush(stderr); std::_Exit(0);
	}
	Town *town = *Town::Iterate().begin();
	Industry *industry = *Industry::Iterate().begin();
	Station *station = nullptr;
	for (Station *st : Station::Iterate()) if (st->facilities.Test(StationFacility::Airport)) { station = st; break; }
	Require(station != nullptr);
	GRFFile grf; grf.grfid = 0x39312345;
	ResolverObject ro(&grf), anonymous(nullptr);
	TownScopeResolver writable(ro, town, false), readonly(ro, nullptr, true), no_grf(anonymous, town, false);
	IndustriesScopeResolver ind(ro, industry->location.tile, industry, industry->type);
	AirportScopeResolver airport(ro, station->airport.tile, station, nullptr, 0), preview(ro, TileIndex{}, nullptr, nullptr, 0);
	bool available = true;
	if (args[1] == "storage-inspect") {
		ro.SetRegister(0x100, -1);
		Require(town->psa_list.size() == 2 && writable.GetVariable(0x7C, 0, available) == 0x80000000 && writable.GetVariable(0x7C, 255, available) == INT32_MAX);
		Require(industry->psa != nullptr && industry->psa->GetValue(15) == -393 && station->airport.psa != nullptr && station->airport.psa->GetValue(255) == 393);
		Require(industry->psa->feature == GSF_INDUSTRIES && industry->psa->tile == industry->location.tile && station->airport.psa->feature == GSF_AIRPORTS && station->airport.psa->tile == station->airport.tile);
		for (auto *psa : town->psa_list) Require(psa->feature == GSF_FAKE_TOWNS && psa->tile == town->xy);
		fmt::print(stderr, "STORAGE reloaded true\n");
		return true;
	}
	Require(town->psa_list.empty() && industry->psa == nullptr && station->airport.psa == nullptr);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_COMMAND);
	preview.StorePSA(0, 7); readonly.StorePSA(0, 7); no_grf.StorePSA(0, 7);
	ro.SetRegister(0x100, 123); writable.StorePSA(0, 7);
	ind.StorePSA(0, 0); airport.StorePSA(0, 0);
	Require(town->psa_list.empty() && industry->psa == nullptr && station->airport.psa == nullptr);
	/* Nonzero out-of-range writes allocate industry/airport, town even for zero. */
	ro.SetRegister(0x100, -1); writable.StorePSA(256, 0); ind.StorePSA(256, 7); airport.StorePSA(256, 7);
	Require(town->psa_list.size() == 1 && industry->psa != nullptr && station->airport.psa != nullptr);
	Require(industry->psa->grfid == GetIndustrySpec(industry->type)->grf_prop.grfid && station->airport.psa->grfid == grf.grfid);
	writable.StorePSA(0, INT32_MIN); writable.StorePSA(255, INT32_MAX); ind.StorePSA(15, -393); airport.StorePSA(255, 393);
	/* Another GRF gets a distinct array; explicit GRFID reads may cross GRFs. */
	GRFFile other; other.grfid = 0x39354321;
	TownResolverObject second(&other, town, false); second.SetRegister(0x100, -1); second.town_scope.StorePSA(0, 99);
	ro.SetRegister(0x100, other.grfid); writable.StorePSA(0, 77);
	Require(writable.GetVariable(0x7C, 0, available) == 99);
	ro.SetRegister(0x100, -1); Require(writable.GetVariable(0x7C, 0, available) == 0x80000000);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_COMMAND);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_TESTMODE);
	writable.StorePSA(0, 1); ind.StorePSA(15, 2); airport.StorePSA(255, 3);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_TESTMODE);
	Require(writable.GetVariable(0x7C, 0, available) == 0x80000000 && industry->psa->GetValue(15) == -393 && station->airport.psa->GetValue(255) == 393);
	/* Destroy a registered array and require native pool ID reuse. */
	Require(PersistentStorage::CanAllocateItem());
	auto *pending = new PersistentStorage(0, GSF_INVALID, TileIndex{}); auto id = pending->index; pending->StoreValue(0, 1); delete pending;
	Require(PersistentStorage::CanAllocateItem());
	auto *reused = new PersistentStorage(0, GSF_INVALID, TileIndex{}); Require(reused->index == id); delete reused;
	/* Real PSAC saving restores temporary payloads; the next switch still visits them. */
	writable.StorePSA(0, 4); ind.StorePSA(15, 5); airport.StorePSA(255, 6);
	Require(args.size() == 3 && SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK);
	Require(writable.GetVariable(0x7C, 0, available) == 0x80000000 && industry->psa->GetValue(15) == -393 && station->airport.psa->GetValue(255) == 393);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
	fmt::print(stderr, "STORAGE policies rollback reuse saved true\n");
	return true;
}

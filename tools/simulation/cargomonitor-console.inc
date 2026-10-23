/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Real facades/chunk handlers; malformed loads terminate before any further game work. */
#include "cargomonitor.h"
static bool MonitorScenario(std::span<std::string_view> args)
{
	if (args[1] == "monitors-capture") {
		if (SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) FixtureAbort(__LINE__);
		fmt::print(stderr, "MONITOR captured true\n"); return true;
	}
	Station *st = *Station::Iterate().begin();
	CargoType cargo = 63; CompanyID company{14};
	auto town = EncodeCargoTownMonitor(company, cargo, st->town->index);
	Source source{st->town->index, SourceType::Town};
	ClearCargoPickupMonitoring(); ClearCargoDeliveryMonitoring();
	if (args[1] == "monitors-load") {
		GetPickupAmount(town, true); GetDeliveryAmount(town, true);
		AddCargoDelivery(cargo, company, 17, source, st);
		auto file = FileHandle::Open(args[2], "rb"); if (!file.has_value()) FixtureAbort(__LINE__);
		_sl.reader = std::make_unique<ReadBuffer>(std::make_shared<FileReader>(std::move(*file)));
		_sl.action = SLA_LOAD; _sl_version = SAVEGAME_VERSION; _next_offs = 0;
		bool failed = false;
		try { SlLoadChunk(*SlFindChunkHandler(args[3] == "CMDL" ? 'CMDL' : 'CMPU')); } catch (const std::exception &) { failed = true; }
		auto query = args[3] == "CMDL" ? GetDeliveryAmount : GetPickupAmount;
		bool good = failed == (args[4] != "duplicate") && (!failed || _sl.extra_msg == (args[4] == "header" ? "Field type is different than expected" : "Unexpected end of chunk")) && query(town, false) == (args[4] == "header" ? 17 : 0) && query(1, false) == (args[4] == "header" ? 0 : 11) && query(2, false) == 0;
		if (!good) FixtureAbort(__LINE__);
		fmt::print(stderr, "MONITOR load {} {} true\n", args[3], args[4]); std::fflush(stderr); std::_Exit(0);
	}
	if (Industry::GetNumItems() < 2) FixtureAbort(__LINE__);
	auto ii = Industry::Iterate().begin(); Industry *i = *ii; ++ii; Industry *j = *ii;
	Backup<IndustryList> near(st->industries_near); st->industries_near = {{0, i}};
	auto industry = EncodeCargoIndustryMonitor(company, cargo, i->index), absent = EncodeCargoIndustryMonitor(company, cargo, j->index);
	bool good = DecodeMonitorCompany(industry) == company && DecodeMonitorCargoType(industry) == cargo && DecodeMonitorIndustry(industry) == i->index && DecodeMonitorTown(town) == st->town->index;
	good &= GetPickupAmount(town, false) == 0 && GetDeliveryAmount(town, true) == 0 && GetPickupAmount(town, true) == 0;
	GetPickupAmount(industry, true); GetDeliveryAmount(industry, true); GetDeliveryAmount(absent, true);
	AddCargoDelivery(cargo, company, 0, source, st, i->index);
	good &= GetPickupAmount(town, true) == 0 && GetDeliveryAmount(town, true) == 0;
	AddCargoDelivery(cargo, company, 5, source, st, i->index);
	good &= GetPickupAmount(town, true) == 5 && GetPickupAmount(town, true) == 0 && GetDeliveryAmount(town, false) == 5 && GetDeliveryAmount(industry, true) == 5;
	AddCargoDelivery(cargo, company, 7, {i->index, SourceType::Industry}, st, j->index);
	good &= GetPickupAmount(industry, false) == 7 && GetDeliveryAmount(town, false) == 0 && GetDeliveryAmount(absent, false) == 0 && GetDeliveryAmount(industry, false) == 0;
	for (Source src : {Source{Source::Invalid, SourceType::Town}, Source{st->town->index, SourceType::Headquarters}}) {
		GetPickupAmount(town, true); GetDeliveryAmount(town, true); AddCargoDelivery(cargo, company, 9, src, st, i->index);
		good &= GetPickupAmount(town, false) == 0 && GetDeliveryAmount(town, false) == 9 && GetDeliveryAmount(industry, false) == 0;
	}
	for (auto [first, second, expected] : std::initializer_list<std::tuple<uint32_t, uint32_t, int32_t>>{{INT32_MAX, 1, INT32_MAX}, {INT32_MAX, 0x80000000U, -1}, {0x80000000U, UINT32_MAX, INT32_MIN}, {0x80000000U, 1, INT32_MIN + 1}}) {
		GetPickupAmount(industry, true); GetDeliveryAmount(town, true);
		for (uint32_t amount : {first, second}) {
			AddCargoDelivery(cargo, company, amount, {i->index, SourceType::Industry}, st);
			good &= GetDeliveryAmount(town, true) == int32_t(amount);
		}
		good &= GetPickupAmount(industry, false) == expected;
	}
	GetPickupAmount(town, true); GetDeliveryAmount(town, true);
	InitializeEconomy();
	fmt::print(stderr, "MONITOR controls {}\n", good);
	if (!good) FixtureAbort(__LINE__);
	return true;
}

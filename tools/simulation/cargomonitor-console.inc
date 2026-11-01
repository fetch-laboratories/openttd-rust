/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Real facades/chunk handlers; malformed loads terminate before any further game work. */
#include "cargomonitor.h"
#include "saveload-codec-console.hpp"
/* Monitors change only through final deliveries. The station's loaded catchment
 * supplies the nearby industry and the always-accepted passengers. */
struct MonitorSite {
	Station *st = nullptr;
	Industry *near = nullptr; ///< First delivery-list industry accepting #accepted.
	Industry *far = nullptr; ///< An industry outside the delivery list.
	CargoType accepted = INVALID_CARGO; ///< Accepted by #near, not always accepted at #st.
	Vehicle *front = nullptr;
};
static MonitorSite FindMonitorSite(CargoType pax)
{
	MonitorSite site;
	for (Station *st : Station::Iterate()) {
		if (site.st != nullptr || !HasBit(st->always_accepted, pax)) continue;
		for (const auto &entry : st->industries_near) {
			if (site.st != nullptr) break;
			Industry *ind = entry.industry;
			if (ind->exclusive_supplier != INVALID_OWNER) continue;
			for (const auto &a : ind->accepted) {
				if (!IsValidCargoType(a.cargo) || HasBit(st->always_accepted, a.cargo) || a.waiting != 0) continue;
				site.st = st; site.near = ind; site.accepted = a.cargo; break;
			}
		}
	}
	if (site.st == nullptr) FixtureAbort(__LINE__);
	for (Industry *ind : Industry::Iterate()) {
		bool listed = false;
		for (const auto &entry : site.st->industries_near) listed |= entry.industry == ind;
		if (!listed) { site.far = ind; break; }
	}
	for (Vehicle *v : Vehicle::Iterate()) if (v->type == VEH_TRAIN && v->IsPrimaryVehicle() && v->cargo_payment == nullptr) { site.front = v; break; }
	if (site.far == nullptr || site.front == nullptr) FixtureAbort(__LINE__);
	return site;
}
/** Deliver @a repeat final payments of @a count pieces each from one packet at the site's station. */
static void MonitorDeliver(const MonitorSite &site, CargoType cargo, Source src, uint count, uint repeat = 1)
{
	if (!CargoPacket::CanAllocateItem() || !CargoPayment::CanAllocateItem()) FixtureAbort(__LINE__);
	CargoPacket *cp = new CargoPacket(site.st->index, UINT16_MAX, src); cp->UpdateLoadingTile(site.st->xy);
	CargoPayment *payment = new CargoPayment(site.front);
	for (uint n = 0; n < repeat; ++n) payment->PayFinalDelivery(cargo, cp, count, site.st->xy);
	payment->route_profit = payment->visual_profit = payment->visual_transfer = 0; delete payment; delete cp;
}
static bool MonitorScenario(std::span<std::string_view> args)
{
	if (args[1] == "monitors-capture") {
		if (SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) FixtureAbort(__LINE__);
		fmt::print(stderr, "MONITOR captured true\n"); return true;
	}
	const CargoType pax = GetCargoTypeByLabel(CT_PASSENGERS);
	if (!IsValidCargoType(pax)) FixtureAbort(__LINE__);
	MonitorSite site = FindMonitorSite(pax);
	Station *st = site.st;
	Backup<StationID> last(site.front->last_station_visited, st->index);
	CompanyID company = site.front->owner;
	auto town = EncodeCargoTownMonitor(company, pax, st->town->index);
	Source source{st->town->index, SourceType::Town};
	ClearCargoPickupMonitoring(); ClearCargoDeliveryMonitoring();
	if (args[1] == "monitors-load") {
		/* The chunk rows use monitors 1 and 2. */
		if (town == 1 || town == 2) FixtureAbort(__LINE__);
		GetPickupAmount(town, true); GetDeliveryAmount(town, true);
		MonitorDeliver(site, pax, source, 17);
		last.Restore();
		auto file = FileHandle::Open(args[2], "rb"); if (!file.has_value()) FixtureAbort(__LINE__);
#ifdef WITH_RUST
		ConsoleOpenReader(std::make_shared<FileReader>(std::move(*file))); _sl_version = SAVEGAME_VERSION;
#else
		_sl.reader = std::make_unique<ReadBuffer>(std::make_shared<FileReader>(std::move(*file)));
		_sl.action = SLA_LOAD; _sl_version = SAVEGAME_VERSION; _next_offs = 0;
#endif
		bool failed = false;
		try { SlLoadChunk(*SlFindChunkHandler(args[3] == "CMDL" ? 'CMDL' : 'CMPU')); } catch (const std::exception &) { failed = true; }
		auto query = args[3] == "CMDL" ? GetDeliveryAmount : GetPickupAmount;
		bool good = failed == (args[4] != "duplicate") && (!failed || _sl.extra_msg == (args[4] == "header" ? "Field type is different than expected" : "Unexpected end of chunk")) && query(town, false) == (args[4] == "header" ? 17 : 0) && query(1, false) == (args[4] == "header" ? 0 : 11) && query(2, false) == 0;
		if (!good) FixtureAbort(__LINE__);
		fmt::print(stderr, "MONITOR load {} {} true\n", args[3], args[4]); std::fflush(stderr); std::_Exit(0);
	}
	Industry *i = site.near, *j = site.far;
	auto town_cargo = EncodeCargoTownMonitor(company, site.accepted, st->town->index);
	auto industry = EncodeCargoIndustryMonitor(company, site.accepted, i->index), other = EncodeCargoIndustryMonitor(company, site.accepted, j->index);
	bool good = DecodeMonitorCompany(industry) == company && DecodeMonitorCargoType(industry) == site.accepted && DecodeMonitorIndustry(industry) == i->index && DecodeMonitorTown(town) == st->town->index;
	good &= GetPickupAmount(town, false) == 0 && GetDeliveryAmount(town, true) == 0 && GetPickupAmount(town, true) == 0;
	/* Industry cargo reaches the listed industry only and credits the supplier's
	 * pickup; the station's remainder delivery adds zero. */
	GetPickupAmount(other, true); GetPickupAmount(town_cargo, true); GetDeliveryAmount(industry, true); GetDeliveryAmount(other, true); GetDeliveryAmount(town_cargo, true);
	MonitorDeliver(site, site.accepted, {j->index, SourceType::Industry}, 5);
	good &= GetPickupAmount(other, true) == 5 && GetDeliveryAmount(industry, true) == 5 && GetDeliveryAmount(town_cargo, true) == 5 && GetDeliveryAmount(other, true) == 0;
	/* A town source has no industry pickup in the industry delivery. */
	MonitorDeliver(site, site.accepted, source, 7);
	good &= GetPickupAmount(town_cargo, true) == 0 && GetPickupAmount(other, true) == 0 && GetDeliveryAmount(industry, true) == 7 && GetDeliveryAmount(town_cargo, true) == 7 && GetDeliveryAmount(other, false) == 0;
	/* Always-accepted passengers credit the town pickup, unless the source is invalid or not a town. */
	GetPickupAmount(town, true); GetDeliveryAmount(town, true);
	MonitorDeliver(site, pax, source, 9);
	good &= GetPickupAmount(town, true) == 9 && GetDeliveryAmount(town, true) == 9 && GetDeliveryAmount(industry, true) == 0;
	for (Source src : {Source{Source::Invalid, SourceType::Town}, Source{st->town->index, SourceType::Headquarters}}) {
		MonitorDeliver(site, pax, src, 9);
		good &= GetPickupAmount(town, true) == 0 && GetDeliveryAmount(town, true) == 9 && GetDeliveryAmount(industry, true) == 0;
	}
	/* Accumulated full packets saturate both maps at INT32_MAX: 32768 * 65535 + 32768 == INT32_MAX + 1. */
	MonitorDeliver(site, pax, source, UINT16_MAX, 32768);
	MonitorDeliver(site, pax, source, 32768);
	good &= GetPickupAmount(town, false) == INT32_MAX && GetDeliveryAmount(town, false) == INT32_MAX;
	GetPickupAmount(other, false); GetPickupAmount(town_cargo, false); GetDeliveryAmount(industry, false); GetDeliveryAmount(town_cargo, false);
	last.Restore();
	InitializeEconomy();
	fmt::print(stderr, "MONITOR controls {}\n", good);
	if (!good) FixtureAbort(__LINE__);
	return true;
}

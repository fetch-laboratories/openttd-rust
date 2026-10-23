/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/* The road corpus has no catchment containing multiple accepting industries.
 * Compare unchanged delivery/production bodies and their flush loop with Rust;
 * the mock leaves record ordered delivery and production effects. */
#include "core/container_func.hpp"
#include <tuple>

using IndustryID = uint16_t;
using CompanyID = uint8_t;
constexpr CompanyID INVALID_OWNER = 255;
constexpr size_t THIS_MONTH = 0;
constexpr int WC_INDUSTRY_VIEW = 0;
enum class SourceType : uint8_t { Industry };
struct Source { IndustryID id; SourceType type; };
enum class PartOfSubsidy : uint8_t { Source, Destination };
#define OPENTTD_SUBSIDY_FIXTURE_TYPES
#include "rust/subsidy_ffi.h"
#include "rust/cargomonitor_ffi.h"
struct TimerGameEconomy { static constexpr int date = 123; };
enum class IndustryCallbackMask { ProductionCargoArrival, Production256Ticks };
struct IndustryCallbackMasks {
	uint8_t bits;
	bool Test(IndustryCallbackMask mask) const { return (this->bits & (1U << uint8_t(mask))) != 0; }
};
enum class IndustryRandomTrigger { CargoReceived };
enum class IndustryAnimationTrigger { CargoReceived };
struct IndustrySpec {
	IndustryCallbackMasks callback_mask;
	uint16_t input_cargo_multiplier[2][2]{{256, 0}, {128, 0}};
};
struct AcceptedCargo {
	CargoType cargo;
	uint16_t waiting;
	struct History {
		uint32_t accepted{};
		bool operator==(const History &) const = default;
	};
	std::array<History, 1> history{};
	int last_accepted{};
	auto &GetOrCreateHistory() { return this->history; }
	bool operator==(const AcceptedCargo &) const = default;
};
struct ProducedCargo {
	CargoType cargo;
	uint16_t waiting;
	bool operator==(const ProducedCargo &) const = default;
};
struct Industry {
	IndustryID index;
	uint8_t type;
	CompanyID exclusive_supplier{INVALID_OWNER};
	bool refuses{}, was_cargo_delivered{};
	std::array<AcceptedCargo, 2> accepted{{{1, 65530}, {2, 16}}};
	std::array<ProducedCargo, 2> produced{{{3, 65000}, {255, 3}}};
	auto GetCargoAccepted(CargoType cargo) { return std::ranges::find(this->accepted, cargo, &AcceptedCargo::cargo); }
	bool operator==(const Industry &) const = default;
};
using SmallIndustryList = std::vector<Industry *>;
struct IndustryListEntry { Industry *industry; };
struct Station { CompanyID owner{}; std::vector<IndustryListEntry> industries_near; };
struct RustNearbyIndustryCursor { size_t next; };
using DestinationEvent = std::tuple<IndustryID, uint8_t, uint32_t, uint32_t>;
static std::vector<DestinationEvent> destination_events;
static std::array<IndustrySpec, 8> destination_specs;
static std::array<Industry, 8> destination_industries;
static Station destination_station;
static const IndustrySpec *GetIndustrySpec(uint8_t type) { return &destination_specs[type]; }
static bool IndustryTemporarilyRefusesCargo(Industry *i, CargoType) { return i->refuses; }
static bool IsValidCargoType(CargoType cargo) { return cargo != 255; }
static void AddCargoDelivery(CargoType cargo, CompanyID, uint amount, Source, const Station *, IndustryID industry)
{
	destination_events.emplace_back(industry, 5, amount, cargo);
}
static void IndustryProductionCallback(Industry *i, uint8_t) { destination_events.emplace_back(i->index, 1, 0, 0); }
static void SetWindowDirty(int, IndustryID i) { destination_events.emplace_back(i, 2, 0, 0); }
static void TriggerIndustryRandomisation(Industry *i, IndustryRandomTrigger) { destination_events.emplace_back(i->index, 3, 0, 0); }
static void TriggerIndustryAnimation(Industry *i, IndustryAnimationTrigger) { destination_events.emplace_back(i->index, 4, 0, 0); }
#include "cargo-destination-reference.inc"

static int CheckCargoDestinationGap(OpenTTDCargoServices s)
{
	s.station_read = [](uint16_t, uint8_t field, uint8_t) noexcept -> uint32_t { return field == 0 ? destination_station.owner : 0; };
	s.industry_read = [](void *p, uint8_t field, uint32_t slot) noexcept -> uint32_t {
		auto *i = static_cast<Industry *>(p);
		switch (field) {
			case 1: { auto it = i->GetCargoAccepted(slot); return it == i->accepted.end() ? UINT32_MAX : it - i->accepted.begin(); }
			case 2: return i->exclusive_supplier;
			case 3: return i->accepted[slot].waiting;
			case 4: return destination_specs[i->type].callback_mask.bits;
			case 5: return i->accepted.size();
			case 6: return i->produced.size();
			case 7: return i->accepted[slot].cargo;
			case 8: return i->produced[slot].cargo;
			case 9: return i->produced[slot].waiting;
			case 10: return destination_specs[i->type].input_cargo_multiplier[slot >> 16][slot & 65535];
			default: std::abort();
		}
	};
	s.industry_write = [](void *p, uint8_t field, uint32_t slot, uint32_t value) noexcept {
		auto *i = static_cast<Industry *>(p);
		if (field == 0) i->produced[slot].waiting = value; else i->accepted[slot].waiting = value;
	};
	s.refuses = [](void *p, uint8_t) noexcept -> uint8_t { return static_cast<Industry *>(p)->refuses; };
	s.accept = [](void *p, uint32_t slot, uint32_t amount) noexcept {
		auto &a = static_cast<Industry *>(p)->accepted[slot];
		a.waiting += amount;
		a.history[THIS_MONTH].accepted += amount;
		a.last_accepted = TimerGameEconomy::date;
		// The removed monitor callback recorded these acceptance arguments.
		destination_events.emplace_back(static_cast<Industry *>(p)->index, 5, amount, a.cargo);
	};
	s.statistics = [](uint16_t, uint8_t, uint8_t, uint32_t, uint8_t) noexcept {};
	s.subsidies = []() noexcept -> const OpenTTDSubsidyServices * { static const OpenTTDSubsidyServices empty{
		.industry_id = [](void *p) noexcept -> uint16_t { return static_cast<Industry *>(p)->index; },
		.has_source_flag = [](Source, PartOfSubsidy) noexcept { return false; },
		.station = [](uint16_t) noexcept -> const Station * { return &destination_station; },
		.begin_nearby = [](RustNearbyIndustryCursor *c, const Station *) noexcept { c->next = 0; },
		.next_nearby = [](RustNearbyIndustryCursor *c) noexcept -> Industry * { return c->next < destination_station.industries_near.size() ? destination_station.industries_near[c->next++].industry : nullptr; },
	}; return &empty; };
	s.industry_effect = [](void *p, uint8_t op) noexcept {
		auto *i = static_cast<Industry *>(p);
		if (op == 0) i->was_cargo_delivered = true; else destination_events.emplace_back(i->index, op, 0, 0);
	};
	s.vehicle_read = [](void *, uint8_t) noexcept -> uint32_t { return 0; };
	s.feeder = [](const void *, uint32_t) noexcept -> int64_t { return 0; };
	OpenTTDCargoMonitorServices monitors{
		.station_town = [](const Station *) noexcept -> uint16_t { return 0; },
		.begin_nearby = [](RustNearbyIndustryCursor *c, const Station *) noexcept { c->next = 0; },
		.next_nearby = [](RustNearbyIndustryCursor *c) noexcept -> Industry * { return c->next < destination_station.industries_near.size() ? destination_station.industries_near[c->next++].industry : nullptr; },
		.industry_id = [](void *p) noexcept -> uint16_t { return static_cast<Industry *>(p)->index; },
		.check = [](bool ok) noexcept { if (!ok) std::abort(); },
	};
	for (uint8_t first_mask = 0; first_mask < 4; ++first_mask) {
		for (uint8_t second_mask = 0; second_mask < 4; ++second_mask) {
			destination_industries = {{{7, 0}, {8, 1}, {9, 2}, {11, 3}, {30, 4}, {20, 5}, {10, 6}, {40, 7}}};
			destination_industries[1].accepted[0].cargo = 2; // No matching cargo.
			destination_industries[2].refuses = true;
			destination_industries[3].exclusive_supplier = 1;
			destination_industries[6].accepted[0].waiting = 65535; // Included even when accepting zero.
			destination_industries[7].accepted[0].waiting = 0;
			for (auto &is : destination_specs) is.callback_mask.bits = 0;
			destination_specs[4].callback_mask.bits = first_mask;
			destination_specs[5].callback_mask.bits = second_mask;
			destination_station.industries_near.clear();
			for (auto &i : destination_industries) destination_station.industries_near.push_back({&i});
			auto initial = destination_industries;
			destination_events.clear();
			uint accepted = DeliverGoodsToIndustry(&destination_station, 1, 12, 7, 0);
			accepted += DeliverGoodsToIndustry(&destination_station, 1, 3, 7, 0);
			FlushCargoDestinationReference();
			auto expected = destination_industries;
			auto events = destination_events;
			FlushCargoDestinationReference();
			if (destination_events != events) return 1;
			destination_industries = initial;
			destination_events.clear();
			auto *delivery = openttd_rust_cargo_delivery_new();
			auto *payment = openttd_rust_cargo_payment_new(nullptr, 0);
			spec = {Money(6000), {20, 20}, true, false, {false}};
			RustNearbyIndustryCursor cursor, monitor_cursor;
			openttd_rust_monitor_clear_deliveries(255);
			for (auto &i : destination_industries) openttd_rust_monitor_delivery(openttd_rust_monitor_encode_industry(0, 1, i.index, monitors.check), true);
			openttd_rust_cargo_payment_final(payment, delivery, &s, 1, nullptr, 12, 250, 45, 7, &cursor, &monitors, &monitor_cursor);
			openttd_rust_cargo_payment_final(payment, delivery, &s, 1, nullptr, 3, 250, 45, 7, &cursor, &monitors, &monitor_cursor);
			for (size_t n = 0; n < destination_industries.size(); ++n) {
				auto key = openttd_rust_monitor_encode_industry(0, 1, destination_industries[n].index, monitors.check);
				uint32_t expected_amount = expected[n].accepted[0].history[0].accepted - initial[n].accepted[0].history[0].accepted;
				if (uint32_t(openttd_rust_monitor_delivery(key, false)) != expected_amount) return 5;
			}
			if (payment->fields.route_profit != (GetTransportedGoodsIncome(12, 250, 45, 1) + GetTransportedGoodsIncome(3, 250, 45, 1)).base() || accepted != 15) return 2;
			openttd_rust_cargo_delivery_flush(delivery, &s);
			if (destination_industries != expected || destination_events != events) return 3;
			openttd_rust_cargo_delivery_flush(delivery, &s);
			if (destination_events != events) return 4;
			openttd_rust_cargo_payment_destroy(payment, nullptr, 1);
			openttd_rust_cargo_delivery_destroy(delivery);
		}
	}
	std::printf("cargo destination gap: 16 ordered/deduplicated multi-industry flush cases passed\n");
	return 0;
}

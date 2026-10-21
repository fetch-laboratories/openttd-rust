/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_service.cpp Canonical owner lifetime and cross-language layouts. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "mock_environment.h"
#ifdef WITH_RUST
#include "../rust/station_service_ffi.h"
#include "../station_base.h"
#include "../station_func.h"
#include "../station_kdtree.h"
#include "../waypoint_base.h"
#include "../timer/timer_game_tick.h"
#include "../openttd.h"
#include "abi_layout.hpp"
#include "../rust/station_queue_save.hpp"

extern void OnTick_Station();

TEST_CASE("Station service - scalar ABI layouts")
{
	auto Layout = [](const char *name, std::initializer_list<size_t> values) { CHECK(AbiLayoutMismatch(name, values) == -1); };
#include "abi-layout-station_service.inc"

}
struct StationLoadingProbe {
	OpenTTDStationService *owner;
	int vehicle, payment;
	bool frame_live = false, staged = false;
};
static StationLoadingProbe *_station_loading_probe;

TEST_CASE("Station service - direct prepare preserves native frames and same-owner reentry")
{
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> owner(openttd_rust_station_service_new(), openttd_rust_station_service_destroy);
	/* An empty station does not cross back into any native service. */
	openttd_rust_station_load(owner.get(), nullptr, nullptr);
	CHECK(openttd_rust_station_queue_size(owner.get()) == 0);
	StationLoadingProbe probe{owner.get(), 0, 0};
	_station_loading_probe = &probe;
	auto *cargo = openttd_rust_station_service_cargo(owner.get(), 0);
	cargo->status = 2;
	cargo->rating = 1;
	/* Only the services this vehicle reaches are set; the semantic harness
	 * covers their order, this covers the caller frame and owner reentry. */
	OpenTTDStationServices services{};
	services.station = [](uint16_t) noexcept -> OpenTTDLoadingStation { return {reinterpret_cast<OpenTTDStation *>(_station_loading_probe), reinterpret_cast<OpenTTDBaseStation *>(_station_loading_probe), _station_loading_probe->owner}; };
	services.owner = [](OpenTTDBaseStation *) noexcept { return _station_loading_probe->owner; };
	OpenTTDVehicleServices vehicle{};
	vehicle.last_station = [](OpenTTDVehicle *) noexcept -> uint16_t { return 0; };
	OpenTTDStationLoading loading{};
	loading.vehicle = &vehicle;
	loading.cargo_cap = [](OpenTTDVehicle *) noexcept -> uint16_t { return 1; };
	loading.unload_count = [](OpenTTDVehicle *) noexcept -> uint32_t { return 1; };
	loading.unload_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	loading.has_orders = [](OpenTTDVehicle *) noexcept { return false; };
	loading.total_count = [](OpenTTDVehicle *) noexcept -> uint32_t { return 1; };
	loading.set_cargo_unloading = [](OpenTTDVehicle *, bool) noexcept {};
	loading.set_loading_finished = [](OpenTTDVehicle *, bool) noexcept {};
	loading.set_load_ticks = [](OpenTTDVehicle *, uint16_t) noexcept {};
	loading.next_vehicle = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	loading.with_next_stations = [](OpenTTDVehicle *, void *context, OpenTTDStationScope body) noexcept {
		int destination = 42;
		_station_loading_probe->frame_live = true;
		body(context, &destination);
		_station_loading_probe->frame_live = false;
	};
	loading.stage = [](OpenTTDStation *st, OpenTTDVehicle *, uint8_t, void *next, OpenTTDNativeCargoPayment *payment) noexcept {
		CHECK(static_cast<void *>(st) == _station_loading_probe);
		CHECK(_station_loading_probe->frame_live);
		CHECK(*static_cast<int *>(next) == 42);
		CHECK(static_cast<void *>(payment) == &_station_loading_probe->payment);
		CHECK(openttd_rust_station_queue_size(_station_loading_probe->owner) == 1);
		openttd_rust_station_modify_rating(_station_loading_probe->owner, 5);
		CHECK(openttd_rust_station_service_cargo(_station_loading_probe->owner, 0)->rating == 6);
		_station_loading_probe->staged = true;
	};
	loading.create_payment = [](OpenTTDVehicle *) noexcept -> OpenTTDNativeCargoPayment * { return reinterpret_cast<OpenTTDNativeCargoPayment *>(&_station_loading_probe->payment); };
	openttd_rust_station_prepare(reinterpret_cast<OpenTTDVehicle *>(&probe.vehicle), &services, &loading);
	CHECK(probe.staged);
	CHECK_FALSE(probe.frame_live);
	CHECK(cargo->rating == 6);
	CHECK(openttd_rust_station_queue_value(openttd_rust_station_queue_first(owner.get())) == &probe.vehicle);
	_station_loading_probe = nullptr;
}

TEST_CASE("Station service - stable aliases, ordered queue and replacement")
{
	auto *owner = openttd_rust_station_service_new();
	auto *fields = openttd_rust_station_service_fields(owner);
	auto *cargo = openttd_rust_station_service_cargo(owner, 63);
	CHECK(fields->delete_ctr == 0);
	CHECK(fields->time_since_load == 255);
	CHECK(fields->time_since_unload == 255);
	CHECK(fields->last_vehicle_type == 255);
	CHECK(fields->always_accepted == 0);
	CHECK(cargo->rating == 175);
	CHECK(cargo->time_since_pickup == 255);
	CHECK(cargo->last_age == 255);
	cargo->rating = 1;
	fields->delete_ctr = 7;
	int first, second;
	for (size_t i = 0; i < 128; ++i) openttd_rust_station_queue_push(owner, &first);
	openttd_rust_station_queue_push(owner, &second);
	openttd_rust_station_queue_push(owner, &first);
	CHECK(fields == openttd_rust_station_service_fields(owner));
	CHECK(cargo == openttd_rust_station_service_cargo(owner, 63));
	CHECK(cargo->rating == 1);
	CHECK(fields->delete_ctr == 7);
	auto *entry = openttd_rust_station_queue_erase_node(owner, openttd_rust_station_queue_first(owner));
	for (size_t i = 0; i < 127; ++i) entry = openttd_rust_station_queue_next(owner, entry);
	CHECK(openttd_rust_station_queue_value(entry) == &second);
	openttd_rust_station_queue_remove(owner, &first);
	REQUIRE(openttd_rust_station_queue_size(owner) == 1);
	CHECK(openttd_rust_station_queue_value(openttd_rust_station_queue_first(owner)) == &second);
	openttd_rust_station_queue_clear(owner);
	CHECK(openttd_rust_station_queue_size(owner) == 0);
	openttd_rust_station_service_destroy(owner);
	owner = openttd_rust_station_service_new();
	CHECK(openttd_rust_station_service_cargo(owner, 63)->rating == 175);
	CHECK(openttd_rust_station_service_fields(owner)->delete_ctr == 0);
	openttd_rust_station_service_destroy(owner);
}
TEST_CASE("Station service - loading facade keeps list node identity")
{
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> owner(openttd_rust_station_service_new(), openttd_rust_station_service_destroy);
	RustStationLoadingQueue queue(owner.get());
	int first, second, third;
	auto *a = reinterpret_cast<Vehicle *>(&first);
	auto *b = reinterpret_cast<Vehicle *>(&second);
	auto *c = reinterpret_cast<Vehicle *>(&third);
	for (Vehicle *v : {a, b, a, c}) queue.push_back(v);
	auto retained = queue.begin();
	++retained;
	auto duplicate = retained;
	++duplicate;
	CHECK(duplicate != queue.begin());
	CHECK(*duplicate == a);
	CHECK(queue.erase(queue.begin()) == retained);
	CHECK(*retained == b);
	/* Preserve a preincremented iterator while removing its predecessor. */
	auto next = retained++;
	CHECK(*next == b);
	queue.remove(b);
	CHECK(retained == duplicate);
	CHECK(*retained == a);
	queue.push_back(a);
	queue.remove(a);
	REQUIRE(queue.size() == 1);
	CHECK(queue.front() == c);
	CHECK(queue.erase(queue.begin()) == queue.end());
	CHECK(queue.empty());
}

TEST_CASE("Station service - loading save staging retains partial and nested effects")
{
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> owner(openttd_rust_station_service_new(), openttd_rust_station_service_destroy);
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> nested_owner(openttd_rust_station_service_new(), openttd_rust_station_service_destroy);
	RustStationLoadingQueue queue(owner.get()), nested(nested_owner.get());
	int first, second;
	auto *a = reinterpret_cast<Vehicle *>(&first);
	auto *b = reinterpret_cast<Vehicle *>(&second);
	queue.push_back(a);
	queue.push_back(b);
	nested.push_back(b);
	{
		RustStationLoadingQueueSaveScope save(&queue, false);
		auto *list = static_cast<std::list<void *> *>(RustStationLoadingQueueSaveScope::Address(queue));
		CHECK(*list == std::list<void *>{a, b});
		list->clear();
	}
	CHECK(queue.size() == 2);
	/* SlRefList clears storage and appends the next default entry before
	 * reading its reference. A failure therefore retains that null entry. */
	try {
		RustStationLoadingQueueSaveScope load(&queue, true);
		auto *list = static_cast<std::list<void *> *>(RustStationLoadingQueueSaveScope::Address(queue));
		list->clear();
		list->push_back(a);
		list->emplace_back();
		throw 1;
	} catch (int) {}
	CHECK(std::vector<Vehicle *>(queue.begin(), queue.end()) == std::vector<Vehicle *>{a, nullptr});
	/* A later nested error keeps each fixed prefix and restores the outer
	 * descriptor's context before it unwinds. */
	try {
		RustStationLoadingQueueSaveScope fix(&queue, true);
		auto *list = static_cast<std::list<void *> *>(RustStationLoadingQueueSaveScope::Address(queue));
		list->front() = b;
		try {
			RustStationLoadingQueueSaveScope inner(&nested, true);
			auto *inner_list = static_cast<std::list<void *> *>(RustStationLoadingQueueSaveScope::Address(nested));
			inner_list->front() = a;
			CHECK(RustStationLoadingQueueSaveScope::Address(queue) == list);
			throw 2;
		} catch (int) {}
		CHECK(nested.front() == a);
		CHECK(RustStationLoadingQueueSaveScope::Address(queue) == list);
		throw 3;
	} catch (int) {}
	CHECK(std::vector<Vehicle *>(queue.begin(), queue.end()) == std::vector<Vehicle *>{b, nullptr});
}
TEST_CASE("Station service - indexed shells, retirement and pool reuse")
{
	MockEnvironment::Instance();
	REQUIRE(BaseStation::CanAllocateItem());
	auto *st = new Station(TileIndex(1));
	st->owner = OWNER_NONE;
	_station_kdtree.Insert(st->index);
	StationID id = st->index;
	st->goods[63].rating = 1;
	st->goods[63].amount_fract = 255;
	st->delete_ctr = 7;
	const auto old_counter = TimerGameTick::counter;
	const auto old_mode = _game_mode;
	TimerGameTick::counter = (250 - id.base() % 250) % 250;
	_game_mode = GM_NORMAL;
	OnTick_Station(); // Real empty-station expiry uses ordinary destruction.
	CHECK_FALSE(BaseStation::IsValidID(id));
	REQUIRE(BaseStation::CanAllocateItem());
	st = new Station(TileIndex(1));
	CHECK(st->index == id);
	CHECK(st->delete_ctr == 0);
	CHECK(st->goods[63].rating == 175);
	CHECK(st->goods[63].amount_fract == 0);
	CHECK(st->loading_vehicles.empty());
	CHECK(st->time_since_load == 255);
	auto *indexed = new (StationID(17)) Station(TileIndex(2));
	CHECK(indexed->index == StationID(17));
	CHECK(indexed->goods[0].time_since_pickup == 255);
	CHECK(indexed->last_vehicle_type == VEH_INVALID);
	auto *waypoint = new (StationID(19)) Waypoint(TileIndex(3));
	CHECK(waypoint->delete_ctr == 0);
	TimerGameTick::counter = 233; // Past the holes, unused 17 gets its big tick.
	OnTick_Station();
	CHECK(indexed->delete_ctr == 1);
	CHECK(waypoint->delete_ctr == 0);
	indexed->goods[0].last_speed = 255;
	_station_pool.CleanPool(); // Derived cleanup early returns still drop owners.
	RebuildStationKdtree();
	TimerGameTick::counter = old_counter;
	_game_mode = old_mode;
}

struct TickShell {
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> owner{openttd_rust_station_service_new(), openttd_rust_station_service_destroy};
	bool waypoint = false, in_use = true;
	int animations = 0, stale = 0;
	uint8_t &Counter() { return openttd_rust_station_service_fields(this->owner.get())->delete_ctr; }
};
static std::vector<TickShell *> _tick_slots;
static TickShell *_tick_grown;
static uint64_t _tick_counter;
static TickShell *Shell(OpenTTDBaseStation *handle) { return reinterpret_cast<TickShell *>(handle); }

TEST_CASE("Station service - tick slots: holes, growth, deletion before animation and fresh predicates")
{
	/* Unset services abort, so only the reads this tick sequence makes are allowed. */
	OpenTTDStationServices w{};
	OpenTTDStationLinks l{};
	w.environment.editor = []() noexcept { return false; };
	w.tick_slot = [](uint16_t id) noexcept -> OpenTTDStationTickSlot {
		TickShell *s = id < _tick_slots.size() ? _tick_slots[id] : nullptr;
		if (s == nullptr) return {_tick_slots.size(), nullptr, nullptr, false, false};
		return {_tick_slots.size(), reinterpret_cast<OpenTTDBaseStation *>(s), s->owner.get(), s->waypoint, !s->waypoint && s->in_use};
	};
	w.tick_phase = [](OpenTTDBaseStation *h) noexcept -> OpenTTDStationTickPhase { return {_tick_counter, !Shell(h)->waypoint}; };
	w.in_use = [](OpenTTDBaseStation *h) noexcept { return Shell(h)->in_use; };
	w.is_station = [](OpenTTDBaseStation *h) noexcept { return !Shell(h)->waypoint; };
	w.facilities = [](OpenTTDBaseStation *h) noexcept -> uint8_t { return Shell(h)->waypoint ? 128 : 0; };
	w.owner = [](OpenTTDBaseStation *h) noexcept { return Shell(h)->owner.get(); };
	w.cargo_valid = [](uint8_t) noexcept { _tick_slots[2]->waypoint = true; return false; }; // Rating converts slot 2.
	w.rating_redraw = [](OpenTTDBaseStation *, bool) noexcept {};
	w.delete_station = [](OpenTTDBaseStation *) noexcept { _tick_slots[0] = nullptr; };
	w.acceptance_animation = [](OpenTTDBaseStation *h) noexcept { Shell(h)->animations++; };
	l.automatic_distribution = [](uint8_t) noexcept { return false; };
	l.graph = [](OpenTTDBaseStation *h, uint8_t) noexcept -> OpenTTDLinkGraph * {
		/* Stale work retires its station and allocates a higher slot. */
		Shell(h)->stale++;
		Shell(h)->in_use = false;
		_tick_slots.resize(251);
		_tick_slots[250] = _tick_grown;
		return nullptr;
	};
	TickShell expiring, grown, rated, reused;
	_tick_grown = &grown;
	grown.waypoint = true;
	expiring.Counter() = 7;
	rated.Counter() = 183;
	_tick_slots = {&expiring, nullptr, &rated};
	_tick_counter = 0;
	openttd_rust_station_tick(&w, &l);
	CHECK(expiring.stale == 64);
	CHECK(_tick_slots[0] == nullptr);
	CHECK(expiring.animations == 0);
	CHECK(grown.animations == 1);
	CHECK(grown.Counter() == 0);
	CHECK(rated.Counter() == 184);
	_tick_slots[0] = &reused;
	_tick_counter = 502; // Slot 2 reaches its link-graph phase after rating.
	openttd_rust_station_tick(&w, &l);
	CHECK(reused.Counter() == 1);
	CHECK(rated.Counter() == 0);
	CHECK(rated.waypoint);
	CHECK(rated.stale == 0);
	_tick_slots.clear();
}
#endif

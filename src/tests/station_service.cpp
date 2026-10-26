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
#include "../rust/station_resolver_ffi.h"
#include "../rust/roadstop_resolver_ffi.h"
#include "../station_base.h"
#include "../linkgraph/linkgraph.h"
#include "../station_func.h"
#include "../station_kdtree.h"
#include "../waypoint_base.h"
#include "../timer/timer_game_tick.h"
#include "../openttd.h"
#include "abi_layout.hpp"
#include "../rust/station_queue_save.hpp"
#include "../rust/services_ffi.h"

extern void OnTick_Station();

TEST_CASE("Shared station traversal - live filtered pool mutation and reentry")
{
	MockEnvironment::Instance();
	struct Restore { ~Restore() { _station_pool.CleanPool(); RebuildStationKdtree(); } } restore;
	const auto Trace = [](bool shared) {
		_station_pool.CleanPool();
		RebuildStationKdtree();
		const auto Create = [](uint32_t id) {
			REQUIRE(BaseStation::CanAllocateItem());
			auto *st = new (StationID(id)) Station(TileIndex(1));
			st->owner = OWNER_NONE;
			_station_kdtree.Insert(st->index);
		};
		for (uint32_t id : {0, 2, 4, 5}) Create(id);
		new (StationID(1)) Waypoint(TileIndex(2)); // Shares the pool, excluded by Iterate.
		std::vector<uint16_t> visits;
		uint32_t growth = UINT32_MAX;
		auto Visit = [&](uint16_t id) {
			visits.push_back(id);
			if (id == 0) delete Station::Get(StationID(id));
			if (id != 2) return;
			std::vector<uint16_t> nested;
			GetRustSharedServices().for_each_station(&nested, [](void *context, const OpenTTDStation *, OpenTTDStationService *, uint16_t next) {
				static_cast<std::vector<uint16_t> *>(context)->push_back(next);
			});
			CHECK(nested == std::vector<uint16_t>{2, 4, 5});
			Create(0); // Behind the live cursor: waits for another traversal.
			Create(3); // Ahead in initialized storage: visited now.
			delete Station::Get(StationID(4));
			Create(4); // Reused ahead: visited now.
			delete Station::Get(StationID(5)); // A new hole must be skipped.
			growth = static_cast<uint32_t>(_station_pool.data.capacity());
			Create(growth); // Relocates the pool's slot array during the callback.
		};
		if (shared) {
			GetRustSharedServices().for_each_station(&Visit, [](void *context, const OpenTTDStation *shell, OpenTTDStationService *owner, uint16_t id) {
				CHECK(shell == reinterpret_cast<const OpenTTDStation *>(Station::Get(StationID(id))));
				CHECK(owner == Station::Get(StationID(id))->rust_service.get());
				(*static_cast<decltype(Visit) *>(context))(id);
			});
		} else {
			for (Station *st : Station::Iterate()) Visit(st->index.base());
		}
		CHECK(visits == std::vector<uint16_t>{0, 2, 3, 4, static_cast<uint16_t>(growth)});
		return visits;
	};
	const auto source = Trace(false);
	CHECK(Trace(true) == source);
}

TEST_CASE("Station service - scalar ABI layouts")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-station_service.inc"

}
TEST_CASE("Station service - goods state conversion reads canonical status")
{
	OpenTTDStationCargo cargo{};
	GoodsEntry goods(&cargo);
	for (uint16_t status = 0; status <= UINT8_MAX; ++status) {
		goods.status = GoodsEntry::States(static_cast<uint8_t>(status));
		uint8_t expected = 0;
		if (goods.status.Test(GoodsEntry::State::EverAccepted))    SetBit(expected, 0);
		if (goods.status.Test(GoodsEntry::State::LastMonth))       SetBit(expected, 1);
		if (goods.status.Test(GoodsEntry::State::CurrentMonth))    SetBit(expected, 2);
		if (goods.status.Test(GoodsEntry::State::AcceptedBigtick)) SetBit(expected, 3);
		CHECK(cargo.status == status);
		CHECK(goods.ConvertState() == expected);
	}
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
	openttd_rust_station_load(owner.get(), nullptr, nullptr, nullptr, nullptr, nullptr);
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
	indexed->goods[0].last_speed = 255;
	_station_pool.CleanPool(); // Derived cleanup early returns still drop owners.
	RebuildStationKdtree();
	TimerGameTick::counter = old_counter;
	_game_mode = old_mode;
}
#endif

#ifdef WITH_RUST
#include "../linkgraph/linkgraphjob.h"
TEST_CASE("Link graph - canonical scalars and independent job snapshot lifetime")
{
	LinkGraph graph(0); graph.index = LinkGraphID::Invalid(); graph.Init(1);
	{
		auto fields = openttd_rust_live_node_fields(graph.RustState(), 0);
		*fields.supply = 0xFEDCBA98; *fields.demand = 17; *fields.station = 42; *fields.xy = 1234; *fields.last_update = -1;
		CHECK(fields.supply == openttd_rust_live_node_fields(graph.RustState(), 0).supply);
		auto &edge = graph[0].AppendEdge(); edge.dest_node = 0; edge.capacity = 65536; edge.travel_time_sum = UINT64_C(0xFEDCBA9876543210);
	}
	{
		LinkGraphJob job(graph); job.Init();
		CHECK(job[0].Station() == StationID(42));
		CHECK(job.Graph().RustState() != graph.RustState());
		CHECK(job.Graph()[0].Supply() == 0xFEDCBA98);
		CHECK(job.Graph()[0][0].travel_time_sum == UINT64_C(0xFEDCBA9876543210));
		CHECK(job.Graph()[0].Edges().data() != graph[0].Edges().data());
		graph[0].UpdateSupply(1);
		CHECK(job.Graph()[0].Supply() == 0xFEDCBA98);
	}
	CHECK(graph[0].Supply() == 0xFEDCBA99);
	CHECK(graph[0].Station() == StationID(42));
	CHECK(GetRustLiveGraphServices().access(&graph).owner == graph.RustState());
}
#endif

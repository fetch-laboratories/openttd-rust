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
#include "../../tools/simulation/session-fixture.hpp"
#include "../rust/station_service_ffi.h"
#include "../engine_type.h"
#include "../rust/vehicle_visits_ffi.h"
#include "../rust/vehicle_record_ffi.h"
#include "../rust/station_association_ffi.h"
#include "../rust/station_definitions_ffi.h"
#include "../rust/newgrf_loading_ffi.h"
#include "../table/strings.h"
#include "../rust/station_resolver_ffi.h"
#include "../rust/roadstop_resolver_ffi.h"
#include "../station_base.h"
#include "../newgrf_station.h"
#include "../newgrf_roadstop.h"
#include "../newgrf_airport.h"
#include "../newgrf_object.h"
#include "../linkgraph/linkgraph.h"
#include "../station_func.h"
#include "../station_kdtree.h"
#include "../waypoint_base.h"
#include "../timer/timer_game_tick.h"
#include "../openttd.h"
#include <thread>
#include "abi_layout.hpp"
#include "../rust/station_queue_save.hpp"
#include "../rust/services_ffi.h"
#include "station_spatial_fixture.hpp"
#include "../core/kdtree.hpp"
extern "C" void openttd_test_station_prepare(OpenTTDVehicle *, const OpenTTDStationServices *, const OpenTTDStationLoading *);



TEST_CASE("Station tree - unsorted native STL traversal, strict splits and mutation timing")
{
	struct XY { uint16_t operator()(StationID id, int dim) const { return dim == 0 ? id.base() % 3 : id.base() % 5; } };
	Kdtree<StationID, XY, uint16_t, int> tree;
	std::vector<std::pair<uint32_t, uint16_t>> expected, actual;
	auto observe = [&](uint32_t phase) {
		tree.FindContained(0, 0, 10, 10, [&](StationID id) { expected.emplace_back(phase, id.base()); });
		tree.FindContained(1, 1, 2, 4, [&](StationID id) { expected.emplace_back(phase + 100, id.base()); });
		if (tree.Count() != 0) expected.emplace_back(phase + 200, tree.FindNearest(1, 2).base());
	};
	std::vector<StationID> ids; for (uint16_t i = 0; i < 20; ++i) ids.emplace_back(i);
	tree.Build(ids.begin(), ids.end()); observe(0);
	auto root = expected[0].second, interior = expected[1].second;
	tree.Remove(StationID{root}); observe(1); tree.Remove(StationID{interior}); observe(2);
	for (uint16_t i = 20; i < 40; ++i) { tree.Insert(StationID{i}); observe(i); }
	tree.Remove(StationID{20}); observe(40); tree.Remove(StationID{30}); observe(41); tree.Remove(StationID{39}); observe(42);
	ids.clear(); tree.Build(ids.begin(), ids.end()); observe(43); tree.Insert(StationID{12}); observe(44);
	openttd_test_station_tree_sequence(&GetRustSharedServices(), &actual, [](void *context, uint32_t phase, uint16_t id) {
		static_cast<decltype(actual) *>(context)->emplace_back(phase, id);
	});
	REQUIRE(actual == expected);
}

TEST_CASE("Shared station traversal - live filtered pool mutation and reentry")
{
	MockEnvironment::Instance();
	struct Restore { ~Restore() { _station_pool.CleanPool(); openttd_rust_station_rebuild_index(&GetRustSharedServices()); } } restore;
	const auto Trace = [](bool shared) {
		_station_pool.CleanPool();
		openttd_rust_station_rebuild_index(&GetRustSharedServices());
		const auto Create = [](uint32_t id) {
			REQUIRE(BaseStation::CanAllocateItem());
			auto *st = new (StationID(id)) Station(TileIndex(1));
			st->owner = OWNER_NONE;
			openttd_test_station_insert(&GetRustSharedServices(), st);
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

TEST_CASE("NewGRF station classes - canonical order, reassignment and first membership")
{
	StationClass::Reset();
	std::thread writer([] { StationClass::Allocate('THRD'); });
	writer.join();
	bool observed = false;
	std::thread reader([&] { observed = StationClass::GetClassCount() == 3 && StationClass::Allocate('THRD') == static_cast<StationClassID>(2); });
	reader.join();
	CHECK(observed);
	StationClass::Reset();
	CHECK(StationClass::GetClassCount() == 2);
	CHECK(StationClass::GetUIClassCount() == 2);
	CHECK(StationClass::Get(STAT_CLASS_DFLT)->GetSpec(0) == nullptr);
	CHECK(StationClass::Get(STAT_CLASS_WAYP)->GetSpecCount() == 1);
	const auto first = StationClass::Allocate('TST1');
	const auto second = StationClass::Allocate('TST2');
	CHECK(StationClass::Allocate('TST1') == first);
	StationSpec a{}, b{};
	a.class_index = first;
	a.grf_prop.grfid = 123;
	a.grf_prop.local_id = 7;
	b = a;
	StationClass::Assign(&a);
	StationClass::Assign(&b);
	CHECK(a.index == 0);
	CHECK(b.index == 1);
	CHECK(StationClass::GetByGrf(123, 7) == &a);
	a.class_index = second;
	StationClass::Assign(&a);
	CHECK(StationClass::GetByGrf(123, 7) == &a);
	CHECK(StationClass::Get(first)->GetSpecCount() == 2);
	CHECK(StationClass::Get(second)->GetUISpecCount() == 1);
	CHECK(StationClass::Get(second)->GetSpec(1) == nullptr);
	CHECK(StationClass::Get(first)->Specs()[1] == &b);
	CHECK(StationClass::Classes()[2].global_id == 'TST1');
	CHECK(IsWaypointClass(*StationClass::Get(StationClass::Allocate(0xFF000001))));
	StationClass::Reset();
}

template <typename Class, typename Services, typename Reset>
static void CheckClassDefaults(const Services *services, Reset reset, std::initializer_list<std::pair<uint32_t, StringID>> expected, bool insert_empty)
{
	Class::Allocate('TST1');
	reset(services);
	CHECK(Class::GetClassCount() == expected.size());
	CHECK(Class::GetUIClassCount() == (insert_empty ? expected.size() : 0));
	size_t index = 0;
	for (const auto &[label, name] : expected) {
		const auto *cls = Class::Get(static_cast<typename Class::index_type>(index++));
		CHECK(cls->global_id == label);
		CHECK(cls->name == name);
		CHECK(cls->GetSpecCount() == (insert_empty ? 1 : 0));
		CHECK(cls->GetSpec(0) == nullptr);
	}
}

TEST_CASE("NewGRF classes - direct loader resets preserve original defaults")
{
	const auto &loading = GetRustGrfLoadingServices();
	StationClass::Reset();
	RoadStopClass::Reset();
	AirportClass::Reset();
	ObjectClass::Reset();
	CheckClassDefaults<StationClass>(loading.station_classes, openttd_rust_station_class_reset,
		{{STATION_CLASS_LABEL_DEFAULT, STR_STATION_CLASS_DFLT}, {STATION_CLASS_LABEL_WAYPOINT, STR_STATION_CLASS_WAYP}}, true);
	CheckClassDefaults<RoadStopClass>(loading.roadstop_classes, openttd_rust_roadstop_class_reset,
		{{ROADSTOP_CLASS_LABEL_DEFAULT, STR_STATION_CLASS_DFLT}, {ROADSTOP_CLASS_LABEL_WAYPOINT, STR_STATION_CLASS_WAYP}}, true);
	CheckClassDefaults<AirportClass>(loading.airport_classes, openttd_rust_airport_class_reset,
		{{'SMAL', STR_AIRPORT_CLASS_SMALL}, {'LARG', STR_AIRPORT_CLASS_LARGE}, {'HUB_', STR_AIRPORT_CLASS_HUB}, {'HELI', STR_AIRPORT_CLASS_HELIPORTS}}, false);
	CheckClassDefaults<ObjectClass>(loading.object_classes, openttd_rust_object_class_reset,
		{{'LTHS', STR_OBJECT_CLASS_LTHS}, {'TRNS', STR_OBJECT_CLASS_TRNS}}, false);
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
static OpenTTDCargoList *_station_loading_cargo;

TEST_CASE("Station service - direct prepare preserves native frames and same-owner reentry")
{
	std::unique_ptr<OpenTTDStationService, decltype(&openttd_rust_station_service_destroy)> owner(openttd_rust_station_service_new(), openttd_rust_station_service_destroy);
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
	OpenTTDStationLoading loading{};
	loading.vehicle = &vehicle;
	/* The Rust-owned list holds one packet to unload; the vehicle has no order list. */
	OpenTTDCargoListFields counts{.cargo_periods_in_transit = 0, .feeder_share = 0, .count = 1, .reserved_count = 0, .action_counts = {0, 1, 0, 0}};
	_station_loading_cargo = openttd_rust_cargo_list_new(true);
	openttd_rust_cargo_list_import(_station_loading_cargo, &counts);
	loading.cargo_capacity = [](OpenTTDVehicle *) noexcept { return OpenTTDCargoCapacity{_station_loading_cargo, 0}; };
	loading.unload_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	loading.set_cargo_unloading = [](OpenTTDVehicle *, bool) noexcept {};
	loading.set_loading_finished = [](OpenTTDVehicle *, bool) noexcept {};
	loading.set_load_ticks = [](OpenTTDVehicle *, uint16_t) noexcept {};
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
		openttd_test_station_modify_rating(_station_loading_probe->owner, 5);
		CHECK(openttd_rust_station_service_cargo(_station_loading_probe->owner, 0)->rating == 6);
		_station_loading_probe->staged = true;
	};
	loading.create_payment = [](OpenTTDVehicle *) noexcept -> OpenTTDNativeCargoPayment * { return reinterpret_cast<OpenTTDNativeCargoPayment *>(&_station_loading_probe->payment); };
	OpenTTDVehicle record{};
	OpenTTDVehicleOrderState orders{};
	record.shell = reinterpret_cast<OpenTTDVehicleShell *>(&probe.vehicle);
	record.orders = &orders;
	record.cargo_cap = 1;
	openttd_test_station_prepare(&record, &services, &loading);
	openttd_rust_cargo_list_destroy(_station_loading_cargo, CargoStorageServices());
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
	openttd_test_station_insert(&GetRustSharedServices(), st);
	StationID id = st->index;
	st->goods[63].rating = 1;
	st->goods[63].amount_fract = 255;
	st->delete_ctr = 7;
	const auto old_counter = TimerGameTick::counter;
	const auto old_mode = _game_mode;
	TimerGameTick::counter = (250 - id.base() % 250) % 250;
	_game_mode = GM_NORMAL;
	FixtureStationTick(GetRustStationServices(), GetRustStationLinks(), GetRustVehicleVisits()); // Empty-station expiry uses ordinary destruction.
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
	openttd_rust_station_rebuild_index(&GetRustSharedServices());
	TimerGameTick::counter = old_counter;
	_game_mode = old_mode;
}

#include "../linkgraph/linkgraphjob.h"
#include "../linkgraph/linkgraphschedule.h"
TEST_CASE("Link graph - canonical scalars and independent job snapshot lifetime")
{
	auto *graph = openttd_rust_live_graph_new(0);
	openttd_rust_live_graph_init(graph, 1);
	{
		auto *fields = &graph->nodes[0];
		fields->supply = 0xFEDCBA98; fields->demand = 17; fields->station = 42; fields->xy = 1234; fields->last_update = -1;
		auto &edge = *openttd_rust_live_node_append_edge(graph, 0); edge.dest_node = 0; edge.capacity = 65536; edge.travel_time_sum = UINT64_C(0xFEDCBA9876543210);
	}
	{
		const auto *settings = reinterpret_cast<const OpenTTDLinkGraphJobSettings *>(&_settings_game.linkgraph);
		auto *job = openttd_rust_linkgraph_job_new(graph, 0, settings);
		CHECK(job->graph->nodes[0].station == 42);
		CHECK(job->graph != graph);
		CHECK(job->graph->shell == nullptr); // Private snapshots never bind to a live pool.
		CHECK(job->graph->nodes[0].supply == 0xFEDCBA98);
		CHECK(job->graph->nodes[0].edges[0].travel_time_sum == UINT64_C(0xFEDCBA9876543210));
		CHECK(job->graph->nodes[0].edges.items != graph->nodes[0].edges.items);
		openttd_rust_live_node_update_supply(graph, 0, 1);
		CHECK(job->graph->nodes[0].supply == 0xFEDCBA98);
		openttd_rust_linkgraph_job_delete(job);
	}
	CHECK(graph->nodes[0].supply == 0xFEDCBA99);
	CHECK(graph->nodes[0].station == 42);
	openttd_rust_live_graph_delete(graph);
}

TEST_CASE("Link graph pools - canonical slots retire before free")
{
	MockEnvironment::Instance();
	LinkGraphSchedule::Clear();
	_link_graph_job_pool.CleanPool();
	_link_graph_pool.CleanPool();
	struct Cleanup {
		~Cleanup() { LinkGraphSchedule::Clear(); _link_graph_job_pool.CleanPool(); _link_graph_pool.CleanPool(); }
	} cleanup;
	const auto *graphs = _link_graph_pool.descriptor;
	const auto *jobs = _link_graph_job_pool.descriptor;
	REQUIRE(LinkGraph::CanAllocateItem());
	auto *sibling = new LinkGraph(0);
	const auto graph_id = sibling->index;
	auto *sibling_record = &sibling->Record();
	REQUIRE(LinkGraphJob::CanAllocateItem());
	auto *sibling_job = new LinkGraphJob();
	const auto job_id = sibling_job->index;
	auto *job_record = &sibling_job->Record();
	auto isolated = [&] {
		CHECK(graphs->slots[graph_id.base()].shell == sibling);
		CHECK(graphs->slots[graph_id.base()].record == sibling_record);
		CHECK(jobs->slots[job_id.base()].shell == sibling_job);
		CHECK(jobs->slots[job_id.base()].record == job_record);
		CHECK(graphs->items == 1);
		CHECK(jobs->items == 1);
	};
	isolated();

	const LinkGraphID indexed_graph_id{37};
	auto *indexed_graph = new (indexed_graph_id) LinkGraph(1);
	CHECK(graphs == _link_graph_pool.descriptor); // Descriptor survives slot growth.
	CHECK(graphs->slots[37].shell == indexed_graph);
	CHECK(graphs->slots[37].record == &indexed_graph->Record());
	indexed_graph->~LinkGraph(); // Observe the real destructor before actual sized delete.
	CHECK(graphs->slots[37].shell == indexed_graph);
	CHECK(graphs->slots[37].record == nullptr);
	LinkGraph::operator delete(indexed_graph, sizeof(LinkGraph));
	CHECK(graphs->slots[37].shell == nullptr);
	CHECK(graphs->first_unused == 38);
	const LinkGraphJobID indexed_job_id{37};
	auto *indexed_job = new (indexed_job_id) LinkGraphJob();
	CHECK(jobs == _link_graph_job_pool.descriptor);
	CHECK(jobs->slots[37].record == &indexed_job->Record());
	indexed_job->~LinkGraphJob();
	CHECK(jobs->slots[37].shell == indexed_job);
	CHECK(jobs->slots[37].record == nullptr);
	LinkGraphJob::operator delete(indexed_job, sizeof(LinkGraphJob));
	CHECK(jobs->slots[37].shell == nullptr);
	CHECK(jobs->first_unused == 38);
	isolated();

	REQUIRE(LinkGraph::CanAllocateItem());
	auto *normal_graph = new LinkGraph(2);
	CHECK(normal_graph->index == LinkGraphID{1}); // Indexed allocation did not advance first_free.
	delete normal_graph;
	CHECK(graphs->slots[1].shell == nullptr);
	CHECK(graphs->slots[1].record == nullptr);
	REQUIRE(LinkGraphJob::CanAllocateItem());
	auto *normal_job = new LinkGraphJob();
	CHECK(normal_job->index == LinkGraphJobID{1});
	delete normal_job;
	CHECK(jobs->slots[1].shell == nullptr);
	CHECK(jobs->slots[1].record == nullptr);
	isolated();

	auto *reused_graph = new (indexed_graph_id) LinkGraph(3);
	CHECK(reused_graph->Record().cargo == 3);
	CHECK(graphs->slots[37].record == &reused_graph->Record());
	auto *reused_job = new (indexed_job_id) LinkGraphJob();
	CHECK(jobs->slots[37].record == &reused_job->Record());
	reused_job->Record().aborted.store(true, std::memory_order_release);
	_link_graph_job_pool.CleanPool(); // Aborted jobs join/retire without normal publication.
	CHECK(jobs->slots.empty());
	CHECK(jobs->items == 0);
	CHECK(!jobs->cleaning);
	_link_graph_pool.CleanPool();
	CHECK(graphs->slots.empty());
	CHECK(graphs->items == 0);
	CHECK(!graphs->cleaning);
	REQUIRE(LinkGraph::CanAllocateItem());
	auto *after_clean = new LinkGraph(4);
	CHECK(after_clean->index == LinkGraphID{0});
	CHECK(after_clean->Record().cargo == 4);
	CHECK(after_clean->Record().nodes.empty());
	CHECK(graphs->slots[0].record == &after_clean->Record());
	REQUIRE(LinkGraphJob::CanAllocateItem());
	auto *job_after_clean = new LinkGraphJob();
	CHECK(job_after_clean->index == LinkGraphJobID{0});
	CHECK(!job_after_clean->Record().aborted.load(std::memory_order_acquire));
	CHECK(!job_after_clean->Record().completed.load(std::memory_order_acquire));
	CHECK(jobs->slots[0].record == &job_after_clean->Record());
}

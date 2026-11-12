#pragma once
#include "rust/linkgraph_schedule_ffi.h"
#include "rust/newgrf_vehicle_ffi.h"

static void LinkGraphJob()
{
	static const OpenTTDVehicleResolverServices resolver{
		.vehicle = nullptr,
		.shared = nullptr,
		.capabilities = nullptr,
		.cargo_classes = [](uint8_t cargo) noexcept -> uint16_t { CHECK(cargo == 0); return 1; },
	};
	static const OpenTTDVehicleServices vehicle{
		.assertions = false,
		.newgrf = []() noexcept { return &resolver; },
		.maintenance = nullptr,
	};
	static const OpenTTDSharedServices shared{
		.clear_square_services = {},
		.context = nullptr,
		.map_max_x = []() noexcept -> uint32_t { return 127; },
		.map_max_y = []() noexcept -> uint32_t { return 127; },
		.snow_line_support = {},
	};
	static const OpenTTDScheduleServices services{
		.allocation_failure = nullptr,
		.assert_graph = nullptr,
		.begin_measurement = nullptr,
		.can_allocate = nullptr,
		.destroy_job = nullptr,
		.end_measurement = nullptr,
		.graph_pool = nullptr,
		.job_pool = nullptr,
		.new_job = nullptr,
		.shared = &shared,
		.start_thread = nullptr,
		.vehicle = &vehicle,
	};
	auto *graph = openttd_rust_live_graph_new(0);
	graph->last_compression = 0;
	openttd_rust_live_graph_init(graph, 2);
	graph->nodes[0].supply = 10; graph->nodes[0].station = 12; graph->nodes[0].xy = 129;
	graph->nodes[1].demand = 1; graph->nodes[1].station = 24; graph->nodes[1].xy = 132;
	auto *edge = openttd_rust_live_node_append_edge(graph, 0);
	edge->capacity = 20; edge->dest_node = 1;
	OpenTTDLinkGraphJobSettings settings{60, 4, 1, 1, 1, 1, 2, 100, 100, 80};
	auto *job = openttd_rust_linkgraph_job_new(graph, 42, &settings);
	job->join_date = 59;
	openttd_rust_schedule_run(&services, job);
	CHECK(job->completed.load(std::memory_order_acquire) && job->nodes.size() == 2);
	for (const auto &node : job->nodes) {
		auto origin = openttd_rust_flow_map_at(node.flows, 12, 1);
		CHECK(origin.found != 0);
		auto share = openttd_rust_flow_share(origin.flow, 0, 0);
		CHECK(share.found != 0 && share.station == 24 && share.cumulative == 10);
		CHECK(openttd_rust_flow_read(origin.flow, 1, 0) == 10);
	}
	CHECK(job->edge_flows[0] == 10);
	openttd_rust_linkgraph_job_delete(job);
	job = openttd_rust_linkgraph_job_new(graph, 42, &settings);
	job->aborted.store(true, std::memory_order_release);
	openttd_rust_schedule_run(&services, job);
	CHECK(!job->completed.load(std::memory_order_acquire) && job->nodes.empty());
	openttd_rust_linkgraph_job_delete(job);
	openttd_rust_live_graph_delete(graph);
	std::printf("linkgraph private job snapshot/results and pre-start cancellation passed\n");
}
ABI_PROBE(LinkGraphJob);

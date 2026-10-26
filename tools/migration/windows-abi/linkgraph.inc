#pragma once
#include "rust/linkgraph_ffi.h"

static uint8_t OPENTTD_RUST_CALL LinkGraphAbort(const void *context) noexcept
{
	auto *state = static_cast<const std::array<uint32_t, 2> *>(context);
	return (*state)[0];
}

static void LinkGraphJob()
{
	std::array<OpenTTDLinkGraphNode, 2> nodes{{{10, 0, 12, 1, 1, 0, 1}, {0, 1, 24, 4, 1, 1, 0}}};
	OpenTTDLinkGraphEdge edge{20, 0, 1};
	OpenTTDLinkGraphSettings settings{2, 100, 100, 80, 1, 1, 127, 127, 30};
	std::array<uint32_t, 2> context{0, 0};
	auto *owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	size_t count = 0;
	const auto *shares = openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 2 && shares[0].node == 0 && shares[1].node == 1);
	CHECK(shares[0].origin == 12 && shares[0].via == 24 && shares[0].cumulative == 10 && shares[0].unrestricted == 10);
	CHECK(shares[1].origin == 12 && shares[1].via == 24 && shares[1].cumulative == 10 && shares[1].unrestricted == 10);
	CHECK(openttd_rust_linkgraph_edges(owner)[0] == 10);
	openttd_rust_linkgraph_destroy(owner);
	context[0] = 1;
	owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 0 && openttd_rust_linkgraph_edges(owner)[0] == 0);
	openttd_rust_linkgraph_destroy(owner);
	std::printf("linkgraph owned snapshot/result and cdecl abort callback passed\n");
}
ABI_PROBE(LinkGraphJob);

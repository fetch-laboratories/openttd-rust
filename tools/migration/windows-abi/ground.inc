#pragma once
#include "rust/ground_ffi.h"
#include "tests/ground_protocol.hpp"

static void GroundProtocol()
{
	CHECK(GroundProbe::Run());
}
ABI_PROBE(GroundProtocol);

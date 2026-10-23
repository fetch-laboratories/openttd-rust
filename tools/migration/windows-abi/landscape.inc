#pragma once
#include "rust/landscape_loop_ffi.h"
#include "tests/landscape_protocol.hpp"

static void LandscapeProtocol()
{
	LandscapeProbe::Run([](bool result) { CHECK(result); });
}
ABI_PROBE(LandscapeProtocol);

#pragma once
#include "rust/water_regions_ffi.h"
#include "tests/water_regions_protocol.hpp"

static void WaterProtocol()
{
	WaterProbe::Run([](bool result) { CHECK(result); });
}
ABI_PROBE(WaterProtocol);

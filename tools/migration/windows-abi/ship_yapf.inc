#pragma once
#include "rust/ship_yapf_ffi.h"
#include "tests/ship_yapf_protocol.hpp"

static void ShipYapfProtocol()
{
	ShipYapfProbe::Run([](bool result) { CHECK(result); });
}
ABI_PROBE(ShipYapfProtocol);

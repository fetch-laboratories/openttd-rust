#pragma once
#include "rust/effect_ffi.h"
#include "tests/effect_protocol.hpp"

static void EffectProtocol()
{
	CheckEffectProtocol([](bool result) { CHECK(result); });
}
ABI_PROBE(EffectProtocol);

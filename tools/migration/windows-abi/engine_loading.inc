#pragma once
#include "rust/engine_loading_ffi.h"
#include "newgrf/newgrf_internal_vehicle.h"
static void EngineLoadingLayouts()
{
#include "abi-layout-engine_loading.inc"
}
ABI_PROBE(EngineLoadingLayouts);

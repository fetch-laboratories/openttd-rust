/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#pragma once
#include "rust/pool_ffi.h"
static void PoolLayouts()
{
#include "abi-layout-pool.inc"
}
ABI_PROBE(PoolLayouts);

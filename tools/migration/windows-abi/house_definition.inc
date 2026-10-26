/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#pragma once
#include "rust/house_definition_ffi.h"
static void HouseDefinitionLayouts()
{
#include "abi-layout-house_definition.inc"
}
ABI_PROBE(HouseDefinitionLayouts);

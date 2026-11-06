/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#pragma once
#include "industrytype.h"
/* SubstituteGRFFileProps derives from a base with fields, so offsetof is conditionally supported;
 * every offset of it and of the records embedding it is checked under each ABI. */
static void IndustrySpecLayouts()
{
#if defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Winvalid-offsetof"
#endif
#include "abi-layout-industry_spec.inc"
#if defined(__GNUC__)
#pragma GCC diagnostic pop
#endif
}
ABI_PROBE(IndustrySpecLayouts);

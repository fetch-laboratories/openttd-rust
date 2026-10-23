/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_yapf.h Shared CYapfBaseT search summary formatting. */
#ifndef RUST_SERVICES_YAPF_H
#define RUST_SERVICES_YAPF_H
#include "../debug.h"
inline void RustYapfDebug(char transport, bool found, uint32_t unit, uint32_t rounds, uint32_t open, uint32_t closed, float ratio, int32_t cost, int32_t distance) noexcept
{
	Debug(yapf, 3, "[YAPF{}]{}{:4d} - {} rounds - {} open - {} closed - CHR {:4.1f}% - C {} D {}",
		transport, found ? '-' : '!', unit, rounds, open, closed, ratio, cost, distance);
}
#endif /* RUST_SERVICES_YAPF_H */

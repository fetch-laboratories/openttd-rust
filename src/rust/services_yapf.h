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
/* ABI adapters share the formatter; neither owns search policy. */
inline void RustRiverYapfDebug(bool found, uint32_t rounds, uint32_t open, uint32_t closed, int32_t cost, int32_t distance) noexcept
{
	RustYapfDebug('~', found, 0, rounds, open, closed, 0.0f, cost, distance);
}
inline void RustShipYapfDebug(uint32_t unit, uint8_t kind, uint8_t found, uint32_t rounds, uint32_t open, uint32_t closed, int32_t cost, int32_t distance) noexcept
{
	RustYapfDebug(kind == 0 ? '^' : 'w', found != 0, unit, rounds, open, closed, 0.0f, cost, distance);
}
#endif /* RUST_SERVICES_YAPF_H */

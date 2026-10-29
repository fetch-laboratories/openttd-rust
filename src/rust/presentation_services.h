/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file presentation_services.h Shared presentation leaves for Rust owners. */
#ifndef RUST_PRESENTATION_SERVICES_H
#define RUST_PRESENTATION_SERVICES_H
#include "../string_func.h"
#include "../debug.h"
#include "../texteff.hpp"
#include "../gfx_func.h"
#include "../town_type.h"
#include "../window_func.h"
inline void OpenTTDInvalidateTownView(uint16_t town) noexcept { InvalidateWindowData(WC_TOWN_VIEW, TownID(town)); }
inline void RustScreenDirty() noexcept { MarkWholeScreenDirty(); }
inline void RustResetWindowSystem() noexcept { ResetWindowSystem(); }
inline void RustSetupColoursAndInitialWindow() noexcept { SetupColoursAndInitialWindow(); }
inline size_t RustUtf8Length(const uint8_t *text, size_t length) noexcept { return Utf8StringLength(std::string_view(reinterpret_cast<const char *>(text), length)); }
inline void RustDebugNewMap(uint32_t seed) noexcept { Debug(desync, 1, "new_map: {:08x}", seed); }
inline void RustCostOrIncomeAnimation(int32_t x, int32_t y, int32_t z, int64_t amount) noexcept { ShowCostOrIncomeAnimation(x, y, z, Money(amount)); }
inline void RustFeederIncomeAnimation(int32_t x, int32_t y, int32_t z, int64_t transfer, int64_t income) noexcept { ShowFeederIncomeAnimation(x, y, z, Money(transfer), Money(income)); }
#endif /* RUST_PRESENTATION_SERVICES_H */

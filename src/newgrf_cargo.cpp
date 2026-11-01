/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_cargo.cpp Implementation of NewGRF cargoes. */

#include "stdafx.h"
#include "debug.h"
#include "newgrf_cargo.h"
#include "newgrf_spritegroup.h"

#include "rust/cargo_resolver.h"
#include "rust/newgrf_spritegroup_ffi.h"

#include "safeguards.h"

uint16_t CargoCallbackFrame(const OpenTTDCargoInvocation *invocation, int32_t *regs, size_t count) noexcept
{
	CargoResolverObject object(*invocation);
	return openttd_rust_cargo_evaluate_callback(invocation, &object, object.CommonState(), regs, count);
}
uint32_t CargoSpriteFrame(const OpenTTDCargoInvocation *invocation) noexcept
{
	CargoResolverObject object(*invocation);
	return openttd_rust_cargo_evaluate_sprite(invocation, &object, object.CommonState());
}

/**
 * Get the custom sprite for the given cargo type.
 * @param cs Cargo being queried.
 * @return Custom sprite to draw, or \c 0 if not available.
 */
SpriteID GetCustomCargoSprite(const CargoSpec *cs)
{
	return openttd_rust_cargo_custom_sprite(cs);
}


uint16_t GetCargoCallback(CallbackID callback, uint32_t param1, uint32_t param2, const CargoSpec *cs, std::span<int32_t> regs100)
{
	return openttd_rust_cargo_callback(callback, param1, param2, cs, regs100.data(), regs100.size());
}

/**
 * Translate a GRF-local cargo slot/bitnum into a CargoType.
 * @param cargo   GRF-local cargo slot/bitnum.
 * @param grffile Originating GRF file.
 * @param usebit  Defines the meaning of \a cargo for GRF version < 7.
 *                If true, then \a cargo is a bitnum. If false, then \a cargo is a cargoslot.
 *                For GRF version >= 7 \a cargo is always a translated cargo bit.
 * @return CargoType or INVALID_CARGO if the cargo is not available.
 */
CargoType GetCargoTranslation(uint8_t cargo, const GRFFile *grffile, bool usebit)
{
	return openttd_rust_cargo_translate(cargo, grffile, usebit);
}

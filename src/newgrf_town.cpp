/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_town.cpp Implementation of the town part of NewGRF houses. */

#include "stdafx.h"
#include "debug.h"
#include "town.h"
#include "newgrf_town.h"
#include "timer/timer_game_tick.h"

#include "safeguards.h"

TownScopeResolver::TownScopeResolver(ResolverObject &ro, Town *t, bool readonly)
	: ScopeResolver(ro), context{t, t == nullptr ? nullptr : t->growth_owner.get(), t == nullptr ? uint16_t{} : t->index.base(), readonly}
{
}

uint32_t TownScopeResolver::GetVariable(uint8_t variable, uint32_t parameter, bool &available) const
{
	return openttd_rust_town_variable(&GetHouseResolverServices(), &this->context, &this->ro, variable, parameter, &available);
}

/* virtual */ void TownScopeResolver::StorePSA(uint pos, int32_t value)
{
	openttd_rust_town_scope_store(&GetHouseResolverServices(), &this->context, &this->ro, pos, value);
}

/**
 * Resolver for a town.
 * @param grffile NewGRF file associated with the town.
 * @param t %Town of the scope.
 * @param readonly Scope may change persistent storage of the town.
 */
TownResolverObject::TownResolverObject(const struct GRFFile *grffile, Town *t, bool readonly)
		: ResolverObject(grffile), town_scope(*this, t, readonly)
{
}

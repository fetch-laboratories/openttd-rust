/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file autoreplace_base.h Base class for autoreplaces/autorenews. */

#ifndef AUTOREPLACE_BASE_H
#define AUTOREPLACE_BASE_H

#include "core/pool_type.hpp"
#include "autoreplace_type.h"
#include "engine_type.h"
#include "group_type.h"

using EngineRenewID = PoolID<uint16_t, struct EngineRenewIDTag, 64000, 0xFFFF>;

/**
 * Memory pool for engine renew elements. DO NOT USE outside of engine.c. Is
 * placed here so the only exception to this rule, the saveload code, can use
 * it.
 */
using EngineRenewPool = Pool<EngineRenew, EngineRenewID, 16>;
extern EngineRenewPool _enginerenew_pool;

/**
 * Struct to store engine replacements. DO NOT USE outside of engine.c. Is
 * placed here so the only exception to this rule, the saveload code, can use
 * it.
 */
#include "rust/fleet_owner.hpp"
struct FleetRenewFields {
	EngineID from = EngineID::Invalid();
	EngineID to = EngineID::Invalid();
	EngineRenew *next = nullptr;
	GroupID group_id = GroupID::Invalid();
	bool replace_when_old = false;
};
struct EngineRenew : EngineRenewPool::PoolItem<&_enginerenew_pool> {
	FleetOwner<FleetRenewFields, openttd_rust_fleet_renew_create, openttd_rust_fleet_renew_destroy> state{};
	EngineID &from = state.state->from;
	EngineID &to = state.state->to;
	EngineRenew *&next = state.state->next;
	GroupID &group_id = state.state->group_id;
	bool &replace_when_old = state.state->replace_when_old;
	EngineRenew() {}
	EngineRenew(EngineID from, EngineID to, GroupID group, bool when_old, EngineRenew *next) {
		this->from = from; this->to = to; this->group_id = group; this->replace_when_old = when_old; this->next = next;
	}
};

#endif /* AUTOREPLACE_BASE_H */

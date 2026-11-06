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
#include "abi-record-fleet.h"
struct EngineRenew : EngineRenewPool::PoolItem<&_enginerenew_pool> {
	struct Owner {
		FleetRenewFields *state = static_cast<FleetRenewFields *>(openttd_rust_fleet_renew_create());
		Owner() = default;
		Owner(const Owner &) = delete;
		~Owner() { openttd_rust_fleet_renew_destroy(this->state); }
	} state;
	FleetRenewFields &Record() { return *this->state.state; }
	const FleetRenewFields &Record() const { return *this->state.state; }
	EngineRenew() {}
	EngineRenew(EngineID from, EngineID to, GroupID group, bool when_old, EngineRenew *next) {
		this->Record().from = from; this->Record().to = to; this->Record().group_id = group; this->Record().replace_when_old = when_old; this->Record().next = next;
	}
};

#endif /* AUTOREPLACE_BASE_H */

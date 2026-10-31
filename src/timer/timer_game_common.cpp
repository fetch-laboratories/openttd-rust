/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file timer_game_common.cpp
 * This file implements the common timer logic for the game-calendar timers.
 */

#include "../stdafx.h"

#include "timer_game_common.h"
#include "timer_game_calendar.h"
#include "timer_game_economy.h"

#include "../rust/clock_ffi.h"
#include "../rust/environment_services.h"
#include "../rust/settings_services.h"
#include "../rust/services_linkgraph.h"
#include "../rust/vehicle_services.h"
#include "timer.h"

#include "../safeguards.h"


const OpenTTDClockServices &GetRustClockServices() noexcept
{
	static const OpenTTDClockServices services{
		.shared = &GetRustSharedServices(),
		.minutes_per_calendar_year = RustMinutesPerCalendarYear,
		.using_wallclock_units = RustUsingWallclockUnits,
		.vehicle_id = RustVehicleIndexOpaque,
		.vehicle_shift_dates = RustVehicleShiftDates,
		.next_graph = RustLinkGraphNext,
		.graph_owner = LiveGraphOwner,
	};
	return services;
}

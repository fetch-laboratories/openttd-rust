/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file base_consist.h Properties for front vehicles/consists. */

#ifndef BASE_CONSIST_H
#define BASE_CONSIST_H

#include "core/enum_type.hpp"
#include "order_type.h"
#include "timer/timer_game_tick.h"
#include "rust/orders_ffi.h"

/** Bit numbers in #Vehicle::vehicle_flags. */
enum class VehicleFlag : uint8_t {
	LoadingFinished = 0, ///< Vehicle has finished loading.
	CargoUnloading = 1, ///< Vehicle is unloading cargo.
	BuiltAsPrototype = 2, ///< Vehicle is a prototype (accepted as exclusive preview).
	TimetableStarted = 3, ///< Whether the vehicle has started running on the timetable yet.
	AutofillTimetable = 4, ///< Whether the vehicle should fill in the timetable automatically.
	AutofillPreserveWaitTime = 5, ///< Whether non-destructive auto-fill should preserve waiting times
	StopLoading = 6, ///< Don't load anymore during the next load cycle.
	PathfinderLost = 7, ///< Vehicle's pathfinder is lost.
	ServiceIntervalIsCustom = 8, ///< Service interval is custom.
	ServiceIntervalIsPercent = 9, ///< Service interval is percent.
};
using VehicleFlags = EnumBitSet<VehicleFlag, uint16_t>;

/** Various front vehicle properties that are preserved when autoreplacing, using order-backup or switching front engines within a consist. */
struct BaseConsist {
	std::string name{}; ///< Name of vehicle

	std::unique_ptr<OpenTTDConsistState, decltype(&openttd_rust_consist_delete)> rust_orders{openttd_rust_consist_new(), openttd_rust_consist_delete};
	TimerGameTick::Ticks &current_order_time = this->rust_orders->current_order_time;
	TimerGameTick::Ticks &lateness_counter = this->rust_orders->lateness_counter;
	TimerGameTick::TickCounter &timetable_start = this->rust_orders->timetable_start;
	TimerGameTick::TickCounter &depot_unbunching_last_departure = this->rust_orders->last_departure;
	TimerGameTick::TickCounter &depot_unbunching_next_departure = this->rust_orders->next_departure;
	TimerGameTick::Ticks &round_trip_time = this->rust_orders->round_trip_time;

	uint16_t service_interval = 0; ///< The interval for (automatic) servicing; either in days or %.

	VehicleOrderID &cur_real_order_index = this->rust_orders->real_index;
	VehicleOrderID &cur_implicit_order_index = this->rust_orders->implicit_index;
	/* EnumBitSet is a C++ class: begin its typed lifetime in Rust-owned storage. */
	VehicleFlags &vehicle_flags = *new (std::addressof(this->rust_orders->vehicle_flags)) VehicleFlags{};

	BaseConsist() = default;
	BaseConsist(const BaseConsist &src) : BaseConsist() { *this = src; }
	BaseConsist &operator=(const BaseConsist &src);
	virtual ~BaseConsist() = default;

	void CopyConsistPropertiesFrom(const BaseConsist *src);
	void ResetDepotUnbunching();
};

#endif /* BASE_CONSIST_H */

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file base_consist.cpp Properties for front vehicles/consists. */

#include "stdafx.h"
#include "base_consist.h"
#include "vehicle_base.h"
#include "string_func.h"

#include "safeguards.h"


/* Ordinary C++ copies retain their original full-value semantics. Each copy
 * constructs its own typed aliases and Rust owner before assigning values. */
BaseConsist &BaseConsist::operator=(const BaseConsist &src)
{
	if (this == &src) return *this;
	this->name = src.name;
	this->current_order_time = src.current_order_time;
	this->lateness_counter = src.lateness_counter;
	this->timetable_start = src.timetable_start;
	this->depot_unbunching_last_departure = src.depot_unbunching_last_departure;
	this->depot_unbunching_next_departure = src.depot_unbunching_next_departure;
	this->round_trip_time = src.round_trip_time;
	this->service_interval = src.service_interval;
	this->cur_real_order_index = src.cur_real_order_index;
	this->cur_implicit_order_index = src.cur_implicit_order_index;
	this->vehicle_flags = src.vehicle_flags;
	return *this;
}

/**
 * Copy properties of other BaseConsist.
 * @param src Source for copying
 */
void BaseConsist::CopyConsistPropertiesFrom(const BaseConsist *src)
{
	if (this == src) return;

	this->name = src->name;

	openttd_rust_consist_copy(this->rust_orders.get(), src->rust_orders.get());
	this->service_interval = src->service_interval;
}

/**
 * Resets all the data used for depot unbunching.
 */
void BaseConsist::ResetDepotUnbunching()
{
	openttd_rust_consist_reset(this->rust_orders.get());
}

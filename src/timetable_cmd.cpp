/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file timetable_cmd.cpp Commands related to time tabling. */

#include "stdafx.h"
#include "command_func.h"
#include "company_func.h"
#include "timer/timer_game_tick.h"
#include "timer/timer_game_economy.h"
#include "window_func.h"
#include "vehicle_base.h"
#include "timetable_cmd.h"
#include "timetable.h"

#include "table/strings.h"

#include "safeguards.h"

/**
 * Get the TimerGameTick::TickCounter tick of a given date.
 * @param start_date The date when the timetable starts.
 * @return The first tick of this date.
 */
TimerGameTick::TickCounter GetStartTickFromDate(TimerGameEconomy::Date start_date)
{
	return openttd_rust_orders_start_tick(start_date.base(), TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, TimerGameTick::counter);
}

/**
 * Get a date from a given start tick of timetable.
 * @param start_tick The TimerGameTick::TickCounter when the timetable starts.
 * @return The date when we reach this tick.
 */
TimerGameEconomy::Date GetDateFromStartTick(TimerGameTick::TickCounter start_tick)
{
	return TimerGameEconomy::Date(openttd_rust_orders_start_date(start_tick, TimerGameEconomy::date.base(), TimerGameEconomy::date_fract, TimerGameTick::counter));
}

/**
 * Change/update a particular timetable entry.
 * @param v            The vehicle to change the timetable of.
 * @param order_number The index of the timetable in the order list.
 * @param val          The new data of the timetable entry.
 * @param mtf          Which part of the timetable entry to change.
 * @param timetabled   If the new value is explicitly timetabled.
 */

/**
 * Change timetable data of an order.
 * @param flags Operation to perform.
 * @param veh Vehicle with the orders to change.
 * @param order_number Order index to modify.
 * @param mtf Timetable data to change (@see ModifyTimetableFlags)
 * @param data The data to modify as specified by \c mtf.
 *             0 to clear times, UINT16_MAX to clear speed limit.
 * @return the cost of this operation or an error
 */
CommandCost CmdChangeTimetable(DoCommandFlags flags, VehicleID veh, VehicleOrderID order_number, ModifyTimetableFlags mtf, uint16_t data)
{
	CommandCost result; openttd_rust_timetable_change(Vehicle::GetIfValid(veh), flags.Test(DoCommandFlag::Execute), order_number, mtf, data, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Change timetable data of all orders of a vehicle.
 * @param flags Operation to perform.
 * @param veh Vehicle with the orders to change.
 * @param mtf Timetable data to change (@see ModifyTimetableFlags)
 * @param data The data to modify as specified by \c mtf.
 *             0 to clear times, UINT16_MAX to clear speed limit.
 * @return the cost of this operation or an error
 */
CommandCost CmdBulkChangeTimetable(DoCommandFlags flags, VehicleID veh, ModifyTimetableFlags mtf, uint16_t data)
{
	CommandCost result; openttd_rust_timetable_bulk(Vehicle::GetIfValid(veh), mtf, data, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Clear the lateness counter to make the vehicle on time.
 * @param flags Operation to perform.
 * @param veh Vehicle with the orders to change.
 * @param apply_to_group Set to reset the late counter for all vehicles sharing the orders.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetVehicleOnTime(DoCommandFlags flags, VehicleID veh, bool apply_to_group)
{
	CommandCost result; openttd_rust_timetable_on_time(Vehicle::GetIfValid(veh), flags.Test(DoCommandFlag::Execute), apply_to_group, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Order vehicles based on their timetable. The vehicles will be sorted in order
 * they would reach the first station.
 *
 * @param a First Vehicle pointer.
 * @param b Second Vehicle pointer.
 * @return Comparison value.
 */

/**
 * Set the start date of the timetable.
 * @param flags Operation to perform.
 * @param veh_id Vehicle ID.
 * @param timetable_all Set to set timetable start for all vehicles sharing this order
 * @param start_tick The TimerGameTick::counter tick when the timetable starts.
 * @return The error or cost of the operation.
 */
CommandCost CmdSetTimetableStart(DoCommandFlags flags, VehicleID veh_id, bool timetable_all, TimerGameTick::TickCounter start_tick)
{
	CommandCost result; openttd_rust_timetable_start(Vehicle::GetIfValid(veh_id), flags.Test(DoCommandFlag::Execute), timetable_all, start_tick, &result, &GetRustOrdersLeaves()); return result;
}


/**
 * Start or stop filling the timetable automatically from the time the vehicle
 * actually takes to complete it. When starting to autofill the current times
 * are cleared and the timetable will start again from scratch.
 * @param flags Operation to perform.
 * @param veh Vehicle index.
 * @param autofill Enable or disable autofill
 * @param preserve_wait_time Set to preserve waiting times in non-destructive mode
 * @return the cost of this operation or an error
 */
CommandCost CmdAutofillTimetable(DoCommandFlags flags, VehicleID veh, bool autofill, bool preserve_wait_time)
{
	CommandCost result; openttd_rust_timetable_autofill(Vehicle::GetIfValid(veh), flags.Test(DoCommandFlag::Execute), autofill, preserve_wait_time, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Update the timetable for the vehicle.
 * @param v The vehicle to update the timetable for.
 * @param travelling Whether we just travelled or waited at a station.
 */
void UpdateVehicleTimetable(Vehicle *v, bool travelling)
{
	openttd_rust_timetable_update(v, travelling, &GetRustOrdersLeaves());
}

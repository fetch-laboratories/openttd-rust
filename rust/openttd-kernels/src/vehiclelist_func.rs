/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions from `vehiclelist_func.h` for generating vehicle lists.
#![allow(unsafe_code)]
#![allow(clippy::cast_possible_truncation)] // OrderList pool IDs fit their u16 representation.
use crate::station_service::{Links, OrderDestination};
use crate::vehicle::VehicleRef;

/// `FindVehiclesWithOrder`: find vehicles matching an order. This can be used, e.g. to find
/// all vehicles that stop at a particular station. `veh_pred` is called only for the first
/// vehicle using the order list; `veh_func` is called for each vehicle that matches both
/// vehicle and order predicates.
pub(crate) fn find_vehicles_with_order(
    links: &Links,
    mut veh_pred: impl FnMut(VehicleRef) -> bool,
    mut ord_pred: impl FnMut(&OrderDestination) -> bool,
    mut veh_func: impl FnMut(VehicleRef),
) {
    for id in 0..(links.order_pool_size)() {
        // OrderList pool IDs are bounded by their u16 representation.
        let orderlist = (links.order_list)(id as u16);
        if orderlist.is_null() {
            continue;
        }
        /* We assume all vehicles sharing an order list match the condition. */
        // A live order list has a first shared vehicle.
        let mut v = unsafe { (links.order_vehicle)(orderlist).unwrap_unchecked() };
        if !veh_pred(v) {
            continue;
        }
        /* Vehicle is a candidate, search for a matching order. */
        for index in 0..(links.order_count)(orderlist) {
            if !ord_pred(&(links.order_destination)(orderlist, index)) {
                continue;
            }
            /* An order matches, we can add all shared vehicles to the list. */
            loop {
                veh_func(v);
                match (links.next_shared)(v) {
                    Some(next) => v = next,
                    None => break,
                }
            }
            break;
        }
    }
}

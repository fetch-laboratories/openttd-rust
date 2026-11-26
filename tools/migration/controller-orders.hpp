/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file controller-orders.hpp Live canonical orders for controller boundary fixtures. */
#ifndef OPENTTD_CONTROLLER_ORDERS_HPP
#define OPENTTD_CONTROLLER_ORDERS_HPP
#include "rust/orders_ffi.h"
#include "rust/station_command_ffi.h"
#include <memory>
#include <new>

/* A real one-order list lets the production orders owner perform its ordinary
 * update/invalidation/destination calls. Fixtures link the vehicle record's order
 * members to `vehicle` and supply the observable native effects; none replaces
 * ProcessOrders with a test callback. */
struct ControllerOrders {
	std::unique_ptr<OpenTTDVehicleOrderState, decltype(&openttd_rust_vehicle_orders_delete)> vehicle{openttd_rust_vehicle_orders_new(), openttd_rust_vehicle_orders_delete};
	std::unique_ptr<OpenTTDOrderListState, decltype(&openttd_rust_orderlist_delete)> list{openttd_rust_orderlist_new(), openttd_rust_orderlist_delete};
	std::unique_ptr<OpenTTDOrderVector, decltype(&openttd_rust_order_vector_delete)> vector{
		openttd_rust_order_vector_new(
			[](void *data, size_t count) noexcept { auto *orders = static_cast<OpenTTDOrderFields *>(data); for (size_t i = 0; i < count; ++i) new (&orders[i]) OpenTTDOrderFields{}; },
			[](void *data, size_t count) noexcept { auto *orders = static_cast<OpenTTDOrderFields *>(data); for (size_t i = 0; i < count; ++i) orders[i].~OpenTTDOrderFields(); }),
		openttd_rust_order_vector_delete};
	OpenTTDConsistState consist{};
	OpenTTDStationCommandServices commands{};
	OpenTTDOrdersLeaves services{};

	/** The list holds `order`, by default a station order. */
	explicit ControllerOrders(const OpenTTDVehicleServices *base, OpenTTDOrderFields order = {.type = 1, .flags = 0, .dest = 1, .refit_cargo = 254, .wait_time = 0, .travel_time = 0, .max_speed = UINT16_MAX})
	{
		vehicle->current_order = {.type = 0, .flags = 0, .dest = 0, .refit_cargo = 254, .wait_time = 0, .travel_time = 0, .max_speed = UINT16_MAX};
		vehicle->orders = this;
		list->num_manual_orders = 1;
		list->num_vehicles = 1;
		openttd_rust_order_vector_insert(vector.get(), 0, &order);
		services.base = base;
		services.station_commands = &commands;
		services.list = [](void *shell) noexcept { return static_cast<ControllerOrders *>(shell)->list.get(); };
		services.vector = [](void *shell) noexcept { return static_cast<ControllerOrders *>(shell)->vector.get(); };
		services.invalidate_vehicle_order = [](void *, int32_t) noexcept {};
		commands.order_station_location = [](OpenTTDVehicle *, uint16_t) noexcept { return uint32_t{1}; };
	}
};
#endif /* OPENTTD_CONTROLLER_ORDERS_HPP */

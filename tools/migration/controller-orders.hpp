/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file controller-orders.hpp Live canonical orders for controller boundary fixtures. */
#ifndef OPENTTD_CONTROLLER_ORDERS_HPP
#define OPENTTD_CONTROLLER_ORDERS_HPP
#include "rust/orders_ffi.h"
#include "rust/vehicle_visits_ffi.h"
#include <memory>
#include <new>

/* A real one-order list lets the production orders owner perform its ordinary
 * update/invalidation/destination calls. Fixtures supply shell resolution and
 * observable native effects; none replaces ProcessOrders with a test callback. */
struct ControllerOrders {
	std::unique_ptr<OpenTTDVehicleOrderState, decltype(&openttd_rust_vehicle_orders_delete)> vehicle{openttd_rust_vehicle_orders_new(), openttd_rust_vehicle_orders_delete};
	std::unique_ptr<OpenTTDOrderListState, decltype(&openttd_rust_orderlist_delete)> list{openttd_rust_orderlist_new(), openttd_rust_orderlist_delete};
	std::unique_ptr<OpenTTDOrderVector, decltype(&openttd_rust_order_vector_delete)> vector{
		openttd_rust_order_vector_new(
			[](void *data, size_t count) noexcept { auto *orders = static_cast<OpenTTDOrderFields *>(data); for (size_t i = 0; i < count; ++i) new (&orders[i]) OpenTTDOrderFields{}; },
			[](void *data, size_t count) noexcept { auto *orders = static_cast<OpenTTDOrderFields *>(data); for (size_t i = 0; i < count; ++i) orders[i].~OpenTTDOrderFields(); }),
		openttd_rust_order_vector_delete};
	OpenTTDConsistState consist{};
	OpenTTDOrdersLeaves services{};
	OpenTTDVehicleCommonServices common{};
	OpenTTDVehicleRefitServices refit{};
	OpenTTDVehicleVisits visits{};

	explicit ControllerOrders(const OpenTTDVehicleServices *base)
	{
		vehicle->current = {.type = 0, .flags = 0, .destination = 0, .refit_cargo = 254, .max_speed = UINT16_MAX};
		vehicle->orders = this;
		list->manual = 1;
		list->vehicles = 1;
		OpenTTDOrderFields order{.type = 1, .flags = 0, .destination = 1, .refit_cargo = 254, .max_speed = UINT16_MAX};
		openttd_rust_order_vector_insert(vector.get(), 0, &order);
		services.base = base;
		common.orders = &services;
		refit.common = &common;
		visits.refit = &refit;
		services.list = [](void *shell) noexcept { return static_cast<ControllerOrders *>(shell)->list.get(); };
		services.vector = [](void *shell) noexcept { return static_cast<ControllerOrders *>(shell)->vector.get(); };
		services.at_station = [](void *) noexcept { return uint64_t{0}; };
		services.destination_tile = [](void *) noexcept { return uint64_t{1}; };
		services.invalidate_order = [](void *, int32_t) noexcept {};
		services.dirty_vehicle_windows = [](void *) noexcept {};
		services.station_location = [](void *, uint16_t) noexcept { return uint64_t{1}; };
		services.set_destination = [](void *, uint32_t) noexcept {};
	}
};
#endif /* OPENTTD_CONTROLLER_ORDERS_HPP */

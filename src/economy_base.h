/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file economy_base.h Base classes related to the economy. */

#ifndef ECONOMY_BASE_H
#define ECONOMY_BASE_H

#include "cargopacket.h"
#include "rust/cargo_payment_ffi.h"

/** Type of pool to store cargo payments in; little over 1 million. */
using CargoPaymentPool = Pool<CargoPayment, CargoPaymentID, 512>;
/** The actual pool to store cargo payments in. */
extern CargoPaymentPool _cargo_payment_pool;

/**
 * Helper class to perform the cargo payment.
 */
struct CargoPayment : CargoPaymentPool::PoolItem<&_cargo_payment_pool> {
	OpenTTDCargoPayment *rust_state; ///< Sole owner of payment state.
	/* Aliases into the repr(C) Rust payment, used by the CAPY descriptors. */
	Vehicle *&front = reinterpret_cast<Vehicle *&>(this->rust_state->fields.front);
	Money &route_profit = *reinterpret_cast<Money *>(&this->rust_state->fields.route_profit);
	Money &visual_profit = *reinterpret_cast<Money *>(&this->rust_state->fields.visual_profit);
	Money &visual_transfer = *reinterpret_cast<Money *>(&this->rust_state->fields.visual_transfer);
	CargoPayment();
	Vehicle *GetFront() const;
	void AfterLoad();
	CargoPayment(Vehicle *front);
	~CargoPayment();

	Money PayTransfer(CargoType cargo, const CargoPacket *cp, uint count, TileIndex current_tile);
	void PayFinalDelivery(CargoType cargo, const CargoPacket *cp, uint count, TileIndex current_tile);
};

#endif /* ECONOMY_BASE_H */

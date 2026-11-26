/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_payment_ffi.h Rust payment lifetime and cargo delivery services. */
#ifndef RUST_CARGO_PAYMENT_FFI_H
#define RUST_CARGO_PAYMENT_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
struct OpenTTDVehicle;
struct OpenTTDCompanyFinanceServices;
struct OpenTTDVehicleServices;
struct OpenTTDCargoRegistryServices;
struct OpenTTDSubsidyServices;
struct OpenTTDIndustryResolverServices;
struct OpenTTDIndustryDeliveryServices;
struct Station;
struct OpenTTDTownHandle;
struct OpenTTDCargoStorageServices;
struct CargoPacket;

/** Opaque live native entity, distinct from Rust-owned boundary state. */
struct CargoPayment;
using OpenTTDNativeCargoPayment = CargoPayment;
#include <cstdint>
#include "call_ffi.h"
struct OpenTTDCargoPaymentFields {
	OpenTTDVehicle *front;
	int64_t route_profit, visual_profit, visual_transfer;
};
/* Rust-allocated repr(C) payment (ABI layout 60); C++ aliases address fields in place. */
struct OpenTTDCargoPayment {
	OpenTTDCargoPaymentFields fields;
	uint16_t station;
};
/* Leaves cannot reenter payment or delivery owners or destroy the supplied world objects. Money add,
 * sub, neg and multiply saturate. Each shell owns one payment; the delivery destinations live for the
 * process. Destroy accepts a null table only when cleaning. Owners and outputs are live, nonnull and
 * exclusive; tables are immutable and shareable. All table entries have the declared ABI.
 * Resolvers cannot run script VMs or mutate payment/destination storage; callbacks retain no pointers,
 * and unresolved front handles are not dereferenced before load fixup. */
struct OpenTTDSharedServices;
struct OpenTTDStationServices;
struct OpenTTDCargoServices {
	void (OPENTTD_RUST_CALL *detach_payment)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(detach_payment);
	void (OPENTTD_RUST_CALL *feeder_animation)(int32_t, int32_t, int32_t, int64_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(feeder_animation);
	const OpenTTDCompanyFinanceServices *finance;
	const OpenTTDIndustryDeliveryServices *industry_delivery;
	const OpenTTDCargoRegistryServices *registry;
	const OpenTTDIndustryResolverServices *resolver;
	const OpenTTDSharedServices *shared;
	const OpenTTDTownHandle *(OPENTTD_RUST_CALL *station_town)(const Station *) noexcept OPENTTD_SERVICE_DEFAULT(station_town);
	const OpenTTDStationServices *stations;
	const OpenTTDSubsidyServices *(OPENTTD_RUST_CALL *subsidies)() noexcept OPENTTD_SERVICE_DEFAULT(subsidies);
	const OpenTTDVehicleServices *vehicles;
};
const OpenTTDCargoServices &GetRustCargoServices() noexcept;
extern "C" {
OpenTTDCargoPayment *OPENTTD_RUST_CALL openttd_rust_cargo_payment_new(OpenTTDVehicle *);
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_destroy(OpenTTDCargoPayment *, const OpenTTDCargoServices *, uint8_t);
OpenTTDVehicle *OPENTTD_RUST_CALL openttd_rust_cargo_payment_front(const OpenTTDCargoPayment *);
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_afterload(OpenTTDCargoPayment *);
/* For the consoles only: CargoPayment::PayFinalDelivery for a packet shell. */
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_final(const OpenTTDCargoStorageServices *, CargoPayment *, uint8_t, CargoPacket *, uint32_t, uint32_t);
int64_t OPENTTD_RUST_CALL openttd_rust_cargo_income(const OpenTTDCargoServices *, uint32_t, uint32_t, uint16_t, uint8_t);
}
#endif /* RUST_CARGO_PAYMENT_FFI_H */

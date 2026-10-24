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
struct OpenTTDSubsidyServices;
struct OpenTTDIndustryResolverServices;
struct OpenTTDCargoMonitorServices;
struct RustNearbyIndustryCursor;

/** Opaque live native entity, distinct from Rust-owned boundary state. */
struct CargoPayment;
using OpenTTDNativeCargoPayment = CargoPayment;
#include <cstdint>
#include "call_ffi.h"
struct OpenTTDCargoDelivery;
struct OpenTTDCargoSpec {
	int64_t payment;
	uint8_t valid, callback, periods1, periods2;
};
struct OpenTTDCargoPaymentFields {
	void *front;
	int64_t route_profit, visual_profit, visual_transfer;
};
/* Rust-allocated repr(C) payment (ABI layout 60); C++ aliases address fields in place. */
struct OpenTTDCargoPayment {
	OpenTTDCargoPaymentFields fields;
	uint16_t station;
};
/* All leaves are synchronous noexcept, cannot reenter payment/delivery owners,
 * retain arguments or destroy the world objects supplied by the caller. NewGRF
 * resolvers execute sprite-group programs rather than script VMs; cargo profit,
 * refusal, production, randomisation, animation and vehicle sound resolution do
 * not invoke arbitrary game commands or mutate payment/destination storage.
 * Entry frames own unbound nearby-industry cursors; begin binds each scan.
 * Delivery finishes its scan before subsidy checks reuse the cursor; production
 * flush is later. Pool/world pointers are opaque handles, never Rust references. Returned fields
 * are copied scalars. Environmental exceptions terminate inside the wrappers.
 * Save/load and pointer fixups write the fields in place between Rust calls and
 * do not run simulation. Panics abort.
 * Money add/sub/neg/multiply saturate; shifts retain the source bit operations.
 * Each shell owns exactly one new/destroy payment; the delivery owner is process
 * lifetime. Calls are serialized, with live nonnull owner/table/output pointers
 * and exclusive mutation access. Every function-table entry must be nonnull and
 * have the declared native C ABI; destroy permits a null table only when cleaning.
 * Rust never dereferences a front reference that load has not yet fixed up.
 * Integer argument widths, slot bounds and pool lifetimes are caller preconditions.
 */
/* Leaf selectors: station read0 owner,1 always-accepted(cargo); industry read
 * 1 accepted-slot(cargo)/UINT32_MAX,2 supplier,3 accepted-waiting,4 production
 * callback bits(arrival=1,periodic=2),5/6 input/output counts,7/8 input/output
 * cargo,9 output-waiting,10 multiplier(input<<16|output). Industry write0 output,
 * 1 input waiting. Statistics0 station,1 company,2 town. Industry effect0 mark,
 * 1 arrival callback,2 dirty,3 randomise,4 animate. Vehicle read0 station,1 owner.
 * Settle0 detach,1 set-company/return-old,2 debit,3 yearly-profit,4 local-company,
 * 5 feeder-animation,6 income-animation,7 restore-company,8 sound/return-played,
 * 9 default sound. Setting0 subsidy multiplier,1 feeder share. Monitor source is
 * SourceID low16/SourceType high16, industry=0xFFFF means town/house delivery. */
struct OpenTTDCargoServices {
	void (OPENTTD_RUST_CALL *spec)(uint8_t, OpenTTDCargoSpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	uint16_t (OPENTTD_RUST_CALL *callback)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(callback);
	uint32_t (OPENTTD_RUST_CALL *station_read)(uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_read);
	uint32_t (OPENTTD_RUST_CALL *industry_read)(void *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_read);
	void (OPENTTD_RUST_CALL *industry_write)(void *, uint8_t, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_write);
	const OpenTTDIndustryResolverServices *resolver;
	void (OPENTTD_RUST_CALL *accept)(void *, uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(accept);
	void (OPENTTD_RUST_CALL *statistics)(uint16_t, uint8_t, uint8_t, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(statistics);
	const OpenTTDSubsidyServices *(OPENTTD_RUST_CALL *subsidies)() noexcept OPENTTD_SERVICE_DEFAULT(subsidies);
	void (OPENTTD_RUST_CALL *industry_effect)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_effect);
	uint32_t (OPENTTD_RUST_CALL *vehicle_read)(void *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_read);
	uint32_t (OPENTTD_RUST_CALL *settle)(void *, uint8_t, int64_t, int64_t) noexcept OPENTTD_SERVICE_DEFAULT(settle);
	int64_t (OPENTTD_RUST_CALL *feeder)(const void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(feeder);
	uint32_t (OPENTTD_RUST_CALL *setting)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(setting);
};
extern "C" {
OpenTTDCargoPayment *OPENTTD_RUST_CALL openttd_rust_cargo_payment_new(void *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_destroy(OpenTTDCargoPayment *, const OpenTTDCargoServices *, uint8_t);
void *OPENTTD_RUST_CALL openttd_rust_cargo_payment_front(const OpenTTDCargoPayment *);
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_afterload(OpenTTDCargoPayment *, const OpenTTDCargoServices *);
int64_t OPENTTD_RUST_CALL openttd_rust_cargo_payment_transfer(OpenTTDCargoPayment *, const OpenTTDCargoServices *, uint8_t, const void *, uint32_t, uint32_t, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_cargo_payment_final(OpenTTDCargoPayment *, OpenTTDCargoDelivery *, const OpenTTDCargoServices *, uint8_t, const void *, uint32_t, uint32_t, uint16_t, uint32_t, RustNearbyIndustryCursor *, const OpenTTDCargoMonitorServices *, RustNearbyIndustryCursor *);
int64_t OPENTTD_RUST_CALL openttd_rust_cargo_income(const OpenTTDCargoServices *, uint32_t, uint32_t, uint16_t, uint8_t);
OpenTTDCargoDelivery *OPENTTD_RUST_CALL openttd_rust_cargo_delivery_new();
void OPENTTD_RUST_CALL openttd_rust_cargo_delivery_destroy(OpenTTDCargoDelivery *);
void OPENTTD_RUST_CALL openttd_rust_cargo_delivery_flush(OpenTTDCargoDelivery *, const OpenTTDCargoServices *);
}
#endif /* RUST_CARGO_PAYMENT_FFI_H */

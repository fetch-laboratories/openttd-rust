/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file depot_ffi.h Depot naming owner boundary; records and names stay in the native pool. */
#ifndef RUST_DEPOT_FFI_H
#define RUST_DEPOT_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
#include "command_cost_ffi.h"
struct OpenTTDSharedServices;
struct OpenTTDCommandServices;
struct OpenTTDRailCommandServices;
struct Depot;
/** Name assignment and window refresh are whole native operations. */
struct OpenTTDDepotServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandServices *command;
	const OpenTTDCommandCostServices *costs;
	const OpenTTDRailCommandServices *rail;
	Depot *(*depot)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(depot);
	uint32_t (*pool_size)() noexcept OPENTTD_SERVICE_DEFAULT(pool_size);
	uint32_t (*xy)(const Depot *) noexcept OPENTTD_SERVICE_DEFAULT(xy);
	OpenTTDByteSpan (*name)(const Depot *) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*set_name)(Depot *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	void (*renamed)(Depot *) noexcept OPENTTD_SERVICE_DEFAULT(renamed);
	uint32_t name_unique;
};
extern "C" {
void openttd_rust_depot_rename(const OpenTTDDepotServices *, OpenTTDCommandCost *, uint16_t, uint16_t, OpenTTDByteSpan);
}
#endif /* RUST_DEPOT_FFI_H */

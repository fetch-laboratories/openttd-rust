/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file depot_ffi.h Depot naming boundary over canonical Rust records. */
#ifndef RUST_DEPOT_FFI_H
#define RUST_DEPOT_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
#include "command_cost_ffi.h"
struct OpenTTDSharedServices;
struct OpenTTDCommandServices;
struct OpenTTDRailCommandServices;
struct Depot;
/** Native window refresh follows canonical Rust name mutation. */
struct OpenTTDDepotServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandServices *command;
	const OpenTTDCommandCostServices *costs;
	void (*renamed)(Depot *) noexcept OPENTTD_SERVICE_DEFAULT(renamed);
	uint32_t name_unique;
};
extern "C" {
void openttd_rust_depot_default_name(const OpenTTDSharedServices *, uint16_t);
void openttd_rust_depot_rename(const OpenTTDDepotServices *, OpenTTDCommandCost *, uint16_t, uint16_t, OpenTTDByteSpan);
}
#endif /* RUST_DEPOT_FFI_H */

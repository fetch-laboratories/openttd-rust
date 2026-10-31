/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file signs_ffi.h Sign command owner boundary; records and names stay in the native pool. */
#ifndef RUST_SIGNS_FFI_H
#define RUST_SIGNS_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
#include "command_cost_ffi.h"
struct OpenTTDSharedServices;
struct Sign;
/** Each operation is one whole native operation; pointers die with their sign. */
struct OpenTTDSignServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *costs;
	Sign *(*sign)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(sign);
	uint8_t (*owner)(const Sign *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	uint16_t (*place)(uint8_t, uint32_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(place);
	void (*rename)(Sign *, uint8_t, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(rename);
	void (*relocate)(Sign *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(relocate);
	void (*remove)(Sign *) noexcept OPENTTD_SERVICE_DEFAULT(remove);
	uint32_t too_many_signs;
};
const OpenTTDSignServices &GetRustSignServices() noexcept;
extern "C" {
uint16_t openttd_rust_sign_place(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint32_t, OpenTTDByteSpan);
void openttd_rust_sign_rename(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint16_t, OpenTTDByteSpan);
void openttd_rust_sign_move(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t);
bool openttd_rust_company_can_edit_sign(const OpenTTDSignServices *, const Sign *);
}
#endif /* RUST_SIGNS_FFI_H */

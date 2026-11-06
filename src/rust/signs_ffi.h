/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file signs_ffi.h Sign command boundary over canonical Rust records and native viewport operations. */
#ifndef RUST_SIGNS_FFI_H
#define RUST_SIGNS_FFI_H
#include "service_defaults.hpp"
#include "byte_span_ffi.h"
#include "command_cost_ffi.h"
struct OpenTTDSharedServices;
struct Sign;
struct OpenTTDSignRecord;
/** Each operation is one whole native operation; pointers die with their sign. */
struct OpenTTDSignServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *costs;
	OpenTTDSignRecord *(*create)() noexcept OPENTTD_SERVICE_DEFAULT(create);
	void (*position)(Sign *) noexcept OPENTTD_SERVICE_DEFAULT(position);
	void (*list_changed)(int32_t) noexcept OPENTTD_SERVICE_DEFAULT(list_changed);
	void (*remove)(Sign *) noexcept OPENTTD_SERVICE_DEFAULT(remove);
	uint32_t too_many_signs;
};
const OpenTTDSignServices &GetRustSignServices() noexcept;
extern "C" {
uint16_t openttd_rust_sign_place(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint32_t, OpenTTDByteSpan);
void openttd_rust_sign_rename(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint16_t, OpenTTDByteSpan);
void openttd_rust_sign_move(const OpenTTDSignServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t);
uint8_t openttd_rust_company_can_edit_sign_abi(const OpenTTDSignServices *, const OpenTTDSignRecord *);
extern "C++" inline bool openttd_rust_company_can_edit_sign(const OpenTTDSignServices *a0, const OpenTTDSignRecord *a1) { return openttd_rust_company_can_edit_sign_abi(a0, a1) != 0; }
}
#endif /* RUST_SIGNS_FFI_H */

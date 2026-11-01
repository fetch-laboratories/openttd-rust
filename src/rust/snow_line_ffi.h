/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file snow_line_ffi.h Native loader allocation and live setting access. */
#ifndef RUST_SNOW_LINE_FFI_H
#define RUST_SNOW_LINE_FFI_H
#include "service_defaults.hpp"
struct SnowLine;
struct OpenTTDSnowLineSupport {
	const uint8_t *height;
	void (*destroy)(SnowLine *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
};
struct OpenTTDSharedServices;
extern "C" {
bool openttd_rust_snow_line_is_set();
void openttd_rust_snow_line_set(const OpenTTDSnowLineSupport *, SnowLine *);
void openttd_rust_snow_line_clear(const OpenTTDSharedServices *);
uint8_t openttd_rust_snow_line_get(const OpenTTDSharedServices *);
uint8_t openttd_rust_snow_line_lowest(const OpenTTDSharedServices *);
}
#endif

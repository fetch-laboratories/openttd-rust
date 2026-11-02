/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_text_ffi.h GRF text table and language selection. */
#ifndef RUST_NEWGRF_TEXT_FFI_H
#define RUST_NEWGRF_TEXT_FFI_H
#include "byte_span_ffi.h"
extern "C" {
size_t openttd_rust_grf_text_prepare(uint32_t, uint32_t, uint32_t);
void openttd_rust_grf_text_publish(size_t, uint8_t, const uint8_t *, size_t);
uint32_t openttd_rust_grf_text_id(uint32_t, uint32_t, uint32_t);
OpenTTDByteSpan openttd_rust_grf_text_get(size_t, uint32_t *);
size_t openttd_rust_grf_text_select(const void *, size_t, uint8_t (*)(const void *, size_t) noexcept);
void openttd_rust_grf_text_set_language(uint8_t);
uint8_t openttd_rust_grf_text_check_language(uint8_t, uint8_t);
void openttd_rust_grf_text_cleanup();
}
#endif

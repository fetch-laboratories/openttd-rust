/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_text_ffi.h GRF text table and language selection. */
#ifndef RUST_NEWGRF_TEXT_FFI_H
#define RUST_NEWGRF_TEXT_FFI_H
#include "byte_span_ffi.h"
#include "../newgrf_text_type.h"
#include "service_table_check.hpp"
struct OpenTTDGrfTextServices {
	uint32_t empty;
	const LanguageMap *(*language_map)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(language_map);
	uint32_t (*legacy_string)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(legacy_string);
	int32_t (*mapping)(const LanguageMap *, int32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(mapping);
	uint8_t max_plural_forms;
	uint8_t (*num_cases)() noexcept OPENTTD_SERVICE_DEFAULT(num_cases);
	uint8_t (*num_genders)() noexcept OPENTTD_SERVICE_DEFAULT(num_genders);
	int32_t (*plural_form)(const LanguageMap *) noexcept OPENTTD_SERVICE_DEFAULT(plural_form);
	int32_t (*reverse_mapping)(const LanguageMap *, int32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(reverse_mapping);
	uint32_t undefined;
};
const OpenTTDGrfTextServices &GetRustGrfTextServices() noexcept;
struct OpenTTDSharedServices;
extern "C" {
void openttd_rust_grf_text_translate(const OpenTTDGrfTextServices *, uint32_t, uint8_t, bool, const uint8_t *, size_t, uint32_t, void *, void (*)(void *, const uint8_t *, size_t) noexcept);
uint32_t openttd_rust_grf_text_add(const OpenTTDGrfTextServices *, uint32_t, uint32_t, uint8_t, bool, bool, const uint8_t *, size_t, uint32_t);
uint32_t openttd_rust_grf_text_id(uint32_t, uint32_t, uint32_t);
uint32_t openttd_rust_grf_text_map(const OpenTTDGrfTextServices *, uint32_t, uint32_t);
OpenTTDByteSpan openttd_rust_grf_text_get(const OpenTTDSharedServices *, size_t, uint32_t *);
size_t openttd_rust_grf_text_select(const GRFText *, size_t, size_t);
void openttd_rust_grf_text_set_language(uint8_t);
uint8_t openttd_rust_grf_text_check_language_abi(uint8_t, uint8_t);
extern "C++" inline bool openttd_rust_grf_text_check_language(uint8_t a0, uint8_t a1) { return openttd_rust_grf_text_check_language_abi(a0, a1) != 0; }
}
#endif

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_translation_ffi.h Shared incremental file-label table policy. */
#ifndef RUST_NEWGRF_TRANSLATION_FFI_H
#define RUST_NEWGRF_TRANSLATION_FFI_H
#include "newgrf_bytereader_ffi.h"
struct OpenTTDTranslationServices {
	void (*reset)(GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reset);
	void (*append)(GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(append);
	void (*copy)(GRFFile *, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(copy);
	GRFFile *(*override_file)() noexcept OPENTTD_SERVICE_DEFAULT(override_file);
	void (*invalid_first)(const char *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_first);
	void (*log_override)(const char *, size_t, const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(log_override);
};
extern "C" int32_t openttd_rust_load_translation(const OpenTTDTranslationServices *, const OpenTTDByteReaderServices *, GRFFile *, uint32_t, uint32_t, ByteReader *, const char *, size_t);
#endif

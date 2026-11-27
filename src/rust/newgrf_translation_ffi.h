/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_translation_ffi.h Shared incremental file-label table policy. */
#ifndef RUST_NEWGRF_TRANSLATION_FFI_H
#define RUST_NEWGRF_TRANSLATION_FFI_H
#include "newgrf_bytereader_ffi.h"
struct OpenTTDTranslationTable;
struct OpenTTDTranslationServices {
	const OpenTTDByteReaderServices *reader;
	const struct OpenTTDSharedServices *shared;
	OpenTTDTranslationTable *(*gettable)(GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(gettable);
	void (*clear)(OpenTTDTranslationTable *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*reserve)(OpenTTDTranslationTable *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reserve);
	void (*push_back)(OpenTTDTranslationTable *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(push_back);
	void (*assign)(OpenTTDTranslationTable *, const OpenTTDTranslationTable *) noexcept OPENTTD_SERVICE_DEFAULT(assign);
	GRFFile *(*get_current_grf_override)() noexcept OPENTTD_SERVICE_DEFAULT(get_current_grf_override);
};
extern "C" int32_t openttd_rust_load_translation(const OpenTTDTranslationServices *, GRFFile *, uint32_t, uint32_t, ByteReader *, const char *, size_t);
#endif

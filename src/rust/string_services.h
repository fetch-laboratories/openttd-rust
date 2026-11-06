/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file string_services.h Shared native string storage operations. */
#ifndef RUST_STRING_SERVICES_H
#define RUST_STRING_SERVICES_H
#include <string>
inline void RustStringAppend(void *text, const uint8_t *data, size_t size) noexcept
{
	static_cast<std::string *>(text)->append(reinterpret_cast<const char *>(data), size);
}
extern "C" void openttd_rust_string_append_bind(void (*append)(void *, const uint8_t *, size_t) noexcept);
#endif /* RUST_STRING_SERVICES_H */

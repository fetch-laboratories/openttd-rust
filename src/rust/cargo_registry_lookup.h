/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_registry_lookup.h Native cargo label facade. */
#ifndef RUST_CARGO_REGISTRY_LOOKUP_H
#define RUST_CARGO_REGISTRY_LOOKUP_H
#include <cstdint>
extern "C" uint8_t openttd_rust_cargo_by_label(uint32_t);
#endif

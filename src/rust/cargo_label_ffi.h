/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_label_ffi.h A cargo label or mixed-cargo alternative, as Rust stores it. */
#ifndef RUST_CARGO_LABEL_FFI_H
#define RUST_CARGO_LABEL_FFI_H
#include <cstdint>
struct OpenTTDNewGRFCargoLabel { uint32_t value; bool mixed; };
#endif /* RUST_CARGO_LABEL_FFI_H */

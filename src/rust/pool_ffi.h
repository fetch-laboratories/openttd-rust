/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file pool_ffi.h Canonical Rust pool operations used by native pool facades. */
#ifndef RUST_POOL_FFI_H
#define RUST_POOL_FFI_H
#include "rust_vector.h"
#include "abi-record-pool.h"
struct RustPoolCore;
extern "C" {
RustPoolCore *OPENTTD_RUST_CALL openttd_rust_pool_new(const RustPoolConfig *, const RustPoolNativeOps *);
void OPENTTD_RUST_CALL openttd_rust_pool_drop(RustPoolCore *);
RustPoolDescriptor *OPENTTD_RUST_CALL openttd_rust_pool_descriptor(RustPoolCore *);
uint8_t OPENTTD_RUST_CALL openttd_rust_pool_can_allocate(RustPoolCore *, size_t);
void *OPENTTD_RUST_CALL openttd_rust_pool_allocate(RustPoolCore *, size_t);
void *OPENTTD_RUST_CALL openttd_rust_pool_allocate_at(RustPoolCore *, size_t, size_t);
void OPENTTD_RUST_CALL openttd_rust_pool_bind_record(RustPoolCore *, size_t, void *);
void OPENTTD_RUST_CALL openttd_rust_pool_retire_record(RustPoolCore *, size_t);
void OPENTTD_RUST_CALL openttd_rust_pool_free(RustPoolCore *, size_t, size_t);
void OPENTTD_RUST_CALL openttd_rust_pool_clean(RustPoolCore *);
}
#endif /* RUST_POOL_FFI_H */

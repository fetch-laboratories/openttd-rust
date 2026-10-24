/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file call_ffi.h Calling convention of Rust `extern "C"` entries and of every callback Rust calls. */
#ifndef OPENTTD_RUST_CALL_FFI_H
#define OPENTTD_RUST_CALL_FFI_H
#if defined(_MSC_VER)
#define OPENTTD_RUST_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_RUST_CALL __attribute__((cdecl))
#else
#define OPENTTD_RUST_CALL
#endif
#endif /* OPENTTD_RUST_CALL_FFI_H */

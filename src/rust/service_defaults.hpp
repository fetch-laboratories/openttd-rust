/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file service_defaults.hpp Signature-correct abort defaults for borrowed callback tables. */
#ifndef OPENTTD_RUST_SERVICE_DEFAULTS_HPP
#define OPENTTD_RUST_SERVICE_DEFAULTS_HPP
#ifdef __cplusplus
#include <cstdlib>
#if defined(_MSC_VER)
#define OPENTTD_SERVICE_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_SERVICE_CALL __attribute__((cdecl))
#else
#define OPENTTD_SERVICE_CALL
#endif
/* These templates must be included outside extern "C". Defaults add no storage,
 * preserve standard layout/trivial copying, and abort before returning to Rust.
 * Omitting a fixture slot is valid; invoking it fails immediately. */
template <typename T> struct OpenTTDAbortService;
template <typename R, typename... A> struct OpenTTDAbortService<R (OPENTTD_SERVICE_CALL *)(A...)> {
	static R OPENTTD_SERVICE_CALL Call(A...) noexcept { std::abort(); }
};
template <typename R, typename... A> struct OpenTTDAbortService<R (OPENTTD_SERVICE_CALL *)(A...) noexcept> : OpenTTDAbortService<R (OPENTTD_SERVICE_CALL *)(A...)> {};
#define OPENTTD_SERVICE_DEFAULT(field) = OpenTTDAbortService<decltype(field)>::Call
#else
#define OPENTTD_SERVICE_DEFAULT(field)
#endif
#endif /* OPENTTD_RUST_SERVICE_DEFAULTS_HPP */

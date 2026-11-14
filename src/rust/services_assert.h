/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_assert.h Canonical native invariant-failure operation. */
#ifndef RUST_SERVICES_ASSERT_H
#define RUST_SERVICES_ASSERT_H
inline void RustAssertCondition([[maybe_unused]] bool condition) noexcept { assert(condition); }
inline void RustNotReached() noexcept { NOT_REACHED(); }
struct OpenTTDSharedServices;
extern "C" void openttd_rust_assert_bind(void (*check)(bool) noexcept, const OpenTTDSharedServices *shared);
#endif /* RUST_SERVICES_ASSERT_H */

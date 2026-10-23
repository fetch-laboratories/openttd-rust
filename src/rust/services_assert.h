/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_assert.h Canonical native invariant-failure operation. */
#ifndef RUST_SERVICES_ASSERT_H
#define RUST_SERVICES_ASSERT_H
inline void RustAssertCondition([[maybe_unused]] bool condition) noexcept { assert(condition); }
inline void RustNotReached() noexcept { NOT_REACHED(); }
template <typename Owner>
inline void RustNotReachedWithUnusedOwner(Owner) noexcept { RustNotReached(); }
#endif /* RUST_SERVICES_ASSERT_H */

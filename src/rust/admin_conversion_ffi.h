/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file admin_conversion_ffi.h Admin-only resumable conversion control. */
#ifndef RUST_ADMIN_CONVERSION_FFI_H
#define RUST_ADMIN_CONVERSION_FFI_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct OpenTTDRustAdminConversion OpenTTDRustAdminConversion;

typedef struct {
	uint64_t frame; ///< Stable LIFO C++ frame slot; never a pointer.
	int64_t index; ///< Original signed SQInteger index or saved stack top.
	uint8_t kind;
	uint8_t argument;
} OpenTTDRustAdminAction;

enum OpenTTDRustAdminOperation {
	OTTD_ADMIN_DONE, OTTD_ADMIN_INSPECT_OUT, OTTD_ADMIN_INSPECT_IN,
	OTTD_ADMIN_COPY_OUT_SCALAR, OTTD_ADMIN_PUSH_IN_SCALAR,
	OTTD_ADMIN_BEGIN_OUT, OTTD_ADMIN_BEGIN_IN, OTTD_ADMIN_NEXT_OUT,
	OTTD_ADMIN_CHECK_IN_ITERATOR, OTTD_ADMIN_COPY_OUT_KEY, OTTD_ADMIN_PUSH_IN_KEY,
	OTTD_ADMIN_CHILD_OUT, OTTD_ADMIN_CHILD_IN, OTTD_ADMIN_POP_PAIR,
	OTTD_ADMIN_POP_ITERATOR, OTTD_ADMIN_COMMIT_OUT_ARRAY, OTTD_ADMIN_COMMIT_OUT_TABLE,
	OTTD_ADMIN_COMMIT_IN_ARRAY, OTTD_ADMIN_COMMIT_IN_TABLE, OTTD_ADMIN_ADVANCE_IN_ITERATOR,
	OTTD_ADMIN_RELEASE_FRAME, OTTD_ADMIN_ERROR_DEPTH, OTTD_ADMIN_ERROR_OUT_TYPE,
	OTTD_ADMIN_ERROR_ROOT, OTTD_ADMIN_ERROR_IN_TYPE, OTTD_ADMIN_SAVE_TOP,
	OTTD_ADMIN_RESTORE_TOP, OTTD_ADMIN_PUSH_NULL,
};

/** Direction 0 Squirrel->JSON, 1 incoming GetObject including root/rollback policy. index is
 * int64 SQInteger even on i686; depth is the original 32-bit int. C++ executes each action after
 * advance returns, so VM, metamethod, JSON and log exceptions never cross a Rust frame; destroy on
 * an exception runs no pending VM pops, rollback or diagnostics. Reentrant calls own separate
 * engines. */
OpenTTDRustAdminConversion *openttd_rust_admin_conversion_create(uint8_t direction, int64_t index, int32_t depth);

/**
 * Supply the previous action's result: raw pinned type, 0/1 iterator availability,
 * modulo-2^64 saved SQInteger top, or zero for void operations. Initial result 0.
 * Done argument is outgoing success / incoming conversion outcome. Scalar argument
 * is 0 null, 1 boolean, 2 string, 3 integer. Container argument is 0 array, 1 table.
 * Release argument 1 destroys the child then its parent's copied table key.
 * Frame values are stack slots representable by size_t on this native target.
 */
OpenTTDRustAdminAction openttd_rust_admin_conversion_advance(OpenTTDRustAdminConversion *engine, uint64_t response);
void openttd_rust_admin_conversion_destroy(OpenTTDRustAdminConversion *engine); ///< Destroy exactly once, including C++ unwinding; no VM cleanup.
#ifdef __cplusplus
}
#endif
#endif /* RUST_ADMIN_CONVERSION_FFI_H */

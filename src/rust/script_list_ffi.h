/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_list_ffi.h Rust-owned list with scalar, synchronous operations. */
#ifndef OPENTTD_RUST_SCRIPT_LIST_FFI_H
#define OPENTTD_RUST_SCRIPT_LIST_FFI_H
#include <stdint.h>
#if defined(_MSC_VER)
#define OPENTTD_LIST_CALL __cdecl
#elif defined(__i386__)
#define OPENTTD_LIST_CALL __attribute__((cdecl))
#else
#define OPENTTD_LIST_CALL
#endif
#ifdef __cplusplus
extern "C" {
#endif
struct OpenTTDScriptList;
/* Owners are allocated and freed only in Rust; no callback or VM operation runs under Rust, and
 * mutations during an external callback finish before the next call. Combine recognizes
 * self-aliasing before creating Rust references. Original signed-overflow and invalidated
 * valuation-iterator cases are outside the defined-input contract. Owners are live, aligned and
 * serially accessed; scalar outputs are writable and disjoint. */
struct OpenTTDScriptList *OPENTTD_LIST_CALL openttd_rust_list_new(void);
void OPENTTD_LIST_CALL openttd_rust_list_destroy(struct OpenTTDScriptList *);
int32_t OPENTTD_LIST_CALL openttd_rust_list_token(const struct OpenTTDScriptList *);
int64_t OPENTTD_LIST_CALL openttd_rust_list_count(const struct OpenTTDScriptList *);
uint8_t OPENTTD_LIST_CALL openttd_rust_list_get(const struct OpenTTDScriptList *, int64_t, int64_t *);
void OPENTTD_LIST_CALL openttd_rust_list_add(struct OpenTTDScriptList *, int64_t, int64_t);
void OPENTTD_LIST_CALL openttd_rust_list_remove(struct OpenTTDScriptList *, int64_t);
uint8_t OPENTTD_LIST_CALL openttd_rust_list_set(struct OpenTTDScriptList *, int64_t, int64_t);
void OPENTTD_LIST_CALL openttd_rust_list_clear(struct OpenTTDScriptList *);
void OPENTTD_LIST_CALL openttd_rust_list_sort(struct OpenTTDScriptList *, int32_t, uint8_t);
/* Iteration 0=Begin, 1=Next, 2=IsEnd. Status zero means never initialized for
 * Next/IsEnd (C++ emits original diagnostic); value is otherwise the result. */
uint8_t OPENTTD_LIST_CALL openttd_rust_list_iter(struct OpenTTDScriptList *, uint8_t, int64_t *);
/* Filter: Remove above/below/between/equal=0..3, Keep equivalents=4..7. */
void OPENTTD_LIST_CALL openttd_rust_list_filter(struct OpenTTDScriptList *, uint8_t, int64_t, int64_t);
/* Rank: RemoveTop/RemoveBottom/KeepTop/KeepBottom=0..3. */
void OPENTTD_LIST_CALL openttd_rust_list_rank(struct OpenTTDScriptList *, uint8_t, int64_t);
/* Combine: Add/Swap/Remove/Keep/CopyContents=0..4. CopyContents retains target's
 * original sort/init flow; it is used on freshly constructed clone targets. */
void OPENTTD_LIST_CALL openttd_rust_list_combine(struct OpenTTDScriptList *, struct OpenTTDScriptList *, uint8_t);
/* ScriptList resumable control. Operations: valuation/filter/save/load/get/set/nexti = 0..6.
 * Controllers own only scalar phase state; every host operation runs after step returns. SQBool
 * uses its full 64 bits. Destruction schedules no pending host action and is safe during C++
 * unwinding. Input, output, control and list allocations are live, aligned and disjoint; scalar
 * inputs are initialized. The extra control allocation changes resource-exhaustion timing. */
struct OpenTTDListControl;
struct OpenTTDListControlInput {
	int64_t a;
	int64_t b;
	uint64_t flag;
	uint32_t kind;
};
struct OpenTTDListControlAction {
	int64_t a;
	uint32_t kind;
};
enum OpenTTDListControlActionKind {
	LC_RETURN, LC_TOP, LC_TYPE, LC_GET_INT, LC_GET_BOOL, LC_PUSH,
	LC_ROOT, LC_PUSH_INT, LC_PUSH_BOOL, LC_PUSH_NULL, LC_TAG,
	LC_NEW_ARRAY, LC_NEW_TABLE, LC_APPEND, LC_RAW_SET, LC_NEXT,
	LC_POP, LC_POP_TOP, LC_CALL, LC_CHARGE, LC_DISABLE, LC_LIMIT,
	LC_ERROR, LC_THROW_RESULT, LC_ITEM, LC_GET_PAIR, LC_LOG_NEXT, LC_LOG_END, LC_THROW_ERROR, LC_INDEX,
};
struct OpenTTDListControl *OPENTTD_LIST_CALL openttd_rust_list_control_new(uint8_t operation);
void OPENTTD_LIST_CALL openttd_rust_list_control_destroy(struct OpenTTDListControl *);
void OPENTTD_LIST_CALL openttd_rust_list_control_step(struct OpenTTDListControl *, struct OpenTTDScriptList *, const struct OpenTTDListControlInput *, struct OpenTTDListControlAction *);
#ifdef __cplusplus
}
#endif
#endif /* OPENTTD_RUST_SCRIPT_LIST_FFI_H */

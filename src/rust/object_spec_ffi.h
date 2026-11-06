/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file object_spec_ffi.h Object definition lifetime and canonical container operations. */
#pragma once
#include "call_ffi.h"
#include <cstdint>
#include <utility>
#include "rust_vector.h"
struct OpenTTDObjectSpecRecord;
struct OpenTTDObjectServices;
struct ObjectSpec;
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_object_spec_init(OpenTTDObjectSpecRecord *);
void OPENTTD_RUST_CALL openttd_rust_object_spec_drop(OpenTTDObjectSpecRecord *);
void OPENTTD_RUST_CALL openttd_rust_object_spec_clone(OpenTTDObjectSpecRecord *, const OpenTTDObjectSpecRecord *);
void OPENTTD_RUST_CALL openttd_rust_object_spec_assign(OpenTTDObjectSpecRecord *, const OpenTTDObjectSpecRecord *);
RustVector<OpenTTDObjectSpecRecord> *OPENTTD_RUST_CALL openttd_rust_object_specs();
size_t OPENTTD_RUST_CALL openttd_rust_object_spec_count();
const OpenTTDObjectSpecRecord *OPENTTD_RUST_CALL openttd_rust_object_spec_get(uint16_t);
}
extern "C" {
void OPENTTD_RUST_CALL openttd_rust_object_file_clear(RustVector<ObjectSpec *> *);
uint8_t OPENTTD_RUST_CALL openttd_rust_object_file_badge(RustVector<ObjectSpec *> *, uint32_t);
}
/** File-owned unique definitions. Moving the file transfers the one NativeVec. */
struct RustObjectFileSpecs : RustVector<ObjectSpec *> {
	RustObjectFileSpecs() : RustVector{nullptr, 0, 0} {}
	RustObjectFileSpecs(RustObjectFileSpecs &&other) noexcept : RustVector{std::exchange(other.items, nullptr), std::exchange(other.count, 0), std::exchange(other.capacity, 0)} {}
	RustObjectFileSpecs(const RustObjectFileSpecs &) = delete;
	~RustObjectFileSpecs() { openttd_rust_object_file_clear(this); }
};
struct OpenTTDByteReaderServices;
class ByteReader;
extern "C" uint8_t OPENTTD_RUST_CALL openttd_rust_object_properties(RustVector<ObjectSpec *> *, uint32_t, uint32_t, int32_t, ByteReader *, const OpenTTDByteReaderServices *, void (*)(uint16_t, ObjectSpec *, bool) noexcept, bool *, int32_t *);

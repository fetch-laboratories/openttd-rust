/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file saveload_ffi.h Rust-owned save/load codec state, buffers and length encoding. */

#ifndef RUST_SAVELOAD_FFI_H
#define RUST_SAVELOAD_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Status of a codec entry that can fail. */
typedef uint8_t OpenTTDRustSaveLoadStatus;
enum {
	OPENTTD_RUST_SL_OK = 0,
	OPENTTD_RUST_SL_CORRUPT = 1, ///< Corrupt savegame; see openttd_rust_saveload_error_message.
	OPENTTD_RUST_SL_NATIVE = 2, ///< A native frame caught an exception and keeps it.
};

/** Codec state shared in place; native code reads and writes it under the codec's serialization. */
typedef struct OpenTTDRustSaveLoadCodec {
	uint8_t *read_cursor; ///< ReadBuffer::bufp.
	uint8_t *read_end; ///< ReadBuffer::bufe.
	uint8_t *write_cursor; ///< MemoryDumper::buf.
	uint8_t *write_end; ///< MemoryDumper::bufe.
	size_t obj_len;
	size_t next_offs; ///< _next_offs.
	int32_t array_index;
	int32_t last_array_index;
	const uint16_t *version; ///< &_sl_version.
	const uint8_t *minor_version; ///< &_sl_minor_version.
	uint8_t action; ///< SaveLoadAction.
	uint8_t need_length; ///< NeedLength.
	uint8_t block_mode; ///< ChunkType.
	bool expect_table_header;
} OpenTTDRustSaveLoadCodec;

/** Native leaves; filter frames catch exceptions, keep them and return false. */
typedef struct OpenTTDSaveLoadServices {
	void (*native_assert)(bool ok);
	void (*debug_iterate_error)(void);
	bool (*read)(void *filter, uint8_t *buf, size_t len, size_t *read);
	bool (*write)(void *filter, const uint8_t *buf, size_t len);
	bool (*finish)(void *filter);
	uint32_t (*remap_old_string)(uint32_t id);
	void (*copy_from_old_name)(void *ptr, uint32_t id);
	/* Pool references: index returns obj->index; get returns T::Get(index) if T::IsValidID(index), else null. */
	void *(*cargo_packet_get)(size_t index);
	uint32_t (*cargo_packet_index)(const void *obj);
	void *(*engine_renew_get)(size_t index);
	uint32_t (*engine_renew_index)(const void *obj);
	void *(*link_graph_get)(size_t index);
	uint32_t (*link_graph_index)(const void *obj);
	void *(*link_graph_job_get)(size_t index);
	uint32_t (*link_graph_job_index)(const void *obj);
	void *(*order_list_get)(size_t index);
	uint32_t (*order_list_index)(const void *obj);
	void *(*persistent_storage_get)(size_t index);
	uint32_t (*persistent_storage_index)(const void *obj);
	void *(*road_stop_get)(size_t index);
	uint32_t (*road_stop_index)(const void *obj);
	void *(*station_get)(size_t index);
	uint32_t (*station_index)(const void *obj);
	void *(*town_get)(size_t index);
	uint32_t (*town_index)(const void *obj);
	void *(*vehicle_get)(size_t index);
	uint32_t (*vehicle_index)(const void *obj);
	const uint8_t *(*string_data)(const void *ptr, size_t *length);
	uint8_t *(*string_resize)(void *ptr, size_t length);
	void (*fix_scc_encoded)(void *ptr, bool fix_code);
	void (*fix_scc_encoded_negative)(void *ptr);
	void (*make_valid)(void *ptr, bool control, bool newline, bool tabcrlf);
} OpenTTDSaveLoadServices;

OpenTTDRustSaveLoadCodec *openttd_rust_saveload_codec(void);
const uint8_t *openttd_rust_saveload_error_message(size_t *length);

void openttd_rust_saveload_reader_open(void *filter);
void openttd_rust_saveload_reader_close(void);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_fill(const OpenTTDSaveLoadServices *services);
size_t openttd_rust_saveload_read_size(void);

void openttd_rust_saveload_dumper_open(void);
void openttd_rust_saveload_dumper_close(void);
void openttd_rust_saveload_grow(void);
size_t openttd_rust_saveload_dumper_size(void);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_flush(const OpenTTDSaveLoadServices *services, void *writer);

OpenTTDRustSaveLoadStatus openttd_rust_saveload_read_gamma(const OpenTTDSaveLoadServices *services, uint32_t *value);
void openttd_rust_saveload_write_gamma(const OpenTTDSaveLoadServices *services, size_t i);
uint32_t openttd_rust_saveload_gamma_length(size_t i);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_iterate_array(const OpenTTDSaveLoadServices *services, int32_t *index);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_skip_array(const OpenTTDSaveLoadServices *services);
void openttd_rust_saveload_set_length(const OpenTTDSaveLoadServices *services, size_t length);

OpenTTDRustSaveLoadStatus openttd_rust_saveload_read_uint16(const OpenTTDSaveLoadServices *services, uint16_t *value);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_read_uint32(const OpenTTDSaveLoadServices *services, uint32_t *value);
void openttd_rust_saveload_write_uint32(uint32_t v);
int64_t openttd_rust_saveload_read_value(const void *ptr, uint32_t conv);
void openttd_rust_saveload_write_value(const OpenTTDSaveLoadServices *services, void *ptr, uint32_t conv, int64_t val);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_conv(const OpenTTDSaveLoadServices *services, void *ptr, uint32_t conv);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_conv_file_len(const OpenTTDSaveLoadServices *services, uint32_t conv, uint8_t *length);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_copy(const OpenTTDSaveLoadServices *services, void *object, size_t length, uint32_t conv);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_array(const OpenTTDSaveLoadServices *services, void *object, size_t length, uint32_t conv);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_std_string(const OpenTTDSaveLoadServices *services, void *ptr, uint32_t conv);
size_t openttd_rust_saveload_std_string_len(const OpenTTDSaveLoadServices *services, const void *ptr);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_read_string(const OpenTTDSaveLoadServices *services, void *ptr, size_t length);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_ref(const OpenTTDSaveLoadServices *services, void *ptr, uint32_t conv);
size_t openttd_rust_saveload_ref_len(void);
OpenTTDRustSaveLoadStatus openttd_rust_saveload_array_len(const OpenTTDSaveLoadServices *services, size_t elements, uint32_t conv, size_t *length);

#ifdef __cplusplus
}
#endif

#endif /* RUST_SAVELOAD_FFI_H */

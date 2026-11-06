/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_content_ffi.h Content client leaves and Rust entries. */
#ifndef RUST_NETWORK_CONTENT_FFI_H
#define RUST_NETWORK_CONTENT_FFI_H
#include "network_tcp_ffi.h"
#include "../network/core/tcp_content_type.h"
#include <memory>
#include <vector>
struct ContentCallback;
/** Vector with content info */
using ContentVector = std::vector<std::unique_ptr<ContentInfo>>;
/** Vector with constant content info */
using ConstContentVector = std::vector<const ContentInfo *>;
enum class OpenTTDHasContent : uint8_t { NoProc, Missing, Present };
struct OpenTTDContentIdSpan {
	const ContentID *data;
	size_t size;
};
/* The native client is a process-wide shell. Leaves do not retain record pointers;
 * callbacks, socket closes and error windows may reenter the Rust entries. */
struct OpenTTDContentServices {
	void (*callback_complete)(ContentCallback *, ContentID) noexcept OPENTTD_SERVICE_DEFAULT(callback_complete);
	void (*callback_connect)(ContentCallback *, bool) noexcept OPENTTD_SERVICE_DEFAULT(callback_connect);
	void (*callback_disconnect)(ContentCallback *) noexcept OPENTTD_SERVICE_DEFAULT(callback_disconnect);
	void (*callback_progress)(ContentCallback *, const OpenTTDContentInfo *, int) noexcept OPENTTD_SERVICE_DEFAULT(callback_progress);
	void (*callback_receive)(ContentCallback *, const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(callback_receive);
	NetworkTCPSocketHandler *client;
	void (*close_socket)() noexcept OPENTTD_SERVICE_DEFAULT(close_socket);
	void (*connect)() noexcept OPENTTD_SERVICE_DEFAULT(connect);
	OpenTTDByteSpan (*content_version)() noexcept OPENTTD_SERVICE_DEFAULT(content_version);
	OpenTTDContentIdSpan (*dependencies)(const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(dependencies);
	void (*dependency_push)(OpenTTDContentInfo *, ContentID) noexcept OPENTTD_SERVICE_DEFAULT(dependency_push);
	void *(*description)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(description);
	void (*error_extract)() noexcept OPENTTD_SERVICE_DEFAULT(error_extract);
	void (*error_not_writable)() noexcept OPENTTD_SERVICE_DEFAULT(error_not_writable);
	bool (*extract)(const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(extract);
	void (*file_close)() noexcept OPENTTD_SERVICE_DEFAULT(file_close);
	bool (*file_create)(const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(file_create);
	bool (*file_open)() noexcept OPENTTD_SERVICE_DEFAULT(file_open);
	size_t (*file_write)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(file_write);
	void *(*filename)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(filename);
	OpenTTDHasContent (*has_content)(const OpenTTDContentInfo *, bool) noexcept OPENTTD_SERVICE_DEFAULT(has_content);
	void (*http_get)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(http_get);
	void (*http_mirror)(OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(http_mirror);
	void (*info_assign)(OpenTTDContentInfo *, const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(info_assign);
	void (*info_delete)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(info_delete);
	OpenTTDContentInfo *(*info_new)() noexcept OPENTTD_SERVICE_DEFAULT(info_new);
	OpenTTDContentInfo *(*list_get)(ContentVector *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(list_get);
	OpenTTDContentInfo *(*list_release)(ContentVector *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(list_release);
	size_t (*list_size)(const ContentVector *) noexcept OPENTTD_SERVICE_DEFAULT(list_size);
	void *(*name)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*queue_reset)() noexcept OPENTTD_SERVICE_DEFAULT(queue_reset);
	void (*string_assign)(void *, const void *) noexcept OPENTTD_SERVICE_DEFAULT(string_assign);
	bool (*string_empty)(const void *) noexcept OPENTTD_SERVICE_DEFAULT(string_empty);
	void *(*tag)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(tag);
	const OpenTTDNetworkTcpServices *tcp;
	void (*tcp_close)() noexcept OPENTTD_SERVICE_DEFAULT(tcp_close);
	void (*tree_push)(ConstContentVector *, const OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(tree_push);
	void *(*url)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(url);
	void *(*version)(OpenTTDContentInfo *) noexcept OPENTTD_SERVICE_DEFAULT(version);
};
const OpenTTDContentServices &GetRustContentServices() noexcept;
extern "C" {
void openttd_rust_content_add_callback(ContentCallback *) noexcept;
void openttd_rust_content_cancel(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_clear(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_close(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_connect(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_connected(const OpenTTDContentServices *, bool) noexcept;
void openttd_rust_content_download(const OpenTTDContentServices *, uint32_t *, uint32_t *, bool) noexcept;
uint8_t openttd_rust_content_http_cancelled() noexcept;
void openttd_rust_content_http_data(const OpenTTDContentServices *, const uint8_t *, size_t) noexcept;
void openttd_rust_content_http_failure(const OpenTTDContentServices *) noexcept;
OpenTTDContentInfo *const *openttd_rust_content_infos(size_t *) noexcept;
void openttd_rust_content_init(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_remove_callback(ContentCallback *) noexcept;
void openttd_rust_content_request_ids(const OpenTTDContentServices *, const ContentID *, size_t) noexcept;
void openttd_rust_content_request_queued(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_request_type(const OpenTTDContentServices *, ContentType) noexcept;
void openttd_rust_content_request_vector(const OpenTTDContentServices *, ContentVector *, bool) noexcept;
void openttd_rust_content_select(const OpenTTDContentServices *, ContentID) noexcept;
void openttd_rust_content_select_all(const OpenTTDContentServices *, bool) noexcept;
void openttd_rust_content_send_receive(const OpenTTDContentServices *) noexcept;
void openttd_rust_content_toggle(const OpenTTDContentServices *, const OpenTTDContentInfo *) noexcept;
void openttd_rust_content_tree(const OpenTTDContentServices *, ConstContentVector *, const OpenTTDContentInfo *) noexcept;
void openttd_rust_content_unselect(const OpenTTDContentServices *, ContentID) noexcept;
void openttd_rust_content_unselect_all() noexcept;
}
#endif /* RUST_NETWORK_CONTENT_FFI_H */

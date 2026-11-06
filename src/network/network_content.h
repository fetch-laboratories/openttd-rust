/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_content.h Part of the network protocol handling content distribution. */

#ifndef NETWORK_CONTENT_H
#define NETWORK_CONTENT_H

#include <ranges>
#include "core/tcp_content.h"
#include "core/http.h"
#include "../core/container_func.hpp"
#include "../rust/network_content_ffi.h"

/** Callbacks for notifying others about incoming data */
struct ContentCallback {
	/**
	 * Callback for when the connection has finished
	 * @param success whether the connection was made or that we failed to make it
	 */
	virtual void OnConnect([[maybe_unused]] bool success) {}

	/**
	 * Callback for when the connection got disconnected.
	 */
	virtual void OnDisconnect() {}

	/**
	 * We received a content info.
	 * @param ci the content info
	 */
	virtual void OnReceiveContentInfo([[maybe_unused]] const ContentInfo &ci) {}

	/**
	 * We have progress in the download of a file
	 * @param ci the content info of the file
	 * @param bytes the number of bytes downloaded since the previous call
	 */
	virtual void OnDownloadProgress([[maybe_unused]] const ContentInfo &ci, [[maybe_unused]] int bytes) {}

	/**
	 * We have finished downloading a file
	 * @param cid the ContentID of the downloaded file
	 */
	virtual void OnDownloadComplete([[maybe_unused]] ContentID cid) {}

	/** Silentium */
	virtual ~ContentCallback() = default;
};

/**
 * Socket handler for the content server connection; Rust owns its metadata, dependency and download state.
 */
class ClientNetworkContentSocketHandler : public NetworkContentSocketHandler, HTTPCallback {
protected:
	std::optional<FileHandle> cur_file; ///< Currently downloaded file

	friend class NetworkContentConnecter;
	friend const OpenTTDContentServices &GetRustContentServices() noexcept;

	void OnFailure() override;
	void OnReceiveData(std::unique_ptr<char[]> data, size_t length) override;
	bool IsCancelled() const override;

public:
	ClientNetworkContentSocketHandler();

	void Connect();
	void SendReceive();
	NetworkRecvStatus CloseConnection(bool error = true) override;
	void Cancel();

	void RequestContentList(ContentType type);
	void RequestContentList(std::span<const ContentID> content_ids);
	void RequestContentList(ContentVector *cv, bool send_md5sum = true);

	void DownloadSelectedContent(uint &files, uint &bytes, bool fallback = false);
	void RequestQueuedContentInfo();

	void Select(ContentID cid);
	void Unselect(ContentID cid);
	void SelectAll();
	void SelectUpgrade();
	void UnselectAll();
	void ToggleSelectedState(const ContentInfo &ci);

	void ReverseLookupTreeDependency(ConstContentVector &tree, const ContentInfo *child) const;

	/**
	 * Get a read-only view of content info for iterating externally.
	 * @return Read-only view of content info.
	 */
	auto Info() const
	{
		size_t count;
		OpenTTDContentInfo *const *infos = openttd_rust_content_infos(&count);
		return std::span(infos, count) | std::views::transform([](const OpenTTDContentInfo *ci) -> const ContentInfo & { return static_cast<const ContentInfo &>(*ci); });
	}

	void Clear();

	/** Add a callback to this class */
	void AddCallback(ContentCallback *cb) { openttd_rust_content_add_callback(cb); }
	/** Remove a callback */
	void RemoveCallback(ContentCallback *cb) { openttd_rust_content_remove_callback(cb); }
};

extern ClientNetworkContentSocketHandler _network_content_client;

void ShowNetworkContentListWindow(ContentVector *cv = nullptr, ContentType type1 = CONTENT_TYPE_END, ContentType type2 = CONTENT_TYPE_END);

void ShowMissingContentWindow(const GRFConfigList &list);

#endif /* NETWORK_CONTENT_H */

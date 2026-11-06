/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_content.cpp Content sending/receiving part of the network protocol. */

#include "../stdafx.h"
#include "../rev.h"
#include "../ai/ai.hpp"
#include "../game/game.hpp"
#include "../window_func.h"
#include "../error.h"
#include "../fileio_func.h"
#include "../base_media_base.h"
#include "../base_media_graphics.h"
#include "../base_media_music.h"
#include "../base_media_sounds.h"
#include "../settings_type.h"
#include "../strings_func.h"
#include "../timer/timer.h"
#include "../timer/timer_window.h"
#include "network_content.h"
#include "../rust/services_assert.h"
#include "../rust/string_services.h"

#include "table/strings.h"

#if defined(WITH_ZLIB)
#	include <zlib.h>
#	if defined(_WIN32)
		/* Required for: dup, fileno, close */
#		include <io.h>
#	endif
#endif

#ifdef __EMSCRIPTEN__
#	include <emscripten.h>
#endif

#include "../safeguards.h"

extern bool HasScenario(const ContentInfo &ci, bool md5sum);

/** The client we use to connect to the server. */
ClientNetworkContentSocketHandler _network_content_client;

static const OpenTTDContentServices &S() noexcept { return GetRustContentServices(); }

/** Wrapper function for the HasProc */
static bool HasGRFConfig(const ContentInfo &ci, bool md5sum)
{
	return FindGRFConfig(std::byteswap(ci.unique_id), md5sum ? FGCM_EXACT : FGCM_ANY, md5sum ? &ci.md5sum : nullptr) != nullptr;
}

/**
 * Check whether a function piece of content is locally known.
 * Matches on the unique ID and possibly the MD5 checksum.
 * @param ci     the content info to search for
 * @param md5sum also match the MD5 checksum?
 * @return true iff it's known
 */
using HasContentProc = bool(const ContentInfo &ci, bool md5sum);

/**
 * Get the has-content check function for the given content type.
 * @param type Content type to get check function for.
 * @return Check function pointer.
 */
static HasContentProc *GetHasContentProcforContentType(ContentType type)
{
	switch (type) {
		case CONTENT_TYPE_NEWGRF: return HasGRFConfig;
		case CONTENT_TYPE_BASE_GRAPHICS: return BaseGraphics::HasSet;
		case CONTENT_TYPE_BASE_MUSIC: return BaseMusic::HasSet;
		case CONTENT_TYPE_BASE_SOUNDS: return BaseSounds::HasSet;
		case CONTENT_TYPE_AI: return AI::HasAI;
		case CONTENT_TYPE_AI_LIBRARY: return AI::HasAILibrary;
		case CONTENT_TYPE_GAME: return Game::HasGame;
		case CONTENT_TYPE_GAME_LIBRARY: return Game::HasGameLibrary;
		case CONTENT_TYPE_SCENARIO: return HasScenario;
		case CONTENT_TYPE_HEIGHTMAP: return HasScenario;
		default: return nullptr;
	}
}

/**
 * Determine the full filename of a piece of content information
 * @param ci         the information to get the filename from
 * @param compressed should the filename end with .gz?
 * @return a statically allocated buffer with the filename or
 *         nullptr when no filename could be made.
 */
static std::string GetFullFilename(const ContentInfo &ci, bool compressed)
{
	Subdirectory dir = GetContentInfoSubDir(ci.type);
	if (dir == NO_DIRECTORY) return {};

	std::string buf = FioGetDirectory(SP_AUTODOWNLOAD_DIR, dir);
	buf += ci.filename;
	buf += compressed ? ".tar.gz" : ".tar";

	return buf;
}

/**
 * Gunzip a given file and remove the .gz if successful.
 * @param ci container with filename
 * @return true if the gunzip completed
 */
static bool GunzipFile(const ContentInfo &ci)
{
#if defined(WITH_ZLIB)
	bool ret = true;

	/* Need to open the file with fopen() to support non-ASCII on Windows. */
	auto ftmp = FileHandle::Open(GetFullFilename(ci, true), "rb");
	if (!ftmp.has_value()) return false;
	/* Duplicate the handle, and close the FILE*, to avoid double-closing the handle later. */
	int fdup = dup(fileno(*ftmp));
	gzFile fin = gzdopen(fdup, "rb");
	ftmp.reset();

	auto fout = FileHandle::Open(GetFullFilename(ci, false), "wb");

	if (fin == nullptr || !fout.has_value()) {
		ret = false;
	} else {
		uint8_t buff[8192];
		for (;;) {
			int read = gzread(fin, buff, sizeof(buff));
			if (read == 0) {
				/* If gzread() returns 0, either the end-of-file has been
				 * reached or an underlying read error has occurred.
				 *
				 * gzeof() can't be used, because:
				 * 1.2.5 - it is safe, 1 means 'everything was OK'
				 * 1.2.3.5, 1.2.4 - 0 or 1 is returned 'randomly'
				 * 1.2.3.3 - 1 is returned for truncated archive
				 *
				 * So we use gzerror(). When proper end of archive
				 * has been reached, then:
				 * errnum == Z_STREAM_END in 1.2.3.3,
				 * errnum == 0 in 1.2.4 and 1.2.5 */
				int errnum;
				gzerror(fin, &errnum);
				if (errnum != 0 && errnum != Z_STREAM_END) ret = false;
				break;
			}
			if (read < 0 || static_cast<size_t>(read) != fwrite(buff, 1, read, *fout)) {
				/* If gzread() returns -1, there was an error in archive */
				ret = false;
				break;
			}
			/* DO NOT DO THIS! It will fail to detect broken archive with 1.2.3.3!
			 * if (read < sizeof(buff)) break; */
		}
	}

	if (fin != nullptr) {
		gzclose(fin);
	} else if (fdup != -1) {
		/* Failing gzdopen does not close the passed file descriptor. */
		close(fdup);
	}

	return ret;
#else
	NOT_REACHED();
#endif /* defined(WITH_ZLIB) */
}

/**
 * Gunzip the downloaded file, make its tar known and extract music.
 * @param ci The downloaded content.
 * @return Whether the archive was extracted.
 */
static bool ExtractDownload(const ContentInfo &ci)
{
	if (!GunzipFile(ci)) return false;
	FioRemove(GetFullFilename(ci, true));

	Subdirectory sd = GetContentInfoSubDir(ci.type);
	if (sd == NO_DIRECTORY) NOT_REACHED();

	TarScanner ts;
	std::string fname = GetFullFilename(ci, false);
	ts.AddFile(sd, fname);

	if (ci.type == CONTENT_TYPE_BASE_MUSIC) {
		/* Music can't be in a tar. So extract the tar! */
		ExtractTar(fname, BASESET_DIR);
		FioRemove(fname);
	}

#ifdef __EMSCRIPTEN__
	EM_ASM(if (window["openttd_syncfs"]) openttd_syncfs());
#endif
	return true;
}

/** Close the download window and report an unwritable file. */
static void ShowNotWritable() noexcept
{
	CloseWindowById(WC_NETWORK_STATUS_WINDOW, WN_NETWORK_STATUS_WINDOW_CONTENT_DOWNLOAD);
	ShowErrorMessage(
		GetEncodedString(STR_CONTENT_ERROR_COULD_NOT_DOWNLOAD),
		GetEncodedString(STR_CONTENT_ERROR_COULD_NOT_DOWNLOAD_FILE_NOT_WRITABLE),
		WL_ERROR);
}

ClientNetworkContentSocketHandler::ClientNetworkContentSocketHandler()
{
	openttd_rust_content_init(&S());
}

bool ClientNetworkContentSocketHandler::IsCancelled() const
{
	return openttd_rust_content_http_cancelled() != 0;
}

void ClientNetworkContentSocketHandler::OnFailure()
{
	openttd_rust_content_http_failure(&S());
}

void ClientNetworkContentSocketHandler::OnReceiveData(std::unique_ptr<char[]> data, size_t length)
{
	openttd_rust_content_http_data(&S(), reinterpret_cast<const uint8_t *>(data.get()), length);
}

/** Connect to the content server. */
class NetworkContentConnecter : public TCPConnecter {
public:
	/**
	 * Initiate the connecting.
	 * @param address The address of the server.
	 */
	NetworkContentConnecter(std::string_view connection_string) : TCPConnecter(connection_string, NETWORK_CONTENT_SERVER_PORT) {}

	void OnFailure() override
	{
		openttd_rust_content_connected(&S(), false);
	}

	void OnConnect(SOCKET s) override
	{
		assert(_network_content_client.Socket() == INVALID_SOCKET);
		_network_content_client.SetSocket(s);
		_network_content_client.Reopen();
		openttd_rust_content_connected(&S(), true);
	}
};

void ClientNetworkContentSocketHandler::Connect() { openttd_rust_content_connect(&S()); }
void ClientNetworkContentSocketHandler::SendReceive() { openttd_rust_content_send_receive(&S()); }
void ClientNetworkContentSocketHandler::Cancel() { openttd_rust_content_cancel(&S()); }
void ClientNetworkContentSocketHandler::RequestContentList(ContentType type) { openttd_rust_content_request_type(&S(), type); }
void ClientNetworkContentSocketHandler::RequestContentList(std::span<const ContentID> content_ids) { openttd_rust_content_request_ids(&S(), content_ids.data(), content_ids.size()); }
void ClientNetworkContentSocketHandler::RequestContentList(ContentVector *cv, bool send_md5sum) { openttd_rust_content_request_vector(&S(), cv, send_md5sum); }
void ClientNetworkContentSocketHandler::DownloadSelectedContent(uint &files, uint &bytes, bool fallback) { openttd_rust_content_download(&S(), &files, &bytes, fallback); }
void ClientNetworkContentSocketHandler::RequestQueuedContentInfo() { openttd_rust_content_request_queued(&S()); }
void ClientNetworkContentSocketHandler::Select(ContentID cid) { openttd_rust_content_select(&S(), cid); }
void ClientNetworkContentSocketHandler::Unselect(ContentID cid) { openttd_rust_content_unselect(&S(), cid); }
void ClientNetworkContentSocketHandler::SelectAll() { openttd_rust_content_select_all(&S(), false); }
void ClientNetworkContentSocketHandler::SelectUpgrade() { openttd_rust_content_select_all(&S(), true); }
void ClientNetworkContentSocketHandler::UnselectAll() { openttd_rust_content_unselect_all(); }
void ClientNetworkContentSocketHandler::ToggleSelectedState(const ContentInfo &ci) { openttd_rust_content_toggle(&S(), &ci); }
void ClientNetworkContentSocketHandler::ReverseLookupTreeDependency(ConstContentVector &tree, const ContentInfo *child) const { openttd_rust_content_tree(&S(), &tree, child); }
void ClientNetworkContentSocketHandler::Clear() { openttd_rust_content_clear(&S()); }

NetworkRecvStatus ClientNetworkContentSocketHandler::CloseConnection(bool)
{
	openttd_rust_content_close(&S());
	return NETWORK_RECV_STATUS_OKAY;
}

/** Timeout after queueing content for it to try to be requested. */
static constexpr auto CONTENT_QUEUE_TIMEOUT = std::chrono::milliseconds(100);

static TimeoutTimer<TimerWindow> _request_queue_timeout = {CONTENT_QUEUE_TIMEOUT, []() {
	_network_content_client.RequestQueuedContentInfo();
}};

static ContentInfo &Info(OpenTTDContentInfo *ci) noexcept { return static_cast<ContentInfo &>(*ci); }

const OpenTTDContentServices &GetRustContentServices() noexcept
{
	static ClientNetworkContentSocketHandler &c = _network_content_client;
	static const OpenTTDContentServices services{
		.callback_complete = [](ContentCallback *cb, ContentID cid) noexcept { cb->OnDownloadComplete(cid); },
		.callback_connect = [](ContentCallback *cb, bool success) noexcept { cb->OnConnect(success); },
		.callback_disconnect = [](ContentCallback *cb) noexcept { cb->OnDisconnect(); },
		.callback_progress = [](ContentCallback *cb, const OpenTTDContentInfo *ci, int bytes) noexcept { cb->OnDownloadProgress(static_cast<const ContentInfo &>(*ci), bytes); },
		.callback_receive = [](ContentCallback *cb, const OpenTTDContentInfo *ci) noexcept { cb->OnReceiveContentInfo(static_cast<const ContentInfo &>(*ci)); },
		.client = _network_content_client.TcpRecord(),
		.close_socket = []() noexcept { c.CloseSocket(); },
		.connect = []() noexcept { TCPConnecter::Create<NetworkContentConnecter>(NetworkContentServerConnectionString()); },
		.content_version = []() noexcept { return OpenTTDByteSpan{reinterpret_cast<const uint8_t *>(_openttd_content_version.data()), _openttd_content_version.size()}; },
		.dependencies = [](const OpenTTDContentInfo *ci) noexcept { auto &d = static_cast<const ContentInfo *>(ci)->dependencies; return OpenTTDContentIdSpan{d.data(), d.size()}; },
		.dependency_push = [](OpenTTDContentInfo *ci, ContentID id) noexcept { Info(ci).dependencies.push_back(id); },
		.description = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).description; },
		.error_extract = []() noexcept { ShowErrorMessage(GetEncodedString(STR_CONTENT_ERROR_COULD_NOT_EXTRACT), {}, WL_ERROR); },
		.error_not_writable = ShowNotWritable,
		.extract = [](const OpenTTDContentInfo *ci) noexcept { return ExtractDownload(static_cast<const ContentInfo &>(*ci)); },
		.file_close = []() noexcept { c.cur_file.reset(); },
		.file_create = [](const OpenTTDContentInfo *ci) noexcept {
			std::string filename = GetFullFilename(static_cast<const ContentInfo &>(*ci), true);
			return !filename.empty() && (c.cur_file = FileHandle::Open(filename, "wb")).has_value();
		},
		.file_open = []() noexcept { return c.cur_file.has_value(); },
		.file_write = [](const uint8_t *data, size_t length) noexcept { return fwrite(data, 1, length, *c.cur_file); },
		.filename = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).filename; },
		.has_content = [](const OpenTTDContentInfo *ci, bool md5sum) noexcept {
			HasContentProc *proc = GetHasContentProcforContentType(ci->type);
			if (proc == nullptr) return OpenTTDHasContent::NoProc;
			return proc(static_cast<const ContentInfo &>(*ci), md5sum) ? OpenTTDHasContent::Present : OpenTTDHasContent::Missing;
		},
		.http_get = [](OpenTTDByteSpan url) noexcept { NetworkHTTPSocketHandler::Connect({reinterpret_cast<const char *>(url.data), url.size}, &c); },
		.http_mirror = [](OpenTTDByteSpan request) noexcept { NetworkHTTPSocketHandler::Connect(NetworkContentMirrorUriString(), &c, std::string(reinterpret_cast<const char *>(request.data), request.size)); },
		.info_assign = [](OpenTTDContentInfo *to, const OpenTTDContentInfo *from) noexcept { Info(to) = static_cast<const ContentInfo &>(*from); },
		.info_delete = [](OpenTTDContentInfo *ci) noexcept { delete &Info(ci); },
		.info_new = []() noexcept -> OpenTTDContentInfo * { return new ContentInfo(); },
		.list_get = [](ContentVector *cv, size_t i) noexcept -> OpenTTDContentInfo * { return (*cv)[i].get(); },
		.list_release = [](ContentVector *cv, size_t i) noexcept -> OpenTTDContentInfo * { return (*cv)[i].release(); },
		.list_size = [](const ContentVector *cv) noexcept { return cv->size(); },
		.name = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).name; },
		.queue_reset = []() noexcept { _request_queue_timeout.Reset(); },
		.string_assign = [](void *to, const void *from) noexcept { *static_cast<std::string *>(to) = *static_cast<const std::string *>(from); },
		.string_empty = [](const void *text) noexcept { return static_cast<const std::string *>(text)->empty(); },
		.tag = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).tags.emplace_back(); },
		.tcp = &GetRustNetworkTcpServices(),
		.tree_push = [](ConstContentVector *tree, const OpenTTDContentInfo *ci) noexcept { tree->push_back(static_cast<const ContentInfo *>(ci)); },
		.url = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).url; },
		.version = [](OpenTTDContentInfo *ci) noexcept -> void * { return &Info(ci).version; },
	};
	return services;
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDContentServices.hpp"
bool CheckAbi_network_content() { return AbiServicesComplete(GetRustContentServices()); }

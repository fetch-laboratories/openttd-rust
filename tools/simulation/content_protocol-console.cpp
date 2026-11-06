/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file content_protocol-console.cpp Content client steps, callbacks and state against a local server. */
#include "stdafx.h"
#include "console_internal.h"
#include "network/network_content.h"
#include "network/network_func.h"
#include "timer/timer_manager.h"
#include "timer/timer_window.h"
#include <chrono>
#include <thread>

static std::string Hex(std::string_view text)
{
	std::string out = text.empty() ? "-" : "";
	for (unsigned char c : text) out += fmt::format("{:02x}", c);
	return out;
}

static std::string Dump(const ContentInfo &ci)
{
	std::string deps, tags;
	for (ContentID d : ci.dependencies) deps += fmt::format("{},", d);
	for (const std::string &tag : ci.tags) tags += Hex(tag) + ",";
	std::string md5;
	for (uint8_t b : ci.md5sum) md5 += fmt::format("{:02x}", b);
	return fmt::format("{} type={} state={} upgrade={} size={} uid={:08x} md5={} name={} version={} url={} desc={} file={} deps={} tags={}",
		ci.id, static_cast<int>(ci.type), static_cast<int>(ci.state), ci.upgrade, ci.filesize, ci.unique_id, md5,
		Hex(ci.name), Hex(ci.version), Hex(ci.url), Hex(ci.description), Hex(ci.filename), deps, tags);
}

enum Event : uint8_t { CONNECT, FAIL, DISCONNECT, INFO, PROGRESS, COMPLETE, EVENTS };
static const std::string_view EVENT_NAMES[EVENTS] = {"connect", "fail", "disconnect", "info", "progress", "complete"};
static int _events[EVENTS];
/* Each wait counts events since the previous wait on that event or the last mark. */
static int _seen[EVENTS];

struct ProbeCallback : ContentCallback {
	void OnConnect(bool success) override
	{
		_events[success ? CONNECT : FAIL]++;
		fmt::print(stderr, "CONTENT event connect {}\n", success);
	}
	void OnDisconnect() override
	{
		_events[DISCONNECT]++;
		fmt::print(stderr, "CONTENT event disconnect\n");
	}
	void OnReceiveContentInfo(const ContentInfo &ci) override
	{
		_events[INFO]++;
		fmt::print(stderr, "CONTENT event info {}\n", Dump(ci));
	}
	void OnDownloadProgress(const ContentInfo &ci, int bytes) override
	{
		_events[PROGRESS]++;
		/* HTTP chunk sizes follow transport segmentation; only the stream's failure marker is ordered. */
		fmt::print(stderr, "CONTENT progress {} {}\n", ci.id, bytes);
		if (bytes < 0) fmt::print(stderr, "CONTENT event progress-failed {}\n", ci.id);
	}
	void OnDownloadComplete(ContentID cid) override
	{
		_events[COMPLETE]++;
		fmt::print(stderr, "CONTENT event complete {}\n", cid);
	}
};

static void Pump()
{
	NetworkBackgroundLoop();
	TimerManager<TimerWindow>::Elapsed(std::chrono::milliseconds(10));
	std::this_thread::sleep_for(std::chrono::milliseconds(10));
}

static bool Wait(std::string_view name, int count, int timeout_ms)
{
	size_t event = 0;
	while (event < EVENTS && EVENT_NAMES[event] != name) event++;
	if (event == EVENTS) return false;
	auto end = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
	while (_events[event] - _seen[event] < count && std::chrono::steady_clock::now() < end) Pump();
	fmt::print(stderr, "CONTENT {} {} {}\n", _events[event] - _seen[event] >= count ? "waited" : "timeout", name, count);
	_seen[event] = _events[event];
	return true;
}

static void Idle(int ms)
{
	auto end = std::chrono::steady_clock::now() + std::chrono::milliseconds(ms);
	while (std::chrono::steady_clock::now() < end) Pump();
}

static ContentID Id(std::string_view text)
{
	return ParseInteger<ContentID>(text).value_or(INVALID_CONTENT_ID);
}

static const ContentInfo *Find(ContentID id)
{
	for (const ContentInfo &ci : _network_content_client.Info()) {
		if (ci.id == id) return &ci;
	}
	return nullptr;
}

/** Parse "type:uid:md5hex" stubs for the external-ID requests. */
static std::unique_ptr<ContentInfo> Stub(std::string_view text)
{
	auto ci = std::make_unique<ContentInfo>();
	auto first = text.find(':'), second = text.rfind(':');
	ci->type = static_cast<ContentType>(ParseInteger<uint>(text.substr(0, first)).value_or(0));
	ci->unique_id = ParseInteger<uint32_t>(text.substr(first + 1, second - first - 1), 16).value_or(0);
	std::string_view md5 = text.substr(second + 1);
	for (size_t i = 0; i < ci->md5sum.size() && 2 * i + 1 < md5.size(); i++) {
		ci->md5sum[i] = ParseInteger<uint8_t>(md5.substr(2 * i, 2), 16).value_or(0);
	}
	return ci;
}

static bool ContentProbe(std::span<std::string_view> args)
{
	static ProbeCallback probe;
	if (args.size() < 2) return false;
	std::string_view op = args[1];
	fmt::print(stderr, "CONTENT step {}\n", fmt::join(args.subspan(1), " "));
	if (op == "listen") {
		_network_content_client.AddCallback(&probe);
	} else if (op == "wait" && args.size() == 4) {
		return Wait(args[2], ParseInteger<int>(args[3]).value_or(0), 10000);
	} else if (op == "mark") {
		std::copy(std::begin(_events), std::end(_events), std::begin(_seen));
	} else if (op == "idle" && args.size() == 3) {
		Idle(ParseInteger<int>(args[2]).value_or(0));
	} else if (op == "update" && args.size() == 3) {
		_network_content_client.RequestContentList(static_cast<ContentType>(ParseInteger<uint>(args[2]).value_or(0)));
	} else if (op == "ids") {
		std::vector<ContentID> ids;
		for (auto arg : args.subspan(2)) ids.push_back(Id(arg));
		_network_content_client.RequestContentList(ids);
	} else if (op == "extid" && args.size() >= 3) {
		ContentVector cv;
		for (auto arg : args.subspan(3)) cv.push_back(Stub(arg));
		_network_content_client.RequestContentList(&cv, args[2] == "1");
		for (const auto &ci : cv) fmt::print(stderr, "CONTENT kept {}\n", ci == nullptr ? std::string("moved") : Dump(*ci));
	} else if (op == "select" && args.size() == 3) {
		_network_content_client.Select(Id(args[2]));
	} else if (op == "unselect" && args.size() == 3) {
		_network_content_client.Unselect(Id(args[2]));
	} else if (op == "toggle" && args.size() == 3) {
		if (const ContentInfo *ci = Find(Id(args[2])); ci != nullptr) _network_content_client.ToggleSelectedState(*ci);
	} else if (op == "selectall") {
		_network_content_client.SelectAll();
	} else if (op == "upgrade") {
		_network_content_client.SelectUpgrade();
	} else if (op == "unselectall") {
		_network_content_client.UnselectAll();
	} else if (op == "tree" && args.size() == 3) {
		const ContentInfo *ci = Find(Id(args[2]));
		if (ci == nullptr) return false;
		ConstContentVector tree;
		_network_content_client.ReverseLookupTreeDependency(tree, ci);
		std::string ids;
		for (const ContentInfo *t : tree) ids += fmt::format(" {}", t->id);
		fmt::print(stderr, "CONTENT tree {}:{}\n", ci->id, ids);
	} else if (op == "state") {
		for (const ContentInfo &ci : _network_content_client.Info()) fmt::print(stderr, "CONTENT state {}\n", Dump(ci));
		fmt::print(stderr, "CONTENT state-end\n");
	} else if (op == "download" && args.size() == 3) {
		uint files, bytes;
		_network_content_client.DownloadSelectedContent(files, bytes, args[2] == "1");
		fmt::print(stderr, "CONTENT downloading {} {}\n", files, bytes);
	} else if (op == "cancel") {
		_network_content_client.Cancel();
	} else if (op == "close") {
		_network_content_client.CloseConnection();
	} else if (op == "connect") {
		_network_content_client.Connect();
	} else if (op == "clear") {
		_network_content_client.Clear();
	} else {
		return false;
	}
	return true;
}

extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("content_probe", ContentProbe);
}

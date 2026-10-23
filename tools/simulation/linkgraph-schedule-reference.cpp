/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph-schedule-reference.cpp Controlled workers for otherwise unreachable scheduler branches. */
#include <algorithm>
#include <array>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <list>
#include <optional>
#include <vector>
#include "rust/linkgraph_schedule_ffi.h"
#define NOT_REACHED() std::abort()
static std::vector<std::array<int, 3>> trace;
struct Date { int32_t value; int32_t base() const { return value; } bool operator==(int value) const { return this->value == value; } };
struct TimerGameEconomy { using Date = ::Date; inline static Date date{0}; inline static uint16_t date_fract = 0; };
namespace EconomyTime { static constexpr int SECONDS_PER_DAY = 2; }
static struct { struct { uint16_t recalc_interval = 4; } linkgraph; } _settings_game;
static bool _networking, _network_server, candidate;
enum class PauseMode { LinkGraph = 1 };
static struct {
	int bits = 0;
	bool Test(PauseMode) const { return this->bits & 1; }
	bool None() const { return this->bits == 0; }
	void Set(PauseMode) { this->bits |= 1; trace.push_back({8, 1, 0}); }
} _pause_mode;
static constexpr int CMD_PAUSE = 0, PFE_GL_LINKGRAPH = 0;
template <int> struct Command { static void Post(PauseMode, bool paused) { trace.push_back({7, paused, 0}); } };
struct PerformanceMeasurer {
	explicit PerformanceMeasurer(int) { trace.push_back({9, 1, 0}); }
	~PerformanceMeasurer() { trace.push_back({9, 2, 0}); }
	static void SetInactive(int) { trace.push_back({9, 0, 0}); }
};
using LinkGraphID = uint16_t;
static std::array<LinkGraph *, 4> graphs;
static std::array<LinkGraphJob *, 4> jobs;
static std::array<int, 2> QueueSizes();
static void Recycle();
class LinkGraph {
public:
	uint16_t index, size, tag;
	uint16_t Size() const { trace.push_back({1, this->tag, 0}); return this->size; }
	static LinkGraph *Get(uint16_t id) { return graphs[id]; }
	static bool IsValidID(uint16_t id) { return graphs[id] != nullptr; }
	static std::vector<LinkGraph *> Iterate() { std::vector<LinkGraph *> out; for (auto *g : graphs) if (g != nullptr) out.push_back(g); return out; }
	void ShiftDates(Date interval) { trace.push_back({5, this->tag, interval.base()}); }
};
static bool recycle;
class LinkGraphJob {
public:
	uint16_t index, graph;
	bool due = false, completed = false;
	explicit LinkGraphJob(const LinkGraph &g) : index(0), graph(g.index) { while (jobs[this->index] != nullptr) ++this->index; jobs[this->index] = this; trace.push_back({2, this->index, g.tag}); }
	~LinkGraphJob() { trace.push_back({3, this->index, this->graph}); jobs[this->index] = nullptr; if (recycle) Recycle(); }
	static bool CanAllocateItem() { return true; }
	static std::vector<LinkGraphJob *> Iterate() { std::vector<LinkGraphJob *> out; for (auto *j : jobs) if (j != nullptr) out.push_back(j); return out; }
	void SpawnThread() { trace.push_back({4, this->index, 0}); this->completed = true; }
	void AbortJob() { auto sizes = QueueSizes(); trace.push_back({6, sizes[0], sizes[1]}); }
	void ShiftJoinDate(Date interval) { trace.push_back({10, this->index, interval.base()}); }
	bool IsScheduledToBeJoined() const { trace.push_back({11, this->index, this->due}); return this->due; }
	bool IsJobCompleted() const { trace.push_back({12, this->index, this->completed}); return this->completed; }
	uint16_t LinkGraphIndex() const { return this->graph; }
};
namespace reference {
class LinkGraphSchedule {
public:
	inline static const uint16_t SPAWN_JOIN_TICK = 21;
	static LinkGraphSchedule instance;
	std::list<LinkGraph *> schedule;
	std::list<LinkGraphJob *> running;
	void SpawnNext(); bool IsJoinWithUnfinishedJobDue() const; void JoinNext(); void SpawnAll(); static void Clear(); void ShiftDates(Date interval);
#include "scheduler-queue-original.inc"
};
LinkGraphSchedule LinkGraphSchedule::instance;
#include "scheduler-original.inc"
}
static const OpenTTDScheduleServices services{
	.graph_size = [](const LinkGraph *g) noexcept { return g->Size(); },
	.assert_graph = [](const LinkGraph *g) noexcept { assert(g == LinkGraph::Get(g->index)); },
	.can_allocate = []() noexcept { return LinkGraphJob::CanAllocateItem(); },
	.allocation_failure = []() noexcept { NOT_REACHED(); },
	.new_job = [](const LinkGraph *g) noexcept { return new LinkGraphJob(*g); },
	.spawn = [](LinkGraphJob *j) noexcept { j->SpawnThread(); },
	.due = [](const LinkGraphJob *j) noexcept { return j->IsScheduledToBeJoined(); },
	.completed = [](const LinkGraphJob *j) noexcept { return j->IsJobCompleted(); },
	.graph_index = [](const LinkGraphJob *j) noexcept { return j->LinkGraphIndex(); },
	.destroy_job = [](LinkGraphJob *j) noexcept { delete j; },
	.graph_valid = [](uint16_t id) noexcept { return LinkGraph::IsValidID(id); },
	.graph_get = [](uint16_t id) noexcept { return LinkGraph::Get(id); },
	.abort_job = [](LinkGraphJob *j) noexcept { j->AbortJob(); },
	.next_graph = [](uint32_t *from) noexcept -> LinkGraph * { while (*from < graphs.size()) { auto *g = graphs[(*from)++]; if (g != nullptr) return g; } return nullptr; },
	.next_job = [](uint32_t *from) noexcept -> LinkGraphJob * { while (*from < jobs.size()) { auto *j = jobs[(*from)++]; if (j != nullptr) return j; } return nullptr; },
	.shift_graph = [](LinkGraph *g, int32_t interval) noexcept { g->ShiftDates({interval}); },
	.shift_job = [](LinkGraphJob *j, int32_t interval) noexcept { j->ShiftJoinDate({interval}); },
	.post_pause = [](bool paused) noexcept { Command<CMD_PAUSE>::Post(PauseMode::LinkGraph, paused); },
	.set_pause = []() noexcept { _pause_mode.Set(PauseMode::LinkGraph); },
	.begin_measurement = [](void *p, bool client) noexcept { if (client) static_cast<std::optional<PerformanceMeasurer> *>(p)->emplace(PFE_GL_LINKGRAPH); else PerformanceMeasurer::SetInactive(PFE_GL_LINKGRAPH); },
};
static auto &ref = reference::LinkGraphSchedule::instance;
static void Import(std::initializer_list<LinkGraph *> schedule, std::initializer_list<LinkGraphJob *> running)
{
	if (candidate) { OpenTTDScheduleQueues q{schedule.begin(), schedule.size(), running.begin(), running.size()}; openttd_rust_schedule_import(&q); }
	else { ref.schedule = schedule; ref.running = running; }
}
static std::array<int, 2> QueueSizes()
{
	if (!candidate) return {static_cast<int>(ref.schedule.size()), static_cast<int>(ref.running.size())};
	OpenTTDScheduleQueues q; openttd_rust_schedule_export(&q); return {static_cast<int>(q.schedule_len), static_cast<int>(q.running_len)};
}
static void Capture()
{
	std::vector<LinkGraph *> schedule(ref.schedule.begin(), ref.schedule.end());
	std::vector<LinkGraphJob *> running(ref.running.begin(), ref.running.end());
	if (candidate) { OpenTTDScheduleQueues q; openttd_rust_schedule_export(&q); schedule.clear(); running.clear(); if (q.schedule_len) schedule.assign(q.schedule, q.schedule + q.schedule_len); if (q.running_len) running.assign(q.running, q.running + q.running_len); }
	trace.push_back({20, static_cast<int>(schedule.size()), static_cast<int>(running.size())});
	for (auto *g : schedule) trace.push_back({21, g->tag, 0});
	for (auto *j : running) trace.push_back({22, j->index, j->graph});
}
static void Queue(LinkGraph *g) { if (candidate) openttd_rust_schedule_queue(&services, g); else ref.Queue(g); }
static void Recycle() { static LinkGraph replacement{1, 2, 19}; graphs[1] = &replacement; Queue(&replacement); Queue(&replacement); }
static void Tick(int date, int fract, bool client = false)
{
	TimerGameEconomy::date = {date}; TimerGameEconomy::date_fract = fract; _networking = client;
	if (candidate) { std::optional<PerformanceMeasurer> m; openttd_rust_schedule_tick(&services, {date, static_cast<uint16_t>(fract), 4}, client, &m); }
	else reference::OnTick_LinkGraph();
}
static void Pause(int bits, bool load = false)
{
	_pause_mode.bits = bits; TimerGameEconomy::date = {1}; TimerGameEconomy::date_fract = 19;
	if (candidate) { if (load) openttd_rust_schedule_after_load(&services); else openttd_rust_schedule_pause(&services, {1, 19, 4}, bits & 1, bits == 0); }
	else { if (load) reference::AfterLoad_LinkGraphPauseControl(); else reference::StateGameLoop_LinkGraphPauseControl(); }
}
static auto Run(bool rust)
{
	candidate = rust; trace.clear(); graphs.fill(nullptr); jobs.fill(nullptr); recycle = false; Import({}, {}); Pause(0); Pause(1); Pause(0, true);
	LinkGraph small{0, 1, 10}, large{1, 2, 11}, unqueued{2, 3, 12}; graphs = {&small, &large, &unqueued, nullptr};
	Tick(0, 21); Import({&small}, {}); Tick(0, 21); Capture(); Import({}, {});
	Queue(&small); Queue(&small); Queue(&large); Tick(0, 20); Tick(0, 21); Capture(); // Duplicate first pointer ends rotation early.
	if (candidate) openttd_rust_schedule_dequeue(&small); else ref.Dequeue(&small);
	Tick(0, 21); Capture(); // Synchronous spawn/failure fallback completes before insertion.
	auto *first = jobs[0]; auto *second = new LinkGraphJob(large); second->due = true; Import({&small}, {first, second});
	Pause(0); Pause(1); Pause(0, true); Tick(1, 21); Capture(); // Only the not-due front is observed; no completed read.
	first->due = true; first->completed = false; Pause(0); Pause(1); Pause(0, true); Pause(2);
	first->completed = true; Pause(1); Pause(0); Tick(1, 21, true); Capture();
	Import({&small}, {second}); recycle = true; Tick(1, 21); recycle = false; Capture(); // Destruction recycles graph ID and queues duplicates.
	auto *listed = new LinkGraphJob(small); auto *unlisted = new LinkGraphJob(unqueued); Import({&small}, {listed});
	if (candidate) { openttd_rust_schedule_spawn_all(&services); openttd_rust_schedule_shift(&services, -17); openttd_rust_schedule_clear(&services); }
	else { ref.SpawnAll(); ref.ShiftDates({-17}); ref.Clear(); }
	Capture(); assert(jobs[listed->index] == listed && jobs[unlisted->index] == unlisted); // Clear must not delete.
	auto result = trace; for (auto *j : jobs) delete j; Import({}, {}); return result;
}
int main()
{
	auto original = Run(false), rust = Run(true);
	if (original != rust) { for (size_t i = 0; i < std::max(original.size(), rust.size()); ++i) { auto a = i < original.size() ? original[i] : std::array<int, 3>{-1,-1,-1}; auto b = i < rust.size() ? rust[i] : std::array<int, 3>{-1,-1,-1}; std::fprintf(stderr, "%zu: %d,%d,%d / %d,%d,%d\n", i, a[0], a[1], a[2], b[0], b[1], b[2]); } return 1; }
	std::printf("PASS %zu unchanged-reference scheduler observations\n", rust.size());
}

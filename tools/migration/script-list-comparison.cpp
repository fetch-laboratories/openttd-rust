/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script-list-comparison.cpp Bounded public-owner, real-VM and persistence gaps. */
#include "stdafx.h"
#include "script/api/script_list.hpp"
#include "script/api/script_tilelist.hpp"
#include "script/api/script_stationlist.hpp"
#include "station_base.h"
#ifdef WITH_RUST
#include "rust/station_cargo_ffi.h"
#endif
#include "script/squirrel.hpp"
#include <iostream>
#include <sstream>
#include <stdexcept>

extern void BeginAllocator();
extern size_t EndAllocator();
static bool allow_commands = true;
ScriptObject::DisableDoCommandScope::DisableDoCommandScope() : AutoRestoreBackup(allow_commands, false) {}
static std::vector<int> charges;
void Squirrel::DecreaseOps(HSQUIRRELVM vm, int amount) { charges.push_back(amount); sq_decreaseops(vm, amount); }
[[noreturn]] void NOT_REACHED(const std::source_location) { throw std::runtime_error("not reached"); }
[[noreturn]] void AssertFailedError(std::string_view, const std::source_location) { throw std::runtime_error("assertion"); }
[[noreturn]] void FatalErrorI(const std::string &) { throw std::runtime_error("fatal"); }
static std::vector<std::string> diagnostics;
int _debug_script_level = 9;
void DebugPrint(std::string_view category, int level, std::string &&message) { diagnostics.push_back(fmt::format("{}:{}:{}", category, level, message)); }
static void Require(bool valid) { if (!valid) throw std::runtime_error("fixture requirement"); }
class TestList : public ScriptList {
public:
	using ScriptList::SaveObject;
	using ScriptList::LoadObject;
	using ScriptList::CloneObject;
#ifdef WITH_RUST
	OpenTTDScriptList *Owner() { return this->GetRustListOwner(); }
#endif
};
class TestTileList : public ScriptTileList {
public:
	using ScriptTileList::SaveObject;
	using ScriptList::LoadObject;
	using ScriptTileList::CloneObject;
};
/* Exact pinned control bodies with fixture-only direct scalar initialization. */
#include "cargo-reducer.hpp"
class CargoProbeList : public ScriptStationList_Cargo {
public:
	CargoProbeList() = default;
#ifdef WITH_RUST
	OpenTTDScriptList *Owner() { return this->GetRustListOwner(); }
#endif
};
#ifndef WITH_RUST
/* Explicit-instantiation access obtains a read-only pointer to the actual private
 * mutation counter. It does not change the production header or list behavior. */
struct CargoModificationTag {
	using type = int ScriptList::*;
	friend type CargoModificationMember(CargoModificationTag);
};
template <CargoModificationTag::type Pointer> struct CargoModificationAccess {
	friend CargoModificationTag::type CargoModificationMember(CargoModificationTag) { return Pointer; }
};
template struct CargoModificationAccess<&ScriptList::modifications>;
#endif
static int CargoToken(CargoProbeList &list)
{
#ifdef WITH_RUST
	return openttd_rust_list_token(list.Owner());
#else
	return list.*CargoModificationMember(CargoModificationTag{});
#endif
}
static int ListToken(TestList &list)
{
#ifdef WITH_RUST
	return openttd_rust_list_token(list.Owner());
#else
	return list.*CargoModificationMember(CargoModificationTag{});
#endif
}
class GapCargoCollector {
public:
	GapCargoCollector(CargoProbeList &list, uint8_t selector, StationID other) : list(list), selector(selector)
#ifndef WITH_RUST
		, original(&list, StationID::Invalid(), 0, other)
#endif
	{
#ifdef WITH_RUST
		openttd_rust_cargo_init(&this->state, selector, other.base());
#endif
	}
	~GapCargoCollector()
	{
#ifdef WITH_RUST
		openttd_rust_cargo_finish(&this->state, this->list.Owner());
#endif
	}
	void Packet(StationID from, StationID via, uint32_t amount)
	{
#ifdef WITH_RUST
		openttd_rust_cargo_packet(&this->state, this->list.Owner(), from.base(), via.base(), amount);
#else
		switch (this->selector) {
			case 0: this->original.Update<ScriptStationList_Cargo::CS_BY_FROM>(from, via, amount); break;
			case 1: this->original.Update<ScriptStationList_Cargo::CS_VIA_BY_FROM>(from, via, amount); break;
			case 2: this->original.Update<ScriptStationList_Cargo::CS_BY_VIA>(from, via, amount); break;
			case 3: this->original.Update<ScriptStationList_Cargo::CS_FROM_BY_VIA>(from, via, amount); break;
		}
#endif
	}
	void Planned(const FlowStatMap &flows, StationID other)
	{
#ifdef WITH_RUST
		auto feed = [this](FlowStatMap::const_iterator iter) {
			openttd_rust_cargo_origin(&this->state, iter->first.base());
			for (const auto &[cumulative, via] : *iter->second.GetShares()) openttd_rust_cargo_share(&this->state, this->list.Owner(), via.base(), cumulative);
		};
		if (openttd_rust_cargo_plan(1, this->selector) == 3) {
			auto iter = flows.find(other);
			if (iter != flows.end()) feed(iter);
		} else {
			for (auto iter = flows.begin(); iter != flows.end(); ++iter) feed(iter);
		}
#else
		switch (this->selector) {
			case 0: OriginalCargoPlanned<ScriptStationList_Cargo::CS_BY_FROM>(this->original, flows); break;
			case 1: OriginalCargoPlanned<ScriptStationList_Cargo::CS_VIA_BY_FROM>(this->original, flows); break;
			case 2: OriginalCargoPlanned<ScriptStationList_Cargo::CS_BY_VIA>(this->original, flows); break;
			case 3: OriginalCargoFind(this->original, flows, other); break;
		}
#endif
	}
private:
	CargoProbeList &list;
	uint8_t selector;
#ifdef WITH_RUST
	OpenTTDCargoCollector state;
#else
	ReferenceCargoCollector original;
#endif
};
static void CargoRecord(std::string_view label, CargoProbeList &list)
{
	std::cout << "cargo " << label << " token=" << CargoToken(list) << " end=" << list.IsEnd() << " count=" << list.Count() << " items=";
	for (int64_t key : {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 20, 30, 40, 65535}) if (list.HasItem(key)) std::cout << key << '=' << list.GetValue(key) << ',';
	std::cout << '\n';
}
static void CargoGaps()
{
	for (uint8_t selector = 0; selector != 4; ++selector) for (auto mode : {ScriptList::SORT_BY_VALUE, ScriptList::SORT_BY_ITEM}) for (bool ascending : {false, true}) {
		std::cout << "cargo_mode " << unsigned(selector) << ' ' << mode << ' ' << ascending << '\n';
		CargoProbeList list; list.AddItem(1, 8); list.AddItem(2, 9); list.AddItem(7, 2); list.Sort(mode, ascending); list.Begin();
		{
			GapCargoCollector collector(list, selector, StationID{7});
			CargoRecord("empty-pending", list);
			collector.Packet(StationID{7}, StationID{7}, UINT32_MAX);
			collector.Packet(StationID{7}, StationID{7}, 2);
			collector.Packet(StationID{8}, StationID{8}, 3); // Both filters reject a different run key.
			collector.Packet(StationID{7}, StationID{7}, 2);
			collector.Packet(StationID{1}, StationID{7}, 0);
			CargoRecord("key-change-wrap-zero", list);
			collector.Packet(StationID::Invalid(), StationID::Invalid(), 10);
			collector.Packet(StationID{7}, StationID{7}, 11);
			CargoRecord("noncontiguous-before-final", list);
		}
		CargoRecord("final", list);
		std::cout << "cargo_cursor";
		for (unsigned n = 0; !list.IsEnd() && n < 32; ++n) std::cout << ' ' << list.Next();
		Require(list.IsEnd()); std::cout << '\n';
		CargoProbeList partial;
		try {
			GapCargoCollector collector(partial, selector, StationID{7});
			collector.Packet(StationID{7}, StationID{7}, 5);
			collector.Packet(StationID{1}, StationID{7}, 7);
			CargoRecord("before-injected-throw", partial);
			{ GapCargoCollector nested(partial, selector, StationID{7}); nested.Packet(StationID{7}, StationID{7}, 3); }
			throw std::runtime_error("fixture interruption between Rust calls");
		} catch (const std::runtime_error &) { CargoRecord("unwind-last-run", partial); }
		FlowStatMap flows;
		FlowStat first(StationID{7}, 5); first.AppendShare(StationID{8}, 9); first.AppendShare(StationID{7}, 4, true);
		flows.emplace(StationID{7}, std::move(first));
		FlowStat second(StationID{7}, 3, true); second.AppendShare(StationID::Invalid(), 2, true);
		flows.emplace(StationID{8}, std::move(second));
		CargoProbeList planned;
		{ GapCargoCollector collector(planned, selector, StationID{7}); collector.Planned(flows, StationID{7}); CargoRecord("planned-before-final", planned); }
		CargoRecord("planned-origin-reset-filter-restricted", planned);
		CargoProbeList absent;
		{ GapCargoCollector collector(absent, selector, StationID{40}); collector.Planned({}, StationID{40}); collector.Planned(flows, StationID{40}); }
		CargoRecord("empty-absent-origin", absent);
	}
}
static constexpr SQInteger keys[] = {INT64_MIN, -5, -1, 0, 1, 2, 3, 4, 5, 6, 8, 10, 20, 30, 40, 99, INT64_MAX};
static std::string Hex(std::string_view input)
{
	std::string out;
	for (unsigned char c : input) { out += "0123456789abcdef"[c >> 4]; out += "0123456789abcdef"[c & 15]; }
	return out;
}
static std::string Content(ScriptList &list)
{
	std::string out = fmt::format("{}:", list.Count());
	SQInteger seen = 0;
	for (auto key : keys) if (list.HasItem(key)) { out += fmt::format("{}={},", key, list.GetValue(key)); ++seen; }
	Require(seen == list.Count());
	return out;
}
static void Record(std::string_view label, ScriptList &list, SQInteger result = 0)
{
	std::cout << "owner " << label << ' ' << result << ' ' << list.IsEnd() << ' ' << Content(list) << " logs=";
	for (auto &message : diagnostics) std::cout << Hex(message) << ',';
	diagnostics.clear();
	std::cout << '\n';
}
static void Fill(ScriptList &list)
{
	list.AddItem(0, 5); list.AddItem(2, 5); list.AddItem(4, -1); list.AddItem(6, 5);
}
static void Drain(std::string_view label, ScriptList &list)
{
	for (unsigned n = 0; !list.IsEnd() && n < 32; ++n) Record(label, list, list.Next());
	Require(list.IsEnd());
}
static void OwnerGaps()
{
	for (auto mode : {ScriptList::SORT_BY_VALUE, ScriptList::SORT_BY_ITEM}) for (bool ascending : {false, true}) {
		std::cout << "mode " << static_cast<int>(mode) << ' ' << ascending << '\n';
		for (unsigned action = 0; action != 4; ++action) {
			TestList list; Fill(list); list.Sort(mode, ascending);
			SQInteger first = list.Begin(); Record("begin", list, first);
			const SQInteger pending = mode == ScriptList::SORT_BY_ITEM ? (ascending ? 2 : 4) : (ascending ? 0 : 2);
			if (action == 0) { list.AddItem(1, 0); list.AddItem(3, 10); list.AddItem(8, 5); }
			if (action == 1) list.RemoveItem(pending);
			if (action == 2) list.SetValue(pending, 7);
			if (action == 3) {
				list.AddItem(0, 999); list.RemoveItem(99); list.SetValue(pending, list.GetValue(pending));
				list.Sort(mode, ascending); list.Sort(static_cast<ScriptList::SorterType>(9), !ascending);
				Record("absent-set", list, list.SetValue(99, 17));
			}
			Record("mutation", list); Drain("next", list);
			list.AddItem(99, 999); Record("insert-ended", list, list.Next());
		}
		for (bool ended : {false, true}) {
			TestList a, b; Fill(a); Fill(b); b.SetValue(4, 20); b.AddItem(10, 30);
			a.Sort(mode, ascending); b.Sort(mode == ScriptList::SORT_BY_ITEM ? ScriptList::SORT_BY_VALUE : ScriptList::SORT_BY_ITEM, !ascending);
			a.Begin(); b.Begin();
			if (ended) Drain("pre-swap-end", b);
			a.SwapList(&b); Record("swap-a", a); Record("swap-b", b);
			Drain("swap-next-a", a); Drain("swap-next-b", b);
		}
		TestList self; Fill(self); self.Sort(mode, ascending); self.Begin();
		self.AddList(&self); self.KeepList(&self); self.SwapList(&self); Record("self-noops", self, self.Next());
		self.RemoveList(&self); Record("self-remove", self);
		for (unsigned operation = 0; operation != 8; ++operation) for (unsigned bounds = 0; bounds != 3; ++bounds) {
			TestList list; Fill(list); list.Sort(mode, ascending); list.Begin();
			SQInteger lo = bounds == 0 ? -1 : 5, hi = bounds == 1 ? -1 : 5;
			switch (operation) {
				case 0: list.RemoveAboveValue(lo); break; case 1: list.RemoveBelowValue(lo); break;
				case 2: list.RemoveBetweenValue(lo, hi); break; case 3: list.RemoveValue(lo); break;
				case 4: list.KeepAboveValue(lo); break; case 5: list.KeepBelowValue(lo); break;
				case 6: list.KeepBetweenValue(lo, hi); break; case 7: list.KeepValue(lo); break;
			}
			Record("filter", list, operation); Drain("filter-next", list);
		}
		for (unsigned operation = 0; operation != 4; ++operation) for (SQInteger count : std::array<SQInteger, 8>{
				0, -1, 2, 6, INT64_MIN, INT64_MIN + 1, INT64_MIN + 4, INT64_MAX}) {
			TestList list; Fill(list); list.Sort(mode, ascending); list.Begin();
			switch (operation) {
				case 0: list.RemoveTop(count); break; case 1: list.RemoveBottom(count); break;
				case 2: list.KeepTop(count); break; case 3: list.KeepBottom(count); break;
			}
			Record("rank", list, count); Record("rank-next", list, list.Next());
		}
		TestList source, empty, initialized; Fill(source); source.Sort(mode, ascending);
		initialized.Begin(); empty.AddList(&source); initialized.AddList(&source);
		Record("empty-fast-uninitialized", empty, empty.Next()); Record("empty-fast-initialized", initialized, initialized.Next());
		TestList cleared; Fill(cleared); cleared.Begin(); cleared.Clear(); cleared.AddList(&source); Record("clear-fast", cleared, cleared.Next());
		TestList partial; partial.AddItem(0, 99); partial.AddItem(10, 88); partial.Sort(mode, ascending); partial.Begin();
		partial.AddList(&source); Record("add-overwrite", partial); partial.KeepList(&source); Record("intersection", partial);
		partial.RemoveList(&source); Record("difference", partial);
	}
}
static std::string VMValue(HSQUIRRELVM vm, SQInteger index)
{
	switch (sq_gettype(vm, index)) {
		case OT_NULL: return "null";
		case OT_INTEGER: { SQInteger value; sq_getinteger(vm, index, &value); return fmt::format("i{}", value); }
		case OT_BOOL: { SQBool value; sq_getbool(vm, index, &value); return value ? "true" : "false"; }
		case OT_STRING: { std::string_view value; sq_getstring(vm, index, value); return "s" + Hex(value); }
		case OT_TABLE:
		case OT_ARRAY: {
			bool table = sq_gettype(vm, index) == OT_TABLE;
			sq_push(vm, index); sq_pushnull(vm);
			std::vector<std::string> entries;
			while (SQ_SUCCEEDED(sq_next(vm, -2))) { entries.push_back(VMValue(vm, -2) + "=" + VMValue(vm, -1)); sq_pop(vm, 2); }
			sq_pop(vm, 2);
			if (table) std::ranges::sort(entries);
			std::string result = table ? "{" : "[";
			for (auto &entry : entries) result += entry + ",";
			return result + (table ? "}" : "]");
		}
		default: return fmt::format("type{}", static_cast<int>(sq_gettype(vm, index)));
	}
}
static ScriptList *active;
static std::vector<SQInteger> calls;
static SQInteger RunValuate(HSQUIRRELVM vm) { return active->Valuate(vm); }
static SQInteger RecordItem(HSQUIRRELVM vm)
{
	SQInteger key; sq_getinteger(vm, 2, &key); calls.push_back(key);
	Require(!allow_commands); return 0;
}
static SQInteger Mutate(HSQUIRRELVM vm)
{
	SQInteger operation; sq_getinteger(vm, 2, &operation);
	switch (operation) {
		case 0: active->AddItem(1, 99); break;
		case 1: active->Sort(ScriptList::SORT_BY_VALUE, false); break;
		case 2: active->AddList(active); active->KeepList(active); active->SwapList(active); break;
		case 3: active->SetValue(1, 11); break;
		case 4: active->RemoveItem(99); break;
	}
	return 0;
}
static void Native(HSQUIRRELVM vm, std::string_view name, SQFUNCTION function)
{
	sq_pushroottable(vm); sq_pushstring(vm, name); sq_newclosure(vm, function, 0); sq_newslot(vm, -3, SQFalse); sq_poptop(vm);
}
static void ValuationGaps()
{
	const std::string_view functions[] = {
		"function(k) { Record(k); Mutate(0); return k*10; }",
		"function(k) { Record(k); Mutate(1); return k*10; }",
		"function(k) { Record(k); Mutate(0); return \"wrong\"; }",
		"function(k) { Record(k); if (k == 2) throw \"later failure\"; return k*10; }",
		"function(k) { Record(k); Mutate(2); return k*10; }",
		"function(k) { Record(k); Mutate(3); return true; }",
		"function(k) { Record(k); Mutate(4); return true; }",
		"function(k) { Record(k); return k != 2; }",
	};
	for (unsigned i = 0; i != std::size(functions); ++i) {
		BeginAllocator(); auto vm = sq_open(1024);
		TestList list; list.AddItem(1, 7); list.AddItem(2, 8); list.AddItem(3, 9); active = &list;
		Native(vm, "RunValuate", RunValuate); Native(vm, "Record", RecordItem); Native(vm, "Mutate", Mutate);
		std::string script = "return RunValuate(" + std::string(functions[i]) + ");";
		Require(SQ_SUCCEEDED(sq_compilebuffer(vm, script, "list-gap", SQFalse)));
		sq_pushroottable(vm); calls.clear(); charges.clear();
		SQRESULT result = sq_call(vm, 1, SQTrue, SQFalse);
		std::string stack;
		for (SQInteger slot = 1; slot <= sq_gettop(vm); ++slot) stack += VMValue(vm, slot) + ";";
		sq_getlasterror(vm); auto error = VMValue(vm, -1); sq_poptop(vm);
		Require(allow_commands);
		std::cout << "valuate " << i << ' ' << result << ' ' << Content(list) << " calls=";
		for (auto key : calls) std::cout << key << ',';
		std::cout << " charge=";
		for (auto amount : charges) std::cout << amount << ',';
		std::cout << " error=" << error << " stack=" << stack << " logs=";
		for (auto &message : diagnostics) std::cout << Hex(message) << ',';
		diagnostics.clear(); std::cout << '\n';
		sq_close(vm); Require(EndAllocator() == 0);
	}
}
template <typename T>
static void Persistence(std::string_view tag)
{
	for (auto mode : {ScriptList::SORT_BY_VALUE, ScriptList::SORT_BY_ITEM}) for (bool ascending : {false, true}) {
		BeginAllocator(); auto vm = sq_open(1024);
		T source, loaded;
		source.AddItem(INT64_MIN, INT64_MAX); source.AddItem(0, 0); source.AddItem(INT64_MAX, INT64_MIN);
		source.Sort(mode, ascending); source.Begin();
		sq_pushinteger(vm, 55); Require(source.SaveObject(vm));
		std::string representation = VMValue(vm, -2) + ":" + VMValue(vm, -1);
		Require(loaded.LoadObject(vm));
		std::string loaded_stack;
		for (SQInteger slot = 1; slot <= sq_gettop(vm); ++slot) loaded_stack += VMValue(vm, slot) + ";";
		std::unique_ptr<ScriptObject> clone(source.CloneObject());
		auto *copied = dynamic_cast<ScriptList *>(clone.get()); Require(copied != nullptr);
		bool tile = dynamic_cast<ScriptTileList *>(clone.get()) != nullptr;
		copied->SetValue(0, 99); copied->RemoveItem(INT64_MIN);
		std::cout << "persist " << tag << ' ' << static_cast<int>(mode) << ' ' << ascending << ' ' << representation
				<< " source=" << Content(source) << " loaded=" << Content(loaded) << " clone=" << Content(*copied)
				<< " tile=" << tile << " stack=" << loaded_stack << '\n';
		Record("save-live-next", source, source.Next());
		Record("loaded-begin", loaded, loaded.Begin());
		sq_close(vm); Require(EndAllocator() == 0);
	}
}
/* Only the ScriptList VM/control paths the harness does not reach; bundled VM and owner are real. */
struct FilterItem {
	SQInteger index;
	static auto &Items() { static FilterItem items[] = {{1}, {2}, {3}, {4}}; return items; }
	static auto Iterate()
	{
		auto &items = Items();
		static const FilterItem *const pointers[] = {&items[0], &items[1], &items[2], &items[3]};
		return std::span(pointers);
	}
};
static std::vector<SQInteger> validation_calls;
static bool throw_validation;
class FilterList : public TestList {
public:
	void Filter(HSQUIRRELVM vm)
	{
		this->FillList<FilterItem>(vm, this, [](const FilterItem *item) {
			Require(!allow_commands);
			validation_calls.push_back(item->index);
			if (throw_validation && item->index == 3) throw std::runtime_error("typed predicate");
			return item->index != 2;
		});
	}
};
static void Expression(HSQUIRRELVM vm, std::string_view expression)
{
	Require(SQ_SUCCEEDED(sq_compilebuffer(vm, "return " + std::string(expression) + ";", "control-gap", SQFalse)));
	sq_pushroottable(vm);
	Require(SQ_SUCCEEDED(sq_call(vm, 1, SQTrue, SQFalse)));
	sq_remove(vm, -2);
}
static std::string Stack(HSQUIRRELVM vm)
{
	std::string out;
	for (SQInteger slot = 1; slot <= sq_gettop(vm); ++slot) out += VMValue(vm, slot) + ";";
	return out;
}
static void ControlObservation(std::string_view label, unsigned number, HSQUIRRELVM vm, TestList &list, SQInteger result, std::string_view thrown)
{
	auto stack = Stack(vm);
	sq_getlasterror(vm); auto error = VMValue(vm, -1); sq_poptop(vm);
	std::cout << "vm-control " << label << ' ' << number << " result=" << result << " thrown=" << thrown
			<< " token=" << ListToken(list) << " content=" << Content(list) << " stack=" << stack << " error=" << error
			<< " allow=" << allow_commands << " calls=";
	for (auto key : calls) std::cout << key << ',';
	std::cout << " valid=";
	for (auto key : validation_calls) std::cout << key << ',';
	std::cout << " charge=";
	for (auto charge : charges) std::cout << charge << ',';
	std::cout << " logs=";
	for (auto &message : diagnostics) std::cout << Hex(message) << ',';
	std::cout << '\n'; diagnostics.clear();
}
static SQInteger CheckRoot(HSQUIRRELVM vm)
{
	sq_push(vm, 2); sq_pushroottable(vm); Require(sq_cmp(vm) == 0); sq_pop(vm, 2); return 0;
}
static SQInteger BoolFilter(HSQUIRRELVM vm)
{
	RecordItem(vm); SQInteger key; sq_getinteger(vm, 2, &key); sq_pushbool(vm, key != 3); return 1;
}
static SQInteger Reindex(HSQUIRRELVM vm)
{
	SQInteger key; sq_getinteger(vm, 2, &key);
	if (key == 1) FilterItem::Items()[0].index = 6;
	return 0;
}
static void FilterGaps()
{
	const std::string_view expressions[] = {
		"", "7", "null", "function(k) { Record(k); return k != 3; }",
		"function(k,a,b) { Record(k); CheckRoot(this); if (a != 17 || b != 23) throw \"args\"; return true; }",
		"function(k) { Record(k); return 1; }",
		"function(k) { Record(k); if (k == 3) throw \"later filter\"; return true; }",
		"function(k) { Record(k); if (k == 3) return null; return true; }",
		"function(k) { Record(k); return true; }",
		"BoolFilter",
		"function(k) { Record(k); Reindex(k); return true; }",
	};
	for (unsigned i = 0; i != std::size(expressions); ++i) {
		BeginAllocator(); auto vm = sq_open(1024); FilterList list;
		for (unsigned n = 0; n != 4; ++n) FilterItem::Items()[n].index = n + 1;
		Native(vm, "Record", RecordItem); Native(vm, "CheckRoot", CheckRoot); Native(vm, "BoolFilter", BoolFilter); Native(vm, "Reindex", Reindex); sq_pushnull(vm);
		if (!expressions[i].empty()) Expression(vm, expressions[i]);
		if (i == 4) { sq_pushinteger(vm, 17); sq_pushinteger(vm, 23); }
		calls.clear(); charges.clear(); validation_calls.clear(); throw_validation = i == 8;
		std::string thrown; SQInteger result = 0;
		try { list.Filter(vm); if (i == 4) Require(list.Count() == 3); } catch (SQInteger value) { result = value; thrown = "integer"; }
		catch (const std::runtime_error &error) { thrown = error.what(); }
		Require(allow_commands);
		ControlObservation("filter", i, vm, list, result, thrown);
		throw_validation = false; sq_close(vm); Require(EndAllocator() == 0);
	}
	/* Missing/nonfunction valuation validates before constructing host scopes. */
	for (unsigned i = 0; i != 3; ++i) {
		BeginAllocator(); auto vm = sq_open(1024); TestList list; list.AddItem(1, 7);
		sq_pushnull(vm); if (i == 1) sq_pushinteger(vm, 7); if (i == 2) sq_pushnull(vm);
		calls.clear(); charges.clear(); validation_calls.clear();
		auto result = list.Valuate(vm); Require(allow_commands);
		ControlObservation("valuate-arguments", i, vm, list, result, "");
		sq_close(vm); Require(EndAllocator() == 0);
	}
}
static void LoadGaps()
{
	const std::string_view expressions[] = {
		"null", "{}", "[]", "[true]", "[0]", "[0, 1]", "[0, false]", "[0, false, []]",
		"[0, false, {}]", "[1, true, {[1]=8, [3]=9}]", "[1, true, {[1]=8, [3]=9}, 99]",
		"[0, false, {[1]=2.75, [3.75]=4}]", "[1, false, {[1]=-2.75, [-1.75]=4}]",
		"[0, true, {[1.25]=2.75}]", "[0, false, {[\"bad\"]=\"bad\"}]",
		"[1, true, {[1]=8, [3]=9, [\"bad\"]=\"bad\"}]",
		"[1, true, {[0]=8, [4]=9, [\"bad\"]=\"bad\"}]",
		"[2, true, {[1]=8}]",
	};
	for (unsigned i = 0; i != std::size(expressions); ++i) {
		BeginAllocator(); auto vm = sq_open(1024); TestList list;
		list.AddItem(1, 71); list.AddItem(6, 66); list.Sort(ScriptList::SORT_BY_ITEM, true); list.Begin();
		sq_pushinteger(vm, 55); Expression(vm, expressions[i]);
		calls.clear(); charges.clear(); validation_calls.clear();
		auto result = list.LoadObject(vm);
		ControlObservation("load", i, vm, list, result, "");
		Record("load-live-next", list, list.Next());
		Record("load-live-begin", list, list.Begin());
		sq_close(vm); Require(EndAllocator() == 0);
	}
}
static void MetamethodGaps()
{
	for (unsigned i = 0; i != 15; ++i) {
		BeginAllocator(); auto vm = sq_open(1024); TestList list; list.AddItem(1, 7); list.AddItem(3, 8);
		sq_pushnull(vm);
		if (i == 0 || i == 3 || i == 12) sq_pushstring(vm, "key");
		else if (i == 1 || i == 5) sq_pushinteger(vm, 99);
		else if (i == 7 || i == 9) sq_pushinteger(vm, 4);
		else if (i == 11 || i == 14) sq_pushnull(vm);
		else sq_pushinteger(vm, 1);
		if (i == 4 || i == 5) sq_pushnull(vm);
		if (i == 6 || i == 7) sq_pushbool(vm, i == 6 ? SQTrue : SQFalse);
		if (i == 8 || i == 9) sq_pushinteger(vm, INT64_MIN);
		if (i == 3 || i == 10) sq_pushstring(vm, "wrong");
		if (i == 14) list.Clear();
		if (i == 13) { list.Sort(ScriptList::SORT_BY_ITEM, true); list.Begin(); list.Next(); list.Next(); }
		calls.clear(); charges.clear(); validation_calls.clear();
		SQInteger result = i < 3 ? list._get(vm) : i < 11 ? list._set(vm) : list._nexti(vm);
		ControlObservation("metamethod", i, vm, list, result, "");
		Record("meta-live-next", list, list.Next());
		sq_close(vm); Require(EndAllocator() == 0);
	}
}
static std::vector<std::string> nested_observations;
static unsigned nested_mode;
static SQInteger NestedControl(HSQUIRRELVM vm)
{
	FilterList nested; nested.AddItem(4, 44);
	for (unsigned n = 0; n != 4; ++n) FilterItem::Items()[n].index = n + 1;
	SQInteger result = 0;
	if (nested_mode == 0) result = nested.Valuate(vm);
	else nested.Filter(vm);
	Require(!allow_commands);
	nested_observations.push_back(Content(nested));
	return result;
}
static void NestedGaps()
{
	for (unsigned mode = 0; mode != 2; ++mode) {
		BeginAllocator(); auto vm = sq_open(1024); TestList list;
		list.AddItem(1, 7); list.AddItem(3, 8); active = &list; nested_mode = mode;
		Native(vm, "RunValuate", RunValuate); Native(vm, "Record", RecordItem); Native(vm, "Nested", NestedControl); Native(vm, "CheckRoot", CheckRoot);
		std::string script = mode == 0
			? "return RunValuate(function(k,a) { Record(k); CheckRoot(this); if (a != 17) throw \"args\"; Nested(function(n,b) { Record(n); return n+b; },23); return k+a; },17);"
			: "return RunValuate(function(k) { Record(k); Nested(function(n,b) { Record(n); return n != b; },3); return true; });";
		Require(SQ_SUCCEEDED(sq_compilebuffer(vm, script, "nested-control", SQFalse))); sq_pushroottable(vm);
		calls.clear(); charges.clear(); validation_calls.clear(); nested_observations.clear();
		auto result = sq_call(vm, 1, SQTrue, SQFalse); Require(SQ_SUCCEEDED(result) && allow_commands);
		ControlObservation("nested", mode, vm, list, result, "");
		std::cout << "vm-control nested-results " << mode << ' ';
		for (auto &observation : nested_observations) std::cout << observation << ';';
		std::cout << '\n'; sq_close(vm); Require(EndAllocator() == 0);
	}
}
int main()
{
	OwnerGaps(); CargoGaps(); ValuationGaps(); Persistence<TestList>("List"); Persistence<TestTileList>("TileList");
	FilterGaps(); LoadGaps(); MetamethodGaps(); NestedGaps();
}

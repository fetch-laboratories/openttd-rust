/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company-control-console.inc Native context, errors, notification order and saved keys. */
#include "company_cmd.h"
#include "vehicle-record-fixture.hpp"
#include "network/network_type.h"
#include "town.h"
#include "window_func.h"

static bool _company_control_capture = false, _company_control_transfer = false;
static std::vector<std::string_view> _company_control_events;
static void CompanyControlEvent(std::string_view name, int cls = 0, int number = 0, int data = 0)
{
	if (!_company_control_capture) return;
	_company_control_events.push_back(name);
	fmt::print(stderr, "COMPANY-CONTROL event {} {} {} {} current {} local {}\n", name, cls, number, data, _current_company.base(), _local_company.base());
}
#define COMPANY_CONTROL_WRAP0(symbol, name) \
	extern "C" void __real_##symbol(); \
	extern "C" void __wrap_##symbol() { CompanyControlEvent(name); __real_##symbol(); }
COMPANY_CONTROL_WRAP0(_Z24CloseConstructionWindowsv, "construction")
COMPANY_CONTROL_WRAP0(_Z20MarkWholeScreenDirtyv, "screen")
COMPANY_CONTROL_WRAP0(_Z21ResetVehicleColourMapv, "colourmap")
#undef COMPANY_CONTROL_WRAP0
extern "C" void __real__Z20InvalidateWindowData11WindowClass12WindowNumberib(WindowClass, WindowNumber, int, bool);
extern "C" void __wrap__Z20InvalidateWindowData11WindowClass12WindowNumberib(WindowClass cls, WindowNumber number, int data, bool gui)
{
	CompanyControlEvent("data", cls, number, data);
	__real__Z20InvalidateWindowData11WindowClass12WindowNumberib(cls, number, data, gui);
}
extern "C" void __real__Z27InvalidateWindowClassesData11WindowClassib(WindowClass, int, bool);
extern "C" void __wrap__Z27InvalidateWindowClassesData11WindowClassib(WindowClass cls, int data, bool gui)
{
	CompanyControlEvent("classes", cls, 0, data);
	__real__Z27InvalidateWindowClassesData11WindowClassib(cls, data, gui);
}
extern "C" void __real__Z14SetWindowDirty11WindowClass12WindowNumber(WindowClass, WindowNumber);
extern "C" void __wrap__Z14SetWindowDirty11WindowClass12WindowNumber(WindowClass cls, WindowNumber number)
{
	CompanyControlEvent("dirty", cls, number);
	__real__Z14SetWindowDirty11WindowClass12WindowNumber(cls, number);
}
extern "C" CommandCost __real__Z19CmdChangeServiceInt10EnumBitSetI13DoCommandFlagtLS0_16EE6PoolIDIj12VehicleIDTagLj1044480ELj1048575EEtbb(DoCommandFlags, VehicleID, uint16_t, bool, bool);
extern "C" CommandCost __wrap__Z19CmdChangeServiceInt10EnumBitSetI13DoCommandFlagtLS0_16EE6PoolIDIj12VehicleIDTagLj1044480ELj1048575EEtbb(DoCommandFlags flags, VehicleID id, uint16_t interval, bool custom, bool percent)
{
	if (_company_control_transfer && flags.Test(DoCommandFlag::Bankrupt)) {
		const Vehicle *v = Vehicle::Get(id);
		if (flags != DoCommandFlags{DoCommandFlag::Execute, DoCommandFlag::Bankrupt} || custom) FixtureAbort(__LINE__);
		fmt::print(stderr, "COMPANY-CONTROL nested {} {} {} {} {} {} {}\n", id.base(), FIXTURE_GET(v, Type, type), FIXTURE_GET(v, Owner, owner).base(), flags.base(), interval, custom, percent);
	}
	return __real__Z19CmdChangeServiceInt10EnumBitSetI13DoCommandFlagtLS0_16EE6PoolIDIj12VehicleIDTagLj1044480ELj1048575EEtbb(flags, id, interval, custom, percent);
}
static void CompanyControlInspect()
{
	for (const Company *c : Company::Iterate()) for (size_t i = 0; i < c->allow_list.size(); ++i) {
		fmt::print(stderr, "COMPANY-CONTROL key {} {} {}\n", c->index.base(), i, c->allow_list[i]);
	}
}
static void CompanyControlProbe()
{
	AutoRestoreBackup<CompanyID> current(_current_company, CompanyID(0)), local(_local_company, CompanyID(0));
	SavedRandomSeeds before, after; SaveRandomSeeds(&before);
	for (CompanyID target : {CompanyID(0), CompanyID(1), COMPANY_SPECTATOR, COMPANY_SPECTATOR, OWNER_NONE, CompanyID(0)}) {
		bool switching = _local_company != target;
		_company_control_events.clear(); _company_control_capture = true;
		SetLocalCompany(target);
		_company_control_capture = false;
		const std::vector<std::string_view> expected = switching ? std::vector<std::string_view>{"data", "classes", "construction", "screen", "classes", "classes", "classes", "colourmap"} : std::vector<std::string_view>{"screen", "classes", "classes", "classes", "colourmap"};
		if (_company_control_events != expected || _current_company != target || _local_company != target) FixtureAbort(__LINE__);
		fmt::print(stderr, "COMPANY-CONTROL local {} {}\n", target.base(), switching);
	}
	const Town *town = *Town::Iterate().begin();
	for (CompanyID observer : {CompanyID(0), COMPANY_SPECTATOR}) {
		_local_company = observer;
		for (Owner owner : {CompanyID(0), CompanyID(1), OWNER_NONE, OWNER_TOWN}) {
			auto cost = CheckOwnership(owner, town->xy);
			if (cost.Succeeded() != (owner == 0) || (!cost.Succeeded() && cost.GetErrorMessage() != STR_ERROR_OWNED_BY)) FixtureAbort(__LINE__);
			if (cost.GetErrorOwner() != (observer == 0 && owner != 0 && owner != OWNER_TOWN ? owner : INVALID_OWNER)) FixtureAbort(__LINE__);
			fmt::print(stderr, "COMPANY-CONTROL ownership {} {} {} {} {} {}\n", observer.base(), owner.base(), cost.Succeeded(), cost.GetErrorMessage(), cost.GetErrorOwner().base(), cost.GetEncodedMessage().GetDecodedString());
		}
	}
	_local_company = CompanyID(0);
	Company *company = Company::Get(_current_company);
	if (!company->allow_list.empty()) FixtureAbort(__LINE__);
	const std::string key(64, 'a'), upper(64, 'A'), missing(64, 'b'), nonhex(64, 'z');
	auto command = [&](std::string_view name, bool execute, CompanyAllowListCtrlAction action, const std::string &value, bool success, bool changed) {
		_company_control_events.clear(); _company_control_capture = true;
		auto cost = CmdCompanyAllowListCtrl(execute ? DoCommandFlags{DoCommandFlag::Execute} : DoCommandFlags{}, action, value);
		_company_control_capture = false;
		const std::vector<std::string_view> expected = changed ? std::vector<std::string_view>{"data", "dirty"} : std::vector<std::string_view>{};
		if (cost.Succeeded() != success || _company_control_events != expected) FixtureAbort(__LINE__);
		fmt::print(stderr, "COMPANY-CONTROL allow {} {} {} {}\n", name, cost.Succeeded(), cost.GetErrorMessage(), company->allow_list.size());
	};
	_current_company = COMPANY_SPECTATOR;
	command("company", true, CALCA_ADD, key, false, false);
	_current_company = CompanyID(0);
	for (size_t size : {0U, 63U, 65U}) command("length", true, CALCA_ADD, std::string(size, 'a'), false, false);
	for (auto action : {CALCA_END, static_cast<CompanyAllowListCtrlAction>(255)}) command("action", true, action, key, false, false);
	command("test-action", false, CALCA_END, key, false, false);
	command("test-add", false, CALCA_ADD, key, true, false);
	command("add", true, CALCA_ADD, key, true, true);
	command("duplicate", true, CALCA_ADD, key, true, false);
	command("case", true, CALCA_ADD, upper, true, false);
	command("missing", true, CALCA_REMOVE, missing, true, false);
	command("test-remove", false, CALCA_REMOVE, upper, true, false);
	command("remove", true, CALCA_REMOVE, upper, true, true);
	command("remove-missing", true, CALCA_REMOVE, key, true, false);
	command("restore", true, CALCA_ADD, upper, true, true);
	command("nonhex", true, CALCA_ADD, nonhex, true, true);
	if (company->allow_list.size() != 2 || company->allow_list[0] != upper || company->allow_list[1] != nonhex) FixtureAbort(__LINE__);
	SaveRandomSeeds(&after);
	if (!SameRandom(before, after)) FixtureAbort(__LINE__);
	fmt::print(stderr, "COMPANY-CONTROL complete rng-unchanged true\n");
}

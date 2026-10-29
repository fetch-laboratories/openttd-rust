/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company-identity-console.inc Company identity production-command witnesses. */
#include "company_cmd.h"
#include "company_manager_face.h"
#include "core/string_consumer.hpp"

static void CompanyIdentityInspect()
{
	for (const Company *c : Company::Iterate()) {
		fmt::print(stderr, "COMPANY-IDENTITY record {} {} {} {} {} {} {}\n", c->index.base(), to_underlying(c->colour), c->name_1, c->name_2, c->president_name_1, c->president_name_2, c->face.bits);
		for (uint scheme = 0; scheme < LS_END; scheme++) {
			const Livery &l = c->livery[scheme];
			fmt::print(stderr, "COMPANY-IDENTITY livery {} {} {} {} {}\n", c->index.base(), scheme, l.in_use.base(), to_underlying(l.colour1), to_underlying(l.colour2));
		}
	}
	for (const Group *g : Group::Iterate()) {
		fmt::print(stderr, "COMPANY-IDENTITY group {} {} {} {} {} {}\n", g->index.base(), g->owner.base(), g->parent.base(), g->livery.in_use.base(), to_underlying(g->livery.colour1), to_underlying(g->livery.colour2));
	}
	for (const Engine *e : Engine::Iterate()) {
		fmt::print(stderr, "COMPANY-IDENTITY engine-scheme {} {}\n", e->index.base(), to_underlying(GetEngineLiveryScheme(e->index, EngineID::Invalid(), nullptr)));
	}
	for (const Vehicle *v : Vehicle::Iterate()) {
		if (!IsCompanyBuildableVehicleType(v->type) || !Engine::IsValidID(v->engine_type)) continue;
		EngineID parent = v == v->First() ? EngineID::Invalid() : v->First()->engine_type;
		fmt::print(stderr, "COMPANY-IDENTITY vehicle-scheme {} {}\n", v->index.base(), to_underlying(GetEngineLiveryScheme(v->engine_type, parent, v)));
	}
	fmt::print(stderr, "COMPANY-IDENTITY rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
}
static void CompanyIdentityResult(std::string_view label, const CommandCost &cost, bool success)
{
	fmt::print(stderr, "COMPANY-IDENTITY command {} {} {}\n", label, cost.Succeeded(), cost.GetErrorMessage());
	if (cost.Succeeded() != success) FixtureAbort(__LINE__);
}
static void CompanyIdentityProbe()
{
	Backup<CompanyID> company(_current_company, CompanyID(1));
	/* The save restores only _random; UI randomness is an explicit fixture input. */
	_interactive_random.SetSeed(0x561C0F1U);
	if (Company::GetNumItems() != MAX_COMPANIES) FixtureAbort(__LINE__);
	SavedRandomSeeds before, after;
	SaveRandomSeeds(&before);
	CompanyIdentityResult("capacity", Command<CMD_COMPANY_CTRL>::Do(DoCommandFlag::Execute, CCA_NEW_AI, CompanyID::Invalid(), CRR_NONE, static_cast<ClientID>(0)), true);
	SaveRandomSeeds(&after);
	if (Company::GetNumItems() != MAX_COMPANIES || !SameRandom(before, after)) FixtureAbort(__LINE__);
	Company *c = Company::Get(_current_company);
	CompanyIdentityResult("rename", Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, "Identity Company"), true);
	CompanyIdentityResult("rename-self", Command<CMD_RENAME_COMPANY>::Do({}, "Identity Company"), false);
	CompanyIdentityResult("president", Command<CMD_RENAME_PRESIDENT>::Do(DoCommandFlag::Execute, "Identity President"), true);
	_current_company = CompanyID(2);
	CompanyIdentityResult("rename-collision", Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, "Identity Company"), false);
	CompanyIdentityResult("president-collision", Command<CMD_RENAME_PRESIDENT>::Do(DoCommandFlag::Execute, "Identity President"), false);
	CompanyIdentityResult("president-auto-company", Command<CMD_RENAME_PRESIDENT>::Do(DoCommandFlag::Execute, "Identity Second"), true);
	if (Company::Get(CompanyID(2))->name != "Identity Second Transport") FixtureAbort(__LINE__);
	CompanyIdentityResult("auto-company-reset", Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, ""), true);
	CompanyIdentityResult("auto-president-reset", Command<CMD_RENAME_PRESIDENT>::Do(DoCommandFlag::Execute, ""), true);
	_current_company = CompanyID(1);
	CompanyIdentityResult("name-length", Command<CMD_RENAME_COMPANY>::Do({}, std::string(MAX_LENGTH_COMPANY_NAME_CHARS, 'x')), false);
	CompanyIdentityResult("president-length", Command<CMD_RENAME_PRESIDENT>::Do({}, std::string(MAX_LENGTH_PRESIDENT_NAME_CHARS, 'x')), false);
	CompanyIdentityResult("rename-reset", Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, ""), true);
	CompanyIdentityResult("president-reset", Command<CMD_RENAME_PRESIDENT>::Do(DoCommandFlag::Execute, ""), true);
	for (uint style : {GetNumCompanyManagerFaceStyles(), 65536U, UINT_MAX}) {
		CompanyIdentityResult("face-style", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do({}, style, 0U), false);
	}
	CompanyIdentityResult("face-bits", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do({}, 0U, UINT32_MAX), false);
	CompanyManagerFace face;
	for (uint i = 0; i < 8; i++) {
		RandomiseCompanyManagerFace(face, _interactive_random);
		CompanyIdentityResult("face-test", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do({}, face.style, face.bits), true);
		CompanyIdentityResult("face-execute", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do(DoCommandFlag::Execute, face.style, face.bits), true);
	}
	uint32_t active = 0;
	FaceVars vars = GetCompanyManagerFaceVars(face.style);
	for (const FaceVar &v : vars) active |= ((1U << v.length) - 1U) << v.offset;
	CompanyIdentityResult("face-unused", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do(DoCommandFlag::Execute, face.style, face.bits | ~active), true);
	const auto [root_cost, root] = Command<CMD_CREATE_GROUP>::Do(DoCommandFlag::Execute, VEH_ROAD, GroupID::Invalid());
	const auto [child_cost, child] = Command<CMD_CREATE_GROUP>::Do(DoCommandFlag::Execute, VEH_ROAD, root);
	CompanyIdentityResult("group-root", root_cost, true); CompanyIdentityResult("group-child", child_cost, true);
	CompanyIdentityResult("group-colour", Command<CMD_SET_GROUP_LIVERY>::Do(DoCommandFlag::Execute, child, false, COLOUR_RED), true);
	CompanyIdentityResult("colour-duplicate", Command<CMD_SET_COMPANY_COLOUR>::Do({}, LS_DEFAULT, true, Company::Get(CompanyID(2))->colour), false);
	CompanyIdentityResult("colour-invalid-default", Command<CMD_SET_COMPANY_COLOUR>::Do({}, LS_DEFAULT, true, INVALID_COLOUR), false);
	CompanyIdentityResult("colour-range", Command<CMD_SET_COMPANY_COLOUR>::Do({}, LS_BUS, true, static_cast<Colours>(COLOUR_END)), false);
	CompanyIdentityResult("colour-invalid-scheme", Command<CMD_SET_COMPANY_COLOUR>::Do({}, LS_END, true, COLOUR_BLUE), false);
	Colours unused = COLOUR_BEGIN;
	for (; unused < COLOUR_END; unused = static_cast<Colours>(unused + 1)) {
		bool used = false;
		for (const Company *cc : Company::Iterate()) used |= cc->colour == unused;
		if (!used) break;
	}
	CompanyIdentityResult("colour", Command<CMD_SET_COMPANY_COLOUR>::Do(DoCommandFlag::Execute, LS_DEFAULT, true, unused), true);
	CompanyIdentityResult("secondary", Command<CMD_SET_COMPANY_COLOUR>::Do(DoCommandFlag::Execute, LS_DEFAULT, false, COLOUR_GREEN), true);
	CompanyIdentityResult("scheme-set", Command<CMD_SET_COMPANY_COLOUR>::Do(DoCommandFlag::Execute, LS_BUS, true, COLOUR_BLUE), true);
	CompanyIdentityResult("scheme-reset", Command<CMD_SET_COMPANY_COLOUR>::Do(DoCommandFlag::Execute, LS_BUS, true, INVALID_COLOUR), true);
	if (Group::Get(root)->livery.colour1 != unused || Group::Get(child)->livery.colour1 != unused || Group::Get(child)->livery.colour2 != COLOUR_RED) FixtureAbort(__LINE__);
	ResetCompanyLivery(c);
	if (Group::Get(child)->livery.in_use.Any({Livery::Flag::Primary, Livery::Flag::Secondary}) || Group::Get(child)->livery.colour2 != unused) FixtureAbort(__LINE__);
	/* Same town input forces the second company's generated-name collision path. */
	for (CompanyID id : {CompanyID(1), CompanyID(2)}) {
		Company *cc = Company::Get(id); cc->name_1 = STR_SV_UNNAMED; cc->name_2 = 0; cc->last_build_coordinate = c->last_build_coordinate;
	}
	CompanyIdentityInspect();
	fmt::print(stderr, "COMPANY-IDENTITY complete true\n");
	company.Restore();
}

static void CompanyIdentityResume(uint32_t first, uint32_t second)
{
	/* Carry the observed interactive stream separately from the native save format. */
	_interactive_random.state[0] = first;
	_interactive_random.state[1] = second;
	Backup<CompanyID> company(_current_company, CompanyID(1));
	CompanyManagerFace face;
	RandomiseCompanyManagerFace(face, _interactive_random);
	CompanyIdentityResult("face-after-reload", Command<CMD_SET_COMPANY_MANAGER_FACE>::Do(DoCommandFlag::Execute, face.style, face.bits), true);
	CompanyIdentityInspect();
	company.Restore();
}

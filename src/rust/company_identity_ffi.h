/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_ffi.h Company identity owner and typed native operations. */
#ifndef RUST_COMPANY_IDENTITY_FFI_H
#define RUST_COMPANY_IDENTITY_FFI_H
#include "services_ffi.h"
#include "company_face_ffi.h"
#include "fleet_ffi.h"
#include "engine_ffi.h"
#include "../livery.h"
struct Company;
struct EngineRenew;
struct OpenTTDCommandCost;
struct OpenTTDCommandServices;
struct OpenTTDEngineCapabilitiesServices;
struct OpenTTDObjectServices;
struct OpenTTDTownAuthorityServices;
struct CompanyFinances;
struct VehicleDefaultSettings;
struct OpenTTDCompanyIdentityServices {
	const OpenTTDTownAuthorityServices *authority;
	const OpenTTDEngineCapabilitiesServices *capabilities;
	const OpenTTDCommandServices *commands;
	const OpenTTDFaceServices *faces;
	const OpenTTDFleetGroupServices *groups;
	Randomizer *randomizer;
	const OpenTTDSharedServices *shared;
	void (OPENTTD_RUST_CALL *admin_update)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(admin_update);
	void (OPENTTD_RUST_CALL *build_owner_legend)() noexcept OPENTTD_SERVICE_DEFAULT(build_owner_legend);
	uint8_t (OPENTTD_RUST_CALL *colour)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(colour);
	Company * (OPENTTD_RUST_CALL *company)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(company);
	Company * (OPENTTD_RUST_CALL *company_if_valid)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(company_if_valid);
	bool (OPENTTD_RUST_CALL *engine_renew)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(engine_renew);
	uint32_t (OPENTTD_RUST_CALL *engine_renew_money)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(engine_renew_money);
	int16_t (OPENTTD_RUST_CALL *engine_renew_months)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(engine_renew_months);
	CompanyManagerFace * (OPENTTD_RUST_CALL *face)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(face);
	CompanyFinances * (OPENTTD_RUST_CALL *finances)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(finances);
	uint8_t (OPENTTD_RUST_CALL *index)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(index);
	bool (OPENTTD_RUST_CALL *is_ai)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(is_ai);
	bool (OPENTTD_RUST_CALL *is_freight)(uint8_t cargo_type) noexcept OPENTTD_SERVICE_DEFAULT(is_freight);
	uint32_t (OPENTTD_RUST_CALL *last_build_coordinate)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(last_build_coordinate);
	Livery * (OPENTTD_RUST_CALL *livery)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *name)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name);
	uint32_t (OPENTTD_RUST_CALL *name_1)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_1);
	uint32_t (OPENTTD_RUST_CALL *name_2)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_2);
	Company * (OPENTTD_RUST_CALL *next_company)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_company);
	const OpenTTDObjectServices * (OPENTTD_RUST_CALL *objects)() noexcept OPENTTD_SERVICE_DEFAULT(objects);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *president_name)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_name);
	uint32_t (OPENTTD_RUST_CALL *president_name_1)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_name_1);
	uint32_t (OPENTTD_RUST_CALL *president_name_2)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_name_2);
	EngineRenew ** (OPENTTD_RUST_CALL *renewal_head)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(renewal_head);
	void (OPENTTD_RUST_CALL *reset_colourmap)() noexcept OPENTTD_SERVICE_DEFAULT(reset_colourmap);
	void (OPENTTD_RUST_CALL *set_colour)(Company *c, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_colour);
	void (OPENTTD_RUST_CALL *set_name)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	void (OPENTTD_RUST_CALL *set_name_1)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_1);
	void (OPENTTD_RUST_CALL *set_name_2)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_2);
	void (OPENTTD_RUST_CALL *set_president_name)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_president_name);
	void (OPENTTD_RUST_CALL *set_president_name_1)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_name_1);
	void (OPENTTD_RUST_CALL *set_president_name_2)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_name_2);
	VehicleDefaultSettings *(OPENTTD_RUST_CALL *vehicle_defaults)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_defaults);
};
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept;
extern "C" {
uint8_t *OPENTTD_RUST_CALL openttd_rust_company_colours();
void OPENTTD_RUST_CALL openttd_rust_company_reset_livery(const OpenTTDCompanyIdentityServices *, Company *);
void OPENTTD_RUST_CALL openttd_rust_company_update_liveries(const OpenTTDCompanyIdentityServices *, Company *);
void OPENTTD_RUST_CALL openttd_rust_company_manager_face(const OpenTTDCompanyIdentityServices *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_company_colour(const OpenTTDCompanyIdentityServices *, OpenTTDCommandCost *, uint16_t, LiveryScheme, bool, Colours);
void OPENTTD_RUST_CALL openttd_rust_company_rename(const OpenTTDCompanyIdentityServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan);
void OPENTTD_RUST_CALL openttd_rust_president_rename(const OpenTTDCompanyIdentityServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan);
LiveryScheme OPENTTD_RUST_CALL openttd_rust_engine_livery_scheme(const OpenTTDCompanyIdentityServices *, uint16_t, uint16_t, OpenTTDVehicle *);
}
#endif

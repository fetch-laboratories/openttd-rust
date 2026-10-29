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
struct Town;
struct alignas(16) OpenTTDCompanyNameStorage { std::byte bytes[128]; };
struct OpenTTDCompanyIdentityStrings {
	uint32_t unnamed, president, company, president_generated, andco, town_begin, town_end, company_begin;
};
struct OpenTTDCompanyIdentityServices {
	const OpenTTDSharedServices *shared;
	Randomizer *randomizer;
	const OpenTTDFaceServices *faces;
	const OpenTTDFleetGroupServices *groups;
	OpenTTDCompanyIdentityStrings strings;
	Company * (OPENTTD_RUST_CALL *company)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(company);
	Company * (OPENTTD_RUST_CALL *next_company)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_company);
	uint8_t (OPENTTD_RUST_CALL *company_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(company_id);
	uint32_t (OPENTTD_RUST_CALL *name_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_id);
	void (OPENTTD_RUST_CALL *set_name_id)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_id);
	uint32_t (OPENTTD_RUST_CALL *name_parts)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_parts);
	void (OPENTTD_RUST_CALL *set_name_parts)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_parts);
	uint32_t (OPENTTD_RUST_CALL *president_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_id);
	void (OPENTTD_RUST_CALL *set_president_id)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_id);
	uint32_t (OPENTTD_RUST_CALL *president_parts)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_parts);
	void (OPENTTD_RUST_CALL *set_president_parts)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_parts);
	uint8_t (OPENTTD_RUST_CALL *colour)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(colour);
	void (OPENTTD_RUST_CALL *set_colour)(Company *c, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_colour);
	uint32_t (OPENTTD_RUST_CALL *last_build)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(last_build);
	bool (OPENTTD_RUST_CALL *is_ai)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(is_ai);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *name)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (OPENTTD_RUST_CALL *set_name)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *president)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president);
	void (OPENTTD_RUST_CALL *set_president)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_president);
	Livery * (OPENTTD_RUST_CALL *livery)(Company *c, LiveryScheme scheme) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	CompanyManagerFace * (OPENTTD_RUST_CALL *face)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(face);
	void (OPENTTD_RUST_CALL *name_frame_init)(OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(name_frame_init);
	void (OPENTTD_RUST_CALL *name_frame_drop)(OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(name_frame_drop);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *render)(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept OPENTTD_SERVICE_DEFAULT(render);
	OpenTTDByteSpan (OPENTTD_RUST_CALL *render_other)(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept OPENTTD_SERVICE_DEFAULT(render_other);
	void (OPENTTD_RUST_CALL *company_renamed)(Company *c, OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(company_renamed);
	void (OPENTTD_RUST_CALL *president_renamed)(Company *c, OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(president_renamed);
	void (OPENTTD_RUST_CALL *launch_news)(Company *c, const Town *t) noexcept OPENTTD_SERVICE_DEFAULT(launch_news);
	void (OPENTTD_RUST_CALL *admin_update)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(admin_update);
	void (OPENTTD_RUST_CALL *rename_transport)(OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(rename_transport);
	void (OPENTTD_RUST_CALL *president_windows)() noexcept OPENTTD_SERVICE_DEFAULT(president_windows);
	void (OPENTTD_RUST_CALL *colour_windows)() noexcept OPENTTD_SERVICE_DEFAULT(colour_windows);
	void (OPENTTD_RUST_CALL *object_colours)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(object_colours);
	void (OPENTTD_RUST_CALL *dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	void (OPENTTD_RUST_CALL *reset_colourmap)() noexcept OPENTTD_SERVICE_DEFAULT(reset_colourmap);
	const OpenTTDEngine * (OPENTTD_RUST_CALL *engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	VehicleType (OPENTTD_RUST_CALL *engine_type)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_type);
	uint8_t (OPENTTD_RUST_CALL *engine_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_cargo);
	RailVehicleTypes (OPENTTD_RUST_CALL *engine_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_kind);
	EngineClass (OPENTTD_RUST_CALL *engine_class)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_class);
	uint8_t (OPENTTD_RUST_CALL *engine_misc)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_misc);
	uint8_t (OPENTTD_RUST_CALL *aircraft_subtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_subtype);
	uint8_t (OPENTTD_RUST_CALL *vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	OpenTTDVehicle * (OPENTTD_RUST_CALL *vehicle_first)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_first);
	bool (OPENTTD_RUST_CALL *articulated)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(articulated);
	bool (OPENTTD_RUST_CALL *wagon_override)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(wagon_override);
	bool (OPENTTD_RUST_CALL *cargo_freight)(uint8_t cargo) noexcept OPENTTD_SERVICE_DEFAULT(cargo_freight);
	uint16_t (OPENTTD_RUST_CALL *cargo_classes)(uint8_t cargo) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	uint32_t (OPENTTD_RUST_CALL *vehicle_id)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_id);
	void (OPENTTD_RUST_CALL *invalidate_vehicle)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_vehicle);
	uint8_t (OPENTTD_RUST_CALL *vehicle_owner)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_owner);
};
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept;
extern "C" {
uint8_t *OPENTTD_RUST_CALL openttd_rust_company_colours();
void OPENTTD_RUST_CALL openttd_rust_company_reset_livery(const OpenTTDCompanyIdentityServices *, Company *);
void OPENTTD_RUST_CALL openttd_rust_company_update_liveries(const OpenTTDCompanyIdentityServices *, Company *);
bool OPENTTD_RUST_CALL openttd_rust_company_manager_face(const OpenTTDCompanyIdentityServices *, uint32_t, uint32_t, bool);
bool OPENTTD_RUST_CALL openttd_rust_company_colour(const OpenTTDCompanyIdentityServices *, LiveryScheme, bool, Colours, bool);
uint8_t OPENTTD_RUST_CALL openttd_rust_company_rename(const OpenTTDCompanyIdentityServices *, OpenTTDByteSpan, bool);
uint8_t OPENTTD_RUST_CALL openttd_rust_president_rename(const OpenTTDCompanyIdentityServices *, OpenTTDByteSpan, bool);
LiveryScheme OPENTTD_RUST_CALL openttd_rust_engine_livery_scheme(const OpenTTDCompanyIdentityServices *, uint16_t, uint16_t, OpenTTDVehicle *);
}
#endif

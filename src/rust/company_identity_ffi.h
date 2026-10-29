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
	Company * (*company)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(company);
	Company * (*next_company)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next_company);
	uint8_t (*company_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(company_id);
	uint32_t (*name_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_id);
	void (*set_name_id)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_id);
	uint32_t (*name_parts)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name_parts);
	void (*set_name_parts)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_name_parts);
	uint32_t (*president_id)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_id);
	void (*set_president_id)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_id);
	uint32_t (*president_parts)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president_parts);
	void (*set_president_parts)(Company *c, uint32_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_president_parts);
	uint8_t (*colour)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(colour);
	void (*set_colour)(Company *c, uint8_t value) noexcept OPENTTD_SERVICE_DEFAULT(set_colour);
	uint32_t (*last_build)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(last_build);
	bool (*is_ai)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(is_ai);
	OpenTTDByteSpan (*name)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(name);
	void (*set_name)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	OpenTTDByteSpan (*president)(const Company *c) noexcept OPENTTD_SERVICE_DEFAULT(president);
	void (*set_president)(Company *c, OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(set_president);
	Livery * (*livery)(Company *c, LiveryScheme scheme) noexcept OPENTTD_SERVICE_DEFAULT(livery);
	CompanyManagerFace * (*face)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(face);
	void (*name_frame_init)(OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(name_frame_init);
	void (*name_frame_drop)(OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(name_frame_drop);
	OpenTTDByteSpan (*render)(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept OPENTTD_SERVICE_DEFAULT(render);
	OpenTTDByteSpan (*render_other)(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept OPENTTD_SERVICE_DEFAULT(render_other);
	void (*company_renamed)(Company *c, OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(company_renamed);
	void (*president_renamed)(Company *c, OpenTTDCompanyNameStorage *slot) noexcept OPENTTD_SERVICE_DEFAULT(president_renamed);
	void (*launch_news)(Company *c, const Town *t) noexcept OPENTTD_SERVICE_DEFAULT(launch_news);
	void (*admin_update)(uint8_t id) noexcept OPENTTD_SERVICE_DEFAULT(admin_update);
	void (*rename_transport)(OpenTTDByteSpan text) noexcept OPENTTD_SERVICE_DEFAULT(rename_transport);
	void (*president_windows)() noexcept OPENTTD_SERVICE_DEFAULT(president_windows);
	void (*colour_windows)() noexcept OPENTTD_SERVICE_DEFAULT(colour_windows);
	void (*object_colours)(Company *c) noexcept OPENTTD_SERVICE_DEFAULT(object_colours);
	uint8_t (*current)() noexcept OPENTTD_SERVICE_DEFAULT(current);
	void (*dirty)() noexcept OPENTTD_SERVICE_DEFAULT(dirty);
	size_t (*utf8_length)(const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(utf8_length);
	void (*reset_colourmap)() noexcept OPENTTD_SERVICE_DEFAULT(reset_colourmap);
	const OpenTTDEngine * (*engine)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(engine);
	VehicleType (*engine_type)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_type);
	uint8_t (*engine_cargo)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_cargo);
	RailVehicleTypes (*engine_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_kind);
	EngineClass (*engine_class)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_class);
	uint8_t (*engine_misc)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_misc);
	uint8_t (*aircraft_subtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_subtype);
	uint8_t (*vehicle_cargo)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_cargo);
	OpenTTDVehicle * (*vehicle_first)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_first);
	bool (*articulated)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(articulated);
	bool (*wagon_override)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(wagon_override);
	bool (*cargo_freight)(uint8_t cargo) noexcept OPENTTD_SERVICE_DEFAULT(cargo_freight);
	bool (*cargo_passengers)(uint8_t cargo) noexcept OPENTTD_SERVICE_DEFAULT(cargo_passengers);
	uint32_t (*vehicle_id)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_id);
	void (*invalidate_vehicle)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_vehicle);
	uint8_t (*vehicle_owner)(OpenTTDVehicle *v) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_owner);
};
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept;
extern "C" {
uint8_t *openttd_rust_company_colours();
void openttd_rust_company_reset_livery(const OpenTTDCompanyIdentityServices *, Company *);
void openttd_rust_company_update_liveries(const OpenTTDCompanyIdentityServices *, Company *);
bool openttd_rust_company_manager_face(const OpenTTDCompanyIdentityServices *, uint32_t, uint32_t, bool);
bool openttd_rust_company_colour(const OpenTTDCompanyIdentityServices *, LiveryScheme, bool, Colours, bool);
uint8_t openttd_rust_company_rename(const OpenTTDCompanyIdentityServices *, OpenTTDByteSpan, bool);
uint8_t openttd_rust_president_rename(const OpenTTDCompanyIdentityServices *, OpenTTDByteSpan, bool);
LiveryScheme openttd_rust_engine_livery_scheme(const OpenTTDCompanyIdentityServices *, uint16_t, uint16_t, OpenTTDVehicle *);
}
#endif

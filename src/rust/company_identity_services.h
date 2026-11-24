/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_services.h Canonical native company identity fields. */
#ifndef RUST_COMPANY_IDENTITY_SERVICES_H
#define RUST_COMPANY_IDENTITY_SERVICES_H
#include "company_identity_ffi.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../company_manager_face.h"
#include "../vehicle_func.h"
#include "services_cargo.h"
inline void OPENTTD_RUST_CALL Identity_admin_update(const Company *c) noexcept { CompanyAdminUpdate(c); }
inline uint8_t OPENTTD_RUST_CALL Identity_colour(const Company *c) noexcept { return c->colour; }
inline Company * OPENTTD_RUST_CALL Identity_company(uint8_t id) noexcept { return Company::Get(CompanyID{id}); }
inline CompanyManagerFace * OPENTTD_RUST_CALL Identity_face(Company *c) noexcept { return &c->face; }
inline uint8_t OPENTTD_RUST_CALL Identity_index(const Company *c) noexcept { return c->index.base(); }
inline bool OPENTTD_RUST_CALL Identity_is_ai(const Company *c) noexcept { return c->is_ai; }
inline bool OPENTTD_RUST_CALL Identity_is_freight(uint8_t cargo_type) noexcept { return RustCargoSpec(cargo_type)->is_freight; }
inline uint32_t OPENTTD_RUST_CALL Identity_last_build_coordinate(const Company *c) noexcept { return c->last_build_coordinate.base(); }
inline Livery * OPENTTD_RUST_CALL Identity_livery(Company *c) noexcept { return c->livery.data(); }
inline const Livery *OPENTTD_RUST_CALL Identity_default_livery(uint8_t owner) noexcept { return &Identity_livery(Identity_company(owner))[LS_DEFAULT]; }
inline OpenTTDByteSpan OPENTTD_RUST_CALL Identity_name(const Company *c) noexcept { return {reinterpret_cast<const uint8_t *>(c->name.data()), c->name.size()}; }
inline uint32_t OPENTTD_RUST_CALL Identity_name_1(const Company *c) noexcept { return c->name_1; }
inline uint32_t OPENTTD_RUST_CALL Identity_name_2(const Company *c) noexcept { return c->name_2; }
inline OpenTTDByteSpan OPENTTD_RUST_CALL Identity_president_name(const Company *c) noexcept { return {reinterpret_cast<const uint8_t *>(c->president_name.data()), c->president_name.size()}; }
inline uint32_t OPENTTD_RUST_CALL Identity_president_name_1(const Company *c) noexcept { return c->president_name_1; }
inline uint32_t OPENTTD_RUST_CALL Identity_president_name_2(const Company *c) noexcept { return c->president_name_2; }
inline void OPENTTD_RUST_CALL Identity_reset_colourmap() noexcept { ResetVehicleColourMap(); }
inline void OPENTTD_RUST_CALL Identity_set_colour(Company *c, uint8_t value) noexcept { c->colour = static_cast<Colours>(value); }
inline void OPENTTD_RUST_CALL Identity_set_name(Company *c, OpenTTDByteSpan text) noexcept { c->name.assign(reinterpret_cast<const char *>(text.data), text.size); }
inline void OPENTTD_RUST_CALL Identity_set_name_1(Company *c, uint32_t value) noexcept { c->name_1 = value; }
inline void OPENTTD_RUST_CALL Identity_set_name_2(Company *c, uint32_t value) noexcept { c->name_2 = value; }
inline void OPENTTD_RUST_CALL Identity_set_president_name(Company *c, OpenTTDByteSpan text) noexcept { c->president_name.assign(reinterpret_cast<const char *>(text.data), text.size); }
inline void OPENTTD_RUST_CALL Identity_set_president_name_1(Company *c, uint32_t value) noexcept { c->president_name_1 = value; }
inline void OPENTTD_RUST_CALL Identity_set_president_name_2(Company *c, uint32_t value) noexcept { c->president_name_2 = value; }
inline uint8_t OPENTTD_RUST_CALL Face_offset(const FaceVar *v) noexcept { return v->offset; }
inline uint8_t OPENTTD_RUST_CALL Face_length(const FaceVar *v) noexcept { return v->length; }
inline uint8_t OPENTTD_RUST_CALL Face_valid_values(const FaceVar *v) noexcept { return v->valid_values; }
inline uint8_t OPENTTD_RUST_CALL Face_type(const FaceVar *v) noexcept { return to_underlying(v->type); }
inline OpenTTDFaceVarToggle OPENTTD_RUST_CALL Face_data(const FaceVar *v) noexcept { const auto &[off, on] = std::get<std::pair<uint64_t, uint64_t>>(v->data); return {off, on}; }
inline uint32_t OPENTTD_RUST_CALL Face_count() noexcept { return GetNumCompanyManagerFaceStyles(); }
inline OpenTTDFaceVars OPENTTD_RUST_CALL Face_vars(uint32_t style) noexcept { auto vars = GetCompanyManagerFaceVars(style); return {vars.data(), vars.size()}; }
inline const FaceVar * OPENTTD_RUST_CALL Face_at(const FaceVar *vars, size_t i) noexcept { return &vars[i]; }
inline const FaceSpec * OPENTTD_RUST_CALL Face_spec(uint32_t style) noexcept { return GetCompanyManagerFaceSpec(style); }
inline void OPENTTD_RUST_CALL Face_set_style(CompanyManagerFace *face, uint32_t style) noexcept { face->style = style; }
inline void OPENTTD_RUST_CALL Face_set_style_label(CompanyManagerFace *face, const FaceSpec *spec) noexcept { face->style_label = spec->label; }
inline uint32_t OPENTTD_RUST_CALL Face_style(const CompanyManagerFace *face) noexcept { return face->style; }
inline uint32_t * OPENTTD_RUST_CALL Face_bits(CompanyManagerFace *face) noexcept { return &face->bits; }
#endif

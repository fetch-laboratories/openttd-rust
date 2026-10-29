/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_services.h Canonical native company identity fields. */
#ifndef RUST_COMPANY_IDENTITY_SERVICES_H
#define RUST_COMPANY_IDENTITY_SERVICES_H
#include "company_identity_ffi.h"
#include "../company_base.h"
#include "../company_manager_face.h"
#include "../object.h"
#include "services_cargo.h"
inline Company * Identity_company(uint8_t id) noexcept { return Company::Get(CompanyID{id}); }
inline uint8_t Identity_company_id(const Company *c) noexcept { return c->index.base(); }
inline uint32_t Identity_name_id(const Company *c) noexcept { return c->name_1; }
inline void Identity_set_name_id(Company *c, uint32_t value) noexcept { c->name_1 = value; }
inline uint32_t Identity_name_parts(const Company *c) noexcept { return c->name_2; }
inline void Identity_set_name_parts(Company *c, uint32_t value) noexcept { c->name_2 = value; }
inline uint32_t Identity_president_id(const Company *c) noexcept { return c->president_name_1; }
inline void Identity_set_president_id(Company *c, uint32_t value) noexcept { c->president_name_1 = value; }
inline uint32_t Identity_president_parts(const Company *c) noexcept { return c->president_name_2; }
inline void Identity_set_president_parts(Company *c, uint32_t value) noexcept { c->president_name_2 = value; }
inline uint8_t Identity_colour(const Company *c) noexcept { return c->colour; }
inline void Identity_set_colour(Company *c, uint8_t value) noexcept { c->colour = static_cast<Colours>(value); }
inline uint32_t Identity_last_build(const Company *c) noexcept { return c->last_build_coordinate.base(); }
inline bool Identity_is_ai(const Company *c) noexcept { return c->is_ai; }
inline OpenTTDByteSpan Identity_name(const Company *c) noexcept { return {reinterpret_cast<const uint8_t *>(c->name.data()), c->name.size()}; }
inline void Identity_set_name(Company *c, OpenTTDByteSpan text) noexcept { if (text.size == 0) c->name.clear(); else c->name.assign(reinterpret_cast<const char *>(text.data), text.size); }
inline OpenTTDByteSpan Identity_president(const Company *c) noexcept { return {reinterpret_cast<const uint8_t *>(c->president_name.data()), c->president_name.size()}; }
inline void Identity_set_president(Company *c, OpenTTDByteSpan text) noexcept { if (text.size == 0) c->president_name.clear(); else c->president_name.assign(reinterpret_cast<const char *>(text.data), text.size); }
inline Livery * Identity_livery(Company *c, LiveryScheme scheme) noexcept { return &c->livery[scheme]; }
inline CompanyManagerFace * Identity_face(Company *c) noexcept { return &c->face; }
void Identity_admin_update(uint8_t id) noexcept;
inline void Identity_object_colours(Company *c) noexcept { UpdateObjectColours(c); }
inline bool Identity_cargo_freight(uint8_t cargo) noexcept { return RustCargoSpec(cargo)->is_freight; }
inline bool Identity_cargo_passengers(uint8_t cargo) noexcept { return IsCargoInClass(cargo, CargoClass::Passengers); }
inline uint8_t Identity_goods() noexcept { return GetCargoTypeByLabel(CT_GOODS); }
inline uint8_t Face_offset(const FaceVar *v) noexcept { return v->offset; }
inline uint8_t Face_length(const FaceVar *v) noexcept { return v->length; }
inline uint8_t Face_valid_values(const FaceVar *v) noexcept { return v->valid_values; }
inline bool Face_toggle(const FaceVar *v) noexcept { return v->type == FaceVarType::Toggle; }
inline uint64_t Face_off(const FaceVar *v) noexcept { return std::get<std::pair<uint64_t, uint64_t>>(v->data).first; }
inline uint64_t Face_on(const FaceVar *v) noexcept { return std::get<std::pair<uint64_t, uint64_t>>(v->data).second; }
inline uint32_t Face_count() noexcept { return GetNumCompanyManagerFaceStyles(); }
inline OpenTTDFaceVars Face_vars(uint32_t style) noexcept { auto vars = GetCompanyManagerFaceVars(style); return {vars.data(), vars.size()}; }
inline const FaceVar * Face_at(const FaceVar *vars, size_t i) noexcept { return &vars[i]; }
inline const FaceSpec * Face_spec(uint32_t style) noexcept { return GetCompanyManagerFaceSpec(style); }
inline void Face_set_style(CompanyManagerFace *face, uint32_t style) noexcept { face->style = style; }
inline void Face_set_label(CompanyManagerFace *face, const FaceSpec *spec) noexcept { face->style_label = spec->label; }
inline uint32_t Face_style(const CompanyManagerFace *face) noexcept { return face->style; }
inline uint32_t * Face_bits(CompanyManagerFace *face) noexcept { return &face->bits; }
#endif

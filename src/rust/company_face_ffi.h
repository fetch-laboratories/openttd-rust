/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
#ifndef RUST_COMPANY_FACE_FFI_H
#define RUST_COMPANY_FACE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
struct CompanyManagerFace;
struct FaceVar;
struct FaceSpec;
struct Randomizer;
struct OpenTTDFaceVars { const FaceVar *data; size_t size; };
struct OpenTTDFaceServices {
	uint8_t (OPENTTD_RUST_CALL *offset)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(offset);
	uint8_t (OPENTTD_RUST_CALL *length)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(length);
	uint8_t (OPENTTD_RUST_CALL *valid_values)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(valid_values);
	bool (OPENTTD_RUST_CALL *toggle)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(toggle);
	uint64_t (OPENTTD_RUST_CALL *off)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(off);
	uint64_t (OPENTTD_RUST_CALL *on)(const FaceVar *) noexcept OPENTTD_SERVICE_DEFAULT(on);
	uint32_t (OPENTTD_RUST_CALL *count)() noexcept OPENTTD_SERVICE_DEFAULT(count);
	OpenTTDFaceVars (OPENTTD_RUST_CALL *vars)(uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(vars);
	const FaceVar * (OPENTTD_RUST_CALL *at)(const FaceVar *vars, size_t i) noexcept OPENTTD_SERVICE_DEFAULT(at);
	const FaceSpec * (OPENTTD_RUST_CALL *spec)(uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (OPENTTD_RUST_CALL *set_style)(CompanyManagerFace *face, uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(set_style);
	void (OPENTTD_RUST_CALL *set_label)(CompanyManagerFace *face, const FaceSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(set_label);
	uint32_t (OPENTTD_RUST_CALL *style)(const CompanyManagerFace *face) noexcept OPENTTD_SERVICE_DEFAULT(style);
	uint32_t * (OPENTTD_RUST_CALL *bits)(CompanyManagerFace *face) noexcept OPENTTD_SERVICE_DEFAULT(bits);
	uint32_t (OPENTTD_RUST_CALL *random)(Randomizer *r) noexcept OPENTTD_SERVICE_DEFAULT(random);
};
const OpenTTDFaceServices &GetRustFaceServices() noexcept;
extern "C" {
uint32_t OPENTTD_RUST_CALL openttd_rust_face_scale_bits(const OpenTTDFaceServices *, const FaceVar *, uint32_t);
uint32_t OPENTTD_RUST_CALL openttd_rust_face_get_bits(const OpenTTDFaceServices *, const FaceVar *, const CompanyManagerFace *);
void OPENTTD_RUST_CALL openttd_rust_face_set_bits(const OpenTTDFaceServices *, const FaceVar *, CompanyManagerFace *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_face_change_bits(const OpenTTDFaceServices *, const FaceVar *, CompanyManagerFace *, int8_t);
uint64_t OPENTTD_RUST_CALL openttd_rust_face_active(const OpenTTDFaceServices *, uint32_t, OpenTTDFaceVars);
void OPENTTD_RUST_CALL openttd_rust_face_scale(const OpenTTDFaceServices *, CompanyManagerFace *, OpenTTDFaceVars);
void OPENTTD_RUST_CALL openttd_rust_face_random_bits(const OpenTTDFaceServices *, CompanyManagerFace *, OpenTTDFaceVars, Randomizer *);
void OPENTTD_RUST_CALL openttd_rust_face_style(const OpenTTDFaceServices *, CompanyManagerFace *, uint32_t);
void OPENTTD_RUST_CALL openttd_rust_face_random(const OpenTTDFaceServices *, CompanyManagerFace *, Randomizer *);
uint32_t OPENTTD_RUST_CALL openttd_rust_face_mask(const OpenTTDFaceServices *, uint32_t, OpenTTDFaceVars);
}
#endif

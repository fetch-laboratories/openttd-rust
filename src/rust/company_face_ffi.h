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
struct OpenTTDFaceVariable { uint8_t offset, length, valid; bool toggle; uint64_t off, on; };
struct OpenTTDFaceServices {
	uint32_t (*count)() noexcept OPENTTD_SERVICE_DEFAULT(count);
	OpenTTDFaceVars (*vars)(uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(vars);
	OpenTTDFaceVariable (*variable)(const FaceVar *v) noexcept OPENTTD_SERVICE_DEFAULT(variable);
	const FaceVar * (*at)(const FaceVar *vars, size_t i) noexcept OPENTTD_SERVICE_DEFAULT(at);
	const FaceSpec * (*spec)(uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	void (*set_style)(CompanyManagerFace *face, uint32_t style) noexcept OPENTTD_SERVICE_DEFAULT(set_style);
	void (*set_label)(CompanyManagerFace *face, const FaceSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(set_label);
	uint32_t (*style)(const CompanyManagerFace *face) noexcept OPENTTD_SERVICE_DEFAULT(style);
	uint32_t * (*bits)(CompanyManagerFace *face) noexcept OPENTTD_SERVICE_DEFAULT(bits);
	uint32_t (*random)(Randomizer *r) noexcept OPENTTD_SERVICE_DEFAULT(random);
};
const OpenTTDFaceServices &GetRustFaceServices() noexcept;
extern "C" {
uint64_t openttd_rust_face_active(const OpenTTDFaceServices *, uint32_t, OpenTTDFaceVars);
void openttd_rust_face_scale(const OpenTTDFaceServices *, CompanyManagerFace *, OpenTTDFaceVars);
void openttd_rust_face_random_bits(const OpenTTDFaceServices *, CompanyManagerFace *, OpenTTDFaceVars, Randomizer *);
void openttd_rust_face_style(const OpenTTDFaceServices *, CompanyManagerFace *, uint32_t);
void openttd_rust_face_random(const OpenTTDFaceServices *, CompanyManagerFace *, Randomizer *);
uint32_t openttd_rust_face_mask(const OpenTTDFaceServices *, uint32_t, OpenTTDFaceVars);
}
#endif

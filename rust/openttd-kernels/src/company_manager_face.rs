/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `company_manager_face.h` bit helpers and existing `company_cmd.cpp` face entries.
//! Native faces, style/variable tables and randomizers remain borrowed for the
//! synchronous call. Their services provide live pointers; Rust retains none.
//! Style indices and face bits keep their complete raw integer domains.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
use crate::company_type::CompanyManagerFace;
/// Compatibility name for the native face marker owned by `company_type.h`.
pub type Face = CompanyManagerFace;
use crate::town::names::Randomizer;
#[repr(C)]
pub struct FaceVar {
    _private: [u8; 0],
}
#[repr(C)]
pub struct FaceSpec {
    _private: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaceVars {
    pub data: *const FaceVar,
    pub size: usize,
}
#[repr(C)]
pub struct Services {
    pub offset: extern "C" fn(*const FaceVar) -> u8,
    pub length: extern "C" fn(*const FaceVar) -> u8,
    pub valid_values: extern "C" fn(*const FaceVar) -> u8,
    pub toggle: extern "C" fn(*const FaceVar) -> bool,
    pub off: extern "C" fn(*const FaceVar) -> u64,
    pub on: extern "C" fn(*const FaceVar) -> u64,
    pub count: extern "C" fn() -> u32,
    pub vars: extern "C" fn(u32) -> FaceVars,
    pub at: extern "C" fn(*const FaceVar, usize) -> *const FaceVar,
    pub spec: extern "C" fn(u32) -> *const FaceSpec,
    pub set_style: extern "C" fn(*mut CompanyManagerFace, u32) -> (),
    pub set_label: extern "C" fn(*mut CompanyManagerFace, *const FaceSpec) -> (),
    pub style: extern "C" fn(*mut CompanyManagerFace) -> u32,
    pub bits: extern "C" fn(*mut CompanyManagerFace) -> *mut u32,
    pub random: extern "C" fn(*mut Randomizer) -> u32,
}
/// Get the company manager's requested face bits (`FaceVar::GetBits`).
fn get_bits(s: &Services, var: *const FaceVar, bits: u32) -> u32 {
    crate::bitmath_func::gb(bits, (s.offset)(var), (s.length)(var))
}
/// Set the company manager's face bits (`FaceVar::SetBits`).
fn set_bits(s: &Services, var: *const FaceVar, bits: &mut u32, value: u32) {
    crate::bitmath_func::sb(bits, (s.offset)(var), (s.length)(var), value);
}
/// Increase/decrease the face variable, wrapping within its valid range.
/// The native `FaceVar::ChangeBits` temporary narrows to `int8_t`.
fn change_bits(s: &Services, var: *const FaceVar, cmf: *mut CompanyManagerFace, amount: i8) {
    let p = (s.bits)(cmf);
    // The new value for the face variable.
    let mut value = (get_bits(s, var, unsafe { p.read() }) as i8).wrapping_add(amount);
    // Scale the new value to the correct scope.
    while value < 0 {
        value = (i32::from(value) + i32::from((s.valid_values)(var))) as i8;
    }
    value = (i32::from(value) % i32::from((s.valid_values)(var))) as i8;
    // Save the new value.
    let mut bits = unsafe { p.read() };
    set_bits(s, var, &mut bits, value as u32);
    unsafe {
        p.write(bits);
    }
}
/// Check whether the face variable's bits have a valid range (`FaceVar::IsValid`).
fn is_valid(s: &Services, var: *const FaceVar, bits: u32) -> bool {
    get_bits(s, var, bits) < u32::from((s.valid_values)(var))
}
/// Get active face variables; toggles can make other variables inactive.
pub(crate) fn get_active_face_vars(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    let mut mask = (1u64 << vars.size) - 1;
    for i in 0..vars.size {
        let v = (s.at)(vars.data, i);
        if (s.toggle)(v) {
            mask &= !if crate::bitmath_func::has_bit(bits, (s.offset)(v)) {
                (s.on)(v)
            } else {
                (s.off)(v)
            };
        }
    }
    mask
}
/// Check the encoding; unused bits are not enforced to be zero.
/// Original `company_cmd.cpp::IsValidCompanyManagerFace`.
pub(crate) fn is_valid_company_manager_face(s: &Services, style: u32, bits: u32) -> bool {
    if style >= (s.count)() {
        return false;
    }
    // Test if each enabled part is valid.
    let vars = (s.vars)(style);
    let mask = get_active_face_vars(s, bits, vars);
    for i in 0..vars.size {
        if mask & (1 << i) != 0 {
            let v = (s.at)(vars.data, i);
            if !is_valid(s, v, bits) {
                return false;
            }
        }
    }
    true
}
/// Scale all active company manager face variables to the correct scope.
pub(crate) fn scale_all_company_manager_face_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
) {
    let bits = unsafe { (s.bits)(cmf).read() };
    let mask = get_active_face_vars(s, bits, vars);
    for i in 0..vars.size {
        if mask & (1 << i) != 0 {
            change_bits(s, (s.at)(vars.data, i), cmf, 0);
        }
    }
}
/// Set a company face style, changing both the style index and the label.
/// Original `company_cmd.cpp::SetCompanyManagerFaceStyle`.
pub(crate) fn set_company_manager_face_style(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    value: u32,
) {
    let spec = (s.spec)(value);
    assert!(!spec.is_null());
    (s.set_style)(cmf, value);
    (s.set_label)(cmf, spec);
}
/// Make a random new face without changing its style, using the passed randomizer.
pub(crate) fn randomise_company_manager_face_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
    r: *mut Randomizer,
) {
    unsafe {
        (s.bits)(cmf).write((s.random)(r));
    }
    scale_all_company_manager_face_bits(s, cmf, vars);
}
/// Completely randomise a face, including style (`company_cmd.cpp`).
/// The caller passes the world or interactive randomizer appropriate to its use.
/// The style draw precedes the face-bit draw from that same randomizer.
pub(crate) fn randomise_company_manager_face(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    r: *mut Randomizer,
) {
    let count = (s.count)();
    let value = crate::random_func::scale_to_limit((s.random)(r), count);
    set_company_manager_face_style(s, cmf, value);
    randomise_company_manager_face_bits(s, cmf, (s.vars)((s.style)(cmf)), r);
}
/// Scale a face variable to its correct scope (`FaceVar::ScaleBits`).
/// The value must be below 2 to the power of the variable's bit length.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_scale_bits(
    s: &Services,
    var: *const FaceVar,
    value: u32,
) -> u32 {
    let length = u32::from((s.length)(var));
    assert!(value < 1u32.wrapping_shl(length));
    value
        .wrapping_mul(u32::from((s.valid_values)(var)))
        .wrapping_shr(length)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_get_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut Face,
) -> u32 {
    get_bits(s, var, unsafe { (s.bits)(cmf).read() })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_set_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut CompanyManagerFace,
    value: u32,
) {
    let p = (s.bits)(cmf);
    let mut bits = unsafe { p.read() };
    set_bits(s, var, &mut bits, value);
    unsafe {
        p.write(bits);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_change_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut CompanyManagerFace,
    amount: i8,
) {
    change_bits(s, var, cmf, amount);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_active(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    get_active_face_vars(s, bits, vars)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_scale(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
) {
    scale_all_company_manager_face_bits(s, cmf, vars);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_random_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
    r: *mut Randomizer,
) {
    randomise_company_manager_face_bits(s, cmf, vars, r);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_style(s: &Services, cmf: *mut CompanyManagerFace, value: u32) {
    set_company_manager_face_style(s, cmf, value);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_random(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    r: *mut Randomizer,
) {
    randomise_company_manager_face(s, cmf, r);
}
/// Mask face bits without updating the face itself; unused bits are cleared.
/// Original `company_cmd.cpp::MaskCompanyManagerFaceBits`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_mask(s: &Services, bits: u32, vars: FaceVars) -> u32 {
    let mut result = 0;
    let mask = get_active_face_vars(s, bits, vars);
    for i in 0..vars.size {
        if mask & (1 << i) != 0 {
            let v = (s.at)(vars.data, i);
            set_bits(s, v, &mut result, get_bits(s, v, bits));
        }
    }
    result
}

crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDFaceVars" ["company_economy"] => FaceVars { data, size, };
 "OpenTTDFaceServices" ["company_economy"] => Services { offset, length, valid_values, toggle, off, on, count, vars, at, spec, set_style, set_label, style, bits, random, };
}

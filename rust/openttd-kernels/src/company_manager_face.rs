/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Company-manager face bit helpers; native style/variable tables remain borrowed.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
/// Opaque native company-manager face; its string stays native.
#[repr(C)]
pub struct Face {
    _private: [u8; 0],
}
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
    pub set_style: extern "C" fn(*mut Face, u32) -> (),
    pub set_label: extern "C" fn(*mut Face, *const FaceSpec) -> (),
    pub style: extern "C" fn(*mut Face) -> u32,
    pub bits: extern "C" fn(*mut Face) -> *mut u32,
    pub random: extern "C" fn(*mut Randomizer) -> u32,
}
/// `FaceVar::GetBits`, using the canonical `GB` callee.
fn get_bits(s: &Services, var: *const FaceVar, bits: u32) -> u32 {
    crate::bitmath_func::gb(bits, (s.offset)(var), (s.length)(var))
}
/// `FaceVar::SetBits`, using the canonical `SB` callee.
fn set_bits(s: &Services, var: *const FaceVar, bits: &mut u32, value: u32) {
    crate::bitmath_func::sb(bits, (s.offset)(var), (s.length)(var), value);
}
/// `FaceVar::ChangeBits`; the native temporary narrows to `int8_t` before wrapping.
fn change_bits(s: &Services, var: *const FaceVar, face: *mut Face, amount: i8) {
    let p = (s.bits)(face);
    let mut value = (get_bits(s, var, unsafe { p.read() }) as i8).wrapping_add(amount);
    while value < 0 {
        value = (i32::from(value) + i32::from((s.valid_values)(var))) as i8;
    }
    value = (i32::from(value) % i32::from((s.valid_values)(var))) as i8;
    let mut bits = unsafe { p.read() };
    set_bits(s, var, &mut bits, value as u32);
    unsafe {
        p.write(bits);
    }
}
/// `FaceVar::IsValid`.
fn is_valid(s: &Services, var: *const FaceVar, bits: u32) -> bool {
    get_bits(s, var, bits) < u32::from((s.valid_values)(var))
}
pub(crate) fn active(s: &Services, bits: u32, vars: FaceVars) -> u64 {
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
pub(crate) fn valid(s: &Services, style: u32, bits: u32) -> bool {
    if style >= (s.count)() {
        return false;
    }
    let vars = (s.vars)(style);
    let mask = active(s, bits, vars);
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
pub(crate) fn scale(s: &Services, face: *mut Face, vars: FaceVars) {
    let bits = unsafe { (s.bits)(face).read() };
    let mask = active(s, bits, vars);
    for i in 0..vars.size {
        if mask & (1 << i) != 0 {
            change_bits(s, (s.at)(vars.data, i), face, 0);
        }
    }
}
pub(crate) fn style(s: &Services, face: *mut Face, value: u32) {
    let spec = (s.spec)(value);
    assert!(!spec.is_null());
    (s.set_style)(face, value);
    (s.set_label)(face, spec);
}
pub(crate) fn random_bits(s: &Services, face: *mut Face, vars: FaceVars, r: *mut Randomizer) {
    unsafe {
        (s.bits)(face).write((s.random)(r));
    }
    scale(s, face, vars);
}
pub(crate) fn random(s: &Services, face: *mut Face, r: *mut Randomizer) {
    let count = (s.count)();
    let value = crate::random_func::scale_to_limit((s.random)(r), count);
    style(s, face, value);
    random_bits(s, face, (s.vars)((s.style)(face)), r);
}
/// Original `FaceVar` scaling used when loading old face encodings.
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
    face: *mut Face,
) -> u32 {
    get_bits(s, var, unsafe { (s.bits)(face).read() })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_set_bits(
    s: &Services,
    var: *const FaceVar,
    face: *mut Face,
    value: u32,
) {
    let p = (s.bits)(face);
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
    face: *mut Face,
    amount: i8,
) {
    change_bits(s, var, face, amount);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_active(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    active(s, bits, vars)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_scale(s: &Services, face: *mut Face, vars: FaceVars) {
    scale(s, face, vars);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_random_bits(
    s: &Services,
    face: *mut Face,
    vars: FaceVars,
    r: *mut Randomizer,
) {
    random_bits(s, face, vars, r);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_style(s: &Services, face: *mut Face, value: u32) {
    style(s, face, value);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_random(s: &Services, face: *mut Face, r: *mut Randomizer) {
    random(s, face, r);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_face_mask(s: &Services, bits: u32, vars: FaceVars) -> u32 {
    let mut result = 0;
    let mask = active(s, bits, vars);
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

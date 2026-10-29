/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Company-manager face bit helpers; native style/variable tables remain borrowed.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
use crate::company_identity::Face;
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
#[derive(Clone, Copy)]
pub struct Variable {
    pub offset: u8,
    pub length: u8,
    pub valid: u8,
    pub toggle: bool,
    pub off: u64,
    pub on: u64,
}
#[repr(C)]
pub struct Services {
    pub count: extern "C" fn() -> u32,
    pub vars: extern "C" fn(u32) -> FaceVars,
    pub variable: extern "C" fn(*const FaceVar) -> Variable,
    pub at: extern "C" fn(*const FaceVar, usize) -> *const FaceVar,
    pub spec: extern "C" fn(u32) -> *const FaceSpec,
    pub set_style: extern "C" fn(*mut Face, u32) -> (),
    pub set_label: extern "C" fn(*mut Face, *const FaceSpec) -> (),
    pub style: extern "C" fn(*mut Face) -> u32,
    pub bits: extern "C" fn(*mut Face) -> *mut u32,
    pub random: extern "C" fn(*mut Randomizer) -> u32,
}
fn get(bits: u32, v: Variable) -> u32 {
    (bits >> v.offset) & ((1u32 << v.length) - 1)
}
fn set(bits: &mut u32, v: Variable, value: u32) {
    let mask = ((1u32 << v.length) - 1) << v.offset;
    *bits = (*bits & !mask) | (value << v.offset);
}
pub(crate) fn active(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    let mut mask = (1u64 << vars.size) - 1;
    for i in 0..vars.size {
        let v = (s.variable)((s.at)(vars.data, i));
        if v.toggle {
            mask &= !if bits & (1 << v.offset) != 0 {
                v.on
            } else {
                v.off
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
            let v = (s.variable)((s.at)(vars.data, i));
            if get(bits, v) >= u32::from(v.valid) {
                return false;
            }
        }
    }
    true
}
pub(crate) fn scale(s: &Services, face: *mut Face, vars: FaceVars) {
    let p = (s.bits)(face);
    let mut bits = unsafe { p.read() };
    let mask = active(s, bits, vars);
    for i in 0..vars.size {
        if mask & (1 << i) != 0 {
            let v = (s.variable)((s.at)(vars.data, i));
            let mut value = get(bits, v) as i8;
            while value < 0 {
                value = (i32::from(value) + i32::from(v.valid)) as i8;
            }
            value = (i32::from(value) % i32::from(v.valid)) as i8;
            set(&mut bits, v, value as u32);
        }
    }
    unsafe {
        p.write(bits);
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
            let v = (s.variable)((s.at)(vars.data, i));
            set(&mut result, v, get(bits, v));
        }
    }
    result
}

crate::abi::layouts! {
 fn abi_layout;
 "OpenTTDFaceVars" ["company_economy"] => FaceVars { data, size, };
 "OpenTTDFaceVariable" ["company_economy"] => Variable { offset, length, valid, toggle, off, on, };
 "OpenTTDFaceServices" ["company_economy"] => Services { count, vars, variable, at, spec, set_style, set_label, style, bits, random, };
}

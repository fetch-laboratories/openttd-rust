/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `company_manager_face.h`: the face variables' bit helpers, with the face entries of
//! `company_cmd.cpp`. Faces, face variables and face specs are native records reached
//! through `Services`; a randomizer is the caller's native `Randomizer`. Every pointer is
//! live for the synchronous call, and no Rust reference to native state outlives one
//! field access or one draw.
use crate::bitmath_func::{SetBitIterator, gb, has_bit, sb};
use crate::company_type::CompanyManagerFace;
use crate::random_func::Randomizer;
use crate::services_assert::assert_with;

/// `FaceVarType`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceVarType(pub u8);

#[allow(non_upper_case_globals)]
impl FaceVarType {
    /// Native `FaceVarType::Sprite`.
    pub const Sprite: Self = Self(0);
    /// Native `FaceVarType::Palette`.
    pub const Palette: Self = Self(1);
    /// Native `FaceVarType::Toggle`.
    pub const Toggle: Self = Self(2);
}

/// Information about the valid values of `CompanyManagerFace` bitgroups as well as the
/// sprites to draw; a native record.
#[repr(C)]
pub struct FaceVar {
    _private: [u8; 0],
}

/// A face style's label and variables; a native record.
#[repr(C)]
pub struct FaceSpec {
    _private: [u8; 0],
}

/// `FaceVars`, the native `std::span<const FaceVar>`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaceVars {
    /// The first variable.
    pub data: *const FaceVar,
    /// The number of variables.
    pub size: usize,
}

/// The `std::pair<uint64_t, uint64_t>` alternative of a toggle variable's `data`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaceVarToggle {
    /// The variables a cleared toggle bit makes inactive.
    pub off: u64,
    /// The variables a set toggle bit makes inactive.
    pub on: u64,
}

/// The native face records' members and the unported `_faces` lookups.
#[repr(C)]
pub struct Services {
    /// `&vars[i]`.
    pub at: extern "C" fn(*const FaceVar, usize) -> *const FaceVar,
    /// `&cmf.bits`.
    pub bits: extern "C" fn(*mut CompanyManagerFace) -> *mut u32,
    /// `GetNumCompanyManagerFaceStyles()`.
    pub count: extern "C" fn() -> u32,
    /// `std::get<std::pair<uint64_t, uint64_t>>(var.data)`.
    pub data: extern "C" fn(*const FaceVar) -> FaceVarToggle,
    /// `FaceVar::length`.
    pub length: extern "C" fn(*const FaceVar) -> u8,
    /// `FaceVar::offset`.
    pub offset: extern "C" fn(*const FaceVar) -> u8,
    /// `cmf.style = style`.
    pub set_style: extern "C" fn(*mut CompanyManagerFace, u32) -> (),
    /// `cmf.style_label = spec->label`, a native string copy.
    pub set_style_label: extern "C" fn(*mut CompanyManagerFace, *const FaceSpec) -> (),
    /// `GetCompanyManagerFaceSpec(style)`.
    pub spec: extern "C" fn(u32) -> *const FaceSpec,
    /// `cmf.style`.
    pub style: extern "C" fn(*mut CompanyManagerFace) -> u32,
    /// `FaceVar::type`.
    pub r#type: extern "C" fn(*const FaceVar) -> FaceVarType,
    /// `FaceVar::valid_values`.
    pub valid_values: extern "C" fn(*const FaceVar) -> u8,
    /// `GetCompanyManagerFaceVars(style)`.
    pub vars: extern "C" fn(u32) -> FaceVars,
}

/// `cmf.bits`.
#[allow(unsafe_code)] // One load through the face's live `bits` member.
fn face_bits(s: &Services, cmf: *mut CompanyManagerFace) -> u32 {
    // SAFETY: The module contract: the face is live for the call.
    unsafe { (s.bits)(cmf).read() }
}

/// `cmf.bits = value`.
#[allow(unsafe_code)] // One store through the face's live `bits` member.
pub(crate) fn set_face_bits(s: &Services, cmf: *mut CompanyManagerFace, value: u32) {
    // SAFETY: The module contract: the face is live and writable for the call.
    unsafe { (s.bits)(cmf).write(value) }
}

/// `randomizer.Next()` on the caller's native randomizer.
#[allow(unsafe_code)] // One draw through the caller's live randomizer.
fn next(randomizer: *mut Randomizer) -> u32 {
    // SAFETY: The module contract: the randomizer is live and serialized for the call; the
    // reference ends with the draw.
    unsafe { (*randomizer).next() }
}

/// Gets the company manager's face bits. `bits` is the face's `bits` member. Returns the
/// requested bits.
fn get_bits(s: &Services, var: *const FaceVar, bits: u32) -> u32 {
    gb(bits, (s.offset)(var), (s.length)(var))
}

/// Sets the company manager's face bits. `bits` is the face's `bits` member and `val` the
/// new value.
fn set_bits(s: &Services, var: *const FaceVar, bits: &mut u32, val: u32) {
    sb(bits, (s.offset)(var), (s.length)(var), val);
}

/// Increase/Decrease the company manager's face variable by the given amount. The value
/// wraps around to stay in the valid range.
fn change_bits(s: &Services, var: *const FaceVar, cmf: *mut CompanyManagerFace, amount: i8) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "`FaceVar::ChangeBits` holds the sum in an `int8_t`."
    )]
    let mut val = (get_bits(s, var, face_bits(s, cmf)) as i8).wrapping_add(amount); // the new value for the cmfv

    // scales the new value to the correct scope
    while val < 0 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "`val += this->valid_values` narrows the `int` sum to `int8_t`."
        )]
        let sum = (i32::from(val) + i32::from((s.valid_values)(var))) as i8;
        val = sum;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "`val %= this->valid_values` narrows the `int` remainder to `int8_t`."
    )]
    let remainder = (i32::from(val) % i32::from((s.valid_values)(var))) as i8;
    val = remainder;

    #[expect(
        clippy::cast_sign_loss,
        reason = "`SetBits(cmf, val)` converts the `int8_t` to its `uint` parameter."
    )]
    let val = val as u32;
    let mut bits = face_bits(s, cmf);
    set_bits(s, var, &mut bits, val); // save the new value
    set_face_bits(s, cmf, bits);
}

/// Checks whether the company manager's face bits have a valid range. `bits` is the face's
/// `bits` member. Returns true if and only if the bits are valid.
fn is_valid(s: &Services, var: *const FaceVar, bits: u32) -> bool {
    gb(bits, (s.offset)(var), (s.length)(var)) < u32::from((s.valid_values)(var))
}

/// Scales a company manager's face bits variable to the correct scope.
/// Precondition: `val < (1U << length)`, i.e. `val` has a value of 0..2^(bits used for this
/// variable)-1. Returns the scaled value.
fn scale_bits(s: &Services, var: *const FaceVar, val: u32) -> u32 {
    let length = u32::from((s.length)(var));
    assert_with(|| val < 1u32.wrapping_shl(length));
    val.wrapping_mul(u32::from((s.valid_values)(var)))
        .wrapping_shr(length)
}

/// Get a bitmask of currently active face variables.
/// Face variables can be inactive due to toggles in the face variables.
/// Takes the face's `bits` member, the only part of the face it reads, since Rust cannot
/// build the native face `CmdSetCompanyManagerFace` checks. Returns the currently active
/// face variables for the face.
pub(crate) fn get_active_face_vars(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    let mut active_vars = (1u64 << vars.size) - 1;

    for i in 0..vars.size {
        let info = (s.at)(vars.data, i);
        if (s.r#type)(info) != FaceVarType::Toggle {
            continue;
        }
        let FaceVarToggle { off, on } = (s.data)(info);
        active_vars &= !if has_bit(bits, (s.offset)(info)) {
            on
        } else {
            off
        };
    }

    active_vars
}

/// Checks whether a company manager's face is a valid encoding. Unused bits are not
/// enforced to be 0.
/// Takes the face's `style` and `bits`, since Rust cannot build the native face
/// `CmdSetCompanyManagerFace` checks. Returns true if and only if the face is valid.
pub(crate) fn is_valid_company_manager_face(s: &Services, style: u32, bits: u32) -> bool {
    if style >= (s.count)() {
        return false;
    }

    // Test if each enabled part is valid.
    let vars = (s.vars)(style);
    for var in SetBitIterator::new(get_active_face_vars(s, bits, vars)) {
        if !is_valid(s, (s.at)(vars.data, usize::from(var)), bits) {
            return false;
        }
    }

    true
}

/// Scales all company manager's face bits to the correct scope. `cmf` is the company
/// manager's face to write the bits to.
pub(crate) fn scale_all_company_manager_face_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
) {
    for var in SetBitIterator::new(get_active_face_vars(s, face_bits(s, cmf), vars)) {
        change_bits(s, (s.at)(vars.data, usize::from(var)), cmf, 0);
    }
}

/// Make a random new face without changing the face style. `cmf` is the company manager's
/// face to write the bits to, `vars` the face variables and `randomizer` the source of
/// random to use for creating the manager face.
pub(crate) fn randomise_company_manager_face_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
    randomizer: *mut Randomizer,
) {
    set_face_bits(s, cmf, next(randomizer));
    scale_all_company_manager_face_bits(s, cmf, vars);
}

/// Set a company face style. Changes both the style index and the label.
pub(crate) fn set_company_manager_face_style(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    style: u32,
) {
    let spec = (s.spec)(style);
    assert_with(|| !spec.is_null());

    (s.set_style)(cmf, style);
    (s.set_style_label)(cmf, spec);
}

/// Completely randomise a company manager face, including style.
/// Note: the randomizer passed should be appropriate for server-side or client-side usage.
pub(crate) fn randomise_company_manager_face(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    randomizer: *mut Randomizer,
) {
    // `randomizer.Next(limit)`.
    let style = crate::random_func::scale_to_limit(next(randomizer), (s.count)());
    set_company_manager_face_style(s, cmf, style);
    randomise_company_manager_face_bits(s, cmf, (s.vars)((s.style)(cmf)), randomizer);
}

/// Mask company manager face bits to ensure they are all within range.
/// Note: does not update the `CompanyManagerFace` itself. Unused bits are cleared.
/// Takes the face's `bits` member, the only part of the face it reads. Returns the masked
/// face bits.
fn mask_company_manager_face_bits(s: &Services, bits: u32, vars: FaceVars) -> u32 {
    // `CompanyManagerFace face{}`: only its bits are written and returned.
    let mut face = 0;

    for var in SetBitIterator::new(get_active_face_vars(s, bits, vars)) {
        let var = (s.at)(vars.data, usize::from(var));
        set_bits(s, var, &mut face, get_bits(s, var, bits));
    }

    face
}

/// `FaceVar::ScaleBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_scale_bits(
    s: &Services,
    var: *const FaceVar,
    val: u32,
) -> u32 {
    scale_bits(s, var, val)
}

/// `FaceVar::GetBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_get_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut CompanyManagerFace,
) -> u32 {
    get_bits(s, var, face_bits(s, cmf))
}

/// `FaceVar::SetBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_set_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut CompanyManagerFace,
    val: u32,
) {
    let mut bits = face_bits(s, cmf);
    set_bits(s, var, &mut bits, val);
    set_face_bits(s, cmf, bits);
}

/// `FaceVar::ChangeBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_change_bits(
    s: &Services,
    var: *const FaceVar,
    cmf: *mut CompanyManagerFace,
    amount: i8,
) {
    change_bits(s, var, cmf, amount);
}

/// `GetActiveFaceVars`, given the face's `bits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_active(s: &Services, bits: u32, vars: FaceVars) -> u64 {
    get_active_face_vars(s, bits, vars)
}

/// `ScaleAllCompanyManagerFaceBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_scale(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
) {
    scale_all_company_manager_face_bits(s, cmf, vars);
}

/// `RandomiseCompanyManagerFaceBits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_random_bits(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    vars: FaceVars,
    randomizer: *mut Randomizer,
) {
    randomise_company_manager_face_bits(s, cmf, vars, randomizer);
}

/// `SetCompanyManagerFaceStyle`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_style(s: &Services, cmf: *mut CompanyManagerFace, style: u32) {
    set_company_manager_face_style(s, cmf, style);
}

/// `RandomiseCompanyManagerFace`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_random(
    s: &Services,
    cmf: *mut CompanyManagerFace,
    randomizer: *mut Randomizer,
) {
    randomise_company_manager_face(s, cmf, randomizer);
}

/// `MaskCompanyManagerFaceBits`, given the face's `bits`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_face_mask(s: &Services, bits: u32, vars: FaceVars) -> u32 {
    mask_company_manager_face_bits(s, bits, vars)
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFaceVars" ["company_economy"] => FaceVars { data, size, };
    "OpenTTDFaceVarToggle" ["company_economy"] => FaceVarToggle { off, on, };
    "OpenTTDFaceServices" ["company_economy"] => Services {
        at,
        bits,
        count,
        data,
        length,
        offset,
        set_style,
        set_style_label,
        spec,
        style,
        r#type => type,
        valid_values,
        vars,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static FAILED_CHECKS: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_failed_check(condition: bool) {
        if !condition {
            FAILED_CHECKS.fetch_add(1, Ordering::Relaxed);
        }
    }
    extern "C" fn length(_: *const FaceVar) -> u8 {
        3
    }
    extern "C" fn valid_values(_: *const FaceVar) -> u8 {
        5
    }

    /// `FaceVar::ScaleBits` asserts `val < (1U << length)` before it scales: a value of
    /// `1 << length` fails the native assertion once, and valid values none.
    #[test]
    fn scale_bits_asserts_the_value_fits_its_length() {
        if crate::services_assert::isolate_test_check(
            "company_manager_face::tests::scale_bits_asserts_the_value_fits_its_length",
            record_failed_check,
        ) {
            return;
        }
        let mut faces = crate::test_tables::FACES;
        faces.length = length;
        faces.valid_values = valid_values;
        let var = std::ptr::NonNull::<FaceVar>::dangling().as_ptr().cast_const();
        assert_eq!(scale_bits(&faces, var, 7), 4);
        assert_eq!(scale_bits(&faces, var, 0), 0);
        assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 0);
        assert_eq!(scale_bits(&faces, var, 8), 5);
        assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 1);
    }
}

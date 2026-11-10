/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Name validation/retry policy; native resolvers retain language/NewGRF handling.
use super::founding::observed;
use super::{State, TownRef};
use crate::services::ByteSpan;
use crate::town_type::MAX_LENGTH_TOWN_NAME_CHARS;
use crate::witness as w;
use std::collections::BTreeSet;

#[repr(C)]
pub struct NameFrame {
    _private: [u8; 0],
}
pub use crate::townname_type::NameParams;
#[repr(C)]
pub struct Randomizer {
    _private: [u8; 0],
}
pub type Names = BTreeSet<Vec<u8>>;
#[repr(C)]
pub struct NameServices {
    pub next: extern "C" fn(u32) -> TownRef,
    pub render: extern "C" fn(*mut NameFrame, *const NameParams, u32, bool) -> ByteSpan,
    pub shared: *const crate::services::Services,
    pub utf8_length: extern "C" fn(*const u8, usize) -> usize,
    pub random_next: extern "C" fn(*mut Randomizer) -> u32,
}
/// SAFETY: Input bytes live for the synchronous call; empty native spans may be null.
pub(super) unsafe fn bytes<'a>(value: ByteSpan) -> &'a [u8] {
    if value.size == 0 {
        &[]
    } else {
        // SAFETY: Producer guarantees readable bytes and full byte count.
        unsafe { std::slice::from_raw_parts(value.data, value.size) }
    }
}
/// # Safety
/// The owner is live and serialized by the game mutex. Returned bytes remain valid until its next name mutation or destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_name(state: *const State) -> ByteSpan {
    unsafe { town_name(state) }
}
/// `Town::name`: the town's custom name, empty when it has none.
/// # Safety
/// The owner is live and serialized by the game mutex. Returned bytes remain valid until its next name mutation or destruction.
pub(crate) unsafe fn town_name(state: *const State) -> ByteSpan {
    // SAFETY: Native shell supplies its live owner, serialized by the game mutex.
    let name = unsafe { &(*state).name };
    ByteSpan {
        data: name.as_ptr(),
        size: name.len(),
    }
}
/// # Safety
/// The owner is live and exclusively mutable; input bytes are readable for this call and may alias the current name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_set_name(state: *mut State, text: ByteSpan) {
    unsafe { set_town_name(state, text) };
}
/// `t->name = text`: set the town's custom name.
/// # Safety
/// The owner is live and exclusively mutable; input bytes are readable for this call and may alias the current name.
pub(crate) unsafe fn set_town_name(state: *mut State, text: ByteSpan) {
    // Copy before replacing: native assignment may pass a view of this same name.
    w::hit(w::TOWN_NAME_SET);
    let name = unsafe { bytes(text) }.to_vec();
    // SAFETY: No owner reference survives allocation or another callback.
    unsafe {
        (*state).name = name;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_names_new() -> *mut Names {
    Box::into_raw(Box::default())
}
/// # Safety
/// The pointer is a live scratch allocation returned by `names_new`, consumed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_names_destroy(names: *mut Names) {
    // SAFETY: Exactly one destruction of the native facade's owned scratch.
    unsafe {
        drop(Box::from_raw(names));
    }
}
/// # Safety
/// The scratch allocation is exclusively owned for this call and input bytes remain readable until copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_names_insert(names: *mut Names, text: ByteSpan) {
    // SAFETY: Native facade owns call-local scratch; input survives this copy.
    unsafe {
        (*names).insert(bytes(text).to_vec());
    }
}
/// # Safety
/// The scratch allocation is live and exclusively owned for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_names_clear(names: *mut Names) {
    // SAFETY: Native facade supplies its exclusively owned invocation scratch.
    unsafe {
        (*names).clear();
    }
}
/// Verifies this custom name is unique. Only custom names are checked
/// (`town_cmd.cpp::IsUniqueTownName`). Return true if the name is unique.
pub(super) unsafe fn is_unique_town_name(s: &NameServices, name: ByteSpan) -> bool {
    for town in super::iterate_refs(|from| observed!((s.next)(from))) {
        // SAFETY: Iteration supplies a live owner; borrow ends before next callback.
        let equal = unsafe { !(*town.state).name.is_empty() && (*town.state).name == bytes(name) };
        if equal {
            return false;
        }
    }
    true
}
/// `townname.cpp::GetTownName`: render the specified parameters and encoded parts.
unsafe fn get_town_name(
    s: &NameServices,
    frame: *mut NameFrame,
    params: &NameParams,
    seed: u32,
    other: bool,
    text: &mut Vec<u8>,
) -> ByteSpan {
    if params.grfid == 0 {
        return observed!((s.render)(frame, params, seed, other));
    }
    text.clear();
    unsafe {
        crate::newgrf_townname::generate(
            &*s.shared,
            &mut |part| text.extend_from_slice(part),
            params.grfid,
            params.kind,
            seed,
        );
    }
    ByteSpan {
        data: text.as_ptr(),
        size: text.len(),
    }
}

/// Verifies the town name is valid and unique (`townname.cpp::VerifyTownName`).
/// `seed` holds the random bits; `names`, when present, checks a generated name's
/// uniqueness against the set. Return true iff the name is valid and unique.
pub(super) unsafe fn verify_town_name(
    s: &NameServices,
    frame: *mut NameFrame,
    seed: u32,
    params: *const NameParams,
    names: *mut Names,
) -> bool {
    let mut candidate_text = Vec::new();
    let mut other_text = Vec::new();
    let candidate = unsafe { get_town_name(s, frame, &*params, seed, false, &mut candidate_text) };
    // Check size and width.
    if observed!((s.utf8_length)(candidate.data, candidate.size))
        >= MAX_LENGTH_TOWN_NAME_CHARS as usize
    {
        return false;
    }
    if !names.is_null() {
        // SAFETY: Invocation-local scratch. Like std::set::find, rejected names
        // allocate no additional buffer; an accepted name alone becomes owned.
        unsafe {
            if (*names).contains(bytes(candidate)) {
                return false;
            }
            (*names).insert(bytes(candidate).to_vec());
        }
        return true;
    }
    for town in super::iterate_refs(|from| observed!((s.next)(from))) {
        // We cannot just compare the numbers: several numbers may map to one name.
        // SAFETY: Short canonical name observation before resolver reentry.
        let custom = unsafe { town_name(town.state) };
        let other = if custom.size == 0 {
            let params = unsafe { crate::townname_type::from_town(&*town.state) };
            let seed = unsafe { (*town.state).simulation.townnameparts };
            unsafe { get_town_name(s, frame, &params, seed, true, &mut other_text) }
        } else {
            custom
        };
        // Native candidate and other-name rendering have distinct stack strings.
        if unsafe { bytes(candidate) == bytes(other) } {
            w::hit(w::TOWN_NAME_DUPLICATE);
            return false;
        }
    }
    true
}
/// Generates valid town name (`townname.cpp::GenerateTownName`).
/// If a name is generated, it is stored in `result`; `names`, when present, checks
/// its uniqueness against the set. Return true iff a name was generated.
pub(super) unsafe fn generate_town_name(
    s: &NameServices,
    frame: *mut NameFrame,
    random: *mut Randomizer,
    result: *mut u32,
    params: *const NameParams,
    names: *mut Names,
) -> bool {
    // Do not set the number of tries too low: once names run out, we try this
    // loop only once before stopping town generation. Do not set it too high
    // either: checking all the other towns can take considerable time (10000
    // is too much).
    for _ in 0..1000 {
        let seed = observed!((s.random_next)(random));
        // SAFETY: Call-local frames/scratch; successful seeds alone update output.
        if unsafe { verify_town_name(s, frame, seed, params, names) } {
            unsafe {
                result.write(seed);
            }
            return true;
        }
        w::hit(w::TOWN_NAME_RETRY);
    }
    w::hit(w::TOWN_NAME_EXHAUSTED);
    false
}

/// # Safety
/// Tables, rendering frame and parameters remain live. Optional scratch is exclusively owned; rendering frames keep candidate and other bytes distinct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_verify_name_abi(
    s: *const NameServices,
    frame: *mut NameFrame,
    seed: u32,
    params: *const NameParams,
    names: *mut Names,
) -> u8 {
    u8::from(unsafe { verify_town_name(&*s, frame, seed, params, names) })
}
/// # Safety
/// Tables, frames, Randomizer and output remain live and writable as applicable. Optional scratch is exclusive; rejected seeds leave output untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_generate_name_abi(
    s: *const NameServices,
    frame: *mut NameFrame,
    random: *mut Randomizer,
    result: *mut u32,
    params: *const NameParams,
    names: *mut Names,
) -> u8 {
    u8::from(unsafe { generate_town_name(&*s, frame, random, result, params, names) })
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownNameServices" ["probe"] => NameServices {
        next,
        render,
        shared,
        utf8_length,
        random_next,
    };
    "TownNameParams" ["probe"] => NameParams { grfid, kind => type };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_name_retains_all_bytes_and_self_assignment() {
        let mut owner = State::default();
        let input: Vec<u8> = (0..=255).collect();
        unsafe {
            set_town_name(
                &raw mut owner,
                ByteSpan {
                    data: input.as_ptr(),
                    size: input.len(),
                },
            );
            let view = town_name(&raw const owner);
            assert_eq!(bytes(view), input);
            set_town_name(&raw mut owner, view);
            assert_eq!(bytes(town_name(&raw const owner)), input);
            set_town_name(
                &raw mut owner,
                ByteSpan {
                    data: std::ptr::null(),
                    size: 0,
                },
            );
            assert_eq!(owner.name, [] as [u8; 0]);
        }
    }
}

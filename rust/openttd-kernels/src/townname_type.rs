/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `townname_type.h`: shared town-name generator parameters.
#![allow(unsafe_code, clippy::cast_possible_truncation)]
use crate::strings_type::{BUILTIN_TOWNNAME_GENERATOR_COUNT, SPECSTR_TOWNNAME_START};

#[repr(C)]
pub struct NameParams {
    pub grfid: u32,
    pub kind: u16,
}

/// `TownNameParams(uint8_t)`; list access holds the game-state mutex.
pub(crate) unsafe fn from_style(style: u8) -> NameParams {
    if u32::from(style) >= BUILTIN_TOWNNAME_GENERATOR_COUNT {
        let gen_ = u16::from(style) - BUILTIN_TOWNNAME_GENERATOR_COUNT as u16;
        NameParams {
            grfid: unsafe { crate::newgrf_townname::get_id(gen_) },
            kind: unsafe { crate::newgrf_townname::get_type(gen_) },
        }
    } else {
        NameParams {
            grfid: 0,
            kind: SPECSTR_TOWNNAME_START as u16 + u16::from(style),
        }
    }
}

/// `TownNameParams(const Town *)`, including the missing-GRF fallback.
pub(crate) unsafe fn from_town(town: &crate::town::State) -> NameParams {
    let simulation = &town.simulation;
    let mut params = NameParams {
        grfid: simulation.townnamegrfid,
        kind: simulation.townnametype,
    };
    if params.grfid != 0 && !unsafe { crate::newgrf_townname::exists(params.grfid) } {
        params.grfid = 0;
        params.kind = SPECSTR_TOWNNAME_START as u16;
    }
    params
}

/// # Safety
/// `out` is writable, `town` is live, and the caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_name_params_for_town(
    out: *mut NameParams,
    town: *const crate::town::State,
) {
    unsafe {
        out.write(from_town(&*town));
    }
}

/// # Safety
/// `out` is writable, and the native caller holds the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_name_params(out: *mut NameParams, style: u8) {
    unsafe {
        out.write(from_style(style));
    }
}

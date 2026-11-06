/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Sign command policy and direct canonical-record mutations. Native viewport
//! callbacks run after all record borrows end, under the game-state mutex.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
use crate::command_type::DoCommandFlag;
use crate::company_type::{CompanyID, Owner};
use crate::openttd::GameMode;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Cost, Services as Shared};
use crate::signs_type::{MAX_LENGTH_SIGN_NAME_CHARS, Sign, SignID, SignRecord};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::TILE_SIZE;

#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub costs: &'static CommandCostServices,
    /// Construct the actual native shell and bind its canonical record.
    pub create: extern "C" fn() -> *mut SignRecord,
    pub position: unsafe extern "C" fn(*mut Sign),
    pub list_changed: extern "C" fn(i32),
    /// Unmark, remove from the viewport index, delete and refresh the list.
    pub remove: unsafe extern "C" fn(*mut Sign),
    pub too_many_signs: u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSignServices" ["probe"] => Services { shared, costs, create, position, list_changed, remove, too_many_signs };
}

fn editor() -> bool {
    crate::session::mode() == GameMode::EDITOR
}
/// `CompanyCanEditSign`.
fn can_edit(s: &Services, si: *const SignRecord) -> bool {
    !(unsafe { (*si).owner } == Owner::DEITY
        && CompanyID((s.shared.current_company)()) != Owner::DEITY
        && !editor())
}
/// The owner a rename or move leaves: unchanged in the editor, else the actor.
fn new_owner(s: &Services, si: *const SignRecord) -> Owner {
    if editor() {
        unsafe { (*si).owner }
    } else {
        CompanyID((s.shared.current_company)())
    }
}
/// `out` starts as native `CommandCost()`; only errors are written.
fn place(s: &Services, out: Cost<'_>, flags: u16, tile: u32, text: ByteSpan) -> SignID {
    if !unsafe { crate::signs_type::can_allocate() } {
        out.error(s.too_many_signs);
        return SignID::INVALID;
    }
    if (s.shared.utf8_length)(text.data, text.size) >= MAX_LENGTH_SIGN_NAME_CHARS {
        out.error(INVALID_STRING_ID);
        return SignID::INVALID;
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        let owner = if editor() {
            Owner::DEITY
        } else {
            CompanyID((s.shared.current_company)())
        };
        let map = s.shared.map();
        let x = map.tile_x(tile).wrapping_mul(TILE_SIZE).cast_signed();
        let y = map.tile_y(tile).wrapping_mul(TILE_SIZE).cast_signed();
        let z = crate::movement::pixel_z(s.shared, x, y, false);
        let si = (s.create)();
        unsafe {
            crate::owned_bytes::openttd_rust_owned_bytes_assign(&raw mut (*si).name, text);
            (*si).owner = owner;
            (*si).x = x;
            (*si).y = y;
            (*si).z = z;
            (s.position)((*si).shell);
            (s.list_changed)(0);
            return (*si).index;
        }
    }
    SignID::INVALID
}
fn editable(s: &Services, out: Cost<'_>, id: SignID) -> *mut SignRecord {
    let si = unsafe { crate::signs_type::pool().record(id) };
    if si.is_null() || !can_edit(s, si) {
        out.error(INVALID_STRING_ID);
        return std::ptr::null_mut();
    }
    si
}
fn rename(s: &Services, out: Cost<'_>, flags: u16, id: SignID, text: ByteSpan) {
    let si = editable(s, out, id);
    if si.is_null() {
        return;
    }
    if text.size != 0 {
        if (s.shared.utf8_length)(text.data, text.size) >= MAX_LENGTH_SIGN_NAME_CHARS {
            return out.error(INVALID_STRING_ID);
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            let owner = new_owner(s, si);
            unsafe {
                crate::owned_bytes::openttd_rust_owned_bytes_assign(&raw mut (*si).name, text);
                (*si).owner = owner;
                (s.position)((*si).shell);
            }
            (s.list_changed)(1);
        }
    } else if flags & DoCommandFlag::EXECUTE != 0 {
        unsafe { (s.remove)((*si).shell) };
    }
}
fn relocate(s: &Services, out: Cost<'_>, flags: u16, id: SignID, tile: u32) {
    let si = editable(s, out, id);
    if !si.is_null() && flags & DoCommandFlag::EXECUTE != 0 {
        let map = s.shared.map();
        let x = map.tile_x(tile).wrapping_mul(TILE_SIZE).cast_signed();
        let y = map.tile_y(tile).wrapping_mul(TILE_SIZE).cast_signed();
        let z = crate::movement::pixel_z(s.shared, x, y, false);
        let owner = new_owner(s, si);
        unsafe {
            (*si).x = x;
            (*si).y = y;
            (*si).z = z;
            (*si).owner = owner;
            (s.position)((*si).shell);
        }
    }
}

/// # Safety
/// Services, native output and borrowed text remain live for the synchronous entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_sign_place(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    text: ByteSpan,
) -> SignID {
    place(s, Cost(out, s.costs), flags, tile, text)
}
/// # Safety
/// Services, native output and borrowed text remain live for the synchronous entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_sign_rename(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    id: SignID,
    text: ByteSpan,
) {
    rename(s, Cost(out, s.costs), flags, id, text);
}
/// # Safety
/// Services and native output remain live for the synchronous entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_sign_move(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    id: SignID,
    tile: u32,
) {
    relocate(s, Cost(out, s.costs), flags, id, tile);
}
/// # Safety
/// `si` is a live native sign.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_can_edit_sign_abi(
    s: &Services,
    si: *const SignRecord,
) -> u8 {
    u8::from(can_edit(s, si))
}

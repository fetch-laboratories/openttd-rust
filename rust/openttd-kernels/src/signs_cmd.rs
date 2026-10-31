/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Sign command policy from `signs_cmd.cpp` and `CompanyCanEditSign`. Records, names and
//! viewport presentation stay in the native pool; each leaf below is one whole native
//! operation. Native pointers are opaque, synchronous and serialized by the game-state
//! mutex; no pointer survives a leaf that deletes its sign.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
use crate::command_type::DoCommandFlag;
use crate::company_type::{CompanyID, Owner};
use crate::openttd::GameMode;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Cost, Services as Shared};
use crate::signs_type::{MAX_LENGTH_SIGN_NAME_CHARS, Sign, SignID};
use crate::strings_type::INVALID_STRING_ID;

#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub costs: &'static CommandCostServices,
    /// `Sign::GetIfValid`.
    pub sign: extern "C" fn(SignID) -> *mut Sign,
    pub owner: unsafe extern "C" fn(*const Sign) -> Owner,
    /// `Sign::CanAllocateItem`.
    pub can_allocate: extern "C" fn() -> bool,
    /// Construct at the tile's pixel position and height, present it and refresh the list.
    pub place: extern "C" fn(Owner, u32, ByteSpan) -> SignID,
    /// Assign the name and owner, then present the sign.
    pub rename: unsafe extern "C" fn(*mut Sign, Owner, ByteSpan),
    /// Assign the tile's pixel position, height and the owner, then present the sign.
    pub relocate: unsafe extern "C" fn(*mut Sign, Owner, u32),
    /// Unmark, remove from the viewport index, delete and refresh the list.
    pub remove: unsafe extern "C" fn(*mut Sign),
    pub too_many_signs: u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSignServices" ["probe"] => Services { shared, costs, sign, owner, can_allocate, place, rename, relocate, remove, too_many_signs };
}

fn editor() -> bool {
    crate::session::mode() == GameMode::EDITOR
}
/// `CompanyCanEditSign`.
fn can_edit(s: &Services, si: *const Sign) -> bool {
    !(unsafe { (s.owner)(si) } == Owner::DEITY
        && CompanyID((s.shared.current_company)()) != Owner::DEITY
        && !editor())
}
/// The owner a rename or move leaves: unchanged in the editor, else the actor.
fn new_owner(s: &Services, si: *const Sign) -> Owner {
    if editor() {
        unsafe { (s.owner)(si) }
    } else {
        CompanyID((s.shared.current_company)())
    }
}
/// `out` starts as native `CommandCost()`; only errors are written.
fn place(s: &Services, out: Cost<'_>, flags: u16, tile: u32, text: ByteSpan) -> SignID {
    if !(s.can_allocate)() {
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
        return (s.place)(owner, tile, text);
    }
    SignID::INVALID
}
fn editable(s: &Services, out: Cost<'_>, id: SignID) -> *mut Sign {
    let si = (s.sign)(id);
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
            unsafe { (s.rename)(si, new_owner(s, si), text) };
        }
    } else if flags & DoCommandFlag::EXECUTE != 0 {
        unsafe { (s.remove)(si) };
    }
}
fn relocate(s: &Services, out: Cost<'_>, flags: u16, id: SignID, tile: u32) {
    let si = editable(s, out, id);
    if !si.is_null() && flags & DoCommandFlag::EXECUTE != 0 {
        unsafe { (s.relocate)(si, new_owner(s, si), tile) };
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
pub unsafe extern "C" fn openttd_rust_company_can_edit_sign(s: &Services, si: *const Sign) -> bool {
    can_edit(s, si)
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Depot naming over the canonical Rust pool and name bytes. Native window
//! callbacks run after all record borrows end under the game-state mutex.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::command_type::DoCommandFlag;
use crate::depot_type::{Depot, DepotID, MAX_LENGTH_DEPOT_NAME_CHARS};
use crate::services::{
    ByteSpan, CommandCost, CommandCostServices, CommandServices, Cost, Services as Shared,
};
use crate::strings_type::INVALID_STRING_ID;

#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub command: &'static CommandServices,
    pub costs: &'static CommandCostServices,
    /// Refresh order, depot and depot-list windows after a rename.
    pub renamed: unsafe extern "C" fn(*mut Depot),
    pub name_unique: u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDepotServices" ["probe"] => Services { shared, command, costs, renamed, name_unique };
}

/// `IsUniqueDepotName`, in `Depot::Iterate` order.
fn unique(text: ByteSpan) -> bool {
    unsafe {
        let pool = crate::depot_type::pool();
        let mut index = pool.next_index(0, crate::depot_type::DepotID::END);
        while index < crate::depot_type::DepotID::END {
            let d = pool.record(DepotID(index as u16));
            if !(*d).name.is_empty() && (*d).name.as_slice() == text.bytes() {
                return false;
            }
            index = pool.next_index(index + 1, crate::depot_type::DepotID::END);
        }
    }
    true
}
fn rename(s: &Services, out: Cost<'_>, flags: u16, id: DepotID, text: ByteSpan) {
    let d = unsafe { crate::depot_type::pool().record(id) };
    if d.is_null() {
        return out.error(INVALID_STRING_ID);
    }
    out.tile_ownership(s.command, unsafe { (*d).xy.0 });
    if !out.ok() {
        return;
    }
    let reset = text.size == 0;
    if !reset {
        if (s.shared.utf8_length)(text.data, text.size) >= MAX_LENGTH_DEPOT_NAME_CHARS {
            return out.error(INVALID_STRING_ID);
        }
        if !unique(text) {
            return out.error(s.name_unique);
        }
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        unsafe { crate::owned_bytes::openttd_rust_owned_bytes_assign(&raw mut (*d).name, text) };
        if reset {
            unsafe { default_name(s.shared, id) };
        }
        unsafe { (s.renamed)((*d).shell) };
    }
    out.zero();
}

/// # Safety
/// Services, native output and borrowed text remain live for the synchronous entry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_depot_rename(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    id: DepotID,
    text: ByteSpan,
) {
    rename(s, Cost(out, s.costs), flags, id, text);
}

/// `MakeDefaultName<Depot>`: the sliding bitmap and wrap point follow town.h.
pub(crate) unsafe fn default_name(shared: &Shared, id: DepotID) {
    unsafe {
        let pool = crate::depot_type::pool();
        let record = pool.record(id);
        crate::movement::native_assert(
            shared,
            (*record).name.is_empty() || (*record).town_cn == u16::MAX,
        );
        let tile = (*record).xy.0;
        let town = crate::town::spatial::closest(shared, tile, u32::MAX)
            .cast_mut()
            .cast();
        (*record).town = town;
        let mut used = 0_u32;
        let mut next = 0_u32;
        let mut stop = 0_usize;
        let mut index = 0_usize;
        loop {
            let other = pool.record(DepotID(index as u16));
            if !other.is_null()
                && other != record
                && (*other).town == town
                && crate::map_func::World::new(shared).kind((*other).xy.0)
                    == crate::map_func::World::new(shared).kind(tile)
            {
                let number = u32::from((*other).town_cn).wrapping_sub(next);
                if number < 32 {
                    used |= 1 << number;
                    if number == 0 {
                        loop {
                            used >>= 1;
                            next += 1;
                            if used & 1 == 0 {
                                break;
                            }
                        }
                        stop = index;
                    }
                }
            }
            index += 1;
            if index == (*crate::depot_type::descriptor()).first_unused {
                index = 0;
            }
            if index == stop {
                break;
            }
        }
        (*record).town_cn = next as u16;
    }
}
/// Native default-name consumers share the canonical Rust implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_depot_default_name(shared: &Shared, id: DepotID) {
    unsafe { default_name(shared, id) };
}

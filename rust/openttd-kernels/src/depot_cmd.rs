/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Depot naming policy from `depot_cmd.cpp`. Records and names stay in the native pool;
//! string assignment, `MakeDefaultName` and window refresh are whole native operations.
//! Native pointers are opaque, synchronous and serialized by the game-state mutex.
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
    /// Canonical `MakeDefaultName<Depot>` leaf, shared by every depot type.
    pub rail: &'static crate::rail_cmd::RailCommandServices,
    /// `Depot::GetIfValid`.
    pub depot: extern "C" fn(DepotID) -> *mut Depot,
    /// `Depot::GetPoolSize`.
    pub pool_size: extern "C" fn() -> u32,
    pub xy: unsafe extern "C" fn(*const Depot) -> u32,
    pub name: unsafe extern "C" fn(*const Depot) -> ByteSpan,
    pub set_name: unsafe extern "C" fn(*mut Depot, ByteSpan),
    /// Refresh order, depot and depot-list windows after a rename.
    pub renamed: unsafe extern "C" fn(*mut Depot),
    pub name_unique: u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDepotServices" ["probe"] => Services { shared, command, costs, rail, depot, pool_size, xy, name, set_name, renamed, name_unique };
}

/// `IsUniqueDepotName`, in `Depot::Iterate` order.
fn unique(s: &Services, text: ByteSpan) -> bool {
    for index in 0..(s.pool_size)() {
        let d = (s.depot)(DepotID(index as u16));
        if !d.is_null() {
            let name = unsafe { (s.name)(d) };
            if name.size != 0 && name.same_bytes(text) {
                return false;
            }
        }
    }
    true
}
fn rename(s: &Services, out: Cost<'_>, flags: u16, id: DepotID, text: ByteSpan) {
    let d = (s.depot)(id);
    if d.is_null() {
        return out.error(INVALID_STRING_ID);
    }
    out.tile_ownership(s.command, unsafe { (s.xy)(d) });
    if !out.ok() {
        return;
    }
    let reset = text.size == 0;
    if !reset {
        if (s.shared.utf8_length)(text.data, text.size) >= MAX_LENGTH_DEPOT_NAME_CHARS {
            return out.error(INVALID_STRING_ID);
        }
        if !unique(s, text) {
            return out.error(s.name_unique);
        }
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        unsafe { (s.set_name)(d, text) };
        if reset {
            (s.rail.depot_name)(id.0);
        }
        unsafe { (s.renamed)(d) };
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

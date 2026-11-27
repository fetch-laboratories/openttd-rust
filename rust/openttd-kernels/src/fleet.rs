/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `group_cmd.cpp` and `autoreplace.cpp`: the group commands, the group statistics and the
//! engine renewal lists, with the Rust owners of the C++ shells `Group` (its fields, name and
//! children), `GroupStatistics` (its fields and engine counts) and `EngineRenew`.
//!
//! Lifetime contract: every group, statistics and renewal record a handle names is a live
//! allocation of the serialized game state, reached through the group table's pool slots or
//! a C++ shell that holds it, and every service table lives for the process. C++ aliases each
//! record's scalar prefix with references, so Rust reads and writes those fields in place, one
//! field at a time, and holds no reference into a record across a call. The name, children and
//! engine-count containers are reached only from Rust, each borrowed for one operation with no
//! call inside it. A handle stays valid until its record is deleted; no body uses one after
//! the deletion.

use crate::command_func::cmd_error;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, OpenTTDCompany, Owner};
use crate::debug::debug;
use crate::engine_type::EngineID;
use crate::gfx_type::Colours;
use crate::group::{GroupFlag, GroupFlags};
use crate::group_cmd::AlterGroupMode;
use crate::group_type::{GroupID, MAX_LENGTH_GROUP_NAME_CHARS};
use crate::livery::{Livery, LiveryFlag, LiveryFlags, LiveryScheme};
use crate::raw_field::{read_field, write_field};
use crate::services::{CommandCost, Cost};
use crate::sprites::PAL_NONE;
use crate::table::strings::STR_ERROR_GROUP_CAN_T_SET_PARENT_RECURSION;
use crate::vehicle::{VehicleRef, VehicleServices};
use crate::vehicle_base::is_ground_vehicle;
use crate::vehicle_func::{VEHICLE_PROFIT_MIN_AGE, is_company_buildable_vehicle_type};
use crate::vehicle_gui::get_window_class_for_vehicle_type;
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};
use crate::window_func::{
    close_window_by_id, invalidate_window_classes_data, invalidate_window_data, set_window_dirty,
};
use crate::window_type::{WindowClass, WindowNumber};
use std::collections::{BTreeMap, BTreeSet};
use std::ptr::NonNull;

pub(crate) mod pending;
#[cfg(test)]
mod tests;
pub(crate) mod transactions;

/// The scalar members of `Group` that the C++ shell aliases with references.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroupFields {
    /// Group Owner.
    pub owner: u8,
    /// Vehicle type of the group.
    pub vehicle_type: u8,
    /// Group flags.
    pub flags: GroupFlags,
    /// Custom colour scheme for vehicles in this group.
    pub livery: Livery,
    /// Parent group.
    pub parent: u16,
    /// Number of group within the company.
    pub number: u16,
}

/// The Rust owner of a `Group`: its scalar members, its name and its children.
#[repr(C)]
struct Group {
    fields: GroupFields,
    /// Group Name.
    name: Vec<u8>,
    /// The `FlatSet` of the group's child groups.
    children: BTreeSet<GroupID>,
}

impl Group {
    /// `g->children`' first member at or after `from`, as the `FlatSet` iterator reaches it.
    ///
    /// # Safety
    /// `p` is a live group allocation.
    #[allow(unsafe_code)]
    unsafe fn child_at_or_after(p: *const Self, from: u32) -> Option<GroupID> {
        let from = GroupID(u16::try_from(from).ok()?);
        // SAFETY: The caller's live allocation; the set is borrowed for this lookup only.
        unsafe { &(*p).children }.range(from..).next().copied()
    }
    /// `g->children.insert(id)`.
    ///
    /// # Safety
    /// `p` is a live group allocation.
    #[allow(unsafe_code)]
    unsafe fn insert_child(p: *mut Self, id: GroupID) {
        // SAFETY: The caller's live allocation; the set is borrowed for this insertion only.
        unsafe { &mut (*p).children }.insert(id);
    }
    /// `g->children.erase(id)`.
    ///
    /// # Safety
    /// `p` is a live group allocation.
    #[allow(unsafe_code)]
    unsafe fn erase_child(p: *mut Self, id: GroupID) {
        // SAFETY: The caller's live allocation; the set is borrowed for this removal only.
        unsafe { &mut (*p).children }.remove(&id);
    }
    /// `g->name = name`.
    ///
    /// # Safety
    /// `p` is a live group allocation.
    #[allow(unsafe_code)]
    unsafe fn set_name(p: *mut Self, name: &[u8]) {
        // SAFETY: The caller's live allocation; the name is borrowed for this assignment only.
        let current = unsafe { &mut (*p).name };
        current.clear();
        current.extend_from_slice(name);
    }
}

/// The scalar members of `GroupStatistics` that the C++ shell aliases with references.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StatsFields {
    /// Sum of profits for all vehicles.
    pub profit_last_year: i64,
    /// Sum of profits for vehicles considered for profit statistics.
    pub profit_last_year_min_age: i64,
    /// Number of vehicles.
    pub num_vehicle: u16,
    /// Number of vehicles considered for profit statistics.
    pub num_vehicle_min_age: u16,
    /// Are any autoreplace rules set?
    pub autoreplace_defined: bool,
    /// Have all autoreplacement finished?
    pub autoreplace_finished: bool,
}

/// The Rust owner of a `GroupStatistics`: its scalar members and its engine counts.
#[repr(C)]
struct Stats {
    fields: StatsFields,
    /// Caches the number of engines of each type the company owns.
    num_engines: BTreeMap<EngineID, u16>,
}

crate::abi::shared_records! {
    fn abi_record;
    /// The Rust owner of an `EngineRenew`; the company owns the list head independently.
    #[derive(Clone, Copy)]
    "FleetRenewFields" ["fleet"] => pub struct Renew {
        pub from: u16 => "EngineID",
        pub to: u16 => "EngineID",
        pub next: *mut () => "EngineRenew *",
        pub group_id: u16 => "GroupID",
        pub replace_when_old: bool,
    }
}

/// The C++ `Group` shell, opaque to Rust.
#[repr(C)]
pub struct OpenTTDGroup {
    _opaque: [u8; 0],
}

/// The C++ `EngineRenew` shell, opaque to Rust.
#[repr(C)]
pub struct OpenTTDEngineRenew {
    _opaque: [u8; 0],
}

/// `EngineRenewList`: the C++ `EngineRenew` shell at the head of a renewal list, or null.
pub(crate) type EngineRenewList = *mut OpenTTDEngineRenew;

/// One step of a group pool lookup: the C++ shell, its Rust owner, the owner of its
/// `statistics` member and its index; the shell and owner are null for no group.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroupRecord {
    pub shell: *mut OpenTTDGroup,
    pub group: *mut GroupFields,
    pub statistics: *mut StatsFields,
    pub index: u16,
}

/// `EngineRenew::Iterate(from)`'s next step: the shell and its index; the shell is null after
/// the last.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RenewRecord {
    pub shell: *mut OpenTTDEngineRenew,
    pub index: u16,
}

/// A live `Group *`, resolved once where the original calls `Group::Get`, `GetIfValid` or
/// `new Group`, or reaches it through `Group::Iterate`.
#[derive(Clone, Copy)]
pub(crate) struct GroupRef {
    shell: *mut OpenTTDGroup,
    group: NonNull<Group>,
    statistics: *mut StatsFields,
    index: GroupID,
}

/// The cursor `for (const GroupID &childgroup : g->children)` walks: each step looks up the
/// first child after the last one, so no borrow of the set lives across the loop body.
pub(crate) struct Children {
    group: *const Group,
    from: u32,
}

impl Iterator for Children {
    type Item = GroupID;
    #[allow(unsafe_code)]
    fn next(&mut self) -> Option<GroupID> {
        // SAFETY: The module's lifetime contract: the group outlives the walk.
        let child = unsafe { Group::child_at_or_after(self.group, self.from) }?;
        self.from = u32::from(child.0) + 1;
        Some(child)
    }
}

#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized accesses"
)]
impl GroupRef {
    /// The group a lookup returned, or `None` for a null record.
    fn of(record: GroupRecord) -> Option<Self> {
        Some(Self {
            shell: record.shell,
            group: NonNull::new(record.group.cast::<Group>())?,
            statistics: record.statistics,
            index: GroupID(record.index),
        })
    }
    fn fields(self) -> *mut GroupFields {
        self.group.as_ptr().cast()
    }
    /// `Group::index`.
    pub(crate) fn index(self) -> GroupID {
        self.index
    }
    /// `Group::owner`.
    pub(crate) fn owner(self) -> Owner {
        CompanyID(read_field!(self.fields(), owner))
    }
    /// Store `Group::owner`.
    pub(crate) fn set_owner(self, owner: Owner) {
        write_field!(self.fields(), owner, owner.0);
    }
    /// `Group::vehicle_type`.
    pub(crate) fn vehicle_type(self) -> VehicleType {
        VehicleType(read_field!(self.fields(), vehicle_type))
    }
    /// `Group::flags`.
    pub(crate) fn flags(self) -> GroupFlags {
        read_field!(self.fields(), flags)
    }
    /// Store `Group::flags`.
    pub(crate) fn set_flags(self, flags: GroupFlags) {
        write_field!(self.fields(), flags, flags);
    }
    /// `&Group::livery`.
    pub(crate) fn livery(self) -> *mut Livery {
        // SAFETY: A place of the live allocation; no reference is made.
        unsafe { &raw mut (*self.fields()).livery }
    }
    /// `Group::parent`.
    pub(crate) fn parent(self) -> GroupID {
        GroupID(read_field!(self.fields(), parent))
    }
    /// Store `Group::parent`.
    pub(crate) fn set_parent(self, parent: GroupID) {
        write_field!(self.fields(), parent, parent.0);
    }
    /// `Group::number`.
    pub(crate) fn number(self) -> u16 {
        read_field!(self.fields(), number)
    }
    /// Store `Group::number`.
    pub(crate) fn set_number(self, number: u16) {
        write_field!(self.fields(), number, number);
    }
    /// `Group::statistics`, of a group `Group::Iterate` reached.
    pub(crate) fn statistics(self) -> GroupStatisticsRef {
        GroupStatisticsRef::of(self.statistics.cast())
    }
    /// `g->children`, walked by cursor.
    pub(crate) fn children(self) -> Children {
        Children {
            group: self.group.as_ptr(),
            from: 0,
        }
    }
    /// `FlatSet<GroupID> children = g->children`.
    pub(crate) fn children_copy(self) -> Vec<GroupID> {
        // SAFETY: The live allocation; the set is borrowed for the copy only.
        unsafe { &(*self.group.as_ptr()).children }
            .iter()
            .copied()
            .collect()
    }
    /// `g->children.insert(id)`.
    pub(crate) fn insert_child(self, id: GroupID) {
        // SAFETY: The live allocation.
        unsafe { Group::insert_child(self.group.as_ptr(), id) };
    }
    /// `g->children.erase(id)`.
    pub(crate) fn erase_child(self, id: GroupID) {
        // SAFETY: The live allocation.
        unsafe { Group::erase_child(self.group.as_ptr(), id) };
    }
    /// `g->name.clear()`.
    pub(crate) fn clear_name(self) {
        // SAFETY: The live allocation; the name is borrowed for the clear only.
        unsafe { &mut (*self.group.as_ptr()).name }.clear();
    }
    /// `g->name = name`.
    pub(crate) fn set_name(self, name: &[u8]) {
        // SAFETY: The live allocation.
        unsafe { Group::set_name(self.group.as_ptr(), name) };
    }
}

/// A live `GroupStatistics &`: the Rust owner the C++ member holds.
#[derive(Clone, Copy)]
pub(crate) struct GroupStatisticsRef(NonNull<Stats>);

#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized accesses"
)]
impl GroupStatisticsRef {
    /// The statistics owner `p` a lookup or a `GroupStatistics` member gave, which every
    /// caller passes non-null.
    pub(crate) fn of(p: *mut ()) -> Self {
        // SAFETY: Each lookup returns a live statistics owner.
        Self(unsafe { NonNull::new_unchecked(p.cast()) })
    }
    fn fields(self) -> *mut StatsFields {
        self.0.as_ptr().cast()
    }
    /// `GroupStatistics::profit_last_year`.
    pub(crate) fn profit_last_year(self) -> i64 {
        read_field!(self.fields(), profit_last_year)
    }
    /// Store `GroupStatistics::profit_last_year`.
    pub(crate) fn set_profit_last_year(self, value: i64) {
        write_field!(self.fields(), profit_last_year, value);
    }
    /// `GroupStatistics::profit_last_year_min_age`.
    pub(crate) fn profit_last_year_min_age(self) -> i64 {
        read_field!(self.fields(), profit_last_year_min_age)
    }
    /// Store `GroupStatistics::profit_last_year_min_age`.
    pub(crate) fn set_profit_last_year_min_age(self, value: i64) {
        write_field!(self.fields(), profit_last_year_min_age, value);
    }
    /// `GroupStatistics::num_vehicle`.
    pub(crate) fn num_vehicle(self) -> u16 {
        read_field!(self.fields(), num_vehicle)
    }
    /// Store `GroupStatistics::num_vehicle`.
    pub(crate) fn set_num_vehicle(self, value: u16) {
        write_field!(self.fields(), num_vehicle, value);
    }
    /// `GroupStatistics::num_vehicle_min_age`.
    pub(crate) fn num_vehicle_min_age(self) -> u16 {
        read_field!(self.fields(), num_vehicle_min_age)
    }
    /// Store `GroupStatistics::num_vehicle_min_age`.
    pub(crate) fn set_num_vehicle_min_age(self, value: u16) {
        write_field!(self.fields(), num_vehicle_min_age, value);
    }
    /// `GroupStatistics::autoreplace_defined`.
    pub(crate) fn autoreplace_defined(self) -> bool {
        read_field!(self.fields(), autoreplace_defined)
    }
    /// Store `GroupStatistics::autoreplace_defined`.
    pub(crate) fn set_autoreplace_defined(self, value: bool) {
        write_field!(self.fields(), autoreplace_defined, value);
    }
    /// Store `GroupStatistics::autoreplace_finished`.
    pub(crate) fn set_autoreplace_finished(self, value: bool) {
        write_field!(self.fields(), autoreplace_finished, value);
    }
    /// `num_engines[engine] += delta`, which inserts a zero entry first and wraps at the
    /// `uint16_t` width.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`uint16_t += int` converts the sum to `uint16_t`, as the original's map entry"
    )]
    pub(crate) fn add_num_engines(self, engine: EngineID, delta: i32) {
        // SAFETY: The live owner; the map is borrowed for this update only.
        let count = unsafe { &mut (*self.0.as_ptr()).num_engines }
            .entry(engine)
            .or_default();
        *count = count.wrapping_add(delta as u16);
    }
    /// `GroupStatistics::Clear`: clear all caches.
    pub(crate) fn clear(self) {
        self.set_num_vehicle(0);
        self.set_profit_last_year(0);
        self.set_num_vehicle_min_age(0);
        self.set_profit_last_year_min_age(0);

        /* This is also called when NewGRF change. So the number of engines might have changed. Reset. */
        // SAFETY: The live owner; the map is borrowed for the clear only.
        unsafe { &mut (*self.0.as_ptr()).num_engines }.clear();
    }
    /// `GroupStatistics::ClearProfits`.
    pub(crate) fn clear_profits(self) {
        self.set_profit_last_year(0);

        self.set_num_vehicle_min_age(0);
        self.set_profit_last_year_min_age(0);
    }
    /// `GroupStatistics::ClearAutoreplace`.
    pub(crate) fn clear_autoreplace(self) {
        self.set_autoreplace_defined(false);
        self.set_autoreplace_finished(false);
    }
    /// `GroupStatistics::GetNumEngines`: get number of vehicles of a specific engine ID.
    pub(crate) fn get_num_engines(self, engine: EngineID) -> u16 {
        // SAFETY: The live owner; the map is borrowed for the lookup only.
        unsafe { &(*self.0.as_ptr()).num_engines }
            .get(&engine)
            .copied()
            .unwrap_or(0)
    }
}

/// A live `EngineRenew *`: the C++ shell and its Rust owner, resolved once through
/// `renew_state`.
#[derive(Clone, Copy)]
pub(crate) struct EngineRenewRef {
    shell: NonNull<OpenTTDEngineRenew>,
    state: *mut Renew,
}

#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized accesses"
)]
impl EngineRenewRef {
    /// The renewal `shell` names, or `None` for null.
    fn of(w: &GroupServices, shell: EngineRenewList) -> Option<Self> {
        let shell = NonNull::new(shell)?;
        Some(Self {
            shell,
            state: (w.renew_state)(shell.as_ptr()),
        })
    }
    /// `EngineRenew::from`.
    pub(crate) fn from(self) -> EngineID {
        EngineID(read_field!(self.state, from))
    }
    /// `EngineRenew::to`.
    pub(crate) fn to(self) -> EngineID {
        EngineID(read_field!(self.state, to))
    }
    /// Store `EngineRenew::to`.
    fn set_to(self, to: EngineID) {
        write_field!(self.state, to, to.0);
    }
    /// `EngineRenew::next`.
    pub(crate) fn next(self) -> EngineRenewList {
        read_field!(self.state, next).cast()
    }
    /// Store `EngineRenew::next`.
    fn set_next(self, next: EngineRenewList) {
        write_field!(self.state, next, next.cast());
    }
    /// `EngineRenew::group_id`.
    pub(crate) fn group_id(self) -> GroupID {
        GroupID(read_field!(self.state, group_id))
    }
    /// `EngineRenew::replace_when_old`: do replacement only when vehicle is old.
    pub(crate) fn replace_when_old(self) -> bool {
        read_field!(self.state, replace_when_old)
    }
    /// Store `EngineRenew::replace_when_old`.
    fn set_replace_when_old(self, value: bool) {
        write_field!(self.state, replace_when_old, value);
    }
}

/// The group table: the pool operations of `Group` and `EngineRenew`, `GroupStatistics::Get`,
/// the `Vehicle` members still stored in the C++ shell, and the tables the bodies reach.
#[repr(C)]
pub struct GroupServices {
    // Tables.
    /// The vehicle selection table, resolved on use: its initialiser reaches this table.
    pub(crate) selection: extern "C" fn() -> *const crate::vehicle_transactions::SelectionServices,
    /// The shared table.
    pub(crate) shared: *const crate::services::Services,
    /// The base vehicle table.
    pub(crate) vehicles: &'static VehicleServices,

    // `Group` pool.
    /// `Group::CanAllocateItem()`.
    pub(crate) group_can_allocate: extern "C" fn() -> bool,
    /// `delete g` of a group shell.
    pub(crate) group_delete: extern "C" fn(*mut OpenTTDGroup),
    /// `Group::Get(index)`, with its assertion.
    pub(crate) group_get: extern "C" fn(GroupID) -> GroupRecord,
    /// `Group::GetIfValid(index)`.
    pub(crate) group_get_if_valid: extern "C" fn(GroupID) -> GroupRecord,
    /// `new Group(owner, vt)`.
    pub(crate) group_new: extern "C" fn(Owner, VehicleType) -> GroupRecord,
    /// `Group::Iterate(from)`'s next group.
    pub(crate) next_group: extern "C" fn(u32) -> GroupRecord,

    // `GroupStatistics`.
    /// `GroupStatistics::Get(company, id_g, type)`, with its assertions and `NOT_REACHED`:
    /// the owner of the statistics the group or company shell holds.
    pub(crate) group_statistics_get:
        extern "C" fn(CompanyID, GroupID, VehicleType) -> *mut StatsFields,

    // `EngineRenew` pool.
    /// `EngineRenew::Iterate(from)`'s next renewal.
    pub(crate) next_renew: extern "C" fn(u32) -> RenewRecord,
    /// `EngineRenew::CanAllocateItem()`.
    pub(crate) renew_can_allocate: extern "C" fn() -> bool,
    /// `delete er` of a renewal shell.
    pub(crate) renew_delete: extern "C" fn(*mut OpenTTDEngineRenew),
    /// `new EngineRenew()`: the shell, whose owner the caller fills.
    pub(crate) renew_new: extern "C" fn() -> *mut OpenTTDEngineRenew,
    /// The Rust owner a renewal shell holds.
    pub(crate) renew_state: extern "C" fn(*mut OpenTTDEngineRenew) -> *mut Renew,

    // `Vehicle` members.
    /// `Vehicle::group_id`.
    pub(crate) group_id: extern "C" fn(VehicleRef) -> GroupID,
    /// `Vehicle::IsEngineCountable()`.
    pub(crate) is_engine_countable: extern "C" fn(VehicleRef) -> bool,
    /// `v->group_id = id`.
    pub(crate) set_group_id: extern "C" fn(VehicleRef, GroupID),
}

#[allow(
    unsafe_code,
    reason = "the table pointers name immutable process-lifetime tables"
)]
impl GroupServices {
    /// The shared table.
    pub(crate) fn shared(&self) -> &'static crate::services::Services {
        // SAFETY: The module's table contract.
        unsafe { &*self.shared }
    }
    /// The vehicle selection table.
    fn selection(&self) -> &'static crate::vehicle_transactions::SelectionServices {
        // SAFETY: The module's table contract.
        unsafe { &*(self.selection)() }
    }
    /// The station command table, which names the command and cost tables.
    pub(crate) fn station_commands(&self) -> &'static crate::station_cmd::CommandServicesTable {
        // SAFETY: The module's table contract.
        unsafe { &*(self.shared().station_commands)() }
    }
    /// The company table.
    fn company(&self) -> &'static crate::company::Services {
        // SAFETY: The module's table contract: the station command table names the command
        // table, whose getter returns the company table.
        unsafe { &*((*self.station_commands().command).company)() }
    }
    /// The company identity table: the company pool and `Company` members.
    pub(crate) fn identity(&self) -> &'static crate::company_identity::Services {
        self.company().identity
    }
    /// The orders table, for the shared order chains, `Vehicle::GetIfValid` and the backups.
    fn orders(&self) -> &'static crate::orders::Leaves {
        self.vehicles.orders()
    }
    /// The vehicle maintenance table, for `Vehicle::economy_age`.
    fn maintenance(&self) -> &'static crate::vehicle::maintenance::Services {
        crate::vehicle::maintenance::Services::of(self.vehicles)
    }
    /// The native `CommandCost` at `out`, written through the shared cost services.
    fn cost(&self, out: *mut CommandCost) -> Cost<'static> {
        // SAFETY: The module's table contract.
        Cost(out, unsafe { &*self.station_commands().cost })
    }
    /// `_current_company`.
    pub(crate) fn current_company(&self) -> CompanyID {
        CompanyID((self.shared().current_company)())
    }
    /// `Utf8StringLength(text)`.
    fn utf8_length(&self, text: &[u8]) -> usize {
        (self.shared().utf8_length)(text.as_ptr(), text.len())
    }
    /// `Engine::Get(engine)`.
    pub(crate) fn engine(&self, engine: EngineID) -> crate::vehicle::EngineRef {
        let capabilities = self.vehicles.newgrf().capabilities;
        crate::vehicle::EngineRef::of((capabilities.engine)(engine.0))
    }
}

/// The record of a live native vehicle shell, for callers that hold shells.
#[allow(unsafe_code)]
pub(crate) fn record(w: &GroupServices, v: *mut ()) -> VehicleRef {
    // SAFETY: Every caller passes a live shell of the serialized game state.
    unsafe { VehicleRef::from_shell(v.cast(), w.vehicles) }
}
/// `v->Next()` as a native shell, for callers that hold shells.
pub(crate) fn next_part(v: VehicleRef) -> *mut () {
    v.next()
        .map_or(std::ptr::null_mut(), |next| next.shell().cast())
}

/// `Group::Get(index)`.
pub(crate) fn group_get(w: &GroupServices, index: GroupID) -> GroupRef {
    let Some(g) = GroupRef::of((w.group_get)(index)) else {
        crate::services_assert::not_reached()
    };
    g
}
/// `Group::GetIfValid(index)`.
pub(crate) fn group_get_if_valid(w: &GroupServices, index: GroupID) -> Option<GroupRef> {
    GroupRef::of((w.group_get_if_valid)(index))
}
/// `Group::IsValidID(index)`.
pub(crate) fn group_is_valid_id(w: &GroupServices, index: GroupID) -> bool {
    group_get_if_valid(w, index).is_some()
}
/// `IsDefaultGroupID(index)`.
fn is_default_group_id(index: GroupID) -> bool {
    index == GroupID::DEFAULT_GROUP
}

/// The cursor `for (Group *g : Group::Iterate())` walks; each step reads the next group after
/// the loop body, as the pool iterator does.
pub(crate) struct Groups<'a> {
    w: &'a GroupServices,
    from: u32,
}
impl Iterator for Groups<'_> {
    type Item = GroupRef;
    fn next(&mut self) -> Option<GroupRef> {
        let g = GroupRef::of((self.w.next_group)(self.from))?;
        self.from = u32::from(g.index.0) + 1;
        Some(g)
    }
}
/// `Group::Iterate()`.
pub(crate) fn groups(w: &GroupServices) -> Groups<'_> {
    Groups { w, from: 0 }
}

/// `Vehicle::Iterate()`, walked over the shared table's `vehicle_next`; each step reads the
/// next vehicle after the loop body.
fn vehicles(w: &GroupServices) -> impl Iterator<Item = VehicleRef> + '_ {
    let shared = w.shared();
    let newgrf = w.vehicles.newgrf();
    let mut from = 0_u32;
    std::iter::from_fn(move || {
        let v = (shared.vehicle_next)(from)?;
        from = v.index(newgrf).0.wrapping_add(1);
        Some(v)
    })
}

/// `VehicleListIdentifier(VL_GROUP_LIST, vtype, company).ToWindowNumber()`'s identifier.
fn group_list(vtype: VehicleType, company: CompanyID) -> VehicleListIdentifier {
    VehicleListIdentifier {
        r#type: VehicleListType::VL_GROUP_LIST,
        vtype,
        company,
        index: 0,
    }
}

/// `&c->livery[LS_DEFAULT]`.
fn default_livery(w: &GroupServices, c: *mut OpenTTDCompany) -> *mut Livery {
    crate::company_identity::livery_at((w.identity().livery)(c), LiveryScheme::LS_DEFAULT)
}

/// `UpdateGroupChildren`: update children list for each group.
pub(crate) fn update_group_children(w: &GroupServices) {
    for g in groups(w) {
        if g.parent() == GroupID::INVALID {
            continue;
        }
        let pg = group_get_if_valid(w, g.parent());
        match pg {
            Some(pg) if pg.owner() == g.owner() && pg.vehicle_type() == g.vehicle_type() => {
                pg.insert_child(g.index());
            }
            _ => {
                /* Due to a bug, groups which should have been deleted could be left with an invalid parent.
                 * Keep the group but clear the invalid parent so that the game is recoverable. */
                debug!(
                    misc,
                    2,
                    "Group {} has invalid parent {}",
                    g.index().0,
                    g.parent().0
                );
                g.set_parent(GroupID::INVALID);
            }
        }
    }
}

/// `GroupStatistics::Get(company, id_g, type)`: returns the statistics for a specific group.
pub(crate) fn group_statistics_get(
    w: &GroupServices,
    company: CompanyID,
    id_g: GroupID,
    r#type: VehicleType,
) -> GroupStatisticsRef {
    GroupStatisticsRef::of((w.group_statistics_get)(company, id_g, r#type).cast())
}

/// `GroupStatistics::Get(const Vehicle *v)`: returns the statistic for the group of a vehicle.
pub(crate) fn group_statistics_get_vehicle(w: &GroupServices, v: VehicleRef) -> GroupStatisticsRef {
    group_statistics_get(w, v.owner(), v.group_id(w), v.vehicle_type())
}

/// `GroupStatistics::GetAllGroup`: returns the statistic for the `ALL_GROUP` of a vehicle type.
pub(crate) fn group_statistics_get_all_group(
    w: &GroupServices,
    v: VehicleRef,
) -> GroupStatisticsRef {
    group_statistics_get(w, v.owner(), GroupID::ALL_GROUP, v.vehicle_type())
}

/// `GroupStatistics::UpdateAfterLoad`: update all caches after loading a game, changing
/// `NewGRF`, etc.
pub(crate) fn update_after_load(w: &GroupServices) {
    let company = w.company();
    /* Set up the engine count for all companies */
    crate::company_identity::companies(company.identity, |c| {
        for r#type in VehicleType::VEH_BEGIN.0..VehicleType::VEH_COMPANY_END.0 {
            GroupStatisticsRef::of((company.group_all)(c, VehicleType(r#type))).clear();
            GroupStatisticsRef::of((company.group_default)(c, VehicleType(r#type))).clear();
        }
        true
    });

    /* Recalculate */
    for g in groups(w) {
        g.statistics().clear();
    }

    for v in vehicles(w) {
        if !v.is_engine_countable(w) {
            continue;
        }

        count_engine(w, v, 1);
        if v.is_primary_vehicle() {
            count_vehicle(w, v, 1);
        }
    }

    let identity = company.identity;
    crate::company_identity::companies(identity, |c| {
        update_autoreplace(w, (identity.index)(c));
        true
    });
}

/// `GroupStatistics::CountVehicle`: update `num_vehicle` when adding or removing a vehicle.
/// `delta` is +1 to add, -1 to remove.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`uint16_t += int` converts the sum to `uint16_t`, as the original's counts"
)]
pub(crate) fn count_vehicle(w: &GroupServices, v: VehicleRef, delta: i32) {
    crate::services_assert::assert_with(|| delta == 1 || delta == -1);

    let stats_all = group_statistics_get_all_group(w, v);
    let stats = group_statistics_get_vehicle(w, v);

    // `Money * int` and `Money += Money` saturate as `OverflowSafeInt64` does.
    let profit = v
        .get_display_profit_last_year(w.vehicles)
        .saturating_mul(i64::from(delta));
    stats_all.set_num_vehicle(stats_all.num_vehicle().wrapping_add(delta as u16));
    stats_all.set_profit_last_year(stats_all.profit_last_year().saturating_add(profit));
    stats.set_num_vehicle(stats.num_vehicle().wrapping_add(delta as u16));
    stats.set_profit_last_year(stats.profit_last_year().saturating_add(profit));

    if v.economy_age(w.maintenance()) > VEHICLE_PROFIT_MIN_AGE {
        stats_all
            .set_num_vehicle_min_age(stats_all.num_vehicle_min_age().wrapping_add(delta as u16));
        stats_all.set_profit_last_year_min_age(
            stats_all.profit_last_year_min_age().saturating_add(profit),
        );
        stats.set_num_vehicle_min_age(stats.num_vehicle_min_age().wrapping_add(delta as u16));
        stats.set_profit_last_year_min_age(stats.profit_last_year_min_age().saturating_add(profit));
    }
}

/// `GroupStatistics::CountEngine`: update `num_engines` when adding/removing an engine.
/// `delta` is +1 to add, -1 to remove.
pub(crate) fn count_engine(w: &GroupServices, v: VehicleRef, delta: i32) {
    crate::services_assert::assert_with(|| delta == 1 || delta == -1);
    group_statistics_get_all_group(w, v).add_num_engines(v.engine_type(), delta);
    group_statistics_get_vehicle(w, v).add_num_engines(v.engine_type(), delta);
}

/// `GroupStatistics::AddProfitLastYear`: add a vehicle's last year profit to the profit sum of
/// its group.
pub(crate) fn add_profit_last_year(w: &GroupServices, v: VehicleRef) {
    let stats_all = group_statistics_get_all_group(w, v);
    let stats = group_statistics_get_vehicle(w, v);

    let profit = v.get_display_profit_last_year(w.vehicles);
    stats_all.set_profit_last_year(stats_all.profit_last_year().saturating_add(profit));
    stats.set_profit_last_year(stats.profit_last_year().saturating_add(profit));
}

/// `GroupStatistics::VehicleReachedMinAge`: add a vehicle to the profit sum of its group.
pub(crate) fn vehicle_reached_min_age(w: &GroupServices, v: VehicleRef) {
    let stats_all = group_statistics_get_all_group(w, v);
    let stats = group_statistics_get_vehicle(w, v);

    let profit = v.get_display_profit_last_year(w.vehicles);
    stats_all.set_num_vehicle_min_age(stats_all.num_vehicle_min_age().wrapping_add(1));
    stats_all
        .set_profit_last_year_min_age(stats_all.profit_last_year_min_age().saturating_add(profit));
    stats.set_num_vehicle_min_age(stats.num_vehicle_min_age().wrapping_add(1));
    stats.set_profit_last_year_min_age(stats.profit_last_year_min_age().saturating_add(profit));
}

/// `GroupStatistics::UpdateProfits`: recompute the profits for all groups.
pub(crate) fn update_profits(w: &GroupServices) {
    let company = w.company();
    /* Set up the engine count for all companies */
    crate::company_identity::companies(company.identity, |c| {
        for r#type in VehicleType::VEH_BEGIN.0..VehicleType::VEH_COMPANY_END.0 {
            GroupStatisticsRef::of((company.group_all)(c, VehicleType(r#type))).clear_profits();
            GroupStatisticsRef::of((company.group_default)(c, VehicleType(r#type))).clear_profits();
        }
        true
    });

    /* Recalculate */
    for g in groups(w) {
        g.statistics().clear_profits();
    }

    let maintenance = w.maintenance();
    for v in vehicles(w) {
        if v.is_primary_vehicle() {
            add_profit_last_year(w, v);
            if v.economy_age(maintenance) > VEHICLE_PROFIT_MIN_AGE {
                vehicle_reached_min_age(w, v);
            }
        }
    }
}

/// `GroupStatistics::UpdateAutoreplace`: update `autoreplace_defined` and
/// `autoreplace_finished` of all statistics of a company.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
pub(crate) fn update_autoreplace(w: &GroupServices, company: CompanyID) {
    let tables = w.company();
    /* Set up the engine count for all companies */
    let c = (tables.identity.company)(company);
    for r#type in VehicleType::VEH_BEGIN.0..VehicleType::VEH_COMPANY_END.0 {
        GroupStatisticsRef::of((tables.group_all)(c, VehicleType(r#type))).clear_autoreplace();
        GroupStatisticsRef::of((tables.group_default)(c, VehicleType(r#type))).clear_autoreplace();
    }

    /* Recalculate */
    for g in groups(w) {
        if g.owner() != company {
            continue;
        }
        g.statistics().clear_autoreplace();
    }

    // SAFETY: The company's live renewal list slot; one pointer-sized load.
    let mut erl = unsafe { (tables.identity.renewal_head)(c).read() };
    while let Some(er) = EngineRenewRef::of(w, erl) {
        let (from, group_id) = (er.from(), er.group_id());
        let e = w.engine(from);
        let stats = group_statistics_get(w, company, group_id, e.kind());
        if !stats.autoreplace_defined() {
            stats.set_autoreplace_defined(true);
            stats.set_autoreplace_finished(true);
        }
        if get_group_num_engines(w, company, group_id, from) > 0 {
            stats.set_autoreplace_finished(false);
        }
        erl = er.next();
    }
}

/// `UpdateNumEngineGroup`: update the num engines of a group ID. Decrease the old one and
/// increase the new one.
/// @note called in `SetTrainGroupID` and `UpdateTrainGroupID`
fn update_num_engine_group(w: &GroupServices, v: VehicleRef, old_g: GroupID, new_g: GroupID) {
    if old_g != new_g {
        /* Decrease the num engines in the old group */
        group_statistics_get(w, v.owner(), old_g, v.vehicle_type())
            .add_num_engines(v.engine_type(), -1);

        /* Increase the num engines in the new group */
        group_statistics_get(w, v.owner(), new_g, v.vehicle_type())
            .add_num_engines(v.engine_type(), 1);
    }
}

/// `GetParentLivery`: the livery a group inherits, its parent's or its company's default.
pub(crate) fn get_parent_livery(w: &GroupServices, g: GroupRef) -> *mut Livery {
    if g.parent() == GroupID::INVALID {
        let c = (w.identity().company)(g.owner());
        return default_livery(w, c);
    }

    let pg = group_get(w, g.parent());
    pg.livery()
}

/// `PropagateChildLivery`: propagate a livery change to a group's children, and optionally
/// update cached vehicle colourmaps. `reset_cache` resets the colourmap of vehicles in this
/// group.
#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized livery accesses"
)]
fn propagate_child_livery(w: &GroupServices, g: GroupRef, reset_cache: bool) {
    if reset_cache {
        let newgrf = w.vehicles.newgrf();
        /* Company colour data is indirectly cached. */
        for v in vehicles(w) {
            if v.group_id(w) == g.index()
                && (!is_ground_vehicle(v.vehicle_type()) || v.is_front_engine())
            {
                let mut u = Some(v);
                while let Some(part) = u {
                    (w.vehicles.set_colourmap)(part, PAL_NONE);
                    // SAFETY: A live vehicle of the serialized game.
                    unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(newgrf, part) };
                    u = part.next();
                }
            }
        }
    }

    let livery = g.livery();
    for childgroup in g.children() {
        let cg = group_get(w, childgroup);
        let child = cg.livery();
        if !read_field!(child, in_use).test(LiveryFlag::Primary) {
            write_field!(child, colour1, read_field!(livery, colour1));
        }
        if !read_field!(child, in_use).test(LiveryFlag::Secondary) {
            write_field!(child, colour2, read_field!(livery, colour2));
        }
        propagate_child_livery(w, cg, reset_cache);
    }
}

/// `UpdateCompanyGroupLiveries`: update group liveries for a company. This is called when the
/// `LS_DEFAULT` scheme is changed, to update groups with colours set to default.
#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized livery accesses"
)]
pub(crate) fn update_company_group_liveries(w: &GroupServices, c: *mut OpenTTDCompany) {
    let index = (w.identity().index)(c);
    let default = default_livery(w, c);
    for g in groups(w) {
        if g.owner() == index && g.parent() == GroupID::INVALID {
            let livery = g.livery();
            if !read_field!(livery, in_use).test(LiveryFlag::Primary) {
                write_field!(livery, colour1, read_field!(default, colour1));
            }
            if !read_field!(livery, in_use).test(LiveryFlag::Secondary) {
                write_field!(livery, colour2, read_field!(default, colour2));
            }
            propagate_child_livery(w, g, false);
        }
    }
}

/// `CmdCreateGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdCreateGroup {
    /// Vehicle type.
    pub(crate) vt: VehicleType,
    /// Parent group ID.
    pub(crate) parent_group: GroupID,
}

/// `CmdCreateGroup`: create a new vehicle group. The cost of this operation or an error is
/// written to `cost`; returns the new group's ID, or `GroupID::INVALID`.
#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized livery accesses"
)]
pub(crate) fn cmd_create_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdCreateGroup,
) -> GroupID {
    let CmdCreateGroup { vt, parent_group } = args;
    if !is_company_buildable_vehicle_type(vt) {
        cmd_error(cost);
        return GroupID::INVALID;
    }

    if !(w.group_can_allocate)() {
        cmd_error(cost);
        return GroupID::INVALID;
    }

    let current_company = w.current_company();
    let pg = group_get_if_valid(w, parent_group);
    if let Some(pg) = pg {
        if pg.owner() != current_company {
            cmd_error(cost);
            return GroupID::INVALID;
        }
        if pg.vehicle_type() != vt {
            cmd_error(cost);
            return GroupID::INVALID;
        }
    }

    if flags.test(DoCommandFlag::Execute) {
        let Some(g) = GroupRef::of((w.group_new)(current_company, vt)) else {
            crate::services_assert::not_reached()
        };

        let tables = w.company();
        let c = (tables.identity.company)(g.owner());
        g.set_number((tables.use_free_group)(c));
        let livery = g.livery();
        if let Some(pg) = pg {
            g.set_parent(pg.index());
            let parent = pg.livery();
            write_field!(livery, colour1, read_field!(parent, colour1));
            write_field!(livery, colour2, read_field!(parent, colour2));
            g.set_flags(pg.flags());
            pg.insert_child(g.index());
        } else {
            let default = default_livery(w, c);
            write_field!(livery, colour1, read_field!(default, colour1));
            write_field!(livery, colour2, read_field!(default, colour2));
            if (tables.identity.renew_keep_length)(c) {
                let mut group_flags = g.flags();
                group_flags.set(GroupFlag::ReplaceWagonRemoval);
                g.set_flags(group_flags);
            }
        }

        invalidate_window_data(
            get_window_class_for_vehicle_type(vt),
            group_list(vt, current_company).to_window_number(),
            0,
            false,
        );
        invalidate_window_data(
            WindowClass::WC_COMPANY_COLOUR,
            WindowNumber::from(g.owner()),
            i32::from(g.vehicle_type().0),
            false,
        );

        return g.index();
    }

    GroupID::INVALID
}

/// `CmdDeleteGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdDeleteGroup {
    /// Index of group.
    pub(crate) group_id: GroupID,
}

/// `CmdDeleteGroup`: add all vehicles in the given group to the default group and then
/// deletes the group. The cost of this operation or an error is written to `cost`.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
pub(crate) fn cmd_delete_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdDeleteGroup,
) {
    let CmdDeleteGroup { group_id } = args;
    let current_company = w.current_company();
    let Some(g) = group_get_if_valid(w, group_id).filter(|g| g.owner() == current_company) else {
        return cmd_error(cost);
    };

    let shared = w.shared();
    /* Remove all vehicles from the group */
    crate::command::command_do(shared, flags, |cost, flags| {
        cmd_remove_all_vehicles_group(w, cost, flags, CmdRemoveAllVehiclesGroup { group_id });
    });

    /* Delete sub-groups, using a copy to avoid invalid iteration. */
    let children = g.children_copy();
    for childgroup in children {
        crate::command::command_do(shared, flags, |cost, flags| {
            cmd_delete_group(
                w,
                cost,
                flags,
                CmdDeleteGroup {
                    group_id: childgroup,
                },
            );
        });
    }

    if flags.test(DoCommandFlag::Execute) {
        /* Update backupped orders if needed */
        crate::orders::order_backup_clear_group(g.index(), w.orders());

        if g.owner() < CompanyID::MAX_COMPANIES {
            let tables = w.company();
            let c = (tables.identity.company)(g.owner());

            /* If we set an autoreplace for the group we delete, remove it. */
            let mut from = 0_u32;
            loop {
                let record = (w.next_renew)(from);
                let Some(er) = EngineRenewRef::of(w, record.shell) else {
                    break;
                };
                from = u32::from(record.index) + 1;
                if er.group_id() == g.index() {
                    // `RemoveEngineReplacementForCompany(c, ...)` is
                    // `RemoveEngineReplacement(&c->engine_renew_list, ...)`, whose result is
                    // discarded.
                    let erl = (tables.identity.renewal_head)(c);
                    // SAFETY: `with_cost` constructs and destroys the native result around the
                    // call.
                    unsafe {
                        crate::command::with_cost(&*w.station_commands().cost, |cost| {
                            remove_engine_replacement(w, cost, erl, er.from(), g.index(), flags);
                        });
                    }
                }
            }

            (tables.release_free_group)(c, g.number());
        }

        if g.parent() != GroupID::INVALID {
            let pg = group_get(w, g.parent());
            pg.erase_child(g.index());
        }

        let vt = g.vehicle_type();

        /* Delete the Replace Vehicle Windows */
        close_window_by_id(
            WindowClass::WC_REPLACE_VEHICLE,
            WindowNumber::from(g.vehicle_type()),
            true,
            0,
        );
        (w.group_delete)(g.shell);

        invalidate_window_data(
            get_window_class_for_vehicle_type(vt),
            group_list(vt, current_company).to_window_number(),
            0,
            false,
        );
        invalidate_window_data(
            WindowClass::WC_COMPANY_COLOUR,
            WindowNumber::from(current_company),
            i32::from(vt.0),
            false,
        );
    }
}

/// `CmdAlterGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdAlterGroup<'a> {
    /// Operation to perform.
    pub(crate) mode: AlterGroupMode,
    /// Group ID.
    pub(crate) group_id: GroupID,
    /// Parent group index.
    pub(crate) parent_id: GroupID,
    /// The new name or an empty string when resetting to the default.
    pub(crate) text: &'a [u8],
}

/// `CmdAlterGroup`: alter a group. The cost of this operation or an error is written to
/// `cost`.
#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized livery accesses"
)]
pub(crate) fn cmd_alter_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdAlterGroup<'_>,
) {
    let CmdAlterGroup {
        mode,
        group_id,
        parent_id,
        text,
    } = args;
    let current_company = w.current_company();
    let Some(g) = group_get_if_valid(w, group_id).filter(|g| g.owner() == current_company) else {
        return cmd_error(cost);
    };

    if mode == AlterGroupMode::Rename {
        /* Rename group */
        let reset = text.is_empty();

        if !reset && w.utf8_length(text) >= MAX_LENGTH_GROUP_NAME_CHARS {
            return cmd_error(cost);
        }

        if flags.test(DoCommandFlag::Execute) {
            /* Assign the new one */
            if reset {
                g.clear_name();
            } else {
                g.set_name(text);
            }
        }
    } else if mode == AlterGroupMode::SetParent {
        /* Do nothing if the parent group isn't actually changed. */
        if g.parent() == parent_id {
            return;
        }

        /* Set group parent */
        let pg = group_get_if_valid(w, parent_id);

        if let Some(pg) = pg {
            if pg.owner() != current_company {
                return cmd_error(cost);
            }
            if pg.vehicle_type() != g.vehicle_type() {
                return cmd_error(cost);
            }

            /* Ensure request parent isn't child of group.
             * This is the only place that infinite loops are prevented. */
            if group_is_in_group(w, pg.index(), g.index()) {
                return cost.error(STR_ERROR_GROUP_CAN_T_SET_PARENT_RECURSION.0);
            }
        }

        if flags.test(DoCommandFlag::Execute) {
            if g.parent() != GroupID::INVALID {
                group_get(w, g.parent()).erase_child(g.index());
            }
            g.set_parent(pg.map_or(GroupID::INVALID, GroupRef::index));
            if g.parent() != GroupID::INVALID {
                group_get(w, g.parent()).insert_child(g.index());
            }

            update_autoreplace(w, g.owner());

            let livery = g.livery();
            if !read_field!(livery, in_use).all(
                LiveryFlags::from_element(LiveryFlag::Primary)
                    | LiveryFlags::from_element(LiveryFlag::Secondary),
            ) {
                /* Update livery with new parent's colours if either colour is default. */
                let parent = get_parent_livery(w, g);
                if !read_field!(livery, in_use).test(LiveryFlag::Primary) {
                    write_field!(livery, colour1, read_field!(parent, colour1));
                }
                if !read_field!(livery, in_use).test(LiveryFlag::Secondary) {
                    write_field!(livery, colour2, read_field!(parent, colour2));
                }

                propagate_child_livery(w, g, true);
                (w.shared().screen_dirty)();
            }
        }
    } else {
        return cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        invalidate_window_data(
            WindowClass::WC_REPLACE_VEHICLE,
            WindowNumber::from(g.vehicle_type()),
            1,
            false,
        );
        invalidate_window_data(
            get_window_class_for_vehicle_type(g.vehicle_type()),
            group_list(g.vehicle_type(), current_company).to_window_number(),
            0,
            false,
        );
        invalidate_window_data(
            WindowClass::WC_COMPANY_COLOUR,
            WindowNumber::from(g.owner()),
            i32::from(g.vehicle_type().0),
            false,
        );
        invalidate_window_classes_data(WindowClass::WC_VEHICLE_VIEW, 0, false);
        invalidate_window_classes_data(WindowClass::WC_VEHICLE_DETAILS, 0, false);
    }
}

/// `AddVehicleToGroup`: do add a vehicle to a group.
#[allow(unsafe_code, reason = "the vehicle's live `NewGRF` cache")]
fn add_vehicle_to_group(w: &GroupServices, v: VehicleRef, new_g: GroupID) {
    count_vehicle(w, v, -1);

    match v.vehicle_type() {
        VehicleType::VEH_TRAIN => set_train_group_id(w, v, new_g),

        VehicleType::VEH_ROAD | VehicleType::VEH_SHIP | VehicleType::VEH_AIRCRAFT => {
            if v.is_engine_countable(w) {
                update_num_engine_group(w, v, v.group_id(w), new_g);
            }
            v.set_group_id(w, new_g);
            let newgrf = w.vehicles.newgrf();
            let mut u = Some(v);
            while let Some(part) = u {
                (w.vehicles.set_colourmap)(part, PAL_NONE);
                // SAFETY: A live vehicle of the serialized game.
                unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(newgrf, part) };
                (w.vehicles.base_viewport)(part);
                u = part.next();
            }
        }
        _ => crate::services_assert::not_reached(),
    }

    let index = v.index(w.vehicles.newgrf());
    invalidate_window_data(
        WindowClass::WC_VEHICLE_VIEW,
        WindowNumber::from(index),
        0,
        false,
    );
    invalidate_window_data(
        WindowClass::WC_VEHICLE_DETAILS,
        WindowNumber::from(index),
        0,
        false,
    );

    count_vehicle(w, v, 1);
}

/// `CmdAddVehicleGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdAddVehicleGroup {
    /// Index of group.
    pub(crate) group_id: GroupID,
    /// Vehicle to add to a group.
    pub(crate) veh_id: VehicleID,
    /// Add shared vehicles as well.
    pub(crate) add_shared: bool,
    /// The vehicle list to add when `veh_id` is invalid.
    pub(crate) vli: VehicleListIdentifier,
}

/// `CmdAddVehicleGroup`: add a vehicle to a group. The cost of this operation or an error is
/// written to `cost`; returns the group the vehicles were added to, or `GroupID::INVALID`.
#[allow(unsafe_code, reason = "the scheduler's live vehicle pool descriptor")]
pub(crate) fn cmd_add_vehicle_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdAddVehicleGroup,
) -> GroupID {
    let CmdAddVehicleGroup {
        group_id,
        veh_id,
        add_shared,
        vli,
    } = args;
    let mut new_g = group_id;
    if !group_is_valid_id(w, new_g) && !is_default_group_id(new_g) && new_g != GroupID::NEW_GROUP {
        cmd_error(cost);
        return GroupID::INVALID;
    }

    let orders = w.orders();
    let mut list: Vec<VehicleRef> = Vec::new();
    if veh_id == VehicleID::INVALID && vli.valid() {
        // `GenerateVehicleSortList(&list, vli)`, which empties the new list first.
        // SAFETY: The selection table's live records; the walk ends with the call.
        let generated =
            { crate::vehicle_transactions::generate_list(w.selection(), vli, |v| list.push(v)) };
        if !generated || list.is_empty() {
            cmd_error(cost);
            return GroupID::INVALID;
        }
    } else {
        let Some(v) = crate::orders::vehicle_get_if_valid(veh_id, orders) else {
            cmd_error(cost);
            return GroupID::INVALID;
        };
        list.push(v);
    }

    let current_company = w.current_company();
    let vtype = list[0].vehicle_type();
    for v in &list {
        if v.owner() != current_company || !v.is_primary_vehicle() {
            cmd_error(cost);
            return GroupID::INVALID;
        }
    }

    if group_is_valid_id(w, new_g) {
        let g = group_get(w, new_g);
        if g.owner() != current_company || g.vehicle_type() != vtype {
            cmd_error(cost);
            return GroupID::INVALID;
        }
    }

    if new_g == GroupID::NEW_GROUP {
        /* Create new group. */
        let new_group_id = cmd_create_group(
            w,
            cost,
            flags,
            CmdCreateGroup {
                vt: vtype,
                parent_group: GroupID::INVALID,
            },
        );
        if !cost.ok() {
            return new_group_id;
        }

        new_g = new_group_id;
    }

    if flags.test(DoCommandFlag::Execute) {
        let newgrf = w.vehicles.newgrf();
        let pool = orders.scheduler().pool();
        for vc in &list {
            /* VehicleList is const but we need to modify the vehicle. */
            // `Vehicle::Get` of a listed vehicle: its slot holds it, as the original's
            // dereference of the result requires.
            let Some(v) = crate::vehicle_base::get_asserted(&pool, vc.index(newgrf)) else {
                crate::services_assert::not_reached()
            };
            add_vehicle_to_group(w, v, new_g);

            if add_shared {
                /* Add vehicles in the shared order list as well. */
                let mut v2 = crate::orders::first_shared(v, orders);
                while let Some(u) = v2 {
                    if u.group_id(w) != new_g {
                        add_vehicle_to_group(w, u, new_g);
                    }
                    v2 = crate::orders::next_shared(u, orders);
                }
            }

            set_window_dirty(WindowClass::WC_VEHICLE_DEPOT, WindowNumber::from(v.tile()));
        }

        update_autoreplace(w, current_company);

        /* Update the Replace Vehicle Windows */
        set_window_dirty(WindowClass::WC_REPLACE_VEHICLE, WindowNumber::from(vtype));
        invalidate_window_data(
            get_window_class_for_vehicle_type(vtype),
            group_list(vtype, current_company).to_window_number(),
            0,
            false,
        );
    }

    new_g
}

/// `CmdAddSharedVehicleGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdAddSharedVehicleGroup {
    /// Index of group.
    pub(crate) id_g: GroupID,
    /// Type of vehicles.
    pub(crate) r#type: VehicleType,
}

/// `CmdAddSharedVehicleGroup`: add all shared vehicles of all vehicles from a group. The cost
/// of this operation or an error is written to `cost`.
pub(crate) fn cmd_add_shared_vehicle_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdAddSharedVehicleGroup,
) {
    let CmdAddSharedVehicleGroup { id_g, r#type } = args;
    if !group_is_valid_id(w, id_g) || !is_company_buildable_vehicle_type(r#type) {
        return cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        let orders = w.orders();
        let newgrf = w.vehicles.newgrf();
        let shared = w.shared();
        /* Find the first front engine which belong to the group id_g
         * then add all shared vehicles of this front engine to the group id_g */
        for v in vehicles(w) {
            if v.vehicle_type() == r#type && v.is_primary_vehicle() {
                if v.group_id(w) != id_g {
                    continue;
                }

                /* For each shared vehicles add it to the group */
                let mut v2 = crate::orders::first_shared(v, orders);
                while let Some(u) = v2 {
                    if u.group_id(w) != id_g {
                        let veh_id = u.index(newgrf);
                        crate::command::command_do(shared, flags, |cost, flags| {
                            cmd_add_vehicle_group(
                                w,
                                cost,
                                flags,
                                CmdAddVehicleGroup {
                                    group_id: id_g,
                                    veh_id,
                                    add_shared: false,
                                    vli: VehicleListIdentifier::default(),
                                },
                            );
                        });
                    }
                    v2 = crate::orders::next_shared(u, orders);
                }
            }
        }

        invalidate_window_data(
            get_window_class_for_vehicle_type(r#type),
            group_list(r#type, w.current_company()).to_window_number(),
            0,
            false,
        );
    }
}

/// `CmdRemoveAllVehiclesGroup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdRemoveAllVehiclesGroup {
    /// Index of group.
    pub(crate) group_id: GroupID,
}

/// `CmdRemoveAllVehiclesGroup`: remove all vehicles from a group. The cost of this operation or
/// an error is written to `cost`.
pub(crate) fn cmd_remove_all_vehicles_group(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdRemoveAllVehiclesGroup,
) {
    let CmdRemoveAllVehiclesGroup { group_id } = args;
    let g = group_get_if_valid(w, group_id);

    let current_company = w.current_company();
    let Some(g) = g.filter(|g| g.owner() == current_company) else {
        return cmd_error(cost);
    };

    if flags.test(DoCommandFlag::Execute) {
        let newgrf = w.vehicles.newgrf();
        let shared = w.shared();
        /* Find each Vehicle that belongs to the group old_g and add it to the default group */
        for v in vehicles(w) {
            if v.is_primary_vehicle() {
                if v.group_id(w) != group_id {
                    continue;
                }

                /* Add The Vehicle to the default group */
                let veh_id = v.index(newgrf);
                crate::command::command_do(shared, flags, |cost, flags| {
                    cmd_add_vehicle_group(
                        w,
                        cost,
                        flags,
                        CmdAddVehicleGroup {
                            group_id: GroupID::DEFAULT_GROUP,
                            veh_id,
                            add_shared: false,
                            vli: VehicleListIdentifier::default(),
                        },
                    );
                });
            }
        }

        invalidate_window_data(
            get_window_class_for_vehicle_type(g.vehicle_type()),
            group_list(g.vehicle_type(), current_company).to_window_number(),
            0,
            false,
        );
    }
}

/// `CmdSetGroupLivery`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetGroupLivery {
    /// Group ID.
    pub(crate) group_id: GroupID,
    /// Set primary instead of secondary colour.
    pub(crate) primary: bool,
    /// Colour.
    pub(crate) colour: Colours,
}

/// `CmdSetGroupLivery`: set the livery for a vehicle group. The cost of this operation or an
/// error is written to `cost`.
#[allow(
    unsafe_code,
    reason = "the module's lifetime contract: field-sized livery accesses"
)]
pub(crate) fn cmd_set_group_livery(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetGroupLivery,
) {
    let CmdSetGroupLivery {
        group_id,
        primary,
        mut colour,
    } = args;
    let g = group_get_if_valid(w, group_id);

    let current_company = w.current_company();
    let Some(g) = g.filter(|g| g.owner() == current_company) else {
        return cmd_error(cost);
    };

    if colour >= Colours::COLOUR_END && colour != Colours::INVALID_COLOUR {
        return cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        let livery = g.livery();
        let mut in_use = read_field!(livery, in_use);
        if primary {
            in_use.set_to(LiveryFlag::Primary, colour != Colours::INVALID_COLOUR);
            write_field!(livery, in_use, in_use);
            if colour == Colours::INVALID_COLOUR {
                colour = read_field!(get_parent_livery(w, g), colour1);
            }
            write_field!(livery, colour1, colour);
        } else {
            in_use.set_to(LiveryFlag::Secondary, colour != Colours::INVALID_COLOUR);
            write_field!(livery, in_use, in_use);
            if colour == Colours::INVALID_COLOUR {
                colour = read_field!(get_parent_livery(w, g), colour2);
            }
            write_field!(livery, colour2, colour);
        }

        propagate_child_livery(w, g, true);
        (w.shared().screen_dirty)();
    }
}

/// `SetGroupFlag`: set group flag for a group and its sub-groups. `set` sets or clears the
/// flag.
fn set_group_flag(w: &GroupServices, g: GroupRef, flag: GroupFlag, set: bool, children: bool) {
    let mut flags = g.flags();
    if set {
        flags.set(flag);
    } else {
        flags.reset(flag);
    }
    g.set_flags(flags);

    if !children {
        return;
    }

    for childgroup in g.children() {
        set_group_flag(w, group_get(w, childgroup), flag, set, true);
    }
}

/// `CmdSetGroupFlag`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetGroupFlag {
    /// Index of group array.
    pub(crate) group_id: GroupID,
    /// Flag to set, by value not bit.
    pub(crate) flag: GroupFlag,
    /// Value to set the flag to.
    pub(crate) value: bool,
    /// To apply to sub-groups.
    pub(crate) recursive: bool,
}

/// `CmdSetGroupFlag`: (un)set group flag from a group. The cost of this operation or an error
/// is written to `cost`.
pub(crate) fn cmd_set_group_flag(
    w: &GroupServices,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetGroupFlag,
) {
    let CmdSetGroupFlag {
        group_id,
        flag,
        value,
        recursive,
    } = args;
    let current_company = w.current_company();
    let Some(g) = group_get_if_valid(w, group_id).filter(|g| g.owner() == current_company) else {
        return cmd_error(cost);
    };

    if flag != GroupFlag::ReplaceProtection && flag != GroupFlag::ReplaceWagonRemoval {
        return cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        set_group_flag(w, g, flag, value, recursive);

        set_window_dirty(
            get_window_class_for_vehicle_type(g.vehicle_type()),
            group_list(g.vehicle_type(), current_company).to_window_number(),
        );
        invalidate_window_data(
            WindowClass::WC_REPLACE_VEHICLE,
            WindowNumber::from(g.vehicle_type()),
            0,
            false,
        );
    }
}

/// `SetTrainGroupID`: affect the group ID of a train to `new_g`. `v` is the first vehicle of
/// the chain.
/// @note called in `CmdAddVehicleGroup` and `CmdMoveRailVehicle`
#[allow(unsafe_code, reason = "the vehicle's live `NewGRF` cache")]
pub(crate) fn set_train_group_id(w: &GroupServices, v: VehicleRef, new_g: GroupID) {
    if !group_is_valid_id(w, new_g) && !is_default_group_id(new_g) {
        return;
    }

    crate::services_assert::assert_with(|| v.is_front_engine() || is_default_group_id(new_g));

    let newgrf = w.vehicles.newgrf();
    let mut u = Some(v);
    while let Some(part) = u {
        if part.is_engine_countable(w) {
            update_num_engine_group(w, part, part.group_id(w), new_g);
        }

        part.set_group_id(w, new_g);
        (w.vehicles.set_colourmap)(part, PAL_NONE);
        // SAFETY: A live vehicle of the serialized game.
        unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(newgrf, part) };
        (w.vehicles.base_viewport)(part);
        u = part.next();
    }

    /* Update the Replace Vehicle Windows */
    update_autoreplace(w, v.owner());
    set_window_dirty(
        WindowClass::WC_REPLACE_VEHICLE,
        WindowNumber::from(VehicleType::VEH_TRAIN),
    );
}

/// `UpdateTrainGroupID`: recalculates the group ID of a train. Should be called each time a
/// vehicle is added to/removed from the chain. `v` is the first vehicle of the chain.
/// @note this needs to be called too for 'wagon chains' (in the depot, without an engine)
/// @note Called in `CmdBuildRailVehicle`, `CmdBuildRailWagon`, `CmdMoveRailVehicle`,
/// `CmdSellRailWagon`
#[allow(unsafe_code, reason = "the vehicle's live `NewGRF` cache")]
pub(crate) fn update_train_group_id(w: &GroupServices, v: VehicleRef) {
    crate::services_assert::assert_with(|| {
        v.is_front_engine() || crate::ground_vehicle::is_free_wagon(v.subtype())
    });

    let new_g = if v.is_front_engine() {
        v.group_id(w)
    } else {
        GroupID::DEFAULT_GROUP
    };
    let newgrf = w.vehicles.newgrf();
    let mut u = Some(v);
    while let Some(part) = u {
        if part.is_engine_countable(w) {
            update_num_engine_group(w, part, part.group_id(w), new_g);
        }

        part.set_group_id(w, new_g);
        (w.vehicles.set_colourmap)(part, PAL_NONE);
        // SAFETY: A live vehicle of the serialized game.
        unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(newgrf, part) };
        u = part.next();
    }

    /* Update the Replace Vehicle Windows */
    update_autoreplace(w, v.owner());
    set_window_dirty(
        WindowClass::WC_REPLACE_VEHICLE,
        WindowNumber::from(VehicleType::VEH_TRAIN),
    );
}

/// `GetGroupNumEngines`: get the number of engines with engine ID `id_e` in the group with
/// group ID `id_g` and its sub-groups; `company` is the company the group belongs to.
pub(crate) fn get_group_num_engines(
    w: &GroupServices,
    company: CompanyID,
    id_g: GroupID,
    id_e: EngineID,
) -> u32 {
    let mut count = 0_u32;

    if let Some(g) = group_get_if_valid(w, id_g) {
        for childgroup in g.children() {
            count = count.wrapping_add(get_group_num_engines(w, company, childgroup, id_e));
        }
    }

    count.wrapping_add(u32::from(
        group_statistics_get(w, company, id_g, w.engine(id_e).kind()).get_num_engines(id_e),
    ))
}

/// `GetGroupNumVehicle`: get the number of vehicles in the group with group ID `id_g` and its
/// sub-groups; `company` is the company the group belongs to and `type` its vehicle type.
pub(crate) fn get_group_num_vehicle(
    w: &GroupServices,
    company: CompanyID,
    id_g: GroupID,
    r#type: VehicleType,
) -> u32 {
    let mut count = 0_u32;

    if let Some(g) = group_get_if_valid(w, id_g) {
        for childgroup in g.children() {
            count = count.wrapping_add(get_group_num_vehicle(w, company, childgroup, r#type));
        }
    }

    count.wrapping_add(u32::from(
        group_statistics_get(w, company, id_g, r#type).num_vehicle(),
    ))
}

/// `GetGroupNumVehicleMinAge`: get the number of vehicles above profit minimum age in the
/// group with group ID `id_g` and its sub-groups.
pub(crate) fn get_group_num_vehicle_min_age(
    w: &GroupServices,
    company: CompanyID,
    id_g: GroupID,
    r#type: VehicleType,
) -> u32 {
    let mut count = 0_u32;

    if let Some(g) = group_get_if_valid(w, id_g) {
        for childgroup in g.children() {
            count = count.wrapping_add(get_group_num_vehicle_min_age(
                w, company, childgroup, r#type,
            ));
        }
    }

    count.wrapping_add(u32::from(
        group_statistics_get(w, company, id_g, r#type).num_vehicle_min_age(),
    ))
}

/// `GetGroupProfitLastYearMinAge`: get last year's profit of vehicles above minimum age for
/// the group with group ID `id_g` and its sub-groups.
pub(crate) fn get_group_profit_last_year_min_age(
    w: &GroupServices,
    company: CompanyID,
    id_g: GroupID,
    r#type: VehicleType,
) -> i64 {
    let mut sum = 0_i64;

    if let Some(g) = group_get_if_valid(w, id_g) {
        for childgroup in g.children() {
            sum = sum.saturating_add(get_group_profit_last_year_min_age(
                w, company, childgroup, r#type,
            ));
        }
    }

    sum.saturating_add(group_statistics_get(w, company, id_g, r#type).profit_last_year_min_age())
}

/// `RemoveAllGroupsForCompany`.
pub(crate) fn remove_all_groups_for_company(w: &GroupServices, company: CompanyID) {
    for g in groups(w) {
        if company == g.owner() {
            (w.group_delete)(g.shell);
        }
    }
}

/// `GroupIsInGroup`: test whether `search` is `group` or one of its descendants, walking from
/// `search` up its parents towards `group`.
pub(crate) fn group_is_in_group(w: &GroupServices, mut search: GroupID, group: GroupID) -> bool {
    if !group_is_valid_id(w, search) {
        return search == group;
    }

    loop {
        if search == group {
            return true;
        }
        search = group_get(w, search).parent();
        if search == GroupID::INVALID {
            break;
        }
    }

    false
}

/// `GetEngineReplacement`: retrieves the `EngineRenew` that specifies the replacement of the
/// given engine type from the given renewlist.
fn get_engine_replacement(
    w: &GroupServices,
    erl: EngineRenewList,
    engine: EngineID,
    group: GroupID,
) -> Option<EngineRenewRef> {
    let mut er = erl;

    while let Some(r) = EngineRenewRef::of(w, er) {
        if r.from() == engine && group_is_in_group(w, group, r.group_id()) {
            return Some(r);
        }
        er = r.next();
    }
    None
}

/// `RemoveAllEngineReplacement`: remove all engine replacement settings for the company.
/// `erl` is the renewlist for a given company, which is empty on return.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
pub(crate) fn remove_all_engine_replacement(w: &GroupServices, erl: *mut EngineRenewList) {
    // SAFETY: The caller's live list slot; one pointer-sized load.
    let mut er = unsafe { erl.read() };

    while let Some(r) = EngineRenewRef::of(w, er) {
        let next = r.next();
        (w.renew_delete)(r.shell.as_ptr());
        er = next;
    }
    // SAFETY: As above; one pointer-sized store.
    unsafe { erl.write(std::ptr::null_mut()) }; // Empty list
}

/// `EngineReplacement`: retrieve the engine replacement in a given renewlist for an original
/// engine type. `replace_when_old` is set to whether the replacement should be done when old.
/// Returns the engine type to replace with, or `EngineID::INVALID` if no replacement is in the
/// list.
pub(crate) fn engine_replacement(
    w: &GroupServices,
    erl: EngineRenewList,
    engine: EngineID,
    group: GroupID,
    replace_when_old: Option<&mut bool>,
) -> EngineID {
    let mut er = get_engine_replacement(w, erl, engine, group);
    if er.is_none()
        && (group == GroupID::DEFAULT_GROUP
            || (group_is_valid_id(w, group)
                && !group_get(w, group)
                    .flags()
                    .test(GroupFlag::ReplaceProtection)))
    {
        /* We didn't find anything useful in the vehicle's own group so we will try ALL_GROUP */
        er = get_engine_replacement(w, erl, engine, GroupID::ALL_GROUP);
    }
    if let Some(replace_when_old) = replace_when_old {
        *replace_when_old = match er {
            /* Not replacing */
            None => false,
            /* When replacing with same model, only ever do it when old */
            Some(er) if er.to() == engine => true,
            /* Use player setting */
            Some(er) => er.replace_when_old(),
        };
    }
    er.map_or(EngineID::INVALID, EngineRenewRef::to)
}

/// The rule `AddEngineReplacement` adds: the members of the `EngineRenew` it writes.
#[derive(Clone, Copy)]
pub(crate) struct EngineRenewRule {
    /// The original engine type.
    pub(crate) old_engine: EngineID,
    /// The replacement engine type.
    pub(crate) new_engine: EngineID,
    /// The group related to this replacement.
    pub(crate) group: GroupID,
    /// Replace when old or always?
    pub(crate) replace_when_old: bool,
}

/// `AddEngineReplacement`: add an engine replacement to the given renewlist. `flags` are the
/// calling command flags; an error is written to `cost`.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
pub(crate) fn add_engine_replacement(
    w: &GroupServices,
    cost: Cost<'_>,
    erl: *mut EngineRenewList,
    rule: EngineRenewRule,
    flags: DoCommandFlags,
) {
    let EngineRenewRule {
        old_engine,
        new_engine,
        group,
        replace_when_old,
    } = rule;
    /* Check if the old vehicle is already in the list */
    // SAFETY: The caller's live list slot; one pointer-sized load.
    if let Some(er) = get_engine_replacement(w, unsafe { erl.read() }, old_engine, group) {
        if flags.test(DoCommandFlag::Execute) {
            er.set_to(new_engine);
            er.set_replace_when_old(replace_when_old);
        }
        return;
    }

    if !(w.renew_can_allocate)() {
        return cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        /* Insert before the first element */
        let shell = (w.renew_new)();
        let Some(er) = EngineRenewRef::of(w, shell) else {
            crate::services_assert::not_reached()
        };
        write_field!(er.state, from, old_engine.0);
        write_field!(er.state, to, new_engine.0);
        // SAFETY: As above.
        write_field!(er.state, next, unsafe { erl.read() }.cast());
        write_field!(er.state, group_id, group.0);
        write_field!(er.state, replace_when_old, replace_when_old);
        // SAFETY: As above; one pointer-sized store.
        unsafe { erl.write(shell) };
    }
}

/// `RemoveEngineReplacement`: remove an engine replacement from a given renewlist. `engine` is
/// the original engine type and `group` the group related to this replacement; `flags` are the
/// calling command flags and an error is written to `cost`.
#[allow(unsafe_code, reason = "the company's live renewal list slot")]
pub(crate) fn remove_engine_replacement(
    w: &GroupServices,
    cost: Cost<'_>,
    erl: *mut EngineRenewList,
    engine: EngineID,
    group: GroupID,
    flags: DoCommandFlags,
) {
    // SAFETY: The caller's live list slot; one pointer-sized load.
    let mut er = unsafe { erl.read() };
    let mut prev: Option<EngineRenewRef> = None;

    while let Some(r) = EngineRenewRef::of(w, er) {
        if r.from() == engine && r.group_id() == group {
            if flags.test(DoCommandFlag::Execute) {
                if let Some(prev) = prev {
                    /* Cut this element out */
                    prev.set_next(r.next());
                } else {
                    // First element
                    /* The second becomes the new first element */
                    // SAFETY: As above; one pointer-sized store.
                    unsafe { erl.write(r.next()) };
                }
                (w.renew_delete)(r.shell.as_ptr());
            }
            return;
        }
        prev = Some(r);
        er = r.next();
    }

    cmd_error(cost);
}

// The C++ shells' owner lifecycles and member operations.

/// Create the Rust owner of a `Group`, with the members' initialisers; C++ starts the scalar
/// prefix's lifetime.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_group_create() -> *mut () {
    Box::into_raw(Box::new(Group {
        fields: GroupFields {
            owner: Owner::INVALID.0,
            vehicle_type: VehicleType::VEH_INVALID.0,
            flags: GroupFlags(0),
            livery: Livery::default(),
            parent: GroupID::INVALID.0,
            number: 0,
        },
        name: Vec::new(),
        children: BTreeSet::new(),
    }))
    .cast()
}
/// Release a `Group` owner after the shell's accesses end.
///
/// # Safety
/// `p` is a live result of `openttd_rust_fleet_group_create`, destroyed once.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_group_destroy(p: *mut ()) {
    // SAFETY: This function's contract.
    unsafe { drop(Box::from_raw(p.cast::<Group>())) }
}
/// Create the Rust owner of a `GroupStatistics`; C++ starts the scalar prefix's lifetime.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_stats_create() -> *mut () {
    Box::into_raw(Box::new(Stats {
        fields: StatsFields::default(),
        num_engines: BTreeMap::new(),
    }))
    .cast()
}
/// Release a `GroupStatistics` owner after the shell's accesses end.
///
/// # Safety
/// `p` is a live result of `openttd_rust_fleet_stats_create`, destroyed once.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_destroy(p: *mut ()) {
    // SAFETY: This function's contract.
    unsafe { drop(Box::from_raw(p.cast::<Stats>())) }
}
/// Create the Rust owner of an `EngineRenew`, with the members' initialisers; C++ starts the
/// scalar prefix's lifetime.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_renew_create() -> *mut () {
    Box::into_raw(Box::new(Renew {
        from: EngineID::INVALID.0,
        to: EngineID::INVALID.0,
        next: std::ptr::null_mut(),
        group_id: GroupID::INVALID.0,
        replace_when_old: false,
    }))
    .cast()
}
/// Release an `EngineRenew` owner after the shell's accesses end.
///
/// # Safety
/// `p` is a live result of `openttd_rust_fleet_renew_create`, destroyed once.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_renew_destroy(p: *mut ()) {
    // SAFETY: This function's contract.
    unsafe { drop(Box::from_raw(p.cast::<Renew>())) }
}
/// Create the Rust owner of a company's renewal list head, an empty `EngineRenewList`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_head_create() -> *mut () {
    Box::into_raw(Box::new(std::ptr::null_mut::<()>())).cast()
}
/// Release a renewal list head after the shell's accesses end.
///
/// # Safety
/// `p` is a live result of `openttd_rust_fleet_head_create`, destroyed once.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_head_destroy(p: *mut ()) {
    // SAFETY: This function's contract.
    unsafe { drop(Box::from_raw(p.cast::<EngineRenewList>())) }
}
/// `GroupStatistics`' copy assignment: the members and the engine counts, without sharing the
/// allocation.
///
/// # Safety
/// `to` and `from` are live statistics owners with no access in progress.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_stats_copy(to: *mut (), from: *const ()) {
    if std::ptr::eq(to, from.cast_mut()) {
        return;
    }
    // SAFETY: This function's contract: two distinct live owners, borrowed for the copy only.
    let (from, to) = unsafe { (&*from.cast::<Stats>(), &mut *to.cast::<Stats>()) };
    to.fields = from.fields;
    to.num_engines.clone_from(&from.num_engines);
}
/// The company's renewal list copy: the head pointer, a shallow copy as the original's.
///
/// # Safety
/// `to` and `from` are live renewal list heads.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_head_copy(to: *mut (), from: *const ()) {
    // SAFETY: This function's contract; one pointer-sized load and store.
    unsafe {
        to.cast::<EngineRenewList>()
            .write(from.cast::<EngineRenewList>().read());
    }
}
/// `Group::name`, copied into `out` for `length` bytes; the name's length. A null `out` asks
/// for the length only.
///
/// # Safety
/// `p` is a live group owner and a non-null `out` is writable for `length` bytes, at most the
/// name's length.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_name(
    p: *const (),
    out: *mut u8,
    length: usize,
) -> usize {
    // SAFETY: This function's contract; the name is borrowed for the copy only.
    let name = unsafe { &(*p.cast::<Group>()).name };
    if !out.is_null() {
        // SAFETY: This function's contract.
        unsafe { std::ptr::copy_nonoverlapping(name.as_ptr(), out, length) };
    }
    name.len()
}
/// `g->name = name` of `length` bytes.
///
/// # Safety
/// `p` is a live group owner and `bytes` is readable for `length` bytes.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_set_name(p: *mut (), bytes: *const u8, length: usize) {
    // SAFETY: This function's contract.
    unsafe { Group::set_name(p.cast(), std::slice::from_raw_parts(bytes, length)) };
}
/// The first member of `g->children` at or after `from`, or `UINT32_MAX` past the last.
///
/// # Safety
/// `p` is a live group owner.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_child(p: *const (), from: u32) -> u32 {
    // SAFETY: This function's contract.
    unsafe { Group::child_at_or_after(p.cast(), from) }.map_or(u32::MAX, |id| u32::from(id.0))
}
/// `g->children.insert(id)`, or `g->children.erase(id)` when `insert` is false.
///
/// # Safety
/// `p` is a live group owner.
#[allow(unsafe_code)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the facade passes `GroupID::base()`, a `uint16_t`"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_child_change(p: *mut (), id: u32, insert: bool) {
    let id = GroupID(id as u16);
    // SAFETY: This function's contract.
    unsafe {
        if insert {
            Group::insert_child(p.cast(), id);
        } else {
            Group::erase_child(p.cast(), id);
        }
    }
}
/// `GroupStatistics::GetNumEngines`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_engine_count(p: *const (), engine: u16) -> u16 {
    GroupStatisticsRef::of(p.cast_mut()).get_num_engines(EngineID(engine))
}
/// `GroupStatistics::Clear`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_stats_clear(p: *mut ()) {
    GroupStatisticsRef::of(p).clear();
}
/// `num_engines[engine] += delta`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_engine_change(p: *mut (), engine: u16, delta: i32) {
    GroupStatisticsRef::of(p).add_num_engines(EngineID(engine), delta);
}

// The bodies' C++ callers.

/// `UpdateGroupChildren`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_update_children(w: &'static GroupServices) {
    update_group_children(w);
}
/// `GroupStatistics::CountVehicle` of the vehicle shell `v`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_count_vehicle(
    w: &'static GroupServices,
    v: *mut (),
    delta: i32,
) {
    count_vehicle(w, record(w, v), delta);
}
/// `GroupStatistics::CountEngine` of the vehicle shell `v`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_count_engine(
    w: &'static GroupServices,
    v: *mut (),
    delta: i32,
) {
    count_engine(w, record(w, v), delta);
}
/// `GroupStatistics::UpdateAfterLoad`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_update_afterload(w: &'static GroupServices) {
    update_after_load(w);
}
/// `GroupStatistics::UpdateProfits`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_update_profits(w: &'static GroupServices) {
    update_profits(w);
}
/// `GroupStatistics::UpdateAutoreplace`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_update_autoreplace(w: &'static GroupServices, company: u8) {
    update_autoreplace(w, CompanyID(company));
}
/// `CmdCreateGroup`, writing its cost to `out`; the tuple's `GroupID`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_create_group(
    w: &'static GroupServices,
    flags: u16,
    vt: u8,
    parent_group: u16,
    out: *mut CommandCost,
) -> u16 {
    cmd_create_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdCreateGroup {
            vt: VehicleType(vt),
            parent_group: GroupID(parent_group),
        },
    )
    .0
}
/// `CmdDeleteGroup`, writing its cost to `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_delete_group(
    w: &'static GroupServices,
    flags: u16,
    group_id: u16,
    out: *mut CommandCost,
) {
    cmd_delete_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdDeleteGroup {
            group_id: GroupID(group_id),
        },
    );
}
/// `CmdAlterGroup`, writing its cost to `out`.
///
/// # Safety
/// `text` is readable for its size.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_alter_group(
    w: &'static GroupServices,
    flags: u16,
    mode: u8,
    group_id: u16,
    parent_id: u16,
    text: crate::services::ByteSpan,
    out: *mut CommandCost,
) {
    cmd_alter_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdAlterGroup {
            mode: AlterGroupMode(mode),
            group_id: GroupID(group_id),
            parent_id: GroupID(parent_id),
            // SAFETY: This function's contract; `std::string::data()` is never null.
            text: unsafe { std::slice::from_raw_parts(text.data, text.size) },
        },
    );
}
/// `CmdAddVehicleGroup`, writing its cost to `out`; the tuple's `GroupID`. The identifier
/// crosses as its four members.
#[allow(unsafe_code)]
#[allow(
    clippy::too_many_arguments,
    reason = "the command's positional export shim"
)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_add_vehicle_group(
    w: &'static GroupServices,
    flags: u16,
    group_id: u16,
    veh_id: u32,
    add_shared: bool,
    r#type: u8,
    vtype: u8,
    company: u8,
    index: u32,
    out: *mut CommandCost,
) -> u16 {
    cmd_add_vehicle_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdAddVehicleGroup {
            group_id: GroupID(group_id),
            veh_id: VehicleID(veh_id),
            add_shared,
            vli: VehicleListIdentifier {
                r#type: VehicleListType(r#type),
                vtype: VehicleType(vtype),
                company: CompanyID(company),
                index,
            },
        },
    )
    .0
}
/// `CmdAddSharedVehicleGroup`, writing its cost to `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_add_shared_group(
    w: &'static GroupServices,
    flags: u16,
    id_g: u16,
    r#type: u8,
    out: *mut CommandCost,
) {
    cmd_add_shared_vehicle_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdAddSharedVehicleGroup {
            id_g: GroupID(id_g),
            r#type: VehicleType(r#type),
        },
    );
}
/// `CmdRemoveAllVehiclesGroup`, writing its cost to `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_remove_vehicles_group(
    w: &'static GroupServices,
    flags: u16,
    group_id: u16,
    out: *mut CommandCost,
) {
    cmd_remove_all_vehicles_group(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdRemoveAllVehiclesGroup {
            group_id: GroupID(group_id),
        },
    );
}
/// `CmdSetGroupLivery`, writing its cost to `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_set_livery(
    w: &'static GroupServices,
    flags: u16,
    group_id: u16,
    primary: bool,
    colour: u8,
    out: *mut CommandCost,
) {
    cmd_set_group_livery(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdSetGroupLivery {
            group_id: GroupID(group_id),
            primary,
            colour: Colours(colour),
        },
    );
}
/// `CmdSetGroupFlag`, writing its cost to `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_flag_command(
    w: &'static GroupServices,
    flags: u16,
    group_id: u16,
    flag: u8,
    value: bool,
    recursive: bool,
    out: *mut CommandCost,
) {
    cmd_set_group_flag(
        w,
        w.cost(out),
        DoCommandFlags(flags),
        CmdSetGroupFlag {
            group_id: GroupID(group_id),
            flag: GroupFlag(flag),
            value,
            recursive,
        },
    );
}
/// `GetGroupNumEngines`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_sum_engines(
    w: &'static GroupServices,
    company: u8,
    id_g: u16,
    id_e: u16,
) -> u32 {
    get_group_num_engines(w, CompanyID(company), GroupID(id_g), EngineID(id_e))
}
/// `GetGroupNumVehicle`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_sum_vehicles(
    w: &'static GroupServices,
    company: u8,
    id_g: u16,
    r#type: u8,
) -> u32 {
    get_group_num_vehicle(w, CompanyID(company), GroupID(id_g), VehicleType(r#type))
}
/// `GetGroupNumVehicleMinAge`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_sum_min_age(
    w: &'static GroupServices,
    company: u8,
    id_g: u16,
    r#type: u8,
) -> u32 {
    get_group_num_vehicle_min_age(w, CompanyID(company), GroupID(id_g), VehicleType(r#type))
}
/// `GetGroupProfitLastYearMinAge`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_sum_profit(
    w: &'static GroupServices,
    company: u8,
    id_g: u16,
    r#type: u8,
) -> i64 {
    get_group_profit_last_year_min_age(w, CompanyID(company), GroupID(id_g), VehicleType(r#type))
}
/// `GroupIsInGroup`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_contains_abi(
    w: &'static GroupServices,
    search: u16,
    group: u16,
) -> u8 {
    u8::from(group_is_in_group(w, GroupID(search), GroupID(group)))
}
/// `EngineReplacement`; a null `replace_when_old` is not written.
///
/// # Safety
/// `erl` is a live renewal list and a non-null `replace_when_old` is writable.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fleet_replacement(
    w: &'static GroupServices,
    erl: EngineRenewList,
    engine: u16,
    group: u16,
    replace_when_old: *mut bool,
) -> u16 {
    engine_replacement(
        w,
        erl,
        EngineID(engine),
        GroupID(group),
        // SAFETY: This function's contract; the caller's local is borrowed for the call.
        unsafe { replace_when_old.as_mut() },
    )
    .0
}
/// `RemoveAllEngineReplacement`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fleet_remove_all_replacements(
    w: &'static GroupServices,
    erl: *mut EngineRenewList,
) {
    remove_all_engine_replacement(w, erl);
}

crate::abi::layouts! {
    fn abi_layout;
    "FleetGroupFields" ["fleet"] => GroupFields {
        owner,
        vehicle_type,
        flags,
        livery,
        parent,
        number,
    };
    "FleetStatisticsFields" ["fleet"] => StatsFields {
        profit_last_year,
        profit_last_year_min_age,
        num_vehicle,
        num_vehicle_min_age,
        autoreplace_defined,
        autoreplace_finished,
    };
    "OpenTTDFleetGroupRecord" ["fleet"] => GroupRecord {
        shell,
        group,
        statistics,
        index,
    };
    "OpenTTDFleetRenewRecord" ["fleet"] => RenewRecord {
        shell,
        index,
    };
    "OpenTTDFleetGroupServices" ["fleet"] => GroupServices {
        selection,
        shared,
        vehicles,
        group_can_allocate,
        group_delete,
        group_get,
        group_get_if_valid,
        group_new,
        next_group,
        group_statistics_get,
        next_renew,
        renew_can_allocate,
        renew_delete,
        renew_new,
        renew_state,
        group_id,
        is_engine_countable,
        set_group_id,
    };
}

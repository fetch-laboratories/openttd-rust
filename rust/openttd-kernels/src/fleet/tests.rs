/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The group commands' window calls and guards, the native fatal paths of
//! `AddVehicleToGroup` and `SetTrainGroupID`, and `VehicleListIdentifier`'s `To*ID`
//! accessors. Expectations are `group_cmd.cpp`'s and `vehiclelist.h`'s.
//!
//! The fixture is a group pool of the Rust `Group` and `GroupStatistics` owners behind the
//! group table's pool slots, a recording `CommandCost` and window table, and the company
//! and command tables the bodies reach through the shared table.

use super::*;
use crate::company::FinanceServices;
use crate::services::CommandCostStorage;
use crate::test_tables::{
    COMMANDS, COMPANY, COMPANY_IDENTITY, FINANCE, FLEET_GROUPS, ORDERS, SCHEDULER, SHARED,
    STATION_COMMANDS, VEHICLE,
};
use crate::window_func::testing::{Effect, take};
use std::cell::{Cell, RefCell};
use std::ptr;

/// One group of the fixture pool: its Rust owner, which also stands for its shell, and its
/// statistics' owner.
#[derive(Clone, Copy)]
struct Slot {
    group: *mut (),
    statistics: *mut (),
}

thread_local! {
    /// The group pool; `None` for a free slot.
    static POOL: RefCell<Vec<Option<Slot>>> = const { RefCell::new(Vec::new()) };
    /// The company's `group_all` and then `group_default` statistics, by vehicle type up to
    /// `VEH_EFFECT`.
    static COMPANY_STATS: RefCell<Vec<*mut ()>> = const { RefCell::new(Vec::new()) };
    /// The pool slot calls, in order.
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// `Group::CanAllocateItem()`.
    static CAN_ALLOCATE: Cell<bool> = const { Cell::new(true) };
    /// `_current_company`.
    static CURRENT: Cell<u8> = const { Cell::new(0) };
    /// The company's renewal list head.
    static RENEWALS: Cell<*mut ()> = const { Cell::new(ptr::null_mut()) };
    /// The company's liveries.
    static LIVERIES: RefCell<[Livery; 1]> = RefCell::new([Livery::default()]);
}

fn call(entry: impl Into<String>) {
    CALLS.with_borrow_mut(|c| c.push(entry.into()));
}
fn calls() -> Vec<String> {
    CALLS.with_borrow_mut(std::mem::take)
}

// The group table's slots.
fn record(index: u16) -> GroupRecord {
    let slot = POOL.with_borrow(|p| p.get(usize::from(index)).copied().flatten());
    slot.map_or(
        GroupRecord {
            shell: ptr::null_mut(),
            group: ptr::null_mut(),
            statistics: ptr::null_mut(),
            index: 0,
        },
        |s| GroupRecord {
            shell: s.group,
            group: s.group,
            statistics: s.statistics,
            index,
        },
    )
}
extern "C" fn group_get(index: GroupID) -> GroupRecord {
    call(format!("get {}", index.0));
    record(index.0)
}
extern "C" fn group_get_if_valid(index: GroupID) -> GroupRecord {
    call(format!("get_if_valid {}", index.0));
    record(index.0)
}
extern "C" fn next_group(from: u32) -> GroupRecord {
    let len = POOL.with_borrow(Vec::len);
    (from as usize..len)
        .map(|i| record(i as u16))
        .find(|r| !r.group.is_null())
        .unwrap_or_else(|| record(u16::MAX))
}
extern "C" fn group_can_allocate() -> bool {
    call("can_allocate");
    CAN_ALLOCATE.get()
}
extern "C" fn group_new(owner: Owner, vt: VehicleType) -> GroupRecord {
    call(format!("new {} {}", owner.0, vt.0));
    let index = add_group(owner, vt, GroupID::INVALID);
    record(index.0)
}
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
extern "C" fn group_delete(shell: *mut ()) {
    POOL.with_borrow_mut(|p| {
        let index = p
            .iter()
            .position(|s| s.is_some_and(|s| s.group == shell))
            .unwrap();
        call(format!("delete {index}"));
        let slot = p[index].take().unwrap();
        // SAFETY: The fixture's owners, out of the pool.
        unsafe {
            openttd_rust_fleet_group_destroy(slot.group);
            openttd_rust_fleet_stats_destroy(slot.statistics);
        }
    });
}
extern "C" fn group_statistics_get(_: CompanyID, id_g: GroupID, r#type: VehicleType) -> *mut () {
    if let Some(slot) = POOL.with_borrow(|p| p.get(usize::from(id_g.0)).copied().flatten()) {
        return slot.statistics;
    }
    let offset = if id_g == GroupID::ALL_GROUP { 0 } else { 5 };
    COMPANY_STATS.with_borrow(|s| s[offset + usize::from(r#type.0)])
}
extern "C" fn next_renew(_: u32) -> RenewRecord {
    RenewRecord {
        shell: ptr::null_mut(),
        index: 0,
    }
}

// The company tables.
extern "C" fn company_of(_: CompanyID) -> *mut OpenTTDCompany {
    ptr::NonNull::dangling().as_ptr()
}
extern "C" fn company_index(_: *mut OpenTTDCompany) -> CompanyID {
    CompanyID(CURRENT.get())
}
extern "C" fn company_livery(_: *mut OpenTTDCompany) -> *mut Livery {
    LIVERIES.with_borrow_mut(|l| l.as_mut_ptr())
}
extern "C" fn renewal_head(_: *const OpenTTDCompany) -> *mut EngineRenewList {
    RENEWALS.with(Cell::as_ptr)
}
extern "C" fn no_keep_length(_: *const OpenTTDCompany) -> bool {
    false
}
extern "C" fn no_company(_: u32) -> *mut OpenTTDCompany {
    ptr::null_mut()
}
extern "C" fn group_all(_: *mut OpenTTDCompany, r#type: VehicleType) -> *mut () {
    COMPANY_STATS.with_borrow(|s| s[usize::from(r#type.0)])
}
extern "C" fn group_default(_: *mut OpenTTDCompany, r#type: VehicleType) -> *mut () {
    COMPANY_STATS.with_borrow(|s| s[5 + usize::from(r#type.0)])
}
extern "C" fn use_free_group(_: *mut OpenTTDCompany) -> u16 {
    call("use free group");
    7
}
extern "C" fn release_free_group(_: *mut OpenTTDCompany, number: u16) {
    call(format!("release free group {number}"));
}
const IDENTITY: crate::company_identity::Services = crate::company_identity::Services {
    company: company_of,
    index: company_index,
    livery: company_livery,
    renewal_head,
    renew_keep_length: no_keep_length,
    next_company: no_company,
    ..COMPANY_IDENTITY
};
extern "C" fn company_table() -> *const crate::company::Services {
    const TABLE: crate::company::Services = crate::company::Services {
        identity: &IDENTITY,
        group_all,
        group_default,
        use_free_group,
        release_free_group,
        ..COMPANY
    };
    &TABLE
}

// The recording `CommandCost`: an error message, or none for success.
struct Recorded(Option<u32>);
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
fn recorded<'a>(p: *const CommandCost) -> &'a mut Recorded {
    // SAFETY: Each fixture cost points to a live `Recorded`.
    unsafe { &mut *p.cast_mut().cast::<Recorded>() }
}
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
unsafe extern "C" fn cost_construct(storage: *mut CommandCostStorage) -> *mut CommandCost {
    let p = storage.cast::<Recorded>();
    // SAFETY: The storage's aligned capacity holds a `Recorded`.
    unsafe { p.write(Recorded(None)) };
    p.cast()
}
extern "C" fn cost_destroy(_: *mut CommandCost) {}
extern "C" fn cost_zero(p: *mut CommandCost) {
    recorded(p).0 = None;
}
extern "C" fn cost_error(p: *mut CommandCost, message: u32) {
    recorded(p).0 = Some(message);
}
extern "C" fn cost_success(p: *const CommandCost) -> bool {
    recorded(p).0.is_none()
}
extern "C" fn cost_money(_: *const CommandCost) -> i64 {
    0
}
static COSTS: crate::services::CommandCostServices = crate::services::CommandCostServices {
    construct: cost_construct,
    destroy: cost_destroy,
    zero: cost_zero,
    error: cost_error,
    success: cost_success,
    money: cost_money,
    ..crate::test_tables::COMMAND_COSTS
};

// The shared and command tables.
extern "C" fn current_company() -> u8 {
    CURRENT.get()
}
extern "C" fn finances(_: u8) -> *mut crate::company::Finances {
    ptr::null_mut()
}
const FINANCES: FinanceServices = FinanceServices {
    current_company,
    company_finances: finances,
    ..FINANCE
};
extern "C" fn utf8_length(_: *const u8, size: usize) -> usize {
    size
}
extern "C" fn screen_dirty() {
    call("screen");
}
extern "C" fn no_vehicle(_: u32) -> Option<VehicleRef> {
    None
}
extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
    const COMMAND: crate::services::CommandServices = crate::services::CommandServices {
        shared: &SHARED_TABLE,
        finance: &FINANCES,
        company: company_table,
        cost: &COSTS,
        ..COMMANDS
    };
    const TABLE: crate::station_cmd::CommandServicesTable =
        crate::station_cmd::CommandServicesTable {
            cost: &COSTS,
            command: &COMMAND,
            ..STATION_COMMANDS
        };
    &TABLE
}
const SHARED_TABLE: crate::services::Services = crate::services::Services {
    current_company,
    utf8_length,
    screen_dirty,
    station_commands,
    vehicle_next: no_vehicle,
    ..SHARED
};

// The orders table, for `Vehicle::GetIfValid` over an empty vehicle pool and the backups.
extern "C" fn no_backup(_: *mut u32) -> *mut std::ffi::c_void {
    ptr::null_mut()
}
fn vehicles() -> &'static VehicleServices {
    thread_local! {
        static TABLE: &'static VehicleServices = {
            let pool: *const crate::pool::PoolDescriptor = Box::leak(Box::new(
                crate::pool::PoolDescriptor {
                    slots: crate::industrytype::NativeVec::new(),
                    used_bitmap: crate::industrytype::NativeVec::new(),
                    first_free: 0,
                    first_unused: 0,
                    items: 0,
                    checked: 0,
                    cleaning: false,
                    alloc_cache: ptr::null_mut(),
                },
            ));
            let orders: &'static crate::orders::Leaves = Box::leak(Box::new(crate::orders::Leaves {
                next_backup: no_backup,
                scheduler: Box::leak(Box::new(crate::vehicle::scheduler::SchedulerServices {
                    pool: Box::leak(Box::new(pool)),
                    ..SCHEDULER
                })),
                ..ORDERS
            }));
            ORDERS_TABLE.set(orders);
            Box::leak(Box::new(VehicleServices {
                newgrf,
                orders: orders_table,
                ..VEHICLE
            }))
        };
        static ORDERS_TABLE: Cell<*const crate::orders::Leaves> = const { Cell::new(ptr::null()) };
    }
    extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
        &crate::newgrf_vehicle::TEST_TABLE
    }
    extern "C" fn orders_table() -> *const crate::orders::Leaves {
        ORDERS_TABLE.get()
    }
    TABLE.with(|t| *t)
}

/// The group table over the fixture pool.
fn table() -> &'static GroupServices {
    Box::leak(Box::new(GroupServices {
        shared: &SHARED_TABLE,
        vehicles: vehicles(),
        group_can_allocate,
        group_delete,
        group_get,
        group_get_if_valid,
        group_new,
        next_group,
        group_statistics_get,
        next_renew,
        ..FLEET_GROUPS
    }))
}

/// Add a group of `owner` and `vt` under `parent` to the pool; its index.
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
fn add_group(owner: Owner, vt: VehicleType, parent: GroupID) -> GroupID {
    let group = openttd_rust_fleet_group_create();
    let fields = group.cast::<GroupFields>();
    // SAFETY: The new owner's scalar prefix.
    unsafe {
        (*fields).owner = owner.0;
        (*fields).vehicle_type = vt.0;
        (*fields).parent = parent.0;
    }
    let slot = Slot {
        group,
        statistics: openttd_rust_fleet_stats_create(),
    };
    let index = POOL.with_borrow_mut(|p| {
        p.push(Some(slot));
        GroupID((p.len() - 1) as u16)
    });
    if parent != GroupID::INVALID {
        let parent = POOL.with_borrow(|p| p[usize::from(parent.0)].unwrap());
        // SAFETY: The parent's live owner.
        unsafe { Group::insert_child(parent.group.cast(), index) };
    }
    index
}

/// The fixture world: a fresh pool with, for company 0, the road group 0 and its child 1;
/// the road group 2 of company 1; and the train group 3 of company 0.
struct World;
impl World {
    fn new() -> Self {
        POOL.with_borrow_mut(Vec::clear);
        COMPANY_STATS.set((0..10).map(|_| openttd_rust_fleet_stats_create()).collect());
        CAN_ALLOCATE.set(true);
        CURRENT.set(0);
        RENEWALS.set(ptr::null_mut());
        add_group(CompanyID(0), VehicleType::VEH_ROAD, GroupID::INVALID);
        add_group(CompanyID(0), VehicleType::VEH_ROAD, GroupID(0));
        add_group(CompanyID(1), VehicleType::VEH_ROAD, GroupID::INVALID);
        add_group(CompanyID(0), VehicleType::VEH_TRAIN, GroupID::INVALID);
        take();
        calls();
        World
    }
}
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
impl Drop for World {
    fn drop(&mut self) {
        for slot in POOL.with_borrow_mut(std::mem::take).into_iter().flatten() {
            // SAFETY: The fixture's owners, out of the pool.
            unsafe {
                openttd_rust_fleet_group_destroy(slot.group);
                openttd_rust_fleet_stats_destroy(slot.statistics);
            }
        }
        for stats in COMPANY_STATS.with_borrow_mut(std::mem::take) {
            // SAFETY: As above.
            unsafe { openttd_rust_fleet_stats_destroy(stats) };
        }
        take();
        calls();
    }
}

/// Every group's members, name and children, and every statistics record's members.
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
fn snapshot() -> Vec<String> {
    let mut out = Vec::new();
    for (i, slot) in POOL.with_borrow(Vec::clone).into_iter().enumerate() {
        let Some(slot) = slot else {
            out.push(format!("{i} free"));
            continue;
        };
        // SAFETY: The fixture's live owners, borrowed for the copy only.
        let (g, s) = unsafe {
            (
                &*slot.group.cast::<Group>(),
                &*slot.statistics.cast::<Stats>(),
            )
        };
        let f = g.fields;
        out.push(format!(
            "{i} {} {} {:?} {:?} {} {} {:?} {:?} {}",
            f.owner,
            f.vehicle_type,
            f.flags,
            f.livery,
            f.parent,
            f.number,
            g.name,
            g.children,
            s.fields.num_vehicle
        ));
    }
    out
}

/// Run `command` on a fresh recording cost; its error message, or `None` for success.
fn run(command: impl FnOnce(Cost<'_>)) -> Option<u32> {
    let mut cost = Recorded(None);
    command(Cost(ptr::from_mut(&mut cost).cast(), &COSTS));
    cost.0
}

fn execute() -> DoCommandFlags {
    DoCommandFlags::from_element(DoCommandFlag::Execute)
}

/// `VehicleListIdentifier(VL_GROUP_LIST, ROAD, 0).ToWindowNumber()`: `type << 23 | vtype
/// << 26` for company 0.
const ROAD_GROUP_LIST: WindowNumber = WindowNumber((4 << 23) | (1 << 26));

/// Evidence 1: `CmdCreateGroup`, `CmdAlterGroup` in both modes, `CmdSetGroupFlag` and
/// `CmdDeleteGroup` with `DoCommandFlag::Execute` make the original's window calls, each with
/// its class, number and data; without `Execute` each makes none.
#[test]
fn group_command_windows() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let w = table();
    let list = WindowClass::WC_ROADVEH_LIST;
    let road = WindowNumber(i32::from(VehicleType::VEH_ROAD.0));
    let alter = [
        Effect::InvalidateWindowData(WindowClass::WC_REPLACE_VEHICLE, road, 1, false),
        Effect::InvalidateWindowData(list, ROAD_GROUP_LIST, 0, false),
        Effect::InvalidateWindowData(WindowClass::WC_COMPANY_COLOUR, WindowNumber(0), 1, false),
        Effect::InvalidateWindowClassesData(WindowClass::WC_VEHICLE_VIEW, 0, false),
        Effect::InvalidateWindowClassesData(WindowClass::WC_VEHICLE_DETAILS, 0, false),
    ];
    for flags in [DoCommandFlags(0), execute()] {
        let executed = flags.test(DoCommandFlag::Execute);
        let expect = |effects: &[Effect]| {
            if executed {
                effects.to_vec()
            } else {
                Vec::new()
            }
        };
        let _world = World::new();

        let mut id = GroupID::INVALID;
        let create = CmdCreateGroup {
            vt: VehicleType::VEH_ROAD,
            parent_group: GroupID::INVALID,
        };
        assert_eq!(
            run(|cost| id = cmd_create_group(w, cost, flags, create)),
            None
        );
        assert_eq!(
            id,
            if executed {
                GroupID(4)
            } else {
                GroupID::INVALID
            }
        );
        assert_eq!(
            take(),
            expect(&[
                Effect::InvalidateWindowData(list, ROAD_GROUP_LIST, 0, false),
                Effect::InvalidateWindowData(
                    WindowClass::WC_COMPANY_COLOUR,
                    WindowNumber(0),
                    1,
                    false
                ),
            ])
        );

        let rename = CmdAlterGroup {
            mode: AlterGroupMode::Rename,
            group_id: GroupID(0),
            parent_id: GroupID::INVALID,
            text: b"Express",
        };
        assert_eq!(run(|cost| cmd_alter_group(w, cost, flags, rename)), None);
        assert_eq!(take(), expect(&alter));

        let set_parent = CmdAlterGroup {
            mode: AlterGroupMode::SetParent,
            group_id: GroupID(1),
            parent_id: GroupID::INVALID,
            text: b"",
        };
        assert_eq!(
            run(|cost| cmd_alter_group(w, cost, flags, set_parent)),
            None
        );
        assert_eq!(take(), expect(&alter));

        let flag = CmdSetGroupFlag {
            group_id: GroupID(0),
            flag: GroupFlag::ReplaceProtection,
            value: true,
            recursive: false,
        };
        assert_eq!(run(|cost| cmd_set_group_flag(w, cost, flags, flag)), None);
        assert_eq!(
            take(),
            expect(&[
                Effect::SetWindowDirty(list, ROAD_GROUP_LIST),
                Effect::InvalidateWindowData(WindowClass::WC_REPLACE_VEHICLE, road, 0, false),
            ])
        );

        let delete = CmdDeleteGroup {
            group_id: GroupID(0),
        };
        assert_eq!(run(|cost| cmd_delete_group(w, cost, flags, delete)), None);
        assert_eq!(
            take(),
            expect(&[
                // The nested `CmdRemoveAllVehiclesGroup`'s execution.
                Effect::InvalidateWindowData(list, ROAD_GROUP_LIST, 0, false),
                Effect::CloseWindowById(WindowClass::WC_REPLACE_VEHICLE, road, true, 0),
                Effect::InvalidateWindowData(list, ROAD_GROUP_LIST, 0, false),
                Effect::InvalidateWindowData(
                    WindowClass::WC_COMPANY_COLOUR,
                    WindowNumber(0),
                    1,
                    false
                ),
            ])
        );
        assert_eq!(snapshot()[0] == "0 free", executed);
    }
}

/// One guard row's command, run without `DoCommandFlag::Execute`.
#[derive(Clone, Copy)]
enum Command {
    Create(VehicleType, GroupID),
    Delete(GroupID),
    Alter(AlterGroupMode, GroupID, GroupID, &'static [u8]),
    Add(GroupID, VehicleID),
    Shared(GroupID, VehicleType),
    Remove(GroupID),
    Livery(GroupID, Colours),
    Flag(GroupID, GroupFlag),
}
fn dispatch(w: &GroupServices, cost: Cost<'_>, command: Command) {
    let none = DoCommandFlags(0);
    match command {
        Command::Create(vt, parent_group) => {
            cmd_create_group(w, cost, none, CmdCreateGroup { vt, parent_group });
        }
        Command::Delete(group_id) => cmd_delete_group(w, cost, none, CmdDeleteGroup { group_id }),
        Command::Alter(mode, group_id, parent_id, text) => cmd_alter_group(
            w,
            cost,
            none,
            CmdAlterGroup {
                mode,
                group_id,
                parent_id,
                text,
            },
        ),
        Command::Add(group_id, veh_id) => {
            cmd_add_vehicle_group(
                w,
                cost,
                none,
                CmdAddVehicleGroup {
                    group_id,
                    veh_id,
                    add_shared: false,
                    vli: VehicleListIdentifier::default(),
                },
            );
        }
        Command::Shared(id_g, r#type) => {
            cmd_add_shared_vehicle_group(w, cost, none, CmdAddSharedVehicleGroup { id_g, r#type });
        }
        Command::Remove(group_id) => {
            cmd_remove_all_vehicles_group(w, cost, none, CmdRemoveAllVehiclesGroup { group_id });
        }
        Command::Livery(group_id, colour) => cmd_set_group_livery(
            w,
            cost,
            none,
            CmdSetGroupLivery {
                group_id,
                primary: true,
                colour,
            },
        ),
        Command::Flag(group_id, flag) => cmd_set_group_flag(
            w,
            cost,
            none,
            CmdSetGroupFlag {
                group_id,
                flag,
                value: true,
                recursive: true,
            },
        ),
    }
}

/// Evidence 2: each guard of the eight commands gives its own result, read from the recorded
/// cost, and changes no group or statistics field; where two guards give the same result,
/// the slot calls show their order.
#[test]
#[allow(clippy::too_many_lines, reason = "one table row for each guard")]
fn group_command_guards() {
    const ERROR: Option<u32> = Some(crate::strings_type::INVALID_STRING_ID);
    const RECURSION: Option<u32> = Some(STR_ERROR_GROUP_CAN_T_SET_PARENT_RECURSION.0);
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let w = table();
    let invalid = GroupID(9);
    let none = GroupID::INVALID;
    let road = VehicleType::VEH_ROAD;
    let effect = VehicleType::VEH_EFFECT;
    let (rename, set_parent) = (AlterGroupMode::Rename, AlterGroupMode::SetParent);
    let name31: &'static [u8] = &[b'a'; 31];
    let name32: &'static [u8] = &[b'a'; 32];
    // Each row: its name, its command, whether the pool is full, the result and, where it
    // shows the guards' order, the slot calls.
    let rows: [(&str, Command, bool, Option<u32>, Option<&[&str]>); 33] = [
        // `CmdCreateGroup`: an unbuildable type makes no allocation check, and a full pool
        // no parent lookup.
        (
            "create unbuildable",
            Command::Create(effect, GroupID(0)),
            false,
            ERROR,
            Some(&[]),
        ),
        (
            "create full pool",
            Command::Create(road, GroupID(0)),
            true,
            ERROR,
            Some(&["can_allocate"]),
        ),
        (
            "create other company's parent",
            Command::Create(road, GroupID(2)),
            false,
            ERROR,
            Some(&["can_allocate", "get_if_valid 2"]),
        ),
        (
            "create other type's parent",
            Command::Create(road, GroupID(3)),
            false,
            ERROR,
            None,
        ),
        (
            "create",
            Command::Create(road, GroupID(0)),
            false,
            None,
            None,
        ),
        // `CmdDeleteGroup`.
        (
            "delete invalid",
            Command::Delete(invalid),
            false,
            ERROR,
            Some(&["get_if_valid 9"]),
        ),
        (
            "delete other company's",
            Command::Delete(GroupID(2)),
            false,
            ERROR,
            None,
        ),
        // `CmdAlterGroup`.
        (
            "alter invalid",
            Command::Alter(rename, invalid, none, b"x"),
            false,
            ERROR,
            None,
        ),
        (
            "alter other company's",
            Command::Alter(rename, GroupID(2), none, b"x"),
            false,
            ERROR,
            None,
        ),
        (
            "rename 32 characters",
            Command::Alter(rename, GroupID(0), none, name32),
            false,
            ERROR,
            None,
        ),
        (
            "rename 31 characters",
            Command::Alter(rename, GroupID(0), none, name31),
            false,
            None,
            None,
        ),
        (
            "rename empty",
            Command::Alter(rename, GroupID(0), none, b""),
            false,
            None,
            None,
        ),
        (
            "parent of other company",
            Command::Alter(set_parent, GroupID(0), GroupID(2), b""),
            false,
            ERROR,
            None,
        ),
        (
            "parent of other type",
            Command::Alter(set_parent, GroupID(0), GroupID(3), b""),
            false,
            ERROR,
            None,
        ),
        (
            "parent is the group",
            Command::Alter(set_parent, GroupID(0), GroupID(0), b""),
            false,
            RECURSION,
            None,
        ),
        (
            "parent is a descendant",
            Command::Alter(set_parent, GroupID(0), GroupID(1), b""),
            false,
            RECURSION,
            None,
        ),
        // The unchanged parent returns before the parent is looked up.
        (
            "parent unchanged",
            Command::Alter(set_parent, GroupID(1), GroupID(0), b""),
            false,
            None,
            Some(&["get_if_valid 1"]),
        ),
        (
            "mode past SetParent",
            Command::Alter(AlterGroupMode(2), GroupID(0), none, b""),
            false,
            ERROR,
            None,
        ),
        // `CmdAddVehicleGroup`.
        (
            "add to invalid group",
            Command::Add(invalid, VehicleID(0)),
            false,
            ERROR,
            None,
        ),
        (
            "add invalid vehicle",
            Command::Add(GroupID::DEFAULT_GROUP, VehicleID(0)),
            false,
            ERROR,
            Some(&["get_if_valid 65534"]),
        ),
        // `CmdAddSharedVehicleGroup`.
        (
            "shared invalid group",
            Command::Shared(invalid, road),
            false,
            ERROR,
            None,
        ),
        (
            "shared unbuildable",
            Command::Shared(GroupID(0), effect),
            false,
            ERROR,
            None,
        ),
        (
            "shared",
            Command::Shared(GroupID(0), road),
            false,
            None,
            None,
        ),
        // `CmdRemoveAllVehiclesGroup`.
        (
            "remove invalid",
            Command::Remove(invalid),
            false,
            ERROR,
            None,
        ),
        (
            "remove other company's",
            Command::Remove(GroupID(2)),
            false,
            ERROR,
            None,
        ),
        // `CmdSetGroupLivery`: the group is tested before the colour.
        (
            "livery invalid",
            Command::Livery(invalid, Colours::COLOUR_END),
            false,
            ERROR,
            Some(&["get_if_valid 9"]),
        ),
        (
            "livery other company's",
            Command::Livery(GroupID(2), Colours(0)),
            false,
            ERROR,
            None,
        ),
        (
            "livery COLOUR_END",
            Command::Livery(GroupID(0), Colours::COLOUR_END),
            false,
            ERROR,
            None,
        ),
        (
            "livery INVALID_COLOUR",
            Command::Livery(GroupID(0), Colours::INVALID_COLOUR),
            false,
            None,
            None,
        ),
        // `CmdSetGroupFlag`.
        (
            "flag invalid",
            Command::Flag(invalid, GroupFlag::ReplaceProtection),
            false,
            ERROR,
            None,
        ),
        (
            "flag other company's",
            Command::Flag(GroupID(2), GroupFlag::ReplaceProtection),
            false,
            ERROR,
            None,
        ),
        (
            "flag past ReplaceWagonRemoval",
            Command::Flag(GroupID(0), GroupFlag(2)),
            false,
            ERROR,
            None,
        ),
        (
            "flag",
            Command::Flag(GroupID(0), GroupFlag::ReplaceWagonRemoval),
            false,
            None,
            None,
        ),
    ];
    for (name, command, full, result, order) in rows {
        let _world = World::new();
        CAN_ALLOCATE.set(!full);
        let before = snapshot();
        assert_eq!(run(|cost| dispatch(w, cost, command)), result, "{name}");
        assert_eq!(snapshot(), before, "{name}");
        let calls = calls();
        if let Some(order) = order {
            assert_eq!(calls, order, "{name}");
        }
    }

    // `SetParent` with an invalid parent ID selects the root.
    let _world = World::new();
    let set_root = CmdAlterGroup {
        mode: set_parent,
        group_id: GroupID(1),
        parent_id: invalid,
        text: b"",
    };
    assert_eq!(
        run(|cost| cmd_alter_group(w, cost, execute(), set_root)),
        None
    );
    assert_eq!(GroupRef::of(record(1)).unwrap().parent(), GroupID::INVALID);
    assert!(GroupRef::of(record(0)).unwrap().children().next().is_none());
}

/// A fixture vehicle record of `kind` with `subtype`, owned by company 0 and in no group.
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
fn vehicle(kind: VehicleType, subtype: u8) -> VehicleRef {
    let record = crate::vehicle_base::openttd_rust_vehicle_record_new(ptr::null_mut(), kind);
    // SAFETY: The fresh record.
    unsafe {
        (*record).owner = CompanyID(0);
        (*record).subtype = subtype;
    }
    VehicleRef::of(record)
}
extern "C" fn default_group(_: VehicleRef) -> GroupID {
    GroupID::DEFAULT_GROUP
}
extern "C" fn no_profit(_: VehicleRef) -> i64 {
    0
}
extern "C" fn young(_: VehicleRef) -> i32 {
    0
}
const NOT_REACHED_EXIT: i32 = 87;
const NOT_REACHED_MARKER: &str = "openttd-rust test: fleet reached the native NOT_REACHED";

/// Evidence 3: `AddVehicleToGroup` on an effect vehicle ends at the native unreachable path.
#[test]
#[allow(unsafe_code, reason = "the fixture's owners, built and read in place")]
fn add_vehicle_to_group_of_an_effect_is_not_reached() {
    extern "C" fn not_reached() {
        eprintln!("{NOT_REACHED_MARKER}");
        std::process::exit(NOT_REACHED_EXIT);
    }
    extern "C" fn holds(condition: bool) {
        assert!(condition);
    }
    if crate::services_assert::isolate_test_ends(
        "fleet::tests::add_vehicle_to_group_of_an_effect_is_not_reached",
        NOT_REACHED_EXIT,
        NOT_REACHED_MARKER,
    ) {
        return;
    }
    let _world = World::new();
    let shared = Box::leak(Box::new(crate::services::Services {
        not_reached,
        map: crate::map_func::fixture::statics(),
        ..SHARED_TABLE
    }));
    // SAFETY: The leaked shared table and this thread's map statics live through the child.
    unsafe { crate::services_assert::bind_test_policy(holds, shared) };
    let w = Box::leak(Box::new(GroupServices {
        vehicles: Box::leak(Box::new(VehicleServices {
            profit_last_year: no_profit,
            maintenance: Box::leak(Box::new(crate::vehicle::maintenance::Services {
                economy_age: young,
                ..crate::test_tables::MAINTENANCE
            })),
            ..*vehicles()
        })),
        group_id: default_group,
        ..*table()
    }));
    add_vehicle_to_group(w, vehicle(VehicleType::VEH_EFFECT, 0), GroupID(0));
    unreachable!("NOT_REACHED ends the child");
}

/// Evidence 3: `SetTrainGroupID` on a wagon returns before its assertion for an invalid group
/// that is not `DEFAULT_GROUP`, and fails the assertion for a valid group.
#[test]
fn set_train_group_id_of_a_wagon_fails_the_assertion() {
    if crate::services_assert::isolate_test_exit(
        "fleet::tests::set_train_group_id_of_a_wagon_fails_the_assertion",
    ) {
        return;
    }
    let _world = World::new();
    let w = table();
    let wagon = vehicle(VehicleType::VEH_TRAIN, 0);
    set_train_group_id(w, wagon, GroupID(9));
    assert_eq!(calls(), ["get_if_valid 9"]);
    set_train_group_id(w, wagon, GroupID(3));
    unreachable!("the assertion ends the child");
}

/// Evidence 4: each `To*ID` returns the index for its own list type.
#[test]
fn vehicle_list_identifier_to_id() {
    let of = |r#type| VehicleListIdentifier {
        r#type,
        vtype: VehicleType::VEH_ROAD,
        company: CompanyID(0),
        index: 0x1_0005,
    };
    assert_eq!(
        of(VehicleListType::VL_STANDARD).to_company_id(),
        CompanyID(5)
    );
    assert_eq!(
        of(VehicleListType::VL_DEPOT_LIST).to_destination_id(),
        crate::order_type::DestinationID(5)
    );
    assert_eq!(of(VehicleListType::VL_GROUP_LIST).to_group_id(), GroupID(5));
    assert_eq!(
        of(VehicleListType::VL_STATION_LIST).to_station_id(),
        crate::station_type::StationID(5)
    );
    assert_eq!(
        of(VehicleListType::VL_SHARED_ORDERS).to_vehicle_id(),
        VehicleID(0x1_0005)
    );
}

/// Evidence 4: a `To*ID` of another list type fails the native assertion.
#[test]
fn vehicle_list_identifier_to_id_of_another_type_fails_the_assertion() {
    if crate::services_assert::isolate_test_exit(
        "fleet::tests::vehicle_list_identifier_to_id_of_another_type_fails_the_assertion",
    ) {
        return;
    }
    let vli = VehicleListIdentifier {
        r#type: VehicleListType::VL_STANDARD,
        vtype: VehicleType::VEH_ROAD,
        company: CompanyID(0),
        index: 5,
    };
    assert_eq!(vli.to_company_id(), CompanyID(5));
    let _ = vli.to_group_id();
    unreachable!("the assertion ends the child");
}

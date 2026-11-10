/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common vehicle transactions. Native shells, names and stack costs stay native;
//! Rust owns policy and transaction scratch. All accesses end before callbacks.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
pub(crate) mod articulated;
pub(crate) mod build;
pub(crate) mod clone;
mod clone_name;
pub(crate) mod construct;
pub(crate) mod mass;
pub(crate) mod rail;
pub(crate) mod refit;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::fleet::{ALL_GROUP, GroupServices};
use crate::network_type::ClientID;
use crate::order_type::{OT_GOTO_WAYPOINT, OT_IMPLICIT};
use crate::services::{ByteSpan, CommandCost, CommandCostServices, CommandServices, Services};
use crate::strings_type::INVALID_STRING_ID as ERROR;
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleType;
use crate::vehicle_type::VehicleID;
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};

#[repr(C)]
pub struct Target {
    pub owner: u8,
    pub primary: bool,
}
#[repr(C)]
pub struct TransactionServices {
    pub shared: &'static Services,
    pub commands: &'static CommandServices,
    pub cost: &'static CommandCostServices,
    pub groups: &'static GroupServices,
    pub target: extern "C" fn(*mut ()) -> Target,
    pub name: extern "C" fn(*mut ()) -> ByteSpan,
    pub clear_name: extern "C" fn(*mut ()),
    pub set_name: extern "C" fn(*mut (), ByteSpan),
    pub name_dirty: extern "C" fn(*mut ()),
    pub interval_percent: extern "C" fn(u8) -> bool,
    pub interval_clamp: extern "C" fn(i32, bool) -> u16,
    pub set_interval: extern "C" fn(*mut (), u16, bool, bool),
    pub unique_name: u32,
    pub max_name: usize,
}

unsafe fn unique_name(s: &TransactionServices, name: ByteSpan) -> bool {
    let mut from = 0;
    loop {
        let v = crate::vehicle::VehicleRef::shell_from_record(s.groups.next_vehicle(from));
        if v.is_null() {
            return true;
        }
        let current = (s.name)(v.cast());
        if current.size != 0 && current.same_bytes(name) {
            return false;
        }
        from = unsafe { (s.groups.vehicle_id)(v.cast()) }.wrapping_add(1);
    }
}
/// `Vehicle::GetIfValid`, `IsPrimaryVehicle` and `CheckOwnership(v->owner)`:
/// resolve once, preserving the original primary check before ownership.
unsafe fn owned_primary(
    s: &TransactionServices,
    cost: *mut CommandCost,
    id: VehicleID,
) -> (*mut (), CompanyID) {
    let v = unsafe { (s.groups.vehicle)(id.0) };
    if v.is_null() {
        (s.cost.error)(cost, ERROR);
        return (v, CompanyID(0));
    }
    let target = (s.target)(v);
    if !target.primary {
        (s.cost.error)(cost, ERROR);
        return (std::ptr::null_mut(), CompanyID(0));
    }
    unsafe { s.commands.ownership(cost, target.owner, 0) };
    if !(s.cost.success)(cost) {
        return (std::ptr::null_mut(), CompanyID(0));
    }
    (v, CompanyID(target.owner))
}
/// # Safety
/// Immutable tables and readable name span live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_unique_name_abi(
    s: &TransactionServices,
    name: ByteSpan,
) -> u8 {
    u8::from(unsafe { unique_name(s, name) })
}
/// `CmdRenameVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdRenameVehicle {
    /// Vehicle ID to name.
    pub(crate) veh_id: VehicleID,
    /// The new name or an empty string when resetting to the default.
    pub(crate) text: ByteSpan,
}
/// `CmdRenameVehicle`: give a custom name to your vehicle.
///
/// # Safety
/// Tables, caller-owned native cost and readable text remain live. No mutable
/// native name view or Rust owner borrow survives a service call.
pub(crate) unsafe fn cmd_rename_vehicle(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdRenameVehicle,
) {
    let (v, _) = unsafe { owned_primary(s, cost, cmd.veh_id) };
    if v.is_null() {
        return;
    }
    let text = cmd.text;
    let reset = text.size == 0;
    if !reset {
        if (s.shared.utf8_length)(text.data, text.size) >= s.max_name {
            return (s.cost.error)(cost, ERROR);
        }
        if !flags.test(DoCommandFlag::AutoReplace) && !unsafe { unique_name(s, text) } {
            return (s.cost.error)(cost, s.unique_name);
        }
    }
    if flags.test(DoCommandFlag::Execute) {
        if reset {
            (s.clear_name)(v);
        } else {
            (s.set_name)(v, text);
        }
        (s.name_dirty)(v);
    }
    (s.cost.zero)(cost);
}
/// # Safety
/// As `cmd_rename_vehicle`. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_rename(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    text: ByteSpan,
) {
    unsafe {
        cmd_rename_vehicle(
            s,
            cost,
            DoCommandFlags(flags),
            CmdRenameVehicle {
                veh_id: VehicleID(id),
                text,
            },
        );
    }
}
/// `CmdChangeServiceInt`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdChangeServiceInt {
    /// Vehicle ID that is being service-interval-changed.
    pub(crate) veh_id: VehicleID,
    /// New service interval.
    pub(crate) serv_int: u16,
    /// Service interval is custom flag.
    pub(crate) is_custom: bool,
    /// Service interval is percentage flag.
    pub(crate) is_percent: bool,
}
/// `CmdChangeServiceInt`: change the service interval of a vehicle.
///
/// # Safety
/// Immutable tables, native cost and resolved vehicle remain live for this call.
/// Setter and presentation leaves may reenter; no owner borrow spans them.
pub(crate) unsafe fn cmd_change_service_int(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdChangeServiceInt,
) {
    let CmdChangeServiceInt {
        veh_id,
        mut serv_int,
        is_custom,
        mut is_percent,
    } = cmd;
    let (v, owner) = unsafe { owned_primary(s, cost, veh_id) };
    if v.is_null() {
        return;
    }
    // The original resolves the company only after ownership succeeds.
    if !is_custom {
        is_percent = (s.interval_percent)(owner.0);
    }
    if is_custom {
        if serv_int != (s.interval_clamp)(i32::from(serv_int), is_percent) {
            return (s.cost.error)(cost, ERROR);
        }
    } else {
        serv_int = s.interval_default(
            owner,
            VehicleType(crate::fleet::record(s.groups, v).vehicle_type()),
        ) as u16;
    }
    if flags.test(DoCommandFlag::Execute) {
        (s.set_interval)(v, serv_int, is_custom, is_percent);
    }
    (s.cost.zero)(cost);
}
/// # Safety
/// As `cmd_change_service_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_service_interval(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    interval: u16,
    custom: bool,
    percent: bool,
) {
    unsafe {
        cmd_change_service_int(
            s,
            cost,
            DoCommandFlags(flags),
            CmdChangeServiceInt {
                veh_id: VehicleID(id),
                serv_int: interval,
                is_custom: custom,
                is_percent: percent,
            },
        );
    }
}

#[repr(C)]
pub struct SelectionServices {
    pub groups: &'static GroupServices,
    pub transactions: &'static crate::fleet::transactions::Services,
    pub vehicles: &'static crate::vehicle::VehicleServices,
    pub in_depot: extern "C" fn(*mut ()) -> bool,
    pub free_wagon: extern "C" fn(*mut ()) -> bool,
    pub first_engine_part: extern "C" fn(*mut ()) -> *mut (),
    pub multiheaded: extern "C" fn(*mut ()) -> bool,
    pub other_multihead: extern "C" fn(*mut ()) -> *mut (),
    pub with_orders: extern "C" fn(
        *mut (),
        unsafe extern "C" fn(*mut (), *const ()) -> u8,
        unsafe extern "C" fn(*mut (), *const ()) -> u8,
        unsafe extern "C" fn(*mut (), *const ()),
    ),
    pub list_clear: extern "C" fn(*mut ()),
    pub list_push: extern "C" fn(*mut (), *mut ()),
}

/// `GetVehicleSet`: calculates the set of vehicles that will be affected by a
/// given selection, emitting each vehicle's index.
unsafe fn vehicle_set(
    s: &SelectionServices,
    mut v: *mut (),
    mut num_vehicles: u8,
    mut emit: impl FnMut(u32),
) {
    if crate::fleet::record(s.groups, v).vehicle_type() != VehicleType::TRAIN.0 {
        return;
    }
    /* Only include whole vehicles, so start with the first articulated part */
    v = (s.first_engine_part)(v);
    /* Include num_vehicles vehicles, not counting articulated parts */
    while !v.is_null() && num_vehicles > 0 {
        loop {
            /* Include current vehicle in the selection. */
            emit(unsafe { (s.groups.vehicle_id)(v) });
            /* If the vehicle is multiheaded, add the other part too. */
            if (s.multiheaded)(v) {
                emit(unsafe { (s.groups.vehicle_id)((s.other_multihead)(v)) });
            }
            let handle = unsafe { vehicle_ref(v, s.vehicles) };
            v = (handle)
                .next()
                .map_or(std::ptr::null_mut(), |v| v.shell().cast());
            if v.is_null() || !crate::fleet::transactions::articulated(s.transactions, v) {
                break;
            }
        }
        num_vehicles -= 1;
    }
}
/// # Safety
/// A live vehicle and native selection set, with a synchronous include callback.
/// The callback may allocate the native set but cannot mutate vehicle shells.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_selection_set(
    s: &SelectionServices,
    v: *mut (),
    count: u8,
    set: *mut (),
    include: extern "C" fn(*mut (), u32),
) {
    unsafe { vehicle_set(s, v, count, |id| include(set, id)) };
}

/// `BuildDepotVehicleList`'s selection: each vehicle of `vehicle_type` in the
/// depot at `tile`, emitted with whether it belongs to the wagon list.
unsafe fn depot_list<F: FnMut(*mut (), bool)>(
    s: &SelectionServices,
    vehicle_type: VehicleType,
    tile: TileIndex,
    wagons: bool,
    individual_wagons: bool,
    mut emit: F,
) {
    struct Visit<'a, F> {
        s: &'a SelectionServices,
        vehicle_type: VehicleType,
        wagons: bool,
        individual_wagons: bool,
        emit: F,
    }
    unsafe extern "C" fn visit<F: FnMut(*mut (), bool)>(
        ctx: *mut std::ffi::c_void,
        handle: crate::vehicle::VehicleRef,
    ) {
        // Callback runs synchronously; emit mutates only caller-local selection.
        let c = unsafe { &mut *ctx.cast::<Visit<'_, F>>() };
        let v = handle.shell().cast();
        if VehicleType(crate::fleet::record(c.s.groups, v).vehicle_type()) != c.vehicle_type
            || !(c.s.in_depot)(v)
        {
            return;
        }
        if c.vehicle_type == VehicleType::TRAIN {
            if crate::fleet::transactions::articulated(c.s.transactions, v)
                || crate::ground_vehicle::is_rear_dualheaded(
                    crate::fleet::transactions::record(c.s.transactions, v).subtype(),
                )
            {
                return;
            }
            if c.wagons && (c.s.free_wagon)(crate::fleet::transactions::first(c.s.transactions, v))
            {
                if c.individual_wagons || (c.s.free_wagon)(v) {
                    (c.emit)(v, true);
                }
                return;
            }
        }
        if unsafe { (c.s.groups.primary)(v) } {
            (c.emit)(v, false);
        }
    }
    let mut ctx = Visit {
        s,
        vehicle_type,
        wagons,
        individual_wagons,
        emit: &mut emit,
    };
    unsafe {
        s.vehicles.for_each_on_tile(tile.0, |v| {
            visit::<&mut F>(std::ptr::from_mut(&mut ctx).cast(), v);
        });
    };
}
/// `BuildDepotVehicleList`: generate a list of vehicles inside a depot.
///
/// # Safety
/// Native engine/wagon vectors remain live; they may alias exactly as in the
/// original depot-sale path. Presentation vectors own their native allocations.
unsafe fn build_depot_vehicle_list(
    s: &SelectionServices,
    vehicle_type: VehicleType,
    tile: TileIndex,
    engines: *mut (),
    wagons: *mut (),
    individual_wagons: bool,
) {
    (s.list_clear)(engines);
    if !wagons.is_null() && engines != wagons {
        (s.list_clear)(wagons);
    }
    unsafe {
        depot_list(
            s,
            vehicle_type,
            tile,
            !wagons.is_null(),
            individual_wagons,
            |v, wagon| {
                (s.list_push)(if wagon { wagons } else { engines }, v);
            },
        );
    };
}
/// # Safety
/// As `build_depot_vehicle_list`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_list(
    s: &SelectionServices,
    kind: u8,
    tile: u32,
    engines: *mut (),
    wagons: *mut (),
    individual: bool,
) {
    unsafe {
        build_depot_vehicle_list(
            s,
            VehicleType(kind),
            TileIndex(tile),
            engines,
            wagons,
            individual,
        );
    }
}

/// `FindVehiclesWithOrder`: find vehicles matching an order. This can be used,
/// e.g. to find all vehicles that stop at a particular station. Here: each
/// vehicle of `vehicle_type` with a station or depot order to `destination`,
/// for `GenerateVehicleSortList`'s station and depot lists.
fn find_vehicles_with_order<F: FnMut(*mut ())>(
    s: &SelectionServices,
    vehicle_type: VehicleType,
    depot: bool,
    destination: u16,
    mut emit: F,
) {
    struct Find<'a, F> {
        s: &'a SelectionServices,
        vehicle_type: VehicleType,
        depot: bool,
        destination: u16,
        emit: F,
    }
    unsafe extern "C" fn vehicle<F>(ctx: *mut (), v: *const ()) -> u8 {
        let c = unsafe { &*ctx.cast::<Find<'_, F>>() };
        /* We assume all vehicles sharing an order list match the condition. */
        u8::from(
            VehicleType(crate::fleet::record(c.s.groups, v.cast_mut()).vehicle_type())
                == c.vehicle_type,
        )
    }
    unsafe extern "C" fn order<F>(ctx: *mut (), o: *const ()) -> u8 {
        let c = unsafe { &*ctx.cast::<Find<'_, F>>() };
        // Native Order is the aliased canonical Rust-owned scalar record.
        let o = crate::orders::OrderHandle(o.cast());
        u8::from(unsafe {
            if c.depot {
                o.is_type(crate::vehicle::OT_GOTO_DEPOT)
                    && !o.nearest_depot()
                    && o.destination() == c.destination
            } else {
                (o.is_type(crate::vehicle::OT_GOTO_STATION)
                    || o.is_type(OT_GOTO_WAYPOINT)
                    || o.is_type(OT_IMPLICIT))
                    && o.destination() == c.destination
            }
        })
    }
    unsafe extern "C" fn visit<F: FnMut(*mut ())>(ctx: *mut (), v: *const ()) {
        let c = unsafe { &mut *ctx.cast::<Find<'_, F>>() };
        /* An order matches, we can add all shared vehicles to the list. */
        (c.emit)(v.cast_mut());
    }
    let mut ctx = Find {
        s,
        vehicle_type,
        depot,
        destination,
        emit: &mut emit,
    };
    (s.with_orders)(
        std::ptr::from_mut(&mut ctx).cast(),
        vehicle::<&mut F>,
        order::<&mut F>,
        visit::<&mut F>,
    );
}
/// `GenerateVehicleSortList`'s selection: each vehicle of the list `vli`
/// describes, in the original order; whether the identifier was valid.
unsafe fn generate_list<F: FnMut(*mut ())>(
    s: &SelectionServices,
    vli: &VehicleListIdentifier,
    mut emit: F,
) -> bool {
    let kind = vli.list_type;
    let vehicle_type = vli.vtype.0;
    if kind == VehicleListType::VL_STATION_LIST || kind == VehicleListType::VL_DEPOT_LIST {
        find_vehicles_with_order(
            s,
            vli.vtype,
            kind == VehicleListType::VL_DEPOT_LIST,
            vli.index as u16,
            emit,
        );
        return true;
    }
    if kind == VehicleListType::VL_SHARED_ORDERS {
        /* Add all vehicles from this vehicle's shared order list */
        let mut v = unsafe { (s.groups.vehicle)(vli.index) };
        if v.is_null()
            || unsafe {
                crate::fleet::record(s.groups, v).vehicle_type() != vehicle_type
                    || !(s.groups.primary)(v)
            }
        {
            return false;
        }
        while !v.is_null() {
            emit(v);
            v = unsafe { (s.groups.next_shared)(v) };
        }
        return true;
    }
    if kind != VehicleListType::VL_STANDARD && kind != VehicleListType::VL_GROUP_LIST {
        return false;
    }
    let mut from = 0;
    loop {
        let v = crate::vehicle::VehicleRef::shell_from_record(s.groups.next_vehicle(from));
        if v.is_null() {
            break;
        }
        // Preserve the distinct predicate order of group and standard lists.
        if crate::fleet::record(s.groups, v).vehicle_type() == vehicle_type {
            let selected =
                if kind == VehicleListType::VL_GROUP_LIST && vli.index as u16 != ALL_GROUP {
                    unsafe {
                        (s.groups.primary)(v)
                            && CompanyID(crate::fleet::record(s.groups, v).owner()) == vli.company
                            && (crate::fleet::openttd_rust_fleet_contains_abi(
                                s.groups,
                                (s.groups.vehicle_group)(v),
                                vli.index as u16,
                            ) != 0)
                    }
                } else {
                    unsafe {
                        CompanyID(crate::fleet::record(s.groups, v).owner()) == vli.company
                            && (s.groups.primary)(v)
                    }
                };
            if selected {
                emit(v);
            }
        }
        from = unsafe { (s.groups.vehicle_id)(v) }.wrapping_add(1);
    }
    true
}
/// `GenerateVehicleSortList`: generate a list of vehicles based on window type.
///
/// # Safety
/// The native presentation list is live and remains valid across append leaves.
unsafe fn generate_vehicle_sort_list(
    s: &SelectionServices,
    list: *mut (),
    vli: &VehicleListIdentifier,
) -> bool {
    (s.list_clear)(list);
    unsafe {
        generate_list(s, vli, |v| {
            (s.list_push)(list, v);
        })
    }
}
/// # Safety
/// As `generate_vehicle_sort_list`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_generate_list_abi(
    s: &SelectionServices,
    list: *mut (),
    kind: u8,
    vehicle_type: u8,
    company: u8,
    index: u32,
) -> u8 {
    u8::from(unsafe {
        generate_vehicle_sort_list(
            s,
            list,
            &VehicleListIdentifier {
                list_type: VehicleListType(kind),
                vtype: VehicleType(vehicle_type),
                company: CompanyID(company),
                index,
            },
        )
    })
}

#[repr(C)]
pub struct AircraftStartState {
    pub state: u8,
    pub flags: u8,
}
#[repr(C)]
pub struct TransactionStrings {
    pub destroyed: u32,
    pub stopped_in_depot: u32,
    pub no_power: u32,
    pub in_flight: u32,
    pub incompatible_rail: u32,
    pub null_string: u32,
    pub grf_text: GrfStringId,
    pub takeoff: u8,
    pub terminal7: u8,
    pub direct_descent: u8,
    pub new_vehicle_expense: u8,
    pub invalid_client: u32,
    pub share_orders: u8,
    pub copy_orders: u8,
    pub cargo_count: u8,
    pub invalid_cargo: u8,
}
#[repr(C)]
pub struct CommonServices {
    pub base: &'static TransactionServices,
    pub transactions: &'static crate::fleet::transactions::Services,
    pub vehicles: &'static crate::vehicle::VehicleServices,
    pub orders: &'static crate::orders::Leaves,
    pub strings: TransactionStrings,
    pub train_power: extern "C" fn(*mut ()) -> u32,
    pub aircraft_start: extern "C" fn(*mut ()) -> AircraftStartState,
    pub grf_string: extern "C" fn(u32, u32) -> u32,
    pub waiting_news: extern "C" fn(u32),
    pub flip_stopped: extern "C" fn(*mut ()),
    pub startstop_ui: extern "C" fn(*mut ()),
    pub value: extern "C" fn(*mut ()) -> i64,
    pub valid_cargo: extern "C" fn(u8) -> bool,
}
/// # Safety
/// `v` denotes a live shell, established by the caller's source preconditions.
unsafe fn vehicle_ref(
    v: *mut (),
    services: &crate::vehicle::VehicleServices,
) -> crate::vehicle::VehicleRef {
    unsafe { crate::vehicle::VehicleRef::from_shell(v.cast(), services) }
}
/// `CmdSellVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSellVehicle {
    /// Vehicle ID being sold.
    pub(crate) v_id: VehicleID,
    /// Sell the vehicle and all vehicles following it in the chain.
    pub(crate) sell_chain: bool,
    /// Make a backup of the vehicle's order (if an engine).
    pub(crate) backup_order: bool,
    /// User.
    pub(crate) client_id: ClientID,
}
/// Raw adapter: `CmdSellVehicle` over base values, for `fleet/transactions.rs`,
/// `vehicle_transactions/build.rs` and `vehicle_transactions/clone.rs` until their passes.
///
/// # Safety
/// As `cmd_sell_vehicle`.
pub(crate) unsafe fn sell(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    chain: bool,
    backup: bool,
    client: u32,
) {
    unsafe {
        cmd_sell_vehicle(
            s,
            cost,
            DoCommandFlags(flags),
            CmdSellVehicle {
                v_id: VehicleID(id),
                sell_chain: chain,
                backup_order: backup,
                client_id: ClientID(client),
            },
        );
    }
}
/// `CmdSellVehicle`: sell a vehicle.
///
/// # Safety
/// Immutable service tables and native stack cost remain live; shell ownership
/// follows the original command preconditions. No borrow survives deletion/reentry.
pub(crate) unsafe fn cmd_sell_vehicle(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdSellVehicle,
) {
    let CmdSellVehicle {
        v_id,
        sell_chain,
        backup_order,
        client_id,
    } = cmd;
    let b = s.base;
    let v = unsafe { (b.groups.vehicle)(v_id.0) };
    if v.is_null() {
        return (b.cost.error)(cost, ERROR);
    }
    let front = crate::fleet::transactions::first(s.transactions, v);
    unsafe {
        b.commands
            .ownership(cost, crate::fleet::record(b.groups, front).owner(), 0);
    };
    if !(b.cost.success)(cost) {
        return;
    }
    if crate::fleet::transactions::record(s.transactions, front).status()
        & crate::vehicle_base::VS_CRASHED
        != 0
    {
        return (b.cost.error)(cost, s.strings.destroyed);
    }
    if !unsafe { (s.transactions.stopped_in_depot)(front) } {
        return (b.cost.error)(
            cost,
            s.strings.stopped_in_depot
                + u32::from(crate::fleet::record(b.groups, front).vehicle_type()),
        );
    }
    if crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN.0 {
        // The build root is late-bound through the fleet table, as autoreplace does.
        let root = (s.transactions.build_services)();
        unsafe {
            rail::sell(
                root,
                cost,
                flags.0,
                v,
                sell_chain,
                backup_order,
                client_id.0,
            );
        };
        return;
    }
    (b.cost.expense_money)(
        cost,
        s.strings.new_vehicle_expense,
        (s.value)(front).saturating_neg(),
    );
    if flags.test(DoCommandFlag::Execute) {
        if unsafe { (b.groups.primary)(front) } && backup_order {
            unsafe {
                crate::orders::openttd_rust_backup_create(front.cast(), client_id.0, s.orders);
            };
        }
        // No owner access or retained reference crosses the deleting callback.
        (s.vehicles.destroy)(unsafe { vehicle_ref(front, s.vehicles) });
    }
}
/// `CmdStartStopVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdStartStopVehicle {
    /// Vehicle to start/stop.
    pub(crate) veh_id: VehicleID,
    /// Shall the start/stop newgrf callback be evaluated (only valid with
    /// `DoCommandFlag::AutoReplace` for network safety).
    pub(crate) evaluate_startstop_cb: bool,
}
/// Raw adapter: `CmdStartStopVehicle` over base values, for `fleet/transactions.rs`
/// until its pass.
///
/// # Safety
/// As `cmd_start_stop_vehicle`.
pub(crate) unsafe fn startstop(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    evaluate: bool,
) {
    unsafe {
        cmd_start_stop_vehicle(
            s,
            cost,
            DoCommandFlags(flags),
            CmdStartStopVehicle {
                veh_id: VehicleID(id),
                evaluate_startstop_cb: evaluate,
            },
        );
    }
}
/// `CmdStartStopVehicle`: start/stop a vehicle.
///
/// # Safety
/// Immutable service tables and stack cost remain live for this synchronous entry.
/// Register 100 follows the original successful `NewGRF` callback preconditions.
pub(crate) unsafe fn cmd_start_stop_vehicle(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdStartStopVehicle,
) {
    let CmdStartStopVehicle {
        veh_id,
        mut evaluate_startstop_cb,
    } = cmd;
    /* Disable the effect of evaluate_startstop_cb, when DoCommandFlag::AutoReplace is not set */
    if !flags.test(DoCommandFlag::AutoReplace) {
        evaluate_startstop_cb = true;
    }
    let b = s.base;
    let (v, _) = unsafe { owned_primary(b, cost, veh_id) };
    if v.is_null() {
        return;
    }
    let handle = unsafe { vehicle_ref(v, s.vehicles) };
    let status = (handle).status();
    if status & crate::vehicle_base::VS_CRASHED != 0 {
        return (b.cost.error)(cost, s.strings.destroyed);
    }
    match VehicleType(crate::fleet::record(b.groups, v).vehicle_type()) {
        VehicleType::TRAIN => {
            if status & crate::vehicle_base::VS_STOPPED != 0 && (s.train_power)(v) == 0 {
                return (b.cost.error)(cost, s.strings.no_power);
            }
        }
        VehicleType::ROAD | VehicleType::SHIP => (),
        VehicleType::AIRCRAFT => {
            /* cannot stop airplane when in flight, or when taking off / landing */
            let state = (s.aircraft_start)(v);
            if (state.state >= s.strings.takeoff && state.state < s.strings.terminal7)
                || state.flags & s.strings.direct_descent != 0
            {
                return (b.cost.error)(cost, s.strings.in_flight);
            }
        }
        _ => return (b.cost.error)(cost, ERROR),
    }
    if evaluate_startstop_cb {
        /* Check if this vehicle can be started/stopped. Failure means 'allow'. */
        let mut register100 = std::mem::MaybeUninit::<i32>::uninit();
        let callback = unsafe {
            crate::newgrf_vehicle::callback(
                s.vehicles,
                Some(vehicle_ref(v, s.vehicles)),
                (vehicle_ref(v, s.vehicles)).engine_id(),
                crate::newgrf_callbacks::CBID_VEHICLE_START_STOP_CHECK.0,
                0,
                std::ptr::null_mut(),
                false,
                register100.as_mut_ptr(),
                1,
            )
        };
        let mut error = s.strings.null_string;
        if callback != crate::vehicle::CALLBACK_FAILED {
            if s.engine_grf_version(unsafe { vehicle_ref(v, s.vehicles) }) < 8 {
                /* 8 bit result 0xFF means 'allow' */
                if callback < 0x400 && callback & 0xff != 0xff {
                    error = s.grf_text(v, s.strings.grf_text.0.wrapping_add(u32::from(callback)));
                }
            } else if callback < 0x400 {
                error = s.grf_text(v, s.strings.grf_text.0.wrapping_add(u32::from(callback)));
            } else {
                match callback {
                    0x400 => (), // allow
                    0x40f => {
                        // The original reads register 100 only for this result.
                        error = s.grf_text(v, unsafe { register100.assume_init() } as u32);
                    }
                    _ => error = s.strings.incompatible_rail, // unknown reason -> disallow
                }
            }
        }
        if error != s.strings.null_string {
            return (b.cost.error)(cost, error);
        }
    }
    if flags.test(DoCommandFlag::Execute) {
        if unsafe { (s.transactions.stopped_in_depot)(v) }
            && !flags.test(DoCommandFlag::AutoReplace)
        {
            (s.waiting_news)(veh_id.0);
        }
        (s.flip_stopped)(v);
        if crate::fleet::record(b.groups, v).vehicle_type() != VehicleType::TRAIN.0 {
            (handle).write_speed(0); // trains can stop 'slowly'
        }
        /* Unbunching data is no longer valid. */
        (s.vehicles.reset_depot_unbunching)(handle);
        // Native presentation may reenter owners; no Rust owner borrow spans it.
        (unsafe { &*s.vehicles.maintenance }.mark_dirty)(handle);
        (s.startstop_ui)(v);
    }
    (b.cost.zero)(cost);
}
/// # Safety
/// As `cmd_sell_vehicle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_sell(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    chain: bool,
    backup: bool,
    client: u32,
) {
    unsafe {
        cmd_sell_vehicle(
            s,
            cost,
            DoCommandFlags(flags),
            CmdSellVehicle {
                v_id: VehicleID(id),
                sell_chain: chain,
                backup_order: backup,
                client_id: ClientID(client),
            },
        );
    }
}
/// # Safety
/// As `cmd_start_stop_vehicle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_startstop(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    evaluate: bool,
) {
    unsafe {
        cmd_start_stop_vehicle(
            s,
            cost,
            DoCommandFlags(flags),
            CmdStartStopVehicle {
                veh_id: VehicleID(id),
                evaluate_startstop_cb: evaluate,
            },
        );
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "GRFStringID" ["probe"] => GrfStringId { };
    "OpenTTDAircraftStartState" ["probe"] => AircraftStartState {
        state, flags,
    };
    "OpenTTDVehicleTransactionStrings" ["probe"] => TransactionStrings {
        destroyed, stopped_in_depot, no_power, in_flight, incompatible_rail, null_string,
        grf_text, takeoff, terminal7, direct_descent, new_vehicle_expense,
        invalid_client, share_orders, copy_orders, cargo_count, invalid_cargo,
    };
    "OpenTTDVehicleCommonServices" ["probe"] => CommonServices {
        base, transactions, vehicles, orders, strings, train_power, aircraft_start,
        grf_string, waiting_news, flip_stopped,
        startstop_ui, value, valid_cargo,
    };
    "OpenTTDVehicleSelectionServices" ["probe"] => SelectionServices {
        groups, transactions, vehicles, in_depot, free_wagon, first_engine_part,
        multiheaded, other_multihead, with_orders, list_clear, list_push,
    };
    "OpenTTDVehicleTransactionTarget" ["probe"] => Target {
        owner, primary,
    };
    "OpenTTDVehicleTransactionServices" ["probe"] => TransactionServices {
        shared, commands, cost, groups, target, name, clear_name, set_name, name_dirty,
        interval_percent, interval_clamp, set_interval, unique_name, max_name,
    };
}

impl CommonServices {
    fn engine_grf_version(&self, v: crate::vehicle::VehicleRef) -> u8 {
        (self.vehicles.engine)(v).grf_version(self.base.shared)
    }
    fn grf_text(&self, v: *mut (), string: u32) -> u32 {
        let cap = unsafe { (&*(self.vehicles.newgrf)()).capabilities };
        let e = (self.vehicles.engine)(unsafe { vehicle_ref(v, self.vehicles) })
            .0
            .as_ptr();
        let grfid = unsafe { crate::engine_capabilities::openttd_rust_engine_grfid(cap, e) };
        (self.grf_string)(grfid, string)
    }
}

impl TransactionServices {
    /// `GetServiceIntervalForType`'s default: the owner's service interval for
    /// vehicles of `kind`.
    pub(crate) fn interval_default(&self, owner: CompanyID, kind: VehicleType) -> i32 {
        let company = unsafe { &*(self.commands.company)() };
        crate::company::service_interval((company.settings)(), Some(owner), kind)
    }
    /// Raw adapter: the service interval default over base values, for
    /// `settings_table.rs` until its pass.
    pub(crate) fn interval_default_raw(&self, owner: u8, kind: u8) -> i32 {
        self.interval_default(CompanyID(owner), VehicleType(kind))
    }
}

/// Compatibility import of the original header identity.
pub use crate::newgrf_text_type::GRFStringID as GrfStringId;

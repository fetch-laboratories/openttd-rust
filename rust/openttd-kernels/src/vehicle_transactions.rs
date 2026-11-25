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
use crate::aircraft::AirVehicleFlags;
use crate::airport::AirportMovementStates;
use crate::bitmath_func::has_bit;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::economy_type::ExpensesType;
use crate::fleet::{ALL_GROUP, GroupServices};
use crate::network_type::ClientID;
use crate::newgrf_text_type::GRFStringID;
use crate::order_type::OrderType;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, CommandServices, Services};
use crate::strings_type::INVALID_STRING_ID as ERROR;
use crate::strings_type::StringID;
use crate::table::strings::{
    STR_ERROR_AIRCRAFT_IS_IN_FLIGHT, STR_ERROR_INCOMPATIBLE_RAIL_TYPES,
    STR_ERROR_NAME_MUST_BE_UNIQUE, STR_ERROR_TRAIN_MUST_BE_STOPPED_INSIDE_DEPOT,
    STR_ERROR_TRAIN_START_NO_POWER, STR_ERROR_VEHICLE_IS_DESTROYED, STR_NULL,
};
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleRef;
use crate::vehicle_base::VehState;
use crate::vehicle_gui::get_window_class_for_vehicle_type;
use crate::vehicle_type::VehicleID;
use crate::vehicle_type::VehicleType;
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};
use crate::widgets::vehicle_widget::VehicleViewWidgets;
use crate::window_func::{
    invalidate_window_classes_data, invalidate_window_data, set_window_classes_dirty,
    set_window_dirty, set_window_widget_dirty,
};
use crate::window_type::{WindowClass, WindowNumber};

#[repr(C)]
pub struct TransactionServices {
    pub shared: &'static Services,
    pub commands: &'static CommandServices,
    pub cost: &'static CommandCostServices,
    pub groups: &'static GroupServices,
    pub name: extern "C" fn(*mut ()) -> ByteSpan,
    pub clear_name: extern "C" fn(*mut ()),
    pub set_name: extern "C" fn(*mut (), ByteSpan),
    pub interval_clamp: extern "C" fn(i32, bool) -> u16,
    pub set_interval: extern "C" fn(*mut (), u16, bool, bool),
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
/// resolve once, preserving the original primary check before ownership. A
/// non-null shell comes with its record's handle.
unsafe fn owned_primary(
    s: &TransactionServices,
    cost: *mut CommandCost,
    id: VehicleID,
) -> (*mut (), CompanyID, Option<VehicleRef>) {
    let v = unsafe { (s.groups.vehicle)(id.0) };
    if v.is_null() {
        (s.cost.error)(cost, ERROR);
        return (v, CompanyID(0), None);
    }
    if !unsafe { (s.groups.primary)(v) } {
        (s.cost.error)(cost, ERROR);
        return (std::ptr::null_mut(), CompanyID(0), None);
    }
    let handle = crate::fleet::record(s.groups, v);
    let owner = handle.owner();
    unsafe { s.commands.ownership(cost, owner.0, 0) };
    if !(s.cost.success)(cost) {
        return (std::ptr::null_mut(), CompanyID(0), None);
    }
    (v, owner, Some(handle))
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
    // `owned_primary` returns the record exactly when the vehicle is owned and primary.
    let (v, _, Some(handle)) = (unsafe { owned_primary(s, cost, cmd.veh_id) }) else {
        return;
    };
    let text = cmd.text;
    let reset = text.size == 0;
    if !reset {
        if (s.shared.utf8_length)(text.data, text.size) >= s.max_name {
            return (s.cost.error)(cost, ERROR);
        }
        if !flags.test(DoCommandFlag::AutoReplace) && !unsafe { unique_name(s, text) } {
            return (s.cost.error)(cost, STR_ERROR_NAME_MUST_BE_UNIQUE.0);
        }
    }
    if flags.test(DoCommandFlag::Execute) {
        if reset {
            (s.clear_name)(v);
        } else {
            (s.set_name)(v, text);
        }
        invalidate_window_classes_data(
            get_window_class_for_vehicle_type(handle.vehicle_type()),
            1,
            false,
        );
        (s.shared.screen_dirty)();
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
    let (v, owner, _) = unsafe { owned_primary(s, cost, veh_id) };
    if v.is_null() {
        return;
    }
    // The original resolves the company once, after ownership succeeds.
    let company = s.owner_company(owner);
    if !is_custom {
        is_percent = s.interval_is_percent_of(company);
    }
    if is_custom {
        if serv_int != (s.interval_clamp)(i32::from(serv_int), is_percent) {
            return (s.cost.error)(cost, ERROR);
        }
    } else {
        serv_int =
            s.interval_default_of(company, crate::fleet::record(s.groups, v).vehicle_type()) as u16;
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
    pub links: &'static crate::station_service::Links,
    /// The shared table, whose map descriptor the depot tile walk hashes with.
    pub shared: &'static Services,
    pub in_depot: extern "C" fn(*mut ()) -> bool,
    pub list_clear: extern "C" fn(*mut ()),
    pub list_push: extern "C" fn(*mut (), *mut ()),
}

/// `GetVehicleSet`: calculates the set of vehicles that will be affected by a
/// given selection, emitting each vehicle's index.
unsafe fn vehicle_set(
    s: &SelectionServices,
    mut v: *mut (),
    mut num_vehicles: u8,
    mut emit: impl FnMut(VehicleID),
) {
    if crate::fleet::record(s.groups, v).vehicle_type() != VehicleType::TRAIN {
        return;
    }
    /* Only include whole vehicles, so start with the first articulated part */
    v = unsafe { vehicle_ref(v, s.vehicles) }
        .first_engine_part()
        .shell()
        .cast();
    /* Include num_vehicles vehicles, not counting articulated parts */
    while !v.is_null() && num_vehicles > 0 {
        loop {
            /* Include current vehicle in the selection. */
            emit(VehicleID(unsafe { (s.groups.vehicle_id)(v) }));
            /* If the vehicle is multiheaded, add the other part too. */
            let u = unsafe { vehicle_ref(v, s.vehicles) };
            if crate::ground_vehicle::is_multiheaded(u.subtype()) {
                let other = u
                    .other_multiheaded_part()
                    .map_or(std::ptr::null_mut(), |o| o.shell().cast::<()>());
                emit(VehicleID(unsafe { (s.groups.vehicle_id)(other) }));
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
    unsafe { vehicle_set(s, v, count, |id| include(set, id.0)) };
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
        if crate::fleet::record(c.s.groups, v).vehicle_type() != c.vehicle_type
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
            let free_wagon = |w: *mut ()| {
                crate::ground_vehicle::is_free_wagon(crate::fleet::record(c.s.groups, w).subtype())
            };
            if c.wagons && free_wagon(crate::fleet::transactions::first(c.s.transactions, v)) {
                if c.individual_wagons || free_wagon(v) {
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
        crate::vehicle_func::vehicles_on_tile(s.shared.map(), tile, |v| {
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

/// `GenerateVehicleSortList`'s selection: each vehicle of the list `vli`
/// describes, in the original order; whether the identifier was valid.
unsafe fn generate_list<F: FnMut(*mut ())>(
    s: &SelectionServices,
    vli: &VehicleListIdentifier,
    mut emit: F,
) -> bool {
    let kind = vli.list_type;
    let vehicle_type = vli.vtype;
    if kind == VehicleListType::VL_STATION_LIST || kind == VehicleListType::VL_DEPOT_LIST {
        let destination = vli.index as u16;
        let ord_pred = |order: &crate::station_service::OrderDestination| {
            if kind == VehicleListType::VL_STATION_LIST {
                (order.kind == OrderType::OT_GOTO_STATION
                    || order.kind == OrderType::OT_GOTO_WAYPOINT
                    || order.kind == OrderType::OT_IMPLICIT)
                    && order.destination == destination
            } else {
                order.kind == OrderType::OT_GOTO_DEPOT
                    && !order.nearest_depot
                    && order.destination == destination
            }
        };
        crate::vehiclelist_func::find_vehicles_with_order(
            s.links,
            |v| v.vehicle_type() == vli.vtype,
            ord_pred,
            |v| emit(v.shell().cast()),
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
        let record = s.groups.next_vehicle(from);
        let Some(handle) = std::ptr::NonNull::new(record).map(crate::vehicle::VehicleRef) else {
            break;
        };
        let v = crate::vehicle::VehicleRef::shell_from_record(record);
        // Preserve the distinct predicate order of group and standard lists.
        if handle.vehicle_type() == vehicle_type {
            let selected =
                if kind == VehicleListType::VL_GROUP_LIST && vli.index as u16 != ALL_GROUP {
                    unsafe {
                        (s.groups.primary)(v)
                            && handle.owner() == vli.company
                            && (crate::fleet::openttd_rust_fleet_contains_abi(
                                s.groups,
                                (s.groups.vehicle_group)(v),
                                vli.index as u16,
                            ) != 0)
                    }
                } else {
                    unsafe { handle.owner() == vli.company && (s.groups.primary)(v) }
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
pub struct CommonServices {
    pub base: &'static TransactionServices,
    pub transactions: &'static crate::fleet::transactions::Services,
    pub vehicles: &'static crate::vehicle::VehicleServices,
    pub orders: &'static crate::orders::Leaves,
    pub aircraft_start: extern "C" fn(*mut ()) -> AircraftStartState,
    pub grf_string: extern "C" fn(u32, u32) -> u32,
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
            .ownership(cost, crate::fleet::record(b.groups, front).owner().0, 0);
    };
    if !(b.cost.success)(cost) {
        return;
    }
    if crate::fleet::transactions::record(s.transactions, front)
        .vehstatus()
        .test(VehState::Crashed)
    {
        return (b.cost.error)(cost, STR_ERROR_VEHICLE_IS_DESTROYED.0);
    }
    if !unsafe { (s.transactions.stopped_in_depot)(front) } {
        return (b.cost.error)(
            cost,
            STR_ERROR_TRAIN_MUST_BE_STOPPED_INSIDE_DEPOT.0
                + u32::from(crate::fleet::record(b.groups, front).vehicle_type().0),
        );
    }
    if crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN {
        // The build root is late-bound through the fleet table, as autoreplace does.
        let root = (s.transactions.build_services)();
        unsafe {
            rail::cmd_sell_rail_wagon(
                root,
                cost,
                flags,
                rail::CmdSellRailWagon {
                    t: v,
                    sell_chain,
                    backup_order,
                    user: client_id,
                },
            );
        };
        return;
    }
    (b.cost.expense_money)(
        cost,
        ExpensesType::EXPENSES_NEW_VEHICLES.0,
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
#[allow(clippy::too_many_lines)] // The complete CmdStartStopVehicle body.
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
    let (v, _, handle) = unsafe { owned_primary(b, cost, veh_id) };
    let Some(handle) = handle else {
        return;
    };
    let status = (handle).vehstatus();
    if status.test(VehState::Crashed) {
        return (b.cost.error)(cost, STR_ERROR_VEHICLE_IS_DESTROYED.0);
    }
    match handle.vehicle_type() {
        VehicleType::TRAIN => {
            if status.test(VehState::Stopped)
                && crate::raw_field::read_field!(handle.ground_cache(), cached_power) == 0
            {
                return (b.cost.error)(cost, STR_ERROR_TRAIN_START_NO_POWER.0);
            }
        }
        VehicleType::ROAD | VehicleType::SHIP => (),
        VehicleType::AIRCRAFT => {
            /* cannot stop airplane when in flight, or when taking off / landing */
            let state = (s.aircraft_start)(v);
            if (state.state >= AirportMovementStates::STARTTAKEOFF.0
                && state.state < AirportMovementStates::TERM7.0)
                || has_bit(state.flags, AirVehicleFlags::VAF_HELI_DIRECT_DESCENT.0)
            {
                return (b.cost.error)(cost, STR_ERROR_AIRCRAFT_IS_IN_FLIGHT.0);
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
                Some(handle),
                handle.engine_type().0,
                crate::newgrf_callbacks::CBID_VEHICLE_START_STOP_CHECK.0,
                0,
                std::ptr::null_mut(),
                false,
                register100.as_mut_ptr(),
                1,
            )
        };
        let mut error = STR_NULL;
        if callback != crate::newgrf_callbacks::CALLBACK_FAILED {
            if s.engine_grf_version(handle) < 8 {
                /* 8 bit result 0xFF means 'allow' */
                if callback < 0x400 && callback & 0xff != 0xff {
                    error = s.grf_text(
                        handle,
                        GRFStringID(
                            GRFStringID::GRFSTR_MISC_GRF_TEXT
                                .0
                                .wrapping_add(u32::from(callback)),
                        ),
                    );
                }
            } else if callback < 0x400 {
                error = s.grf_text(
                    handle,
                    GRFStringID(
                        GRFStringID::GRFSTR_MISC_GRF_TEXT
                            .0
                            .wrapping_add(u32::from(callback)),
                    ),
                );
            } else {
                match callback {
                    0x400 => (), // allow
                    0x40f => {
                        // The original reads register 100 only for this result.
                        error = s.grf_text(
                            handle,
                            GRFStringID(unsafe { register100.assume_init() } as u32),
                        );
                    }
                    _ => error = STR_ERROR_INCOMPATIBLE_RAIL_TYPES, // unknown reason -> disallow
                }
            }
        }
        if error != STR_NULL {
            return (b.cost.error)(cost, error.0);
        }
    }
    if flags.test(DoCommandFlag::Execute) {
        if unsafe { (s.transactions.stopped_in_depot)(v) }
            && !flags.test(DoCommandFlag::AutoReplace)
        {
            crate::news_func::delete_vehicle_news(
                veh_id,
                crate::news_type::AdviceType::VehicleWaiting,
            );
        }
        // The stopped bit is read again here, after the news and callback above.
        handle.set_vehstatus(handle.vehstatus().flip(VehState::Stopped));
        if handle.vehicle_type() != VehicleType::TRAIN {
            (handle).set_cur_speed(0); // trains can stop 'slowly'
        }
        /* Unbunching data is no longer valid. */
        crate::orders::reset_unbunching(handle);
        // Native presentation may reenter owners; no Rust owner borrow spans it.
        (unsafe { &*s.vehicles.maintenance }.mark_dirty)(handle);
        set_window_widget_dirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(veh_id),
            VehicleViewWidgets::WID_VV_START_STOP.0,
        );
        set_window_dirty(
            WindowClass::WC_VEHICLE_DEPOT,
            WindowNumber::from(handle.tile()),
        );
        set_window_classes_dirty(get_window_class_for_vehicle_type(handle.vehicle_type()));
        invalidate_window_data(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(veh_id),
            0,
            false,
        );
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
    "OpenTTDAircraftStartState" ["probe"] => AircraftStartState {
        state, flags,
    };
    "OpenTTDVehicleCommonServices" ["probe"] => CommonServices {
        base, transactions, vehicles, orders, aircraft_start,
        grf_string,
        value, valid_cargo,
    };
    "OpenTTDVehicleSelectionServices" ["probe"] => SelectionServices {
        groups, transactions, vehicles, links, shared, in_depot, list_clear, list_push,
    };
    "OpenTTDVehicleTransactionServices" ["probe"] => TransactionServices {
        shared, commands, cost, groups, name, clear_name, set_name,
        interval_clamp, set_interval, max_name,
    };
}

impl CommonServices {
    fn engine_grf_version(&self, v: crate::vehicle::VehicleRef) -> u8 {
        (self.vehicles.engine)(v).grf_version(self.base.shared)
    }
    /// `GetGRFStringID(v->GetGRFID(), string)`.
    fn grf_text(&self, v: VehicleRef, string: GRFStringID) -> StringID {
        let cap = unsafe { (&*(self.vehicles.newgrf)()).capabilities };
        let e = (self.vehicles.engine)(v).0.as_ptr();
        let grfid = unsafe { crate::engine_capabilities::openttd_rust_engine_grfid(cap, e) };
        StringID((self.grf_string)(grfid, string.0))
    }
}

impl TransactionServices {
    /// The company table.
    fn company_table(&self) -> &crate::company::Services {
        // SAFETY: The command table names the process-lifetime company table.
        unsafe { &*(self.commands.company)() }
    }
    /// `Company::Get(owner)`.
    pub(crate) fn owner_company(
        &self,
        owner: CompanyID,
    ) -> *mut crate::company_type::OpenTTDCompany {
        (self.company_table().identity.company)(owner)
    }
    /// `company->settings.vehicle.servint_ispercent` of a resolved company.
    pub(crate) fn interval_is_percent_of(
        &self,
        company: *mut crate::company_type::OpenTTDCompany,
    ) -> bool {
        let defaults = (self.company_table().identity.vehicle_defaults)(company);
        crate::raw_field::read_field!(defaults, servint_ispercent)
    }
    /// `CompanyServiceInterval(company, kind)` of a resolved company.
    pub(crate) fn interval_default_of(
        &self,
        company: *mut crate::company_type::OpenTTDCompany,
        kind: VehicleType,
    ) -> i32 {
        let table = self.company_table();
        crate::company::company_service_interval(
            (table.settings)(),
            table.identity,
            Some(company),
            kind,
        )
    }
    /// `Company::Get(owner)->settings.vehicle.servint_ispercent`.
    pub(crate) fn interval_is_percent(&self, owner: CompanyID) -> bool {
        self.interval_is_percent_of(self.owner_company(owner))
    }
    /// `GetServiceIntervalForType`'s default: the owner's service interval for
    /// vehicles of `kind`.
    pub(crate) fn interval_default(&self, owner: CompanyID, kind: VehicleType) -> i32 {
        self.interval_default_of(self.owner_company(owner), kind)
    }
    /// Raw adapter: the service interval default over base values, for
    /// `settings_table.rs` until its pass.
    pub(crate) fn interval_default_raw(&self, owner: u8, kind: u8) -> i32 {
        self.interval_default(CompanyID(owner), VehicleType(kind))
    }
}

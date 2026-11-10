/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Depot and list command dispatch with retained, transaction-local selection.
use super::{CommonServices, ERROR, SelectionServices, depot_list, generate_list};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::game_state::GameState;
use crate::network_type::ClientID;
use crate::services::{CommandCost, Cost};
use crate::tile_type::TileIndex;
use crate::vehicle_type::{DepotCommandFlags, VehicleID, VehicleType};
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};

/// The native `VehicleListIdentifier` and its `Valid()` result.
#[repr(C)]
pub struct ListId {
    pub kind: u8,
    pub vehicle_type: u8,
    pub company: u8,
    pub index: u32,
    pub valid: bool,
}
#[repr(C)]
pub struct Services {
    pub common: &'static CommonServices,
    pub selection: &'static SelectionServices,
    pub is_depot: extern "C" fn(u32) -> bool,
    pub send: extern "C" fn(*mut CommandCost, *mut (), u16, u8),
    pub service: u8,
    pub mass_send: u8,
    pub dont_cancel: u8,
    pub invalid_client: u32,
}
impl ListId {
    /// The typed identifier.
    fn identifier(&self) -> VehicleListIdentifier {
        VehicleListIdentifier {
            list_type: VehicleListType(self.kind),
            vtype: VehicleType(self.vehicle_type),
            company: CompanyID(self.company),
            index: self.index,
        }
    }
}
static POOL: GameState<Vec<Vec<*mut ()>>> = GameState::new(Vec::new());
struct List(Vec<*mut ()>);
impl List {
    fn new() -> Self {
        // SAFETY: Serialized access; nested commands cannot see this active frame.
        Self(unsafe { (&mut *POOL.get()).pop() }.unwrap_or_default())
    }
}
impl Drop for List {
    fn drop(&mut self) {
        self.0.clear();
        // SAFETY: Cleared pointers have no destructors or callbacks.
        unsafe {
            (&mut *POOL.get()).push(std::mem::take(&mut self.0));
        }
    }
}
/// `GenerateVehicleSortList` into the transaction-local list.
unsafe fn generate(s: &Services, list: &mut List, vli: &VehicleListIdentifier) -> bool {
    unsafe { generate_list(s.selection, vli, |v| list.0.push(v)) }
}
/// `IsDepotTile(tile) && IsTileOwner(tile, _current_company)`.
fn depot(s: &Services, tile: TileIndex) -> bool {
    (s.is_depot)(tile.0)
        && crate::tile_map::is_tile_owner(
            s.common.base.shared,
            tile.0,
            (s.common.base.shared.current_company)(),
        )
}
/// `Command<CMD_START_STOP_VEHICLE>::Do(flags, v->index, false)`.
unsafe fn nested_start(s: &Services, cost: *mut CommandCost, flags: DoCommandFlags, id: VehicleID) {
    let b = s.common.base;
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(cost, b.cost),
        0,
        flags.0,
        |flags| unsafe {
            super::cmd_start_stop_vehicle(
                s.common,
                cost,
                DoCommandFlags(flags),
                super::CmdStartStopVehicle {
                    veh_id: id,
                    evaluate_startstop_cb: false,
                },
            );
        },
    );
}
/// `Command<CMD_SELL_VEHICLE>::Do(flags, v->index, true, false, INVALID_CLIENT_ID)`.
unsafe fn nested_sell(s: &Services, cost: *mut CommandCost, flags: DoCommandFlags, id: VehicleID) {
    let b = s.common.base;
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(cost, b.cost),
        0,
        flags.0,
        |flags| unsafe {
            super::cmd_sell_vehicle(
                s.common,
                cost,
                DoCommandFlags(flags),
                super::CmdSellVehicle {
                    v_id: id,
                    sell_chain: true,
                    backup_order: false,
                    client_id: ClientID(s.invalid_client),
                },
            );
        },
    );
}
/// `CmdMassStartStopVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdMassStartStopVehicle {
    /// Tile of the depot where the vehicles are started/stopped (only used for depots).
    pub(crate) tile: TileIndex,
    /// Set = start vehicles, unset = stop vehicles.
    pub(crate) do_start: bool,
    /// If set, then it's a vehicle list window, not a depot and Tile is ignored in this case.
    pub(crate) vehicle_list_window: bool,
    /// The vehicle list, if `vli_valid`.
    pub(crate) vli: VehicleListIdentifier,
    /// `vli.Valid()`.
    pub(crate) vli_valid: bool,
}
/// `CmdMassStartStopVehicle`: starts or stops a lot of vehicles.
///
/// # Safety
/// Live immutable services and distinct caller-owned stack costs. Selection is
/// frozen before commands; no list or owner borrow survives command reentry.
pub(crate) unsafe fn cmd_mass_start_stop_vehicle(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdMassStartStopVehicle,
) {
    let b = s.common.base;
    let mut list = List::new();
    if !cmd.vli_valid || !unsafe { (b.groups.buildable_type)(cmd.vli.vtype.0) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    if cmd.vehicle_list_window {
        if !unsafe { generate(s, &mut list, &cmd.vli) } {
            (b.cost.error)(cost, ERROR);
            return;
        }
    } else {
        if !depot(s, cmd.tile) {
            (b.cost.error)(cost, ERROR);
            return;
        }
        /* Get the list of vehicles in the depot */
        unsafe {
            depot_list(
                s.selection,
                cmd.vli.vtype,
                cmd.tile,
                false,
                false,
                |v, _| {
                    list.0.push(v);
                },
            );
        }
    }
    for i in 0..list.0.len() {
        let v = list.0[i];
        if (crate::fleet::transactions::record(s.common.transactions, v).status()
            & crate::vehicle_base::VS_STOPPED
            != 0)
            != cmd.do_start
        {
            continue;
        }
        if !cmd.vehicle_list_window && !unsafe { (s.common.transactions.chain_depot)(v) } {
            continue;
        }
        /* Just try and don't care if some vehicle's can't be stopped. */
        unsafe {
            nested_start(s, item, flags, VehicleID((b.groups.vehicle_id)(v)));
        }
    }
    (b.cost.zero)(cost);
}
/// # Safety
/// As `cmd_mass_start_stop_vehicle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_mass_start(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: u16,
    tile: u32,
    start: bool,
    window: bool,
    id: &ListId,
) {
    unsafe {
        cmd_mass_start_stop_vehicle(
            s,
            cost,
            item,
            DoCommandFlags(flags),
            CmdMassStartStopVehicle {
                tile: TileIndex(tile),
                do_start: start,
                vehicle_list_window: window,
                vli: id.identifier(),
                vli_valid: id.valid,
            },
        );
    }
}
/// `CmdDepotSellAllVehicles` and `CmdDepotMassAutoReplace`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdDepotVehicles {
    /// Tile of the depot where the vehicles are.
    pub(crate) tile: TileIndex,
    /// Type of vehicle.
    pub(crate) vehicle_type: VehicleType,
}
/// `CmdDepotSellAllVehicles`: sells all vehicles in a depot.
///
/// # Safety
/// Native stack costs and selection services remain live through deletion.
pub(crate) unsafe fn cmd_depot_sell_all_vehicles(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    last_error: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdDepotVehicles,
) {
    let b = s.common.base;
    let mut list = List::new();
    (b.cost.expense_money)(cost, s.common.strings.new_vehicle_expense, 0);
    if !unsafe { (b.groups.buildable_type)(cmd.vehicle_type.0) } || !depot(s, cmd.tile) {
        (b.cost.error)(cost, ERROR);
        return;
    }
    /* Get the list of vehicles in the depot */
    unsafe {
        depot_list(
            s.selection,
            cmd.vehicle_type,
            cmd.tile,
            true,
            false,
            |v, _| {
                list.0.push(v);
            },
        );
    }
    (b.cost.error)(last_error, ERROR);
    let mut had_success = false;
    for i in 0..list.0.len() {
        let v = list.0[i];
        unsafe {
            nested_sell(s, item, flags, VehicleID((b.groups.vehicle_id)(v)));
        }
        if (b.cost.success)(item) {
            (b.cost.amount)(cost, (b.cost.money)(item));
            had_success = true;
        } else {
            (b.cost.move_cost)(last_error, item);
        }
    }
    if !had_success {
        (b.cost.move_cost)(cost, last_error);
    }
}
/// # Safety
/// As `cmd_depot_sell_all_vehicles`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_sell(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    last_error: *mut CommandCost,
    flags: u16,
    tile: u32,
    kind: u8,
) {
    unsafe {
        cmd_depot_sell_all_vehicles(
            s,
            cost,
            item,
            last_error,
            DoCommandFlags(flags),
            CmdDepotVehicles {
                tile: TileIndex(tile),
                vehicle_type: VehicleType(kind),
            },
        );
    }
}
/// `CmdDepotMassAutoReplace`: autoreplace all vehicles in the depot.
///
/// # Safety
/// Fleet cost slots/seeds are caller-stack storage, distinct from the aggregate.
pub(crate) unsafe fn cmd_depot_mass_auto_replace(
    s: &Services,
    cost: *mut CommandCost,
    costs: &crate::fleet::transactions::Costs,
    flags: DoCommandFlags,
    cmd: CmdDepotVehicles,
) {
    let b = s.common.base;
    let mut list = List::new();
    (b.cost.expense_money)(cost, s.common.strings.new_vehicle_expense, 0);
    if !unsafe { (b.groups.buildable_type)(cmd.vehicle_type.0) } || !depot(s, cmd.tile) {
        (b.cost.error)(cost, ERROR);
        return;
    }
    /* Get the list of vehicles in the depot */
    unsafe {
        depot_list(
            s.selection,
            cmd.vehicle_type,
            cmd.tile,
            true,
            true,
            |v, _| {
                list.0.push(v);
            },
        );
    }
    for i in 0..list.0.len() {
        let v = list.0[i];
        /* Ensure that the vehicle completely in the depot */
        if !unsafe { (s.common.transactions.chain_depot)(v) } {
            continue;
        }
        let id = unsafe { (b.groups.vehicle_id)(v) };
        crate::command::nested(
            b.commands,
            b.shared,
            Cost(costs.result.cast(), b.cost),
            0,
            flags.0,
            |flags| unsafe {
                crate::fleet::transactions::openttd_rust_fleet_autoreplace(
                    s.common.transactions,
                    b.groups,
                    costs,
                    u32::from(flags),
                    id,
                );
            },
        );
        if (b.cost.success)(costs.result.cast()) {
            (b.cost.amount)(cost, (b.cost.money)(costs.result.cast()));
        }
    }
}
/// # Safety
/// As `cmd_depot_mass_auto_replace`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_replace(
    s: &Services,
    cost: *mut CommandCost,
    costs: &crate::fleet::transactions::Costs,
    flags: u16,
    tile: u32,
    kind: u8,
) {
    unsafe {
        cmd_depot_mass_auto_replace(
            s,
            cost,
            costs,
            DoCommandFlags(flags),
            CmdDepotVehicles {
                tile: TileIndex(tile),
                vehicle_type: VehicleType(kind),
            },
        );
    }
}
/// `CmdSendVehicleToDepot` for one vehicle: `v->SendToDepot(flags, depot_cmd)`
/// after `Vehicle::GetIfValid` and `IsPrimaryVehicle`.
unsafe fn send_one(
    s: &Services,
    cost: *mut CommandCost,
    flags: DoCommandFlags,
    veh_id: VehicleID,
    depot_cmd: DepotCommandFlags,
) {
    let b = s.common.base;
    let v = unsafe { (b.groups.vehicle)(veh_id.0) };
    if v.is_null() || !unsafe { (b.groups.primary)(v) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    (s.send)(cost, v, flags.0, depot_cmd.0);
}
/// `SendAllVehiclesToDepot`: send all vehicles of type to depots.
unsafe fn send_all_vehicles_to_depot(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: DoCommandFlags,
    service: bool,
    vli: &VehicleListIdentifier,
) {
    let b = s.common.base;
    let mut list = List::new();
    if !unsafe { generate(s, &mut list, vli) } {
        (b.cost.error)(cost, ERROR);
        return;
    }
    /* Send all the vehicles to a depot */
    let mut had_success = false;
    let depot_cmd = DepotCommandFlags(if service { s.service } else { 0 } | s.dont_cancel);
    for i in 0..list.0.len() {
        let id = VehicleID(unsafe { (b.groups.vehicle_id)(list.0[i]) });
        crate::command::nested(
            b.commands,
            b.shared,
            Cost(item, b.cost),
            0,
            flags.0,
            |flags| unsafe {
                send_one(s, item, DoCommandFlags(flags), id, depot_cmd);
            },
        );
        if (b.cost.success)(item) {
            had_success = true;
            /* Return 0 if DoCommandFlag::Execute is not set this is a valid goto depot command)
             * In this case we know that at least one vehicle can be sent to a depot
             * and we will issue the command. We can now safely quit the loop, knowing
             * it will succeed at least once. With DoCommandFlag::Execute we really need to send them to the depot */
            if !flags.test(DoCommandFlag::Execute) {
                break;
            }
        }
    }
    if had_success {
        (b.cost.zero)(cost);
    } else {
        (b.cost.error)(cost, ERROR);
    }
}
/// `CmdSendVehicleToDepot`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSendVehicleToDepot {
    /// Vehicle ID to send to the depot.
    pub(crate) veh_id: VehicleID,
    /// The depot command flags.
    pub(crate) depot_cmd: DepotCommandFlags,
    /// The vehicle list, if `vli_valid`, for a mass send.
    pub(crate) vli: VehicleListIdentifier,
    /// `vli.Valid()`.
    pub(crate) vli_valid: bool,
}
/// `CmdSendVehicleToDepot`: send a vehicle to the depot.
///
/// # Safety
/// The native virtual `SendToDepot` callee retains its original owner. All Rust
/// selection borrows end before calling it or the canonical nested protocol.
pub(crate) unsafe fn cmd_send_vehicle_to_depot(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdSendVehicleToDepot,
) {
    if cmd.depot_cmd.0 & s.mass_send != 0 {
        /* Mass goto depot requested */
        if !cmd.vli_valid {
            (s.common.base.cost.error)(cost, ERROR);
            return;
        }
        unsafe {
            send_all_vehicles_to_depot(
                s,
                cost,
                item,
                flags,
                cmd.depot_cmd.0 & s.service != 0,
                &cmd.vli,
            );
        }
        return;
    }
    unsafe {
        send_one(s, cost, flags, cmd.veh_id, cmd.depot_cmd);
    }
}
/// # Safety
/// As `cmd_send_vehicle_to_depot`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_send(
    s: &Services,
    cost: *mut CommandCost,
    item: *mut CommandCost,
    flags: u16,
    id: u32,
    command: u8,
    list_id: &ListId,
) {
    unsafe {
        cmd_send_vehicle_to_depot(
            s,
            cost,
            item,
            DoCommandFlags(flags),
            CmdSendVehicleToDepot {
                veh_id: VehicleID(id),
                depot_cmd: DepotCommandFlags(command),
                vli: list_id.identifier(),
                vli_valid: list_id.valid,
            },
        );
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleListId" ["probe"] => ListId { kind, vehicle_type, company, index, valid, };
    "OpenTTDVehicleMassServices" ["probe"] => crate::vehicle_transactions::mass::Services { common, selection, is_depot, send, service, mass_send, dont_cancel, invalid_client, };
}

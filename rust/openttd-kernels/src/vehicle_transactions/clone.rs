/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole clone assembly, including partial construction and order/cash rollback.
use super::{ERROR, build, refit, vehicle_ref};
use crate::cargo_type::CargoType;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::engine_type::EngineID;
use crate::network_type::ClientID;
use crate::services::{ByteSpan, CommandCost, Cost};
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleType;
use crate::vehicle_type::VehicleID;
use std::ffi::c_long;
use std::ptr;
#[repr(C)]
pub struct Services {
    pub build: &'static build::Services,
    pub next_vehicle: extern "C" fn(*mut ()) -> *mut (),
    pub has_articulated: extern "C" fn(*mut ()) -> bool,
    pub next_articulated: extern "C" fn(*mut ()) -> *mut (),
    pub copy_service: extern "C" fn(*mut (), *mut ()),
    pub parse_suffix: extern "C" fn(ByteSpan) -> c_long,
}
/// `Command<CMD_SELL_VEHICLE>::Do(flags, v->index, true, false, INVALID_CLIENT_ID)`.
unsafe fn sell(s: &Services, out: *mut CommandCost, flags: DoCommandFlags, v: *mut ()) {
    let c = s.build.refit.common;
    let b = c.base;
    let id = unsafe { (b.groups.vehicle_id)(v) };
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(out, b.cost),
        0,
        flags.0,
        |flags| unsafe {
            super::cmd_sell_vehicle(
                c,
                out,
                DoCommandFlags(flags),
                super::CmdSellVehicle {
                    v_id: VehicleID(id),
                    sell_chain: true,
                    backup_order: false,
                    client_id: ClientID(c.strings.invalid_client),
                },
            );
        },
    );
}
/// `Command<CMD_ADD_VEHICLE_GROUP>::Do(flags, old->group_id, new->index, false, {})`.
unsafe fn copy_group(
    s: &Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    old: *mut (),
    new: *mut (),
) {
    let b = s.build.refit.common.base;
    let group = unsafe { (b.groups.vehicle_group)(old) };
    let id = unsafe { (b.groups.vehicle_id)(new) };
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(out, b.cost),
        0,
        flags.0,
        |flags| unsafe {
            let mut ignored = 0;
            let error = crate::fleet::openttd_rust_fleet_add_vehicle_group(
                b.groups,
                u32::from(flags),
                group,
                id,
                false,
                ptr::null_mut(),
                false,
                &raw mut ignored,
            );
            if error == u32::MAX {
                (b.cost.zero)(out);
            } else {
                (b.cost.error)(out, error);
            }
        },
    );
}
/// `Command<CMD_CLONE_ORDER>::Do(flags, share_orders ? CO_SHARE : CO_COPY, new->index, old->index)`.
unsafe fn orders(
    s: &Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    share: bool,
    old: *mut (),
    new: *mut (),
) {
    let c = s.build.refit.common;
    let b = c.base;
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(out, b.cost),
        0,
        flags.0,
        |flags| unsafe {
            crate::orders::openttd_rust_command_clone(
                new.cast(),
                old.cast(),
                if share {
                    c.strings.share_orders
                } else {
                    c.strings.copy_orders
                },
                u8::from(flags & DoCommandFlag::EXECUTE != 0),
                out.cast(),
                c.orders,
            );
        },
    );
}
/// `CmdCloneVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdCloneVehicle {
    /// Tile of the depot where the cloned vehicle is build.
    pub(crate) tile: TileIndex,
    /// The original vehicle's index.
    pub(crate) veh_id: VehicleID,
    /// Shared orders, else copied orders.
    pub(crate) share_orders: bool,
}
/// `CmdCloneVehicle`: clone a vehicle. If it is a train, it will clone all the
/// cars too. Returns the new vehicle ID; the cost or error is in `out`.
///
/// `original` is `v_front`, the front engine in the original vehicle; `v` is
/// the car/vehicle of the original vehicle that is currently being copied;
/// `front` is `w_front`, the front engine of the cloned vehicle; `w` is the
/// car/vehicle currently being cloned; `rear` is `w_rear`, the rear end of the
/// cloned train. It's used to add more cars and is only used by trains.
///
/// # Safety
/// Four distinct stack-constructed native costs and immutable tables remain live.
/// Resolved shells follow original clone lifetimes. No borrow survives callbacks,
/// native rail commands, nested transactions, order attachment or deletion.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
unsafe fn cmd_clone_vehicle(
    s: &Services,
    out: *mut CommandCost,
    part: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdCloneVehicle,
) -> u32 {
    let CmdCloneVehicle {
        tile,
        veh_id,
        share_orders: share,
    } = cmd;
    let r = s.build.refit;
    let c = r.common;
    let b = c.base;
    let total = Cost(out, b.cost);
    let invalid = crate::fleet::INVALID_VEHICLE;
    (b.cost.expense_money)(out, c.strings.new_vehicle_expense, 0);
    let mut v = unsafe { (b.groups.vehicle)(veh_id.0) };
    if v.is_null() || !unsafe { (b.groups.primary)(v) } {
        total.error(ERROR);
        return invalid;
    }
    let original = v;
    unsafe {
        b.commands
            .ownership(temporary, crate::fleet::record(b.groups, v).owner(), 0);
    };
    if !(b.cost.success)(temporary) {
        total.take(Cost(temporary, b.cost));
        return invalid;
    }
    /* Crashed trains can only be cloned before cleanup begins. */
    if crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN.0
        && (!crate::fleet::record(b.groups, v).is_front_engine()
            || r.train_handle(unsafe { super::vehicle_ref(v, c.vehicles) }.0.as_ptr())
                .get_crash_anim_pos()
                >= 4400)
    {
        total.error(c.strings.destroyed);
        return invalid;
    }
    /* check that we can allocate enough vehicles */
    if !flags.test(DoCommandFlag::Execute) {
        let mut count = 0_i32;
        loop {
            count += 1;
            v = (unsafe { vehicle_ref(v, c.vehicles) })
                .next()
                .map_or(ptr::null_mut(), |v| v.shell().cast());
            if v.is_null() {
                break;
            }
        }
        if !(s.build.refit.common.vehicles.can_allocate)(count as usize) {
            total.error(s.build.too_many);
            return invalid;
        }
    }
    v = original;
    let mut w: *mut ();
    let mut front: *mut () = ptr::null_mut();
    let mut rear: *mut () = ptr::null_mut();
    let mut new_id = invalid;
    loop {
        /* we build the rear ends of multiheaded trains with the front ones */
        if crate::fleet::record(b.groups, v).vehicle_type() != VehicleType::TRAIN.0
            || !crate::ground_vehicle::is_rear_dualheaded(
                crate::fleet::transactions::record(c.transactions, v).subtype(),
            )
        {
            /* In case we're building a multi headed vehicle and the maximum number of
             * vehicles is almost reached (e.g. max trains - 1) not all vehicles would
             * be cloned. When the non-primary engines were build they were seen as
             * 'new' vehicles whereas they would immediately be joined with a primary
             * engine. This caused the vehicle to be not build as 'the limit' had been
             * reached, resulting in partially build vehicles and such. */
            let build_flags =
                if flags.test(DoCommandFlag::Execute) && !unsafe { (b.groups.primary)(v) } {
                    flags.with(DoCommandFlags::from_element(DoCommandFlag::AutoReplace))
                } else {
                    flags
                };
            let engine = EngineID(crate::fleet::record(b.groups, v).engine_id());
            let built = crate::command::nested(
                b.commands,
                b.shared,
                Cost(part, b.cost),
                tile.0,
                build_flags.0,
                |build_flags| unsafe {
                    build::cmd_build_vehicle(
                        s.build,
                        part,
                        temporary,
                        refit_temporary,
                        DoCommandFlags(build_flags),
                        build::CmdBuildVehicle {
                            tile,
                            eid: engine,
                            use_free_vehicles: false,
                            cargo: CargoType(c.strings.invalid_cargo),
                            client_id: ClientID(c.strings.invalid_client),
                        },
                    )
                },
            );
            new_id = built.id;
            if !(b.cost.success)(part) {
                /* Can't build a part, then sell the stuff we already made; clear up the mess */
                if !front.is_null() {
                    crate::witness::hit(crate::witness::VT_CLONE_BUILD_SELL);
                    unsafe { sell(s, temporary, flags, front) };
                }
                total.take(Cost(part, b.cost));
                return invalid;
            }
            total.add_money((b.cost.money)(part));
            if flags.test(DoCommandFlag::Execute) {
                w = unsafe { (b.groups.vehicle)(new_id) };
                if crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN.0 {
                    /* Only copy the reverse state if neither old or new vehicle implements reverse-on-build probability callback. */
                    let old_train =
                        r.train_handle(unsafe { super::vehicle_ref(v, c.vehicles) }.0.as_ptr());
                    if old_train.get_flags() & (1 << crate::train::FLIPPED) != 0
                        && !unsafe {
                            crate::newgrf_vehicle::reverse_probability(
                                c.vehicles,
                                super::vehicle_ref(v, c.vehicles),
                            )
                        }
                        && !unsafe {
                            crate::newgrf_vehicle::reverse_probability(
                                c.vehicles,
                                super::vehicle_ref(w, c.vehicles),
                            )
                        }
                    {
                        r.train_handle(unsafe { super::vehicle_ref(w, c.vehicles) }.0.as_ptr())
                            .set_flag(crate::train::FLIPPED, true);
                    }
                }
                if crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN.0
                    && !crate::fleet::record(b.groups, v).is_front_engine()
                {
                    /* this s a train car
                     * add this unit to the end of the train */
                    let moved = unsafe {
                        super::rail::nested_move_rail_vehicle(
                            s.build,
                            part,
                            flags,
                            super::rail::CmdMoveRailVehicle {
                                src_veh: VehicleID((b.groups.vehicle_id)(w)),
                                dest_veh: VehicleID((b.groups.vehicle_id)(rear)),
                                move_chain: true,
                            },
                        )
                    };
                    if !moved {
                        /* The train can't be joined to make the same consist as the original.
                         * Sell what we already made (clean up) and return an error.           */
                        crate::witness::hit(crate::witness::VT_CLONE_JOIN_SELL);
                        unsafe {
                            sell(s, temporary, flags, front);
                            sell(s, temporary, flags, w);
                        }
                        total.take(Cost(part, b.cost));
                        return invalid;
                    }
                } else {
                    /* this is a front engine or not a train. */
                    front = w;
                    (s.copy_service)(v, w);
                }
                rear = w; // trains needs to know the last car in the train, so they can add more in next loop
            }
        }
        if crate::fleet::record(b.groups, v).vehicle_type() != VehicleType::TRAIN.0 {
            break;
        }
        v = (s.next_vehicle)(v);
        if v.is_null() {
            break;
        }
    }
    if flags.test(DoCommandFlag::Execute)
        && crate::fleet::record(b.groups, original).vehicle_type() == VehicleType::TRAIN.0
    {
        /* for trains this needs to be the front engine due to the callback function */
        new_id = unsafe { (b.groups.vehicle_id)(front) };
    }
    if flags.test(DoCommandFlag::Execute) {
        /* Cloned vehicles belong to the same group */
        unsafe { copy_group(s, temporary, flags, original, front) };
    }
    /* Take care of refitting. */
    w = front;
    v = original;
    /* Both building and refitting are influenced by newgrf callbacks, which
     * makes it impossible to accurately estimate the cloning costs. In
     * particular, it is possible for engines of the same type to be built with
     * different numbers of articulated parts, so when refitting we have to
     * loop over real vehicles first, and then the articulated parts of those
     * vehicles in a different loop. */
    loop {
        loop {
            if flags.test(DoCommandFlag::Execute) {
                if c.transactions.assertions {
                    c.transactions.check(!w.is_null());
                }
                /* Find out what's the best sub type */
                let cargo = unsafe { (c.transactions.cargo_type)(v) };
                let subtype = unsafe { (c.transactions.subtype)(v, w, cargo) };
                let new_cargo =
                    (r.cargo_kind)(unsafe { super::vehicle_ref(w, c.vehicles) }.0.as_ptr());
                // Subtype selection invokes NewGRF: observe the source cargo again.
                if new_cargo.cargo != unsafe { (c.transactions.cargo_type)(v) }
                    || new_cargo.subtype != subtype
                {
                    let id = unsafe { (b.groups.vehicle_id)(w) };
                    let cargo = unsafe { (c.transactions.cargo_type)(v) };
                    crate::command::nested(
                        b.commands,
                        b.shared,
                        Cost(part, b.cost),
                        0,
                        flags.0,
                        |flags| unsafe {
                            refit::cmd_refit_vehicle(
                                r,
                                part,
                                temporary,
                                DoCommandFlags(flags),
                                refit::CmdRefitVehicle {
                                    veh_id: VehicleID(id),
                                    new_cargo_type: CargoType(cargo),
                                    new_subtype: subtype,
                                    auto_refit: false,
                                    only_this: true,
                                    num_vehicles: 0,
                                },
                            )
                        },
                    );
                    if (b.cost.success)(part) {
                        total.add_money((b.cost.money)(part));
                    }
                }
                if crate::vehicle_base::is_ground_vehicle(
                    crate::fleet::record(b.groups, w).vehicle_type(),
                ) && (s.has_articulated)(w)
                {
                    w = (s.next_articulated)(w);
                } else {
                    break;
                }
            } else {
                let e = r.vehicle_engine(unsafe { super::vehicle_ref(v, c.vehicles) }.0.as_ptr());
                let initial = if unsafe {
                    crate::engine_capabilities::openttd_rust_engine_can_carry_abi(r.capabilities, e)
                        != 0
                } {
                    crate::vehicle::EngineRef::of(e).default_cargo()
                } else {
                    c.strings.invalid_cargo
                };
                let old_cargo =
                    (r.cargo_kind)(unsafe { super::vehicle_ref(v, c.vehicles) }.0.as_ptr());
                if old_cargo.cargo != initial && (c.valid_cargo)(initial) {
                    unsafe {
                        refit::get_refit_cost(
                            r,
                            part,
                            ptr::null_mut(),
                            EngineID(crate::fleet::record(b.groups, v).engine_id()),
                            CargoType(old_cargo.cargo),
                            old_cargo.subtype,
                        )
                    };
                    total.add(Cost(part, b.cost));
                }
            }
            if crate::vehicle_base::is_ground_vehicle(
                crate::fleet::record(b.groups, v).vehicle_type(),
            ) && (s.has_articulated)(v)
            {
                v = (s.next_articulated)(v);
            } else {
                break;
            }
            if v.is_null() {
                break;
            }
        }
        if flags.test(DoCommandFlag::Execute)
            && crate::fleet::record(b.groups, v).vehicle_type() == VehicleType::TRAIN.0
        {
            w = (s.next_vehicle)(w);
        }
        if crate::fleet::record(b.groups, v).vehicle_type() != VehicleType::TRAIN.0 {
            break;
        }
        v = (s.next_vehicle)(v);
        if v.is_null() {
            break;
        }
    }
    if flags.test(DoCommandFlag::Execute) {
        /*
         * Set the orders of the vehicle. Cannot do it earlier as we need
         * the vehicle refitted before doing this, otherwise the moved
         * cargo types might not match (passenger vs non-passenger)
         */
        unsafe { orders(s, part, flags, share, original, front) };
        if !(b.cost.success)(part) {
            /* The vehicle has already been bought, so now it must be sold again. */
            unsafe { sell(s, temporary, flags, front) };
            total.take(Cost(part, b.cost));
            return invalid;
        }
        /* Now clone the vehicle's name, if it has one. */
        if (b.name)(original).size != 0 {
            unsafe { super::clone_name::clone_name(c, s.parse_suffix, original, front) };
        }
        /* Since we can't estimate the cost of cloning a vehicle accurately we must
         * check whether the company has enough money manually. */
        if !unsafe { crate::command::check_money(b.commands, b.cost, out) } {
            unsafe { sell(s, temporary, flags, front) };
            return invalid;
        }
    }
    new_id
}
/// # Safety
/// As `cmd_clone_vehicle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_clone(
    s: &Services,
    out: *mut CommandCost,
    part: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: u16,
    tile: u32,
    id: u32,
    share: bool,
) -> u32 {
    unsafe {
        cmd_clone_vehicle(
            s,
            out,
            part,
            temporary,
            refit_temporary,
            DoCommandFlags(flags),
            CmdCloneVehicle {
                tile: TileIndex(tile),
                veh_id: VehicleID(id),
                share_orders: share,
            },
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleCloneServices" ["probe"] => Services {
        build, next_vehicle, has_articulated, next_articulated, copy_service, parse_suffix,
    };
}

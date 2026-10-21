/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole clone assembly, including partial construction and order/cash rollback.
use super::{AUTOREPLACE, ERROR, EXECUTE, build, refit, vehicle_ref};
use crate::services::{ByteSpan, CommandCost, Cost};
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
unsafe fn sell(s: &Services, out: *mut CommandCost, flags: u16, v: *mut ()) {
    let c = s.build.refit.common;
    let b = c.base;
    let id = unsafe { (b.groups.vehicle_id)(v) };
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(out, b.cost),
        0,
        flags,
        |flags| unsafe { super::sell(c, out, flags, id, true, false, c.strings.invalid_client) },
    );
}
unsafe fn copy_group(s: &Services, out: *mut CommandCost, flags: u16, old: *mut (), new: *mut ()) {
    let b = s.build.refit.common.base;
    let group = unsafe { (b.groups.vehicle_group)(old) };
    let id = unsafe { (b.groups.vehicle_id)(new) };
    crate::command::nested(
        b.commands,
        b.shared,
        Cost(out, b.cost),
        0,
        flags,
        |flags| unsafe {
            let mut ignored = 0;
            let error = crate::fleet::openttd_rust_fleet_add_vehicle_group(
                b.groups,
                u32::from(flags),
                group,
                id,
                false,
                ptr::null_mut(),
                true,
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
unsafe fn orders(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
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
        flags,
        |flags| unsafe {
            crate::orders::openttd_rust_command_clone(
                new.cast(),
                old.cast(),
                if share {
                    c.strings.share_orders
                } else {
                    c.strings.copy_orders
                },
                u8::from(flags & EXECUTE != 0),
                out.cast(),
                c.orders,
            );
        },
    );
}
/// # Safety
/// Four distinct stack-constructed native costs and immutable tables remain live.
/// Resolved shells follow original clone lifetimes. No borrow survives callbacks,
/// native rail commands, nested transactions, order attachment or deletion.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
unsafe fn clone(
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
    let r = s.build.refit;
    let c = r.common;
    let b = c.base;
    let total = Cost(out, b.cost);
    let invalid = crate::fleet::INVALID_VEHICLE;
    (b.cost.expense_money)(out, c.strings.new_vehicle_expense, 0);
    let mut v = unsafe { (b.groups.vehicle)(id) };
    if v.is_null() || !unsafe { (b.groups.primary)(v) } {
        total.error(ERROR);
        return invalid;
    }
    let original = v;
    unsafe { (b.commands.ownership)(temporary, (b.groups.vehicle_owner)(v)) };
    if !(b.cost.success)(temporary) {
        total.take(Cost(temporary, b.cost));
        return invalid;
    }
    if unsafe { (b.groups.vehicle_type)(v) } == 0
        && (!unsafe { (b.groups.front)(v) }
            || (r.train_handle)(v.cast()).get_crash_anim_pos() >= 4400)
    {
        total.error(c.strings.destroyed);
        return invalid;
    }
    if flags & EXECUTE == 0 {
        let mut count = 0_i32;
        loop {
            count += 1;
            v = (c.vehicles.next)(unsafe { vehicle_ref(v) })
                .map_or(ptr::null_mut(), |v| v.0.as_ptr().cast());
            if v.is_null() {
                break;
            }
        }
        if !(s.build.pool_available)(count as usize) {
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
        if unsafe { (b.groups.vehicle_type)(v) } != 0 || !unsafe { (c.transactions.rear)(v) } {
            let build_flags = if flags & EXECUTE != 0 && !unsafe { (b.groups.primary)(v) } {
                flags | AUTOREPLACE
            } else {
                flags
            };
            let engine = unsafe { (b.groups.vehicle_engine)(v) };
            let built = crate::command::nested(
                b.commands,
                b.shared,
                Cost(part, b.cost),
                tile,
                build_flags,
                |build_flags| unsafe {
                    build::build(
                        s.build,
                        part,
                        temporary,
                        refit_temporary,
                        build_flags,
                        tile,
                        engine,
                        false,
                        c.strings.invalid_cargo,
                        c.strings.invalid_client,
                    )
                },
            );
            new_id = built.id;
            if !(b.cost.success)(part) {
                if !front.is_null() {
                    unsafe { sell(s, temporary, flags, front) };
                }
                total.take(Cost(part, b.cost));
                return invalid;
            }
            total.add_money((b.cost.money)(part));
            if flags & EXECUTE != 0 {
                w = unsafe { (b.groups.vehicle)(new_id) };
                if unsafe { (b.groups.vehicle_type)(v) } == 0 {
                    let old_train = (r.train_handle)(v.cast());
                    if old_train.get_flags() & (1 << crate::train::FLIPPED) != 0
                        && !unsafe { (c.transactions.reverse_probability)(v) }
                        && !unsafe { (c.transactions.reverse_probability)(w) }
                    {
                        (r.train_handle)(w.cast()).set_flag(crate::train::FLIPPED, true);
                    }
                }
                if unsafe { (b.groups.vehicle_type)(v) } == 0 && !unsafe { (b.groups.front)(v) } {
                    unsafe {
                        (c.transactions.move_vehicle)(part.cast(), w, rear, u32::from(flags), true);
                    };
                    if !(b.cost.success)(part) {
                        unsafe {
                            sell(s, temporary, flags, front);
                            sell(s, temporary, flags, w);
                        }
                        total.take(Cost(part, b.cost));
                        return invalid;
                    }
                } else {
                    front = w;
                    (s.copy_service)(v, w);
                }
                rear = w;
            }
        }
        if unsafe { (b.groups.vehicle_type)(v) } != 0 {
            break;
        }
        v = (s.next_vehicle)(v);
        if v.is_null() {
            break;
        }
    }
    if flags & EXECUTE != 0 && unsafe { (b.groups.vehicle_type)(original) } == 0 {
        new_id = unsafe { (b.groups.vehicle_id)(front) };
    }
    if flags & EXECUTE != 0 {
        unsafe { copy_group(s, temporary, flags, original, front) };
    }
    w = front;
    v = original;
    loop {
        loop {
            if flags & EXECUTE != 0 {
                if c.transactions.assertions {
                    unsafe { (c.transactions.check)(!w.is_null()) };
                }
                let cargo = unsafe { (c.transactions.cargo_type)(v) };
                let subtype = unsafe { (c.transactions.subtype)(v, w, cargo) };
                let new_cargo = (r.cargo_kind)(w.cast());
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
                        flags,
                        |flags| unsafe {
                            refit::refit(
                                r, part, temporary, flags, id, cargo, subtype, false, true, 0,
                            )
                        },
                    );
                    if (b.cost.success)(part) {
                        total.add_money((b.cost.money)(part));
                    }
                }
                if unsafe { (b.groups.ground)(w) } && (s.has_articulated)(w) {
                    w = (s.next_articulated)(w);
                } else {
                    break;
                }
            } else {
                let e = (r.vehicle_engine)(v.cast());
                let initial = if (r.can_carry)(e) {
                    (s.build.default_cargo)(e)
                } else {
                    c.strings.invalid_cargo
                };
                let old_cargo = (r.cargo_kind)(v.cast());
                if old_cargo.cargo != initial && (c.valid_cargo)(initial) {
                    unsafe {
                        refit::get_cost(
                            r,
                            part,
                            ptr::null_mut(),
                            (b.groups.vehicle_engine)(v),
                            old_cargo.cargo,
                            old_cargo.subtype,
                        )
                    };
                    total.add(Cost(part, b.cost));
                }
            }
            if unsafe { (b.groups.ground)(v) } && (s.has_articulated)(v) {
                v = (s.next_articulated)(v);
            } else {
                break;
            }
            if v.is_null() {
                break;
            }
        }
        if flags & EXECUTE != 0 && unsafe { (b.groups.vehicle_type)(v) } == 0 {
            w = (s.next_vehicle)(w);
        }
        if unsafe { (b.groups.vehicle_type)(v) } != 0 {
            break;
        }
        v = (s.next_vehicle)(v);
        if v.is_null() {
            break;
        }
    }
    if flags & EXECUTE != 0 {
        unsafe { orders(s, part, flags, share, original, front) };
        if !(b.cost.success)(part) {
            unsafe { sell(s, temporary, flags, front) };
            total.take(Cost(part, b.cost));
            return invalid;
        }
        if (b.name)(original).size != 0 {
            unsafe { super::clone_name::clone_name(c, s.parse_suffix, original, front) };
        }
        if !unsafe { crate::command::check_money(b.commands, b.cost, out) } {
            unsafe { sell(s, temporary, flags, front) };
            return invalid;
        }
    }
    new_id
}
/// # Safety
/// Same serialized shell and stack-cost lifetime contract as `clone`.
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
        clone(
            s,
            out,
            part,
            temporary,
            refit_temporary,
            flags,
            tile,
            id,
            share,
        )
    }
}

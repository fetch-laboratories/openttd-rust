/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! Disaster-private ownership and complete controllers over canonical C++ world data.
// Source-width casts and bit masks intentionally preserve the original encodings.
// Keeping controller bodies intact makes their service ordering reviewable.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::struct_field_names,
    clippy::verbose_bit_mask
)]
use crate::random_func::chance16_i;
use crate::services::Services;
use crate::tile_type::INVALID_TILE;
use std::cell::Cell;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as PollContext, Poll, Waker};

#[repr(C)]
pub struct State {
    pub image_override: u32,
    pub target: u32,
    pub state: u16,
    pub flags: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Action {
    pub kind: u32,
    pub id: u32,
    pub other: u32,
    pub a: i64,
    pub b: i64,
    pub c: i64,
    pub d: i64,
}
static mut DELAY: u16 = 0;
struct Channel {
    action: Cell<Action>,
    response: Cell<i64>,
    vehicle: Cell<Option<(u32, [i64; 24])>>,
    world: Cell<Option<[i64; 24]>>,
}
struct Request {
    channel: Rc<Channel>,
    action: Option<Action>,
}
impl Future for Request {
    type Output = i64;
    fn poll(mut self: Pin<&mut Self>, _: &mut PollContext<'_>) -> Poll<i64> {
        if let Some(action) = self.action.take() {
            self.channel.action.set(action);
            Poll::Pending
        } else {
            Poll::Ready(self.channel.response.get())
        }
    }
}
#[derive(Clone)]
struct World<'a> {
    context: *mut c_void,
    read: unsafe extern "C" fn(*mut c_void, u32, u32, i64, i64, *mut i64),
    write: unsafe extern "C" fn(*mut c_void, u32, u32, i64),
    channel: Rc<Channel>,
    services: &'a Services,
    occupancy: &'a Occupancy,
    service: unsafe extern "C" fn(*mut c_void, *const Action) -> i64,
}
/// Borrowed immutable tables and shell resolver, live through every World clone
/// and Run drop/yield. The context remains independently mutable; no owner data
/// is borrowed by this table. Calls use the game-state mutex like native callers.
#[repr(C)]
pub struct Occupancy {
    pub commands: *const crate::vehicle::occupancy::Commands,
    pub vehicle: unsafe extern "C" fn(*mut c_void, u32) -> crate::vehicle::VehicleRef,
}
// The fixed copied-record slots below are shared with disaster_adapter.hpp.
const SUBTYPE: usize = 0;
const X: usize = 1;
const Y: usize = 2;
const Z: usize = 3;
const TILE: usize = 4;
const DEST: usize = 5;
const DIR: usize = 6;
const AGE: usize = 7;
const TICK: usize = 8;
const OWNER: usize = 9;
const STATUS: usize = 10;
const NEXT: usize = 11;
const SPRITE: usize = 12;
const FRONT: usize = 13;
const GROUND: usize = 14;
const BREAKDOWN: usize = 16;
const BREAK_DELAY: usize = 17;
const CRASHED: usize = 18;
const BACKLINK: usize = 19;
const INVALID: u32 = 1_048_575;
impl World<'_> {
    fn read(&self, kind: u32, id: u32, a: i64, b: i64) -> [i64; 24] {
        // Reuse observations between canonical scalar writes. Direct services
        // and returned actions discard these copies before the next observation.
        if kind == 0 && a == 0 {
            if let Some(record) = self.channel.world.get() {
                return record;
            }
        } else if kind == 1
            && let Some((cached, record)) = self.channel.vehicle.get()
            && cached == id
        {
            return record;
        }
        let mut record = [0; 24];
        // SAFETY: Synchronous nonthrowing callback writes 24 copied scalars only.
        unsafe {
            (self.read)(self.context, kind, id, a, b, record.as_mut_ptr());
        }
        if kind == 0 && a == 0 {
            self.channel.world.set(Some(record));
        } else if kind == 1 {
            self.channel.vehicle.set(Some((id, record)));
        }
        record
    }
    fn world(&self) -> [i64; 24] {
        self.read(0, 0, 0, 0)
    }
    fn vehicle(&self, id: u32) -> [i64; 24] {
        self.read(1, id, 0, 0)
    }
    fn v(&self, id: u32, field: usize) -> i64 {
        self.vehicle(id)[field]
    }
    fn set(&self, id: u32, field: usize, value: i64) {
        // SAFETY: Source-valid ID; callback performs one canonical scalar write.
        unsafe {
            (self.write)(self.context, field as u32, id, value);
        }
        if let Some((cached, mut record)) = self.channel.vehicle.get()
            && cached == id
        {
            if field == OWNER || field == SUBTYPE {
                self.channel.vehicle.set(None);
            } else {
                record[field] = match field {
                    X | Y | Z | AGE => i64::from(value as i32),
                    DIR | TICK | STATUS | BREAKDOWN | BREAK_DELAY => i64::from(value as u8),
                    _ => i64::from(value as u32),
                };
                self.channel.vehicle.set(Some((id, record)));
            }
        }
    }
    fn increment(&self, id: u32, field: usize) -> i64 {
        let old = self.v(id, field);
        let new = if field == TICK {
            i64::from((old as u8).wrapping_add(1))
        } else {
            i64::from((old as i32).wrapping_add(1))
        };
        self.set(id, field, new);
        new
    }
    fn private(&self, id: u32) -> *mut State {
        self.read(7, id, 0, 0)[0] as usize as *mut State
    }
    fn state(&self, id: u32) -> u16 {
        // SAFETY: Rust-owned stable allocation of the live shell; scalar read only.
        unsafe { (*self.private(id)).state }
    }
    fn set_state(&self, id: u32, value: u16) {
        // SAFETY: No reference retained; game-thread-only scalar mutation.
        unsafe {
            (*self.private(id)).state = value;
        }
    }
    fn image(&self, id: u32) -> u32 {
        // SAFETY: Same stable live state contract.
        unsafe { (*self.private(id)).image_override }
    }
    fn set_image(&self, id: u32, value: u32) {
        // SAFETY: No owner/world reference survives this scalar access.
        unsafe {
            (*self.private(id)).image_override = value;
        }
    }
    fn target(&self, id: u32) -> u32 {
        // SAFETY: Same stable live state contract.
        unsafe { (*self.private(id)).target }
    }
    fn tile(&self, tile: u32) -> [i64; 24] {
        self.read(5, tile, 0, 0)
    }
    fn next(&self, kind: u32, id: u32) -> u32 {
        self.read(6, kind, i64::from(id), 0)[0] as u32
    }
    fn slope(&self, x: i64, y: i64) -> i64 {
        i64::from(crate::movement::pixel_z(
            self.services,
            x as i32,
            y as i32,
            false,
        ))
    }
    fn toward(&self, id: u32, x: i64, y: i64) -> i64 {
        self.read(9, id, x, y)[0]
    }
    fn position(&self, id: u32) -> [i64; 24] {
        self.read(10, id, 0, 0)
    }
    fn tile_xy(&self, x: u32, y: u32) -> u32 {
        crate::map_func::Map::from_log_x((self.world()[0] as u32).trailing_zeros()).tile_xy(x, y)
    }
    fn tile_virt(&self, x: i64, y: i64) -> u32 {
        self.tile_xy((x as i32 as u32) >> 4, (y as i32 as u32) >> 4)
    }
    fn tile_x(&self, tile: u32) -> i64 {
        i64::from(
            crate::map_func::Map::from_log_x((self.world()[0] as u32).trailing_zeros())
                .tile_x(tile),
        )
    }
    fn tile_y(&self, tile: u32) -> i64 {
        i64::from(
            crate::map_func::Map::from_log_x((self.world()[0] as u32).trailing_zeros())
                .tile_y(tile),
        )
    }
    async fn action(&self, kind: u32, id: u32, other: u32, a: i64, b: i64, c: i64, d: i64) -> i64 {
        Request {
            channel: self.channel.clone(),
            action: Some(Action {
                kind,
                id,
                other,
                a,
                b,
                c,
                d,
            }),
        }
        .await
    }
    fn service(&self, kind: u32, id: u32, other: u32, a: i64, b: i64, c: i64, d: i64) -> i64 {
        // No world/private owner reference survives the synchronous wrapper. The
        // shell constructor can invoke a separate Rust owner; it cannot delete
        // an existing vehicle. Refresh observations after canonical services.
        let action = Action {
            kind,
            id,
            other,
            a,
            b,
            c,
            d,
        };
        let result = unsafe { (self.service)(self.context, &raw const action) };
        self.channel.vehicle.set(None);
        self.channel.world.set(None);
        result
    }
    fn random(&self) -> u32 {
        self.services.random()
    }
    fn random_tile(&self) -> u32 {
        self.random() & self.world()[4] as u32
    }
    fn range(&self, n: u32) -> u32 {
        self.services.random_range(n)
    }
    fn chance(&self, b: u32) -> bool {
        chance16_i(1, b, self.random())
    }
    async fn delete(&self, id: u32) -> i64 {
        self.action(2, id, 0, 0, 0, 0, 0).await;
        0
    }
    fn flight(&self, id: u32) -> i64 {
        self.service(5, id, 0, 0, 0, 0, 0)
    }
    fn bounds(&self, id: u32) -> i64 {
        self.service(6, id, 0, 0, 0, 0, 0)
    }
    fn viewport(&self, id: u32) {
        // Resolve the same shell once at the original service observation point.
        // Index accesses end before the exact base viewport leaf can reenter.
        unsafe {
            let v = (self.occupancy.vehicle)(self.context, id);
            let s = (&*self.occupancy.commands).vehicles;
            if (s.vehicle_type)(v) != crate::vehicle::VehicleType::EFFECT.0 {
                crate::vehicle::occupancy::update(s, v, false);
            }
            (s.base_viewport)(v);
        }
        self.channel.vehicle.set(None);
        self.channel.world.set(None);
    }
    fn effect_rel(&self, id: u32, x: i64, y: i64, z: i64, kind: u32) {
        self.service(7, id, kind, x, y, z, 0);
    }
    fn effect_above(&self, x: i64, y: i64, z: i64, kind: u32) {
        self.service(8, 0, kind, x, y, z, 0);
    }
    fn sound_vehicle(&self, id: u32) {
        if self.world()[7] != 0 {
            self.service(9, id, 0, 0, 0, 0, 0);
        }
    }
    fn sound_tile(&self, tile: u32) {
        if self.world()[7] != 0 {
            self.service(10, tile, 0, 0, 0, 0, 0);
        }
    }
    fn news(&self, kind: u32, id: u32, a: i64) {
        self.service(11, id, kind, a, 0, 0, 0);
    }
    fn update_image(&self, id: u32) {
        let image = self.image(id);
        let image = if image != 0 {
            image
        } else {
            let dir = self.v(id, DIR) as u32;
            match self.v(id, SUBTYPE) {
                0 | 1 => 3905,
                2 | 3 => 3908,
                4 | 5 => 3918,
                6 | 7 => 3922,
                8 => 3902,
                9 | 10 => 3920,
                11 | 12 => 3921,
                13 => 3910 + dir / 2,
                _ => 3914 + dir / 2,
            }
        };
        self.set(id, SPRITE, i64::from(image));
    }
    fn update_position(&self, id: u32, x: i64, y: i64, z: i64) {
        self.set(id, X, x);
        self.set(id, Y, y);
        self.set(id, Z, z);
        self.set(id, TILE, i64::from(self.tile_virt(x, y)));
        self.update_image(id);
        self.viewport(id);
        let shadow = self.v(id, NEXT) as u32;
        if shadow != INVALID {
            let w = self.world();
            let safe_x = x.clamp(0, w[2] * 16);
            let mut safe_y = i64::from((y as i32).wrapping_sub(1)).clamp(0, w[3] * 16);
            self.set(shadow, X, x);
            self.set(
                shadow,
                Y,
                y - 1
                    - (i64::from((z as i32).wrapping_sub(self.slope(safe_x, safe_y) as i32))
                        .max(0)
                        >> 3),
            );
            safe_y = self.v(shadow, Y).clamp(0, w[3] * 16);
            self.set(shadow, Z, self.slope(safe_x, safe_y));
            self.set(shadow, DIR, self.v(id, DIR));
            self.update_image(shadow);
            self.viewport(shadow);
            let rotor = self.v(shadow, NEXT) as u32;
            if rotor != INVALID {
                self.set(rotor, X, x);
                self.set(rotor, Y, y);
                self.set(rotor, Z, z + 5);
                self.viewport(rotor);
            }
        }
    }
    fn move_flight(&self, id: u32) {
        let gp = self.position(id);
        let z = self.flight(id);
        self.update_position(id, gp[0], gp[1], z);
    }
    fn create_vehicle(&self, x: i64, y: i64, dir: i64, subtype: u32, target: u32) -> u32 {
        self.service(15, subtype, target, x, y, dir, 0) as u32
    }
    fn link(&self, id: u32, next: u32) {
        self.set(id, NEXT, i64::from(next));
    }

    async fn clear(&self, tile: u32) {
        // Keep full native result construction, error observation and destruction;
        // the occupancy traversal itself never returns through a native facade.
        let failed = unsafe {
            let c = &*self.occupancy.commands;
            crate::command::with_cost(c.costs, |cost| {
                crate::vehicle::occupancy::openttd_rust_occupancy_ground(c, cost.0, tile);
                !cost.ok()
            })
        };
        self.channel.vehicle.set(None);
        self.channel.world.set(None);
        if failed {
            return;
        }
        let t = self.tile(tile);
        match t[1] {
            1 if self.read(15, t[2] as u32, 0, 0)[0] != 0 && t[4] == 0 => {
                self.action(17, tile, 0, 0, 0, 0, 0).await;
                self.service(18, 0, 0, 0, 0, 0, 0);
            }
            3 => {
                self.action(17, tile, 1, 0, 0, 0, 0).await;
            }
            0 | 4 => {
                crate::ground::clear_square(tile, self.services);
                self.channel.vehicle.set(None);
                self.channel.world.set(None);
            }
            _ => {}
        }
    }
    fn destruct_industry(&self, id: u32) {
        for tile in 0..=self.world()[4] as u32 {
            if self.read(11, id, i64::from(tile), 0)[0] != 0 {
                self.service(20, tile, 0, 0, 0, 0, 0);
            }
        }
    }
}
fn delta(a: i64, b: i64) -> i64 {
    i64::from(if a < b {
        (b as i32).wrapping_sub(a as i32)
    } else {
        (a as i32).wrapping_sub(b as i32)
    })
}
fn distance(ax: i64, ay: i64, bx: i64, by: i64) -> i64 {
    i64::from((delta(ax, bx) as i32).wrapping_add(delta(ay, by) as i32))
}

async fn zeppelin(w: World<'_>, id: u32) -> i64 {
    let tick = w.increment(id, TICK);
    if w.state(id) < 2 {
        if tick & 1 != 0 {
            return 1;
        }
        w.move_flight(id);
        if w.state(id) == 1 {
            if w.increment(id, AGE) == 38 {
                w.set_state(id, 2);
                w.set(id, AGE, 0);
            }
            if w.v(id, TICK) & 7 == 0 {
                w.effect_rel(id, 0, -17, 2, 4);
            }
        } else if w.state(id) == 0 {
            let t = w.tile(w.v(id, TILE) as u32);
            if t[0] != 0 && t[5] != 0 {
                w.set_state(id, 1);
                w.set(id, AGE, 0);
                w.news(0, w.v(id, TILE) as u32, t[6]);
                w.service(12, w.v(id, TILE) as u32, 0, 0, 0, 0, 0);
            }
        }
        if w.v(id, Y) >= ((w.world()[1] + 9) * 16 - 1) {
            return w.delete(id).await;
        }
        return 1;
    }
    let tile = w.v(id, TILE) as u32;
    let t = w.tile(tile);
    if t[0] != 0 && t[5] != 0 {
        w.service(13, t[6] as u32, 1, 0, 0, 0, 0);
    }
    if w.state(id) > 2 {
        if w.increment(id, AGE) <= 13320 {
            return 1;
        }
        let tile = w.v(id, TILE) as u32;
        let t = w.tile(tile);
        if t[0] != 0 && t[5] != 0 {
            w.service(13, t[6] as u32, 0, 0, 0, 0, 0);
            w.service(12, tile, 1, i64::from(t[6] as u32), 0, 0, 0);
        }
        let x = w.v(id, X);
        let y = w.v(id, Y);
        let z = w.flight(id);
        w.update_position(id, x, y, z);
        return w.delete(id).await;
    }
    let x = w.v(id, X);
    let y = w.v(id, Y);
    let mut z = w.slope(x, y);
    if z < w.v(id, Z) {
        z = w.v(id, Z) - 1;
    }
    w.update_position(id, x, y, z);
    let age = w.increment(id, AGE);
    if age == 1 {
        w.effect_rel(id, 0, 7, 8, 5);
        w.sound_vehicle(id);
        w.set_image(id, 3906);
    } else if age == 70 {
        w.set_image(id, 3907);
    } else if age <= 300 {
        if w.v(id, TICK) & 7 == 0 {
            let r = w.random();
            w.effect_rel(
                id,
                i64::from(r & 15) - 7,
                i64::from((r >> 4) & 15) - 7,
                i64::from((r >> 8) & 7) + 5,
                7,
            );
        }
    } else if age == 350 {
        w.set_state(id, 3);
        w.set(id, AGE, 0);
    }
    1
}
async fn small_ufo(w: World<'_>, id: u32) -> i64 {
    let tick = w.increment(id, TICK);
    w.set_image(id, if tick & 8 != 0 { 3909 } else { 3908 });
    if w.state(id) == 0 {
        let dest = w.v(id, DEST) as u32;
        let x = w.tile_x(dest) * 16;
        let y = w.tile_y(dest) * 16;
        if distance(x, y, w.v(id, X), w.v(id, Y)) >= 16 {
            w.set(id, DIR, w.toward(id, x, y));
            w.move_flight(id);
            return 1;
        }
        if w.increment(id, AGE) < 6 {
            let tile = w.random_tile();
            w.set(id, DEST, i64::from(tile));
            return 1;
        }
        w.set_state(id, 1);
        let mut n = 0u32;
        let mut company = w.next(0, INVALID);
        while company != INVALID {
            n = n.wrapping_add(w.read(4, company, 0, 0)[0] as u32);
            company = w.next(0, company);
        }
        if n == 0 {
            return w.delete(id).await;
        }
        n = w.range(n);
        let mut target = w.next(1, INVALID);
        while target != INVALID {
            let t = w.vehicle(target);
            if t[FRONT] != 0 {
                let old = n;
                n = n.wrapping_sub(1);
                if old == 0 {
                    if t[CRASHED] != 0 || t[BACKLINK] as u32 != INVALID {
                        return w.delete(id).await;
                    }
                    w.set(id, DEST, i64::from(target));
                    w.set(id, AGE, 0);
                    w.set(target, BACKLINK, i64::from(id));
                    break;
                }
            }
            target = w.next(1, target);
        }
        return 1;
    }
    let target = w.v(id, DEST) as u32;
    let t = w.vehicle(target);
    let dist = (distance(w.v(id, X), w.v(id, Y), t[X], t[Y])) as u32;
    if dist < 16 && t[STATUS] & 1 == 0 && t[BREAKDOWN] == 0 {
        w.set(target, BREAKDOWN, 3);
        w.set(target, BREAK_DELAY, 140);
    }
    w.set(id, DIR, w.toward(id, w.v(target, X), w.v(target, Y)));
    let gp = w.position(id);
    let mut z = w.v(id, Z);
    if dist <= 16 && z > w.v(target, Z) {
        z -= 1;
    }
    w.update_position(id, gp[0], gp[1], z);
    if z <= w.v(target, Z) {
        w.increment(id, AGE);
        if w.v(target, STATUS) & 1 == 0 && w.v(target, CRASHED) == 0 {
            let victims = w.action(14, target, 0, 0, 0, 0, 0).await;
            w.set(target, BACKLINK, i64::from(INVALID));
            w.news(1, w.v(target, TILE) as u32, 0);
            w.service(12, target, 2, victims, 0, 0, 0);
        }
    }
    if w.v(id, AGE) > 50 {
        w.effect_rel(id, 0, 7, 8, 5);
        w.sound_vehicle(id);
        return w.delete(id).await;
    }
    1
}
async fn aircraft(w: World<'_>, id: u32, leave_top: bool) -> i64 {
    let tick = w.increment(id, TICK);
    w.set_image(
        id,
        if w.state(id) == 1 && tick & 4 != 0 {
            if leave_top { 3919 } else { 3923 }
        } else {
            0
        },
    );
    let gp = w.position(id);
    let z = w.flight(id);
    w.update_position(id, gp[0], gp[1], z);
    if (leave_top && gp[0] < -160) || (!leave_top && gp[0] > (w.world()[0] + 9) * 16 - 1) {
        return w.delete(id).await;
    }
    match w.state(id) {
        2 => {
            if w.v(id, TICK) & 3 == 0 {
                let i = w.read(2, w.v(id, DEST) as u32, 0, 0);
                let x = w.tile_x(i[0] as u32) * 16;
                let y = w.tile_y(i[0] as u32) * 16;
                let r = w.random();
                w.effect_above(
                    i64::from(r & 63) + x,
                    i64::from((r >> 6) & 63) + y,
                    i64::from((r >> 12) & 15),
                    7,
                );
                if w.increment(id, AGE) >= 55 {
                    w.set_state(id, 3);
                }
            }
        }
        1 => {
            if w.increment(id, AGE) == 112 {
                w.set_state(id, 2);
                w.set(id, AGE, 0);
                let industry = w.v(id, DEST) as u32;
                w.destruct_industry(industry);
                let i = w.read(2, industry, 0, 0);
                w.news(if leave_top { 2 } else { 3 }, industry, i[5]);
                w.sound_tile(w.read(2, industry, 0, 0)[0] as u32);
            }
        }
        0 => {
            let x = w.v(id, X) + if leave_top { -240 } else { 240 };
            let y = w.v(id, Y);
            if (x as u32) > w.world()[2] as u32 * 16 - 1 {
                return 1;
            }
            let tile = w.tile_virt(x, y);
            let t = w.tile(tile);
            if t[1] != 8 {
                return 1;
            }
            let industry = t[7] as u32;
            w.set(id, DEST, i64::from(industry));
            if w.read(2, industry, 0, 0)[if leave_top { 2 } else { 3 }] != 0 {
                w.set_state(id, 1);
                w.set(id, AGE, 0);
            }
        }
        _ => {}
    }
    1
}
fn rotors(w: &World<'_>, id: u32) -> i64 {
    if w.increment(id, TICK) & 1 != 0 {
        return 1;
    }
    let image = (w.v(id, SPRITE) as u32).wrapping_add(1);
    w.set(
        id,
        SPRITE,
        i64::from(if image > 3904 { 3902 } else { image }),
    );
    w.viewport(id);
    1
}
fn valid_train(w: &World<'_>, id: u32) -> bool {
    let t = w.vehicle(id);
    t[FRONT] != 0
        && w.read(15, t[OWNER] as u32, 0, 0)[0] != 0
        && w.tile(t[TILE] as u32)[8] != 0
        && t[STATUS] & 128 == 0
}
async fn big_ufo(w: World<'_>, id: u32) -> i64 {
    w.increment(id, TICK);
    if w.state(id) == 1 {
        let dest = w.v(id, DEST) as u32;
        let x = w.tile_x(dest) * 16 + 8;
        let y = w.tile_y(dest) * 16 + 8;
        if distance(w.v(id, X), w.v(id, Y), x, y) >= 8 {
            w.set(id, DIR, w.toward(id, x, y));
            w.move_flight(id);
            return 1;
        }
        if w.tile(w.v(id, DEST) as u32)[0] == 0 {
            return w.delete(id).await;
        }
        let z = w.slope(w.v(id, X), w.v(id, Y));
        if z < w.v(id, Z) {
            w.update_position(id, w.v(id, X), w.v(id, Y), w.v(id, Z) - 1);
            return 1;
        }
        w.set_state(id, 2);
        let mut target = w.next(2, INVALID);
        while target != INVALID {
            let t = w.vehicle(target);
            if t[GROUND] != 0 && distance(t[X], t[Y], w.v(id, X), w.v(id, Y)) <= 192 {
                w.set(target, BREAKDOWN, 5);
                w.set(target, BREAK_DELAY, 240);
            }
            target = w.next(2, target);
        }
        let town = w.service(21, w.v(id, DEST) as u32, 0, 0, 0, 0, 0);
        w.news(4, w.v(id, TILE) as u32, town);
        if w.read(0, 0, 2, 0)[9] == 0 {
            return w.delete(id).await;
        }
        let y = w.v(id, Y);
        let destroyer = w.create_vehicle(-96, y, 5, 11, id);
        let shadow = w.create_vehicle(-96, w.v(id, Y), 5, 12, INVALID);
        w.link(destroyer, shadow);
    } else if w.state(id) == 0 {
        let dest = w.v(id, DEST) as u32;
        let x = w.tile_x(dest) * 16;
        let y = w.tile_y(dest) * 16;
        if distance(x, y, w.v(id, X), w.v(id, Y)) >= 16 {
            w.set(id, DIR, w.toward(id, x, y));
            w.move_flight(id);
            return 1;
        }
        if w.increment(id, AGE) < 6 {
            let tile = w.random_tile();
            w.set(id, DEST, i64::from(tile));
            return 1;
        }
        w.set_state(id, 1);
        let mut n = 0u32;
        let mut train = w.next(3, INVALID);
        while train != INVALID {
            if valid_train(&w, train) {
                n = n.wrapping_add(1);
            }
            train = w.next(3, train);
        }
        if n == 0 {
            return w.delete(id).await;
        }
        n = w.range(n);
        train = w.next(3, INVALID);
        while train != INVALID {
            if valid_train(&w, train) {
                let old = n;
                n = n.wrapping_sub(1);
                if old == 0 {
                    w.set(id, DEST, w.v(train, TILE));
                    w.set(id, AGE, 0);
                    break;
                }
            }
            train = w.next(3, train);
        }
    }
    1
}
async fn destroyer(w: World<'_>, id: u32) -> i64 {
    w.increment(id, TICK);
    let gp = w.position(id);
    let z = w.flight(id);
    w.update_position(id, gp[0], gp[1], z);
    if gp[0] > (w.world()[0] + 9) * 16 - 1 {
        return w.delete(id).await;
    }
    if w.state(id) == 0 {
        let target = w.target(id);
        if delta(w.v(id, X), w.v(target, X)) > 16 {
            return 1;
        }
        w.set_state(id, 1);
        w.effect_rel(target, 0, 7, 8, 5);
        w.sound_vehicle(target);
        w.delete(target).await;
        for _ in 0..80 {
            let r = w.random();
            w.effect_above(
                i64::from(r & 63) + w.v(id, X) - 32,
                i64::from((r >> 5) & 63) + w.v(id, Y) - 32,
                0,
                7,
            );
        }
        for dy in -3..3 {
            for dx in -3..3 {
                let tile = w.read(12, w.v(id, TILE) as u32, dx, dy)[0] as u32;
                if tile != INVALID_TILE {
                    w.clear(tile).await;
                }
            }
        }
    }
    1
}
async fn submarine(w: World<'_>, id: u32) -> i64 {
    w.increment(id, TICK);
    if w.increment(id, AGE) > 8880 {
        return w.delete(id).await;
    }
    if w.v(id, TICK) & 1 == 0 {
        return 1;
    }
    let step = w.read(13, 0, w.v(id, DIR) >> 1, 0)[0] as u32;
    let tile = (w.v(id, TILE) as u32).wrapping_add(step);
    if w.tile(tile)[0] != 0 {
        let bits = crate::movement::water_tracks(w.services, tile, 255);
        if bits == 63 && !w.chance(90) {
            let gp = w.position(id);
            w.update_position(id, gp[0], gp[1], w.v(id, Z));
            return 1;
        }
    }
    let r = w.random();
    let dir = (w.v(id, DIR) + if r & 1 != 0 { 2 } else { 6 }) & 7;
    w.set(id, DIR, dir);
    1
}
async fn tick(w: World<'_>, id: u32) -> i64 {
    match w.v(id, SUBTYPE) {
        0 => zeppelin(w, id).await,
        2 => small_ufo(w, id).await,
        4 => aircraft(w, id, true).await,
        6 => aircraft(w, id, false).await,
        8 => rotors(&w, id),
        9 => big_ufo(w, id).await,
        11 => destroyer(w, id).await,
        13 | 14 => submarine(w, id).await,
        _ => 1,
    }
}
async fn initialize(w: World<'_>, kind: u32) -> i64 {
    let count = if kind == 3 {
        3
    } else if kind >= 5 {
        1
    } else {
        2
    };
    if kind != 7 && w.read(0, 0, count, 0)[9] == 0 {
        return 0;
    }
    match kind {
        0 | 1 | 4 => {
            let tile = w.random_tile();
            let mut x = w.tile_x(tile) * 16 + 8;
            if kind == 0 {
                let mut station = w.next(4, INVALID);
                while station != INVALID {
                    let st = w.read(3, station, 0, 0);
                    if st[0] as u32 != INVALID_TILE && (st[1] == 0 || st[1] == 1) {
                        x = (w.tile_x(st[0] as u32) + 2) * 16;
                        break;
                    }
                    station = w.next(4, station);
                }
            }
            let (y, dir, subtype) = if kind == 4 {
                (w.world()[2] * 16 - 1, 7, 9)
            } else {
                (0, 3, kind * 2)
            };
            let id = w.create_vehicle(x, y, dir, subtype, INVALID);
            if kind != 0 {
                let s = w.world();
                w.set(
                    id,
                    DEST,
                    i64::from(w.tile_xy(s[0] as u32 / 2, s[1] as u32 / 2)),
                );
            }
            let shadow = w.create_vehicle(x, y, dir, subtype + 1, INVALID);
            w.link(id, shadow);
        }
        2 | 3 => {
            let mut found = INVALID;
            let mut industry = w.next(5, INVALID);
            while industry != INVALID {
                if w.read(2, industry, 0, 0)[kind as usize] != 0
                    && (found == INVALID || w.chance(2))
                {
                    found = industry;
                }
                industry = w.next(5, industry);
            }
            if found == INVALID {
                return 0;
            }
            let x = if kind == 2 {
                (w.world()[0] + 9) * 16 - 1
            } else {
                -256
            };
            let y = w.tile_y(w.read(2, found, 0, 0)[0] as u32) * 16 + 37;
            let dir = if kind == 2 { 1 } else { 5 };
            let subtype = if kind == 2 { 4 } else { 6 };
            let id = w.create_vehicle(x, y, dir, subtype, INVALID);
            let shadow = w.create_vehicle(x, y, dir, subtype + 1, INVALID);
            w.link(id, shadow);
            if kind == 3 {
                let rotor = w.create_vehicle(x, y, dir, 8, INVALID);
                w.link(shadow, rotor);
            }
        }
        5 | 6 => {
            let r = w.random();
            let x = w.tile_x(r & w.world()[4] as u32) * 16 + 8;
            let (y, dir) = if r & 0x8000_0000 != 0 {
                (w.world()[3] * 16 - 9, 7)
            } else {
                (8 + if w.world()[8] != 0 { 16 } else { 0 }, 3)
            };
            if w.tile(w.tile_virt(x, y))[1] != 6 {
                return 0;
            }
            w.create_vehicle(x, y, dir, kind + 8, INVALID);
        }
        7 => {
            let mut index = (w.random() & 15) as i32;
            for _ in 0..15 {
                let mut industry = w.next(5, INVALID);
                while industry != INVALID {
                    let i = w.read(2, industry, 0, 0);
                    if i[4] != 0 {
                        index -= 1;
                        if index < 0 {
                            let news_tile = (i[0] as u32).wrapping_add(w.world()[0] as u32 + 1);
                            w.news(5, news_tile, i[5]);
                            let mut tile = w.read(2, industry, 0, 0)[0] as u32;
                            let dir = w.random() & 3;
                            let step = w.read(13, 0, i64::from(dir), 0)[0] as u32;
                            for _ in 0..30 {
                                w.clear(tile).await;
                                tile = tile.wrapping_add(step);
                                if w.tile(tile)[0] == 0 {
                                    break;
                                }
                            }
                            return 0;
                        }
                    }
                    industry = w.next(5, industry);
                }
            }
        }
        _ => {}
    }
    0
}
fn reset_delay(w: &World<'_>) {
    let delay = (w.random() & 511) + 730;
    // SAFETY: Stable Rust-owned scalar, game thread only; no reference is formed.
    unsafe {
        DELAY = delay as u16;
    }
}
async fn schedule(w: World<'_>, startup: bool) -> i64 {
    if !startup {
        // SAFETY: Same exclusive game-thread scalar boundary as serialization.
        let delay = unsafe { DELAY.wrapping_sub(1) };
        // SAFETY: No reference retained or concurrent access.
        unsafe {
            DELAY = delay;
        }
        if delay != 0 {
            return 0;
        }
    }
    reset_delay(&w);
    if startup || w.world()[6] == 0 {
        return 0;
    }
    let year = w.world()[5];
    let years = [
        (1930, 1955),
        (1940, 1970),
        (1960, 1990),
        (1970, 2000),
        (2000, 2100),
        (1940, 1965),
        (1975, 2010),
        (1950, 1985),
    ];
    let mut available = [0u32; 8];
    let mut length = 0;
    for (index, (min, max)) in years.into_iter().enumerate() {
        if year >= min && year < max {
            available[length] = index as u32;
            length += 1;
        }
    }
    if length == 0 {
        return 0;
    }
    let kind = available[w.range(length as u32) as usize];
    initialize(w, kind).await
}
fn construct(w: &World<'_>, id: u32, x: i64, y: i64, dir: i64, subtype: u32) -> i64 {
    w.set(id, STATUS, 4);
    w.set(id, X, x);
    w.set(id, Y, y);
    let z = match subtype {
        0 | 2 | 4 | 6 | 9 | 11 => w.bounds(id),
        8 => w.bounds(id) + 5,
        13 | 14 => 0,
        _ => {
            w.set(id, STATUS, w.v(id, STATUS) | 32);
            0
        }
    };
    w.set(id, Z, z);
    w.set(id, DIR, dir);
    w.set(id, TILE, i64::from(w.tile_virt(x, y)));
    w.set(id, SUBTYPE, i64::from(subtype));
    w.service(24, id, 0, 0, 0, 0, 0);
    w.set(id, OWNER, 16);
    w.set_image(id, 0);
    w.set_state(id, 0);
    w.update_image(id);
    w.viewport(id);
    0
}
fn release_industry(w: &World<'_>, industry: u32) -> i64 {
    let mut id = w.next(6, INVALID);
    while id != INVALID {
        let subtype = w.v(id, SUBTYPE);
        if (subtype == 4 || subtype == 6) && w.state(id) > 0 && w.v(id, DEST) as u32 == industry {
            w.set_state(id, 3);
        }
        id = w.next(6, id);
    }
    0
}
fn release_vehicle(w: &World<'_>, id: u32) -> i64 {
    if w.read(14, id, 0, 0)[0] == 0 {
        return 0;
    }
    w.set_state(id, 0);
    let tile = w.random_tile();
    w.set(id, DEST, i64::from(tile));
    let z = w.bounds(id);
    w.set(id, Z, z);
    w.set(id, AGE, 0);
    0
}
async fn run(w: World<'_>, operation: u32, id: u32, a: i64, b: i64, c: i64, d: i64) -> i64 {
    match operation {
        0 => tick(w, id).await,
        1 => schedule(w, false).await,
        2 => schedule(w, true).await,
        3 => construct(&w, id, a, b, c, d as u32),
        4 => {
            w.update_position(id, a, b, c);
            0
        }
        5 => release_industry(&w, id),
        6 => release_vehicle(&w, id),
        7 => {
            w.clear(id).await;
            0
        }
        8 => initialize(w, id).await,
        9 => {
            w.update_image(id);
            0
        }
        _ => 0,
    }
}
pub struct Run<'a> {
    channel: Rc<Channel>,
    future: Pin<Box<dyn Future<Output = i64> + 'a>>,
}
/// Allocate the private owner. The target uses the original `VehicleID` sentinel.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_state_create(target: u32) -> *mut State {
    Box::into_raw(Box::new(State {
        image_override: 0,
        target,
        state: 0,
        flags: 0,
    }))
}
/// Destroy exactly once after the shell stops using its raw scalar addresses.
/// # Safety
/// `state` is the live allocation returned by `state_create`, with no active access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_state_destroy(state: *mut State) {
    // SAFETY: Sole live owner is transferred back to Rust.
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// Process-lifetime raw scalar address for the original DATE/legacy descriptors.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_delay() -> *mut u16 {
    &raw mut DELAY
}
/// Create one component continuation; arguments and IDs follow original preconditions.
/// # Safety
/// Context/callbacks remain live until destruction. Read/write leaves cannot
/// throw or reenter, and copy/write only source-valid canonical scalars. Shared
/// and occupancy tables, including their pointees, stay immutable and live through
/// Run destruction, suspension and all World clones. Context may mutate
/// independently; table references borrow no world storage. Services are
/// synchronous and noexcept; allocation may construct a separate owner. No raw
/// State reference spans any service; flight helpers access flags only during
/// their call. Returned actions 2/14/17 allow deletion/Crash/clear callbacks on
/// the C++ stack. All operations run on the game thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_create<'a>(
    operation: u32,
    id: u32,
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    context: *mut c_void,
    read: unsafe extern "C" fn(*mut c_void, u32, u32, i64, i64, *mut i64),
    write: unsafe extern "C" fn(*mut c_void, u32, u32, i64),
    services: &'a Services,
    occupancy: &'a Occupancy,
    service: unsafe extern "C" fn(*mut c_void, *const Action) -> i64,
) -> *mut Run<'a> {
    let channel = Rc::new(Channel {
        action: Cell::new(Action::default()),
        response: Cell::new(0),
        vehicle: Cell::new(None),
        world: Cell::new(None),
    });
    let w = World {
        context,
        read,
        write,
        services,
        occupancy,
        service,
        channel: channel.clone(),
    };
    Box::into_raw(Box::new(Run {
        channel,
        future: Box::pin(run(w, operation, id, a, b, c, d)),
    }))
}
/// Resume after the prior C++ action completes. Kind0 contains the final result.
/// # Safety
/// `owner` is live/exclusive for this call; source-valid callback contracts hold.
/// Returned actions retain no Rust owner/world references. Panics abort, never unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_advance(
    owner: *mut Run<'_>,
    response: i64,
) -> Action {
    // SAFETY: One invocation exclusively polls its separate continuation allocation.
    let run = unsafe { &mut *owner };
    run.channel.response.set(response);
    run.channel.vehicle.set(None);
    run.channel.world.set(None);
    let mut context = PollContext::from_waker(Waker::noop());
    match run.future.as_mut().poll(&mut context) {
        Poll::Pending => run.channel.action.get(),
        Poll::Ready(result) => Action {
            a: result,
            ..Action::default()
        },
    }
}
/// Cancel/drop the continuation exactly once, including while an action is suspended.
/// # Safety
/// Owner is a live create result with no active advance call; no action borrows it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_destroy(owner: *mut Run<'_>) {
    // SAFETY: Table stays live through drop of the future and its World clones.
    // Cancellation calls no world service and releases every table borrow.
    unsafe {
        drop(Box::from_raw(owner));
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDisasterOccupancy" ["probe"] => Occupancy { commands, vehicle, };
    "OpenTTDDisasterState" ["probe"] => State {
        image_override,
        target,
        state,
        flags,
    };
    "OpenTTDDisasterAction" ["probe"] => Action {
        kind,
        id,
        other,
        a,
        b,
        c,
        d,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{CommandCost, CommandCostServices, CommandCostStorage};
    use crate::vehicle::{self, VehicleRef, VehicleServices};
    static INDEX_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct Fixture {
        world: [i64; 24],
        vehicle: [i64; 24],
        private: State,
        writes: u32,
        reads: u32,
        disaster_in_pool: bool,
        rng: u32,
        draws: u32,
        calls: Vec<Action>,
        viewport_age: Option<i64>,
        links: vehicle::occupancy::Links,
    }
    impl Default for Fixture {
        fn default() -> Self {
            let mut world = [0; 24];
            world[..10].copy_from_slice(&[64, 128, 63, 127, 8191, 1935, 1, 0, 1, 1]);
            let mut vehicle = [0; 24];
            vehicle[NEXT] = i64::from(INVALID);
            Self {
                world,
                vehicle,
                private: State {
                    image_override: 0,
                    target: INVALID,
                    state: 0,
                    flags: 0,
                },
                writes: 0,
                reads: 0,
                disaster_in_pool: false,
                rng: 5,
                draws: 0,
                calls: Vec::new(),
                viewport_age: None,
                links: vehicle::occupancy::Links {
                    next: std::ptr::null_mut(),
                    prev: std::ptr::null_mut(),
                    current: std::ptr::null_mut(),
                },
            }
        }
    }
    unsafe extern "C" fn read(
        context: *mut c_void,
        kind: u32,
        id: u32,
        a: i64,
        _b: i64,
        out: *mut i64,
    ) {
        // SAFETY: Tests own a stable fixture; only this synchronous callback accesses it.
        let fixture = unsafe { &mut *context.cast::<Fixture>() };
        fixture.reads += 1;
        let mut record = [0i64; 24];
        match kind {
            0 => record = fixture.world,
            1 => record = fixture.vehicle,
            6 => {
                record[0] = i64::from(
                    if id == 6 && fixture.disaster_in_pool && a as u32 == INVALID {
                        17
                    } else {
                        INVALID
                    },
                );
            }
            7 => record[0] = (&raw mut fixture.private) as usize as i64,
            13 => {
                record[0] = if a == 0 {
                    -1
                } else if a == 1 {
                    fixture.world[0]
                } else if a == 2 {
                    1
                } else {
                    -fixture.world[0]
                }
            }
            14 => record[0] = 1,
            _ => {}
        }
        // SAFETY: Caller supplies exactly 24 writable copied slots.
        unsafe {
            std::ptr::copy_nonoverlapping(record.as_ptr(), out, 24);
        }
    }
    unsafe extern "C" fn write(context: *mut c_void, field: u32, _id: u32, value: i64) {
        // SAFETY: One canonical scalar write into the exclusive test fixture.
        let fixture = unsafe { &mut *context.cast::<Fixture>() };
        fixture.writes += 1;
        fixture.vehicle[field as usize] = value;
    }
    extern "C" fn random(context: *mut c_void) -> u32 {
        // SAFETY: The call-scoped exclusive fixture has no active Rust borrow.
        let fixture = unsafe { &mut *context.cast::<Fixture>() };
        fixture.draws += 1;
        fixture.rng
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    unsafe extern "C" fn service(context: *mut c_void, request: *const Action) -> i64 {
        // SAFETY: Call-scoped fixture/request; no borrow spans nested world access.
        let fixture = unsafe { &mut *context.cast::<Fixture>() };
        let action = unsafe { *request };
        fixture.calls.push(action);
        match action.kind {
            5 => {
                fixture.private.flags = 0xab;
                100
            }
            6 => 100,
            15 => 16 + fixture.calls.iter().filter(|a| a.kind == 15).count() as i64,
            _ => 0,
        }
    }
    // Unused typed slots abort if a controller starts using an unmodelled leaf.
    macro_rules! unused {
        ($($arg:ty),* $(; $result:ty)?) => {{
            extern "C" fn unused($(_: $arg),*) $(-> $result)? { panic!("unexpected fixture leaf") }
            unused
        }};
    }
    unsafe extern "C" fn shell(context: *mut c_void, _: u32) -> VehicleRef {
        VehicleRef(std::ptr::NonNull::new(context.cast()).unwrap())
    }
    extern "C" fn links(v: VehicleRef) -> *mut vehicle::occupancy::Links {
        unsafe { &raw mut (*v.0.as_ptr().cast::<Fixture>()).links }
    }
    extern "C" fn tile(v: VehicleRef) -> u32 {
        unsafe { (*v.0.as_ptr().cast::<Fixture>()).vehicle[TILE] as u32 }
    }
    extern "C" fn occupancy_update(v: VehicleRef) -> vehicle::occupancy::UpdateObservation {
        vehicle::occupancy::UpdateObservation {
            links: links(v),
            tile: tile(v),
        }
    }
    extern "C" fn kind(_: VehicleRef) -> u8 {
        vehicle::VehicleType::DISASTER.0
    }
    extern "C" fn log_x() -> u32 {
        6
    }
    extern "C" fn viewport(v: VehicleRef) {
        // The actual owner must have indexed the shell before entering the leaf.
        assert!(unsafe { vehicle::occupancy::OnTile::new(&VEHICLES, tile(v)).get() } == Some(v));
        unsafe {
            vehicle::occupancy::update(&VEHICLES, v, true);
        }
        let f = unsafe { &mut *v.0.as_ptr().cast::<Fixture>() };
        f.calls.push(Action {
            kind: 4,
            ..Action::default()
        });
        if let Some(age) = f.viewport_age {
            f.vehicle[AGE] = age;
        }
    }
    const VEHICLES: VehicleServices = VehicleServices {
        tile_links: links,
        occupancy_update,
        occupancy_near: unused!(vehicle::VehicleRef ; vehicle::occupancy::NearObservation),
        map_log_x: log_x,
        assertions: true,
        train_handle: unused!(*mut vehicle::OpenTTDVehicle ; crate::train::Handle),
        write_counters: unused!(vehicle::VehicleRef, u8, u8, i32),
        waiting_for_unbunching: unused!(vehicle::VehicleRef ; bool),
        leave_unbunching_depot: unused!(vehicle::VehicleRef),
        reset_depot_unbunching: unused!(vehicle::VehicleRef),
        enter_depot: unused!(vehicle::VehicleRef, *const vehicle::visits::Services),
        pathfinding_result: unused!(vehicle::VehicleRef, bool),
        new_position: unused!(vehicle::VehicleRef ; vehicle::NewPosition),
        write_xy: unused!(vehicle::VehicleRef, i32, i32),
        base_viewport: viewport,
        newgrf: unused!(; *const crate::newgrf_vehicle::Services),
        property: unused!(*const crate::newgrf_vehicle::Services, vehicle::VehicleRef, u8, i32 ; i32),
        callback: unused!(*const crate::newgrf_vehicle::Services, vehicle::VehicleRef, crate::newgrf_callbacks::CallbackID ; u16),
        unknown_callback_result: unused!(
            vehicle::EngineRef,
            crate::newgrf_callbacks::CallbackID,
            u16
        ),
        length_changed: unused!(vehicle::VehicleRef),
        play_vehicle_sound: unused!(vehicle::VehicleRef, crate::newgrf_sound::VehicleSoundEvent ; bool),
        play_sound: unused!(vehicle::VehicleRef, u16),
        large_explosion: unused!(vehicle::VehicleRef),
        order_free: unused!(vehicle::VehicleRef),
        order_dummy: unused!(vehicle::VehicleRef),
        start_stop_dirty: unused!(vehicle::VehicleRef),
        destroy: unused!(vehicle::VehicleRef),
        check_orders: unused!(vehicle::VehicleRef ; u8),
        day: unused!(vehicle::VehicleRef ; u8),
        write_day: unused!(vehicle::VehicleRef, u8),
        write_direction: unused!(vehicle::VehicleRef, u8),
        write_speed: unused!(vehicle::VehicleRef, u16),
        last_station: unused!(vehicle::VehicleRef ; u16),
        write_last_station: unused!(vehicle::VehicleRef, u16),
        set_dest_tile: unused!(vehicle::VehicleRef, u32),
        write_dest: unused!(vehicle::VehicleRef, u32),
        write_progress: unused!(vehicle::VehicleRef, u8),
        write_max_speed: unused!(vehicle::VehicleRef, u16),
        previous_tile: unused!(vehicle::VehicleRef ; vehicle::PreviousTile),
        detach_last: unused!(
            vehicle::VehicleRef,
            vehicle::VehicleRef,
            vehicle::VehicleRef
        ),
        next: unused!(vehicle::VehicleRef ; Option<vehicle::VehicleRef>),
        engine_id: unused!(vehicle::VehicleRef ; u16),
        engine: unused!(vehicle::VehicleRef ; vehicle::EngineRef),
        engine_for_length: unused!(vehicle::VehicleRef ; vehicle::EngineForLength),
        write_tick: unused!(vehicle::VehicleRef, u8),
        direction: unused!(vehicle::VehicleRef ; u8),
        status: unused!(vehicle::VehicleRef ; u8),
        tile,
        progress: unused!(vehicle::VehicleRef ; u8),
        dest_tile: unused!(vehicle::VehicleRef ; u32),
        order_type: unused!(vehicle::VehicleRef ; u8),
        order_destination: unused!(vehicle::VehicleRef ; u16),
        order_nonstop: unused!(vehicle::VehicleRef ; u8),
        subtype: unused!(vehicle::VehicleRef ; u8),
        x: unused!(vehicle::VehicleRef ; i32),
        y: unused!(vehicle::VehicleRef ; i32),
        z: unused!(vehicle::VehicleRef ; i32),
        tick: unused!(vehicle::VehicleRef ; u8),
        owner: unused!(vehicle::VehicleRef ; u8),
        speed: unused!(vehicle::VehicleRef ; u16),
        subspeed: unused!(vehicle::VehicleRef ; u8),
        running: unused!(vehicle::VehicleRef ; u8),
        order_time: unused!(vehicle::VehicleRef ; i32),
        vehicle_type: kind,
        write_x: unused!(vehicle::VehicleRef, i32),
        write_y: unused!(vehicle::VehicleRef, i32),
        write_z: unused!(vehicle::VehicleRef, i32),
        write_tile: unused!(vehicle::VehicleRef, u32),
        write_status: unused!(vehicle::VehicleRef, u8),
        write_subspeed: unused!(vehicle::VehicleRef, u8),
        write_breakdown_counter: unused!(vehicle::VehicleRef, u8),
        write_running: unused!(vehicle::VehicleRef, u8),
        write_order_time: unused!(vehicle::VehicleRef, i32),
        write_profit: unused!(vehicle::VehicleRef, i64),
        write_economy_service: unused!(vehicle::VehicleRef, i32),
        write_calendar_service: unused!(vehicle::VehicleRef, i32),
        write_breakdowns: unused!(vehicle::VehicleRef, u8),
        write_reliability: unused!(vehicle::VehicleRef, u16),
        write_cargo_age: unused!(vehicle::VehicleRef, u16),
        coordinates: unused!(vehicle::VehicleRef ; vehicle::Coordinates),
        xy: unused!(vehicle::VehicleRef ; vehicle::XY),
        speed_inputs: unused!(vehicle::VehicleRef ; vehicle::SpeedInputs),
        direction_progress: unused!(vehicle::VehicleRef ; vehicle::DirectionProgress),
        tick_status: unused!(vehicle::VehicleRef ; vehicle::TickStatus),
        day_cost: unused!(vehicle::VehicleRef ; vehicle::DayCost),
        make_dummy: unused!(vehicle::VehicleRef),
        loading: unused!(vehicle::VehicleRef, bool, *const vehicle::visits::Services),
        check_orders_only: unused!(vehicle::VehicleRef),
        write_coordinates: unused!(vehicle::VehicleRef, i32, i32, i32),
        altitude_inputs: unused!(vehicle::VehicleRef ; vehicle::AltitudeInputs),
        position_observations: unused!(vehicle::VehicleRef ; vehicle::PositionObservations),
        movement_observations: unused!(vehicle::VehicleRef ; vehicle::MovementObservations),
        max_speed: unused!(vehicle::VehicleRef ; u16),
        maintenance: std::ptr::null(),
        visits: unused!( ; *const vehicle::visits::Services),
        orders: unused!( ; *const crate::orders::Leaves),
        ship_get: unused!(u32 ; vehicle::VehicleRef),
    };
    // Existing tests never clear; the native ABI fixture below covers full costs.
    const OCCUPANCY: Occupancy = Occupancy {
        commands: &COMMANDS,
        vehicle: shell,
    };
    const COMMANDS: vehicle::occupancy::Commands = vehicle::occupancy::Commands {
        vehicles: &VEHICLES,
        max_pixel_z: unused!(u32; i32),
        costs: &COSTS,
        vehicle_error: 0,
    };
    const COSTS: CommandCostServices = CommandCostServices {
        construct: unused!(*mut CommandCostStorage ; *mut CommandCost),
        destroy: unused!(*mut CommandCost),
        zero: unused!(*mut CommandCost),
        error: unused!(*mut CommandCost, u32),
        expense_money: unused!(*mut CommandCost, u8, i64),
        error_param: unused!(*mut CommandCost, u32, u64),
        add: unused!(*mut CommandCost, *mut CommandCost),
        move_cost: unused!(*mut CommandCost, *mut CommandCost),
        amount: unused!(*mut CommandCost, i64),
        success: unused!(*const CommandCost ; bool),
        error_message: unused!(*const CommandCost ; u32),
        money: unused!(*const CommandCost ; i64),
        expense: unused!(*const CommandCost ; crate::services::ExpenseType),
        make_error: unused!(*mut CommandCost, u32),
    };
    fn owner(services: &Services, operation: u32, id: u32) -> *mut Run<'_> {
        // SAFETY: Fixture and immutable table remain stable through run destruction;
        // leaves never reenter. Reuse the table's raw context so no new exclusive
        // fixture borrow invalidates it. The table is separate from fixture storage.
        unsafe {
            openttd_rust_disaster_create(
                operation,
                id,
                0,
                0,
                0,
                0,
                services.context,
                read,
                write,
                services,
                &OCCUPANCY,
                service,
            )
        }
    }
    fn advance(run: *mut Run<'_>, response: i64) -> Action {
        // SAFETY: Tests hold the live exclusive owner and complete each action before resuming.
        unsafe { openttd_rust_disaster_advance(run, response) }
    }
    fn destroy(run: *mut Run<'_>) {
        // SAFETY: No advance or world reference is active.
        unsafe {
            openttd_rust_disaster_destroy(run);
        }
    }
    #[test]
    fn cancelling_suspended_deletion_calls_no_world_service() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        fixture.vehicle[SUBTYPE] = 13;
        fixture.vehicle[AGE] = 8880;
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let run = owner(&services, 0, 17);
        let channel = unsafe { Rc::downgrade(&(*run).channel) };
        assert_eq!(advance(run, 0).kind, 2);
        let counts = (fixture.reads, fixture.writes, fixture.draws);
        destroy(run);
        assert_eq!((fixture.reads, fixture.writes, fixture.draws), counts);
        assert!(channel.upgrade().is_none());
    }
    #[test]
    fn submarine_expiry_wraps_tick_before_returning_delete() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        fixture.vehicle[SUBTYPE] = 13;
        fixture.vehicle[TICK] = 255;
        fixture.vehicle[AGE] = 8880;
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let run = owner(&services, 0, 17);
        let action = advance(run, 0);
        assert_eq!((action.kind, action.id), (2, 17));
        assert_eq!((fixture.vehicle[TICK], fixture.vehicle[AGE]), (0, 8881));
        assert_eq!(advance(run, 0).a, 0);
        destroy(run);
    }
    #[test]
    fn flight_helper_mutates_stable_private_flags_synchronously() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        fixture.vehicle[TICK] = 1;
        let address = &raw mut fixture.private;
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let run = owner(&services, 0, 17);
        assert_eq!(advance(run, 0).kind, 0);
        assert_eq!(
            (&raw mut fixture.private, fixture.private.flags),
            (address, 0xab)
        );
        assert_eq!(
            fixture.calls.iter().map(|a| a.kind).collect::<Vec<_>>(),
            [5, 4]
        );
        destroy(run);
    }
    #[test]
    fn big_ufo_initializer_uses_map_max_x_for_y_on_rectangular_maps() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let run = owner(&services, 8, 4);
        assert_eq!(advance(run, 0).kind, 0);
        let creations: Vec<_> = fixture.calls.iter().filter(|a| a.kind == 15).collect();
        assert_eq!(
            (
                creations[0].id,
                creations[0].a,
                creations[0].b,
                creations[0].c
            ),
            (9, 88, 1007, 7)
        );
        assert_eq!(
            (creations[1].id, creations[1].a, creations[1].b),
            (10, 88, 1007)
        );
        assert_eq!(fixture.vehicle[DEST], i64::from(64 * 64 + 32));
        assert_eq!(fixture.vehicle[NEXT], 18);
        assert_eq!(fixture.draws, 1);
        destroy(run);
    }
    #[test]
    fn industry_release_commits_private_state_before_return() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        fixture.vehicle[SUBTYPE] = 4;
        fixture.private.state = 1;
        fixture.vehicle[DEST] = 99;
        fixture.disaster_in_pool = true;
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let release = owner(&services, 5, 99);
        assert_eq!(advance(release, 0).kind, 0);
        assert_eq!(fixture.private.state, 3);
        destroy(release);
    }
    #[test]
    fn direct_service_refreshes_canonical_age_before_phase_transition() {
        let _guard = INDEX_TEST.lock().unwrap();
        let mut fixture = Fixture::default();
        fixture.vehicle[TICK] = 1;
        fixture.private.state = 1;
        fixture.viewport_age = Some(37);
        let services =
            crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
        let run = owner(&services, 0, 17);
        assert_eq!(advance(run, 0).kind, 0);
        assert_eq!((fixture.private.state, fixture.vehicle[AGE]), (2, 0));
        destroy(run);
    }
    #[test]
    fn failed_pool_capacity_consumes_no_initializer_rng() {
        let _guard = INDEX_TEST.lock().unwrap();
        for kind in 0..7 {
            let mut fixture = Fixture::default();
            fixture.world[9] = 0;
            let services =
                crate::services::fixture(std::ptr::from_mut(&mut fixture).cast(), random, industry);
            let run = owner(&services, 8, kind);
            assert_eq!(advance(run, 0).kind, 0);
            assert_eq!((fixture.writes, fixture.draws), (0, 0));
            destroy(run);
        }
    }
}

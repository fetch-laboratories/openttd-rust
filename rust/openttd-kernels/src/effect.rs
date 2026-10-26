/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Owns every effect controller and the private animation bytes.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::verbose_bit_mask
)]

#[derive(Default)]
pub struct State {
    animation: u16,
    substate: u8,
}
use crate::random_func::chance16_i;
use crate::services::Services;
use std::ffi::c_void;

/// Call-scoped copied observation of canonical C++ fields.
#[repr(C)]
#[derive(Default)]
pub struct View {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub sprite: u32,
    pub progress: u8,
    pub spritenum: u8,
    pub subtype: u8,
    pub ambient: u8,
}
/// All leaves are noexcept and cannot reenter or retain pointers. Shared vehicle
/// fields remain canonical in C++; every mutation is written immediately.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub observe: extern "C" fn(*mut c_void, *mut View),
    pub write: extern "C" fn(*mut c_void, u8, u32),
    pub viewport: extern "C" fn(*mut c_void),
    pub sound: extern "C" fn(*mut c_void, u8),
}
const INITIAL: [u32; 12] = [
    3701, 3079, 3073, 3084, 2040, 3709, 3737, 3725, 1416, 4751, 2040, 2040,
];
const LAST: [u32; 12] = [
    3708, 3083, 3078, 3089, 2044, 3724, 3740, 3736, 0, 0, 2044, 2044,
];
const BULLDOZER: [(usize, u32, u8); 20] = [
    (0, 0, 4),
    (3, 3, 4),
    (2, 2, 7),
    (0, 2, 7),
    (1, 1, 3),
    (2, 2, 7),
    (0, 2, 7),
    (1, 1, 3),
    (2, 2, 7),
    (0, 2, 7),
    (3, 3, 6),
    (2, 2, 6),
    (1, 1, 7),
    (3, 1, 7),
    (0, 0, 3),
    (1, 1, 7),
    (3, 1, 7),
    (0, 0, 3),
    (1, 1, 7),
    (3, 1, 7),
];
const INC: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const BUBBLE_FLOAT_SW: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 0),
    (1, 0, 1, 1),
    (0, 0, 1, 0),
    (1, 0, 1, 2),
    (1, 4, 0, 0),
];
const BUBBLE_FLOAT_NE: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 0),
    (-1, 0, 1, 1),
    (0, 0, 1, 0),
    (-1, 0, 1, 2),
    (1, 4, 0, 0),
];
const BUBBLE_FLOAT_SE: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 0),
    (0, 1, 1, 1),
    (0, 0, 1, 0),
    (0, 1, 1, 2),
    (1, 4, 0, 0),
];
const BUBBLE_FLOAT_NW: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 0),
    (0, -1, 1, 1),
    (0, 0, 1, 0),
    (0, -1, 1, 2),
    (1, 4, 0, 0),
];
const BUBBLE_BURST: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 2),
    (0, 0, 1, 7),
    (0, 0, 1, 8),
    (0, 0, 1, 9),
    (0, 4, 0, 0),
];
const BUBBLE_ABSORB: &[(i32, i32, i32, u32)] = &[
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (0, 0, 1, 0),
    (0, 0, 1, 2),
    (0, 0, 1, 0),
    (0, 0, 1, 1),
    (2, 1, 3, 0),
    (1, 1, 3, 1),
    (2, 1, 3, 0),
    (1, 1, 3, 2),
    (2, 1, 3, 0),
    (1, 1, 3, 1),
    (2, 1, 3, 0),
    (1, 0, 1, 2),
    (0, 0, 1, 0),
    (1, 0, 1, 1),
    (0, 0, 1, 0),
    (1, 0, 1, 2),
    (0, 0, 1, 0),
    (1, 0, 1, 1),
    (0, 0, 1, 0),
    (1, 0, 1, 2),
    (2, 4, 0, 0),
    (0, 0, 0, 10),
    (0, 0, 0, 11),
    (0, 0, 0, 12),
    (0, 0, 0, 13),
    (0, 0, 0, 14),
    (0, 4, 0, 0),
];
const BUBBLES: [&[(i32, i32, i32, u32)]; 6] = [
    BUBBLE_FLOAT_SW,
    BUBBLE_FLOAT_NE,
    BUBBLE_FLOAT_SE,
    BUBBLE_FLOAT_NW,
    BUBBLE_BURST,
    BUBBLE_ABSORB,
];

// No Rust reference to the private owner survives a service call. Its lifetime
// ends only after run returns false and the C++ facade deletes the shell.
struct Controller<'a> {
    owner: *mut State,
    context: *mut c_void,
    leaves: Leaves,
    services: &'a Services,
    v: View,
}
#[allow(unsafe_code)]
impl Controller<'_> {
    // Every call-scoped borrow ends at its statement, before any service call.
    fn state(&mut self) -> &mut State {
        unsafe { &mut *self.owner }
    }
    fn write(&mut self, field: u8, value: u32) {
        match field {
            0 => self.v.x = value as i32,
            1 => self.v.y = value as i32,
            2 => self.v.z = value as i32,
            3 | 4 => self.v.sprite = value,
            5 => self.v.progress = value as u8,
            6 => self.v.spritenum = value as u8,
            _ => {} // Only the seven private field constants above reach this helper.
        }
        (self.leaves.write)(self.context, field, value);
    }
    fn increment(&mut self) -> bool {
        if self.v.sprite == LAST[usize::from(self.v.subtype)] {
            return false;
        }
        self.write(4, self.v.sprite.wrapping_add(1));
        true
    }
    fn viewport(&self) {
        (self.leaves.viewport)(self.context);
    }
    fn industry(&self, tile: &mut u32) -> u32 {
        (self.services.industry)(self.v.x, self.v.y, tile)
    }
    fn init(&mut self) {
        if self.v.subtype == 0 {
            let r = self.services.random();
            self.write(3, INITIAL[0] + (r & 7));
            self.write(5, (r >> 16) & 7);
            return;
        }
        self.write(3, INITIAL[usize::from(self.v.subtype)]);
        if self.v.subtype == 9 {
            self.write(6, 0);
        }
        self.write(
            5,
            match self.v.subtype {
                1 | 4 | 10 | 11 => 12,
                3 => 1,
                _ => 0,
            },
        );
        if self.v.subtype == 8 {
            self.state().animation = 0;
            self.state().substate = 0;
        }
    }
    fn countdown(&mut self) -> bool {
        self.state().animation = self.state().animation.wrapping_sub(1);
        self.state().animation != 0
    }
    fn bubble(&mut self) -> bool {
        if self.v.progress & 3 != 0 {
            return true;
        }
        let mut animation;
        if self.v.spritenum == 0 {
            self.write(4, self.v.sprite.wrapping_add(1));
            if self.v.sprite < 4754 {
                self.viewport();
                return true;
            }
            let spritenum = if self.state().substate != 0 {
                (self.services.random() & 3) + 1
            } else {
                6
            };
            self.write(6, spritenum);
            animation = 0;
        } else {
            animation = u32::from(self.state().animation) + 1;
        }
        let b = BUBBLES[usize::from(self.v.spritenum) - 1][animation as usize];
        if b.1 == 4 && b.0 == 0 {
            return false;
        }
        if b.1 == 4 && b.0 == 1 {
            if self.v.z > 180 || chance16_i(1, 96, self.services.random()) {
                self.write(6, 5);
                if self.v.ambient != 0 {
                    (self.leaves.sound)(self.context, 0);
                }
            }
            animation = 0;
        }
        if b.1 == 4 && b.0 == 2 {
            animation += 1;
            if self.v.ambient != 0 {
                (self.leaves.sound)(self.context, 1);
            }
            let mut tile = 0;
            if self.industry(&mut tile) == 2 {
                crate::animated_tile::openttd_rust_animation_add(tile, true);
            }
        }
        self.state().animation = animation as u16;
        let b = BUBBLES[usize::from(self.v.spritenum) - 1][animation as usize];
        self.write(0, self.v.x.wrapping_add(b.0) as u32);
        self.write(1, self.v.y.wrapping_add(b.1) as u32);
        self.write(2, self.v.z.wrapping_add(b.2) as u32);
        self.write(3, 4748 + b.3);
        self.viewport();
        true
    }
    #[allow(clippy::too_many_lines)]
    fn tick(&mut self) -> bool {
        match self.v.subtype {
            0 => {
                if self.v.progress > 0 {
                    self.write(5, u32::from(self.v.progress - 1));
                    return true;
                }
                if self.industry(&mut 0) == 0 {
                    return false;
                }
                if !self.increment() {
                    self.write(3, 3701);
                }
                self.write(5, 7);
                self.viewport();
            }
            3 => {
                if self.v.progress < 2 {
                    self.write(5, u32::from(self.v.progress + 1));
                    return true;
                }
                self.write(5, 0);
                if !self.increment() {
                    return false;
                }
                self.viewport();
            }
            _ => {
                self.write(5, u32::from(self.v.progress.wrapping_add(1)));
                match self.v.subtype {
                    1 | 4 | 10 | 11 => {
                        let mut moved = false;
                        if self.v.progress & (if self.v.subtype == 1 { 7 } else { 3 }) == 0 {
                            self.write(2, self.v.z.wrapping_add(1) as u32);
                            moved = true;
                        }
                        if self.v.progress & 15 == 4 {
                            if !self.increment() {
                                return false;
                            }
                            moved = true;
                        }
                        if moved {
                            self.viewport();
                        }
                    }
                    2 => {
                        if self.v.progress & 3 == 0 {
                            self.write(2, self.v.z.wrapping_add(1) as u32);
                            self.viewport();
                        } else if self.v.progress & 7 == 1 {
                            if !self.increment() {
                                return false;
                            }
                            self.viewport();
                        }
                    }
                    5 | 7 => {
                        if self.v.progress & 3 == 0 {
                            if !self.increment() {
                                return false;
                            }
                            self.viewport();
                        }
                    }
                    6 => {
                        if self.v.progress & 7 == 0 {
                            if !self.increment() {
                                self.write(3, 3737);
                            }
                            self.viewport();
                        }
                        return self.countdown();
                    }
                    8 => {
                        if self.v.progress & 7 != 0 {
                            return true;
                        }
                        let b = BULLDOZER[usize::from(self.state().animation)];
                        self.write(3, 1416 + b.1);
                        self.write(0, self.v.x.wrapping_add(INC[b.0].0) as u32);
                        self.write(1, self.v.y.wrapping_add(INC[b.0].1) as u32);
                        self.state().substate = self.state().substate.wrapping_add(1);
                        if self.state().substate >= b.2 {
                            self.state().substate = 0;
                            self.state().animation = self.state().animation.wrapping_add(1);
                            if usize::from(self.state().animation) == BULLDOZER.len() {
                                return false;
                            }
                        }
                        self.viewport();
                    }
                    9 => return self.bubble(),
                    _ => {
                        let _ = INITIAL[usize::from(self.v.subtype)];
                    }
                }
            }
        }
        true
    }
}

/// Allocates zeroed private state for both ordinary construction and indexed loading.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_new() -> *mut State {
    Box::into_raw(Box::default())
}
/// # Safety
/// Destroy exactly once after the shell's final use. No borrow survives return.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_destroy(owner: *mut State) {
    drop(unsafe { Box::from_raw(owner) });
}
/// # Safety
/// Owner is live/aligned; field is 0 animation or 1 substate. Synchronous access
/// is serialized and no callback is active. Original narrowing is preserved.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_set(owner: *mut State, field: u8, value: u16) {
    let s = unsafe { &mut *owner };
    if field == 0 {
        s.animation = value;
    } else {
        s.substate = value as u8;
    }
}
/// # Safety
/// Same live/serialized owner contract as set; no pointer/reference is retained.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_get(owner: *const State, field: u8) -> u16 {
    let s = unsafe { &*owner };
    if field == 0 {
        s.animation
    } else {
        u16::from(s.substate)
    }
}
/// # Safety
/// Owner is live and exclusive for this invocation and context is its live C++ shell; tables
/// stay live and immutable until return. Leaves may allocate but cannot destroy the owner.
/// False requests deletion after return. Save/load uses the get/set exports instead. The
/// alignment, disjointness, callback and source-index requirements are in `effect_ffi.h`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_run(
    owner: *mut State,
    context: *mut c_void,
    leaves: *const Leaves,
    services: *const Services,
    initialize: u8,
) -> u8 {
    let leaves = unsafe { *leaves };
    let mut v = View::default();
    (leaves.observe)(context, &raw mut v);
    let mut controller = Controller {
        owner,
        context,
        leaves,
        services: unsafe { &*services },
        v,
    };
    if initialize != 0 {
        controller.init();
        1
    } else {
        u8::from(controller.tick())
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEffectView" ["probe"] => View {
        x,
        y,
        z,
        sprite,
        progress,
        spritenum,
        subtype,
        ambient,
    };
    "OpenTTDEffectLeaves" ["probe"] => Leaves {
        observe,
        write,
        viewport,
        sound,

    };
"OpenTTDEffectAllocation" ["probe"] => Allocation { shell, owner, };
    "OpenTTDEffectPosition" ["probe"] => Position { x, y, z, };
    "OpenTTDEffectCreationServices" ["probe"] => CreationServices {
        effect, shared, can_allocate, allocate, place, delta, unclickable, position, slope_pixel_z,
    };

}

/// Creation owns the allocation gate, field/initializer/viewport order and offsets.
/// Services are immutable for the process. Native pools and shells remain canonical;
/// creation callbacks cannot reset the map or delete a newly allocated effect.
#[repr(C)]
struct Allocation {
    shell: *mut c_void,
    owner: *mut State,
}
#[repr(C)]
struct Position {
    x: i32,
    y: i32,
    z: i32,
}
#[repr(C)]
pub struct CreationServices {
    effect: &'static Leaves,
    shared: &'static Services,
    can_allocate: extern "C" fn(usize) -> bool,
    allocate: extern "C" fn() -> Allocation,
    place: extern "C" fn(*mut c_void, i32, i32, i32, u8),
    delta: extern "C" fn(*mut c_void),
    unclickable: extern "C" fn(*mut c_void),
    position: extern "C" fn(*const crate::vehicle::OpenTTDVehicle) -> Position,
    slope_pixel_z: extern "C" fn(i32, i32) -> i32,
}
static CREATION: crate::game_state::GameState<*const CreationServices> =
    crate::game_state::GameState::new(std::ptr::null());
/// Bind immutable process-wide services before vehicle initialization or native creation.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn openttd_rust_effect_bind(services: *const CreationServices) {
    unsafe {
        *CREATION.get() = services;
    }
}
#[allow(unsafe_code)]
fn creation() -> &'static CreationServices {
    unsafe { &**CREATION.get() }
}
/// `CreateEffectVehicle` over the shared native pool.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn openttd_rust_effect_create(x: i32, y: i32, z: i32, kind: u8) -> *mut c_void {
    let s = creation();
    if !(s.can_allocate)(1) {
        return std::ptr::null_mut();
    }
    let v = (s.allocate)();
    (s.place)(v.shell, x, y, z, kind);
    (s.delta)(v.shell);
    (s.unclickable)(v.shell);
    unsafe {
        openttd_rust_effect_run(v.owner, v.shell, s.effect, s.shared, 1);
    }
    (s.effect.viewport)(v.shell);
    v.shell
}
/// `CreateEffectVehicleAbove`, clamping only the terrain lookup.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn openttd_rust_effect_create_above(
    x: i32,
    y: i32,
    z: i32,
    kind: u8,
) -> *mut c_void {
    let s = creation();
    let safe_x = x.clamp(0, ((s.shared.map_max_x)() * 16) as i32);
    let safe_y = y.clamp(0, ((s.shared.map_max_y)() * 16) as i32);
    openttd_rust_effect_create(
        x,
        y,
        (s.slope_pixel_z)(safe_x, safe_y).wrapping_add(z),
        kind,
    )
}
/// `CreateEffectVehicleRel`; the source shell is live for this synchronous call.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn openttd_rust_effect_create_rel(
    v: *const crate::vehicle::OpenTTDVehicle,
    x: i32,
    y: i32,
    z: i32,
    kind: u8,
) -> *mut c_void {
    let p = (creation().position)(v);
    openttd_rust_effect_create(
        p.x.wrapping_add(x),
        p.y.wrapping_add(y),
        p.z.wrapping_add(z),
        kind,
    )
}

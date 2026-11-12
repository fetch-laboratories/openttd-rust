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
use crate::vehicle::VehicleRef;
use std::ffi::c_void;

/// Controller-local values at the original observation points.
#[derive(Default)]
struct View {
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
/// fields use the shared record; every mutation is written immediately.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub vehicles: *const crate::vehicle::VehicleServices,
    pub sprite_sequence: extern "C" fn(*mut c_void, u32),
    pub sprite: extern "C" fn(*mut c_void, u32),
    pub spritenum: extern "C" fn(*mut c_void, u8),
    pub viewport: extern "C" fn(*mut c_void),
    pub sound: extern "C" fn(*mut c_void, u8),
}
const INITIAL: [u32; crate::effectvehicle_func::EV_END as usize] = [
    3701, 3079, 3073, 3084, 2040, 3709, 3737, 3725, 1416, 4751, 2040, 2040,
];
const LAST: [u32; crate::effectvehicle_func::EV_END as usize] = [
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
    record: VehicleRef,
    leaves: &'a Leaves,
    services: &'a Services,
    v: View,
}
#[allow(unsafe_code)]
impl Controller<'_> {
    // Every call-scoped borrow ends at its statement, before any service call.
    fn state(&mut self) -> &mut State {
        unsafe { &mut *self.owner }
    }
    fn set_x(&mut self, value: i32) {
        self.v.x = value;
        self.record.write_x(value);
    }
    fn set_y(&mut self, value: i32) {
        self.v.y = value;
        self.record.write_y(value);
    }
    fn set_z(&mut self, value: i32) {
        self.v.z = value;
        self.record.write_z(value);
    }
    fn set_sprite_sequence(&mut self, value: u32) {
        self.v.sprite = value;
        (self.leaves.sprite_sequence)(self.context, value);
    }
    fn set_first_sprite(&mut self, value: u32) {
        self.v.sprite = value;
        (self.leaves.sprite)(self.context, value);
    }
    fn set_progress(&mut self, value: u8) {
        self.v.progress = value;
        self.record.write_progress(value);
    }
    fn set_spritenum(&mut self, value: u8) {
        self.v.spritenum = value;
        (self.leaves.spritenum)(self.context, value);
    }
    fn increment(&mut self) -> bool {
        if self.v.sprite == LAST[usize::from(self.v.subtype)] {
            return false;
        }
        self.set_first_sprite(self.v.sprite.wrapping_add(1));
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
            self.set_sprite_sequence(INITIAL[0] + (r & 7));
            self.set_progress(((r >> 16) & 7) as u8);
            return;
        }
        self.set_sprite_sequence(INITIAL[usize::from(self.v.subtype)]);
        if self.v.subtype == 9 {
            self.set_spritenum(0);
        }
        self.set_progress(match self.v.subtype {
            1 | 4 | 10 | 11 => 12,
            3 => 1,
            _ => 0,
        });
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
            self.set_first_sprite(self.v.sprite.wrapping_add(1));
            if self.v.sprite < 4754 {
                self.viewport();
                return true;
            }
            let spritenum = if self.state().substate != 0 {
                (self.services.random() & 3) + 1
            } else {
                6
            };
            self.set_spritenum(spritenum as u8);
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
                self.set_spritenum(5);
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
        self.set_x(self.v.x.wrapping_add(b.0));
        self.set_y(self.v.y.wrapping_add(b.1));
        self.set_z(self.v.z.wrapping_add(b.2));
        self.set_sprite_sequence(4748 + b.3);
        self.viewport();
        true
    }
    #[allow(clippy::too_many_lines)]
    fn tick(&mut self) -> bool {
        match self.v.subtype {
            0 => {
                if self.v.progress > 0 {
                    self.set_progress(self.v.progress - 1);
                    return true;
                }
                if self.industry(&mut 0) == 0 {
                    return false;
                }
                if !self.increment() {
                    self.set_sprite_sequence(3701);
                }
                self.set_progress(7);
                self.viewport();
            }
            3 => {
                if self.v.progress < 2 {
                    self.set_progress(self.v.progress + 1);
                    return true;
                }
                self.set_progress(0);
                if !self.increment() {
                    return false;
                }
                self.viewport();
            }
            _ => {
                self.set_progress(self.v.progress.wrapping_add(1));
                match self.v.subtype {
                    1 | 4 | 10 | 11 => {
                        let mut moved = false;
                        if self.v.progress & (if self.v.subtype == 1 { 7 } else { 3 }) == 0 {
                            self.set_z(self.v.z.wrapping_add(1));
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
                            self.set_z(self.v.z.wrapping_add(1));
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
                                self.set_sprite_sequence(3737);
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
                        self.set_sprite_sequence(1416 + b.1);
                        self.set_x(self.v.x.wrapping_add(INC[b.0].0));
                        self.set_y(self.v.y.wrapping_add(INC[b.0].1));
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
/// Owner is live/aligned and access is serialized with no active callback.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_set_animation(owner: *mut State, value: u16) {
    unsafe { (*owner).animation = value };
}
/// # Safety
/// Same live/serialized owner contract as `openttd_rust_effect_set_animation`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_get_animation(owner: *const State) -> u16 {
    unsafe { (*owner).animation }
}
/// # Safety
/// Same live/serialized owner contract as `openttd_rust_effect_set_animation`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_set_substate(owner: *mut State, value: u8) {
    unsafe { (*owner).substate = value };
}
/// # Safety
/// Same live/serialized owner contract as `openttd_rust_effect_set_animation`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_get_substate(owner: *const State) -> u8 {
    unsafe { (*owner).substate }
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
    record: VehicleRef,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let leaves = unsafe { &*leaves };
    let vehicles = unsafe { &*leaves.vehicles };
    let v = View {
        x: record.x(),
        y: record.y(),
        z: record.z(),
        sprite: (vehicles.first_sprite)(record),
        progress: record.progress(),
        spritenum: unsafe { ((*((vehicles.newgrf)())).sprite)(record.0.as_ptr()) },
        subtype: record.subtype(),
        ambient: u8::from(unsafe { (*services).settings().ambient.read() }),
    };
    let context = record.shell();
    let mut controller = Controller {
        owner,
        context,
        record,
        leaves,
        services: unsafe { &*services },
        v,
    };
    u8::from(controller.tick())
}
/// Initialize one newly allocated effect through the same owner used by creation.
/// # Safety
/// The live owner, shell and immutable tables obey `openttd_rust_effect_run`'s contract.
#[allow(unsafe_code)]
unsafe fn initialize(
    owner: *mut State,
    record: VehicleRef,
    kind: u8,
    leaves: &Leaves,
    services: &Services,
) {
    let mut controller = Controller {
        owner,
        context: record.shell(),
        record,
        leaves,
        services,
        v: View {
            subtype: kind,
            ..View::default()
        },
    };
    controller.init();
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEffectLeaves" ["probe"] => Leaves {
        vehicles,
        sprite_sequence, sprite, spritenum,
        viewport,
        sound,

    };
"OpenTTDEffectAllocation" ["probe"] => Allocation { shell, record, owner, };
    "OpenTTDEffectCreationServices" ["probe"] => CreationServices {
        effect, shared, allocate, owner,
    };

}

/// Creation owns the allocation gate, field/initializer/viewport order and offsets.
/// Services are immutable for the process. Native pools and shells remain canonical;
/// creation callbacks cannot reset the map or delete a newly allocated effect.
#[repr(C)]
struct Allocation {
    shell: *mut c_void,
    record: VehicleRef,
    owner: *mut State,
}
#[repr(C)]
pub struct CreationServices {
    effect: &'static Leaves,
    shared: &'static Services,
    allocate: extern "C" fn() -> Allocation,
    owner: extern "C" fn(*mut c_void) -> *mut State,
}
static CREATION: std::sync::atomic::AtomicPtr<CreationServices> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
/// Bind immutable process-wide services before vehicle initialization or native creation.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn openttd_rust_effect_bind(
    services: *const CreationServices,
) -> *const CreationServices {
    CREATION
        .swap(services.cast_mut(), std::sync::atomic::Ordering::AcqRel)
        .cast_const()
}
#[allow(unsafe_code)]
fn creation() -> &'static CreationServices {
    unsafe { &*CREATION.load(std::sync::atomic::Ordering::Acquire) }
}
/// `CreateEffectVehicle` over the shared native pool.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn openttd_rust_effect_create(x: i32, y: i32, z: i32, kind: u8) -> *mut c_void {
    let s = creation();
    // SAFETY: The bound leaves name the process-wide canonical vehicle table.
    let vehicles = unsafe { &*s.effect.vehicles };
    if !(vehicles.can_allocate)(1) {
        return std::ptr::null_mut();
    }
    let v = (s.allocate)();
    crate::raw_field::write_field!(v.record.0.as_ptr(), subtype, kind);
    v.record.write_x(x);
    v.record.write_y(y);
    v.record.write_z(z);
    v.record.write_tile(0);
    (vehicles.update_delta)(v.record);
    v.record.write_status(crate::vehicle_base::VS_UNCLICKABLE);
    unsafe {
        initialize(v.owner, v.record, kind, s.effect, s.shared);
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
    let safe_x = x.clamp(
        0,
        ((s.shared.map_max_x)() * crate::tile_type::TILE_SIZE) as i32,
    );
    let safe_y = y.clamp(
        0,
        ((s.shared.map_max_y)() * crate::tile_type::TILE_SIZE) as i32,
    );
    openttd_rust_effect_create(
        x,
        y,
        crate::movement::get_slope_pixel_z(s.shared, safe_x, safe_y, false).wrapping_add(z),
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
    let record = VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast_mut()) });
    openttd_rust_effect_create(
        record.x().wrapping_add(x),
        record.y().wrapping_add(y),
        record.z().wrapping_add(z),
        kind,
    )
}

/// `EffectVehicle::SetAnimationState` on a live created shell.
#[allow(unsafe_code)]
pub(crate) fn set_animation(shell: *mut c_void, duration: u16) {
    let owner = (creation().owner)(shell);
    unsafe {
        openttd_rust_effect_set_animation(owner, duration);
    }
}

/// `EffectVehicle::SetAnimationSubstate` on a live created shell.
#[allow(unsafe_code)]
pub(crate) fn set_substate(shell: *mut c_void, value: u8) {
    let owner = (creation().owner)(shell);
    unsafe {
        openttd_rust_effect_set_substate(owner, value);
    }
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `effectvehicle.cpp`: the effect vehicles' init and tick procedures, their creation, and
//! the Rust owner of the two members `EffectVehicle` adds to `Vehicle`.
//!
//! Each native `EffectVehicle` shell owns one [`State`], allocated with the shell and freed
//! by its destructor. An [`EffectVehicle`] handle names the shell's shared record and that
//! owner. Every access to the owner is one field load or store, and no reference to it
//! lives across a service call. A tick procedure that ends its effect returns false where
//! the original has `delete v; return false;`, and `RunEffect` (`src/effectvehicle.cpp`)
//! deletes the shell after the export returns, with nothing run in between.
#![allow(clippy::cast_possible_truncation, clippy::verbose_bit_mask)]

use crate::bitmath_func::gb;
use crate::effectvehicle_func::EffectVehicleType;
use crate::gfx_type::SpriteID;
use crate::industry_type::IndustryGfx;
use crate::random_func::chance16_i;
use crate::services::Services;
use crate::sound_type::{SoundFx, SoundID};
use crate::sprites::{
    SPR_BREAKDOWN_SMOKE_0, SPR_BREAKDOWN_SMOKE_3, SPR_BUBBLE_0, SPR_BUBBLE_GENERATE_0,
    SPR_BUBBLE_GENERATE_3, SPR_BULLDOZER_NE, SPR_CHIMNEY_SMOKE_0, SPR_CHIMNEY_SMOKE_7,
    SPR_DIESEL_SMOKE_0, SPR_DIESEL_SMOKE_5, SPR_ELECTRIC_SPARK_0, SPR_ELECTRIC_SPARK_5,
    SPR_EXPLOSION_LARGE_0, SPR_EXPLOSION_LARGE_F, SPR_EXPLOSION_SMALL_0, SPR_EXPLOSION_SMALL_B,
    SPR_SMOKE_0, SPR_SMOKE_4, SPR_STEAM_SMOKE_0, SPR_STEAM_SMOKE_4,
};
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
use crate::vehicle::{VehicleRef, VehicleServices};
use std::ffi::c_void;

/// The members `EffectVehicle` adds to `Vehicle` (`effectvehicle_base.h`).
#[derive(Default)]
pub struct State {
    /// State primarily used to change the graphics/behaviour.
    animation_state: u16,
    /// Sub state to time the change of the graphics/behaviour.
    animation_substate: u8,
}

/// `EffectVehicle *`: the shared record of one effect vehicle and the owner its shell holds.
#[derive(Clone, Copy)]
pub(crate) struct EffectVehicle {
    record: VehicleRef,
    owner: *mut State,
}

/// The tables an effect procedure calls: the process-wide vehicle and shared tables, or a
/// fixture's.
struct Tables<'a> {
    vehicles: &'a VehicleServices,
    shared: &'a Services,
}

impl EffectVehicle {
    /// `animation_state`.
    #[allow(unsafe_code)]
    fn animation_state(self) -> u16 {
        // SAFETY: The shell's live owner (module contract).
        unsafe { (*self.owner).animation_state }
    }
    /// `EffectVehicle::SetAnimationState`: `animation_state = state`.
    #[allow(unsafe_code)]
    pub(crate) fn set_animation_state(self, state: u16) {
        // SAFETY: The shell's live owner (module contract).
        unsafe { (*self.owner).animation_state = state };
    }
    /// `animation_substate`.
    #[allow(unsafe_code)]
    fn animation_substate(self) -> u8 {
        // SAFETY: The shell's live owner (module contract).
        unsafe { (*self.owner).animation_substate }
    }
    /// `EffectVehicle::SetAnimationSubstate`: `animation_substate = state`.
    #[allow(unsafe_code)]
    pub(crate) fn set_animation_substate(self, state: u8) {
        // SAFETY: The shell's live owner (module contract).
        unsafe { (*self.owner).animation_substate = state };
    }
    /// `spritenum`, through the resolver table's reader.
    #[allow(unsafe_code)]
    fn spritenum(self, t: &Tables) -> u8 {
        // SAFETY: The live record of the shell.
        unsafe { (t.vehicles.newgrf().sprite)(self.record.0.as_ptr()) }
    }
    /// `spritenum = sprite`.
    fn set_spritenum(self, t: &Tables, sprite: u8) {
        (t.vehicles.write_spritenum)(self.record, sprite);
    }
    /// `sprite_cache.sprite_seq.seq[0].sprite`.
    fn first_sprite(self, t: &Tables) -> SpriteID {
        (t.vehicles.first_sprite)(self.record)
    }
    /// `sprite_cache.sprite_seq.seq[0].sprite = sprite`.
    fn set_first_sprite(self, t: &Tables, sprite: SpriteID) {
        (t.vehicles.write_first_sprite)(self.record, sprite);
    }
    /// `sprite_cache.sprite_seq.Set(sprite)`.
    fn set_sprite_seq(self, t: &Tables, sprite: SpriteID) {
        (t.vehicles.write_sprite_seq)(self.record, sprite);
    }
    /// `UpdatePositionAndViewport()`.
    fn update_position_and_viewport(self, t: &Tables) {
        t.vehicles
            .update_position_and_viewport(self.record, || t.shared.map());
    }
    /// `TileVirtXY(v->x_pos, v->y_pos)`, the coordinates converted to its `uint` parameters.
    fn tile_virt_xy(self, t: &Tables) -> TileIndex {
        t.shared.map().tile_virt_xy(
            self.record.x_pos().cast_unsigned(),
            self.record.y_pos().cast_unsigned(),
        )
    }
    /// The effect vehicle's native shell.
    pub(crate) fn shell(self) -> *mut c_void {
        self.record.shell()
    }
    /// `EffectVehicle::Tick`.
    fn tick(self, t: &Tables) -> bool {
        (EFFECT_PROCS[usize::from(self.record.subtype())].tick_proc)(self, t)
    }
}

/// `IncrementSprite`: increment the sprite unless it has reached the end of the animation.
/// `last` is the last sprite of the animation. Returns true if the sprite was incremented,
/// false if the end was reached.
fn increment_sprite(v: EffectVehicle, t: &Tables, last: SpriteID) -> bool {
    let sprite = v.first_sprite(t);
    if sprite == last {
        return false;
    }
    v.set_first_sprite(t, sprite.wrapping_add(1));
    true
}

/// `ChimneySmokeInit`.
fn chimney_smoke_init(v: EffectVehicle, t: &Tables) {
    let r = t.shared.random();
    v.set_sprite_seq(t, SPR_CHIMNEY_SMOKE_0 + gb(r, 0, 3));
    v.record.set_progress(gb(r, 16, 3) as u8);
}

/// `ChimneySmokeTick`.
fn chimney_smoke_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress();
    if progress > 0 {
        v.record.set_progress(progress - 1);
    } else {
        let tile = v.tile_virt_xy(t);
        if !crate::tile_map::is_tile_type(t.shared, tile, TileType::MP_INDUSTRY) {
            return false;
        }

        if !increment_sprite(v, t, SPR_CHIMNEY_SMOKE_7) {
            v.set_sprite_seq(t, SPR_CHIMNEY_SMOKE_0);
        }
        v.record.set_progress(7);
        v.update_position_and_viewport(t);
    }

    true
}

/// `SteamSmokeInit`.
fn steam_smoke_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_STEAM_SMOKE_0);
    v.record.set_progress(12);
}

/// `SteamSmokeTick`.
fn steam_smoke_tick(v: EffectVehicle, t: &Tables) -> bool {
    let mut moved = false;

    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);

    if (progress & 7) == 0 {
        v.record.set_z_pos(v.record.z_pos().wrapping_add(1));
        moved = true;
    }

    if (progress & 0xF) == 4 {
        if !increment_sprite(v, t, SPR_STEAM_SMOKE_4) {
            return false;
        }
        moved = true;
    }

    if moved {
        v.update_position_and_viewport(t);
    }

    true
}

/// `DieselSmokeInit`.
fn diesel_smoke_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_DIESEL_SMOKE_0);
    v.record.set_progress(0);
}

/// `DieselSmokeTick`.
fn diesel_smoke_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);

    if (progress & 3) == 0 {
        v.record.set_z_pos(v.record.z_pos().wrapping_add(1));
        v.update_position_and_viewport(t);
    } else if (progress & 7) == 1 {
        if !increment_sprite(v, t, SPR_DIESEL_SMOKE_5) {
            return false;
        }
        v.update_position_and_viewport(t);
    }

    true
}

/// `ElectricSparkInit`.
fn electric_spark_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_ELECTRIC_SPARK_0);
    v.record.set_progress(1);
}

/// `ElectricSparkTick`.
fn electric_spark_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress();
    if progress < 2 {
        v.record.set_progress(progress + 1);
    } else {
        v.record.set_progress(0);

        if !increment_sprite(v, t, SPR_ELECTRIC_SPARK_5) {
            return false;
        }
        v.update_position_and_viewport(t);
    }

    true
}

/// `SmokeInit`.
fn smoke_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_SMOKE_0);
    v.record.set_progress(12);
}

/// `SmokeTick`.
fn smoke_tick(v: EffectVehicle, t: &Tables) -> bool {
    let mut moved = false;

    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);

    if (progress & 3) == 0 {
        v.record.set_z_pos(v.record.z_pos().wrapping_add(1));
        moved = true;
    }

    if (progress & 0xF) == 4 {
        if !increment_sprite(v, t, SPR_SMOKE_4) {
            return false;
        }
        moved = true;
    }

    if moved {
        v.update_position_and_viewport(t);
    }

    true
}

/// `ExplosionLargeInit`.
fn explosion_large_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_EXPLOSION_LARGE_0);
    v.record.set_progress(0);
}

/// `ExplosionLargeTick`.
fn explosion_large_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);
    if (progress & 3) == 0 {
        if !increment_sprite(v, t, SPR_EXPLOSION_LARGE_F) {
            return false;
        }
        v.update_position_and_viewport(t);
    }

    true
}

/// `BreakdownSmokeInit`.
fn breakdown_smoke_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_BREAKDOWN_SMOKE_0);
    v.record.set_progress(0);
}

/// `BreakdownSmokeTick`.
fn breakdown_smoke_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);
    if (progress & 7) == 0 {
        if !increment_sprite(v, t, SPR_BREAKDOWN_SMOKE_3) {
            v.set_sprite_seq(t, SPR_BREAKDOWN_SMOKE_0);
        }
        v.update_position_and_viewport(t);
    }

    let animation_state = v.animation_state().wrapping_sub(1);
    v.set_animation_state(animation_state);
    if animation_state == 0 {
        return false;
    }

    true
}

/// `ExplosionSmallInit`.
fn explosion_small_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_EXPLOSION_SMALL_0);
    v.record.set_progress(0);
}

/// `ExplosionSmallTick`.
fn explosion_small_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);
    if (progress & 3) == 0 {
        if !increment_sprite(v, t, SPR_EXPLOSION_SMALL_B) {
            return false;
        }
        v.update_position_and_viewport(t);
    }

    true
}

/// `BulldozerInit`.
fn bulldozer_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_BULLDOZER_NE);
    v.record.set_progress(0);
    v.set_animation_state(0);
    v.set_animation_substate(0);
}

/// `BulldozerMovement`, whose `direction:2`, `image:2` and `duration:3` bit fields are
/// whole bytes here.
struct BulldozerMovement {
    direction: u8,
    image: u8,
    duration: u8,
}

/// `_bulldozer_movement`.
static BULLDOZER_MOVEMENT: [BulldozerMovement; 20] = {
    const fn m(direction: u8, image: u8, duration: u8) -> BulldozerMovement {
        BulldozerMovement {
            direction,
            image,
            duration,
        }
    }
    [
        m(0, 0, 4),
        m(3, 3, 4),
        m(2, 2, 7),
        m(0, 2, 7),
        m(1, 1, 3),
        m(2, 2, 7),
        m(0, 2, 7),
        m(1, 1, 3),
        m(2, 2, 7),
        m(0, 2, 7),
        m(3, 3, 6),
        m(2, 2, 6),
        m(1, 1, 7),
        m(3, 1, 7),
        m(0, 0, 3),
        m(1, 1, 7),
        m(3, 1, 7),
        m(0, 0, 3),
        m(1, 1, 7),
        m(3, 1, 7),
    ]
};

/// An `_inc_by_dir` entry.
struct IncByDir {
    x: i8,
    y: i8,
}

/// `_inc_by_dir`.
static INC_BY_DIR: [IncByDir; 4] = [
    IncByDir { x: -1, y: 0 },
    IncByDir { x: 0, y: 1 },
    IncByDir { x: 1, y: 0 },
    IncByDir { x: 0, y: -1 },
];

/// `BulldozerTick`.
fn bulldozer_tick(v: EffectVehicle, t: &Tables) -> bool {
    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);
    if (progress & 7) == 0 {
        let b = &BULLDOZER_MOVEMENT[usize::from(v.animation_state())];

        v.set_sprite_seq(t, SPR_BULLDOZER_NE + SpriteID::from(b.image));

        let inc = &INC_BY_DIR[usize::from(b.direction)];
        v.record
            .set_x_pos(v.record.x_pos().wrapping_add(i32::from(inc.x)));
        v.record
            .set_y_pos(v.record.y_pos().wrapping_add(i32::from(inc.y)));

        let animation_substate = v.animation_substate().wrapping_add(1);
        v.set_animation_substate(animation_substate);
        if animation_substate >= b.duration {
            v.set_animation_substate(0);
            let animation_state = v.animation_state().wrapping_add(1);
            v.set_animation_state(animation_state);
            if usize::from(animation_state) == BULLDOZER_MOVEMENT.len() {
                return false;
            }
        }
        v.update_position_and_viewport(t);
    }

    true
}

/// `BubbleInit`.
fn bubble_init(v: EffectVehicle, t: &Tables) {
    v.set_sprite_seq(t, SPR_BUBBLE_GENERATE_0);
    v.set_spritenum(t, 0);
    v.record.set_progress(0);
}

/// `BubbleMovement`, whose signed `x:4`, `y:4` and `z:4` and unsigned `image:4` bit fields
/// are whole bytes here.
#[derive(Clone, Copy)]
struct BubbleMovement {
    x: i8,
    y: i8,
    z: i8,
    image: u8,
}

/// `MK(x, y, z, i)`.
const fn mk(x: i8, y: i8, z: i8, i: u8) -> BubbleMovement {
    BubbleMovement { x, y, z, image: i }
}
/// `ME(i)`: the end of a movement, whose `x` says what follows.
const fn me(i: i8) -> BubbleMovement {
    BubbleMovement {
        x: i,
        y: 4,
        z: 0,
        image: 0,
    }
}

/// `_bubble_float_sw`.
static BUBBLE_FLOAT_SW: [BubbleMovement; 5] = [
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 2),
    me(1),
];

/// `_bubble_float_ne`.
static BUBBLE_FLOAT_NE: [BubbleMovement; 5] = [
    mk(0, 0, 1, 0),
    mk(-1, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(-1, 0, 1, 2),
    me(1),
];

/// `_bubble_float_se`.
static BUBBLE_FLOAT_SE: [BubbleMovement; 5] = [
    mk(0, 0, 1, 0),
    mk(0, 1, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 1, 1, 2),
    me(1),
];

/// `_bubble_float_nw`.
static BUBBLE_FLOAT_NW: [BubbleMovement; 5] = [
    mk(0, 0, 1, 0),
    mk(0, -1, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, -1, 1, 2),
    me(1),
];

/// `_bubble_burst`.
static BUBBLE_BURST: [BubbleMovement; 5] = [
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 7),
    mk(0, 0, 1, 8),
    mk(0, 0, 1, 9),
    me(0),
];

/// `_bubble_absorb`.
static BUBBLE_ABSORB: [BubbleMovement; 85] = [
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(0, 0, 1, 1),
    mk(2, 1, 3, 0),
    mk(1, 1, 3, 1),
    mk(2, 1, 3, 0),
    mk(1, 1, 3, 2),
    mk(2, 1, 3, 0),
    mk(1, 1, 3, 1),
    mk(2, 1, 3, 0),
    mk(1, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 2),
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 1),
    mk(0, 0, 1, 0),
    mk(1, 0, 1, 2),
    me(2),
    mk(0, 0, 0, 0xA),
    mk(0, 0, 0, 0xB),
    mk(0, 0, 0, 0xC),
    mk(0, 0, 0, 0xD),
    mk(0, 0, 0, 0xE),
    me(0),
];

/// `_bubble_movement`.
static BUBBLE_MOVEMENT: [&[BubbleMovement]; 6] = [
    &BUBBLE_FLOAT_SW,
    &BUBBLE_FLOAT_NE,
    &BUBBLE_FLOAT_SE,
    &BUBBLE_FLOAT_NW,
    &BUBBLE_BURST,
    &BUBBLE_ABSORB,
];

/// `BubbleTick`.
fn bubble_tick(v: EffectVehicle, t: &Tables) -> bool {
    let mut anim_state: u32;

    let progress = v.record.progress().wrapping_add(1);
    v.record.set_progress(progress);
    if (progress & 3) != 0 {
        return true;
    }

    let mut spritenum = v.spritenum(t);
    if spritenum == 0 {
        let sprite = v.first_sprite(t).wrapping_add(1);
        v.set_first_sprite(t, sprite);
        if sprite < SPR_BUBBLE_GENERATE_3 {
            v.update_position_and_viewport(t);
            return true;
        }
        spritenum = if v.animation_substate() != 0 {
            (gb(t.shared.random(), 0, 2) + 1) as u8
        } else {
            6
        };
        v.set_spritenum(t, spritenum);
        anim_state = 0;
    } else {
        anim_state = u32::from(v.animation_state()) + 1;
    }

    let b = &BUBBLE_MOVEMENT[usize::from(spritenum) - 1][anim_state as usize];

    if b.y == 4 && b.x == 0 {
        return false;
    }

    if b.y == 4 && b.x == 1 {
        if v.record.z_pos() > 180 || chance16_i(1, 96, t.shared.random()) {
            spritenum = 5;
            v.set_spritenum(t, spritenum);
            if sound_ambient(t) {
                snd_play_vehicle_fx(SoundFx::SND_2F_BUBBLE_GENERATOR_FAIL, v, t);
            }
        }
        anim_state = 0;
    }

    if b.y == 4 && b.x == 2 {
        anim_state += 1;
        if sound_ambient(t) {
            snd_play_vehicle_fx(SoundFx::SND_31_BUBBLE_GENERATOR_SUCCESS, v, t);
        }

        let tile = v.tile_virt_xy(t);
        if crate::tile_map::is_tile_type(t.shared, tile, TileType::MP_INDUSTRY)
            && crate::industry_map::get_industry_gfx(t.shared, tile)
                == IndustryGfx::GFX_BUBBLE_CATCHER
        {
            crate::animated_tile::add_animated_tile(tile, true);
        }
    }

    v.set_animation_state(anim_state as u16);
    let b = &BUBBLE_MOVEMENT[usize::from(spritenum) - 1][anim_state as usize];

    v.record
        .set_x_pos(v.record.x_pos().wrapping_add(i32::from(b.x)));
    v.record
        .set_y_pos(v.record.y_pos().wrapping_add(i32::from(b.y)));
    v.record
        .set_z_pos(v.record.z_pos().wrapping_add(i32::from(b.z)));
    v.set_sprite_seq(t, SPR_BUBBLE_0 + SpriteID::from(b.image));

    v.update_position_and_viewport(t);

    true
}

/// `_settings_client.sound.ambient`, one field-sized load.
#[allow(unsafe_code)]
fn sound_ambient(t: &Tables) -> bool {
    // SAFETY: The catalog addresses the process-wide setting, serialized by the game-state
    // mutex.
    unsafe { t.shared.settings().ambient.read() }
}

/// `SndPlayVehicleFx(sound, v)`.
fn snd_play_vehicle_fx(sound: SoundFx, v: EffectVehicle, t: &Tables) {
    (t.vehicles.play_sound)(v.record, SoundID::from(sound).0);
}

/// `EffectProcs`: the init and tick procedures of one effect vehicle type. The original's
/// third member, the transparency option, is drawing and stays in
/// `EffectVehicle::GetTransparencyOption` (`src/effectvehicle.cpp`).
struct EffectProcs {
    /// Function to initialise an effect vehicle after construction.
    init_proc: fn(EffectVehicle, &Tables),
    /// Functions for controlling effect vehicles at each tick.
    tick_proc: fn(EffectVehicle, &Tables) -> bool,
}

/// Per-`EffectVehicleType` handling.
static EFFECT_PROCS: [EffectProcs; EffectVehicleType::EV_END.0 as usize] = {
    const fn p(
        init_proc: fn(EffectVehicle, &Tables),
        tick_proc: fn(EffectVehicle, &Tables) -> bool,
    ) -> EffectProcs {
        EffectProcs {
            init_proc,
            tick_proc,
        }
    }
    [
        p(chimney_smoke_init, chimney_smoke_tick), // EV_CHIMNEY_SMOKE
        p(steam_smoke_init, steam_smoke_tick),     // EV_STEAM_SMOKE
        p(diesel_smoke_init, diesel_smoke_tick),   // EV_DIESEL_SMOKE
        p(electric_spark_init, electric_spark_tick), // EV_ELECTRIC_SPARK
        p(smoke_init, smoke_tick),                 // EV_CRASH_SMOKE
        p(explosion_large_init, explosion_large_tick), // EV_EXPLOSION_LARGE
        p(breakdown_smoke_init, breakdown_smoke_tick), // EV_BREAKDOWN_SMOKE
        p(explosion_small_init, explosion_small_tick), // EV_EXPLOSION_SMALL
        p(bulldozer_init, bulldozer_tick),         // EV_BULLDOZER
        p(bubble_init, bubble_tick),               // EV_BUBBLE
        p(smoke_init, smoke_tick),                 // EV_BREAKDOWN_SMOKE_AIRCRAFT
        p(smoke_init, smoke_tick),                 // EV_COPPER_MINE_SMOKE
    ]
};

/// The services creation calls: how the native vehicle pool allocates an effect vehicle,
/// and the tables its procedures call. `src/vehicle.cpp` binds the process-wide table.
#[repr(C)]
pub struct CreationServices {
    allocate: extern "C" fn() -> Allocation,
    shared: &'static Services,
    vehicles: &'static VehicleServices,
}
/// `new EffectVehicle()`: the new shell's record and the owner it holds.
#[repr(C)]
struct Allocation {
    record: VehicleRef,
    owner: *mut State,
}
static CREATION: std::sync::atomic::AtomicPtr<CreationServices> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
/// Bind the process-wide creation services, returning the previous binding.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_bind(
    services: *const CreationServices,
) -> *const CreationServices {
    CREATION
        .swap(services.cast_mut(), std::sync::atomic::Ordering::AcqRel)
        .cast_const()
}
#[allow(unsafe_code)]
fn creation() -> &'static CreationServices {
    // SAFETY: The bound table is immutable and outlives every creation.
    unsafe { &*CREATION.load(std::sync::atomic::Ordering::Acquire) }
}

/// `CreateEffectVehicle`: create an effect vehicle at a particular location (`x`, `y`, `z`)
/// of the given `type`. Returns the effect vehicle, or `None` where the original returns
/// `nullptr`.
pub(crate) fn create_effect_vehicle(
    x: i32,
    y: i32,
    z: i32,
    r#type: EffectVehicleType,
) -> Option<EffectVehicle> {
    let services = creation();
    let tables = Tables {
        vehicles: services.vehicles,
        shared: services.shared,
    };
    if !(tables.vehicles.can_allocate)(1) {
        return None;
    }

    let allocation = (services.allocate)();
    let v = EffectVehicle {
        record: allocation.record,
        owner: allocation.owner,
    };
    v.record.set_subtype(r#type.0);
    v.record.set_x_pos(x);
    v.record.set_y_pos(y);
    v.record.set_z_pos(z);
    v.record.set_tile(TileIndex(0));
    (tables.vehicles.update_delta)(v.record);
    v.record
        .set_vehstatus(crate::vehicle_base::VehStates::from_element(
            crate::vehicle_base::VehState::Unclickable,
        ));

    (EFFECT_PROCS[usize::from(r#type.0)].init_proc)(v, &tables);

    v.update_position_and_viewport(&tables);

    Some(v)
}

/// `CreateEffectVehicleAbove`: create an effect vehicle above a particular location; `z`
/// is the offset from the ground.
pub(crate) fn create_effect_vehicle_above(
    x: i32,
    y: i32,
    z: i32,
    r#type: EffectVehicleType,
) -> Option<EffectVehicle> {
    let shared = creation().shared;
    let world = crate::map_func::World::new(shared);
    let check = |condition| world.check(condition);
    let safe_x = crate::math::clamp(
        x,
        0,
        world
            .dimensions
            .max_x()
            .wrapping_mul(TILE_SIZE)
            .cast_signed(),
        check,
    );
    let safe_y = crate::math::clamp(
        y,
        0,
        crate::map_func::Map::max_y(world.size_y())
            .wrapping_mul(TILE_SIZE)
            .cast_signed(),
        check,
    );
    create_effect_vehicle(
        x,
        y,
        crate::movement::get_slope_pixel_z(shared, safe_x, safe_y, false).wrapping_add(z),
        r#type,
    )
}

/// `CreateEffectVehicleRel`: create an effect vehicle above a particular vehicle `v`; `x`,
/// `y` and `z` are the offsets to the vehicle.
pub(crate) fn create_effect_vehicle_rel(
    v: VehicleRef,
    x: i32,
    y: i32,
    z: i32,
    r#type: EffectVehicleType,
) -> Option<EffectVehicle> {
    create_effect_vehicle(
        v.x_pos().wrapping_add(x),
        v.y_pos().wrapping_add(y),
        v.z_pos().wrapping_add(z),
        r#type,
    )
}

/// The shell of a created effect vehicle, or null.
fn shell_or_null(v: Option<EffectVehicle>) -> *mut c_void {
    v.map_or(std::ptr::null_mut(), EffectVehicle::shell)
}
/// `CreateEffectVehicle`, for `src/effectvehicle.cpp`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_create(x: i32, y: i32, z: i32, r#type: u8) -> *mut c_void {
    shell_or_null(create_effect_vehicle(x, y, z, EffectVehicleType(r#type)))
}
/// `CreateEffectVehicleAbove`, for `src/effectvehicle.cpp`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_create_above(
    x: i32,
    y: i32,
    z: i32,
    r#type: u8,
) -> *mut c_void {
    shell_or_null(create_effect_vehicle_above(
        x,
        y,
        z,
        EffectVehicleType(r#type),
    ))
}
/// `CreateEffectVehicleRel`, for `src/effectvehicle.cpp`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_create_rel(
    v: VehicleRef,
    x: i32,
    y: i32,
    z: i32,
    r#type: u8,
) -> *mut c_void {
    shell_or_null(create_effect_vehicle_rel(
        v,
        x,
        y,
        z,
        EffectVehicleType(r#type),
    ))
}

/// `EffectVehicle::Tick` for the shell that holds `owner`, over the given tables. Returns
/// 0 where the original deletes the effect, which the caller then does.
///
/// # Safety
/// `owner` is the live owner `record`'s shell holds, and both tables are live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_run(
    owner: *mut State,
    record: VehicleRef,
    vehicles: *const VehicleServices,
    shared: *const Services,
) -> u8 {
    // SAFETY: The caller's live tables.
    let t = unsafe {
        Tables {
            vehicles: &*vehicles,
            shared: &*shared,
        }
    };
    u8::from(EffectVehicle { record, owner }.tick(&t))
}

/// A new shell's zeroed owner.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_effect_new() -> *mut State {
    Box::into_raw(Box::default())
}
/// # Safety
/// `owner` came from `openttd_rust_effect_new` and is destroyed once, after the shell's last
/// use of it.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_destroy(owner: *mut State) {
    // SAFETY: The caller's owner, destroyed once.
    drop(unsafe { Box::from_raw(owner) });
}
/// `animation_state = value`, for the shell and its save/load.
///
/// # Safety
/// `owner` is live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_set_animation(owner: *mut State, value: u16) {
    // SAFETY: The caller's live owner.
    unsafe { (*owner).animation_state = value };
}
/// `animation_state`, for the shell and its save/load.
///
/// # Safety
/// `owner` is live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_get_animation(owner: *const State) -> u16 {
    // SAFETY: The caller's live owner.
    unsafe { (*owner).animation_state }
}
/// `animation_substate = value`, for the shell and its save/load.
///
/// # Safety
/// `owner` is live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_set_substate(owner: *mut State, value: u8) {
    // SAFETY: The caller's live owner.
    unsafe { (*owner).animation_substate = value };
}
/// `animation_substate`, for the shell and its save/load.
///
/// # Safety
/// `owner` is live.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_effect_get_substate(owner: *const State) -> u8 {
    // SAFETY: The caller's live owner.
    unsafe { (*owner).animation_substate }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEffectAllocation" ["probe"] => Allocation {
        record,
        owner,
    };
    "OpenTTDEffectCreationServices" ["probe"] => CreationServices {
        allocate,
        shared,
        vehicles,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use crate::vehicle::OpenTTDVehicle;
    use crate::vehicle_base::VehicleRecord;
    use crate::vehicle_type::VehicleType;
    use std::cell::{Cell, RefCell};

    /// What one test's services observed.
    #[derive(Default)]
    struct Observed {
        /// The tiles `industry_gfx` was asked for.
        industry_gfx: Vec<u32>,
        /// The sounds `play_sound` played.
        sounds: Vec<u16>,
        /// The number of random draws.
        draws: u32,
        /// The `spritenum` the record holds.
        spritenum: u8,
        /// The first sprite the record holds.
        sprite: SpriteID,
    }
    thread_local! {
        static OBSERVED: RefCell<Observed> = RefCell::default();
        static RANDOM: Cell<u32> = const { Cell::new(0) };
        static AMBIENT: Cell<bool> = const { Cell::new(false) };
    }
    fn observe<R>(f: impl FnOnce(&mut Observed) -> R) -> R {
        OBSERVED.with(|o| f(&mut o.borrow_mut()))
    }

    extern "C" fn random(_: *mut c_void) -> u32 {
        observe(|o| o.draws += 1);
        RANDOM.get()
    }
    extern "C" fn industry_gfx(tile: u32) -> u16 {
        observe(|o| o.industry_gfx.push(tile));
        IndustryGfx::GFX_BUBBLE_GENERATOR.0
    }
    extern "C" fn play_sound(_: VehicleRef, sound: u16) {
        observe(|o| o.sounds.push(sound));
    }
    extern "C" fn viewport(_: VehicleRef) {}
    extern "C" fn update_delta(_: VehicleRef) {}
    extern "C" fn first_sprite(_: VehicleRef) -> SpriteID {
        observe(|o| o.sprite)
    }
    extern "C" fn write_sprite(_: VehicleRef, sprite: SpriteID) {
        observe(|o| o.sprite = sprite);
    }
    extern "C" fn write_spritenum(_: VehicleRef, sprite: u8) {
        observe(|o| o.spritenum = sprite);
    }
    #[allow(unsafe_code)]
    unsafe extern "C" fn spritenum(_: *mut OpenTTDVehicle) -> u8 {
        observe(|o| o.spritenum)
    }
    extern "C" fn resolver() -> *const crate::newgrf_vehicle::Services {
        let mut resolver = crate::newgrf_vehicle::TEST_TABLE;
        resolver.sprite = spritenum;
        Box::leak(Box::new(resolver))
    }

    /// A 64 by 64 map of clear tiles, installed for this thread, with `industry` the one
    /// industry tile, and the tables a procedure calls over it.
    fn tables(industry: Option<u32>) -> Tables<'static> {
        const SIZE: u32 = 64 * 64;
        let base: &'static mut [BaseTileLayout] = Box::leak(
            (0..SIZE)
                .map(|tile| BaseTileLayout {
                    kind: if Some(tile) == industry {
                        TileType::MP_INDUSTRY.0 << 4
                    } else {
                        TileType::MP_CLEAR.0 << 4
                    },
                    height: 0,
                    m2: 0,
                    m1: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect(),
        );
        let extended: &'static mut [ExtendedTileLayout] = Box::leak(
            (0..SIZE)
                .map(|_| ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect(),
        );
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: base.as_ptr(),
                extended: extended.as_ptr(),
                count: SIZE,
                assertions: true,
            },
            6,
        );
        let mut settings = crate::settings_type::SettingAddresses::NULL;
        settings.ambient = AMBIENT.with(Cell::as_ptr);
        let mut shared = crate::services::fixture(std::ptr::null_mut(), random);
        shared.map = crate::map_func::fixture::statics();
        shared.industry_gfx = industry_gfx;
        shared.settings = Box::leak(Box::new(settings));
        Tables {
            vehicles: Box::leak(Box::new(VehicleServices {
                base_viewport: viewport,
                can_allocate: crate::test_tables::unreachable1,
                first_sprite,
                newgrf: resolver,
                play_sound,
                update_delta,
                write_first_sprite: write_sprite,
                write_sprite_seq: write_sprite,
                write_spritenum,
                ..crate::test_tables::VEHICLE
            })),
            shared: Box::leak(Box::new(shared)),
        }
    }

    /// A bubble at (`x`, `y`, `z`) whose next movement entry is `_bubble_movement[spritenum
    /// - 1][animation_state + 1]`, at a progress that moves it on this tick.
    fn bubble(x: i32, y: i32, z: i32, spritenum: u8, animation_state: u16) -> EffectVehicle {
        let record = VehicleRef::of(VehicleRecord::new(
            std::ptr::null_mut(),
            VehicleType::EFFECT,
        ));
        record.set_subtype(EffectVehicleType::EV_BUBBLE.0);
        record.set_x_pos(x);
        record.set_y_pos(y);
        record.set_z_pos(z);
        record.set_progress(3);
        observe(|o| {
            *o = Observed {
                spritenum,
                ..Observed::default()
            }
        });
        let v = EffectVehicle {
            record,
            owner: openttd_rust_effect_new(),
        };
        v.set_animation_state(animation_state);
        v
    }

    /// At the absorber's `ME(2)` entry, the catcher test reads the tile of (`x_pos`,
    /// `y_pos`) and asks for its graphics only when that tile is an industry. (0x35, 0x52)
    /// is on tile (3, 5), index 323; its transpose (5, 3) is index 197.
    #[test]
    fn bubble_tick_asks_for_the_industry_graphics_of_its_own_industry_tile() {
        AMBIENT.set(true);
        for (industry, asked) in [(None, vec![]), (Some(197), vec![]), (Some(323), vec![323])] {
            let t = tables(industry);
            let v = bubble(0x35, 0x52, 100, 6, 77);
            assert!(bubble_tick(v, &t));
            observe(|o| {
                assert_eq!(o.industry_gfx, asked);
                assert_eq!(
                    o.sounds,
                    [SoundID::from(SoundFx::SND_31_BUBBLE_GENERATOR_SUCCESS).0]
                );
                assert_eq!(o.sprite, SPR_BUBBLE_0 + 0xA);
            });
            assert_eq!(v.animation_state(), 79);
            assert_eq!(
                (v.record.x_pos(), v.record.y_pos(), v.record.z_pos()),
                (0x35, 0x52, 100)
            );
        }
    }

    /// With `ambient` off the bubble plays nothing at `ME(1)` and `ME(2)`, and `ME(1)`
    /// still draws its `Chance16I` number at `z_pos <= 180`.
    #[test]
    fn bubble_tick_without_ambient_sound_plays_nothing_and_still_draws() {
        AMBIENT.set(false);
        let t = tables(None);
        // `_bubble_float_sw[4]` is `ME(1)`; a zero draw bursts the bubble.
        RANDOM.set(0);
        let v = bubble(10, 20, 180, 1, 3);
        assert!(bubble_tick(v, &t));
        observe(|o| {
            assert_eq!(o.draws, 1);
            assert_eq!(o.spritenum, 5);
            assert!(o.sounds.is_empty());
        });
        // `_bubble_absorb[78]` is `ME(2)`.
        let v = bubble(10, 20, 100, 6, 77);
        assert!(bubble_tick(v, &t));
        observe(|o| {
            assert_eq!(o.draws, 0);
            assert!(o.sounds.is_empty());
        });
    }

    extern "C" fn unused_check(_: bool) {
        std::process::abort();
    }
    static ALLOCATED: std::sync::atomic::AtomicPtr<OpenTTDVehicle> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
    static OWNER: std::sync::atomic::AtomicPtr<State> =
        std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
    extern "C" fn allocate() -> Allocation {
        Allocation {
            record: VehicleRef::of(ALLOCATED.load(std::sync::atomic::Ordering::Relaxed)),
            owner: OWNER.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
    extern "C" fn cannot_allocate(count: usize) -> bool {
        assert_eq!(count, 1);
        false
    }
    extern "C" fn can_allocate(count: usize) -> bool {
        assert_eq!(count, 1);
        true
    }
    extern "C" fn unused_allocate() -> Allocation {
        std::process::abort();
    }

    /// Without room in the pool, creation returns `None` and calls nothing else: every
    /// other service of its tables aborts. Binds the process-wide creation table, so it
    /// runs in a child process.
    #[test]
    fn create_effect_vehicle_without_room_returns_none() {
        if crate::services_assert::isolate_test_check(
            "effect::tests::create_effect_vehicle_without_room_returns_none",
            unused_check,
        ) {
            return;
        }
        let creation: &'static CreationServices = Box::leak(Box::new(CreationServices {
            allocate: unused_allocate,
            shared: &crate::test_tables::SHARED,
            vehicles: Box::leak(Box::new(VehicleServices {
                can_allocate: cannot_allocate,
                ..crate::test_tables::VEHICLE
            })),
        }));
        openttd_rust_effect_bind(creation);
        assert!(create_effect_vehicle(1, 2, 3, EffectVehicleType::EV_ELECTRIC_SPARK).is_none());
    }

    /// `SetAnimationState` and `SetAnimationSubstate` on a created effect write the owner
    /// its shell holds, which the shell's readers read. Binds the process-wide creation
    /// table, so it runs in a child process.
    #[test]
    fn created_effect_animation_setters_write_the_shell_owner() {
        if crate::services_assert::isolate_test_check(
            "effect::tests::created_effect_animation_setters_write_the_shell_owner",
            unused_check,
        ) {
            return;
        }
        let t = tables(None);
        let shell = std::ptr::dangling_mut::<crate::vehicle_base::OpenTTDVehicleShell>();
        ALLOCATED.store(
            VehicleRecord::new(shell, VehicleType::EFFECT),
            std::sync::atomic::Ordering::Relaxed,
        );
        let owner = openttd_rust_effect_new();
        OWNER.store(owner, std::sync::atomic::Ordering::Relaxed);
        let creation: &'static CreationServices = Box::leak(Box::new(CreationServices {
            allocate,
            shared: t.shared,
            vehicles: Box::leak(Box::new(VehicleServices {
                can_allocate,
                ..*t.vehicles
            })),
        }));
        openttd_rust_effect_bind(creation);
        let v = create_effect_vehicle(1, 2, 3, EffectVehicleType::EV_ELECTRIC_SPARK).unwrap();
        assert_eq!(v.shell(), shell.cast());
        v.set_animation_state(0xBEEF);
        v.set_animation_substate(0xA5);
        // SAFETY: The owner the allocation returned, live until the test ends.
        #[allow(unsafe_code)]
        unsafe {
            assert_eq!(openttd_rust_effect_get_animation(owner), 0xBEEF);
            assert_eq!(openttd_rust_effect_get_substate(owner), 0xA5);
        }
    }
}

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `Vehicle::UpdateVisualEffect`, `SpawnAdvancedVisualEffect`, `IsBridgeAboveVehicle` and
//! `Vehicle::ShowVisualEffect` (`vehicle.cpp`): a vehicle's cached visual effect, and the
//! smoke and sparks its chain emits.
//!
//! A [`VehicleRef`] names a live record for the whole entry. Each field is one load or store
//! at the original's expression, and no reference into a record lives across a callback: the
//! `NewGRF` callbacks, effect creation and the sound can reenter resolvers, the effect pool and
//! the calling controller.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the original's conversions: uint8_t visual_effect = callback, the int8_t register fields GB(reg, 8, 8), int8_t x_center = _vehicle_smoke_pos[l_dir] * l_center, TileVirtXY's uint parameters and uint max_speed = GetCurrentMaxSpeed()"
)]

use crate::bitmath_func::{gb, has_bit, sb, set_bit, toggle_bit};
use crate::direction_func::{change_dir, reverse_dir};
use crate::direction_type::DirDiff;
use crate::effect::create_effect_vehicle_rel;
use crate::effectvehicle_func::EffectVehicleType;
use crate::engine_type::{
    EC_MONORAIL, EC_STEAM, VE_ADVANCED_EFFECT, VE_DEFAULT, VE_DISABLE_EFFECT,
    VE_DISABLE_WAGON_POWER, VE_OFFSET_CENTRE, VE_OFFSET_COUNT, VE_OFFSET_START, VE_TYPE_COUNT,
    VE_TYPE_DEFAULT, VE_TYPE_DIESEL, VE_TYPE_ELECTRIC, VE_TYPE_START, VE_TYPE_STEAM,
};
use crate::map_func::World;
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CallbackID, VehicleCallbackMask, VehicleCallbackMasks,
};
use crate::newgrf_config::GRFBug;
use crate::newgrf_sound::VehicleSoundEvent;
use crate::newgrf_vehicle::{VehicleCallback, get_vehicle_callback, resolve_vehicle_callback};
use crate::random_func::chance16;
use crate::services::Services;
use crate::services_assert::{assert_with, not_reached};
use crate::table::strings::{STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_POWERED_WAGON};
use crate::train::{Handle as Train, VehicleRailFlag};
use crate::vehicle::{RailVehicleTypes, VEHICLE_LENGTH, VehicleRef, VehicleServices};
use crate::vehicle_base::{VehState, VehStates, VisualEffectSpawnModel};
use crate::vehicle_type::VehicleType;
use std::mem::MaybeUninit;

/// `Vehicle::UpdateVisualEffect`: update the cached visual effect. `allow_power_change` is
/// true if the wagon-is-powered-state may change.
pub(crate) fn update_visual_effect(
    vs: &VehicleServices,
    shared: &Services,
    this: VehicleRef,
    allow_power_change: bool,
) {
    let powered_before = has_bit(this.cached_vis_effect(), VE_DISABLE_WAGON_POWER);
    let e = vs.newgrf().engine(this.engine_type());

    /* Evaluate properties */
    // `uint8_t visual_effect`, held widened for the bit helpers and stored truncated.
    let mut visual_effect = u32::from(e.visual_effect());

    /* Check powered wagon / visual effect callback */
    if VehicleCallbackMasks(e.callbacks()).test(VehicleCallbackMask::VisualEffect) {
        let callback = get_vehicle_callback(
            vs.newgrf(),
            CallbackID::CBID_VEHICLE_VISUAL_EFFECT,
            0,
            0,
            this.engine_type(),
            Some(this),
        );

        if callback != CALLBACK_FAILED {
            if callback >= 0x100 && e.grf_version(shared) >= 8 {
                vs.unknown_callback_result(e, CallbackID::CBID_VEHICLE_VISUAL_EFFECT, callback);
            }

            let mut callback = gb(u32::from(callback), 0, 8);
            /* Avoid accidentally setting 'visual_effect' to the default value
             * Since bit 6 (disable effects) is set anyways, we can safely erase some bits. */
            if callback == u32::from(VE_DEFAULT) {
                assert_with(|| has_bit(callback, VE_DISABLE_EFFECT));
                sb(&mut callback, VE_TYPE_START, VE_TYPE_COUNT, 0);
            }
            visual_effect = callback;
        }
    }

    /* Apply default values */
    if visual_effect == u32::from(VE_DEFAULT)
        || (!has_bit(visual_effect, VE_DISABLE_EFFECT)
            && gb(visual_effect, VE_TYPE_START, VE_TYPE_COUNT) == u32::from(VE_TYPE_DEFAULT))
    {
        /* Only train engines have default effects.
         * Note: This is independent of whether the engine is a front engine or articulated part or whatever. */
        if e.kind() != VehicleType::VEH_TRAIN
            || e.rail_kind() == RailVehicleTypes::RAILVEH_WAGON
            || !crate::math::is_inside_mm(
                usize::from(e.train_class()),
                usize::from(EC_STEAM),
                usize::from(EC_MONORAIL),
            )
        {
            if visual_effect == u32::from(VE_DEFAULT) {
                visual_effect = 1 << VE_DISABLE_EFFECT;
            } else {
                set_bit(&mut visual_effect, VE_DISABLE_EFFECT);
            }
        } else {
            if visual_effect == u32::from(VE_DEFAULT) {
                /* Also set the offset */
                visual_effect =
                    u32::from(VE_OFFSET_CENTRE - if e.train_class() == EC_STEAM { 4 } else { 0 })
                        << VE_OFFSET_START;
            }
            sb(
                &mut visual_effect,
                VE_TYPE_START,
                VE_TYPE_COUNT,
                u32::from(e.train_class() - EC_STEAM + VE_TYPE_STEAM),
            );
        }
    }

    this.set_cached_vis_effect(visual_effect as u8);

    if !allow_power_change
        && powered_before != has_bit(this.cached_vis_effect(), VE_DISABLE_WAGON_POWER)
    {
        let mut cached_vis_effect = u32::from(this.cached_vis_effect());
        toggle_bit(&mut cached_vis_effect, VE_DISABLE_WAGON_POWER);
        this.set_cached_vis_effect(cached_vis_effect as u8);
        (vs.show_newgrf_vehicle_error)(
            this.engine_type().0,
            STR_NEWGRF_BROKEN.0,
            STR_NEWGRF_BROKEN_POWERED_WAGON.0,
            GRFBug::VehPoweredWagon,
            false,
        );
    }
}

/// `_vehicle_smoke_pos`.
const VEHICLE_SMOKE_POS: [i8; 8] = [1, 1, 1, 0, -1, -1, -1, 0];

/// `SpawnAdvancedVisualEffect`: call `CBID_VEHICLE_SPAWN_VISUAL_EFFECT` and spawn the
/// requested effects for `v`.
#[allow(unsafe_code)]
fn spawn_advanced_visual_effect(vs: &VehicleServices, shared: &Services, v: VehicleRef) {
    // `std::array<int32_t, 4> regs100`. The original leaves it uninitialised; it is
    // zero-filled here, and only the `count` registers of a successful callback are read.
    let mut regs100 = [MaybeUninit::new(0_i32); 4];
    let callback = resolve_vehicle_callback(
        vs.newgrf(),
        VehicleCallback {
            callback: CallbackID::CBID_VEHICLE_SPAWN_VISUAL_EFFECT,
            param1: 0,
            param2: shared.random(),
            engine: v.engine_type(),
            v: Some(v),
            parent: None,
            regs100: &mut regs100,
        },
    );
    if callback == CALLBACK_FAILED {
        return;
    }

    let count = gb(u32::from(callback), 0, 2) as usize;
    assert_with(|| count <= regs100.len());
    let auto_center = has_bit(callback, 13);
    let auto_rotate = !has_bit(callback, 14);

    let mut l_center: i8 = 0;
    if auto_center {
        /* For road vehicles: Compute offset from vehicle position to vehicle center */
        if v.vehicle_type() == VehicleType::VEH_ROAD {
            // `RoadVehicle::From(v)` asserts the type.
            assert_with(|| v.vehicle_type() == VehicleType::VEH_ROAD);
            let cached_veh_length = v.cached_veh_length();
            l_center = (-(u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(cached_veh_length))
                as i32)
                / 2) as i8;
        }
    } else {
        /* For trains: Compute offset from vehicle position to sprite position */
        if v.vehicle_type() == VehicleType::VEH_TRAIN {
            let cached_veh_length = Train::from_vehicle(v).vehicle().cached_veh_length();
            l_center =
                (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(cached_veh_length)) / 2) as i8;
        }
    }

    let mut l_dir = v.direction();
    if v.vehicle_type() == VehicleType::VEH_TRAIN
        && Train::from_vehicle(v)
            .flags()
            .test(VehicleRailFlag::Flipped)
    {
        l_dir = reverse_dir(l_dir, |ok| assert_with(|| ok));
    }
    let t_dir = change_dir(l_dir, DirDiff::DIRDIFF_90RIGHT, |ok| assert_with(|| ok));

    let l_pos = VEHICLE_SMOKE_POS[usize::from(l_dir.0)];
    let t_pos = VEHICLE_SMOKE_POS[usize::from(t_dir.0)];
    let x_center = l_pos.wrapping_mul(l_center);
    let y_center = t_pos.wrapping_mul(l_center);

    for reg in &regs100[..count] {
        // SAFETY: Every register was initialised by the zero fill above.
        let reg = unsafe { reg.assume_init() } as u32;
        let r#type = gb(reg, 0, 8);
        let mut x = gb(reg, 8, 8) as i8;
        let mut y = gb(reg, 16, 8) as i8;
        let z = gb(reg, 24, 8) as i8;

        if auto_rotate {
            // `int8_t l = x; int8_t t = y;`
            let (along, across) = (i32::from(x), i32::from(y));
            x = (i32::from(l_pos) * along + i32::from(t_pos) * across) as i8;
            y = (i32::from(t_pos) * along - i32::from(l_pos) * across) as i8;
        }

        if r#type >= 0xF0 {
            let evt = match r#type {
                0xF1 => EffectVehicleType::EV_STEAM_SMOKE,
                0xF2 => EffectVehicleType::EV_DIESEL_SMOKE,
                0xF3 => EffectVehicleType::EV_ELECTRIC_SPARK,
                0xFA => EffectVehicleType::EV_BREAKDOWN_SMOKE_AIRCRAFT,
                _ => continue,
            };
            let _ = create_effect_vehicle_rel(
                v,
                i32::from(x_center) + i32::from(x),
                i32::from(y_center) + i32::from(y),
                i32::from(z),
                evt,
            );
        }
    }
}

/// `IsBridgeAboveVehicle`: test if a bridge is above vehicle `v`.
fn is_bridge_above_vehicle(world: &World<'_>, v: VehicleRef) -> bool {
    if world.bridge_tile(v.tile()) {
        /* If the vehicle is 'on' a bridge tile, check the real position of the vehicle. If it's different then the
         * vehicle is on the middle of the bridge, which cannot have a bridge above. */
        let tile = world
            .dimensions
            .tile_virt_xy(v.x_pos() as u32, v.y_pos() as u32);
        if tile != v.tile() {
            return false;
        }
    }
    world.bridge_above(v.tile())
}

// `static_assert`s beside `ShowVisualEffect`'s cast of the type bits to a spawning model.
const _: () = assert!(VisualEffectSpawnModel::VESM_STEAM.0 == VE_TYPE_STEAM);
const _: () = assert!(VisualEffectSpawnModel::VESM_DIESEL.0 == VE_TYPE_DIESEL);
const _: () = assert!(VisualEffectSpawnModel::VESM_ELECTRIC.0 == VE_TYPE_ELECTRIC);

/// `ShowVisualEffect`'s `switch (effect_model)`: the effect part `v` of `this`'s chain emits
/// on this tick under its spawning model, or `EV_END` for none.
fn effect_vehicle_type(
    shared: &Services,
    this: VehicleRef,
    v: VehicleRef,
    effect_model: VisualEffectSpawnModel,
    max_speed: u32,
) -> EffectVehicleType {
    let mut evt = EffectVehicleType::EV_END;
    match effect_model {
        VisualEffectSpawnModel::VESM_STEAM => {
            /* Steam smoke - amount is gradually falling until vehicle reaches its maximum speed, after that it's normal.
             * Details: while vehicle's current speed is gradually increasing, steam plumes' density decreases by one third each
             * third of its maximum speed spectrum. Steam emission finally normalises at very close to vehicle's maximum speed.
             * REGULATION:
             * - instead of 1, 4 / 2^smoke_amount (max. 2) is used to provide sufficient regulation to steam puffs' amount. */
            let n = (4_u32 >> shared.settings().smoke_amount())
                + (u32::from(this.cur_speed()) * 3) / max_speed;
            if gb(u32::from(v.tick_counter()), 0, n as u8) == 0 {
                evt = EffectVehicleType::EV_STEAM_SMOKE;
            }
        }

        VisualEffectSpawnModel::VESM_DIESEL => {
            /* Diesel smoke - thicker when vehicle is starting, gradually subsiding till it reaches its maximum speed
             * when smoke emission stops.
             * Details: Vehicle's (max.) speed spectrum is divided into 32 parts. When max. speed is reached, chance for smoke
             * emission erodes by 32 (1/4). For trains, power and weight come in handy too to either increase smoke emission in
             * 6 steps (1000HP each) if the power is low or decrease smoke emission in 6 steps (512 tonnes each) if the train
             * isn't overweight. Power and weight contributions are expressed in a way that neither extreme power, nor
             * extreme weight can ruin the balance (e.g. FreightWagonMultiplier) in the formula. When the vehicle reaches
             * maximum speed no diesel_smoke is emitted.
             * REGULATION:
             * - up to which speed a diesel vehicle is emitting smoke (with reduced/small setting only until 1/2 of max_speed),
             * - in Chance16 - the last value is 512 / 2^smoke_amount (max. smoke when 128 = smoke_amount of 2). */
            let mut power_weight_effect: i32 = 0;
            if v.vehicle_type() == VehicleType::VEH_TRAIN {
                let gcache = Train::from_vehicle(this).vehicle().gcache();
                // The shift counts are left to the processor, as the original's are.
                power_weight_effect = 32_i32.wrapping_shr(gcache.cached_power() >> 10)
                    - 32_i32.wrapping_shr(gcache.cached_weight() >> 9);
            }
            if u32::from(this.cur_speed()) < (max_speed >> (2 >> shared.settings().smoke_amount()))
                && chance16(
                    shared,
                    64_u32
                        .wrapping_sub((u32::from(this.cur_speed()) << 5) / max_speed)
                        .wrapping_add(power_weight_effect as u32),
                    512 >> shared.settings().smoke_amount(),
                )
            {
                evt = EffectVehicleType::EV_DIESEL_SMOKE;
            }
        }

        VisualEffectSpawnModel::VESM_ELECTRIC => {
            /* Electric train's spark - more often occurs when train is departing (more load)
             * Details: Electric locomotives are usually at least twice as powerful as their diesel counterparts, so spark
             * emissions are kept simple. Only when starting, creating huge force are sparks more likely to happen, but when
             * reaching its max. speed, quarter by quarter of it, chance decreases until the usual 2,22% at train's top speed.
             * REGULATION:
             * - in Chance16 the last value is 360 / 2^smoke_amount (max. sparks when 90 = smoke_amount of 2). */
            if gb(u32::from(v.tick_counter()), 0, 2) == 0
                && chance16(
                    shared,
                    6_u32.wrapping_sub((u32::from(this.cur_speed()) << 2) / max_speed),
                    360 >> shared.settings().smoke_amount(),
                )
            {
                evt = EffectVehicleType::EV_ELECTRIC_SPARK;
            }
        }

        _ => not_reached(),
    }
    evt
}

/// `ShowVisualEffect`'s stock emission of `evt` from part `v` at its `effect_offset`.
fn spawn_visual_effect(v: VehicleRef, mut effect_offset: i32, evt: EffectVehicleType) {
    /* The effect offset is relative to a point 4 units behind the vehicle's
     * front (which is the center of an 8/8 vehicle). Shorter vehicles need a
     * correction factor. */
    if v.vehicle_type() == VehicleType::VEH_TRAIN {
        let cached_veh_length = Train::from_vehicle(v).vehicle().cached_veh_length();
        effect_offset = effect_offset.wrapping_add(
            (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(cached_veh_length)) / 2) as i32,
        );
    }

    let mut x =
        i32::from(VEHICLE_SMOKE_POS[usize::from(v.direction().0)]).wrapping_mul(effect_offset);
    let mut y = i32::from(VEHICLE_SMOKE_POS[((u32::from(v.direction().0) + 2) % 8) as usize])
        .wrapping_mul(effect_offset);

    if v.vehicle_type() == VehicleType::VEH_TRAIN
        && Train::from_vehicle(v)
            .flags()
            .test(VehicleRailFlag::Flipped)
    {
        x = x.wrapping_neg();
        y = y.wrapping_neg();
    }

    let _ = create_effect_vehicle_rel(v, x, y, 10, evt);
}

/// `Vehicle::ShowVisualEffect`: draw visual effects (smoke and/or sparks) for the vehicle
/// chain of `this`, a primary vehicle. `get_current_max_speed` is the vehicle's
/// `GetCurrentMaxSpeed()`, which the caller resolves for its vehicle type.
pub(crate) fn show_visual_effect(
    vs: &VehicleServices,
    shared: &Services,
    this: VehicleRef,
    get_current_max_speed: impl FnOnce() -> i32,
) {
    assert_with(|| this.is_primary_vehicle());
    let mut sound = false;

    /* Do not show any smoke when:
     * - vehicle smoke is disabled by the player
     * - the vehicle is slowing down or stopped (by the player)
     * - the vehicle is moving very slowly
     */
    if shared.settings().smoke_amount() == 0
        || this.vehstatus().any(
            VehStates::from_element(VehState::TrainSlowing)
                .with(VehStates::from_element(VehState::Stopped)),
        )
        || this.cur_speed() < 2
    {
        return;
    }

    /* Use the speed as limited by underground and orders. */
    let max_speed = get_current_max_speed() as u32;

    let world = World::new(shared);
    if this.vehicle_type() == VehicleType::VEH_TRAIN {
        let t = Train::from_vehicle(this);
        /* For trains, do not show any smoke when:
         * - the train is reversing
         * - is entering a station with an order to stop there and its speed is equal to maximum station entering speed
         */
        if t.flags().test(VehicleRailFlag::Reversing)
            || (world.is_rail_station_tile(t.vehicle().tile())
                && t.vehicle().is_front_engine()
                && crate::movement::should_stop_at_station(
                    t.vehicle(),
                    world.get_station_index(t.vehicle().tile()),
                )
                && u32::from(t.vehicle().cur_speed()) >= max_speed)
        {
            return;
        }
    }

    let mut v = this;

    loop {
        'part: {
            let advanced = has_bit(v.cached_vis_effect(), VE_ADVANCED_EFFECT);
            let mut effect_offset = gb(
                u32::from(v.cached_vis_effect()),
                VE_OFFSET_START,
                VE_OFFSET_COUNT,
            ) as i32
                - i32::from(VE_OFFSET_CENTRE);
            let mut effect_model;
            if advanced {
                effect_offset = i32::from(VE_OFFSET_CENTRE);
                effect_model = VisualEffectSpawnModel(gb(
                    u32::from(v.cached_vis_effect()),
                    0,
                    VE_ADVANCED_EFFECT,
                ) as u8);
                if effect_model >= VisualEffectSpawnModel::VESM_END {
                    effect_model = VisualEffectSpawnModel::VESM_NONE; // unknown spawning model
                }
            } else {
                effect_model = VisualEffectSpawnModel(gb(
                    u32::from(v.cached_vis_effect()),
                    VE_TYPE_START,
                    VE_TYPE_COUNT,
                ) as u8);
                assert_with(|| effect_model != VisualEffectSpawnModel(VE_TYPE_DEFAULT)); // should have been resolved by UpdateVisualEffect
            }

            /* Show no smoke when:
             * - Smoke has been disabled for this vehicle
             * - The vehicle is not visible
             * - The vehicle is under a bridge
             * - The vehicle is on a depot tile
             * - The vehicle is on a tunnel tile
             * - The vehicle is a train engine that is currently unpowered */
            if effect_model == VisualEffectSpawnModel::VESM_NONE
                || v.vehstatus().test(VehState::Hidden)
                || is_bridge_above_vehicle(&world, v)
                || crate::depot_map::is_depot_tile(&world, v.tile())
                || world.tunnel_tile(v.tile())
                || (v.vehicle_type() == VehicleType::VEH_TRAIN
                    && !crate::rail::has_power_on_rail_types(
                        shared,
                        Train::from_vehicle(v).railtypes(),
                        world.get_tile_rail_type(v.tile()),
                    ))
            {
                break 'part;
            }

            let evt = effect_vehicle_type(shared, this, v, effect_model, max_speed);

            if evt != EffectVehicleType::EV_END && advanced {
                sound = true;
                spawn_advanced_visual_effect(vs, shared, v);
            } else if evt != EffectVehicleType::EV_END {
                sound = true;

                spawn_visual_effect(v, effect_offset, evt);
            }
        }
        match v.next() {
            Some(next) => v = next,
            None => break,
        }
    }

    if sound {
        (vs.play_vehicle_sound)(this, VehicleSoundEvent::VSE_VISUAL_EFFECT);
    }
}

/// [`update_visual_effect`] for the fleet harness (`tools/simulation/fleet-console.cpp`).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_update(
    v: VehicleRef,
    allow_power_change: bool,
    vehicle: &VehicleServices,
    shared: &Services,
) {
    update_visual_effect(vehicle, shared, v, allow_power_change);
}

/// [`show_visual_effect`] for the fleet harness (`tools/simulation/fleet-console.cpp`);
/// `get_current_max_speed` calls the shell's `GetCurrentMaxSpeed()`, after the gate.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_show(
    v: VehicleRef,
    vehicle: &VehicleServices,
    shared: &Services,
    get_current_max_speed: extern "C" fn(VehicleRef) -> i32,
) {
    show_visual_effect(vehicle, shared, v, || get_current_max_speed(v));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_type::OpenTTDEngine;
    use crate::map_func::TileTypeAllocation;
    use crate::newgrf_vehicle::State;
    use crate::vehicle_base::{GroundVehicleSubtypeFlags, VehicleRecord};
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;

    /// One `ShowNewGrfVehicleError` call: engine, the two strings, the bug and `critical`.
    type Error = (u16, u32, u32, GRFBug, bool);
    thread_local! {
        static SMOKE_AMOUNT: Cell<u8> = const { Cell::new(0) };
        static ENGINE: Cell<*const OpenTTDEngine> = const { Cell::new(std::ptr::null()) };
        /// The visual effect callback's answer.
        static ANSWER: Cell<u16> = const { Cell::new(CALLBACK_FAILED) };
        static ERRORS: RefCell<Vec<Error>> = const { RefCell::new(Vec::new()) };
    }

    extern "C" fn random(_: *mut c_void) -> u32 {
        unreachable!("no test draws")
    }
    /// A shared table over an empty map whose settings catalog holds `SMOKE_AMOUNT`.
    fn shared() -> &'static Services {
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 0,
                assertions: true,
            },
            0,
        );
        let mut settings = crate::settings_type::SettingAddresses::NULL;
        settings.smoke_amount = SMOKE_AMOUNT.with(Cell::as_ptr);
        let mut shared = crate::services::fixture(std::ptr::null_mut(), random);
        shared.map = crate::map_func::fixture::statics();
        shared.settings = Box::leak(Box::new(settings));
        Box::leak(Box::new(shared))
    }
    /// A live record of `kind` and `subtype`, its own first part, with `cache` as its
    /// `vcache.cached_vis_effect`.
    fn record(kind: VehicleType, subtype: u8, cache: &'static mut u8) -> VehicleRef {
        let v = VehicleRef::of(VehicleRecord::new(std::ptr::null_mut(), kind));
        v.set_subtype(subtype);
        v.set_visual(cache);
        v
    }

    /// `ShowVisualEffect` asks for the maximum speed exactly when its gate passes: smoke is
    /// on, neither `TrainSlowing` nor `Stopped` is set, and `cur_speed >= 2`. The front road
    /// vehicle's cache `0x40` is the advanced model `VESM_NONE`, so the chain emits nothing
    /// and the vehicle table, whose every entry is unreachable, is never called.
    #[test]
    fn show_visual_effect_asks_for_the_maximum_speed_only_past_its_gate() {
        let shared = shared();
        let v = record(
            VehicleType::VEH_ROAD,
            1 << GroundVehicleSubtypeFlags::GVSF_FRONT.0,
            Box::leak(Box::new(0x40)),
        );
        for smoke_amount in [0, 1, 2] {
            for status in [0, 1, 2, 16, 128] {
                for cur_speed in [0, 1, 2] {
                    SMOKE_AMOUNT.set(smoke_amount);
                    v.set_vehstatus(VehStates(status));
                    v.set_cur_speed(cur_speed);
                    let mut asked = 0;
                    show_visual_effect(&crate::test_tables::VEHICLE, shared, v, || {
                        asked += 1;
                        100
                    });
                    // 0x12 is `TrainSlowing` (bit 4) with `Stopped` (bit 1).
                    let passes = smoke_amount != 0 && status & 0x12 == 0 && cur_speed >= 2;
                    assert_eq!(
                        asked,
                        u32::from(passes),
                        "smoke {smoke_amount} status {status} speed {cur_speed}"
                    );
                }
            }
        }
    }

    extern "C" fn engine_by_id(_: u16) -> *const OpenTTDEngine {
        ENGINE.get()
    }
    extern "C" fn resolve_callback(
        _: *mut State,
        _: u16,
        callback: u16,
        _: u32,
        _: u32,
        _: *mut i32,
        _: usize,
    ) -> u16 {
        assert_eq!(callback, CallbackID::CBID_VEHICLE_VISUAL_EFFECT.0);
        ANSWER.get()
    }
    extern "C" fn unused_member(_: *mut crate::vehicle::OpenTTDVehicle) -> u32 {
        unreachable!()
    }
    extern "C" fn cargo_type(_: *mut crate::vehicle::OpenTTDVehicle) -> u8 {
        0
    }
    extern "C" fn unused_byte(_: *mut crate::vehicle::OpenTTDVehicle) -> u8 {
        unreachable!()
    }
    extern "C" fn unused_owner(
        _: *mut crate::vehicle::OpenTTDVehicle,
    ) -> *const crate::cargo_storage::List {
        unreachable!()
    }
    extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
        let members = crate::newgrf_vehicle::test_table_with_members(
            unused_member,
            cargo_type,
            unused_byte,
            unused_owner,
        );
        Box::leak(Box::new(crate::newgrf_vehicle::test_table_answering(
            &members,
            engine_by_id,
            resolve_callback,
        )))
    }
    extern "C" fn show_newgrf_vehicle_error(
        engine: u16,
        part1: u32,
        part2: u32,
        bug: GRFBug,
        critical: bool,
    ) {
        ERRORS.with(|e| e.borrow_mut().push((engine, part1, part2, bug, critical)));
    }

    /// When the visual effect callback flips the powered-wagon bit and the power state may
    /// not change, `UpdateVisualEffect` toggles the bit back and reports the broken `NewGRF`
    /// once; when it may change, the new bit stays and nothing is reported. The fleet
    /// harness compares the cache but not the message.
    #[test]
    fn update_visual_effect_reports_a_power_change_it_may_not_make() {
        let mut ship = crate::engine::Lifecycle::default();
        ship.r#type = VehicleType::VEH_SHIP;
        ship.vehicle_info_type = VehicleType::VEH_SHIP;
        ship.info.callback_mask =
            VehicleCallbackMasks::from_element(VehicleCallbackMask::VisualEffect).0;
        ENGINE.set(Box::leak(Box::new(ship)));
        let vs = VehicleServices {
            newgrf,
            show_newgrf_vehicle_error,
            ..crate::test_tables::VEHICLE
        };
        let v = record(VehicleType::VEH_SHIP, 0, Box::leak(Box::new(0)));
        v.set_engine_type(crate::engine_type::EngineID(7));
        // Bit 6 disables the effect, so no default applies; bit 7 is the powered-wagon bit.
        ANSWER.set(0xC0);
        for (allow_power_change, cached, errors) in [
            (
                false,
                0x40,
                vec![(
                    7,
                    STR_NEWGRF_BROKEN.0,
                    STR_NEWGRF_BROKEN_POWERED_WAGON.0,
                    GRFBug::VehPoweredWagon,
                    false,
                )],
            ),
            (true, 0xC0, vec![]),
        ] {
            v.set_cached_vis_effect(0x40);
            ERRORS.with(|e| e.borrow_mut().clear());
            update_visual_effect(&vs, shared(), v, allow_power_change);
            assert_eq!(v.cached_vis_effect(), cached);
            assert_eq!(ERRORS.with(|e| e.borrow().clone()), errors);
        }
    }
}

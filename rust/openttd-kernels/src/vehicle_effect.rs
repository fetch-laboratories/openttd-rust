/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Vehicle effect cache and complete chain emission policy. The game mutex serializes
//! process-wide services and native shell lifetimes. Field reads and owner borrows end
//! before callbacks, which can reenter resolvers, sound and effect allocation.
//! Each Part copies the visual byte before any callback, as `ShowVisualEffectAtSpeed` does.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::struct_field_names,
    clippy::verbose_bit_mask,
    clippy::too_many_lines
)]
use crate::bitmath_func::{gb, has_bit, sb, set_bit, toggle_bit};
use crate::direction_func::{change_dir, reverse_dir};
use crate::direction_type::DIRDIFF_90RIGHT;
use crate::effect::openttd_rust_effect_create_rel as create_rel;
use crate::effectvehicle_func::{
    EV_BREAKDOWN_SMOKE_AIRCRAFT, EV_DIESEL_SMOKE, EV_ELECTRIC_SPARK, EV_STEAM_SMOKE,
};
use crate::engine_type::{
    EC_MONORAIL, EC_STEAM, VE_ADVANCED_EFFECT, VE_DEFAULT, VE_DISABLE_EFFECT,
    VE_DISABLE_WAGON_POWER, VE_OFFSET_CENTRE, VE_OFFSET_COUNT, VE_OFFSET_START, VE_TYPE_COUNT,
    VE_TYPE_DEFAULT, VE_TYPE_START, VE_TYPE_STEAM,
};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_VEHICLE_SPAWN_VISUAL_EFFECT, CBID_VEHICLE_VISUAL_EFFECT,
};
use crate::random_func::chance16_i;
use crate::services::Services as Shared;
use crate::vehicle::{RailVehicleTypes, VEHICLE_LENGTH, VehicleRef, VehicleServices, VehicleType};
use crate::vehicle_base::{
    VESM_DIESEL, VESM_ELECTRIC, VESM_END, VESM_NONE, VESM_STEAM, VS_HIDDEN, VS_STOPPED,
    VS_TRAIN_SLOWING,
};
use std::sync::atomic::{AtomicPtr, Ordering};
/// The front's speed and kind with the smoke setting, observed at entry.
struct Show {
    speed: u16,
    kind: VehicleType,
    smoke: u8,
}
/// One part's position, kind, status, tick and visual cache, observed at its visit.
struct Part {
    tile: u32,
    kind: VehicleType,
    status: u8,
    tick: u8,
    cache: u8,
}
fn part(v: VehicleRef) -> Part {
    Part {
        tile: v.tile(),
        kind: VehicleType(v.vehicle_type()),
        status: v.status(),
        tick: v.tick(),
        cache: v.visual_effect(),
    }
}
/// `v->direction` and the ground part length (`VEHICLE_LENGTH` otherwise).
fn pose(v: VehicleRef) -> (u8, u8) {
    let length = if crate::vehicle_base::is_ground_vehicle(v.vehicle_type()) {
        crate::raw_field::read_field!(v.ground_cache(), cached_veh_length)
    } else {
        VEHICLE_LENGTH
    };
    (v.direction(), length)
}
/// The gate callback initializes every output field and retains no output pointer.
#[repr(C)]
pub struct Services {
    vehicle: &'static VehicleServices,
    shared: &'static Shared,
    /// `_settings_client.gui.smoke_amount`.
    smoke: extern "C" fn() -> u8,
    power_error: extern "C" fn(u16),
    depot: extern "C" fn(u32) -> bool,
    tunnel: extern "C" fn(u32) -> bool,
    powered: extern "C" fn(u64, crate::services::RailType) -> bool,
}
/// `GetTileRailType`, read in place from the canonical map.
fn tile_rail(s: &Services, tile: u32) -> crate::rail_type::RailType {
    crate::rail_type::RailType(crate::map_func::World::new(s.shared).tile_rail_type(tile))
}
static RUNTIME: AtomicPtr<Services> = AtomicPtr::new(std::ptr::null_mut());
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_visual_bind(s: *const Services) -> *const Services {
    RUNTIME.swap(s.cast_mut(), Ordering::AcqRel).cast_const()
}
fn services() -> &'static Services {
    unsafe { &*RUNTIME.load(Ordering::Acquire) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_update(v: VehicleRef, allow_power_change: bool) {
    let s = services();
    let engine = (s.vehicle.engine)(v);
    let cache = crate::raw_field::read_field!(v.0.as_ptr(), visual);
    let id = v.engine_id();
    let powered_before = has_bit(unsafe { *cache }, VE_DISABLE_WAGON_POWER);
    let mut visual = u32::from(engine.visual_effect());
    if engine.callbacks() & crate::newgrf_callbacks::VEHICLE_VISUAL_EFFECT != 0 {
        let mut callback =
            (s.vehicle.callback)((s.vehicle.newgrf)(), v, CBID_VEHICLE_VISUAL_EFFECT);
        if callback != CALLBACK_FAILED {
            if callback >= 0x100 && engine.grf_version(s.shared) >= 8 {
                s.vehicle
                    .unknown_callback_result(engine, CBID_VEHICLE_VISUAL_EFFECT, callback);
            }
            callback = gb(u32::from(callback), 0, 8) as u16;
            if callback == u16::from(VE_DEFAULT) {
                let mut value = u32::from(callback);
                sb(&mut value, VE_TYPE_START, VE_TYPE_COUNT, 0);
                callback = value as u16;
            }
            visual = u32::from(callback);
        }
    }
    if visual == u32::from(VE_DEFAULT)
        || (!has_bit(visual, VE_DISABLE_EFFECT)
            && gb(visual, VE_TYPE_START, VE_TYPE_COUNT) == u32::from(VE_TYPE_DEFAULT))
    {
        if engine.kind() != VehicleType::TRAIN
            || engine.rail_kind() == RailVehicleTypes::WAGON
            || !crate::math::is_inside_mm(
                usize::from(engine.train_class()),
                usize::from(EC_STEAM),
                usize::from(EC_MONORAIL),
            )
        {
            visual = if visual == u32::from(VE_DEFAULT) {
                1 << VE_DISABLE_EFFECT
            } else {
                set_bit(&mut visual, VE_DISABLE_EFFECT)
            };
        } else {
            let engclass = engine.train_class();
            if visual == u32::from(VE_DEFAULT) {
                visual = u32::from(VE_OFFSET_CENTRE - if engclass == EC_STEAM { 4 } else { 0 })
                    << VE_OFFSET_START;
            }
            sb(
                &mut visual,
                VE_TYPE_START,
                VE_TYPE_COUNT,
                u32::from(engclass - EC_STEAM + VE_TYPE_STEAM),
            );
        }
    }
    unsafe {
        *cache = visual as u8;
    }
    if !allow_power_change && powered_before != has_bit(visual, VE_DISABLE_WAGON_POWER) {
        unsafe {
            let mut cached = u32::from(*cache);
            toggle_bit(&mut cached, VE_DISABLE_WAGON_POWER);
            *cache = cached as u8;
        }
        (s.power_error)(id);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_eligible_abi(v: VehicleRef) -> u8 {
    let s = services();
    if s.vehicle.assertions {
        // SAFETY: The shared orders table and live shell obey the vehicle entry contract.
        assert!(unsafe { (s.vehicle.orders().primary_vehicle)(v.shell()) } != 0);
    }
    u8::from(
        (s.smoke)() != 0 && v.status() & (VS_TRAIN_SLOWING | VS_STOPPED) == 0 && v.speed() >= 2,
    )
}
const SMOKE_POS: [i8; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
fn advanced(s: &Services, v: VehicleRef, kind: VehicleType, train: Option<crate::train::Handle>) {
    let mut regs = [0_i32; 4];
    let n = (s.vehicle.newgrf)();
    let engine_id = v.engine_id();
    let callback = unsafe {
        crate::newgrf_vehicle::openttd_rust_vehicle_callback(
            &*n,
            CBID_VEHICLE_SPAWN_VISUAL_EFFECT.0,
            0,
            s.shared.random(),
            engine_id,
            v.0.as_ptr(),
            std::ptr::null_mut(),
            false,
            regs.as_mut_ptr(),
            regs.len(),
        )
    };
    if callback == CALLBACK_FAILED {
        return;
    }
    let count = gb(u32::from(callback), 0, 2) as usize;
    let center = has_bit(callback, 13);
    let rotate = !has_bit(callback, 14);
    let (direction, length) = pose(v);
    let l_center = if center && kind == VehicleType::ROAD {
        -(u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(length)) as i32) / 2
    } else if !center && kind == VehicleType::TRAIN {
        (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(length)) / 2) as i32
    } else {
        0
    } as i8;
    let dir = if train.is_some_and(|t| has_bit(t.get_flags() as u16, crate::train::FLIPPED)) {
        reverse_dir(direction, |ok| {
            if s.vehicle.assertions {
                assert!(ok);
            }
        })
    } else {
        direction
    };
    let a = SMOKE_POS[usize::from(dir)];
    let b = SMOKE_POS[usize::from(change_dir(dir, DIRDIFF_90RIGHT, |ok| {
        if s.vehicle.assertions {
            assert!(ok);
        }
    }))];
    let x_center = a.wrapping_mul(l_center);
    let y_center = b.wrapping_mul(l_center);
    for reg in &regs[..count] {
        let typ = gb(*reg as u32, 0, 8) as u8;
        let mut x = gb(*reg as u32, 8, 8) as i8;
        let mut y = gb(*reg as u32, 16, 8) as i8;
        let z = gb(*reg as u32, 24, 8) as i8;
        if rotate {
            let l = i32::from(x);
            let t = i32::from(y);
            x = (i32::from(a) * l + i32::from(b) * t) as i8;
            y = (i32::from(b) * l - i32::from(a) * t) as i8;
        }
        let typ = match typ {
            0xf1 => EV_STEAM_SMOKE,
            0xf2 => EV_DIESEL_SMOKE,
            0xf3 => EV_ELECTRIC_SPARK,
            0xfa => EV_BREAKDOWN_SMOKE_AIRCRAFT,
            _ => continue,
        };
        create_rel(
            v.0.as_ptr(),
            i32::from(x_center) + i32::from(x),
            i32::from(y_center) + i32::from(y),
            i32::from(z),
            typ,
        );
    }
}
// Keep emission setup after the model, visibility, structure and power gates.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn emit_part(
    s: &Services,
    front: VehicleRef,
    v: VehicleRef,
    gate: &Show,
    p: &Part,
    train: Option<crate::train::Handle>,
    cache: u8,
    model: u8,
    max_speed: u32,
) -> bool {
    let speed = u32::from(gate.speed);
    let should_emit = match model {
        VESM_STEAM => {
            let n = (4_u32 >> gate.smoke) + speed * 3 / max_speed;
            gb(u32::from(p.tick), 0, n as u8) == 0
        }
        VESM_DIESEL => {
            let power_weight = if p.kind == VehicleType::TRAIN {
                let cache = front.ground_cache();
                let power = crate::raw_field::read_field!(cache, cached_power);
                let weight = crate::raw_field::read_field!(cache, cached_weight);
                (32_u32.wrapping_shr(power >> 10)) as i32
                    - (32_u32.wrapping_shr(weight >> 9)) as i32
            } else {
                0
            };
            speed < (max_speed >> (2 >> gate.smoke))
                && chance16_i(
                    (64 - ((speed << 5) / max_speed) as i32 + power_weight) as u32,
                    512 >> gate.smoke,
                    s.shared.random(),
                )
        }
        VESM_ELECTRIC => {
            gb(u32::from(p.tick), 0, 2) == 0
                && chance16_i(
                    6_u32.wrapping_sub((speed << 2) / max_speed),
                    360 >> gate.smoke,
                    s.shared.random(),
                )
        }
        _ => unreachable!(),
    };
    if !should_emit {
        return false;
    }
    let is_advanced = has_bit(cache, VE_ADVANCED_EFFECT);
    let mut offset = if is_advanced {
        i32::from(VE_OFFSET_CENTRE)
    } else {
        gb(u32::from(cache), VE_OFFSET_START, VE_OFFSET_COUNT) as i32 - i32::from(VE_OFFSET_CENTRE)
    };
    if is_advanced {
        advanced(s, v, p.kind, train);
    } else {
        let (direction, length) = pose(v);
        if p.kind == VehicleType::TRAIN {
            offset = offset.wrapping_add(
                (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(length)) / 2) as i32,
            );
        }
        let mut x = i32::from(SMOKE_POS[usize::from(direction)]).wrapping_mul(offset);
        let mut y =
            i32::from(SMOKE_POS[usize::from(direction.wrapping_add(2) % 8)]).wrapping_mul(offset);
        if train.is_some_and(|t| has_bit(t.get_flags() as u16, crate::train::FLIPPED)) {
            x = x.wrapping_neg();
            y = y.wrapping_neg();
        }
        let kind = match model {
            VESM_STEAM => EV_STEAM_SMOKE,
            VESM_DIESEL => EV_DIESEL_SMOKE,
            VESM_ELECTRIC => EV_ELECTRIC_SPARK,
            _ => unreachable!(),
        };
        create_rel(v.0.as_ptr(), x, y, 10, kind);
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_show(front: VehicleRef, max_speed: u32) {
    let s = services();
    let gate = Show {
        speed: front.speed(),
        kind: VehicleType(front.vehicle_type()),
        smoke: (s.smoke)(),
    };
    let front_train = if gate.kind == VehicleType::TRAIN {
        let handle = crate::train::Handle::from_record(front.0.as_ptr());
        let tile = front.tile();
        if handle.get_flags() & crate::train::FLAG_REVERSING != 0
            || (crate::map_func::World::new(s.shared).rail_station(tile)
                && front.is_front_engine()
                && crate::movement::should_stop(s.shared, front, (s.shared.station_index)(tile))
                && u32::from(front.speed()) >= max_speed)
        {
            return;
        }
        Some(handle)
    } else {
        None
    };
    let mut sound = false;
    let mut current = Some(front);
    while let Some(v) = current {
        let p = part(v);
        let cache = p.cache;
        let is_advanced = has_bit(cache, VE_ADVANCED_EFFECT);
        let model = if is_advanced {
            let m = gb(u32::from(cache), 0, VE_ADVANCED_EFFECT) as u8;
            if m >= VESM_END { VESM_NONE } else { m }
        } else {
            let m = gb(u32::from(cache), VE_TYPE_START, VE_TYPE_COUNT) as u8;
            if s.vehicle.assertions {
                assert_ne!(m, VE_TYPE_DEFAULT);
            }
            m
        };
        'part: {
            if model == VESM_NONE || p.status & VS_HIDDEN != 0 {
                break 'part;
            }
            let bridge = if (s.shared.is_bridge)(p.tile) {
                let map = s.shared.map();
                map.tile_virt_xy(v.x() as u32, v.y() as u32) == p.tile
                    && crate::map_func::World::new(s.shared).bridge_above(p.tile)
            } else {
                crate::map_func::World::new(s.shared).bridge_above(p.tile)
            };
            if bridge || (s.depot)(p.tile) || (s.tunnel)(p.tile) {
                break 'part;
            }
            let train = if p.kind == VehicleType::TRAIN {
                Some(if v == front {
                    front_train.unwrap_or_else(|| crate::train::Handle::from_record(v.0.as_ptr()))
                } else {
                    crate::train::Handle::from_record(v.0.as_ptr())
                })
            } else {
                None
            };
            if train.is_some_and(|t| !(s.powered)(t.get_railtypes(), tile_rail(s, p.tile))) {
                break 'part;
            }
            if emit_part(s, front, v, &gate, &p, train, cache, model, max_speed) {
                sound = true;
            }
        }
        current = (v).next();
    }
    if sound {
        (s.vehicle.play_vehicle_sound)(front, crate::newgrf_sound::VSE_VISUAL_EFFECT);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVisualServices" ["probe"] => Services { vehicle, shared, smoke, power_error, depot, tunnel, powered, };
}

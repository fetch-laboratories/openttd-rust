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
use crate::vehicle::{
    OpenTTDEngine, RailVehicleTypes, VEHICLE_LENGTH, VehicleRef, VehicleServices, VehicleType,
};
use crate::vehicle_base::{
    VESM_DIESEL, VESM_ELECTRIC, VESM_END, VESM_NONE, VESM_STEAM, VS_HIDDEN, VS_STOPPED,
    VS_TRAIN_SLOWING,
};
use std::sync::atomic::{AtomicPtr, Ordering};
#[repr(C)]
pub struct Gate {
    speed: u16,
    status: u8,
    smoke: u8,
    primary: bool,
}
#[repr(C)]
pub struct Show {
    speed: u16,
    kind: VehicleType,
    smoke: u8,
}
#[repr(C)]
pub struct BridgePosition {
    x: i32,
    y: i32,
}
#[repr(C)]
pub struct Engine {
    engine: *const OpenTTDEngine,
    cache: *mut u8,
    id: u16,
    kind: VehicleType,
    visual: u8,
    callback: bool,
}
#[repr(C)]
pub struct Defaults {
    rail_kind: RailVehicleTypes,
    engclass: u8,
}
#[repr(C)]
pub struct Part {
    tile: u32,
    kind: VehicleType,
    status: u8,
    tick: u8,
    cache: u8,
}
#[repr(C)]
pub struct Offset {
    direction: u8,
    length: u8,
}
#[repr(C)]
pub struct Train {
    handle: crate::train::Handle,
    tile: u32,
    speed: u16,
    front: bool,
}
#[repr(C)]
pub struct PowerWeight {
    power: u32,
    weight: u32,
}
/// The gate callback initializes every output field and retains no output pointer.
#[repr(C)]
pub struct Services {
    vehicle: &'static VehicleServices,
    shared: &'static Shared,
    gate: extern "C" fn(VehicleRef, *mut Gate),
    show: extern "C" fn(VehicleRef) -> Show,
    engine: extern "C" fn(VehicleRef) -> Engine,
    defaults: extern "C" fn(*const OpenTTDEngine) -> Defaults,
    grf_version: extern "C" fn(*const OpenTTDEngine) -> u8,
    power_error: extern "C" fn(u16),
    part: extern "C" fn(VehicleRef) -> Part,
    bridge_position: extern "C" fn(VehicleRef) -> BridgePosition,
    offset: extern "C" fn(VehicleRef) -> Offset,
    train: extern "C" fn(VehicleRef) -> Train,
    station: extern "C" fn(u32) -> bool,
    depot: extern "C" fn(u32) -> bool,
    tunnel: extern "C" fn(u32) -> bool,
    tile_rail: extern "C" fn(u32) -> crate::services::RailType,
    powered: extern "C" fn(u64, crate::services::RailType) -> bool,
    power_weight: extern "C" fn(VehicleRef) -> PowerWeight,
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
    let e = (s.engine)(v);
    let powered_before = has_bit(unsafe { *e.cache }, VE_DISABLE_WAGON_POWER);
    let mut visual = if matches!(
        e.kind,
        VehicleType::TRAIN | VehicleType::ROAD | VehicleType::SHIP
    ) {
        u32::from(e.visual)
    } else {
        1 << VE_DISABLE_EFFECT
    };
    if e.callback {
        let mut callback =
            (s.vehicle.callback)((s.vehicle.newgrf)(), v, CBID_VEHICLE_VISUAL_EFFECT);
        if callback != CALLBACK_FAILED {
            if callback >= 0x100 && (s.grf_version)(e.engine) >= 8 {
                (s.vehicle.unknown_callback_result)(
                    crate::vehicle::EngineRef(unsafe {
                        std::ptr::NonNull::new_unchecked(e.engine.cast_mut())
                    }),
                    CBID_VEHICLE_VISUAL_EFFECT,
                    callback,
                );
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
        let defaults = if e.kind == VehicleType::TRAIN {
            Some((s.defaults)(e.engine))
        } else {
            None
        };
        if defaults.as_ref().is_none_or(|d| {
            d.rail_kind == RailVehicleTypes::WAGON
                || !crate::math::is_inside_mm(
                    usize::from(d.engclass),
                    usize::from(EC_STEAM),
                    usize::from(EC_MONORAIL),
                )
        }) {
            visual = if visual == u32::from(VE_DEFAULT) {
                1 << VE_DISABLE_EFFECT
            } else {
                set_bit(&mut visual, VE_DISABLE_EFFECT)
            };
        } else {
            let engclass = defaults.unwrap().engclass;
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
        *e.cache = visual as u8;
    }
    if !allow_power_change && powered_before != has_bit(visual, VE_DISABLE_WAGON_POWER) {
        unsafe {
            let mut cached = u32::from(*e.cache);
            toggle_bit(&mut cached, VE_DISABLE_WAGON_POWER);
            *e.cache = cached as u8;
        }
        (s.power_error)(e.id);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_visual_eligible(v: VehicleRef) -> bool {
    let s = services();
    let mut g = std::mem::MaybeUninit::uninit();
    (s.gate)(v, g.as_mut_ptr());
    let g = unsafe { g.assume_init() };
    if s.vehicle.assertions {
        assert!(g.primary);
    }
    g.smoke != 0 && g.status & (VS_TRAIN_SLOWING | VS_STOPPED) == 0 && g.speed >= 2
}
const SMOKE_POS: [i8; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
fn advanced(s: &Services, v: VehicleRef, kind: VehicleType, train: Option<crate::train::Handle>) {
    let mut regs = [0_i32; 4];
    let n = (s.vehicle.newgrf)();
    let engine_id = (s.vehicle.engine_id)(v);
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
    let p = (s.offset)(v);
    let l_center = if center && kind == VehicleType::ROAD {
        -(u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(p.length)) as i32) / 2
    } else if !center && kind == VehicleType::TRAIN {
        (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(p.length)) / 2) as i32
    } else {
        0
    } as i8;
    let dir = if train.is_some_and(|t| has_bit(t.get_flags() as u16, crate::train::FLIPPED)) {
        reverse_dir(p.direction, |ok| {
            if s.vehicle.assertions {
                assert!(ok);
            }
        })
    } else {
        p.direction
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
                let w = (s.power_weight)(front);
                (32_u32.wrapping_shr(w.power >> 10)) as i32
                    - (32_u32.wrapping_shr(w.weight >> 9)) as i32
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
        let o = (s.offset)(v);
        if p.kind == VehicleType::TRAIN {
            offset = offset.wrapping_add(
                (u32::from(VEHICLE_LENGTH).wrapping_sub(u32::from(o.length)) / 2) as i32,
            );
        }
        let mut x = i32::from(SMOKE_POS[usize::from(o.direction)]).wrapping_mul(offset);
        let mut y =
            i32::from(SMOKE_POS[usize::from(o.direction.wrapping_add(2) % 8)]).wrapping_mul(offset);
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
    let gate = (s.show)(front);
    let front_train = if gate.kind == VehicleType::TRAIN {
        let t = (s.train)(front);
        if t.handle.get_flags() & crate::train::FLAG_REVERSING != 0
            || ((s.station)(t.tile)
                && t.front
                && crate::movement::should_stop(s.shared, front, (s.shared.station_index)(t.tile))
                && u32::from(t.speed) >= max_speed)
        {
            return;
        }
        Some(t.handle)
    } else {
        None
    };
    let mut sound = false;
    let mut current = Some(front);
    while let Some(v) = current {
        let p = (s.part)(v);
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
                let pos = (s.bridge_position)(v);
                let map = crate::map_func::Map::from_log_x((s.shared.map_log_x)());
                map.tile_virt_xy(pos.x as u32, pos.y as u32) == p.tile
                    && (s.shared.bridge_above)(p.tile)
            } else {
                (s.shared.bridge_above)(p.tile)
            };
            if bridge || (s.depot)(p.tile) || (s.tunnel)(p.tile) {
                break 'part;
            }
            let train = if p.kind == VehicleType::TRAIN {
                Some(if v == front {
                    front_train.unwrap_or_else(|| (s.vehicle.train_handle)(v.0.as_ptr()))
                } else {
                    (s.vehicle.train_handle)(v.0.as_ptr())
                })
            } else {
                None
            };
            if train.is_some_and(|t| !(s.powered)(t.get_railtypes(), (s.tile_rail)(p.tile))) {
                break 'part;
            }
            if emit_part(s, front, v, &gate, &p, train, cache, model, max_speed) {
                sound = true;
            }
        }
        current = (s.vehicle.next)(v);
    }
    if sound {
        (s.vehicle.play_vehicle_sound)(front, crate::newgrf_sound::VSE_VISUAL_EFFECT);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVisualGate" ["probe"] => Gate { speed, status, smoke, primary, };
    "OpenTTDVisualShow" ["probe"] => Show { speed, kind, smoke, };
    "OpenTTDVisualBridgePosition" ["probe"] => BridgePosition { x, y, };
    "OpenTTDVisualEngine" ["probe"] => Engine { engine, cache, id, kind, visual, callback, };
    "OpenTTDVisualDefaults" ["probe"] => Defaults { rail_kind, engclass, };
    "OpenTTDVisualPart" ["probe"] => Part { tile, kind, status, tick, cache, };
    "OpenTTDVisualOffset" ["probe"] => Offset { direction, length, };
    "OpenTTDVisualTrain" ["probe"] => Train { handle, tile, speed, front, };
    "OpenTTDVisualPowerWeight" ["probe"] => PowerWeight { power, weight, };
    "OpenTTDVisualServices" ["probe"] => Services { vehicle, shared, gate, show, engine, defaults, grf_version, power_error, part, bridge_position, offset, train, station, depot, tunnel, tile_rail, powered, power_weight, };
}

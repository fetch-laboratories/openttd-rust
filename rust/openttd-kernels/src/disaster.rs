/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! Disaster-private ownership and complete controllers over canonical C++ world data.
// Source-width casts and bit masks preserve the original encodings.
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
use crate::direction_type::{DIR_NE, DIR_NW, DIR_SE, DIR_SW, DIRDIFF_90LEFT, DIRDIFF_90RIGHT};
use crate::map_func::Map;
use crate::random_func::chance16_i;
use crate::services::Services as Shared;
use crate::sprites::{
    SPR_AH_64A, SPR_AH_64A_FIRING, SPR_BLIMP, SPR_BLIMP_CRASHED, SPR_BLIMP_CRASHING, SPR_F_15,
    SPR_F_15_FIRING, SPR_ROTOR_MOVING_1, SPR_ROTOR_MOVING_3, SPR_SUB_LARGE_NE, SPR_SUB_SMALL_NE,
    SPR_UFO_HARVESTER, SPR_UFO_SMALL_SCOUT, SPR_UFO_SMALL_SCOUT_DARKER, SPR_XCOM_SKYRANGER,
};
use crate::tile_type::{
    INVALID_TILE, MP_CLEAR, MP_HOUSE, MP_INDUSTRY, MP_RAILWAY, MP_TREES, MP_WATER, TILE_SIZE,
};
use crate::vehicle::{OpenTTDVehicle, VehicleRef, VehicleServices};
use crate::vehicle_base::{VS_CRASHED, VS_HIDDEN};
use std::ffi::c_void;
use std::ptr::NonNull;

#[repr(C)]
pub struct State {
    pub image_override: u32,
    pub target: u32,
    pub state: u16,
    pub flags: u8,
}
static mut DELAY: u16 = 0;
use crate::fleet::INVALID_VEHICLE;

/// A live native shell and its separate Rust owner; null marks the end of iteration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vehicle {
    pub handle: *mut OpenTTDVehicle,
    pub state: *mut State,
    pub id: u32,
}
impl Vehicle {
    fn shell(self) -> VehicleRef {
        // SAFETY: Source-valid live shell, except iteration's null end marker.
        VehicleRef(unsafe { NonNull::new_unchecked(self.handle) })
    }
    fn state(self) -> u16 {
        // SAFETY: Every private scalar access ends before any callback or deletion.
        unsafe { (*self.state).state }
    }
    fn set_state(self, value: u16) {
        unsafe {
            (*self.state).state = value;
        }
    }
    fn image(self) -> u32 {
        unsafe { (*self.state).image_override }
    }
    fn set_image(self, value: u32) {
        unsafe {
            (*self.state).image_override = value;
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoadTarget {
    pub handle: *mut OpenTTDVehicle,
    pub state: *mut crate::road::State,
    pub id: u32,
}
impl RoadTarget {
    fn shell(self) -> VehicleRef {
        VehicleRef(unsafe { NonNull::new_unchecked(self.handle) })
    }
}
/// Industry attributes consumed together while selecting an eligible target.
#[repr(C)]
pub struct IndustryTarget {
    pub industry: *mut c_void,
    pub id: u16,
    pub airplane: bool,
    pub helicopter: bool,
    pub subsidence: bool,
}
#[repr(C)]
pub struct IndustryLocation {
    pub tile: u32,
    pub town: u16,
}
#[repr(C)]
pub struct StationTarget {
    pub next: u32,
    pub tile: u32,
    pub airport_type: u8,
}
#[repr(C)]
pub struct RoadSelection {
    pub front: bool,
    pub disaster: u32,
}
#[repr(C)]
pub struct RoadApproach {
    pub x: i32,
    pub y: i32,
    pub status: u8,
    pub breakdown: u8,
}
#[repr(C)]
pub struct TrainSelection {
    pub front: bool,
    pub owner: u8,
    pub tile: u32,
    pub status: u8,
}
/// Immutable tables outlive each synchronous entry and nested entry. Native shells
/// obey the original pool lifetimes; no Rust borrow spans a service call. Callbacks
/// are noexcept, and environmental failures terminate. No map/Vehicle layout is exposed.
#[repr(C)]
pub struct Services {
    pub shared: *const Shared,
    pub vehicles: *const VehicleServices,
    pub occupancy: *const crate::vehicle::occupancy::Commands,
    pub environment: *const crate::services::EnvironmentServices,
    pub rail_map: *const crate::services::RailMapServices,
    pub aircraft: *const crate::aircraft::Leaves,
    pub road: *const crate::road::Leaves,
    pub age: extern "C" fn(VehicleRef) -> i32,
    pub write_age: extern "C" fn(VehicleRef, i32),
    pub write_sprite: extern "C" fn(VehicleRef, u32),
    pub first_sprite: extern "C" fn(VehicleRef) -> u32,
    pub write_first_sprite: extern "C" fn(VehicleRef, u32),
    pub write_subtype: extern "C" fn(VehicleRef, u8),
    pub write_owner: extern "C" fn(VehicleRef, u8),
    pub set_next: extern "C" fn(VehicleRef, VehicleRef),
    pub next_part: extern "C" fn(VehicleRef) -> Vehicle,
    pub update_delta: extern "C" fn(VehicleRef),
    pub toward: extern "C" fn(VehicleRef, i32, i32) -> u8,
    pub create: extern "C" fn(i32, i32, u8, u8, u32) -> Vehicle,
    pub resolve: extern "C" fn(u32) -> Vehicle,
    pub next_disaster: extern "C" fn(u32) -> Vehicle,
    pub next_road: extern "C" fn(u32) -> RoadTarget,
    pub road_target: extern "C" fn(u32) -> RoadTarget,
    pub road_selection: extern "C" fn(VehicleRef) -> RoadSelection,
    pub road_approach: extern "C" fn(VehicleRef) -> RoadApproach,
    pub write_road_disaster: extern "C" fn(VehicleRef, u32),
    pub write_breakdown: extern "C" fn(VehicleRef, u8, u8),
    pub next_vehicle: extern "C" fn(u32) -> Vehicle,
    pub ground: extern "C" fn(VehicleRef) -> bool,
    pub next_train: extern "C" fn(u32) -> Vehicle,
    pub train_selection: extern "C" fn(VehicleRef) -> TrainSelection,
    pub road_count: extern "C" fn(u8) -> u16,
    pub human: extern "C" fn(u8) -> bool,
    pub next_station: extern "C" fn(u32) -> StationTarget,
    pub next_industry: extern "C" fn(u32) -> IndustryTarget,
    pub industry: extern "C" fn(u16) -> IndustryTarget,
    pub industry_location: extern "C" fn(*mut c_void) -> IndustryLocation,
    pub industry_tile: extern "C" fn(*mut c_void) -> u32,
    pub industry_town: extern "C" fn(*mut c_void) -> u16,
    pub industry_index: extern "C" fn(u32) -> u16,
    pub industry_belongs: extern "C" fn(*mut c_void, u32) -> bool,
    pub reset_industry_tile: extern "C" fn(u32),
    pub airport_tile: extern "C" fn(u32) -> bool,
    pub airport_blocks: extern "C" fn(u16, bool),
    pub zeppelin_crashed: extern "C" fn(u32),
    pub zeppelin_cleared: extern "C" fn(u32, u16),
    pub road_crashed: extern "C" fn(VehicleRef, u32),
    pub zeppelin_news: extern "C" fn(u32, u16),
    pub small_ufo_news: extern "C" fn(u32),
    pub airplane_news: extern "C" fn(u16, u16),
    pub helicopter_news: extern "C" fn(u16, u16),
    pub big_ufo_news: extern "C" fn(u32, u16),
    pub coal_news: extern "C" fn(u32, u16),
    pub clear: extern "C" fn(u32, u8),
    pub update_signals: extern "C" fn(),
    pub disasters_enabled: extern "C" fn() -> bool,
    pub can_allocate: extern "C" fn(usize) -> bool,
    pub town_index: extern "C" fn(*const crate::services::TownHandle) -> u16,
}
struct World<'a> {
    s: &'a Services,
    shared: &'a Shared,
    v: &'a VehicleServices,
}
impl<'a> World<'a> {
    unsafe fn new(s: &'a Services) -> Self {
        Self {
            s,
            shared: unsafe { &*s.shared },
            v: unsafe { &*s.vehicles },
        }
    }
    fn map(&self) -> Map {
        self.shared.map()
    }
    fn size_y(&self) -> u32 {
        (self.shared.map_max_y)().wrapping_add(1)
    }
    fn random(&self) -> u32 {
        self.shared.random()
    }
    fn random_tile(&self) -> u32 {
        self.random() & (self.map().size_x() * self.size_y() - 1)
    }
    fn chance(&self, denominator: u32) -> bool {
        chance16_i(1, denominator, self.random())
    }
    fn tile(&self, v: Vehicle) -> u32 {
        (self.v.tile)(v.shell())
    }
    fn dest(&self, v: Vehicle) -> u32 {
        (self.v.dest_tile)(v.shell())
    }
    fn set_dest(&self, v: Vehicle, tile: u32) {
        (self.v.write_dest)(v.shell(), tile);
    }
    fn age(&self, v: Vehicle) -> i32 {
        (self.s.age)(v.shell())
    }
    fn set_age(&self, v: Vehicle, age: i32) {
        (self.s.write_age)(v.shell(), age);
    }
    fn increment_age(&self, v: Vehicle) -> i32 {
        let age = self.age(v).wrapping_add(1);
        self.set_age(v, age);
        age
    }
    fn tick(&self, v: Vehicle) -> u8 {
        (self.v.tick)(v.shell())
    }
    fn increment_tick(&self, v: Vehicle) -> u8 {
        let tick = self.tick(v).wrapping_add(1);
        (self.v.write_tick)(v.shell(), tick);
        tick
    }
    fn xy(&self, v: Vehicle) -> crate::vehicle::XY {
        (self.v.xy)(v.shell())
    }
    fn slope(&self, x: i32, y: i32) -> i32 {
        crate::movement::pixel_z(self.shared, x, y, false)
    }
    fn flight_vehicle(v: Vehicle) -> crate::aircraft::Vehicle {
        crate::aircraft::Vehicle {
            handle: v.handle,
            state: std::ptr::null_mut(),
            id: v.id,
        }
    }
    fn bounds(&self, v: Vehicle) -> i32 {
        let mut z = 0;
        unsafe {
            crate::aircraft::openttd_rust_aircraft_flight_bounds(
                self.s.aircraft,
                Self::flight_vehicle(v),
                &raw mut z,
                std::ptr::null_mut(),
            );
        }
        z
    }
    fn flight(&self, v: Vehicle) -> i32 {
        unsafe {
            crate::aircraft::openttd_rust_aircraft_flight_level(
                self.s.aircraft,
                Self::flight_vehicle(v),
                &raw mut (*v.state).flags,
                false,
            )
        }
    }
    fn delete(&self, v: Vehicle) -> bool {
        (self.v.destroy)(v.shell());
        false
    }
    fn viewport(&self, v: Vehicle) {
        unsafe {
            crate::vehicle::occupancy::update(self.v, v.shell(), false);
        }
        (self.v.base_viewport)(v.shell());
    }
    fn update_image(&self, v: Vehicle) {
        let mut image = v.image();
        if image == 0 {
            let subtype = (self.v.subtype)(v.shell());
            let dir = (self.v.direction)(v.shell());
            image = match subtype {
                0 | 1 => SPR_BLIMP,
                2 | 3 => SPR_UFO_SMALL_SCOUT,
                4 | 5 => SPR_F_15,
                6 | 7 => SPR_AH_64A,
                8 => SPR_ROTOR_MOVING_1,
                9 | 10 => SPR_UFO_HARVESTER,
                11 | 12 => SPR_XCOM_SKYRANGER,
                13 => SPR_SUB_SMALL_NE + u32::from(dir) / 2,
                _ => SPR_SUB_LARGE_NE + u32::from(dir) / 2,
            };
        }
        (self.s.write_sprite)(v.shell(), image);
    }
    fn update_position(&self, v: Vehicle, x: i32, y: i32, z: i32) {
        (self.v.write_coordinates)(v.shell(), x, y, z);
        (self.v.write_tile)(v.shell(), self.map().tile_virt_xy(x as u32, y as u32));
        self.update_image(v);
        self.viewport(v);
        let u = (self.s.next_part)(v.shell());
        if !u.handle.is_null() {
            let safe_x = x.clamp(0, self.map().max_x() as i32 * TILE_SIZE as i32);
            let safe_y = y
                .wrapping_sub(1)
                .clamp(0, (self.shared.map_max_y)() as i32 * TILE_SIZE as i32);
            (self.v.write_x)(u.shell(), x);
            let shadow_y = y
                .wrapping_sub(1)
                .wrapping_sub(z.wrapping_sub(self.slope(safe_x, safe_y)).max(0) >> 3);
            (self.v.write_y)(u.shell(), shadow_y);
            let safe_y = shadow_y.clamp(0, (self.shared.map_max_y)() as i32 * TILE_SIZE as i32);
            (self.v.write_z)(u.shell(), self.slope(safe_x, safe_y));
            (self.v.write_direction)(u.shell(), (self.v.direction)(v.shell()));
            self.update_image(u);
            self.viewport(u);
            let rotor = (self.s.next_part)(u.shell());
            if !rotor.handle.is_null() {
                (self.v.write_coordinates)(rotor.shell(), x, y, z.wrapping_add(5));
                self.viewport(rotor);
            }
        }
    }
    fn move_flight(&self, v: Vehicle) {
        let gp = (self.v.new_position)(v.shell());
        self.update_position(v, gp.x, gp.y, self.flight(v));
    }
    fn effect_rel(v: Vehicle, x: i32, y: i32, z: i32, kind: u8) {
        crate::effect::openttd_rust_effect_create_rel(v.handle, x, y, z, kind);
    }
    fn sound_vehicle(&self, v: Vehicle) {
        if (self.shared.disaster_sound)() {
            (self.v.play_sound)(v.shell(), crate::sound_type::SND_12_EXPLOSION);
        }
    }
    fn clear(&self, tile: u32) {
        let failed = unsafe {
            let c = &*self.s.occupancy;
            crate::command::with_cost(c.costs, |cost| {
                crate::vehicle::occupancy::openttd_rust_occupancy_ground(c, cost.0, tile);
                !cost.ok()
            })
        };
        if failed {
            return;
        }
        match (self.shared.tile_type)(tile) {
            MP_RAILWAY => {
                if (self.s.human)((self.shared.tile_owner)(tile))
                    && !(unsafe { &*self.s.rail_map }.is_rail_depot)(tile)
                {
                    (self.s.clear)(tile, crate::company_type::OWNER_WATER);
                    (self.s.update_signals)();
                }
            }
            MP_HOUSE => (self.s.clear)(tile, crate::company_type::OWNER_NONE),
            MP_CLEAR | MP_TREES => crate::ground::clear_square(tile, self.shared),
            _ => {}
        }
    }
    fn destruct_industry(&self, industry: *mut c_void) {
        let count = self.map().size_x() * self.size_y();
        for tile in Map::iterate(count) {
            if (self.s.industry_belongs)(industry, tile) {
                (self.s.reset_industry_tile)(tile);
                (self.shared.mark_dirty)(tile);
            }
        }
    }
    fn zeppelin(&self, v: Vehicle) -> bool {
        self.increment_tick(v);
        if v.state() < 2 {
            if self.tick(v) & 1 != 0 {
                return true;
            }
            self.move_flight(v);
            if v.state() == 1 {
                if self.increment_age(v) == 38 {
                    v.set_state(2);
                    self.set_age(v, 0);
                }
                if self.tick(v) & 7 == 0 {
                    Self::effect_rel(v, 0, -17, 2, crate::effectvehicle_func::EV_CRASH_SMOKE);
                }
            } else if v.state() == 0 {
                let tile = self.tile(v);
                if (self.shared.tile_valid)(tile) && (self.s.airport_tile)(tile) {
                    v.set_state(1);
                    self.set_age(v, 0);
                    (self.s.zeppelin_news)(tile, (self.shared.station_index)(tile));
                    (self.s.zeppelin_crashed)(self.tile(v));
                }
            }
            if (self.v.y)(v.shell()) >= ((self.size_y() + 9) * TILE_SIZE - 1) as i32 {
                return self.delete(v);
            }
            return true;
        }
        let tile = self.tile(v);
        if (self.shared.tile_valid)(tile) && (self.s.airport_tile)(tile) {
            (self.s.airport_blocks)((self.shared.station_index)(tile), true);
        }
        if v.state() > 2 {
            if self.increment_age(v) <= 13320 {
                return true;
            }
            let tile = self.tile(v);
            if (self.shared.tile_valid)(tile) && (self.s.airport_tile)(tile) {
                let station = (self.shared.station_index)(tile);
                (self.s.airport_blocks)(station, false);
                (self.s.zeppelin_cleared)(tile, station);
            }
            let p = self.xy(v);
            self.update_position(v, p.x, p.y, self.flight(v));
            return self.delete(v);
        }
        let p = self.xy(v);
        let mut z = self.slope(p.x, p.y);
        if z < (self.v.z)(v.shell()) {
            z = (self.v.z)(v.shell()).wrapping_sub(1);
        }
        self.update_position(v, p.x, p.y, z);
        let age = self.increment_age(v);
        if age == 1 {
            Self::effect_rel(v, 0, 7, 8, crate::effectvehicle_func::EV_EXPLOSION_LARGE);
            self.sound_vehicle(v);
            v.set_image(SPR_BLIMP_CRASHING);
        } else if age == 70 {
            v.set_image(SPR_BLIMP_CRASHED);
        } else if age <= 300 {
            if self.tick(v) & 7 == 0 {
                let r = self.random();
                Self::effect_rel(
                    v,
                    (r & 15).wrapping_sub(7) as i32,
                    ((r >> 4) & 15).wrapping_sub(7) as i32,
                    ((r >> 8) & 7) as i32 + 5,
                    crate::effectvehicle_func::EV_EXPLOSION_SMALL,
                );
            }
        } else if age == 350 {
            v.set_state(3);
            self.set_age(v, 0);
        }
        true
    }
    fn small_ufo(&self, v: Vehicle) -> bool {
        let tick = self.increment_tick(v);
        v.set_image(if tick & 8 != 0 {
            SPR_UFO_SMALL_SCOUT_DARKER
        } else {
            SPR_UFO_SMALL_SCOUT
        });
        if v.state() == 0 {
            let map = self.map();
            let dest = self.dest(v);
            let x = (map.tile_x(dest) * TILE_SIZE) as i32;
            let y = (map.tile_y(dest) * TILE_SIZE) as i32;
            let p = self.xy(v);
            if distance(x, y, p.x, p.y) >= TILE_SIZE as i32 {
                (self.v.write_direction)(v.shell(), (self.s.toward)(v.shell(), x, y));
                self.move_flight(v);
                return true;
            }
            if self.increment_age(v) < 6 {
                self.set_dest(v, self.random_tile());
                return true;
            }
            v.set_state(1);
            let mut n = 0u32;
            let mut from = 0;
            loop {
                let company = (self.shared.next_company)(from);
                if company == u8::MAX {
                    break;
                }
                n = n.wrapping_add(u32::from((self.s.road_count)(company)));
                from = company + 1;
            }
            if n == 0 {
                return self.delete(v);
            }
            n = self.shared.random_range(n);
            let mut from = 0;
            loop {
                let target = (self.s.next_road)(from);
                if target.handle.is_null() {
                    break;
                }
                let selection = (self.s.road_selection)(target.shell());
                if selection.front {
                    let old = n;
                    n = n.wrapping_sub(1);
                    if old == 0 {
                        if unsafe { crate::road::crashed_counter(target.state) } != 0
                            || selection.disaster != INVALID_VEHICLE
                        {
                            return self.delete(v);
                        }
                        self.set_dest(v, target.id);
                        self.set_age(v, 0);
                        (self.s.write_road_disaster)(target.shell(), v.id);
                        break;
                    }
                }
                from = target.id + 1;
            }
            return true;
        }
        let target = (self.s.road_target)(self.dest(v));
        let t = (self.s.road_approach)(target.shell());
        let p = self.xy(v);
        let dist = distance(p.x, p.y, t.x, t.y) as u32;
        if dist < TILE_SIZE && t.status & VS_HIDDEN == 0 && t.breakdown == 0 {
            (self.s.write_breakdown)(target.shell(), 3, 140);
        }
        (self.v.write_direction)(v.shell(), (self.s.toward)(v.shell(), t.x, t.y));
        let gp = (self.v.new_position)(v.shell());
        let mut z = (self.v.z)(v.shell());
        if dist <= TILE_SIZE && z > (self.v.z)(target.shell()) {
            z = z.wrapping_sub(1);
        }
        self.update_position(v, gp.x, gp.y, z);
        if z <= (self.v.z)(target.shell()) {
            self.increment_age(v);
            if (self.v.status)(target.shell()) & VS_HIDDEN == 0
                && unsafe { crate::road::crashed_counter(target.state) } == 0
            {
                let victims = unsafe {
                    crate::road::openttd_rust_road_crash(
                        target.shell(),
                        target.state,
                        false,
                        self.s.road,
                    )
                };
                (self.s.write_road_disaster)(target.shell(), INVALID_VEHICLE);
                (self.s.small_ufo_news)((self.v.tile)(target.shell()));
                (self.s.road_crashed)(target.shell(), victims);
            }
        }
        if self.age(v) > 50 {
            Self::effect_rel(v, 0, 7, 8, crate::effectvehicle_func::EV_EXPLOSION_LARGE);
            self.sound_vehicle(v);
            return self.delete(v);
        }
        true
    }
    fn aircraft(&self, v: Vehicle, leave_top: bool) -> bool {
        let tick = self.increment_tick(v);
        v.set_image(if v.state() == 1 && tick & 4 != 0 {
            if leave_top {
                SPR_F_15_FIRING
            } else {
                SPR_AH_64A_FIRING
            }
        } else {
            0
        });
        let gp = (self.v.new_position)(v.shell());
        self.update_position(v, gp.x, gp.y, self.flight(v));
        if (leave_top && gp.x < -10 * TILE_SIZE as i32)
            || (!leave_top && gp.x > ((self.map().size_x() + 9) * TILE_SIZE - 1) as i32)
        {
            return self.delete(v);
        }
        if v.state() == 2 {
            if self.tick(v) & 3 == 0 {
                let i = (self.s.industry)(self.dest(v) as u16);
                let tile = (self.s.industry_tile)(i.industry);
                let map = self.map();
                let x = map.tile_x(tile) * TILE_SIZE;
                let y = map.tile_y(tile) * TILE_SIZE;
                let r = self.random();
                crate::effect::openttd_rust_effect_create_above(
                    ((r & 63) + x) as i32,
                    (((r >> 6) & 63) + y) as i32,
                    ((r >> 12) & 15) as i32,
                    crate::effectvehicle_func::EV_EXPLOSION_SMALL,
                );
                if self.increment_age(v) >= 55 {
                    v.set_state(3);
                }
            }
        } else if v.state() == 1 {
            if self.increment_age(v) == 112 {
                v.set_state(2);
                self.set_age(v, 0);
                let i = (self.s.industry)(self.dest(v) as u16);
                self.destruct_industry(i.industry);
                let town = (self.s.industry_town)(i.industry);
                if leave_top {
                    (self.s.airplane_news)(i.id, town);
                } else {
                    (self.s.helicopter_news)(i.id, town);
                }
                if (self.shared.disaster_sound)() {
                    (self.shared.play_sound)(
                        (self.s.industry_tile)(i.industry),
                        crate::sound_type::SND_12_EXPLOSION,
                    );
                }
            }
        } else if v.state() == 0 {
            let p = self.xy(v);
            let x =
                p.x.wrapping_add(if leave_top { -15 } else { 15 } * TILE_SIZE as i32);
            if x as u32 > self.map().max_x() * TILE_SIZE - 1 {
                return true;
            }
            let tile = self.map().tile_virt_xy(x as u32, p.y as u32);
            if (self.shared.tile_type)(tile) != MP_INDUSTRY {
                return true;
            }
            let industry = (self.s.industry_index)(tile);
            self.set_dest(v, u32::from(industry));
            let i = (self.s.industry)(industry);
            if if leave_top { i.airplane } else { i.helicopter } {
                v.set_state(1);
                self.set_age(v, 0);
            }
        }
        true
    }
    fn rotors(&self, v: Vehicle) -> bool {
        if self.increment_tick(v) & 1 != 0 {
            return true;
        }
        let image = (self.s.first_sprite)(v.shell()).wrapping_add(1);
        (self.s.write_first_sprite)(
            v.shell(),
            if image > SPR_ROTOR_MOVING_3 {
                SPR_ROTOR_MOVING_1
            } else {
                image
            },
        );
        self.viewport(v);
        true
    }
    fn valid_train(&self, v: Vehicle) -> bool {
        let t = (self.s.train_selection)(v.shell());
        t.front
            && (self.s.human)(t.owner)
            && (self.shared.is_plain_rail)(t.tile)
            && t.status & VS_CRASHED == 0
    }
    fn big_ufo(&self, v: Vehicle) -> bool {
        self.increment_tick(v);
        if v.state() == 1 {
            let dest = self.dest(v);
            let map = self.map();
            let x = (map.tile_x(dest) * TILE_SIZE + TILE_SIZE / 2) as i32;
            let y = (map.tile_y(dest) * TILE_SIZE + TILE_SIZE / 2) as i32;
            let p = self.xy(v);
            if distance(p.x, p.y, x, y) >= 8 {
                (self.v.write_direction)(v.shell(), (self.s.toward)(v.shell(), x, y));
                self.move_flight(v);
                return true;
            }
            if !(self.shared.tile_valid)(self.dest(v)) {
                return self.delete(v);
            }
            let p = (self.v.coordinates)(v.shell());
            let z = self.slope(p.x, p.y);
            if z < p.z {
                self.update_position(v, p.x, p.y, p.z.wrapping_sub(1));
                return true;
            }
            v.set_state(2);
            let mut from = 0;
            loop {
                let target = (self.s.next_vehicle)(from);
                if target.handle.is_null() {
                    break;
                }
                if (self.s.ground)(target.shell()) {
                    let t = self.xy(target);
                    let p = self.xy(v);
                    if distance(t.x, t.y, p.x, p.y) <= 12 * TILE_SIZE as i32 {
                        (self.s.write_breakdown)(target.shell(), 5, 0xF0);
                    }
                }
                from = target.id + 1;
            }
            let town = (self.shared.closest_town)(self.dest(v), u32::MAX);
            (self.s.big_ufo_news)(self.tile(v), (self.s.town_index)(town));
            if !(self.s.can_allocate)(2) {
                return self.delete(v);
            }
            let destroyer = (self.s.create)(
                -6 * TILE_SIZE as i32,
                (self.v.y)(v.shell()),
                DIR_SW,
                11,
                v.id,
            );
            let shadow = (self.s.create)(
                -6 * TILE_SIZE as i32,
                (self.v.y)(v.shell()),
                DIR_SW,
                12,
                INVALID_VEHICLE,
            );
            (self.s.set_next)(destroyer.shell(), shadow.shell());
        } else if v.state() == 0 {
            let map = self.map();
            let dest = self.dest(v);
            let p = self.xy(v);
            let x = (map.tile_x(dest) * TILE_SIZE) as i32;
            let y = (map.tile_y(dest) * TILE_SIZE) as i32;
            if distance(x, y, p.x, p.y) >= TILE_SIZE as i32 {
                (self.v.write_direction)(v.shell(), (self.s.toward)(v.shell(), x, y));
                self.move_flight(v);
                return true;
            }
            if self.increment_age(v) < 6 {
                self.set_dest(v, self.random_tile());
                return true;
            }
            v.set_state(1);
            let mut n = 0u32;
            let mut from = 0;
            loop {
                let train = (self.s.next_train)(from);
                if train.handle.is_null() {
                    break;
                }
                if self.valid_train(train) {
                    n = n.wrapping_add(1);
                }
                from = train.id + 1;
            }
            if n == 0 {
                return self.delete(v);
            }
            n = self.shared.random_range(n);
            from = 0;
            loop {
                let train = (self.s.next_train)(from);
                if train.handle.is_null() {
                    break;
                }
                if self.valid_train(train) {
                    let old = n;
                    n = n.wrapping_sub(1);
                    if old == 0 {
                        self.set_dest(v, self.tile(train));
                        self.set_age(v, 0);
                        break;
                    }
                }
                from = train.id + 1;
            }
        }
        true
    }
    fn destroyer(&self, v: Vehicle) -> bool {
        self.increment_tick(v);
        let gp = (self.v.new_position)(v.shell());
        self.update_position(v, gp.x, gp.y, self.flight(v));
        if gp.x > ((self.map().size_x() + 9) * TILE_SIZE - 1) as i32 {
            return self.delete(v);
        }
        if v.state() == 0 {
            let target = (self.s.resolve)(unsafe { (*v.state).target });
            if delta((self.v.x)(v.shell()), (self.v.x)(target.shell())) > TILE_SIZE as i32 {
                return true;
            }
            v.set_state(1);
            Self::effect_rel(
                target,
                0,
                7,
                8,
                crate::effectvehicle_func::EV_EXPLOSION_LARGE,
            );
            self.sound_vehicle(target);
            self.delete(target);
            for _ in 0..80 {
                let r = self.random();
                let p = self.xy(v);
                crate::effect::openttd_rust_effect_create_above(
                    (r & 63).wrapping_add(p.x as u32).wrapping_sub(32) as i32,
                    ((r >> 5) & 63).wrapping_add(p.y as u32).wrapping_sub(32) as i32,
                    0,
                    crate::effectvehicle_func::EV_EXPLOSION_SMALL,
                );
            }
            for dy in -3..3 {
                for dx in -3..3 {
                    let tile = (self.shared.tile_add_wrap)(self.tile(v), dx, dy);
                    if tile != INVALID_TILE {
                        self.clear(tile);
                    }
                }
            }
        }
        true
    }
    fn submarine(&self, v: Vehicle) -> bool {
        self.increment_tick(v);
        if self.increment_age(v) > 8880 {
            return self.delete(v);
        }
        if self.tick(v) & 1 == 0 {
            return true;
        }
        let dir = (self.v.direction)(v.shell());
        let tile = self.map().add_diag(
            self.tile(v),
            crate::direction_func::dir_to_diag_dir(dir, |valid| {
                if self.v.assertions {
                    assert!(valid);
                }
            }),
        );
        if (self.shared.tile_valid)(tile) {
            let bits = crate::movement::water_tracks(
                self.shared,
                tile,
                crate::direction_type::INVALID_DIAGDIR,
            );
            if bits == u32::from(crate::track_type::TRACK_BIT_ALL) && !self.chance(90) {
                let gp = (self.v.new_position)(v.shell());
                self.update_position(v, gp.x, gp.y, (self.v.z)(v.shell()));
                return true;
            }
        }
        let diff = if self.random() & 1 != 0 {
            DIRDIFF_90RIGHT
        } else {
            DIRDIFF_90LEFT
        };
        (self.v.write_direction)(
            v.shell(),
            crate::direction_func::change_dir((self.v.direction)(v.shell()), diff, |valid| {
                if self.v.assertions {
                    assert!(valid);
                }
            }),
        );
        true
    }
    fn tick_vehicle(&self, v: Vehicle) -> bool {
        match (self.v.subtype)(v.shell()) {
            0 => self.zeppelin(v),
            2 => self.small_ufo(v),
            4 => self.aircraft(v, true),
            6 => self.aircraft(v, false),
            8 => self.rotors(v),
            9 => self.big_ufo(v),
            11 => self.destroyer(v),
            13 | 14 => self.submarine(v),
            _ => true,
        }
    }
    fn initialize(&self, kind: u8) {
        let count = if kind == 3 {
            3
        } else if kind >= 5 {
            1
        } else {
            2
        };
        if kind != 7 && !(self.s.can_allocate)(count) {
            return;
        }
        match kind {
            0 | 1 | 4 => {
                let tile = self.random_tile();
                let mut x = (self.map().tile_x(tile) * TILE_SIZE + TILE_SIZE / 2) as i32;
                if kind == 0 {
                    let mut from = 0;
                    loop {
                        let st = (self.s.next_station)(from);
                        if st.next == u32::MAX {
                            break;
                        }
                        if st.tile != INVALID_TILE && (st.airport_type == 0 || st.airport_type == 1)
                        {
                            x = ((self.map().tile_x(st.tile) + 2) * TILE_SIZE) as i32;
                            break;
                        }
                        from = st.next;
                    }
                }
                let (y, dir, subtype) = if kind == 4 {
                    ((self.map().max_x() * TILE_SIZE - 1) as i32, DIR_NW, 9)
                } else {
                    (0, DIR_SE, kind * 2)
                };
                let v = (self.s.create)(x, y, dir, subtype, INVALID_VEHICLE);
                if kind != 0 {
                    self.set_dest(
                        v,
                        self.map()
                            .tile_xy(self.map().size_x() / 2, self.size_y() / 2),
                    );
                }
                let shadow = (self.s.create)(x, y, dir, subtype + 1, INVALID_VEHICLE);
                (self.s.set_next)(v.shell(), shadow.shell());
            }
            2 | 3 => {
                let mut found: *mut c_void = std::ptr::null_mut();
                let mut from = 0;
                loop {
                    let i = (self.s.next_industry)(from);
                    if i.industry.is_null() {
                        break;
                    }
                    if (if kind == 2 { i.airplane } else { i.helicopter })
                        && (found.is_null() || self.chance(2))
                    {
                        found = i.industry;
                    }
                    from = u32::from(i.id) + 1;
                }
                if found.is_null() {
                    return;
                }
                let x = if kind == 2 {
                    ((self.map().size_x() + 9) * TILE_SIZE - 1) as i32
                } else {
                    -16 * TILE_SIZE as i32
                };
                let y = (self.map().tile_y((self.s.industry_tile)(found)) * TILE_SIZE + 37) as i32;
                let dir = if kind == 2 { DIR_NE } else { DIR_SW };
                let subtype = if kind == 2 { 4 } else { 6 };
                let v = (self.s.create)(x, y, dir, subtype, INVALID_VEHICLE);
                let shadow = (self.s.create)(x, y, dir, subtype + 1, INVALID_VEHICLE);
                (self.s.set_next)(v.shell(), shadow.shell());
                if kind == 3 {
                    let rotor = (self.s.create)(x, y, dir, 8, INVALID_VEHICLE);
                    (self.s.set_next)(shadow.shell(), rotor.shell());
                }
            }
            5 | 6 => {
                let r = self.random();
                let tile = r & (self.map().size_x() * self.size_y() - 1);
                let x = (self.map().tile_x(tile) * TILE_SIZE + TILE_SIZE / 2) as i32;
                let (y, dir) = if r & 0x8000_0000 != 0 {
                    (
                        ((self.shared.map_max_y)() * TILE_SIZE - TILE_SIZE / 2 - 1) as i32,
                        DIR_NW,
                    )
                } else {
                    (
                        TILE_SIZE as i32 / 2
                            + if (unsafe { &*self.s.environment }.freeform_edges)() {
                                TILE_SIZE as i32
                            } else {
                                0
                            },
                        DIR_SE,
                    )
                };
                if (self.shared.tile_type)(self.map().tile_virt_xy(x as u32, y as u32)) != MP_WATER
                {
                    return;
                }
                (self.s.create)(x, y, dir, kind + 8, INVALID_VEHICLE);
            }
            7 => {
                let mut index = (self.random() & 15) as i32;
                for _ in 0..15 {
                    let mut from = 0;
                    loop {
                        let i = (self.s.next_industry)(from);
                        if i.industry.is_null() {
                            break;
                        }
                        if i.subsidence {
                            index = index.wrapping_sub(1);
                            if index < 0 {
                                let location = (self.s.industry_location)(i.industry);
                                (self.s.coal_news)(
                                    location.tile.wrapping_add(self.map().diff(1, 1)),
                                    location.town,
                                );
                                let mut tile = (self.s.industry_tile)(i.industry);
                                let dir = (self.random() & 3) as u8;
                                let step = self.map().offs_by_diag_dir(dir);
                                for _ in 0..30 {
                                    self.clear(tile);
                                    tile = tile.wrapping_add(step);
                                    if !(self.shared.tile_valid)(tile) {
                                        break;
                                    }
                                }
                                return;
                            }
                        }
                        from = u32::from(i.id) + 1;
                    }
                }
            }
            _ => {}
        }
    }
    fn reset_delay(&self) {
        unsafe {
            DELAY = ((self.random() & 511) + 730) as u16;
        }
    }
    fn daily(&self) {
        let delay = unsafe { DELAY.wrapping_sub(1) };
        unsafe {
            DELAY = delay;
        }
        if delay != 0 {
            return;
        }
        self.reset_delay();
        if !(self.s.disasters_enabled)() {
            return;
        }
        let year = (self.shared.calendar_year)();
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
        let mut available = Vec::new();
        for (index, (min, max)) in years.into_iter().enumerate() {
            if year >= min && year < max {
                available.push(index as u8);
            }
        }
        if !available.is_empty() {
            self.initialize(available[self.shared.random_range(available.len() as u32) as usize]);
        }
    }
    fn construct(&self, v: Vehicle, x: i32, y: i32, dir: u8, subtype: u8) {
        (self.v.write_status)(v.shell(), 4);
        (self.v.write_xy)(v.shell(), x, y);
        let z = match subtype {
            0 | 2 | 4 | 6 | 9 | 11 => self.bounds(v),
            8 => self.bounds(v).wrapping_add(5),
            13 | 14 => 0,
            _ => {
                (self.v.write_status)(v.shell(), (self.v.status)(v.shell()) | 32);
                0
            }
        };
        (self.v.write_z)(v.shell(), z);
        (self.v.write_direction)(v.shell(), dir);
        (self.v.write_tile)(v.shell(), self.map().tile_virt_xy(x as u32, y as u32));
        (self.s.write_subtype)(v.shell(), subtype);
        (self.s.update_delta)(v.shell());
        (self.s.write_owner)(v.shell(), crate::company_type::OWNER_NONE);
        v.set_image(0);
        v.set_state(0);
        self.update_image(v);
        self.viewport(v);
    }
    fn release_industry(&self, industry: u16) {
        let mut from = 0;
        loop {
            let v = (self.s.next_disaster)(from);
            if v.handle.is_null() {
                break;
            }
            let subtype = (self.v.subtype)(v.shell());
            if (subtype == 4 || subtype == 6)
                && v.state() > 0
                && self.dest(v) == u32::from(industry)
            {
                v.set_state(3);
            }
            from = v.id + 1;
        }
    }
    fn release_vehicle(&self, v: Vehicle) {
        v.set_state(0);
        self.set_dest(v, self.random_tile());
        (self.v.write_z)(v.shell(), self.bounds(v));
        self.set_age(v, 0);
    }
}
fn delta(a: i32, b: i32) -> i32 {
    if a < b {
        b.wrapping_sub(a)
    } else {
        a.wrapping_sub(b)
    }
}
fn distance(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    delta(ax, bx).wrapping_add(delta(ay, by))
}

/// Allocate the private owner once per native shell, including indexed load.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_state_create(target: u32) -> *mut State {
    Box::into_raw(Box::new(State {
        image_override: 0,
        target,
        state: 0,
        flags: 0,
    }))
}
/// # Safety
/// `state` is its live allocation with no remaining native alias use or Rust borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_state_destroy(state: *mut State) {
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// Process-wide address shared by modern and legacy save descriptors.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_delay() -> *mut u16 {
    &raw mut DELAY
}

// SAFETY: Each entry borrows only immutable tables, never native/owner storage.
// Shell/owner scalars use the original preconditions and end before callbacks.
macro_rules! entry {
    ($s:ident, $w:ident, $body:block) => {{
        let $w = unsafe { World::new(&*$s) };
        $body
    }};
}
/// # Safety
/// The services and live shell/owner satisfy the module's synchronous contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_tick(s: *const Services, v: Vehicle) -> bool {
    entry!(s, w, { w.tick_vehicle(v) })
}
/// # Safety
/// As `openttd_rust_disaster_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_daily(s: *const Services) {
    entry!(s, w, {
        w.daily();
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_startup(s: *const Services) {
    entry!(s, w, {
        w.reset_delay();
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`; `v` is the newly allocated native shell.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_construct(
    s: *const Services,
    v: Vehicle,
    x: i32,
    y: i32,
    dir: u8,
    subtype: u8,
) {
    entry!(s, w, {
        w.construct(v, x, y, dir, subtype);
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_position(
    s: *const Services,
    v: Vehicle,
    x: i32,
    y: i32,
    z: i32,
) {
    entry!(s, w, {
        w.update_position(v, x, y, z);
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_image(s: *const Services, v: Vehicle) {
    entry!(s, w, {
        w.update_image(v);
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`; industry deletion retains source ordering.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_release_industry(s: *const Services, industry: u16) {
    entry!(s, w, {
        w.release_industry(industry);
    });
}
/// # Safety
/// As `openttd_rust_disaster_tick`; native facade performs source validity/assertions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_release_vehicle(s: *const Services, v: Vehicle) {
    entry!(s, w, {
        w.release_vehicle(v);
    });
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDisasterState" ["probe"] => State { image_override, target, state, flags, };
    "OpenTTDDisasterVehicle" ["probe"] => Vehicle { handle, state, id, };
    "OpenTTDDisasterRoadTarget" ["probe"] => RoadTarget { handle, state, id, };
    "OpenTTDDisasterIndustryTarget" ["probe"] => IndustryTarget { industry, id, airplane, helicopter, subsidence, };
    "OpenTTDDisasterIndustryLocation" ["probe"] => IndustryLocation { tile, town, };
    "OpenTTDDisasterStationTarget" ["probe"] => StationTarget { next, tile, airport_type, };
    "OpenTTDDisasterRoadSelection" ["probe"] => RoadSelection { front, disaster, };
    "OpenTTDDisasterRoadApproach" ["probe"] => RoadApproach { x, y, status, breakdown, };
    "OpenTTDDisasterTrainSelection" ["probe"] => TrainSelection { front, owner, tile, status, };
    "OpenTTDDisasterServices" ["probe"] => Services { shared, vehicles, occupancy, environment, rail_map, aircraft, road, age, write_age, write_sprite, first_sprite, write_first_sprite, write_subtype, write_owner, set_next, next_part, update_delta, toward, create, resolve, next_disaster, next_road, road_target, road_selection, road_approach, write_road_disaster, write_breakdown, next_vehicle, ground, next_train, train_selection, road_count, human, next_station, next_industry, industry, industry_location, industry_tile, industry_town, industry_index, industry_belongs, reset_industry_tile, airport_tile, airport_blocks, zeppelin_crashed, zeppelin_cleared, road_crashed, zeppelin_news, small_ufo_news, airplane_news, helicopter_news, big_ufo_news, coal_news, clear, update_signals, disasters_enabled, can_allocate, town_index, };
}

#[cfg(test)]
mod tests {
    #[test]
    fn persisted_delay_is_process_wide_across_serialized_threads() {
        let address = super::openttd_rust_disaster_delay() as usize;
        // SAFETY: The worker runs after this access and is joined before the next.
        unsafe {
            (address as *mut u16).write(41);
        }
        std::thread::spawn(move || {
            let worker = super::openttd_rust_disaster_delay();
            assert_eq!(worker as usize, address);
            unsafe {
                assert_eq!(worker.read(), 41);
                worker.write(73);
            }
        })
        .join()
        .unwrap();
        unsafe {
            assert_eq!((address as *const u16).read(), 73);
        }
    }
}

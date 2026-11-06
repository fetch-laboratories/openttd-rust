/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine handles borrow one live canonical record for a serialized entry. Each method
//! reads fields without retaining a record reference across callbacks/drop.
#![allow(unsafe_code)]
use crate::engine::Lifecycle;
use crate::raw_field::read_field;
use crate::vehicle::{EngineRef, RailVehicleTypes, VehicleType};
use std::ptr::NonNull;
impl EngineRef {
    pub(crate) fn of(record: *const Lifecycle) -> Self {
        Self(NonNull::new(record.cast_mut()).expect("live engine"))
    }
    pub(crate) fn flags(self) -> u8 {
        read_field!(self.0.as_ptr(), flags)
    }
    pub(crate) fn kind(self) -> VehicleType {
        read_field!(self.0.as_ptr(), r#type)
    }
    pub(crate) fn default_cargo(self) -> u8 {
        read_field!(self.0.as_ptr(), info.cargo_type)
    }
    pub(crate) fn misc(self) -> u8 {
        read_field!(self.0.as_ptr(), info.misc_flags)
    }
    pub(crate) fn callbacks(self) -> u16 {
        read_field!(self.0.as_ptr(), info.callback_mask)
    }
    pub(crate) fn refit_mask(self) -> u64 {
        read_field!(self.0.as_ptr(), info.refit_mask)
    }
    pub(crate) fn life_years(self) -> i32 {
        read_field!(self.0.as_ptr(), info.lifelength)
    }
    pub(crate) fn grf(self) -> *const std::ffi::c_void {
        read_field!(self.0.as_ptr(), bindings.grf_prop.grffile)
    }
    pub(crate) fn ship_capacity(self) -> u16 {
        read_field!(Lifecycle::ship(self.0.as_ptr()), capacity)
    }
    pub(crate) fn passenger_capacity(self) -> u16 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), passenger_capacity)
    }
    pub(crate) fn mail_capacity(self) -> u8 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), mail_capacity)
    }
    pub(crate) fn train_class(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), engclass)
    }
    pub(crate) fn aircraft_subtype(self) -> u8 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), subtype)
    }
    pub(crate) fn road_sound(self) -> u16 {
        read_field!(Lifecycle::road(self.0.as_ptr()), sfx)
    }
    pub(crate) fn rail_kind(self) -> RailVehicleTypes {
        RailVehicleTypes(read_field!(Lifecycle::rail(self.0.as_ptr()), railveh_type))
    }
    pub(crate) fn aircraft_cost(self) -> u8 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), cost_factor)
    }
    pub(crate) fn aircraft_range(self) -> u16 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), max_range)
    }
    pub(crate) fn aircraft_running(self) -> u8 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), running_cost)
    }
    pub(crate) fn road_cost(self) -> u8 {
        read_field!(Lifecycle::road(self.0.as_ptr()), cost_factor)
    }
    pub(crate) fn road_price(self) -> u8 {
        read_field!(Lifecycle::road(self.0.as_ptr()), running_cost_class)
    }
    pub(crate) fn road_running(self) -> u8 {
        read_field!(Lifecycle::road(self.0.as_ptr()), running_cost)
    }
    pub(crate) fn ship_cost(self) -> u8 {
        read_field!(Lifecycle::ship(self.0.as_ptr()), cost_factor)
    }
    pub(crate) fn ship_running(self) -> u8 {
        read_field!(Lifecycle::ship(self.0.as_ptr()), running_cost)
    }
    pub(crate) fn train_cost(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), cost_factor)
    }
    pub(crate) fn train_price(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), running_cost_class)
    }
    pub(crate) fn train_running(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), running_cost)
    }
    pub(crate) fn climate(self, climate: u8) -> bool {
        read_field!(self.0.as_ptr(), info.climates) & (1 << climate) != 0
    }
    pub(crate) fn railtypes(self) -> u64 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), railtypes)
    }
    pub(crate) fn roadtype(self) -> u8 {
        read_field!(Lifecycle::road(self.0.as_ptr()), roadtype)
    }
    pub(crate) fn refit_cost(self) -> u8 {
        read_field!(self.0.as_ptr(), info.refit_cost)
    }
    pub(crate) fn visual_effect(self) -> u8 {
        match self.kind() {
            VehicleType::TRAIN => read_field!(Lifecycle::rail(self.0.as_ptr()), visual_effect),
            VehicleType::ROAD => read_field!(Lifecycle::road(self.0.as_ptr()), visual_effect),
            VehicleType::SHIP => read_field!(Lifecycle::ship(self.0.as_ptr()), visual_effect),
            _ => 1 << crate::engine_type::VE_DISABLE_EFFECT,
        }
    }
    pub(crate) fn grf_version(self, shared: &crate::services::Services) -> u8 {
        let file = self.grf();
        if file.is_null() {
            0
        } else {
            (shared.grf_file_version)(file)
        }
    }
    pub(crate) fn ground_power(self, kind: VehicleType) -> u16 {
        if kind == VehicleType::TRAIN {
            read_field!(Lifecycle::rail(self.0.as_ptr()), power)
        } else {
            u16::from(read_field!(Lifecycle::road(self.0.as_ptr()), power))
        }
    }
    pub(crate) fn ground_weight(self, kind: VehicleType) -> u16 {
        if kind == VehicleType::TRAIN {
            read_field!(Lifecycle::rail(self.0.as_ptr()), weight)
        } else {
            u16::from(read_field!(Lifecycle::road(self.0.as_ptr()), weight))
        }
    }
    pub(crate) fn ground_te(self, kind: VehicleType) -> u8 {
        if kind == VehicleType::TRAIN {
            read_field!(Lifecycle::rail(self.0.as_ptr()), tractive_effort)
        } else {
            read_field!(Lifecycle::road(self.0.as_ptr()), tractive_effort)
        }
    }
    pub(crate) fn ground_drag(self, kind: VehicleType) -> u8 {
        if kind == VehicleType::TRAIN {
            read_field!(Lifecycle::rail(self.0.as_ptr()), air_drag)
        } else {
            read_field!(Lifecycle::road(self.0.as_ptr()), air_drag)
        }
    }
    pub(crate) fn wagon_power(self) -> u16 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), pow_wag_power)
    }
    pub(crate) fn wagon_weight(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), pow_wag_weight)
    }
    pub(crate) fn compatible_railtypes(self, shared: &crate::services::Services) -> u64 {
        (shared.all_compatible_railtypes)(self.railtypes())
    }
    pub(crate) fn powered_roadtypes(self, shared: &crate::services::Services) -> u64 {
        (shared.powered_roadtypes)(crate::services::RoadType(self.roadtype()))
    }
    pub(crate) fn cargo_age(self) -> u16 {
        read_field!(self.0.as_ptr(), info.cargo_age_period)
    }
    pub(crate) fn train_curve_mod(self) -> i16 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), curve_speed_mod)
    }
    pub(crate) fn train_user_data(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), user_def_data)
    }
    pub(crate) fn train_shorten(self) -> u8 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), shorten_factor)
    }
    pub(crate) fn train_speed(self) -> u16 {
        read_field!(Lifecycle::rail(self.0.as_ptr()), max_speed)
    }
    pub(crate) fn road_shorten(self) -> u8 {
        read_field!(Lifecycle::road(self.0.as_ptr()), shorten_factor)
    }
    pub(crate) fn road_speed(self) -> u16 {
        read_field!(Lifecycle::road(self.0.as_ptr()), max_speed)
    }
    pub(crate) fn aircraft_speed(self) -> u16 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), max_speed)
    }
    pub(crate) fn aircraft_sound(self) -> u16 {
        read_field!(Lifecycle::aircraft(self.0.as_ptr()), sfx)
    }
    pub(crate) fn ship_speed(self) -> u16 {
        read_field!(Lifecycle::ship(self.0.as_ptr()), max_speed)
    }
    pub(crate) fn ship_speed_fraction(self, sea: bool) -> u8 {
        if sea {
            read_field!(Lifecycle::ship(self.0.as_ptr()), ocean_speed_frac)
        } else {
            read_field!(Lifecycle::ship(self.0.as_ptr()), canal_speed_frac)
        }
    }
    pub(crate) fn exclusive_preview(self) -> bool {
        self.flags() & super::EXCLUSIVE != 0
    }
    pub(crate) fn ground(self) -> bool {
        matches!(self.kind(), VehicleType::TRAIN | VehicleType::ROAD)
    }
    pub(crate) fn carrying(self) -> crate::engine::Carrying {
        let kind = self.kind();
        let ground_capacity = match kind {
            VehicleType::TRAIN => read_field!(Lifecycle::rail(self.0.as_ptr()), capacity),
            VehicleType::ROAD => read_field!(Lifecycle::road(self.0.as_ptr()), capacity),
            _ => 0,
        };
        let default_cargo =
            if matches!(kind, VehicleType::TRAIN | VehicleType::ROAD) && ground_capacity == 0 {
                crate::cargo_type::INVALID_CARGO
            } else {
                self.default_cargo()
            };
        crate::engine::Carrying {
            kind,
            ground_capacity,
            default_cargo,
        }
    }
    pub(crate) fn capacity_config(self, id: u16) -> crate::engine::CapacityConfig {
        crate::engine::CapacityConfig {
            id,
            callbacks: self.callbacks(),
            misc: read_field!(self.0.as_ptr(), info.misc_flags),
        }
    }
    pub(crate) fn valid_string(self, invalid: u32) -> bool {
        read_field!(self.0.as_ptr(), info.string_id) != invalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::records::{
        AircraftVehicleInfo, RailVehicleInfo, RoadVehicleInfo, ShipVehicleInfo, VehicleInfo,
    };
    /// Carry the original native capability fixture's observations across reader retirement.
    #[test]
    fn carrying_and_config_preserve_type_and_zero_capacity_gates() {
        for (kind, capacity, cargo, observed_cargo) in [
            (VehicleType::TRAIN, 0, 0, 255),
            (VehicleType::TRAIN, 255, 0, 0),
            (VehicleType::ROAD, 0, 0, 255),
            (VehicleType::ROAD, 255, 255, 255),
            (VehicleType::SHIP, 0, 0, 0),
            (VehicleType::AIRCRAFT, 0, 255, 255),
        ] {
            let mut record = Lifecycle::default();
            record.r#type = kind;
            record.vehicle_info_type = kind;
            record.vehicle_info = match kind {
                VehicleType::TRAIN => VehicleInfo {
                    rail: RailVehicleInfo {
                        capacity,
                        ..RailVehicleInfo::default()
                    },
                },
                VehicleType::ROAD => VehicleInfo {
                    road: RoadVehicleInfo {
                        capacity,
                        ..RoadVehicleInfo::default()
                    },
                },
                VehicleType::SHIP => VehicleInfo {
                    ship: ShipVehicleInfo::default(),
                },
                VehicleType::AIRCRAFT => VehicleInfo {
                    aircraft: AircraftVehicleInfo::default(),
                },
                _ => unreachable!(),
            };
            record.info.cargo_type = cargo;
            let engine = EngineRef::of(&raw const record);
            let carrying = engine.carrying();
            assert_eq!(carrying.kind.0, kind.0);
            assert_eq!(carrying.ground_capacity, capacity);
            assert_eq!(carrying.default_cargo, observed_cargo);
            record.info.callback_mask = 24;
            record.info.misc_flags = 96;
            let config = engine.capacity_config(60000);
            assert_eq!((config.id, config.callbacks, config.misc), (60000, 24, 96));
        }
    }
}

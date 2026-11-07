/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine specification payloads and original constructor policy. The active tag gates
//! every union read, including native monostate prototypes. Access is serialized and
//! no record borrow survives mutation, native reentry or destruction.
#![allow(unsafe_code, clippy::struct_field_names)]
use crate::engine_type::EngineID;
use crate::newgrf::CargoLabel;
use crate::vehicle_type::VehicleType;
crate::abi::shared_records! {
    fn abi_record;
    /// `RailVehicleInfo` fields with their native widths.
    #[derive(Clone, Copy)]
    "RailVehicleInfo" ["engine_spec"] => pub struct RailVehicleInfo {
        pub image_index: u8,
        pub railveh_type: u8 => "RailVehicleTypes",
        pub cost_factor: u8,
        pub railtypes: u64 => "RailTypes",
        pub intended_railtypes: u64 => "RailTypes",
        pub ai_passenger_only: u8,
        pub max_speed: u16,
        pub power: u16,
        pub weight: u16,
        pub running_cost: u8,
        pub running_cost_class: u8 => "Price",
        pub engclass: u8 => "EngineClass",
        pub capacity: u8,
        pub pow_wag_power: u16,
        pub pow_wag_weight: u8,
        pub visual_effect: u8,
        pub shorten_factor: u8,
        pub tractive_effort: u8,
        pub air_drag: u8,
        pub user_def_data: u8,
        pub curve_speed_mod: i16,
    }
    /// `RoadVehicleInfo` fields with their native widths.
    #[derive(Clone, Copy)]
    "RoadVehicleInfo" ["engine_spec"] => pub struct RoadVehicleInfo {
        pub image_index: u8,
        pub cost_factor: u8,
        pub running_cost: u8,
        pub running_cost_class: u8 => "Price",
        pub sfx: u16 => "SoundID",
        pub max_speed: u16,
        pub capacity: u8,
        pub weight: u8,
        pub power: u8,
        pub tractive_effort: u8,
        pub air_drag: u8,
        pub visual_effect: u8,
        pub shorten_factor: u8,
        pub roadtype: u8 => "RoadType",
    }
    /// `ShipVehicleInfo` fields with their native widths.
    #[derive(Clone, Copy)]
    "ShipVehicleInfo" ["engine_spec"] => pub struct ShipVehicleInfo {
        pub image_index: u8,
        pub cost_factor: u8,
        pub running_cost: u8,
        pub acceleration: u8,
        pub max_speed: u16,
        pub capacity: u16,
        pub sfx: u16 => "SoundID",
        pub old_refittable: bool,
        pub visual_effect: u8,
        pub ocean_speed_frac: u8,
        pub canal_speed_frac: u8,
    }
    /// `AircraftVehicleInfo` fields with their native widths.
    #[derive(Clone, Copy, Default)]
    "AircraftVehicleInfo" ["engine_spec"] => pub struct AircraftVehicleInfo {
        pub image_index: u8,
        pub cost_factor: u8,
        pub running_cost: u8,
        pub subtype: u8,
        pub sfx: u16 => "SoundID",
        pub max_speed: u16,
        pub acceleration: u8,
        pub mail_capacity: u8,
        pub passenger_capacity: u16,
        pub max_range: u16,
    }
    /// `EngineInfo` fields with their native widths.
    #[derive(Clone, Copy)]
    "EngineInfo" ["engine_spec"] => pub struct EngineInfo {
        pub base_intro: i32 => "TimerGameCalendar::Date",
        pub lifelength: i32 => "TimerGameCalendar::Year",
        pub base_life: i32 => "TimerGameCalendar::Year",
        pub decay_speed: u8,
        pub load_amount: u8,
        pub climates: u8 => "LandscapeTypes",
        pub cargo_type: u8 => "CargoType",
        pub cargo_label: CargoLabel => "OpenTTDNewGRFCargoLabel",
        pub refit_mask: u64 => "CargoTypes",
        pub refit_cost: u8,
        pub misc_flags: u8 => "EngineMiscFlags",
        pub callback_mask: u16 => "VehicleCallbackMasks",
        pub retire_early: i8,
        pub extra_flags: u8 => "ExtraEngineFlags",
        pub string_id: u32 => "StringID",
        pub cargo_age_period: u16,
        pub variant_id: EngineID => "EngineID",
    }
    /// The original variant payload; monostate is represented by its separate invalid tag.
    #[derive(Clone, Copy)]
    "OpenTTDEngineVehicleInfo" ["engine_spec"] => pub union VehicleInfo {
        pub rail: RailVehicleInfo,
        pub road: RoadVehicleInfo,
        pub ship: ShipVehicleInfo,
        pub aircraft: AircraftVehicleInfo,
    }
    /// Borrowed original tables and string IDs, retained by native phase-4 data consumers.
    "OpenTTDEngineDefaults" ["engine_spec"] => pub struct Defaults {
        pub info: *const EngineInfo,
        pub rail: *const RailVehicleInfo,
        pub road: *const RoadVehicleInfo,
        pub ship: *const ShipVehicleInfo,
        pub aircraft: *const AircraftVehicleInfo,
        pub counts: [u8; 4] => "std::array<uint8_t, 4>",
        pub offsets: [u8; 4] => "std::array<uint8_t, 4>",
        pub first_names: [u32; 4] => "std::array<StringID, 4>",
    }
}
impl Default for RailVehicleInfo {
    fn default() -> Self {
        Self {
            image_index: 0,
            railveh_type: crate::vehicle::RailVehicleTypes::WAGON.0,
            cost_factor: 0,
            railtypes: 1 << crate::rail_type::RAILTYPE_RAIL,
            intended_railtypes: 1 << crate::rail_type::RAILTYPE_RAIL,
            ai_passenger_only: 0,
            max_speed: 0,
            power: 0,
            weight: 0,
            running_cost: 0,
            running_cost_class: 0,
            engclass: 0,
            capacity: 0,
            pow_wag_power: 0,
            pow_wag_weight: 0,
            visual_effect: crate::engine_type::VE_DEFAULT,
            shorten_factor: 0,
            tractive_effort: 0,
            air_drag: 0,
            user_def_data: 0,
            curve_speed_mod: 0,
        }
    }
}
impl Default for RoadVehicleInfo {
    fn default() -> Self {
        Self {
            image_index: 0,
            cost_factor: 0,
            running_cost: 0,
            running_cost_class: 0,
            sfx: 0,
            max_speed: 0,
            capacity: 0,
            weight: 0,
            power: 0,
            tractive_effort: 0x4C,
            air_drag: 0,
            visual_effect: crate::engine_type::VE_DEFAULT,
            shorten_factor: 0,
            roadtype: 0,
        }
    }
}
impl Default for ShipVehicleInfo {
    fn default() -> Self {
        Self {
            image_index: 0,
            cost_factor: 0,
            running_cost: 0,
            acceleration: 1,
            max_speed: 0,
            capacity: 0,
            sfx: 0,
            old_refittable: false,
            visual_effect: crate::engine_type::VE_DEFAULT,
            ocean_speed_frac: 0,
            canal_speed_frac: 0,
        }
    }
}
impl Default for EngineInfo {
    fn default() -> Self {
        Self {
            base_intro: 0,
            lifelength: 0,
            base_life: 0,
            decay_speed: 0,
            load_amount: 0,
            climates: 0,
            cargo_type: 0,
            cargo_label: CargoLabel {
                value: 0,
                mixed: false,
            },
            refit_mask: 0,
            refit_cost: 0,
            misc_flags: 0,
            callback_mask: 0,
            retire_early: 0,
            extra_flags: 0,
            string_id: crate::strings_type::INVALID_STRING_ID,
            cargo_age_period: 0,
            variant_id: EngineID(0),
        }
    }
}
impl Default for VehicleInfo {
    fn default() -> Self {
        Self {
            rail: RailVehicleInfo::default(),
        }
    }
}
/// Original `Engine::Engine(type, local_id)`; the shell owns a unique live record.
/// # Safety
/// Native tables have their original sizes; type is a company vehicle type, and
/// the unique owner has its original default fields. No observer runs mid-construction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_construct(
    record: *mut super::Lifecycle,
    tables: &Defaults,
    kind: VehicleType,
    local: u16,
) {
    let index = usize::from(kind.0);
    // SAFETY: unique live constructor owner and borrowed original tables.
    let record = unsafe { &mut *record };
    record.r#type = kind;
    record.bindings.grf_prop.local_id = local;
    record.list_position = local;
    record.preview_company = 255;
    if usize::from(local) >= usize::from(tables.counts[index]) {
        record.vehicle_info = match kind {
            VehicleType::TRAIN => VehicleInfo {
                rail: RailVehicleInfo::default(),
            },
            VehicleType::ROAD => VehicleInfo {
                road: RoadVehicleInfo::default(),
            },
            VehicleType::SHIP => VehicleInfo {
                ship: ShipVehicleInfo::default(),
            },
            VehicleType::AIRCRAFT => VehicleInfo {
                aircraft: AircraftVehicleInfo::default(),
            },
            _ => unreachable!(),
        };
        record.vehicle_info_type = kind;
        record.info.base_life = 255;
        record.info.cargo_type = crate::cargo_type::INVALID_CARGO;
        record.info.cargo_label = CargoLabel {
            value: if kind == VehicleType::AIRCRAFT {
                crate::cargo_type::CT_INVALID
            } else {
                crate::cargo_type::CT_PASSENGERS
            },
            mixed: false,
        };
        record.info.cargo_age_period = crate::timer_game_tick::CARGO_AGING_TICKS;
        record.info.variant_id = EngineID::INVALID;
        return;
    }
    unsafe {
        record.info = *tables
            .info
            .add(usize::from(tables.offsets[index]) + usize::from(local));
        match kind {
            VehicleType::TRAIN => {
                let rail = *tables.rail.add(usize::from(local));
                record.vehicle_info = VehicleInfo { rail };
                record.vehicle_info_type = kind;
                record.original_image_index = rail.image_index;
                record.info.string_id = tables.first_names[index].wrapping_add(u32::from(local));
                if rail.railveh_type == crate::vehicle::RailVehicleTypes::WAGON.0 {
                    record.info.base_life = 255;
                }
            }
            VehicleType::ROAD => {
                let road = *tables.road.add(usize::from(local));
                record.vehicle_info = VehicleInfo { road };
                record.vehicle_info_type = kind;
                record.original_image_index = road.image_index;
                record.info.string_id = tables.first_names[index].wrapping_add(u32::from(local));
            }
            VehicleType::SHIP => {
                let ship = *tables.ship.add(usize::from(local));
                record.vehicle_info = VehicleInfo { ship };
                record.vehicle_info_type = kind;
                record.original_image_index = ship.image_index;
                record.info.string_id = tables.first_names[index].wrapping_add(u32::from(local));
            }
            VehicleType::AIRCRAFT => {
                let aircraft = *tables.aircraft.add(usize::from(local));
                record.vehicle_info = VehicleInfo { aircraft };
                record.vehicle_info_type = kind;
                record.original_image_index = aircraft.image_index;
                record.info.string_id = tables.first_names[index].wrapping_add(u32::from(local));
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn out_of_range_prototypes_match_original() {
        // Frozen Engine(type, UINT16_MAX) observations from unchanged 15.3,
        // 14ec60f248547d4d062a1160f0fc26d742319888, GNU/Linux x86_64.
        // All inputs take the out-of-range path; the production constructor must
        // return before reading any of the original specification table pointers.
        let tables = Defaults {
            info: std::ptr::null(),
            rail: std::ptr::null(),
            road: std::ptr::null(),
            ship: std::ptr::null(),
            aircraft: std::ptr::null(),
            counts: [116, 88, 11, 41],
            offsets: [0, 116, 204, 215],
            first_names: [0; 4],
        };
        let mut observed = String::new();
        for kind in [
            VehicleType::TRAIN,
            VehicleType::ROAD,
            VehicleType::SHIP,
            VehicleType::AIRCRAFT,
        ] {
            let mut record = super::super::Lifecycle::default();
            // SAFETY: A unique live record, a company vehicle type and an index
            // outside every original table. The constructor reads no table data.
            unsafe {
                openttd_rust_engine_construct(&raw mut record, &tables, kind, u16::MAX);
            }
            assert_eq!(record.vehicle_info_type.0, kind.0);
            assert!(!record.info.cargo_label.mixed);
            let extra = match kind {
                VehicleType::TRAIN => {
                    // SAFETY: The constructor has installed the rail variant.
                    let rail = unsafe { record.vehicle_info.rail };
                    assert_eq!(rail.railtypes, 1);
                    rail.railveh_type
                }
                // SAFETY: Each arm reads only the variant installed for kind.
                VehicleType::ROAD => unsafe { record.vehicle_info.road }.tractive_effort,
                // SAFETY: The constructor has installed the ship variant.
                VehicleType::SHIP => unsafe { record.vehicle_info.ship }.acceleration,
                // SAFETY: The constructor has installed the aircraft variant.
                VehicleType::AIRCRAFT => unsafe { record.vehicle_info.aircraft }.subtype,
                _ => unreachable!(),
            };
            writeln!(
                observed,
                "prototype {} {} {} {} {} {} {}",
                record.r#type.0,
                record.info.base_life,
                record.info.cargo_type,
                record.info.cargo_label.value,
                record.info.cargo_age_period,
                record.info.variant_id.0,
                extra
            )
            .unwrap();
        }
        assert_eq!(observed, include_str!("records/prototype-oracle.txt"));
    }
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action 3 dispatch and ordered sprite bindings. Engine history is process-wide
//! and survives loader resets. No history borrow spans native engine operations.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
use crate::debug::grf_msg;
use crate::engine_type::EngineID;
use crate::game_state::GameState;
use crate::newgrf::{
    CF_END, GSF_AIRCRAFT, GSF_AIRPORTS, GSF_AIRPORTTILES, GSF_BADGES, GSF_CANALS, GSF_CARGOES,
    GSF_DEFAULT, GSF_END, GSF_HOUSES, GSF_INDUSTRIES, GSF_INDUSTRYTILES, GSF_OBJECTS,
    GSF_RAILTYPES, GSF_ROADSTOPS, GSF_ROADTYPES, GSF_ROADVEHICLES, GSF_SHIPS, GSF_STATIONS,
    GSF_TRAINS, GSF_TRAMTYPES,
};
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_commons::{SG_DEFAULT, StandardSpriteGroup};
use crate::newgrf_loading::{self as loading, Services};
use crate::newgrf_specs::SpecServices;
use std::ffi::c_void;

static LAST_ENGINES: GameState<Vec<EngineID>> = GameState::new(Vec::new());
unsafe fn resize_last(count: u8) {
    unsafe { &mut *LAST_ENGINES.get() }.resize(usize::from(count), EngineID(0));
}
unsafe fn remember_last(index: usize, engine: EngineID) {
    unsafe {
        (&mut *LAST_ENGINES.get())[index] = engine;
    }
}
unsafe fn group(id: u16) -> *const c_void {
    unsafe { (*loading::openttd_rust_grf_execution()).spritegroups[usize::from(id)] }
}
unsafe fn valid_group(id: u16, function: &str) -> bool {
    if u32::from(id) > crate::newgrf_internal::MAX_SPRITEGROUP || unsafe { group(id) }.is_null() {
        grf_msg!(
            1,
            "{function}: Spritegroup 0x{id:04X} out of range or empty, skipping."
        );
        false
    } else {
        true
    }
}
unsafe fn vehicle_map(
    s: &Services,
    buf: &mut ByteReader,
    feature: u8,
    mut count: u8,
) -> Result<(), ReadBounds> {
    unsafe {
        let bindings = &*s.bindings;
        let override_wagon = count & 0x80 != 0;
        if override_wagon {
            count &= 0x7f;
            let size = (&*LAST_ENGINES.get()).len();
            if size == 0 {
                grf_msg!(
                    0,
                    "VehicleMapSpriteGroup: WagonOverride: No engine to do override with"
                );
                return Ok(());
            }
            grf_msg!(
                6,
                "VehicleMapSpriteGroup: WagonOverride: {size} engines, {count} wagons"
            );
        } else {
            resize_last(count);
        }
        let file = loading::file();
        let mut engines = Vec::with_capacity(usize::from(count));
        for i in 0..count {
            let local = buf.extended()?;
            let engine = crate::engine_loading::openttd_rust_engine_definition(
                &*s.engine, file, feature, local, false,
            );
            if engine.is_null() {
                loading::disable_invalid_id(s);
                return Ok(());
            }
            let id = EngineID(((*(*s.engine).resolver).capabilities.id)(engine));
            engines.push(id);
            if !override_wagon {
                remember_last(usize::from(i), id);
            }
        }
        let cargos = buf.byte()?;
        for cargo_index in 0..cargos {
            let cargo = buf.byte()?;
            let id = buf.word()?;
            if !valid_group(id, "VehicleMapSpriteGroup") {
                continue;
            }
            grf_msg!(
                8,
                "VehicleMapSpriteGroup: * [{cargo_index}] Cargo type 0x{cargo:X}, group id 0x{id:02X}"
            );
            let cargo = super::translate_cargo(feature, cargo, file);
            if !crate::cargo_type::is_valid_cargo_type(cargo) {
                continue;
            }
            for (i, engine) in engines.iter().enumerate() {
                grf_msg!(7, "VehicleMapSpriteGroup: [{i}] Engine {}...", engine.0);
                if override_wagon {
                    let (last, len) = {
                        let last = &mut *LAST_ENGINES.get();
                        (last.as_mut_ptr(), last.len())
                    };
                    (bindings.wagon_override)(engine.0, cargo, group(id), last, len);
                } else {
                    (bindings.custom_engine_sprites)(engine.0, cargo, group(id));
                }
            }
        }
        let id = buf.word()?;
        if !valid_group(id, "VehicleMapSpriteGroup") {
            return Ok(());
        }
        grf_msg!(8, "-- Default group id 0x{id:04X}");
        for engine in engines {
            if override_wagon {
                let (last, len) = {
                    let last = &mut *LAST_ENGINES.get();
                    (last.as_mut_ptr(), last.len())
                };
                (bindings.wagon_override)(engine.0, SG_DEFAULT, group(id), last, len);
            } else {
                (bindings.custom_engine_sprites)(engine.0, SG_DEFAULT, group(id));
                (bindings.engine_grf)(engine.0, file);
            }
        }
        Ok(())
    }
}
fn bind_standard<S, T>(
    table: &SpecServices<S, T>,
    spec: *mut S,
    local: u16,
    cargo: Option<u8>,
    group: *const c_void,
) {
    if spec.is_null() {
        grf_msg!(1, "MapSpriteGroup: {local} undefined, skipping.");
    } else if cargo.is_some() {
        (table.standard().set_sprite_group)(spec, StandardSpriteGroup::Purchase as u8, group);
    } else {
        (table.standard().set_sprite_group)(spec, StandardSpriteGroup::Default as u8, group);
        (table.set_file)(spec, unsafe { loading::file() });
        (table.set_local_id)(spec, local);
    }
}
unsafe fn map_binding(
    s: &Services,
    feature: u8,
    local: u16,
    cargo: Option<u8>,
    group: *const c_void,
) {
    unsafe {
        let file = loading::file();
        let bindings = &*s.bindings;
        let specs = s.shared().specs();
        if matches!(
            feature,
            GSF_HOUSES
                | GSF_INDUSTRIES
                | GSF_INDUSTRYTILES
                | GSF_AIRPORTS
                | GSF_AIRPORTTILES
                | GSF_OBJECTS
        ) && cargo.is_some_and(|c| c != 0xff)
        {
            grf_msg!(
                1,
                "MapSpriteGroup: Invalid cargo bitnum {}, skipping.",
                cargo.unwrap()
            );
            return;
        }
        match feature {
            GSF_HOUSES => {
                let owner = &*s.houses;
                let spec = if usize::from(local) >= (owner.local_count)(file) {
                    std::ptr::null_mut()
                } else {
                    (owner.local)(file, usize::from(local)).cast_mut().cast()
                };
                bind_standard(specs.house(), spec, local, cargo, group);
            }
            GSF_INDUSTRIES => {
                let owner = &*s.industry;
                let spec = if usize::from(local) >= (owner.industry_count)(file) {
                    std::ptr::null_mut()
                } else {
                    (owner.file_industry)(file, usize::from(local))
                };
                bind_standard(specs.industry(), spec, local, cargo, group);
            }
            GSF_INDUSTRYTILES => {
                let owner = &*s.industry;
                let spec = if usize::from(local) >= (owner.tile_count)(file) {
                    std::ptr::null_mut()
                } else {
                    (owner.file_tile)(file, usize::from(local))
                };
                bind_standard(specs.industry_tile(), spec, local, cargo, group);
            }
            GSF_AIRPORTS => {
                let owner = &*s.airports;
                let spec = if usize::from(local) >= (owner.airport_count)(file) {
                    std::ptr::null_mut()
                } else {
                    (owner.airport_get)(file, usize::from(local))
                };
                bind_standard(specs.airport(), spec, local, cargo, group);
            }
            GSF_AIRPORTTILES => {
                let owner = &*s.airports;
                let spec = if usize::from(local) >= (owner.tile_count)(file) {
                    std::ptr::null_mut()
                } else {
                    (owner.tile_get)(file, usize::from(local))
                };
                bind_standard(specs.airport_tile(), spec, local, cargo, group);
            }
            GSF_OBJECTS => bind_standard(
                specs.object(),
                (bindings.object_spec)(file, local),
                local,
                cargo,
                group,
            ),
            GSF_STATIONS => {
                if let Some(cargo) = cargo {
                    crate::station_definitions::station_map_specific(
                        s.stations, file, local, cargo, group,
                    );
                } else {
                    crate::station_definitions::station_map_default(s.stations, file, local, group);
                }
            }
            GSF_ROADSTOPS => {
                if let Some(cargo) = cargo {
                    crate::station_definitions::roadstop_map_specific(
                        s.stations, file, local, cargo, group,
                    );
                } else {
                    crate::station_definitions::roadstop_map_default(
                        s.stations, file, local, group,
                    );
                }
            }
            GSF_CANALS => {
                if cargo.is_none() {
                    if local >= CF_END {
                        grf_msg!(
                            1,
                            "CanalMapSpriteGroup: Canal subset {local} out of range, skipping"
                        );
                    } else {
                        (bindings.canal_bind)(local as u8, file, group);
                    }
                }
            }
            GSF_CARGOES => {
                if cargo.is_none() {
                    if local >= u16::from(crate::cargo_type::NUM_CARGO) {
                        grf_msg!(
                            1,
                            "CargoMapSpriteGroup: Cargo type {local} out of range, skipping"
                        );
                    } else {
                        (bindings.cargo_bind)(local as u8, file, group);
                    }
                }
            }
            GSF_RAILTYPES => {
                if let Some(cargo) = cargo {
                    if cargo >= crate::rail::RTSG_END {
                        return;
                    }
                    let map = ((*s.types).rail_map)(file);
                    let railtype = if usize::from(local) < map.size {
                        map.data.add(usize::from(local)).read()
                    } else {
                        crate::rail_type::INVALID_RAILTYPE
                    };
                    if railtype != crate::rail_type::INVALID_RAILTYPE {
                        (bindings.rail_bind)(railtype, cargo, file, group);
                    }
                }
            }
            GSF_ROADTYPES | GSF_TRAMTYPES => {
                if let Some(cargo) = cargo {
                    if cargo >= crate::road::ROTSG_END {
                        return;
                    }
                    let map = if feature == GSF_TRAMTYPES {
                        ((*s.types).tram_map)(file)
                    } else {
                        ((*s.types).road_map)(file)
                    };
                    let roadtype = if usize::from(local) < map.size {
                        map.data.add(usize::from(local)).read()
                    } else {
                        crate::road_type::INVALID_ROADTYPE
                    };
                    if roadtype != crate::road_type::INVALID_ROADTYPE {
                        (bindings.road_bind)(roadtype, cargo, file, group);
                    }
                }
            }
            GSF_BADGES => {
                if cargo.is_some_and(|c| c >= GSF_END) {
                    return;
                }
                if !(bindings.badge_bind)(file, local, cargo.unwrap_or(GSF_DEFAULT), group) {
                    grf_msg!(1, "BadgeMapSpriteGroup: Badge {local} undefined, skipping");
                }
            }
            _ => unreachable!(),
        }
    }
}
unsafe fn map_sprite_group(
    s: &Services,
    buf: &mut ByteReader,
    feature: u8,
    count: u8,
) -> Result<(), ReadBounds> {
    unsafe {
        let mut ids = [0; 256];
        for id in &mut ids[..usize::from(count)] {
            *id = buf.extended()?;
        }
        let ids = &ids[..usize::from(count)];
        let cargos = buf.byte()?;
        for _ in 0..cargos {
            let cargo = buf.byte()?;
            let id = buf.word()?;
            if !valid_group(id, "MapSpriteGroup") {
                continue;
            }
            for &local in ids {
                map_binding(s, feature, local, Some(cargo), group(id));
            }
        }
        let id = buf.word()?;
        if valid_group(id, "MapSpriteGroup") {
            for &local in ids {
                map_binding(s, feature, local, None, group(id));
            }
        }
        Ok(())
    }
}
pub(crate) unsafe fn feature_map_sprite_group(
    s: &Services,
    buf: &mut ByteReader,
) -> Result<(), ReadBounds> {
    unsafe {
        let feature = buf.byte()?;
        let count = buf.byte()?;
        if feature >= GSF_END {
            grf_msg!(
                1,
                "FeatureMapSpriteGroup: Unsupported feature 0x{feature:02X}, skipping"
            );
            return Ok(());
        }
        if count == 0 {
            buf.byte()?;
            let id = buf.word()?;
            if !valid_group(id, "FeatureMapSpriteGroup") {
                return Ok(());
            }
            grf_msg!(
                6,
                "FeatureMapSpriteGroup: Adding generic feature callback for feature 0x{feature:02X}"
            );
            crate::newgrf_generic::openttd_rust_generic_add(feature, loading::file(), group(id));
            return Ok(());
        }
        ((*s.bindings).mark_feature)(loading::file(), feature);
        grf_msg!(
            6,
            "FeatureMapSpriteGroup: Feature 0x{feature:02X}, {count} ids"
        );
        match feature {
            GSF_TRAINS | GSF_ROADVEHICLES | GSF_SHIPS | GSF_AIRCRAFT => {
                vehicle_map(s, buf, feature, count)
            }
            GSF_CANALS | GSF_STATIONS | GSF_HOUSES | GSF_INDUSTRIES | GSF_INDUSTRYTILES
            | GSF_CARGOES | GSF_AIRPORTS | GSF_OBJECTS | GSF_RAILTYPES | GSF_ROADTYPES
            | GSF_TRAMTYPES | GSF_AIRPORTTILES | GSF_ROADSTOPS | GSF_BADGES => {
                map_sprite_group(s, buf, feature, count)
            }
            _ => {
                grf_msg!(
                    1,
                    "FeatureMapSpriteGroup: Unsupported feature 0x{feature:02X}, skipping"
                );
                Ok(())
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_engines_serialized_threads_and_resize() {
        unsafe {
            resize_last(0);
        }
        std::thread::spawn(|| unsafe {
            resize_last(2);
            remember_last(0, EngineID(42));
            remember_last(1, EngineID(99));
        })
        .join()
        .unwrap();
        std::thread::spawn(|| unsafe {
            resize_last(3);
            assert_eq!(
                &*LAST_ENGINES.get(),
                &[EngineID(42), EngineID(99), EngineID(0)]
            );
            resize_last(1);
            resize_last(2);
            assert_eq!(&*LAST_ENGINES.get(), &[EngineID(42), EngineID(0)]);
        })
        .join()
        .unwrap();
        unsafe {
            resize_last(0);
        }
    }
}

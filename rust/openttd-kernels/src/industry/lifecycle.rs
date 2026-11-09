/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry destruction and tile and parent trigger traversal. Station deletion, cargo
//! invalidation and resolver calls can revisit Rust; map and pool storage stay native.
use super::{Builder, Industry, Map, Services, TileArea};
use crate::company_type::Owner;
use crate::industry_type::IndustryID;
use crate::industry_type::{
    IndustryAnimationTrigger, IndustryAnimationTriggers, IndustryRandomTrigger,
    IndustryRandomTriggers,
};
use crate::industrytype::{IndustryBehaviour, IndustryBehaviours};
use crate::tile_type::{TileIndex, TileType};
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct CleanupTile {
    pub industry: u16,
    pub kind: u8,
    pub oilrig: bool,
}
#[repr(C)]
pub struct Reseed {
    pub own: u32,
    pub parent: u32,
    pub used: u8,
}
#[repr(C)]
pub struct RandomisationSpec {
    pub gfx: u16,
    pub enabled: bool,
}
#[repr(C)]
pub struct AnimationSpec {
    pub spec: *const c_void,
    pub triggers: u8,
}
#[repr(C)]
pub struct LifecycleServices {
    pub neutral_station: unsafe extern "C" fn(*mut c_void) -> *mut super::StationHandle,
    pub cleanup_tile: unsafe extern "C" fn(u32) -> CleanupTile,
    pub delete_tile_inspect: unsafe extern "C" fn(u32),
    pub restoration: *const crate::water::RestorationServices,
    pub delete_oilrig: unsafe extern "C" fn(u32),
    pub clear_docking: unsafe extern "C" fn(u32),
    pub industry_field: unsafe extern "C" fn(u32, u16) -> bool,
    pub disassociate_field: unsafe extern "C" fn(u32),
    pub release_disasters: unsafe extern "C" fn(u16),
    pub delete_storage: unsafe extern "C" fn(*mut c_void),
    pub delete_news: unsafe extern "C" fn(u16),
    pub close_view: unsafe extern "C" fn(u16),
    pub close_production: unsafe extern "C" fn(u16),
    pub delete_inspect: unsafe extern "C" fn(u16),
    pub subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub invalidate_cargo: unsafe extern "C" fn(u16),
    pub belongs: unsafe extern "C" fn(u32, u16) -> bool,
    pub randomisation_spec: unsafe extern "C" fn(u32) -> RandomisationSpec,
    pub random_triggers: unsafe extern "C" fn(u32) -> u8,
    pub set_random_triggers: unsafe extern "C" fn(u32, u8),
    pub resolve_reseed: unsafe extern "C" fn(u32, u16, *mut c_void, u8) -> Reseed,
    pub random_bits: unsafe extern "C" fn(u32) -> u8,
    pub set_random_bits: unsafe extern "C" fn(u32, u8),
    pub animation_spec: unsafe extern "C" fn(u32) -> AnimationSpec,
}

/// `Industry::location`, read from the canonical metadata.
pub(super) unsafe fn location(owner: *mut Industry) -> TileArea {
    unsafe {
        TileArea {
            tile: ptr::read(ptr::addr_of!((*owner).metadata.tile)),
            w: ptr::read(ptr::addr_of!((*owner).metadata.width)),
            h: ptr::read(ptr::addr_of!((*owner).metadata.height)),
        }
    }
}
/// `for (TileIndex tile : area)`, through the shared orthogonal tile iterator. The map
/// width is a power of two.
pub(super) fn tiles(area: TileArea, size_x: u32, mut operation: impl FnMut(TileIndex)) {
    for tile in area.tiles(crate::map_func::Map::from_log_x(size_x.ilog2())) {
        operation(TileIndex(tile));
    }
}

/// Original `Industry::~Industry`.
///
/// # Safety
/// Live shell/owner in its native destructor, including partial initialization or
/// pool cleanup. Tables/map are valid for the call. No callback destroys this owner.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_cleanup(
    industry: *mut c_void,
    owner: *mut Industry,
    builder: *mut Builder,
    id: u16,
    cleaning: bool,
    map: Map,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    if cleaning {
        return;
    }
    let id = IndustryID(id);
    let width = unsafe { ptr::read(ptr::addr_of!((*owner).metadata.width)) };
    // Industry can also be destroyed when not fully initialized.
    // This means that we do not have to clear tiles either.
    // Also we must not decrement industry counts in that case.
    if width == 0 {
        return;
    }
    let has_neutral_station = !unsafe { (w.neutral_station)(industry) }.is_null();
    let area = unsafe {
        TileArea {
            tile: ptr::read(ptr::addr_of!((*owner).metadata.tile)),
            w: width,
            h: ptr::read(ptr::addr_of!((*owner).metadata.height)),
        }
    };
    tiles(area, map.size_x, |tile_cur| unsafe {
        let observation = (w.cleanup_tile)(tile_cur.0);
        if TileType(observation.kind) == TileType::MP_INDUSTRY {
            if IndustryID(observation.industry) == id {
                (w.delete_tile_inspect)(tile_cur.0);
                let shared = s.shared();
                // MakeWaterKeepingClass() can also handle 'land'
                crate::water::restore(
                    tile_cur.0,
                    Owner::NONE.0,
                    shared,
                    &*(shared.water_services)(),
                    &*w.restoration,
                );
            }
        } else if TileType(observation.kind) == TileType::MP_STATION && observation.oilrig {
            (w.delete_oilrig)(tile_cur.0);
        }
    });
    if has_neutral_station {
        // Remove possible docking tiles
        tiles(unsafe { location(owner) }, map.size_x, |tile_cur| unsafe {
            (w.clear_docking)(tile_cur.0);
        });
    }
    let r#type = crate::industry_type::IndustryType(unsafe { (*owner).metadata.kind });
    if IndustryBehaviours(unsafe { (*crate::industrytype::get_industry_spec(r#type)).behaviour })
        .test(IndustryBehaviour::PlantFields)
    {
        let mut ta = TileArea {
            tile: unsafe { ptr::read(ptr::addr_of!((*owner).metadata.tile)) },
            w: 0,
            h: 0,
        };
        ta.expand(
            21,
            crate::map_func::Map::from_log_x(map.size_x.ilog2()),
            map.size_y,
        );
        // Remove the farmland and convert it to regular tiles over time.
        tiles(ta, map.size_x, |tile_cur| unsafe {
            if (w.industry_field)(tile_cur.0, id.0) {
                (w.disassociate_field)(tile_cur.0);
            }
        });
    }
    unsafe {
        // don't let any disaster vehicle target invalid industry
        (w.release_disasters)(id.0);
        // Clear the persistent storage.
        (w.delete_storage)(industry);
        let kind = ptr::read(ptr::addr_of!((*owner).metadata.kind));
        super::openttd_rust_industry_registry_erase(builder, kind, id.0);
        (w.delete_news)(id.0);
        (w.close_view)(id.0);
        (w.close_production)(id.0);
        (w.delete_inspect)(id.0);
        (&*(w.subsidies)()).delete_with(crate::subsidy::Source::industry(id.0));
        (w.invalidate_cargo)(id.0);
    }
    let c = crate::station::Ctx::from_shared(s.shared());
    // SAFETY: Live industry owner; the visits mutate only station delivery lists.
    unsafe {
        crate::station::for_each_station(super::stations_near(owner), |near| {
            crate::station::remove_industry_to_deliver((c.c.owner)(near.station), industry.cast());
        });
    }
}

/// Original `DoTriggerIndustryTileRandomisation`: trigger random triggers for an industry
/// tile and reseed its random bits. Returns the bits to reseed for the industry, which the
/// caller collects.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DoTriggerIndustryTileRandomisation keeps the original's implicit C++ integer conversions"
)]
unsafe fn do_trigger_industry_tile_randomisation(
    tile: TileIndex,
    trigger: IndustryRandomTrigger,
    ind: *mut c_void,
    s: &Services,
    w: &LifecycleServices,
) -> u32 {
    let spec = unsafe { (w.randomisation_spec)(tile.0) };
    if !spec.enabled {
        return 0;
    }
    let waiting_random_triggers = IndustryRandomTriggers(unsafe { (w.random_triggers)(tile.0) })
        .with(IndustryRandomTriggers::from_element(trigger));
    unsafe { (w.set_random_triggers)(tile.0, waiting_random_triggers.0) }; // store now for var 5F
    let object = unsafe { (w.resolve_reseed)(tile.0, spec.gfx, ind, waiting_random_triggers.0) };
    // Store remaining triggers.
    let waiting_random_triggers = waiting_random_triggers & !IndustryRandomTriggers(object.used);
    unsafe { (w.set_random_triggers)(tile.0, waiting_random_triggers.0) };
    // Rerandomise tile bits
    let new_random_bits = s.shared().random() as u8;
    let random_bits = unsafe { (w.random_bits)(tile.0) };
    let random_bits = (random_bits & !(object.own as u8)) | (new_random_bits & object.own as u8);
    unsafe {
        (w.set_random_bits)(tile.0, random_bits);
        (s.shared().mark_dirty)(tile.0);
    }
    object.parent
}
/// Original `DoReseedIndustry`: reseeds the random bits of an industry.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DoReseedIndustry keeps the original's implicit C++ integer conversions"
)]
unsafe fn do_reseed_industry(ind: *mut Industry, reseed: u32, s: &Services) {
    if reseed == 0 || ind.is_null() {
        return;
    }
    let random_bits = s.shared().random() as u16;
    unsafe {
        let field = ptr::addr_of_mut!((*ind).metadata.random);
        let old = ptr::read(field);
        // Historical &= reseed (not &= ~reseed) is the specification.
        ptr::write(field, (old & reseed as u16) | (random_bits & reseed as u16));
    }
}
/// Original `TriggerIndustryTileRandomisation`: trigger a random trigger for a single
/// industry tile.
///
/// # Safety
/// Live industry owner for this industry tile and immutable tables; serial call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_randomise_tile(
    tile: u32,
    trigger: u8,
    industry: *mut c_void,
    owner: *mut Industry,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let reseed_industry = unsafe {
        do_trigger_industry_tile_randomisation(
            TileIndex(tile),
            IndustryRandomTrigger(trigger),
            industry,
            s,
            w,
        )
    };
    unsafe { do_reseed_industry(owner, reseed_industry, s) };
}
/// Original `TriggerIndustryRandomisation`: trigger a random trigger for all industry tiles.
///
/// # Safety
/// Live industry and map with valid original trigger; tables outlive this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_randomise(
    industry: *mut c_void,
    owner: *mut Industry,
    id: u16,
    trigger: u8,
    size_x: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let mut reseed_industry = 0;
    tiles(unsafe { location(owner) }, size_x, |tile| unsafe {
        if (w.belongs)(tile.0, id) {
            reseed_industry |= do_trigger_industry_tile_randomisation(
                tile,
                IndustryRandomTrigger(trigger),
                industry,
                s,
                w,
            );
        }
    });
    unsafe { do_reseed_industry(owner, reseed_industry, s) };
}
/// Original `DoTriggerIndustryTileAnimation`.
unsafe fn do_trigger_industry_tile_animation(
    tile: TileIndex,
    iat: IndustryAnimationTrigger,
    random: u32,
    var18_extra: u32,
    w: &LifecycleServices,
) -> bool {
    let itspec = unsafe { (w.animation_spec)(tile.0) };
    if !IndustryAnimationTriggers(itspec.triggers).test(iat) {
        return false;
    }
    unsafe {
        crate::animated_tile::industry_change(
            itspec.spec,
            tile.0,
            random,
            u32::from(iat.0) | var18_extra,
        );
    };
    true
}
/// Original `TriggerIndustryTileAnimation` and
/// `TriggerIndustryTileAnimation_ConstructionStageChanged`, whose caller passes `extra`.
///
/// # Safety
/// Industry tile and valid original trigger; native frame service may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_animate_tile_abi(
    tile: u32,
    trigger: u8,
    extra: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) -> u8 {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let random = s.shared().random();
    u8::from(unsafe {
        do_trigger_industry_tile_animation(
            TileIndex(tile),
            IndustryAnimationTrigger(trigger),
            random,
            extra,
            w,
        )
    })
}
/// Original `TriggerIndustryAnimation`.
///
/// # Safety
/// Live industry and map; immutable tables and valid original trigger.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_animate_abi(
    owner: *mut Industry,
    id: u16,
    trigger: u8,
    size_x: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) -> u8 {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let mut ret = true;
    let mut random = s.shared().random();
    tiles(unsafe { location(owner) }, size_x, |tile| unsafe {
        if (w.belongs)(tile.0, id) {
            if do_trigger_industry_tile_animation(
                tile,
                IndustryAnimationTrigger(trigger),
                random,
                0,
                w,
            ) {
                // SB does not mask its input, so the draw can set high bits too.
                crate::bitmath_func::sb(&mut random, 0, 16, s.shared().random());
            } else {
                ret = false;
            }
        }
    });
    u8::from(ret)
}

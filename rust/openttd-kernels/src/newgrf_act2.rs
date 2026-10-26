/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Whole Action2 construction. Native nodes and final payload vectors stay pooled; only
//! temporary algorithms and the process-wide callback interner are Rust-owned. The game-state
//! mutex serializes calls. Scalar storage aliases are used without creating references across
//! callbacks, and vector element addresses expire at the next mutation of that vector.
//! Checked reads return errors after dropping local vectors; no exception crosses Rust.
//! Native allocation/diagnostic leaves are noexcept; allocation failure and Rust panic abort.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::if_not_else, // Preserve the original branch order.
)]
use crate::game_state::GameState;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS,
};
use crate::newgrf::{
    GSF_AIRCRAFT, GSF_AIRPORTS, GSF_AIRPORTTILES, GSF_BADGES, GSF_CANALS, GSF_CARGOES, GSF_END,
    GSF_HOUSES, GSF_INDUSTRIES, GSF_INDUSTRYTILES, GSF_OBJECTS, GSF_RAILTYPES, GSF_ROADSTOPS,
    GSF_ROADTYPES, GSF_ROADVEHICLES, GSF_SHIPS, GSF_STATIONS, GSF_TRAINS, GSF_TRAMTYPES,
};
use crate::newgrf_bytereader::{ByteReader, Read, ReadError, Reader, VarSize};
use crate::newgrf_commons::{
    TLF_BB_XY_OFFSET, TLF_BB_Z_OFFSET, TLF_CHILD_X_OFFSET, TLF_CHILD_Y_OFFSET, TLF_CUSTOM_PALETTE,
    TLF_DODRAW, TLF_DRAWING_FLAGS, TLF_KNOWN_FLAGS, TLF_NON_GROUND_FLAGS, TLF_NOTHING, TLF_PALETTE,
    TLF_PALETTE_REG_FLAGS, TLF_PALETTE_VAR10, TLF_SPRITE, TLF_SPRITE_REG_FLAGS, TLF_SPRITE_VAR10,
    TLF_VAR10_FLAGS, TLR_MAX_VAR10,
};
use crate::newgrf_loading::{self as loading, Services as LoadingServices};
use crate::newgrf_spritegroup::{
    DSG_SIZE_BYTE, DSG_SIZE_DWORD, DSG_SIZE_WORD, DSGA_OP_ADD, DSGA_TYPE_NONE, GroupResult, Handle,
    RSG_CMP_ALL, RSG_CMP_ANY, Range, VSG_SCOPE_PARENT, VSG_SCOPE_RELATIVE, VSG_SCOPE_SELF,
};
use crate::sprites::{
    PAL_NONE, PALETTE_MODIFIER_COLOUR, PALETTE_MODIFIER_TRANSPARENT, SPR_IMG_QUERY,
    SPRITE_MODIFIER_CUSTOM_SPRITE, SPRITE_MODIFIER_OPAQUE, SPRITE_WIDTH,
};
use std::{ffi::c_void, ptr};
#[repr(C)]
pub struct Layout {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct Sequence {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct PalSprite {
    pub sprite: u32,
    pub pal: u32,
}
#[repr(C)]
pub struct LayoutRegisters {
    pub flags: u8,
    pub dodraw: u8,
    pub sprite: u8,
    pub palette: u8,
    pub max_sprite_offset: u16,
    pub max_palette_offset: u16,
    pub delta: [u8; 3],
    pub sprite_var10: u8,
    pub palette_var10: u8,
}
#[repr(C)]
pub struct AdjustStorage {
    operation: *mut u8,
    kind: *mut u8,
    variable: *mut u8,
    parameter: *mut u8,
    shift_num: *mut u8,
    and_mask: *mut u32,
    add_val: *mut u32,
    divmod_val: *mut u32,
    subroutine: *mut Handle,
}
#[repr(C)]
pub struct RandomStorage {
    scope: *mut u8,
    compare: *mut u8,
    triggers: *mut u8,
    count: *mut u8,
    lowest_bit: *mut u8,
}
#[repr(C)]
pub struct ProductionStorage {
    version: *mut u8,
    num_input: *mut u8,
    subtract_input: *mut i16,
    cargo_input: *mut u8,
    num_output: *mut u8,
    add_output: *mut u16,
    cargo_output: *mut u8,
    again: *mut u8,
}
#[repr(C)]
pub struct LayoutStorage {
    ground: *mut PalSprite,
    consistent_max_offset: *mut u32,
}
#[repr(C)]
pub struct SequenceStorage {
    image: *mut PalSprite,
    x: *mut i8,
    y: *mut i8,
    z: *mut i8,
    extent_x: *mut u8,
    extent_y: *mut u8,
    extent_z: *mut u8,
}
#[repr(C)]
pub struct Services {
    invalid_layout: u32,
    invalid_production: u32,
    callback_new: unsafe extern "C" fn(u16) -> Handle,
    group_index: unsafe extern "C" fn(Handle) -> u32,
    group_get: unsafe extern "C" fn(u32) -> Handle,
    result_new: unsafe extern "C" fn(u32, u8) -> Handle,
    assert_loaded: unsafe extern "C" fn(u32, u32),
    deterministic_new: unsafe extern "C" fn(u32, u8, u8) -> Handle,
    random_new: unsafe extern "C" fn(u32) -> Handle,
    real_new: unsafe extern "C" fn(u32) -> Handle,
    layout_new: unsafe extern "C" fn(u32) -> Handle,
    production_new: unsafe extern "C" fn(u32) -> Handle,
    adjust_begin: unsafe extern "C" fn(Handle) -> AdjustStorage,
    deterministic_default: unsafe extern "C" fn(Handle, GroupResult),
    deterministic_error: unsafe extern "C" fn(Handle, Handle),
    range_append: unsafe extern "C" fn(Handle, Range),
    random_storage: unsafe extern "C" fn(Handle) -> RandomStorage,
    random_reserve: unsafe extern "C" fn(Handle, usize),
    random_append: unsafe extern "C" fn(Handle, Handle),
    loaded_reserve: unsafe extern "C" fn(Handle, usize),
    loaded_append: unsafe extern "C" fn(Handle, Handle),
    loading_reserve: unsafe extern "C" fn(Handle, usize),
    loading_append: unsafe extern "C" fn(Handle, Handle),
    production_storage: unsafe extern "C" fn(Handle) -> ProductionStorage,
    group_layout: unsafe extern "C" fn(Handle) -> *mut Layout,
    layout_allocate: unsafe extern "C" fn(*mut Layout, u32),
    layout_storage: unsafe extern "C" fn(*mut Layout) -> LayoutStorage,
    layout_sequence: unsafe extern "C" fn(*mut Layout, u32) -> *mut Sequence,
    sequence_storage: unsafe extern "C" fn(*mut Sequence) -> SequenceStorage,
    is_parent: unsafe extern "C" fn(*const Sequence) -> bool,
    has_registers: unsafe extern "C" fn(*const Layout) -> bool,
    allocate_registers: unsafe extern "C" fn(*mut Layout),
    registers: unsafe extern "C" fn(*mut Layout, u32) -> *mut LayoutRegisters,
    assert_offsets: unsafe extern "C" fn(bool, bool, u32),
    error_data: unsafe extern "C" fn(*mut c_void, *const u8, usize),
    missing_group: unsafe extern "C" fn(u8, u8, u16),
    missing_set: unsafe extern "C" fn(u8, u8, u16),
    unsupported_feature: unsafe extern "C" fn(u8),
    reserved_type: unsafe extern "C" fn(u8),
    random_count: unsafe extern "C" fn(),
    no_sets: unsafe extern "C" fn(),
    real_counts: unsafe extern "C" fn(u8, u8, u8),
    real_empty: unsafe extern "C" fn(),
    real_one: unsafe extern "C" fn(u16),
    real_same: unsafe extern "C" fn(u16),
    loaded_entry: unsafe extern "C" fn(u32, u16),
    loading_entry: unsafe extern "C" fn(u32, u16),
    production_version: unsafe extern "C" fn(u8),
    sprite_set: unsafe extern "C" fn(u32),
    palette_set: unsafe extern "C" fn(u32),
    sprite_var10: unsafe extern "C" fn(),
    palette_var10: unsafe extern "C" fn(),
    register_var10: unsafe extern "C" fn(u8, u8),
    ground_flags: unsafe extern "C" fn(u8),
    unknown_flags: unsafe extern "C" fn(u8),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAdjustStorage" ["act2"] => AdjustStorage { operation, kind, variable, parameter, shift_num, and_mask, add_val, divmod_val, subroutine };
    "OpenTTDRandomStorage" ["act2"] => RandomStorage { scope, compare, triggers, count, lowest_bit };
    "OpenTTDProductionStorage" ["act2"] => ProductionStorage { version, num_input, subtract_input, cargo_input, num_output, add_output, cargo_output, again };
    "OpenTTDLayoutStorage" ["act2"] => LayoutStorage { ground, consistent_max_offset };
    "OpenTTDSequenceStorage" ["act2"] => SequenceStorage { image, x, y, z, extent_x, extent_y, extent_z };
    "OpenTTDPalSprite" ["act2"] => PalSprite { sprite, pal };
    "OpenTTDLayoutRegisters" ["act2"] => LayoutRegisters { flags, dodraw, sprite, palette, max_sprite_offset, max_palette_offset, delta, sprite_var10, palette_var10 };
    "OpenTTDAction2Services" ["act2"] => Services { invalid_layout, invalid_production, callback_new, group_index, group_get, result_new, assert_loaded, deterministic_new, random_new, real_new, layout_new, production_new, adjust_begin, deterministic_default, deterministic_error, range_append, random_storage, random_reserve, random_append, loaded_reserve, loaded_append, loading_reserve, loading_append, production_storage, group_layout, layout_allocate, layout_storage, layout_sequence, sequence_storage, is_parent, has_registers, allocate_registers, registers, assert_offsets, error_data, missing_group, missing_set, unsupported_feature, reserved_type, random_count, no_sets, real_counts, real_empty, real_one, real_same, loaded_entry, loading_entry, production_version, sprite_set, palette_set, sprite_var10, palette_var10, register_var10, ground_flags, unknown_flags };
}

const GROUPID_CALLBACK_FAILED: u16 = 0x7FFF;
const GROUPID_CALCULATED_RESULT: u16 = 0x7FFE;
static CALLBACKS: GameState<Vec<(u16, u32)>> = GameState::new(Vec::new());

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_action2_reset(final_reset: bool) {
    let cache = unsafe { &mut *CALLBACKS.get() };
    cache.clear();
    if final_reset {
        cache.shrink_to_fit();
    }
}
unsafe fn callback(s: &Services, l: &LoadingServices, mut value: u16) -> Handle {
    unsafe {
        let file = (*loading::openttd_rust_grf_execution()).grffile;
        if ((*l.global).file_version)(file) < 8 && value >> 8 == 0xFF {
            value &= !0xFF00;
        } else {
            value &= !0x8000;
        }
        let position = (*CALLBACKS.get()).partition_point(|entry| entry.0 < value);
        let cached = (&*CALLBACKS.get()).get(position).copied();
        if let Some((key, index)) = cached
            && key == value
        {
            return (s.group_get)(index);
        }
        let group = (s.callback_new)(value);
        let index = (s.group_index)(group);
        (*CALLBACKS.get()).insert(position, (value, index));
        group
    }
}
unsafe fn group_ref(s: &Services, l: &LoadingServices, setid: u8, kind: u8, id: u16) -> Handle {
    unsafe {
        if id & 0x8000 != 0 {
            return callback(s, l, id);
        }
        if id == GROUPID_CALLBACK_FAILED {
            return ptr::null();
        }
        let group = if u32::from(id) <= crate::newgrf_internal::MAX_SPRITEGROUP {
            (*loading::openttd_rust_grf_execution()).spritegroups[usize::from(id)]
        } else {
            ptr::null()
        };
        if group.is_null() {
            (s.missing_group)(setid, kind, id);
        }
        group
    }
}
unsafe fn create_group(
    s: &Services,
    l: &LoadingServices,
    feature: u8,
    setid: u8,
    kind: u8,
    id: u16,
) -> Handle {
    unsafe {
        if id & 0x8000 != 0 {
            return callback(s, l, id);
        }
        if !loading::openttd_rust_grf_has_set(feature, u32::from(id)) {
            (s.missing_set)(setid, kind, id);
            return ptr::null();
        }
        let start = loading::openttd_rust_grf_set_sprite(feature, u32::from(id));
        let count = loading::openttd_rust_grf_set_size(feature, u32::from(id));
        (s.assert_loaded)(
            start.wrapping_add(count),
            (*loading::openttd_rust_grf_execution()).spriteid,
        );
        (s.result_new)(start, count as u8)
    }
}
fn result(group: Handle) -> GroupResult {
    GroupResult {
        group,
        calculated_result: false,
    }
}
unsafe fn read_result(
    s: &Services,
    l: &LoadingServices,
    setid: u8,
    kind: u8,
    id: u16,
) -> GroupResult {
    if id == GROUPID_CALCULATED_RESULT {
        GroupResult {
            group: ptr::null(),
            calculated_result: true,
        }
    } else {
        result(unsafe { group_ref(s, l, setid, kind, id) })
    }
}
unsafe fn deterministic<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    setid: u8,
    kind: u8,
    line: u32,
) -> Result<Handle, ReadError> {
    unsafe {
        let scope = if kind & 2 != 0 {
            VSG_SCOPE_PARENT
        } else {
            VSG_SCOPE_SELF
        };
        // The six dispatch cases prove that the native size switch has no default here.
        let (size, varsize) = match (kind >> 2) & 3 {
            0 => (DSG_SIZE_BYTE, VarSize::Byte),
            1 => (DSG_SIZE_WORD, VarSize::Word),
            _ => (DSG_SIZE_DWORD, VarSize::Dword),
        };
        let group = (s.deterministic_new)(line, scope, size);
        let mut first = true;
        loop {
            let a = (s.adjust_begin)(group);
            *a.operation = if first { DSGA_OP_ADD } else { b.byte()? };
            first = false;
            let variable = b.byte()?;
            *a.variable = variable;
            if variable == 0x7E {
                *a.subroutine = group_ref(s, l, setid, kind, u16::from(b.byte()?));
            } else {
                *a.parameter = if (0x60..0x80).contains(&variable) {
                    b.byte()?
                } else {
                    0
                };
            }
            let varadjust = b.byte()?;
            *a.shift_num = varadjust & 0x1F;
            let adjust_type = varadjust >> 6;
            *a.kind = adjust_type;
            *a.and_mask = b.var_size(varsize)?;
            if adjust_type != DSGA_TYPE_NONE {
                *a.add_val = b.var_size(varsize)?;
                let divisor = b.var_size(varsize)?;
                *a.divmod_val = if divisor == 0 { 1 } else { divisor };
            } else {
                *a.add_val = 0;
                *a.divmod_val = 0;
            }
            if varadjust & 0x20 == 0 {
                break;
            }
        }
        let count = b.byte()?;
        let mut ranges = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let r = read_result(s, l, setid, kind, b.word()?);
            let low = b.var_size(varsize)?;
            let high = b.var_size(varsize)?;
            ranges.push(Range {
                result: r,
                low,
                high,
            });
        }
        let mut default = read_result(s, l, setid, kind, b.word()?);
        (s.deterministic_default)(group, default);
        (s.deterministic_error)(
            group,
            ranges.first().map_or(default.group, |r| r.result.group),
        );
        if ranges.is_empty() {
            default = GroupResult {
                group: ptr::null(),
                calculated_result: true,
            };
            (s.deterministic_default)(group, default);
        }
        let mut bounds = Vec::with_capacity(ranges.len());
        for r in &ranges {
            bounds.push(r.low);
            if r.high != u32::MAX {
                bounds.push(r.high + 1);
            }
        }
        bounds.sort_unstable();
        bounds.dedup();
        let mut target = Vec::with_capacity(bounds.len());
        for &bound in &bounds {
            target.push(
                ranges
                    .iter()
                    .find(|r| r.low <= bound && bound <= r.high)
                    .map_or(default, |r| r.result),
            );
        }
        let mut j = 0;
        while j < bounds.len() {
            if target[j] != default {
                let value = target[j];
                let low = bounds[j];
                while j < bounds.len() && target[j] == value {
                    j += 1;
                }
                let high = if j < bounds.len() {
                    bounds[j] - 1
                } else {
                    u32::MAX
                };
                (s.range_append)(
                    group,
                    Range {
                        result: value,
                        low,
                        high,
                    },
                );
            } else {
                j += 1;
            }
        }
        Ok(group)
    }
}
unsafe fn randomized<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    feature: u8,
    setid: u8,
    kind: u8,
    line: u32,
) -> Result<Handle, ReadError> {
    unsafe {
        let group = (s.random_new)(line);
        let fields = (s.random_storage)(group);
        *fields.scope = if kind & 2 != 0 {
            VSG_SCOPE_PARENT
        } else {
            VSG_SCOPE_SELF
        };
        if kind & 4 != 0 {
            if feature <= GSF_AIRCRAFT {
                *fields.scope = VSG_SCOPE_RELATIVE;
            }
            *fields.count = b.byte()?;
        }
        let triggers = b.byte()?;
        *fields.triggers = triggers & 0x7F;
        *fields.compare = if triggers & 0x80 != 0 {
            RSG_CMP_ALL
        } else {
            RSG_CMP_ANY
        };
        *fields.lowest_bit = b.byte()?;
        let count = b.byte()?;
        if !count.is_power_of_two() {
            (s.random_count)();
        }
        (s.random_reserve)(group, usize::from(count));
        for _ in 0..count {
            let value = group_ref(s, l, setid, kind, b.word()?);
            (s.random_append)(group, value);
        }
        Ok(group)
    }
}
unsafe fn real<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    feature: u8,
    setid: u8,
    kind: u8,
    line: u32,
) -> Result<Option<Handle>, ReadError> {
    unsafe {
        let num_loading = b.byte()?;
        if !loading::openttd_rust_grf_has_sets(feature) {
            (s.no_sets)();
            return Ok(None);
        }
        (s.real_counts)(setid, kind, num_loading);
        match u16::from(kind) + u16::from(num_loading) {
            0 => {
                (s.real_empty)();
                return Ok(Some(ptr::null()));
            }
            1 => {
                let id = b.word()?;
                let group = create_group(s, l, feature, setid, kind, id);
                (s.real_one)(id);
                return Ok(Some(group));
            }
            _ => (),
        }
        let mut loaded = Vec::with_capacity(usize::from(kind));
        for i in 0..kind {
            let id = b.word()?;
            loaded.push(id);
            (s.loaded_entry)(u32::from(i), id);
        }
        let mut loading = Vec::with_capacity(usize::from(num_loading));
        for i in 0..num_loading {
            let id = b.word()?;
            loading.push(id);
            (s.loading_entry)(u32::from(i), id);
        }
        let loaded_same = !loaded.is_empty() && loaded.windows(2).all(|p| p[0] == p[1]);
        let loading_same = !loading.is_empty() && loading.windows(2).all(|p| p[0] == p[1]);
        if loaded_same && loading_same && loaded[0] == loading[0] {
            let group = create_group(s, l, feature, setid, kind, loaded[0]);
            (s.real_same)(loaded[0]);
            return Ok(Some(group));
        }
        let group = (s.real_new)(line);
        if loaded_same && loaded.len() > 1 {
            loaded.truncate(1);
        }
        (s.loaded_reserve)(group, loaded.len());
        for id in loaded {
            let value = create_group(s, l, feature, setid, kind, id);
            (s.loaded_append)(group, value);
        }
        if loading_same && loading.len() > 1 {
            loading.truncate(1);
        }
        (s.loading_reserve)(group, loading.len());
        for id in loading {
            let value = create_group(s, l, feature, setid, kind, id);
            (s.loading_append)(group, value);
        }
        Ok(Some(group))
    }
}
unsafe fn disable(s: &Services, l: &LoadingServices, message: u32, data: &[u8]) {
    unsafe {
        let error = loading::openttd_rust_grf_disable(l, message, ptr::null_mut());
        if !data.is_empty() {
            (s.error_data)(error, data.as_ptr(), data.len());
        }
    }
}
unsafe fn production<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    kind: u8,
    line: u32,
) -> Result<Option<Handle>, ReadError> {
    unsafe {
        if kind > 2 {
            (s.production_version)(kind);
            return Ok(Some(ptr::null()));
        }
        let group = (s.production_new)(line);
        let f = (s.production_storage)(group);
        *f.version = kind;
        if kind < 2 {
            *f.num_input = INDUSTRY_ORIGINAL_NUM_INPUTS;
            for i in 0..usize::from(INDUSTRY_ORIGINAL_NUM_INPUTS) {
                *f.subtract_input.add(i) = if kind == 0 {
                    b.word()? as i16
                } else {
                    i16::from(b.byte()?)
                };
            }
            *f.num_output = INDUSTRY_ORIGINAL_NUM_OUTPUTS;
            for i in 0..usize::from(INDUSTRY_ORIGINAL_NUM_OUTPUTS) {
                *f.add_output.add(i) = if kind == 0 {
                    b.word()?
                } else {
                    u16::from(b.byte()?)
                };
            }
        } else {
            let count = b.byte()?;
            *f.num_input = count;
            if usize::from(count) > INDUSTRY_NUM_INPUTS {
                disable(s, l, s.invalid_production, b"too many inputs (max 16)");
                return Ok(None);
            }
            for i in 0..usize::from(count) {
                let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
                    b.byte()?,
                    (*loading::openttd_rust_grf_execution()).grffile,
                    false,
                );
                if !crate::cargo_type::is_valid_cargo_type(cargo) {
                    *f.version = 0xFF;
                } else if (0..i).any(|j| *f.cargo_input.add(j) == cargo) {
                    disable(s, l, s.invalid_production, b"duplicate input cargo");
                    return Ok(None);
                }
                *f.cargo_input.add(i) = cargo;
                *f.subtract_input.add(i) = i16::from(b.byte()?);
            }
            let count = b.byte()?;
            *f.num_output = count;
            if usize::from(count) > INDUSTRY_NUM_OUTPUTS {
                disable(s, l, s.invalid_production, b"too many outputs (max 16)");
                return Ok(None);
            }
            for i in 0..usize::from(count) {
                let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
                    b.byte()?,
                    (*loading::openttd_rust_grf_execution()).grffile,
                    false,
                );
                if !crate::cargo_type::is_valid_cargo_type(cargo) {
                    *f.version = 0xFF;
                } else if (0..i).any(|j| *f.cargo_output.add(j) == cargo) {
                    disable(s, l, s.invalid_production, b"duplicate output cargo");
                    return Ok(None);
                }
                *f.cargo_output.add(i) = cargo;
                *f.add_output.add(i) = u16::from(b.byte()?);
            }
        }
        *f.again = b.byte()?;
        Ok(Some(group))
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_action2_recolour(image: *mut PalSprite) {
    unsafe {
        if (*image).pal & (1 << 14) != 0 {
            (*image).pal &= !(1 << 14);
            (*image).sprite |= 1 << SPRITE_MODIFIER_OPAQUE;
        }
        if (*image).sprite & (1 << 14) != 0 {
            (*image).sprite &= !(1 << 14);
            (*image).sprite |= 1 << PALETTE_MODIFIER_TRANSPARENT;
        }
        if (*image).sprite & (1 << 15) != 0 {
            (*image).sprite &= !(1 << 15);
            (*image).sprite |= 1 << PALETTE_MODIFIER_COLOUR;
        }
    }
}
pub(crate) unsafe fn layout_sprite<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    read_flags: bool,
    invert: bool,
    use_sets: bool,
    feature: u8,
    image: *mut PalSprite,
    max_sprite: *mut u16,
    max_palette: *mut u16,
) -> Result<u8, ReadError> {
    unsafe {
        (*image).sprite = u32::from(b.word()?);
        (*image).pal = u32::from(b.word()?);
        let flags = if read_flags {
            b.word()? as u8
        } else {
            TLF_NOTHING
        };
        openttd_rust_action2_recolour(image);
        let custom = (((*image).pal & (1 << 15)) != 0) != invert;
        (*image).pal &= !(1 << 15);
        if custom {
            let index = (*image).sprite & 0x3FFF;
            if use_sets
                && (!loading::openttd_rust_grf_has_set(feature, index)
                    || loading::openttd_rust_grf_set_size(feature, index) == 0)
            {
                (s.sprite_set)(index);
                (*image).sprite = SPR_IMG_QUERY;
                (*image).pal = PAL_NONE;
            } else {
                let sprite = if use_sets {
                    loading::openttd_rust_grf_set_sprite(feature, index)
                } else {
                    index
                };
                if !max_sprite.is_null() {
                    *max_sprite = if use_sets {
                        loading::openttd_rust_grf_set_size(feature, index) as u16
                    } else {
                        u16::MAX
                    };
                }
                crate::bitmath_func::sb(&mut (*image).sprite, 0, SPRITE_WIDTH, sprite);
                (*image).sprite |= 1 << SPRITE_MODIFIER_CUSTOM_SPRITE;
            }
        } else if flags & TLF_SPRITE_VAR10 != 0 && flags & TLF_SPRITE_REG_FLAGS == 0 {
            (s.sprite_var10)();
            disable(s, l, s.invalid_layout, b"");
            return Ok(flags);
        }
        if flags & TLF_CUSTOM_PALETTE != 0 {
            let index = (*image).pal & 0x3FFF;
            if use_sets
                && (!loading::openttd_rust_grf_has_set(feature, index)
                    || loading::openttd_rust_grf_set_size(feature, index) == 0)
            {
                (s.palette_set)(index);
                (*image).pal = PAL_NONE;
            } else {
                let sprite = if use_sets {
                    loading::openttd_rust_grf_set_sprite(feature, index)
                } else {
                    index
                };
                if !max_palette.is_null() {
                    *max_palette = if use_sets {
                        loading::openttd_rust_grf_set_size(feature, index) as u16
                    } else {
                        u16::MAX
                    };
                }
                crate::bitmath_func::sb(&mut (*image).pal, 0, SPRITE_WIDTH, sprite);
                (*image).pal |= 1 << SPRITE_MODIFIER_CUSTOM_SPRITE;
            }
        } else if flags & TLF_PALETTE_VAR10 != 0 && flags & TLF_PALETTE_REG_FLAGS == 0 {
            (s.palette_var10)();
            disable(s, l, s.invalid_layout, b"");
        }
        Ok(flags)
    }
}
unsafe fn layout_registers<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    flags: u8,
    parent: bool,
    layout: *mut Layout,
    index: u32,
) -> Result<(), ReadError> {
    unsafe {
        if flags & TLF_DRAWING_FLAGS == 0 {
            return Ok(());
        }
        if !(s.has_registers)(layout) {
            (s.allocate_registers)(layout);
        }
        let regs = (s.registers)(layout, index);
        (*regs).flags = flags & TLF_DRAWING_FLAGS;
        if flags & TLF_DODRAW != 0 {
            (*regs).dodraw = b.byte()?;
        }
        if flags & TLF_SPRITE != 0 {
            (*regs).sprite = b.byte()?;
        }
        if flags & TLF_PALETTE != 0 {
            (*regs).palette = b.byte()?;
        }
        if parent {
            if flags & TLF_BB_XY_OFFSET != 0 {
                (*regs).delta[0] = b.byte()?;
                (*regs).delta[1] = b.byte()?;
            }
            if flags & TLF_BB_Z_OFFSET != 0 {
                (*regs).delta[2] = b.byte()?;
            }
        } else {
            if flags & TLF_CHILD_X_OFFSET != 0 {
                (*regs).delta[0] = b.byte()?;
            }
            if flags & TLF_CHILD_Y_OFFSET != 0 {
                (*regs).delta[1] = b.byte()?;
            }
        }
        if flags & TLF_SPRITE_VAR10 != 0 {
            let value = b.byte()?;
            (*regs).sprite_var10 = value;
            if value > TLR_MAX_VAR10 {
                (s.register_var10)(value, TLR_MAX_VAR10);
                disable(s, l, s.invalid_layout, b"");
                return Ok(());
            }
        }
        if flags & TLF_PALETTE_VAR10 != 0 {
            let value = b.byte()?;
            (*regs).palette_var10 = value;
            if value > TLR_MAX_VAR10 {
                (s.register_var10)(value, TLR_MAX_VAR10);
                disable(s, l, s.invalid_layout, b"");
            }
        }
        Ok(())
    }
}
unsafe fn disabled() -> bool {
    unsafe { (*loading::openttd_rust_grf_execution()).skip_sprites < 0 }
}
pub(crate) unsafe fn read_layout<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
    mut count: u32,
    use_sets: bool,
    feature: u8,
    allow_var10: bool,
    no_z: bool,
    layout: *mut Layout,
) -> Result<bool, ReadError> {
    unsafe {
        let has_flags = count & (1 << 6) != 0;
        count &= !(1 << 6);
        let valid = if allow_var10 {
            TLF_KNOWN_FLAGS
        } else {
            TLF_KNOWN_FLAGS & !TLF_VAR10_FLAGS
        };
        (s.layout_allocate)(layout, count);
        let fields = (s.layout_storage)(layout);
        let mut max_sprite = vec![0u16; count as usize + 1];
        let mut max_palette = vec![0u16; count as usize + 1];
        let flags = layout_sprite(
            s,
            l,
            b,
            has_flags,
            false,
            use_sets,
            feature,
            fields.ground,
            max_sprite.as_mut_ptr(),
            max_palette.as_mut_ptr(),
        )?;
        if disabled() {
            return Ok(true);
        }
        let invalid = flags & !(valid & !TLF_NON_GROUND_FLAGS);
        if invalid != 0 {
            (s.ground_flags)(invalid);
            disable(s, l, s.invalid_layout, b"");
            return Ok(true);
        }
        layout_registers(s, l, b, flags, false, layout, 0)?;
        if disabled() {
            return Ok(true);
        }
        for i in 0..count {
            let seq = (s.layout_sequence)(layout, i);
            let f = (s.sequence_storage)(seq);
            let flags = layout_sprite(
                s,
                l,
                b,
                has_flags,
                false,
                use_sets,
                feature,
                f.image,
                max_sprite.as_mut_ptr().add(i as usize + 1),
                max_palette.as_mut_ptr().add(i as usize + 1),
            )?;
            if disabled() {
                return Ok(true);
            }
            if flags & !valid != 0 {
                (s.unknown_flags)(flags & !valid);
                disable(s, l, s.invalid_layout, b"");
                return Ok(true);
            }
            *f.x = b.byte()? as i8;
            *f.y = b.byte()? as i8;
            if !no_z {
                *f.z = b.byte()? as i8;
            }
            if (s.is_parent)(seq) {
                *f.extent_x = b.byte()?;
                *f.extent_y = b.byte()?;
                *f.extent_z = b.byte()?;
            }
            layout_registers(s, l, b, flags, (s.is_parent)(seq), layout, i + 1)?;
            if disabled() {
                return Ok(true);
            }
        }
        let mut consistent = true;
        *fields.consistent_max_offset = 0;
        for i in 0..=count as usize {
            if max_sprite[i] > 0 {
                if *fields.consistent_max_offset == 0 {
                    *fields.consistent_max_offset = u32::from(max_sprite[i]);
                } else if *fields.consistent_max_offset != u32::from(max_sprite[i]) {
                    consistent = false;
                    break;
                }
            }
            if max_palette[i] > 0 {
                if *fields.consistent_max_offset == 0 {
                    *fields.consistent_max_offset = u32::from(max_palette[i]);
                } else if *fields.consistent_max_offset != u32::from(max_palette[i]) {
                    consistent = false;
                    break;
                }
            }
        }
        (s.assert_offsets)(use_sets, consistent, *fields.consistent_max_offset);
        if !consistent || (s.has_registers)(layout) {
            *fields.consistent_max_offset = 0;
            if !(s.has_registers)(layout) {
                (s.allocate_registers)(layout);
            }
            for i in 0..=count {
                let regs = (s.registers)(layout, i);
                (*regs).max_sprite_offset = max_sprite[i as usize];
                (*regs).max_palette_offset = max_palette[i as usize];
            }
        }
        Ok(false)
    }
}
pub(crate) unsafe fn parse<R: Read>(
    s: &Services,
    l: &LoadingServices,
    b: &mut R,
) -> Result<(), ReadError> {
    unsafe {
        let feature = b.byte()?;
        if feature >= GSF_END {
            (s.unsupported_feature)(feature);
            return Ok(());
        }
        let setid = b.byte()?;
        let kind = b.byte()?;
        let line = (*loading::openttd_rust_grf_execution()).nfo_line;
        let group = match kind {
            0x81 | 0x82 | 0x85 | 0x86 | 0x89 | 0x8A => deterministic(s, l, b, setid, kind, line)?,
            0x80 | 0x83 | 0x84 => randomized(s, l, b, feature, setid, kind, line)?,
            _ => {
                if kind >= 0x80 {
                    (s.reserved_type)(kind);
                    return Ok(());
                }
                match feature {
                    GSF_TRAINS | GSF_ROADVEHICLES | GSF_SHIPS | GSF_AIRCRAFT | GSF_STATIONS
                    | GSF_CANALS | GSF_CARGOES | GSF_AIRPORTS | GSF_RAILTYPES | GSF_ROADTYPES
                    | GSF_TRAMTYPES | GSF_BADGES => {
                        let Some(group) = real(s, l, b, feature, setid, kind, line)? else {
                            return Ok(());
                        };
                        group
                    }
                    GSF_HOUSES | GSF_AIRPORTTILES | GSF_OBJECTS | GSF_INDUSTRYTILES
                    | GSF_ROADSTOPS => {
                        let group = (s.layout_new)(line);
                        let layout = (s.group_layout)(group);
                        if read_layout(
                            s,
                            l,
                            b,
                            u32::from(kind.max(1)),
                            true,
                            feature,
                            false,
                            kind == 0,
                            layout,
                        )? {
                            return Ok(());
                        }
                        group
                    }
                    GSF_INDUSTRIES => {
                        let Some(group) = production(s, l, b, kind, line)? else {
                            return Ok(());
                        };
                        group
                    }
                    _ => {
                        (s.unsupported_feature)(feature);
                        ptr::null()
                    }
                }
            }
        };
        (*loading::openttd_rust_grf_execution()).spritegroups[usize::from(setid)] = group;
        Ok(())
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_action2_parse(
    s: &Services,
    l: &LoadingServices,
    reads: &crate::newgrf_bytereader::Services,
    native: *mut ByteReader,
) -> bool {
    unsafe { parse(s, l, &mut Reader::new(native, reads)).is_ok() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_action2_layout(
    s: &Services,
    l: &LoadingServices,
    reads: &crate::newgrf_bytereader::Services,
    native: *mut ByteReader,
    count: u32,
    use_sets: bool,
    feature: u8,
    allow_var10: bool,
    no_z: bool,
    layout: *mut Layout,
) -> i32 {
    unsafe {
        match read_layout(
            s,
            l,
            &mut Reader::new(native, reads),
            count,
            use_sets,
            feature,
            allow_var10,
            no_z,
            layout,
        ) {
            Ok(error) => i32::from(error),
            Err(_) => -1,
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_action2_layout_sprite(
    s: &Services,
    l: &LoadingServices,
    reads: &crate::newgrf_bytereader::Services,
    native: *mut ByteReader,
    read_flags: bool,
    invert: bool,
    use_sets: bool,
    feature: u8,
    image: *mut PalSprite,
    max_sprite: *mut u16,
    max_palette: *mut u16,
    flags: *mut u8,
) -> bool {
    unsafe {
        match layout_sprite(
            s,
            l,
            &mut Reader::new(native, reads),
            read_flags,
            invert,
            use_sets,
            feature,
            image,
            max_sprite,
            max_palette,
        ) {
            Ok(value) => {
                *flags = value;
                true
            }
            Err(_) => false,
        }
    }
}

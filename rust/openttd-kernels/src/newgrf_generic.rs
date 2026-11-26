/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_generic.cpp`: handling of generic feature callbacks, with `newgrf_generic.h`'s
//! `AmbientSoundEffect`.
//!
//! The registry `_gcl` is written only by `NewGRF` loading (`ResetGenericCallbacks` and Action
//! 3's `AddGenericCallback`), which runs outside every callback evaluation, so its files and
//! sprite groups stay live while one is resolved. No borrow of the registry is held across a
//! callback. The native resolver frame (`callback_frame`) reads the `GenericResolverObject`
//! on the evaluating Rust stack through a pointer, for that synchronous call only.
use crate::bitmath_func::gb;
use crate::cargo_type::CargoType;
use crate::cargotype::cargo_spec_get;
use crate::debug::{debug, grf_msg};
use crate::game_state::GameState;
use crate::industry_type::IndustryType;
use crate::industrytype::get_industry_spec;
use crate::map_func::World;
use crate::math::clamp;
use crate::newgrf::{GRFFile, GRFFileRef, GrfSpecFeature};
use crate::newgrf_callbacks::{CALLBACK_FAILED, CallbackID};
use crate::newgrf_spritegroup::VarSpriteGroupScope;
use crate::random_func::chance16r;
use crate::services::Services as Shared;
use crate::services_assert::native_check;
use crate::tile_type::{TileIndex, TileType};
use std::ffi::c_void;
use std::ptr;

/// The AI has no specific industry in mind.
pub const IT_AI_UNKNOWN: IndustryType = IndustryType(0xFE);
/// The AI actually wants to transport to/from a town, not an industry.
pub const IT_AI_TOWN: IndustryType = IndustryType(0xFF);

/// `AIConstructionEvent`: AI events for asking the `NewGRF` for information.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AIConstructionEvent(pub u8);

/// `GenericScopeResolver`: scope resolver for generic objects and properties.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GenericScopeResolver {
    cargo_type: CargoType,
    default_selection: u8,
    /// Source industry substitute type. 0xFF for "town", 0xFE for "unknown".
    src_industry: IndustryType,
    /// Destination industry substitute type. 0xFF for "town", 0xFE for "unknown".
    dst_industry: IndustryType,
    distance: u8,
    event: AIConstructionEvent,
    count: u8,
    station_size: u8,

    feature: GrfSpecFeature,

    /// Callback comes from the AI.
    ai_callback: bool,
}

impl GenericScopeResolver {
    /// Generic scope resolver; `ai_callback`: callback comes from the AI.
    const fn new(ai_callback: bool) -> Self {
        Self {
            cargo_type: CargoType(0),
            default_selection: 0,
            src_industry: IndustryType(0),
            dst_industry: IndustryType(0),
            distance: 0,
            event: AIConstructionEvent(0),
            count: 0,
            station_size: 0,
            feature: GrfSpecFeature::GSF_INVALID,
            ai_callback,
        }
    }

    /// `GenericScopeResolver::GetVariable`, for the scope of `ro`.
    #[allow(unsafe_code, reason = "the AI callback's live answering file")]
    fn get_variable(&self, ro: &GenericResolverObject, variable: u8, available: &mut bool) -> u32 {
        if self.ai_callback {
            match variable {
                0x40 => {
                    // SAFETY: An AI callback's resolver has the answering file of its
                    // attempt, live for the evaluation (the module's contract).
                    let grffile = unsafe { GRFFileRef::of(services().shared(), ro.grffile) };
                    return u32::from(grffile.cargo_map(self.cargo_type));
                }

                0x80 => return u32::from(self.cargo_type.0),
                0x81 => {
                    return u32::from(
                        cargo_spec_get(crate::cargo_registry::services(), self.cargo_type).bitnum(),
                    );
                }
                0x82 => return u32::from(self.default_selection),
                0x83 => return u32::from(self.src_industry.0),
                0x84 => return u32::from(self.dst_industry.0),
                0x85 => return u32::from(self.distance),
                0x86 => return u32::from(self.event.0),
                0x87 => return u32::from(self.count),
                0x88 => return u32::from(self.station_size),

                _ => (),
            }
        }

        debug!(
            grf,
            1, "Unhandled generic feature variable 0x{:02X}", variable
        );

        *available = false;
        u32::MAX
    }
}

/// `GenericResolverObject`: resolver object for generic objects/properties, with the members
/// of `ResolverObject` its native frame (`callback_frame`) copies.
#[repr(C)]
pub struct GenericResolverObject {
    generic_scope: GenericScopeResolver,
    callback: CallbackID,
    callback_param1: u32,
    grffile: *const GRFFile,
    root_spritegroup: *const c_void,
}

impl GenericResolverObject {
    /// Generic resolver; `ai_callback`: callback comes from the AI, `callback`: callback ID.
    const fn new(ai_callback: bool, callback: CallbackID) -> Self {
        Self {
            generic_scope: GenericScopeResolver::new(ai_callback),
            callback,
            callback_param1: 0,
            grffile: ptr::null(),
            root_spritegroup: ptr::null(),
        }
    }

    /// `GenericResolverObject::GetScope`: the generic scope for `VSG_SCOPE_SELF`, else
    /// `ResolverObject::GetScope`'s default scope.
    pub(crate) fn get_scope(
        generic_scope: *mut c_void,
        default_scope: *mut c_void,
        scope: VarSpriteGroupScope,
    ) -> *mut c_void {
        match scope {
            VarSpriteGroupScope::VSG_SCOPE_SELF => generic_scope,
            _ => default_scope,
        }
    }
}

/// The answer of a generic callback for C++: the answering file, or null, and the result.
#[repr(C)]
pub struct Result {
    file: *const GRFFile,
    result: u16,
}

/// The generic callback table.
#[repr(C)]
pub struct Services {
    /// Builds the native `GenericResolverObject` frame for the object and calls
    /// `openttd_rust_generic_evaluate` with it (#1064 replaces it with the resolver frames).
    callback_frame: extern "C" fn(*const GenericResolverObject, *mut i32, usize) -> u16,
    /// The common sprite-group table.
    common: *const crate::newgrf_spritegroup::Services,
    /// The global-variable table.
    global: *const crate::newgrf::Services,
    /// `PlayTileSound(const GRFFile *, SoundID, TileIndex)` (`newgrf_sound.cpp`).
    play_tile_sound: extern "C" fn(*const GRFFile, u16, u32),
    /// The shared table.
    shared: *const Shared,
}
impl Services {
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    fn shared(&self) -> &Shared {
        // SAFETY: Both tables are process-lifetime.
        unsafe { &*self.shared }
    }
    /// The global-variable table.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    pub(crate) fn global(&self) -> &crate::newgrf::Services {
        // SAFETY: Both tables are process-lifetime.
        unsafe { &*self.global }
    }
}

/// `GenericCallback`.
#[derive(Clone, Copy)]
struct GenericCallback {
    file: *const GRFFile,
    group: *const c_void,
}

/// `_gcl`, one list for each feature below `GSF_END`.
static GCL: GameState<[Vec<GenericCallback>; GrfSpecFeature::GSF_END.0 as usize]> =
    GameState::new([const { Vec::new() }; GrfSpecFeature::GSF_END.0 as usize]);
/// The generic callback table, installed by `reset_generic_callbacks`.
static SERVICES: GameState<*const Services> = GameState::new(ptr::null());

/// The generic callback table.
#[allow(unsafe_code, reason = "the installed process-lifetime table")]
pub(crate) fn services() -> &'static Services {
    // SAFETY: `reset_generic_callbacks` installs the process-lifetime table before loading
    // registers any callback, and nothing evaluates one before that.
    unsafe { &**SERVICES.get() }
}

/// `ResetGenericCallbacks`: reset all generic feature callback sprite groups. The registry
/// also takes the table its evaluations use.
///
/// # Safety
/// `native` is the process-lifetime table, and no evaluation is running (the module's
/// contract).
#[allow(unsafe_code, reason = "the loader's exclusive registry writes")]
pub(crate) unsafe fn reset_generic_callbacks(native: *const Services) {
    // SAFETY: Loading is outside every evaluation, so nothing borrows the registry.
    unsafe {
        SERVICES.get().write(native);
        for gcl in &mut *GCL.get() {
            gcl.clear();
        }
    }
}

/// `ResetGenericCallbacks` for the native landscape fixture.
///
/// # Safety
/// As [`reset_generic_callbacks`].
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_reset(native: *const Services) {
    unsafe { reset_generic_callbacks(native) };
}

/// `AddGenericCallback`: add a generic feature callback sprite group to the appropriate feature
/// list.
#[allow(unsafe_code, reason = "the loader's exclusive registry write")]
pub(crate) fn add_generic_callback(
    feature: GrfSpecFeature,
    file: *const GRFFile,
    group: *const c_void,
) {
    if feature >= GrfSpecFeature::GSF_END {
        grf_msg!(
            5,
            "AddGenericCallback: Unsupported feature 0x{:02X}",
            feature.0
        );
        return;
    }

    // Generic feature callbacks are evaluated in reverse (i.e. the last group to be added is
    // evaluated first, etc), so the group is pushed to the end of the list and the list is
    // evaluated from the back.
    // SAFETY: Loading is outside every evaluation, so nothing borrows the registry.
    unsafe {
        (&mut *GCL.get())[usize::from(feature.0)].push(GenericCallback { file, group });
    }
}

/// `AddGenericCallback` for the native landscape fixture's facade.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_add(
    feature: u8,
    file: *const GRFFile,
    group: *const c_void,
) {
    add_generic_callback(GrfSpecFeature(feature), file, group);
}

/// `GetGenericCallbackResult`: follow a generic feature callback list and return the first
/// successful answer: the answering file and callback value, or `None` and `CALLBACK_FAILED`.
/// `object` is the pre-populated resolver object, `param1_grfv7` and `param1_grfv8` the
/// `callback_param1` for GRFs up to version 7 and from version 8 on, and `regs100` receives
/// the additional result values from registers 100+.
#[allow(unsafe_code, reason = "registry reads between the loader's writes")]
fn get_generic_callback_result(
    feature: GrfSpecFeature,
    object: &mut GenericResolverObject,
    param1_grfv7: u32,
    param1_grfv8: u32,
    regs100: &mut [i32],
) -> (Option<GRFFileRef<'static>>, u16) {
    crate::services_assert::assert_with(|| feature < GrfSpecFeature::GSF_END);
    let s = services();

    // Test each feature callback sprite group.
    // SAFETY: Only loading writes the registry; each entry is copied out, so no borrow of the
    // list is held across the callback.
    let count = unsafe { (&*GCL.get())[usize::from(feature.0)].len() };
    for index in (0..count).rev() {
        // SAFETY: As above.
        let it = unsafe { (&*GCL.get())[usize::from(feature.0)][index] };
        // SAFETY: Registered files are live until the loader's next reset.
        let file = unsafe { GRFFileRef::of(s.shared(), it.file) };
        object.grffile = it.file;
        object.root_spritegroup = it.group;
        // Set callback param based on GRF version.
        object.callback_param1 = if file.grf_version() >= 8 {
            param1_grfv8
        } else {
            param1_grfv7
        };
        let result = (s.callback_frame)(
            ptr::from_mut(object).cast_const(),
            regs100.as_mut_ptr(),
            regs100.len(),
        );
        if result == CALLBACK_FAILED {
            continue;
        }

        return (Some(file), result);
    }

    // No callback returned a valid result, so we've failed.
    (None, CALLBACK_FAILED)
}

/// `GenericScopeResolver::GetVariable` for the native scope's override.
///
/// # Safety
/// `object` is the evaluating resolver object and `available` is writable.
#[allow(unsafe_code, reason = "export over the evaluating stack object")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_variable(
    object: *const GenericResolverObject,
    variable: u8,
    available: *mut bool,
) -> u32 {
    // SAFETY: The caller's contract.
    unsafe { generic_scope_variable(object, variable, &mut *available) }
}

/// `GenericScopeResolver::GetVariable` for the common resolver's dispatch.
///
/// # Safety
/// As [`openttd_rust_generic_variable`].
#[allow(unsafe_code, reason = "the evaluating stack object")]
pub(crate) unsafe fn generic_scope_variable(
    object: *const GenericResolverObject,
    variable: u8,
    available: &mut bool,
) -> u32 {
    // SAFETY: The caller's contract.
    let ro = unsafe { &*object };
    ro.generic_scope.get_variable(ro, variable, available)
}

/// `GenericResolverObject::GetScope` for the native resolver's override.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_scope(
    generic_scope: *mut c_void,
    default_scope: *mut c_void,
    scope: u8,
) -> *mut c_void {
    GenericResolverObject::get_scope(generic_scope, default_scope, VarSpriteGroupScope(scope))
}

/// `ResolverObject::ResolveCallback` of the native frame `callback_frame` built for `object`.
///
/// # Safety
/// `object` is the evaluating resolver object, `resolver` and `state` its live native frame,
/// and `regs` holds `count` writable registers.
#[allow(unsafe_code, reason = "export over the native frame")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_evaluate(
    object: *const GenericResolverObject,
    resolver: *mut c_void,
    state: *mut crate::newgrf_spritegroup::State,
    regs: *mut i32,
    count: usize,
) -> u16 {
    // SAFETY: The caller's contract; only the two handles are copied before evaluation.
    unsafe {
        let (root_spritegroup, grffile) = ((*object).root_spritegroup, (*object).grffile);
        crate::newgrf_spritegroup::resolve_callback(
            &*services().common,
            root_spritegroup,
            resolver,
            state,
            grffile.cast(),
            regs,
            count,
        )
    }
}

/// The arguments of `GetAiPurchaseCallbackResult`, in its order.
pub(crate) struct AiPurchaseCallback {
    /// GRF Feature to call callback for.
    pub(crate) feature: GrfSpecFeature,
    /// Cargotype to pass to callback. (Variable 80)
    pub(crate) cargo_type: CargoType,
    /// 'Default selection' to pass to callback. (Variable 82)
    pub(crate) default_selection: u8,
    /// 'Source industry type' to pass to callback. (Variable 83)
    pub(crate) src_industry: IndustryType,
    /// 'Destination industry type' to pass to callback. (Variable 84)
    pub(crate) dst_industry: IndustryType,
    /// 'Distance between source and destination' to pass to callback. (Variable 85)
    pub(crate) distance: u8,
    /// 'AI construction event' to pass to callback. (Variable 86)
    pub(crate) event: AIConstructionEvent,
    /// 'Construction number' to pass to callback. (Variable 87)
    pub(crate) count: u8,
    /// 'Station size' to pass to callback. (Variable 88)
    pub(crate) station_size: u8,
}

/// `GetAiPurchaseCallbackResult`: 'execute' an AI purchase selection callback. Returns the
/// answering file and callback value if successful, or `CALLBACK_FAILED`.
///
/// # Safety
/// The returned handle is a call-time value: the caller uses it only before the loader's next
/// reset, which destroys the file (`GRFFileRef`'s contract).
#[allow(unsafe_code, reason = "the live industry specs")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the uint16_t subst_id stored to an IndustryType (newgrf_generic.cpp)"
)]
pub(crate) unsafe fn get_ai_purchase_callback_result(
    args: &AiPurchaseCallback,
) -> (Option<GRFFileRef<'static>>, u16) {
    let mut object =
        GenericResolverObject::new(true, CallbackID::CBID_GENERIC_AI_PURCHASE_SELECTION);

    let mut src_industry = args.src_industry;
    let mut dst_industry = args.dst_industry;

    if src_industry != IT_AI_UNKNOWN && src_industry != IT_AI_TOWN {
        let is = get_industry_spec(src_industry);
        // If this is no original industry, use the substitute type
        // SAFETY: `GetIndustrySpec` returns a live spec of the table.
        let subst_id = unsafe { (*is).grf_prop.subst_id };
        if subst_id != u16::from(IndustryType::IT_INVALID.0) {
            src_industry = IndustryType(subst_id as u8);
        }
    }

    if dst_industry != IT_AI_UNKNOWN && dst_industry != IT_AI_TOWN {
        let is = get_industry_spec(dst_industry);
        // If this is no original industry, use the substitute type
        // SAFETY: `GetIndustrySpec` returns a live spec of the table.
        let subst_id = unsafe { (*is).grf_prop.subst_id };
        if subst_id != u16::from(IndustryType::IT_INVALID.0) {
            dst_industry = IndustryType(subst_id as u8);
        }
    }

    object.generic_scope.cargo_type = args.cargo_type;
    object.generic_scope.default_selection = args.default_selection;
    object.generic_scope.src_industry = src_industry;
    object.generic_scope.dst_industry = dst_industry;
    object.generic_scope.distance = args.distance;
    object.generic_scope.event = args.event;
    object.generic_scope.count = args.count;
    object.generic_scope.station_size = args.station_size;
    object.generic_scope.feature = args.feature;

    let mut callback = get_generic_callback_result(args.feature, &mut object, 0, 0, &mut []);
    if callback.1 != CALLBACK_FAILED {
        let file = callback.0.unwrap();
        if file.grf_version() < 8 {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "GB(callback.second, 0, 8) stored to the uint16_t result (newgrf_generic.cpp)"
            )]
            {
                callback.1 = gb(u32::from(callback.1), 0, 8) as u16;
            }
        }
    }
    callback
}

/// `GetAiPurchaseCallbackResult` for C++.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_ai(
    feature: u8,
    cargo_type: u8,
    default_selection: u8,
    src_industry: u8,
    dst_industry: u8,
    distance: u8,
    event: u8,
    count: u8,
    station_size: u8,
) -> Result {
    // SAFETY: Only the answering file's pointer leaves this call.
    let (file, result) = unsafe {
        get_ai_purchase_callback_result(&AiPurchaseCallback {
            feature: GrfSpecFeature(feature),
            cargo_type: CargoType(cargo_type),
            default_selection,
            src_industry: IndustryType(src_industry),
            dst_industry: IndustryType(dst_industry),
            distance,
            event: AIConstructionEvent(event),
            count,
            station_size,
        })
    };
    Result {
        file: file.map_or(ptr::null(), GRFFileRef::as_ptr),
        result,
    }
}

/// `AmbientSoundEffectCallback`: 'execute' the ambient sound effect callback for the tile the
/// sound effect should be generated for.
#[inline(never)]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "Clamp's int overload over TileHeight's uint, and GetTileZ's int, shifted into the uint32_t param1_v7 and param1_v8 (newgrf_generic.cpp)"
)]
fn ambient_sound_effect_callback(tile: TileIndex) {
    let s = services();
    let shared = s.shared();
    let map = World::new(shared);
    crate::services_assert::assert_with(|| {
        map.is_tile_type(tile, TileType::MP_CLEAR)
            || map.is_tile_type(tile, TileType::MP_TREES)
            || map.is_tile_type(tile, TileType::MP_WATER)
    });

    // Only run every 1/200-th time.
    let mut r = 0; // Save for later
    if !chance16r(shared, 1, 200, &mut r) || !shared.settings().ambient() {
        return;
    }

    // Prepare resolver object.
    let mut object = GenericResolverObject::new(false, CallbackID::CBID_SOUNDS_AMBIENT_EFFECT);
    object.generic_scope.feature = GrfSpecFeature::GSF_SOUNDFX;

    let tile_type = || u32::from(map.asserted(|m| crate::tile_map::get_tile_type(m, tile)).0);
    let param1_v7 = tile_type() << 28
        | (clamp(
            crate::tile_map::tile_height_in(&map, tile) as i32,
            0,
            15,
            native_check,
        ) as u32)
            << 24
        | gb(r, 16, 8) << 16
        | (shared.terrain_type)(tile.0);
    let param1_v8 = tile_type() << 24
        | (map.tile_z(tile) as u32) << 16
        | gb(r, 16, 8) << 8
        | (if map.asserted(|m| crate::water_map::has_tile_water_class(m, tile.0)) {
            u32::from(map.water_class(tile.0))
        } else {
            0
        }) << 3
        | (shared.terrain_type)(tile.0);

    // Run callback.
    let callback = get_generic_callback_result(
        GrfSpecFeature::GSF_SOUNDFX,
        &mut object,
        param1_v7,
        param1_v8,
        &mut [],
    );

    if let (Some(file), result) = callback {
        (s.play_tile_sound)(file.as_ptr(), result, tile.0);
    }
}

/// `AmbientSoundEffect`: play an ambient sound effect for an empty tile. `enabled` is the
/// caller's observation of `HasGrfMiscBit(GrfMiscBit::AmbientSoundCallback)`, which the tile
/// loops read once for a batch.
#[inline(always)]
#[allow(clippy::inline_always, reason = "the tile loops' per-tile gate")]
pub(crate) fn ambient_sound_effect(enabled: bool, tile: TileIndex) {
    // Only run callback if enabled.
    if !enabled {
        return;
    }

    ambient_sound_effect_callback(tile);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGenericScope" ["probe"] => GenericScopeResolver { cargo_type, default_selection, src_industry, dst_industry, distance, event, count, station_size, feature, ai_callback };
    "OpenTTDGenericInvocation" ["probe"] => GenericResolverObject { generic_scope, callback, callback_param1, grffile, root_spritegroup };
    "OpenTTDGenericResult" ["probe"] => Result { file, result };
    "OpenTTDGenericServices" ["probe"] => Services { callback_frame, common, global, play_tile_sound, shared };
}

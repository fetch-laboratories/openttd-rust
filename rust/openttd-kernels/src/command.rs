/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Command invocation lifecycle: `Command<Tcmd>::Do`, `Post`, `Unsafe` and `Execute`
//! control, process-wide recursion, admission, company restoration and money
//! reconciliation. Native frames keep typed argument and result tuples; every Rust
//! access ends before a native step can reenter a command.
#![allow(
    unsafe_code,
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::nonminimal_bool,
    clippy::too_many_lines
)]
use crate::command_type::{
    CMD_COMPANY_CTRL, COMMAND_TYPE_PAUSE_LEVEL, COMMAND_TYPE_SERVER_SETTING, CommandFlag,
    DoCommandFlag, command_flags_to_dc_flags,
};
use crate::company_type::{COMPANY_SPECTATOR, OWNER_DEITY};
use crate::game_state::GameState;
use crate::openttd::{GameMode, PauseMode};
use crate::services::{CommandCost, CommandCostServices, CommandServices, Cost};

/// `INVALID_STRING_ID`, the message of `CMD_ERROR`.
const INVALID_STRING_ID: u32 = 0xFFFF;

pub(crate) fn before(_s: &CommandServices, top_level: bool, test: bool) {
    if top_level {
        crate::objects::clear_areas();
    }
    if test {
        crate::town::openttd_rust_town_rating_test(true);
    }
}

pub(crate) fn available_money(s: &CommandServices) -> i64 {
    if (s.infinite_money)() {
        return i64::MAX;
    }
    let id = (s.finance.current_company)();
    let owner = (s.finance.company_finances)(id);
    // SAFETY: the canonical resolver returns null or the live finance owner.
    unsafe { crate::company::openttd_rust_company_available_money(owner, false) }
}

/// # Safety
/// `result` is a live native result, exclusively mutated for this call.
pub(crate) unsafe fn check_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
) -> bool {
    let amount = (c.money)(result);
    if amount <= 0 || (s.infinite_money)() {
        return true;
    }
    let owner = (s.finance.company_finances)((s.finance.current_company)());
    // SAFETY: the canonical resolver returns null or a live finance owner.
    if unsafe { crate::company::has_money(owner, false, amount) } {
        return true;
    }
    // SAFETY: the caller supplies exclusive live native result storage.
    unsafe {
        (s.money_error)(result, amount);
    }
    false
}

/// # Safety
/// `result` is live caller-owned native command storage for the whole call.
pub(crate) unsafe fn after(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
    flags: u16,
    top_level: bool,
    test: bool,
) {
    if test {
        crate::town::openttd_rust_town_rating_test(false);
        if (c.success)(result)
            && top_level
            && flags & (DoCommandFlag::QUERY_COST | DoCommandFlag::BANKRUPT) == 0
        {
            // SAFETY: caller guarantees live exclusive storage.
            unsafe {
                check_money(s, c, result);
            }
        }
    } else if (c.success)(result) && top_level && flags & DoCommandFlag::BANKRUPT == 0 {
        // SAFETY: caller guarantees live native result storage.
        unsafe {
            subtract_money(s, c, result);
        }
    }
}

/// # Safety
/// `result` is a live native result for this call.
pub(crate) unsafe fn subtract_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *const CommandCost,
) {
    let id = (s.finance.current_company)();
    let owner = (s.finance.company_finances)(id);
    if owner.is_null() {
        return;
    }
    let amount = (c.money)(result);
    if amount == 0 {
        return;
    }
    let expense = (c.expense)(result);
    // SAFETY: resolver returned the live owner; no borrow crosses invalidation.
    unsafe {
        crate::company::subtract_money(s.finance, owner, id, amount, expense.0);
    }
}

/// # Safety
/// `result` is a live native result for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_subtract_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *const CommandCost,
) {
    unsafe {
        subtract_money(s, c, result);
    }
}

/// # Safety
/// `result` is a live exclusive native command result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_check_money(
    s: &CommandServices,
    c: &CommandCostServices,
    result: *mut CommandCost,
) -> bool {
    unsafe { check_money(s, c, result) }
}

/// `RecursiveCommandCounter::_counter`, the process-wide command nesting depth.
static COUNTER: GameState<i32> = GameState::new(0);

/// `RecursiveCommandCounter` construction; reports whether this entry is top-level.
fn enter() -> bool {
    // SAFETY: serialized game-state access; no reference outlives the statement.
    unsafe {
        *COUNTER.get() = (*COUNTER.get()).wrapping_add(1);
        *COUNTER.get() == 1
    }
}

/// One `RecursiveCommandCounter` scope, left on every exit including abandonment.
struct Nesting(bool);
impl Nesting {
    fn new() -> Self {
        Self(enter())
    }
}
impl Drop for Nesting {
    fn drop(&mut self) {
        // SAFETY: serialized game-state access.
        unsafe {
            *COUNTER.get() = (*COUNTER.get()).wrapping_sub(1);
        }
    }
}

/// Ordinary `CommandHelper::Do` lifecycle around a Rust-owned command body.
/// Native callbacks operate on caller-owned stack results and serialized state.
pub(crate) fn nested<T: Default>(
    s: &CommandServices,
    shared: &crate::services::Services,
    out: crate::services::Cost<'_>,
    tile: u32,
    flags: u16,
    mut body: impl FnMut(u16) -> T,
) -> T {
    do_command(s, shared, out, tile, flags, |f| Some(body(f))).unwrap_or_default()
}

/// `CommandHelper::Do`. A body returns `None` when a native exception abandons the
/// call; the remaining steps are skipped and only the nesting scope unwinds.
fn do_command<T: Default>(
    s: &CommandServices,
    shared: &crate::services::Services,
    out: crate::services::Cost<'_>,
    tile: u32,
    flags: u16,
    mut body: impl FnMut(u16) -> Option<T>,
) -> Option<T> {
    let size = ((shared.map_max_y)() + 1) << shared.map().log_x;
    if tile != 0
        && (tile >= size || (!(shared.tile_valid)(tile) && flags & DoCommandFlag::ALL_TILES == 0))
    {
        out.error(INVALID_STRING_ID);
        return Some(T::default());
    }
    let counter = Nesting::new();
    let top = counter.0;
    if top || flags & DoCommandFlag::EXECUTE == 0 {
        before(s, top, true);
        let extra = body(flags & !DoCommandFlag::EXECUTE)?;
        // SAFETY: the command caller supplies live exclusive native result storage.
        unsafe {
            after(s, out.1, out.0, flags, top, true);
        }
        if !out.ok() || flags & DoCommandFlag::EXECUTE == 0 {
            return Some(extra);
        }
    }
    before(s, top, false);
    let extra = body(flags)?;
    // SAFETY: the command caller supplies live exclusive native result storage.
    unsafe {
        after(s, out.1, out.0, flags, top, false);
    }
    Some(extra)
}

/// Native frame of one typed `Command<Tcmd>` invocation. `results` address the
/// `CommandCost` of the test, execution and untouched default result tuples. Each
/// step runs only the typed native part: the trait procedure on the argument tuple,
/// its desync log line, network enqueueing, client identifiers and the callback.
/// A procedure step returns `false` after capturing a native exception, which the
/// caller rethrows once Rust has returned. `Do` frames set only the first result and
/// `run`, and frames without a callback leave `callback` null, so Rust reads single
/// fields and never forms a reference to a whole frame.
#[repr(C)]
pub struct Frame {
    pub results: [*mut CommandCost; 3],
    pub run: unsafe extern "C" fn(*mut Frame, u16, u8) -> bool,
    pub additional_money: unsafe extern "C" fn(*mut Frame) -> i64,
    pub log: unsafe extern "C" fn(*mut Frame, bool),
    pub send: unsafe extern "C" fn(*mut Frame, u8),
    pub set_client_ids: unsafe extern "C" fn(*mut Frame),
    pub assert_client_ids: unsafe extern "C" fn(*mut Frame),
    pub callback: unsafe extern "C" fn(*mut Frame, u8),
}

/// Result slot of the test run.
const TEST: u8 = 0;
/// Result slot of the execution run.
const EXEC: u8 = 1;
/// Untouched default result, `{}` or `MakeResult(CMD_ERROR)`.
const EMPTY: u8 = 2;
/// A captured native exception abandons the invocation.
const ABANDONED: u8 = u8::MAX;
/// `PSM_ENTER_COMMAND` to `PSM_LEAVE_TESTMODE`.
const PSM_ENTER_COMMAND: u8 = 2;
const PSM_LEAVE_COMMAND: u8 = 3;
const PSM_ENTER_TESTMODE: u8 = 4;
const PSM_LEAVE_TESTMODE: u8 = 5;
/// `TILE_SIZE`.
const TILE_SIZE: u32 = 16;

fn storage_mode(s: &CommandServices, mode: u8) {
    // SAFETY: serialized persistent storage owner; the leaves never reenter it.
    unsafe {
        crate::newgrf_storage::openttd_rust_storage_switch_mode(
            mode,
            false,
            s.signals.assert_condition,
            s.shared.storage_discard,
        );
    }
}

fn flush_signals(s: &CommandServices) {
    // SAFETY: immutable process tables; the caller holds the game-state mutex.
    unsafe {
        crate::signal::openttd_rust_signal_flush(
            s.shared,
            s.rail_map,
            s.signals,
            u32::from(s.shared.map().log_x),
        );
    }
}

fn is_local_company(s: &CommandServices) -> bool {
    // SAFETY: the company table is immutable for the process lifetime.
    let company = unsafe { &*(s.company)() };
    (company.local_company)() == (s.finance.current_company)()
}

/// `IsCommandAllowedWhilePaused`, over the command's `CommandType`.
pub(crate) fn allowed_while_paused(s: &CommandServices, command_type: u8) -> bool {
    crate::session::mode() == GameMode::EDITOR
        || COMMAND_TYPE_PAUSE_LEVEL[usize::from(command_type)] <= (s.pause_level)()
}

/// `IsCommandAllowedWhilePaused` facade for native network queue admission.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_allowed_while_paused(
    s: &CommandServices,
    command_type: u8,
) -> bool {
    allowed_while_paused(s, command_type)
}

/// One `CommandHelper::Execute` call: its process-wide recursion scope, company
/// backup and restoration, storage modes, money checks and result reconciliation.
/// Returns the slot holding the native result, or `ABANDONED`.
/// # Safety
/// `frame` is a live native frame whose result slots stay valid for the call.
unsafe fn execute(
    s: &CommandServices,
    frame: *mut Frame,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    estimate_only: bool,
    network_command: bool,
    tile: u32,
) -> u8 {
    let c = s.cost;
    // SAFETY: the caller supplies a live frame; its steps never move it.
    let (results, run, additional_money, log, send, assert_client_ids) = unsafe {
        (
            (*frame).results,
            (*frame).run,
            (*frame).additional_money,
            (*frame).log,
            (*frame).send,
            (*frame).assert_client_ids,
        )
    };
    let counter = Nesting::new();
    (s.signals.assert_condition)(counter.0);
    if cmd_flags & CommandFlag::CLIENT_ID != 0 {
        // SAFETY: live frame for the call.
        unsafe { assert_client_ids(frame) };
    }
    let finance = s.finance;
    let original = (finance.current_company)();

    let exec_as_spectator = cmd_flags & (CommandFlag::SPECTATOR | CommandFlag::SERVER) != 0;
    let current = (finance.current_company)();
    if crate::session::mode() == GameMode::NORMAL
        && !exec_as_spectator
        && !(s.shared.company_valid)(current)
        && !(current == OWNER_DEITY && cmd_flags & CommandFlag::DEITY != 0)
    {
        Cost(results[usize::from(EMPTY)], c).error(INVALID_STRING_ID);
        return EMPTY;
    }
    if exec_as_spectator {
        (finance.set_current_company)(COMPANY_SPECTATOR);
    }
    before(s, true, true);
    storage_mode(s, PSM_ENTER_TESTMODE);

    let flags = command_flags_to_dc_flags(cmd_flags);
    // SAFETY: live frame; no Rust borrow spans the native procedure.
    if !unsafe { run(frame, flags, TEST) } {
        return abandon(s, original);
    }

    storage_mode(s, PSM_LEAVE_TESTMODE);
    crate::town::openttd_rust_town_rating_test(false);
    (s.signals.assert_condition)(if exec_as_spectator {
        (finance.current_company)() == COMPANY_SPECTATOR
    } else {
        (finance.current_company)() == original
    });
    let test = results[usize::from(TEST)];
    let test_and_exec_can_differ = cmd_flags & CommandFlag::NO_TEST != 0;
    // SAFETY: live exclusive native results in the frame.
    if !(c.success)(test)
        || estimate_only
        || (!test_and_exec_can_differ && !unsafe { check_money(s, c, test) })
    {
        let generating = (s.shared.clear_square_services.generating_world)();
        if !(finance.networking)() || generating || network_command {
            // SAFETY: live frame.
            unsafe { log(frame, true) };
        }
        (finance.set_current_company)(original);
        return TEST;
    }
    let send_net = (finance.networking)()
        && !(s.shared.clear_square_services.generating_world)()
        && !network_command;
    if !send_net {
        before(s, true, false);
        storage_mode(s, PSM_ENTER_COMMAND);
    }
    if send_net {
        // SAFETY: live frame.
        unsafe { send(frame, (finance.current_company)()) };
        (finance.set_current_company)(original);
        return EMPTY;
    }
    // The log step applies `Debug(desync, 1)`'s level filter, the original condition here.
    // SAFETY: live frame.
    unsafe { log(frame, false) };
    // SAFETY: live frame; no Rust borrow spans the native procedure.
    if !unsafe { run(frame, flags | DoCommandFlag::EXECUTE, EXEC) } {
        return abandon(s, original);
    }
    // SAFETY: live frame with a constructed execution result.
    let extra_cash = unsafe { additional_money(frame) };
    let exec = results[usize::from(EXEC)];

    storage_mode(s, PSM_LEAVE_COMMAND);
    if cmd == CMD_COMPANY_CTRL {
        // A new company switches to it, a closed one to spectator; the caller restores others.
        // SAFETY: the company table is immutable for the process lifetime.
        (finance.set_current_company)((unsafe { &*(s.company)() }.local_company)());
    } else {
        (s.signals.assert_condition)(if exec_as_spectator {
            (finance.current_company)() == COMPANY_SPECTATOR
        } else {
            (finance.current_company)() == original
        });
        (finance.set_current_company)(original);
    }

    if !test_and_exec_can_differ {
        (s.signals.assert_condition)(
            (c.money)(test) == (c.money)(exec) && (c.success)(test) == (c.success)(exec),
        );
    } else if !(c.success)(exec) {
        return EXEC;
    }
    if extra_cash != 0 && (c.money)(exec) == 0 {
        // Removed rail can refund money and leave queued signals.
        flush_signals(s);
        (c.error_param)(exec, s.not_enough_cash, extra_cash as u64);
        return EXEC;
    }
    if tile != 0 {
        (s.last_build_coordinate)((finance.current_company)(), tile);
    }
    // SAFETY: live native execution result.
    unsafe { subtract_money(s, c, exec) };
    let pause = crate::session::pause();
    if pause != 0 && command_type != COMMAND_TYPE_SERVER_SETTING {
        crate::session::set_pause(pause | PauseMode::COMMAND_DURING_PAUSE.bit());
    }
    flush_signals(s);
    EXEC
}

/// `Backup<CompanyID>` destruction during unwinding: restore without a prior `Restore`.
fn abandon(s: &CommandServices, original: u8) -> u8 {
    (s.finance.set_current_company)(original);
    ABANDONED
}

/// `CommandHelper::InternalPost` after its typed tuple preparation. Returns 0 or 1
/// for the result's success, or `ABANDONED`.
/// # Safety
/// `frame` is a live native frame whose result slots stay valid for the call.
unsafe fn post(
    s: &CommandServices,
    frame: *mut Frame,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    err_message: u32,
    my_cmd: bool,
    network_command: bool,
    tile: u32,
    has_callback: bool,
) -> u8 {
    let shared = s.shared;
    let size = ((shared.map_max_y)() + 1) << shared.map().log_x;
    if tile != 0
        && (tile >= size || (!(shared.tile_valid)(tile) && cmd_flags & CommandFlag::ALL_TILES == 0))
    {
        return 0;
    }
    let map = shared.map();
    let x = map.tile_x(tile).wrapping_mul(TILE_SIZE) as i32;
    let y = map.tile_y(tile).wrapping_mul(TILE_SIZE) as i32;

    /* InternalPostBefore: estimates come from a local shift press, never from network,
     * world generation or the pause button. */
    let estimate_only = (s.shift_pressed)()
        && is_local_company(s)
        && !(shared.clear_square_services.generating_world)()
        && !network_command
        && cmd_flags & CommandFlag::NO_EST == 0;
    let only_sending = (s.finance.networking)() && !network_command;
    if crate::session::pause() != 0 && !allowed_while_paused(s, command_type) && !estimate_only {
        (s.paused_error)(err_message, x, y);
        return 0;
    }

    // SAFETY: live frame; no Rust borrow spans the native steps.
    unsafe {
        if !network_command && cmd_flags & CommandFlag::CLIENT_ID != 0 {
            ((*frame).set_client_ids)(frame);
        }
        let slot = execute(
            s,
            frame,
            cmd,
            command_type,
            cmd_flags,
            estimate_only,
            network_command,
            tile,
        );
        if slot == ABANDONED {
            return ABANDONED;
        }
        let c = s.cost;
        let res = (*frame).results[usize::from(slot)];

        /* InternalPostResult. */
        if !(c.success)(res) {
            if estimate_only || (is_local_company(s) && err_message != 0 && my_cmd) {
                (s.error_message)(err_message, x, y, res);
            }
        } else if estimate_only {
            (s.estimate)((c.money)(res), x, y);
        } else if !only_sending
            && tile != 0
            && is_local_company(s)
            && crate::session::mode() != GameMode::EDITOR
        {
            (s.cost_animation)(x, y, (c.money)(res));
        }

        if !estimate_only && !only_sending && has_callback {
            ((*frame).callback)(frame, slot);
        }
        u8::from((c.success)(res))
    }
}

/// `CommandHelper::Do` for a native caller of `Command<Tcmd>::Do`. Returns `false`
/// when a native exception abandoned the call.
/// # Safety
/// `frame` is a live native frame whose first result slot stays valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_do(
    s: &CommandServices,
    frame: *mut Frame,
    tile: u32,
    flags: u16,
) -> bool {
    // SAFETY: live frame for the whole call.
    let (out, run) = unsafe { ((*frame).results[usize::from(TEST)], (*frame).run) };
    do_command(s, s.shared, Cost(out, s.cost), tile, flags, |f| {
        // SAFETY: live frame; no Rust borrow spans the native procedure.
        unsafe { run(frame, f, TEST) }.then_some(())
    })
    .is_some()
}

/// `CommandHelper::Execute` for `Command<Tcmd>::Unsafe`; returns its result slot.
/// # Safety
/// `frame` is a live native frame whose result slots stay valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_execute(
    s: &CommandServices,
    frame: *mut Frame,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    estimate_only: bool,
    tile: u32,
) -> u8 {
    unsafe {
        execute(
            s,
            frame,
            cmd,
            command_type,
            cmd_flags,
            estimate_only,
            false,
            tile,
        )
    }
}

/// `CommandHelper::InternalPost` for `Post` and `PostFromNet`.
/// # Safety
/// `frame` is a live native frame whose result slots stay valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_post(
    s: &CommandServices,
    frame: *mut Frame,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    err_message: u32,
    my_cmd: bool,
    network_command: bool,
    tile: u32,
    has_callback: bool,
) -> u8 {
    unsafe {
        post(
            s,
            frame,
            cmd,
            command_type,
            cmd_flags,
            err_message,
            my_cmd,
            network_command,
            tile,
            has_callback,
        )
    }
}

/// Own one independently constructed native result for a synchronous nested call.
/// The storage never moves while its native object is alive. No native layout is
/// inspected, and construction/destruction are the only lifetime operations.
/// # Safety
/// The callback must not retain or return either the cost or its pointer, rebuild
/// a Cost from that pointer outside the callback, destroy it, or send it elsewhere.
/// Native callbacks may use the pointer only until they return synchronously.
pub(crate) unsafe fn with_cost<T>(
    s: &CommandCostServices,
    body: impl for<'a> FnOnce(crate::services::Cost<'a>) -> T,
) -> T {
    struct Guard<'a>(*mut CommandCost, &'a CommandCostServices);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            // SAFETY: the guard is dropped before its storage, exactly once,
            // and uses the actual pointer returned by native construction.
            unsafe {
                (self.1.destroy)(self.0);
            }
        }
    }
    let mut storage = std::mem::MaybeUninit::<crate::services::CommandCostStorage>::uninit();
    // SAFETY: aligned stack capacity stays at this fixed address until the guard
    // ends its native lifetime. Native static assertions prove the capacity.
    let p = unsafe { (s.construct)(storage.as_mut_ptr()) };
    let guard = Guard(p, s);
    let result = body(crate::services::Cost(p, s));
    drop(guard);
    result
}

/// Native default test of distinct nested stack results and early return cleanup.
/// # Safety
/// The supplied table constructs/destroys native objects in the advertised storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_stack_probe(s: &CommandCostServices) -> bool {
    unsafe {
        /* SAFETY: synchronous costs never escape or outlive this call. */
        with_cost(s, |outer| {
            with_cost(s, |inner| {
                outer.zero();
                inner.zero();
                outer.0 != inner.0 && outer.ok() && inner.ok()
            })
        })
    }
}

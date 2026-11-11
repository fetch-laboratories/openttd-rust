/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Command invocation lifecycle: `Command<Tcmd>::Do`, `Post`, `Unsafe` and `Execute`
//! control, process-wide recursion, admission, company restoration and money
//! reconciliation. The typed native templates call each decision step and run the
//! procedures between them, so a procedure's exception unwinds native frames only;
//! no Rust access spans a procedure. Exported predicates return `u8` 0 or 1: rustc
//! leaves bits 1 to 7 of a returned `bool` undefined, while GCC tests the whole byte.
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
    DoCommandFlag,
};
use crate::company_type::{COMPANY_SPECTATOR, OWNER_DEITY};
use crate::game_state::GameState;
use crate::newgrf_storage::{
    PSM_ENTER_COMMAND, PSM_ENTER_TESTMODE, PSM_LEAVE_COMMAND, PSM_LEAVE_TESTMODE,
};
use crate::openttd::{GameMode, PauseMode};
use crate::services::{CommandCost, CommandCostServices, CommandServices};
use crate::tile_type::TileIndex;

use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::TILE_SIZE;

pub(crate) fn before(s: &CommandServices, top_level: bool, test: bool) {
    if top_level {
        crate::objects::clear_areas();
    }
    if test {
        crate::town::set_town_rating_test_mode(s.shared, true);
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
        crate::town::set_town_rating_test_mode(s.shared, false);
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
) -> u8 {
    u8::from(unsafe { check_money(s, c, result) })
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
        openttd_rust_command_leave();
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
    if tile_rejected(shared, tile, flags & DoCommandFlag::ALL_TILES != 0) {
        out.error(INVALID_STRING_ID);
        return T::default();
    }
    let counter = Nesting::new();
    let top = counter.0;
    if top || flags & DoCommandFlag::EXECUTE == 0 {
        before(s, top, true);
        let extra = body(flags & !DoCommandFlag::EXECUTE);
        // SAFETY: the command caller supplies live exclusive native result storage.
        unsafe {
            after(s, out.1, out.0, flags, top, true);
        }
        if !out.ok() || flags & DoCommandFlag::EXECUTE == 0 {
            return extra;
        }
    }
    before(s, top, false);
    let extra = body(flags);
    // SAFETY: the command caller supplies live exclusive native result storage.
    unsafe {
        after(s, out.1, out.0, flags, top, false);
    }
    extra
}

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
            s.shared.map().log_x,
        );
    }
}

/// `IsLocalCompany`.
pub(crate) fn is_local_company(s: &CommandServices) -> bool {
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
) -> u8 {
    u8::from(allowed_while_paused(s, command_type))
}

/// `RecursiveCommandCounter` construction from a native command template.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_enter() -> u8 {
    u8::from(enter())
}

/// `RecursiveCommandCounter` destruction, also while an exception unwinds.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_leave() {
    // SAFETY: serialized game-state access.
    unsafe {
        *COUNTER.get() = (*COUNTER.get()).wrapping_sub(1);
    }
}

/// "Do not even think about executing out-of-bounds tile-commands."
fn tile_rejected(shared: &crate::services::Services, tile: u32, all_tiles: bool) -> bool {
    if tile == 0 {
        return false;
    }
    let world = crate::map_func::World::new(shared);
    let size = world.types.count();
    tile >= size
        || (!crate::tile_map::is_valid_tile(&world.types, TileIndex(tile), size).expect("live map")
            && !all_tiles)
}

/// Tile rejection of `Do` and `InternalPost`, before any nesting.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_tile_rejected(
    s: &CommandServices,
    tile: u32,
    all_tiles: bool,
) -> u8 {
    u8::from(tile_rejected(s.shared, tile, all_tiles))
}

/// `Do`: whether to run the test procedure, entering test or execution preparation.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_do_test(s: &CommandServices, top: bool, flags: u16) -> u8 {
    let test = top || flags & DoCommandFlag::EXECUTE == 0;
    before(s, top, test);
    u8::from(test)
}

/// `Do` after its test procedure: reconcile the result and, when execution follows,
/// prepare it. `false` returns the test result.
/// # Safety
/// `result` is the live exclusive test result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_do_tested(
    s: &CommandServices,
    result: *mut CommandCost,
    flags: u16,
    top: bool,
) -> u8 {
    // SAFETY: caller guarantees live exclusive storage.
    unsafe { after(s, s.cost, result, flags, top, true) };
    if !(s.cost.success)(result) || flags & DoCommandFlag::EXECUTE == 0 {
        return 0;
    }
    before(s, top, false);
    1
}

/// `Do` after its execution procedure.
/// # Safety
/// `result` is the live exclusive execution result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_do_executed(
    s: &CommandServices,
    result: *mut CommandCost,
    flags: u16,
    top: bool,
) {
    // SAFETY: caller guarantees live exclusive storage.
    unsafe { after(s, s.cost, result, flags, top, false) };
}

/// Company backup of the running top-level `Execute`; `Execute` never nests.
static BACKUP: GameState<Option<u8>> = GameState::new(None);

fn backup() -> Option<u8> {
    // SAFETY: serialized game-state access.
    unsafe { *BACKUP.get() }
}

fn set_backup(value: Option<u8>) {
    // SAFETY: serialized game-state access.
    unsafe { *BACKUP.get() = value };
}

/// `Backup::Restore`.
fn restore(s: &CommandServices) {
    if let Some(original) = backup() {
        (s.finance.set_current_company)(original);
    }
    set_backup(None);
}

/// `Backup::Verify` under the spectator rule of server and spectator commands.
fn verify_company(s: &CommandServices, cmd_flags: u16) -> bool {
    let current = (s.finance.current_company)();
    if cmd_flags & (CommandFlag::SPECTATOR | CommandFlag::SERVER) != 0 {
        current == COMPANY_SPECTATOR
    } else {
        Some(current) == backup()
    }
}

/// `Execute` up to its test procedure: company backup, admission of the current
/// company, spectator execution and test-mode entry. `false` returns `CMD_ERROR`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_prepare(
    s: &CommandServices,
    top: bool,
    cmd_flags: u16,
) -> u8 {
    /* Prevent recursion; it gives a mess over the network. */
    (s.signals.assert_condition)(top);
    let finance = s.finance;
    let current = (finance.current_company)();
    set_backup(Some(current));
    /* Always execute server and spectator commands as spectator. */
    let exec_as_spectator = cmd_flags & (CommandFlag::SPECTATOR | CommandFlag::SERVER) != 0;
    /* An invalid company may only do server commands or start a new company. */
    if crate::session::mode() == GameMode::NORMAL
        && !exec_as_spectator
        && !(s.shared.company_valid)(current)
        && !(current == OWNER_DEITY && cmd_flags & CommandFlag::DEITY != 0)
    {
        set_backup(None);
        return 0;
    }
    if exec_as_spectator {
        (finance.set_current_company)(COMPANY_SPECTATOR);
    }
    before(s, true, true);
    storage_mode(s, PSM_ENTER_TESTMODE);
    1
}

/// Decision of `Execute` after its test procedure.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tested {
    /// Restore the company and return the test result.
    Exit = 0,
    /// Log the test result as failed, restore the company and return the test result.
    ExitLogged = 1,
    /// Enqueue the command for the network, restore the company and return `{}`.
    Send = 2,
    /// Run the execution procedure; execution preparation is done.
    Execute = 3,
}

/// `Execute` after its test procedure: leave test mode, apply the money check and
/// decide between returning, sending and executing.
/// # Safety
/// `result` is the live exclusive test result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_tested(
    s: &CommandServices,
    result: *mut CommandCost,
    cmd_flags: u16,
    estimate_only: bool,
    network_command: bool,
) -> Tested {
    let finance = s.finance;
    storage_mode(s, PSM_LEAVE_TESTMODE);
    crate::town::set_town_rating_test_mode(s.shared, false);
    /* Make sure we're not messing things up here. */
    (s.signals.assert_condition)(verify_company(s, cmd_flags));
    /* Commands whose test and execution may cost differently skip the money check. */
    let test_and_exec_can_differ = cmd_flags & CommandFlag::NO_TEST != 0;
    // SAFETY: live exclusive native result.
    if !(s.cost.success)(result)
        || estimate_only
        || (!test_and_exec_can_differ && !unsafe { check_money(s, s.cost, result) })
    {
        let generating = crate::genworld::generating();
        let logged = !(finance.networking)() || generating || network_command;
        return if logged {
            Tested::ExitLogged
        } else {
            Tested::Exit
        };
    }
    let send_net = (finance.networking)() && !crate::genworld::generating() && !network_command;
    if send_net {
        return Tested::Send;
    }
    before(s, true, false);
    storage_mode(s, PSM_ENTER_COMMAND);
    Tested::Execute
}

/// `Backup::Restore` when `Execute` returns after its test procedure.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_restore(s: &CommandServices) {
    restore(s);
}

/// `InternalExecuteProcessResult` on the execution result in place.
/// # Safety
/// `test` and `exec` are the live test and exclusive execution results.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_finish(
    s: &CommandServices,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    test: *const CommandCost,
    exec: *mut CommandCost,
    extra_cash: i64,
    tile: u32,
) {
    let c = s.cost;
    let finance = s.finance;
    storage_mode(s, PSM_LEAVE_COMMAND);
    if cmd == CMD_COMPANY_CTRL {
        /* A new company switches to it, a closed one to spectator; the caller restores others. */
        set_backup(None);
        // SAFETY: the company table is immutable for the process lifetime.
        (finance.set_current_company)((unsafe { &*(s.company)() }.local_company)());
    } else {
        /* Make sure nothing bad happened, like changing the current company. */
        (s.signals.assert_condition)(verify_company(s, cmd_flags));
        restore(s);
    }
    if cmd_flags & CommandFlag::NO_TEST == 0 {
        (s.signals.assert_condition)(
            (c.money)(test) == (c.money)(exec) && (c.success)(test) == (c.success)(exec),
        );
    } else if !(c.success)(exec) {
        return;
    }
    if extra_cash != 0 && (c.money)(exec) == 0 {
        /* Removed rail can refund money and leave queued signals. */
        flush_signals(s);
        (c.error_param)(exec, s.not_enough_cash, extra_cash as u64);
        return;
    }
    if tile != 0 {
        (s.last_build_coordinate)((finance.current_company)(), tile);
    }
    // SAFETY: live native execution result.
    unsafe { subtract_money(s, c, exec) };
    /* Record a command during pause; pause and other setting changes do not count. */
    let pause = crate::session::pause();
    if pause != 0 && command_type != COMMAND_TYPE_SERVER_SETTING {
        crate::session::set_pause(pause | PauseMode::COMMAND_DURING_PAUSE.bit());
    }
    flush_signals(s);
}

/// `Backup::IsValid` for native destruction, before its diagnostic and restoration.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_backup_held() -> u8 {
    u8::from(backup().is_some())
}

/// Admission and presentation decisions of `InternalPostBefore`.
#[repr(C)]
pub struct Admission {
    pub rejected: bool,
    pub estimate_only: bool,
    pub only_sending: bool,
}

fn pixel(s: &CommandServices, tile: u32) -> (i32, i32) {
    let map = s.shared.map();
    (
        map.tile_x(TileIndex(tile)).wrapping_mul(TILE_SIZE) as i32,
        map.tile_y(TileIndex(tile)).wrapping_mul(TILE_SIZE) as i32,
    )
}

/// `InternalPostBefore`: estimate and send decisions and pause admission.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_admit(
    s: &CommandServices,
    command_type: u8,
    cmd_flags: u16,
    tile: u32,
    err_message: u32,
    network_command: bool,
) -> Admission {
    /* Estimates come from a local shift press, never from network, world
     * generation or the pause button. */
    let estimate_only = (s.shift_pressed)()
        && is_local_company(s)
        && !crate::genworld::generating()
        && !network_command
        && cmd_flags & CommandFlag::NO_EST == 0;
    /* Only sending: no fancy things for success. */
    let only_sending = (s.finance.networking)() && !network_command;
    let rejected =
        crate::session::pause() != 0 && !allowed_while_paused(s, command_type) && !estimate_only;
    if rejected {
        let (x, y) = pixel(s, tile);
        (s.paused_error)(err_message, x, y);
    }
    Admission {
        rejected,
        estimate_only,
        only_sending,
    }
}

/// `InternalPostResult`, then callback eligibility.
/// # Safety
/// `result` is the live final result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_posted(
    s: &CommandServices,
    result: *const CommandCost,
    tile: u32,
    estimate_only: bool,
    only_sending: bool,
    err_message: u32,
    my_cmd: bool,
    has_callback: bool,
) -> u8 {
    let c = s.cost;
    let (x, y) = pixel(s, tile);
    if !(c.success)(result) {
        /* Only show the error when it's for us. */
        if estimate_only || (is_local_company(s) && err_message != 0 && my_cmd) {
            (s.error_message)(err_message, x, y, result);
        }
    } else if estimate_only {
        (s.estimate)((c.money)(result), x, y);
    } else if !only_sending
        && tile != 0
        && is_local_company(s)
        && crate::session::mode() != GameMode::EDITOR
    {
        /* The cost animation shows only for an executed local command outside the editor. */
        let height = crate::movement::pixel_z(s.shared, x, y, false);
        (s.shared.cost_animation)(x, y, height, (c.money)(result));
    }
    u8::from(!estimate_only && !only_sending && has_callback)
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

/// `InternalPost`/`Execute` for Rust-owned procedures with no callback or tile argument.
/// Admission, test/send/execute and completion use the same lifecycle as native templates.
/// Procedures are synchronous and nonthrowing; native argument packing runs only for logs/send.
pub(crate) fn post(
    p: &crate::command_post::Services,
    cmd: u8,
    command_type: u8,
    cmd_flags: u16,
    client_set: bool,
    mut proc: impl FnMut(crate::services::Cost<'_>, u16),
    mut pack: impl FnMut() -> crate::network_command::CommandData,
) -> bool {
    let s = (p.command)();
    if tile_rejected(s.shared, 0, cmd_flags & CommandFlag::ALL_TILES != 0) {
        return false;
    }
    let admission = openttd_rust_command_admit(s, command_type, cmd_flags, 0, 0, false);
    if admission.rejected {
        return false;
    }
    // SAFETY: Each result stays native and live for the synchronous procedure/lifecycle calls.
    unsafe {
        with_cost(s.cost, |result| {
            let counter = Nesting::new();
            (s.signals.assert_condition)(counter.0);
            if cmd_flags & CommandFlag::CLIENT_ID != 0 {
                (s.signals.assert_condition)(client_set);
            }
            if openttd_rust_command_prepare(s, counter.0, cmd_flags) == 0 {
                result.error(INVALID_STRING_ID);
            } else {
                let flags = crate::command_func::command_flags_to_dc_flags(cmd_flags);
                proc(result, flags);
                match openttd_rust_command_tested(
                    s,
                    result.0,
                    cmd_flags,
                    admission.estimate_only,
                    false,
                ) {
                    Tested::ExitLogged => {
                        let data = pack();
                        (p.log)(cmd, data.span(), true);
                        drop(data);
                        restore(s);
                    }
                    Tested::Exit => restore(s),
                    Tested::Send => {
                        let data = pack();
                        crate::network_command::send_command(
                            (p.network)(),
                            cmd,
                            0,
                            std::ptr::null(),
                            (s.finance.current_company)(),
                            data.span(),
                        );
                        drop(data);
                        restore(s);
                        result.zero();
                    }
                    Tested::Execute => {
                        if (p.desync_logging)() {
                            let data = pack();
                            (p.log)(cmd, data.span(), false);
                        }
                        with_cost(s.cost, |exec| {
                            proc(exec, flags | DoCommandFlag::EXECUTE);
                            openttd_rust_command_finish(
                                s,
                                cmd,
                                command_type,
                                cmd_flags,
                                result.0,
                                exec.0,
                                0,
                                0,
                            );
                            result.take(exec);
                        });
                    }
                }
            }
            drop(counter);
            openttd_rust_command_posted(
                s,
                result.0,
                0,
                admission.estimate_only,
                admission.only_sending,
                0,
                true,
                false,
            );
            result.ok()
        })
    }
}

/// Command money short circuits, the cash-error call and executed subtraction. Unread finance
/// and money entries stay unreachable, so a skipped read fails.
#[cfg(test)]
mod money_tests {
    use super::*;
    use crate::company::Finances;
    use crate::economy_type::ExpensesType;
    use crate::test_tables::{COMMAND_COSTS, COMMANDS, FINANCE};
    use std::cell::Cell;

    /// The fixture's `CommandCost`: success, money and expense type.
    #[derive(Clone, Copy)]
    struct Result {
        ok: bool,
        money: i64,
        expense: ExpensesType,
    }
    fn cost(result: &mut Result) -> *mut CommandCost {
        std::ptr::from_mut(result).cast()
    }
    fn of(cost: *const CommandCost) -> Result {
        // SAFETY: Every fixture cost is a `Result`.
        unsafe { cost.cast::<Result>().read() }
    }
    extern "C" fn success(cost: *const CommandCost) -> bool {
        of(cost).ok
    }
    extern "C" fn money(cost: *const CommandCost) -> i64 {
        of(cost).money
    }
    extern "C" fn expense(cost: *const CommandCost) -> ExpensesType {
        of(cost).expense
    }
    extern "C" fn finite() -> bool {
        false
    }
    extern "C" fn infinite() -> bool {
        true
    }

    thread_local! {
        static CURRENT: Cell<u8> = const { Cell::new(0) };
        static OWNER: Cell<*mut Finances> = const { Cell::new(std::ptr::null_mut()) };
        static CASH_ERROR: Cell<Option<i64>> = const { Cell::new(None) };
    }
    extern "C" fn current_company() -> u8 {
        CURRENT.get()
    }
    extern "C" fn company_finances(id: u8) -> *mut Finances {
        if id == 0 {
            OWNER.get()
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn invalidate(_: u8) {}
    unsafe extern "C" fn money_error(_: *mut CommandCost, amount: i64) {
        CASH_ERROR.set(Some(amount));
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    /// Commands whose finance and infinite-money entries are never read.
    fn unread(costs: &'static CommandCostServices) -> CommandServices {
        let mut command = COMMANDS;
        command.cost = costs;
        command
    }
    /// `Do`'s steps: entry into test or execution, then reconciliation of that result.
    fn run(
        command: &CommandServices,
        result: &mut Result,
        flags: crate::command_type::DoCommandFlags,
        top: bool,
        test: bool,
    ) {
        let entry = openttd_rust_command_do_test(
            command,
            false,
            if test { 0 } else { DoCommandFlag::EXECUTE },
        );
        assert_eq!(entry != 0, test);
        // SAFETY: The fixture's live result.
        unsafe {
            if test {
                assert_eq!(
                    openttd_rust_command_do_tested(command, cost(result), flags.0, top),
                    0
                );
            } else {
                openttd_rust_command_do_executed(command, cost(result), flags.0, top);
            }
        }
    }

    /// A failed result, a nested call, bankruptcy and a cost query read no money.
    fn check_short_circuits(costs: &CommandCostServices) {
        let mut skipped = *costs;
        skipped.money = crate::test_tables::unreachable1;
        let command = unread(leak(skipped));
        for test in [false, true] {
            let mut failed = Result {
                ok: false,
                money: 0,
                expense: ExpensesType::EXPENSES_CONSTRUCTION,
            };
            run(
                &command,
                &mut failed,
                crate::command_type::DoCommandFlags::EMPTY,
                true,
                test,
            );
            assert!(!failed.ok);
            for flags in [
                crate::command_type::DoCommandFlags::EMPTY,
                crate::command_type::DoCommandFlags(DoCommandFlag::BANKRUPT),
                crate::command_type::DoCommandFlags(DoCommandFlag::QUERY_COST),
            ] {
                if !test && flags.0 == DoCommandFlag::QUERY_COST {
                    continue;
                }
                let mut result = Result {
                    ok: true,
                    money: 50,
                    expense: ExpensesType::EXPENSES_CONSTRUCTION,
                };
                run(&command, &mut result, flags, flags.0 != 0, test);
                assert!(result.ok, "flags {:#x} test {test}", flags.0);
            }
        }
    }

    #[test]
    fn money_short_circuits_and_cash_error() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let mut costs_table = COMMAND_COSTS;
        costs_table.success = success;
        costs_table.money = money;
        costs_table.expense = expense;
        let costs = leak(costs_table);
        check_short_circuits(costs);

        // Non-positive amounts pass before any finance read; infinite money passes.
        let mut command = unread(costs);
        for amount in [-5, 0] {
            let mut result = Result {
                ok: true,
                money: amount,
                expense: ExpensesType::EXPENSES_CONSTRUCTION,
            };
            // SAFETY: The fixture's live result.
            assert!(unsafe { check_money(&command, costs, cost(&mut result)) });
        }
        command.infinite_money = infinite;
        let mut huge = Result {
            ok: true,
            money: i64::MAX,
            expense: ExpensesType::EXPENSES_CONSTRUCTION,
        };
        // SAFETY: As above.
        assert!(unsafe { check_money(&command, costs, cost(&mut huge)) });

        // Company 0 holds 20: a cost of 21 fails once through the cash error, unchanged.
        command.infinite_money = finite;
        command.money_error = money_error;
        let mut finance = FINANCE;
        finance.current_company = current_company;
        finance.company_finances = company_finances;
        finance.invalidate_company_windows = invalidate;
        command.finance = leak(finance);
        // SAFETY: Plain integer record.
        let mut owner: Box<Finances> = Box::new(unsafe { std::mem::zeroed() });
        owner.money = 20;
        OWNER.set(&raw mut *owner);
        CURRENT.set(0);
        CASH_ERROR.set(None);
        let mut short = Result {
            ok: true,
            money: 21,
            expense: ExpensesType::EXPENSES_CONSTRUCTION,
        };
        // SAFETY: As above.
        assert!(!unsafe { check_money(&command, costs, cost(&mut short)) });
        assert_eq!((short.money, CASH_ERROR.get()), (21, Some(21)));

        // A zero cost subtracts nothing and reads no expense type.
        let mut no_expense = *costs;
        no_expense.expense = crate::test_tables::unreachable1;
        let mut zero = Result {
            ok: true,
            money: 0,
            expense: ExpensesType::EXPENSES_CONSTRUCTION,
        };
        // SAFETY: As above.
        unsafe { subtract_money(&command, &no_expense, cost(&mut zero)) };
        // Executed top-level commands subtract construction, revenue and running costs.
        for (amount, kind) in [
            (5, ExpensesType::EXPENSES_CONSTRUCTION),
            (-3, ExpensesType::EXPENSES_ROADVEH_REVENUE),
            (2, ExpensesType::EXPENSES_ROADVEH_RUN),
        ] {
            let mut result = Result {
                ok: true,
                money: amount,
                expense: kind,
            };
            run(
                &command,
                &mut result,
                crate::command_type::DoCommandFlags::EMPTY,
                true,
                false,
            );
        }
        assert_eq!(owner.money, 16);
        assert_eq!(
            owner.expenses[0][usize::from(ExpensesType::EXPENSES_CONSTRUCTION.0)],
            5
        );
        assert_eq!((owner.current.income, owner.current.expenses), (3, -2));
        // An invalid current company subtracts nothing.
        CURRENT.set(18);
        // SAFETY: As above.
        unsafe { subtract_money(&command, &no_expense, cost(&mut huge)) };
        assert_eq!(owner.money, 16);
        OWNER.set(std::ptr::null_mut());
    }
}

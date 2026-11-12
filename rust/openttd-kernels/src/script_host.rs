/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Script host owner (`script_instance.cpp`, `script_storage.hpp`, `script_object.cpp`,
//! `script_event.cpp`): lifecycle, suspension and callback state, the active instance,
//! company and command-mode context, pending command bytes, result, cost and error, and
//! the event FIFO. The Squirrel VM, its exceptions and object handles stay native: each
//! entry decides one step and returns, and the native frame runs VM calls, callbacks and
//! throws between steps, so no VM exception crosses Rust and no Rust access spans one.
//!
//! Every entry is serialized by the game-state mutex. A `Host` is the first field of its
//! boxed `Owner`; native code holds the `Host` pointer from `openttd_rust_script_host_new`
//! until `openttd_rust_script_host_free` and reads or writes its fields in place.
//! Exported predicates return `u8` 0 or 1 (see `command.rs`).
#![allow(unsafe_code, clippy::cast_sign_loss)]

use std::collections::VecDeque;

use crate::command_type::CMD_END;
use crate::company_type::{INVALID_OWNER, OWNER_DEITY};
use crate::game_state::GameState;
use crate::services::{CommandCost, CommandServices};

/// Opaque native `ScriptInstance`.
pub struct ScriptInstance {
    _private: [u8; 0],
}
/// Opaque native `ScriptObject` owning a command mode.
pub struct ScriptObject {
    _private: [u8; 0],
}
/// Opaque native `Script_SuspendCallbackProc`; stored here and called only natively.
pub struct ScriptCallback {
    _private: [u8; 0],
}
/// Opaque reference-counted native `ScriptEvent`.
pub struct ScriptEvent {
    _private: [u8; 0],
}

/// No mode object: `ScriptStorage::mode` or `async_mode` is null.
const MODE_NONE: u8 = 0;
/// A mode procedure returning true: `ScriptExecMode` or `ScriptAsyncMode(true)`.
const MODE_EXEC: u8 = 1;
/// A mode procedure returning false: `ScriptTestMode` or `ScriptAsyncMode(false)`.
const MODE_TEST: u8 = 2;

/// `ScriptError::ERR_NONE`.
const ERR_NONE: u32 = 0;
/// `ScriptError::ERR_PRECONDITION_INVALID_COMPANY`.
pub(crate) const ERR_PRECONDITION_INVALID_COMPANY: u32 = 4;

/// The maximum recursive depth for items stored in the savegame (`script_instance.hpp`).
pub(crate) const SQUIRREL_MAX_DEPTH: u32 = 25;

crate::abi::shared_records! {
    fn abi_record;
    /// Per-instance host state (`ScriptInstance` lifecycle fields and `ScriptStorage`).
    "OpenTTDScriptHost" ["script"] => pub struct Host {
        /// Native instance owning this host.
        pub instance: *mut ScriptInstance => class,
        /// Callback to call in the next tick the script runs.
        pub callback: *mut ScriptCallback,
        /// Object owning the current build mode.
        pub mode_instance: *mut ScriptObject => class,
        /// Object owning the current asynchronous mode.
        pub async_mode_instance: *mut ScriptObject => class,
        /// Extra data returned by the last command.
        pub last_cmd_ret: *const u8,
        /// Byte count of `last_cmd_ret`.
        pub last_cmd_ret_size: usize,
        /// Values that survive a callback; `callback_value_size` are initialized.
        pub callback_value: *mut i32,
        /// Element count of `callback_value`.
        pub callback_value_size: usize,
        /// Costs the script is tracking (`OverflowSafeInt64`).
        pub costs: i64,
        /// Cost of the last command.
        pub last_cost: i64,
        /// Ticks to suspend; negative values wait for a network command.
        pub suspend: i32,
        /// `ScriptController::ticks`.
        pub ticks: u32,
        /// Ticks of delay each command has.
        pub delay: u32,
        /// `ScriptErrorType` of the last command.
        pub last_error: u32,
        /// Current company.
        pub company: u8,
        /// Company the script really belongs to.
        pub root_company: u8,
        /// Current road type.
        pub road_type: u8,
        /// Current rail type.
        pub rail_type: u8,
        /// `MODE_*` build mode (`script_host_ffi.h`).
        pub mode: u8,
        /// `MODE_*` asynchronous mode.
        pub async_mode: u8,
        /// Whether commands may suspend the script.
        pub allow_do_command: bool,
        /// Result of the last command.
        pub last_command_res: bool,
        /// Whether the script constructor ran.
        pub is_started: bool,
        /// Whether the script stopped.
        pub is_dead: bool,
        /// Whether the script is paused.
        pub is_paused: bool,
        /// Whether the instance is being destructed.
        pub in_shutdown: bool,
        /// Whether save data is on the Squirrel stack.
        pub is_save_data_on_stack: bool,
    }
    /// Process-wide `ScriptObject::ActiveInstance::active`.
    "OpenTTDScriptContext" ["script"] => pub struct Context {
        /// Host of the active instance, or null.
        pub active: *mut Host,
    }
}

/// Native leaves of the host.
#[repr(C)]
pub struct Services {
    /// Shared command, company and result services.
    pub command: &'static CommandServices,
    /// `SimpleCountedObject::Release` of a queued event.
    pub release_event: unsafe extern "C" fn(*mut ScriptEvent),
    /// `ScriptError::StringToError`.
    pub string_to_error: extern "C" fn(u32) -> u32,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDScriptHostServices" ["script"] => Services {
        command,
        release_event,
        string_to_error,
    };
}

#[repr(C)]
struct Owner {
    host: Host,
    last_data: Vec<u8>,
    last_cmd: u8,
    result: Vec<u8>,
    callback_value: Vec<i32>,
    events: VecDeque<*mut ScriptEvent>,
}

static CONTEXT: GameState<Context> = GameState::new(Context {
    active: std::ptr::null_mut(),
});

/// # Safety
/// `host` came from `openttd_rust_script_host_new` and is not yet freed; no other
/// reference to its owner is live.
unsafe fn owner<'a>(host: *mut Host) -> &'a mut Owner {
    // SAFETY: `Host` is the first field of the `repr(C)` boxed `Owner`.
    unsafe { &mut *host.cast::<Owner>() }
}

/// Process-wide active-instance context, read in place natively.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_context() -> *mut Context {
    CONTEXT.get()
}

/// `ScriptObject::GetCompany` of the active instance.
pub(crate) fn active_company() -> u8 {
    // SAFETY: script API entries run with an active instance whose host is live.
    unsafe { (*(*CONTEXT.get()).active).company }
}

/// `ScriptObject::GetRailType` of the active instance.
pub(crate) fn active_rail_type() -> u8 {
    // SAFETY: as for `active_company`.
    unsafe { (*(*CONTEXT.get()).active).rail_type }
}

/// `ScriptObject::SetRailType` on the active instance.
pub(crate) fn set_active_rail_type(rail_type: u8) {
    // SAFETY: as for `active_company`; no other reference to the host is live.
    unsafe { (*(*CONTEXT.get()).active).rail_type = rail_type };
}

/// `ScriptObject::GetRoadType` of the active instance.
pub(crate) fn active_road_type() -> u8 {
    // SAFETY: as for `active_company`.
    unsafe { (*(*CONTEXT.get()).active).road_type }
}

/// `ScriptObject::SetRoadType` on the active instance.
pub(crate) fn set_active_road_type(road_type: u8) {
    // SAFETY: as for `active_company`; no other reference to the host is live.
    unsafe { (*(*CONTEXT.get()).active).road_type = road_type };
}

/// `ScriptObject::SetLastError` on the active instance.
pub(crate) fn set_active_error(error: u32) {
    // SAFETY: as for `active_company`; no other reference to the host is live.
    unsafe { (*(*CONTEXT.get()).active).last_error = error };
}

/// `ScriptStorage::ScriptStorage` for `instance`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_script_host_new(instance: *mut ScriptInstance) -> *mut Host {
    let owner = Box::new(Owner {
        host: Host {
            instance,
            callback: std::ptr::null_mut(),
            mode_instance: std::ptr::null_mut(),
            async_mode_instance: std::ptr::null_mut(),
            last_cmd_ret: std::ptr::null(),
            last_cmd_ret_size: 0,
            callback_value: std::ptr::null_mut(),
            callback_value_size: 0,
            costs: 0,
            last_cost: 0,
            suspend: 0,
            ticks: 0,
            delay: 1,
            last_error: ERR_NONE,
            company: INVALID_OWNER,
            root_company: INVALID_OWNER,
            road_type: crate::road_type::INVALID_ROADTYPE,
            rail_type: crate::rail_type::INVALID_RAILTYPE,
            mode: MODE_NONE,
            async_mode: MODE_NONE,
            allow_do_command: true,
            last_command_res: true,
            is_started: false,
            is_dead: false,
            is_paused: false,
            in_shutdown: false,
            is_save_data_on_stack: false,
        },
        last_data: Vec::new(),
        last_cmd: CMD_END,
        result: Vec::new(),
        callback_value: Vec::new(),
        events: VecDeque::new(),
    });
    Box::into_raw(owner).cast::<Host>()
}

/// `ScriptStorage::~ScriptStorage`: release queued events front to back, as the
/// `std::queue` destructor does, then free the owner.
/// # Safety
/// `host` is live and unused afterwards; `s` releases events without reentering it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_host_free(host: *mut Host, s: &Services) {
    // SAFETY: the caller transfers the box back.
    let owner = unsafe { Box::from_raw(host.cast::<Owner>()) };
    for event in owner.events.iter().copied() {
        // SAFETY: each queued event holds one native reference.
        unsafe { (s.release_event)(event) };
    }
}

/// Next step of `ScriptInstance::GameLoop` after its dead and crash checks.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// Return; the script is paused or suspended.
    Idle = 0,
    /// Call the stored callback.
    Callback = 1,
    /// Pop the save data, then call the stored callback.
    PopCallback = 2,
    /// Run the constructor, `Load` and `Start`, then mark the script started.
    Start = 3,
    /// Resume the VM.
    Resume = 4,
    /// Pop the save data, then resume the VM.
    PopResume = 5,
}

/// `ScriptInstance::GameLoop` from the pause check to its callback or VM step.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_loop(host: *mut Host, s: &Services) -> Step {
    // SAFETY: caller contract; the borrow ends before any native call.
    let h = unsafe { &mut owner(host).host };
    if h.is_paused {
        return Step::Idle;
    }
    h.ticks = h.ticks.wrapping_add(1);
    /* Multiplayer suspend, increase up to -1, then wait for Continue(). */
    if h.suspend < -1 {
        h.suspend += 1;
    }
    if h.suspend < 0 {
        return Step::Idle;
    }
    /* Singleplayer suspend, decrease to 0. */
    h.suspend -= 1;
    if h.suspend > 0 {
        return Step::Idle;
    }
    let company = h.company;
    let pending = !h.callback.is_null();
    let pop = pending && h.is_save_data_on_stack;
    (s.command.finance.set_current_company)(company);
    if pending {
        return if pop {
            Step::PopCallback
        } else {
            Step::Callback
        };
    }
    // SAFETY: caller contract.
    unsafe { openttd_rust_script_callback_done(host) }
}

/// `ScriptInstance::GameLoop` after its callback returned: clear the suspension and
/// choose between starting and resuming.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_callback_done(host: *mut Host) -> Step {
    // SAFETY: caller contract.
    let h = unsafe { &mut owner(host).host };
    h.suspend = 0;
    h.callback = std::ptr::null_mut();
    if !h.is_started {
        Step::Start
    } else if h.is_save_data_on_stack {
        Step::PopResume
    } else {
        Step::Resume
    }
}

/// A caught `Script_Suspend`: suspend for `time` ticks, then call `callback`.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_suspend(
    host: *mut Host,
    time: i32,
    callback: *mut ScriptCallback,
) {
    // SAFETY: caller contract.
    let h = unsafe { &mut owner(host).host };
    h.suspend = time;
    h.callback = callback;
}

/// `ScriptInstance::Continue` after a network command's callback.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_continue(host: *mut Host, s: &Services) {
    // SAFETY: caller contract; the borrow ends before the assertion leaf.
    let suspend = unsafe { owner(host).host.suspend };
    (s.command.signals.assert_condition)(suspend < 0);
    // SAFETY: caller contract.
    unsafe { owner(host).host.suspend = suspend.wrapping_neg().wrapping_sub(1) };
}

/// `ScriptObject::SetCompany`: latch the root company, then switch the current company.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_set_company(
    host: *mut Host,
    s: &Services,
    company: u8,
) {
    // SAFETY: caller contract; the borrow ends before the native setter.
    let h = unsafe { &mut owner(host).host };
    if h.root_company == INVALID_OWNER {
        h.root_company = company;
    }
    h.company = company;
    (s.command.finance.set_current_company)(company);
}

/// `ScriptObject::SetCallbackVariable`.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_set_callback_value(
    host: *mut Host,
    index: i32,
    value: i32,
) {
    // SAFETY: caller contract.
    let o = unsafe { owner(host) };
    /* The original converts a negative index to `size_t` for the comparison. */
    if index as usize >= o.callback_value.len() {
        o.callback_value.resize(index.wrapping_add(1) as usize, 0);
    }
    o.callback_value[index as usize] = value;
    o.host.callback_value = o.callback_value.as_mut_ptr();
    o.host.callback_value_size = o.callback_value.len();
}

/// `ScriptObject::SetLastCommand`.
/// # Safety
/// `host` is live; `data` is a readable span of `size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_set_last_command(
    host: *mut Host,
    data: *const u8,
    size: usize,
    cmd: u8,
) {
    // SAFETY: caller contract.
    let o = unsafe { owner(host) };
    o.last_data.clear();
    // SAFETY: caller contract.
    o.last_data
        .extend_from_slice(unsafe { crate::byte_strings::borrowed(data, size) });
    o.last_cmd = cmd;
}

/// `ScriptObject::SetLastCommandResData`.
/// # Safety
/// `host` is live; `data` is a readable span of `size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_set_result(
    host: *mut Host,
    data: *const u8,
    size: usize,
) {
    // SAFETY: caller contract.
    let o = unsafe { owner(host) };
    o.result.clear();
    // SAFETY: caller contract.
    o.result
        .extend_from_slice(unsafe { crate::byte_strings::borrowed(data, size) });
    o.host.last_cmd_ret = o.result.as_ptr();
    o.host.last_cmd_ret_size = o.result.len();
}

/// `ScriptObject::DoCommandPrep` after its `CanSuspend` check.
#[repr(C)]
pub struct Prep {
    pub rejected: bool,
    pub estimate_only: bool,
    pub asynchronous: bool,
    pub networking: bool,
}

/// `ScriptObject::DoCommandPrep`: modes, networking and the company precondition.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_command_prep(host: *mut Host, s: &Services) -> Prep {
    // SAFETY: caller contract; copies end the borrow.
    let (mode, async_mode, company) = unsafe {
        let h = &owner(host).host;
        (h.mode, h.async_mode, h.company)
    };
    /* Are we only interested in the estimate costs? */
    let estimate_only = mode == MODE_TEST;
    /* Should the command be executed asynchronously? */
    let asynchronous = async_mode == MODE_EXEC;
    let networking = (s.command.finance.networking)() && !crate::genworld::generating();
    let rejected = company != OWNER_DEITY && !(s.command.shared.company_valid)(company);
    if rejected {
        // SAFETY: caller contract.
        unsafe { owner(host).host.last_error = ERR_PRECONDITION_INVALID_COMPANY };
    }
    Prep {
        rejected,
        estimate_only,
        asynchronous,
        networking,
    }
}

/// What `ScriptObject::DoCommandProcessResult` does after its accounting.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// Return false.
    Failed = 0,
    /// Return true.
    Done = 1,
    /// Call the callback and throw its stack result.
    Callback = 2,
    /// Charge the nominal asynchronous fee, then call the callback and throw.
    ChargedCallback = 3,
    /// Throw `Script_Suspend(suspend, callback)`.
    Suspend = 4,
}

/// Decision of `ScriptObject::DoCommandProcessResult` with its suspension time.
#[repr(C)]
pub struct Processed {
    pub outcome: Outcome,
    pub suspend: i32,
}

/// `ScriptObject::DoCommandProcessResult` up to its callback and throws.
/// # Safety
/// `host` is live; `result` is a live native result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_command_result(
    host: *mut Host,
    s: &Services,
    result: *const CommandCost,
    estimate_only: bool,
    asynchronous: bool,
) -> Processed {
    let c = s.command.cost;
    let done = |outcome, suspend| Processed { outcome, suspend };
    /* We failed; set the error and bail out. */
    if !(c.success)(result) {
        let error = (s.string_to_error)((c.error_message)(result));
        // SAFETY: caller contract.
        unsafe { owner(host).host.last_error = error };
        return done(Outcome::Failed, 0);
    }
    let money = (c.money)(result);
    let generating = crate::genworld::generating();
    let networking = (s.command.finance.networking)();
    // SAFETY: caller contract; no native call while borrowed.
    let h = unsafe { &mut owner(host).host };
    /* No error, then clear it. */
    h.last_error = ERR_NONE;
    /* Estimates, update the cost for the estimate and be done. */
    if estimate_only {
        crate::overflowsafe_type::add_i64(&mut h.costs, money);
        return done(Outcome::Done, 0);
    }
    h.last_cost = money;
    h.last_command_res = true;
    if generating || asynchronous {
        crate::overflowsafe_type::add_i64(&mut h.costs, money);
        let outcome = if generating {
            Outcome::Callback
        } else {
            Outcome::ChargedCallback
        };
        return done(outcome, 0);
    }
    if networking {
        /* Suspend the script till the command is really executed. */
        return done(Outcome::Suspend, h.delay.cast_signed().wrapping_neg());
    }
    crate::overflowsafe_type::add_i64(&mut h.costs, money);
    /* Suspend the script for 1+ ticks, so it simulates multiplayer. */
    done(Outcome::Suspend, h.delay.cast_signed())
}

/// `ScriptInstance::DoCommandCallback` after its active-instance scope: check the
/// pending command, store the result and clear the pending command. Returns 0 when
/// the command differs and the script must terminate.
/// # Safety
/// `host` is live; `result` is a live native result; both spans are readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_command_done(
    host: *mut Host,
    s: &Services,
    result: *const CommandCost,
    data: *const u8,
    size: usize,
    result_data: *const u8,
    result_size: usize,
    cmd: u8,
) -> u8 {
    // SAFETY: caller contract; the borrow ends before native leaves.
    let matches = unsafe {
        let o = owner(host);
        o.last_cmd == cmd && o.last_data.as_slice() == crate::byte_strings::borrowed(data, size)
    };
    if !matches {
        return 0;
    }
    let c = s.command.cost;
    let success = (c.success)(result);
    // SAFETY: caller contract.
    unsafe {
        owner(host).host.last_command_res = success;
        openttd_rust_script_set_result(host, result_data, result_size);
    }
    if success {
        let money = (c.money)(result);
        // SAFETY: caller contract.
        let h = unsafe { &mut owner(host).host };
        crate::overflowsafe_type::add_i64(&mut h.costs, money);
        h.last_cost = money;
    } else {
        let error = (s.string_to_error)((c.error_message)(result));
        // SAFETY: caller contract.
        unsafe { owner(host).host.last_error = error };
    }
    // SAFETY: caller contract.
    unsafe { openttd_rust_script_set_last_command(host, std::ptr::null(), 0, CMD_END) };
    1
}

/// `ScriptEventController::InsertEvent`: queue `event`, whose reference the caller added.
/// # Safety
/// `host` is live; `event` holds one native reference transferred to the queue.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_event_push(host: *mut Host, event: *mut ScriptEvent) {
    // SAFETY: caller contract.
    unsafe { owner(host).events.push_back(event) };
}

/// `ScriptEventController::GetNextEvent`: the oldest event and its reference, or null.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_event_pop(host: *mut Host) -> *mut ScriptEvent {
    // SAFETY: caller contract.
    unsafe { owner(host).events.pop_front() }.unwrap_or(std::ptr::null_mut())
}

/// `ScriptEventController::IsEventWaiting`.
/// # Safety
/// `host` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_script_event_waiting(host: *mut Host) -> u8 {
    // SAFETY: caller contract.
    u8::from(!unsafe { owner(host) }.events.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_data_stays_marked_until_native_pop() {
        let host = openttd_rust_script_host_new(std::ptr::null_mut());
        // SAFETY: this test exclusively owns the host and queues no native events.
        unsafe {
            (*host).is_started = true;
            (*host).is_save_data_on_stack = true;
            assert_eq!(openttd_rust_script_callback_done(host), Step::PopResume);
            // ScriptInstance::GameLoop clears this only after sq_poptop returns.
            assert!((*host).is_save_data_on_stack);
            drop(Box::from_raw(host.cast::<Owner>()));
        }
    }
}

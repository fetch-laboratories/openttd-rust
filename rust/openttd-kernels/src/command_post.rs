/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Typed Rust command posts. Native traits pack arguments; command.rs owns invocation policy.
//! Provider tables and ownership scratch outlive each synchronous procedure; no borrow of
//! company/world state crosses a native call or a procedure that can delete a company.
#![allow(unsafe_code, missing_docs)]
use crate::command_type::{
    CMD_COMPANY_CTRL, CMD_PAUSE, CMD_SET_COMPANY_COLOUR, COMMAND_TYPE_COMPANY_SETTING,
    COMMAND_TYPE_SERVER_SETTING, CommandFlag, DoCommandFlag,
};
use crate::network_command::CommandData;
use crate::openttd::PauseMode;
use crate::services::{ByteSpan, CommandCost, CommandServices, Cost};
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDCommandControlStorage" ["probe"] => pub struct ControlStorage {
        pub words: [u64; 128] => "std::array<uint64_t, 128>",
    }
}
#[repr(C)]
pub struct Services {
    pub(crate) assign_company_cost: extern "C" fn(*mut CommandCost, *const crate::company::Cost),
    pub(crate) command: extern "C" fn() -> &'static CommandServices,
    pub(crate) control_begin:
        extern "C" fn(*mut ControlStorage) -> *const crate::tile_owner::Context,
    pub(crate) control_end: extern "C" fn(*mut ControlStorage),
    pub(crate) desync_logging: extern "C" fn() -> bool,
    pub(crate) log: extern "C" fn(u8, ByteSpan, bool),
    pub(crate) network: extern "C" fn() -> &'static crate::network_command::Services,
    pub(crate) pack_colour: extern "C" fn(bool, u8) -> CommandData,
    pub(crate) pack_control: extern "C" fn(u8, u8, u8, u32) -> CommandData,
    pub(crate) pack_pause: extern "C" fn(PauseMode, bool) -> CommandData,
    pub(crate) session: extern "C" fn() -> &'static crate::session::Services,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCommandPostServices" ["probe"] => Services {
        assign_company_cost, command, control_begin, control_end, desync_logging, log,
        network, pack_colour, pack_control, pack_pause, session,
    };
}

pub(crate) fn pause(shared: &crate::services::Services, mode: PauseMode, paused: bool) -> bool {
    let p = (shared.command_post)();
    crate::command::post(
        p,
        CMD_PAUSE,
        COMMAND_TYPE_SERVER_SETTING,
        CommandFlag::SERVER | CommandFlag::NO_EST,
        true,
        |out, flags| {
            if crate::session::openttd_rust_session_pause_abi(
                (p.session)(),
                mode,
                paused,
                flags & DoCommandFlag::EXECUTE != 0,
            ) != 0
            {
                out.zero();
            } else {
                out.error(crate::strings_type::INVALID_STRING_ID);
            }
        },
        || (p.pack_pause)(mode, paused),
    )
}

pub(crate) fn colour(shared: &crate::services::Services, primary: bool, colour: u8) -> bool {
    let p = (shared.command_post)();
    crate::command::post(
        p,
        CMD_SET_COMPANY_COLOUR,
        COMMAND_TYPE_COMPANY_SETTING,
        0,
        true,
        |out, flags| {
            let s = unsafe { &*((p.command)().company)() };
            if crate::company_identity::openttd_rust_company_colour_abi(
                s.identity,
                crate::livery::LS_DEFAULT,
                primary,
                colour,
                flags & DoCommandFlag::EXECUTE != 0,
            ) != 0
            {
                out.zero();
            } else {
                out.error(crate::strings_type::INVALID_STRING_ID);
            }
        },
        || (p.pack_colour)(primary, colour),
    )
}

fn control_cost(
    p: &Services,
    out: Cost<'_>,
    action: u8,
    target: u8,
    reason: u8,
    client: u32,
    execute: bool,
) {
    let mut storage = std::mem::MaybeUninit::<ControlStorage>::uninit();
    let context = (p.control_begin)(storage.as_mut_ptr());
    let mut frame = crate::company::Control {
        result: crate::company::Cost {
            cost: 0,
            param: 0,
            error: 0,
            expense: 0,
        },
        startup: crate::company::Startup {
            company: 0,
            requested: 0,
            is_ai: false,
            stage: 0,
        },
        client,
        action,
        target,
        reason,
        execute,
        stage: 0,
    };
    // Human creation and deletion never request the AI startup action.
    crate::company::control(
        unsafe { &*((p.command)().company)() },
        unsafe { &*context },
        &mut frame,
    );
    (p.assign_company_cost)(out.0, &raw const frame.result);
    (p.control_end)(storage.as_mut_ptr());
}

fn control(
    shared: &crate::services::Services,
    action: u8,
    target: u8,
    reason: u8,
    client: u32,
) -> bool {
    let p = (shared.command_post)();
    let client = if client == crate::network_type::INVALID_CLIENT_ID {
        crate::network_type::CLIENT_ID_SERVER
    } else {
        client
    };
    crate::command::post(
        p,
        CMD_COMPANY_CTRL,
        COMMAND_TYPE_SERVER_SETTING,
        CommandFlag::SPECTATOR | CommandFlag::CLIENT_ID | CommandFlag::NO_EST,
        client != crate::network_type::INVALID_CLIENT_ID,
        |out, flags| {
            control_cost(
                p,
                out,
                action,
                target,
                reason,
                client,
                flags & DoCommandFlag::EXECUTE != 0,
            );
        },
        || (p.pack_control)(action, target, reason, client),
    )
}
pub(crate) fn new_company(shared: &crate::services::Services, client: u32) -> bool {
    control(
        shared,
        crate::company_type::CCA_NEW,
        crate::company_type::INVALID_OWNER,
        crate::company_type::CRR_NONE,
        client,
    )
}
pub(crate) fn delete_company(shared: &crate::services::Services, id: u8) -> bool {
    control(
        shared,
        crate::company_type::CCA_DELETE,
        id,
        crate::company_type::CRR_BANKRUPT,
        crate::network_type::INVALID_CLIENT_ID,
    )
}

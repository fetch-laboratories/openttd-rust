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
use crate::services::{ByteSpan, CommandServices, Cost};
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDCommandControlStorage" ["probe"] => pub struct ControlStorage {
        pub words: [u64; 128] => "std::array<uint64_t, 128>",
    }
}
#[repr(C)]
pub struct Services {
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
        command, control_begin, control_end, desync_logging, log,
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
            crate::company_identity::cmd_set_company_colour(
                s.identity,
                out,
                crate::command_type::DoCommandFlags(flags),
                crate::company_identity::CmdSetCompanyColour {
                    scheme: crate::livery::LiveryScheme::LS_DEFAULT,
                    primary,
                    colour: crate::gfx_type::Colours(colour),
                },
            );
        },
        || (p.pack_colour)(primary, colour),
    )
}

/// `CmdCompanyCtrl` executed for a `Post`; human creation and deletion never request the AI
/// startup action, so the frame needs no continuation.
fn control_cost(
    p: &Services,
    out: Cost<'_>,
    flags: crate::command_type::DoCommandFlags,
    args: crate::company::CmdCompanyCtrl,
) {
    let mut storage = std::mem::MaybeUninit::<ControlStorage>::uninit();
    let context = (p.control_begin)(storage.as_mut_ptr());
    let mut frame = crate::company::Control::new(flags, args);
    crate::company::cmd_company_ctrl(
        unsafe { &*((p.command)().company)() },
        unsafe { &*context },
        out,
        &mut frame,
    );
    (p.control_end)(storage.as_mut_ptr());
}

/// `Command<CMD_COMPANY_CTRL>::Post(cca, company_id, reason, client_id)`.
fn control(shared: &crate::services::Services, args: crate::company::CmdCompanyCtrl) -> bool {
    let p = (shared.command_post)();
    let client = if args.client_id.0 == crate::network_type::INVALID_CLIENT_ID {
        crate::network_type::CLIENT_ID_SERVER
    } else {
        args.client_id.0
    };
    let args = crate::company::CmdCompanyCtrl {
        client_id: crate::network_type::ClientID(client),
        ..args
    };
    crate::command::post(
        p,
        CMD_COMPANY_CTRL,
        COMMAND_TYPE_SERVER_SETTING,
        CommandFlag::SPECTATOR | CommandFlag::CLIENT_ID | CommandFlag::NO_EST,
        client != crate::network_type::INVALID_CLIENT_ID,
        |out, flags| control_cost(p, out, crate::command_type::DoCommandFlags(flags), args),
        || (p.pack_control)(args.cca.0, args.company_id.0, args.reason.0, client),
    )
}
/// `Command<CMD_COMPANY_CTRL>::Post(CCA_NEW, INVALID_COMPANY, CRR_NONE, client)`.
pub(crate) fn new_company(shared: &crate::services::Services, client: u32) -> bool {
    control(
        shared,
        crate::company::CmdCompanyCtrl {
            cca: crate::company_type::CompanyCtrlAction::CCA_NEW,
            company_id: crate::company_type::CompanyID::INVALID,
            reason: crate::company_type::CompanyRemoveReason::CRR_NONE,
            client_id: crate::network_type::ClientID(client),
        },
    )
}
/// `Command<CMD_COMPANY_CTRL>::Post(CCA_DELETE, company, CRR_BANKRUPT, INVALID_CLIENT_ID)`.
pub(crate) fn delete_company(
    shared: &crate::services::Services,
    company: crate::company_type::CompanyID,
) -> bool {
    control(
        shared,
        crate::company::CmdCompanyCtrl {
            cca: crate::company_type::CompanyCtrlAction::CCA_DELETE,
            company_id: company,
            reason: crate::company_type::CompanyRemoveReason::CRR_BANKRUPT,
            client_id: crate::network_type::ClientID(crate::network_type::INVALID_CLIENT_ID),
        },
    )
}

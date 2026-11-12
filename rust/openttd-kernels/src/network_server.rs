/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ServerNetworkGameSocketHandler::ClientStatus` from `network_server.h`, read and written in
//! place on the server-side game socket shell, and `NetworkServerNewCompany`.

use crate::command_type::Commands;
use crate::company_type::{CompanyID, OpenTTDCompany};
use crate::network_frame::Services as FrameServices;
use crate::network_lifecycle::Services as LifecycleServices;
use crate::network_socket::{self as socket, GameSocket};
use crate::strings_type::StringID;
use std::ffi::c_void;

/// Status of a client, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientStatus(pub u8);

impl ClientStatus {
    /// The client is not connected nor active.
    pub const STATUS_INACTIVE: Self = Self(0);
    /// The client is authorizing with game (server) password.
    pub const STATUS_AUTH_GAME: Self = Self(1);
    /// The client is identifying itself.
    pub const STATUS_IDENTIFY: Self = Self(2);
    /// The client is checking `NewGRFs`.
    pub const STATUS_NEWGRFS_CHECK: Self = Self(3);
    /// The client is authorized.
    pub const STATUS_AUTHORIZED: Self = Self(4);
    /// The client is waiting as someone else is downloading the map.
    pub const STATUS_MAP_WAIT: Self = Self(5);
    /// The client is downloading the map.
    pub const STATUS_MAP: Self = Self(6);
    /// The client has downloaded the map.
    pub const STATUS_DONE_MAP: Self = Self(7);
    /// The client is catching up the delayed frames.
    pub const STATUS_PRE_ACTIVE: Self = Self(8);
    /// The client is active within in the game.
    pub const STATUS_ACTIVE: Self = Self(9);
    /// Must ALWAYS be on the end of this list!! (period).
    pub const STATUS_END: Self = Self(10);
}

/// `ServerNetworkGameSocketHandler::status` of a live server-side socket.
pub(crate) fn status(cs: *mut GameSocket) -> ClientStatus {
    ClientStatus(socket::status(cs))
}

/// Stores `ServerNetworkGameSocketHandler::status` of a live server-side socket.
pub(crate) fn set_status(cs: *mut GameSocket, status: ClientStatus) {
    socket::set_status(cs, status.0);
}

/// `NetworkServerNewCompany`: perform all the server specific administration of a new
/// company. `c` is the newly created company and can't be null; `ci` is the client
/// information of the client that made the company and can be null.
pub(crate) fn new_company(
    s: &LifecycleServices,
    f: &FrameServices,
    c: *mut OpenTTDCompany,
    ci: *mut c_void,
) {
    crate::network_frame::packet_check(f)(!c.is_null());

    if !(f.command.company.network_server)() {
        return;
    }

    if !ci.is_null() {
        /* ci is nullptr when replaying, or for AIs. In neither case there is a client. */
        let index: CompanyID = (f.command.company.identity.company_id)(c);
        (s.set_info_company)(ci, index.0);
        (s.update_client_info)(ci);

        /*
         * This function is called from a command, but is only called for the server.
         * The client information is managed out-of-band from the commands, so to not have a
         * different state/president/company name in the different clients, we need to
         * circumvent the normal ::Post logic and go directly to sending the command.
         */
        let allow = (s.pack_company_allow)(ci);
        crate::network_command::send_command(
            f.command,
            Commands::CMD_COMPANY_ALLOW_LIST_CTRL,
            StringID::STR_NULL,
            std::ptr::null(),
            index,
            allow.span(),
        );
        drop(allow);
        let president = (s.pack_president)(ci);
        crate::network_command::send_command(
            f.command,
            Commands::CMD_RENAME_PRESIDENT,
            StringID::STR_NULL,
            std::ptr::null(),
            index,
            president.span(),
        );
        drop(president);

        (s.announce_company)(ci, index.0);
    }
}

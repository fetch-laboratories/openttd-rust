/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ServerNetworkGameSocketHandler::ClientStatus` from `network_server.h`, read and written in
//! place on the server-side game socket shell, `ServerNetworkGameSocketHandler::GetName` and
//! `GetClientName`, `NetworkServerSendChat` and `NetworkServerNewCompany`.

use crate::command_type::Commands;
use crate::company_type::{CompanyID, OpenTTDCompany};
use crate::network_connection::ClientInfo;
use crate::network_frame::Services as FrameServices;
use crate::network_lifecycle::Services as LifecycleServices;
use crate::network_socket::{self as socket, GameSocket};
use crate::network_type::{ClientID, DestType, NetworkAction};
use crate::services::ByteSpan;
use crate::table::strings::STR_NULL;

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

/// `ServerNetworkGameSocketHandler::GetName`.
pub(crate) const fn get_name() -> &'static str {
    "server"
}

/// `NetworkServerSendChat(action, type, dest, msg, from_id, data, from_admin)`: send a chat
/// message from `from_id` to the destination `dest` of the given type; pass `0` and `false`
/// for the original's defaults of `data` and `from_admin`.
pub(crate) fn network_server_send_chat(
    action: NetworkAction,
    r#type: DestType,
    dest: i32,
    msg: &[u8],
    from_id: ClientID,
    data: i64,
    from_admin: bool,
) {
    (crate::news_func::services().network_server_send_chat)(
        action.0,
        r#type.0,
        dest,
        ByteSpan {
            data: msg.as_ptr(),
            size: msg.len(),
        },
        from_id.0,
        data,
        from_admin,
    );
}

/// `ServerNetworkGameSocketHandler::GetClientName`: get the name of the client, if the user
/// did not send it yet, Client ID is used.
///
/// # Safety
/// `cs` is a live server socket, the member function's receiver.
#[allow(
    unsafe_code,
    reason = "a member function of a live native socket, reading its client info's name"
)]
pub(crate) unsafe fn get_client_name(f: &FrameServices, cs: *mut GameSocket) -> Vec<u8> {
    let ci = ClientInfo::get_info(f.connection, cs);
    // SAFETY: A non-null `GetInfo()` of a live socket is live, and the name is copied before
    // any other call.
    if !ci.is_null() && !unsafe { ci.client_name(f.connection) }.is_empty() {
        return unsafe { ci.client_name(f.connection) }.to_vec();
    }

    format!("Client #{}", (f.dispatch.client_id)(cs)).into_bytes()
}

/// `NetworkServerNewCompany`: perform all the server specific administration of a new
/// company. `c` is the newly created company and can't be null; `ci` is the client
/// information of the client that made the company and can be null.
///
/// # Safety
/// `ci` is null or a live record that nothing deletes before this function returns.
#[allow(
    unsafe_code,
    reason = "takes a client info that is null or live, reading its ID after the null check"
)]
pub(crate) unsafe fn new_company(
    s: &LifecycleServices,
    f: &FrameServices,
    c: *mut OpenTTDCompany,
    ci: ClientInfo,
) {
    crate::network_frame::packet_check(f)(!c.is_null());

    if !(f.command.company.network_server)() {
        return;
    }

    if !ci.is_null() {
        /* ci is nullptr when replaying, or for AIs. In neither case there is a client. */
        let index: CompanyID = (f.command.company.identity.company_id)(c);
        (s.set_info_company)(ci.as_ptr(), index.0);
        (s.update_client_info)(ci.as_ptr());

        /*
         * This function is called from a command, but is only called for the server.
         * The client information is managed out-of-band from the commands, so to not have a
         * different state/president/company name in the different clients, we need to
         * circumvent the normal ::Post logic and go directly to sending the command.
         */
        let allow = (s.pack_company_allow)(ci.as_ptr());
        crate::network_command::send_command(
            f.command,
            Commands::CMD_COMPANY_ALLOW_LIST_CTRL,
            STR_NULL,
            std::ptr::null(),
            index,
            allow.span(),
        );
        drop(allow);
        let president = (s.pack_president)(ci.as_ptr());
        crate::network_command::send_command(
            f.command,
            Commands::CMD_RENAME_PRESIDENT,
            STR_NULL,
            std::ptr::null(),
            index,
            president.span(),
        );
        drop(president);

        network_server_send_chat(
            NetworkAction::NETWORK_ACTION_COMPANY_NEW,
            DestType::DESTTYPE_BROADCAST,
            0,
            b"",
            // SAFETY: `ci` is not null here, and live by this function's contract.
            unsafe { ci.client_id(f.connection) },
            i64::from(index.0) + 1,
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::network_type::{ClientID, DestType, NetworkAction};
    use crate::news_func::testing::{Effect, take};

    /// Each of the seven arguments reaches the native call unchanged and in its position.
    #[test]
    fn network_server_send_chat_passes_every_argument() {
        take();
        super::network_server_send_chat(
            NetworkAction::NETWORK_ACTION_SERVER_MESSAGE,
            DestType::DESTTYPE_CLIENT,
            -7,
            b"full\xFF",
            ClientID::CLIENT_ID_SERVER,
            i64::MIN,
            true,
        );
        assert_eq!(
            take(),
            [Effect::NetworkServerSendChat {
                action: 2,
                r#type: 2,
                dest: -7,
                msg: b"full\xFF".to_vec(),
                from_id: 1,
                data: i64::MIN,
                from_admin: true,
            }]
        );
    }
}

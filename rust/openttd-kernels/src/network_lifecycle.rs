/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Session connect, start, reboot and disconnect orchestration. Native pools, address parsing,
//! admin/coordinator protocols, authentication and presentation remain named native operations.
//! All scalar pointers address canonical native fields. Reconnect constructs native strings in
//! call-local u64 arrays whose size/alignment is checked natively; no state borrow spans a leaf.
#![allow(unsafe_code, missing_docs)]
use crate::debug::{Bytes, debug};
use crate::network_action::{Actions, Resume};
use crate::network_command::next_client;
use crate::network_frame::Services as FrameServices;
use crate::network_socket as socket;
use crate::network_type::{CLIENT_ID_SERVER, NetworkRecvStatus, SERVER_GAME_TYPE_LOCAL};
use crate::openttd::{GameMode, SwitchMode};
use crate::services::ByteSpan;
use crate::session::Services as SessionServices;
use std::ffi::c_void;
#[repr(C)]
pub struct Services {
    pub admin_auth_configured: extern "C" fn() -> bool,
    pub admin_port: extern "C" fn() -> u16,
    pub admins_close: extern "C" fn(),
    pub admins_newgame: extern "C" fn(),
    pub admins_shutdown: extern "C" fn(),
    pub admins_welcome: extern "C" fn(),
    pub allocate_server_info: extern "C" fn() -> *mut c_void,
    pub announce_company: extern "C" fn(*mut c_void, u8),
    pub auth_client_key: extern "C" fn(),
    pub check_names: extern "C" fn(),
    pub close_admin_listeners: extern "C" fn(),
    pub close_client_listeners: extern "C" fn(),
    pub connect_text_begin: extern "C" fn(*mut crate::session::Frame, bool) -> *mut c_void,
    pub connect_text_end: extern "C" fn(*mut crate::session::Frame),
    pub coordinator_close_client: extern "C" fn(),
    pub coordinator_close_server: extern "C" fn(),
    pub coordinator_register: extern "C" fn(),
    pub create_client_connecter: extern "C" fn(),
    pub fill_game_info: extern "C" fn(),
    pub initialize_pools: extern "C" fn(bool),
    pub join_connection: extern "C" fn() -> crate::services::ByteSpan,
    pub kill_connecters: extern "C" fn(),
    pub last_joined: extern "C" fn() -> crate::services::ByteSpan,
    pub listen_admins: extern "C" fn(u16) -> bool,
    pub listen_clients: extern "C" fn(u16) -> bool,
    pub on_dedicated: extern "C" fn(*mut crate::session::Frame) -> bool,
    pub on_server: extern "C" fn(*mut crate::session::Frame) -> bool,
    pub pack_company_allow: extern "C" fn(*mut c_void) -> crate::network_command::CommandData,
    pub pack_president: extern "C" fn(*mut c_void) -> crate::network_command::CommandData,
    pub parse_connection: extern "C" fn(crate::services::ByteSpan, *mut u8, *mut c_void),
    pub pre_dedicated: extern "C" fn(*mut crate::session::Frame) -> bool,
    pub pre_server: extern "C" fn(*mut crate::session::Frame) -> bool,
    pub remember_joined_server: extern "C" fn(ByteSpan),
    pub reset_restart_time: extern "C" fn(),
    pub server_game_type: extern "C" fn() -> u8,
    pub server_port: extern "C" fn() -> u16,
    pub set_info_company: extern "C" fn(*mut c_void, u8),
    pub set_join_info: extern "C" fn(*mut c_void, u8, *const c_void),
    pub set_network_server: extern "C" fn(bool),
    pub set_networking: extern "C" fn(bool),
    pub set_server_info_key: extern "C" fn(*mut c_void),
    pub udp_initialize: extern "C" fn(),
    pub udp_listen: extern "C" fn(),
    pub update_client_info: extern "C" fn(*mut c_void),
    pub update_game_info: extern "C" fn(),
    pub validate_client_name: extern "C" fn() -> bool,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkLifecycleServices" ["probe"] => Services {
        admin_auth_configured,
        admin_port,
        admins_close,
        admins_newgame,
        admins_shutdown,
        admins_welcome,
        allocate_server_info,
        announce_company,
        auth_client_key,
        check_names,
        close_admin_listeners,
        close_client_listeners,
        connect_text_begin,
        connect_text_end,
        coordinator_close_client,
        coordinator_close_server,
        coordinator_register,
        create_client_connecter,
        fill_game_info,
        initialize_pools,
        join_connection,
        kill_connecters,
        last_joined,
        listen_admins,
        listen_clients,
        on_dedicated,
        on_server,
        pack_company_allow,
        pack_president,
        parse_connection,
        pre_dedicated,
        pre_server,
        remember_joined_server,
        reset_restart_time,
        server_game_type,
        server_port,
        set_info_company,
        set_join_info,
        set_network_server,
        set_networking,
        set_server_info_key,
        udp_initialize,
        udp_listen,
        update_client_info,
        update_game_info,
        validate_client_name,
    };
}

pub(crate) fn close(s: &Services, f: &FrameServices, close_admins: bool, work: Actions) {
    work.network(f);
    if (f.command.company.network_server)() {
        if close_admins {
            (s.admins_close)();
        }
        close_clients(s, f, close_admins, 0, work);
        return;
    }
    close_client(s, f);
    close_end(s, f, close_admins);
}

pub(crate) fn close_clients(
    s: &Services,
    f: &FrameServices,
    close_admins: bool,
    mut cursor: usize,
    work: Actions,
) {
    while let Some(client) = next_client(f.command, &mut cursor) {
        crate::network_connection::server_close(
            f.connection,
            f,
            client.socket,
            NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
            work,
        );
        if work.pending() {
            work.push(Resume::CloseClients(cursor, close_admins));
            return;
        }
    }
    (s.close_client_listeners)();
    (s.close_admin_listeners)();
    (s.coordinator_close_server)();
    close_end(s, f, close_admins);
}

fn close_client(s: &Services, f: &FrameServices) {
    if !(f.client_game)().is_null() {
        crate::network_client::send_quit(f);
        crate::network_client::close_connection(
            f.client,
            f,
            (f.client_game)(),
            NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT,
        );
    }
    (s.coordinator_close_client)();
}

fn close_end(s: &Services, f: &FrameServices, close_admins: bool) {
    socket::process_deferred(f);
    (s.kill_connecters)();
    (s.set_networking)(false);
    (s.set_network_server)(false);
    crate::network_command::free_local_command_queue();
    (s.initialize_pools)(close_admins);
}

fn initialize(s: &Services, f: &FrameServices, close_admins: bool) {
    (s.initialize_pools)(close_admins);
    crate::network_frame::initialize();
    unsafe { f.client.reconnect.write(0) };
}

pub(crate) fn disconnect(s: &Services, f: &FrameServices, close_admins: bool, work: Actions) {
    if (f.command.company.network_server)() {
        disconnect_send(s, f, close_admins, 0, work);
    } else {
        disconnect_close(s, f, close_admins, work);
    }
}

pub(crate) fn disconnect_send(
    s: &Services,
    f: &FrameServices,
    close_admins: bool,
    mut cursor: usize,
    work: Actions,
) {
    while let Some(client) = next_client(f.command, &mut cursor) {
        crate::network_connection::send_shutdown(f, client.socket);
        socket::send(f, client.socket, work);
        if work.pending() {
            work.push(Resume::DisconnectSend(cursor, close_admins));
            return;
        }
    }
    if close_admins {
        (s.admins_shutdown)();
    }
    disconnect_close(s, f, close_admins, work);
}

fn disconnect_close(s: &Services, f: &FrameServices, close_admins: bool, work: Actions) {
    (f.command.company.close_network_status)();
    close(s, f, close_admins, work);
    if work.pending() {
        work.push(Resume::DisconnectEnd);
        return;
    }
    disconnect_end(s);
}

pub(crate) fn disconnect_end(s: &Services) {
    (s.udp_initialize)();
}

pub(crate) fn reboot(s: &Services, f: &FrameServices, session: &SessionServices, work: Actions) {
    if (f.command.company.network_server)() {
        reboot_send(s, f, session, 0, work);
    } else {
        close(s, f, !(unsafe { &*(session.world)() }.dedicated)(), work);
    }
}

pub(crate) fn reboot_send(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    mut cursor: usize,
    work: Actions,
) {
    while let Some(client) = next_client(f.command, &mut cursor) {
        crate::network_connection::send_new_game(f, client.socket);
        socket::send(f, client.socket, work);
        if work.pending() {
            work.push(Resume::RebootSend(cursor));
            return;
        }
    }
    (s.admins_newgame)();
    close(s, f, !(unsafe { &*(session.world)() }.dedicated)(), work);
}

pub(crate) fn join_game(s: &Services, f: &FrameServices, work: Actions) {
    disconnect(s, f, true, work);
    if work.pending() {
        work.push(Resume::JoinEnd);
        return;
    }
    join_end(s, f);
}

pub(crate) fn join_end(s: &Services, f: &FrameServices) {
    initialize(s, f, true);
    (s.remember_joined_server)((s.join_connection)());
    debug!(net, 9, "status = CONNECTING");
    unsafe {
        f.client
            .join_status
            .write(crate::network_internal::NetworkJoinStatus::NETWORK_JOIN_STATUS_CONNECTING.0);
    };
    (f.client.show_join_window)();
    (s.create_client_connecter)();
}

#[allow(clippy::too_many_arguments)]
fn connect_game(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    connection: ByteSpan,
    mut company: u8,
    password: *const c_void,
    resolved: *mut c_void,
    work: Actions,
) -> bool {
    debug!(
        net,
        9,
        "NetworkClientConnectGame(): connection_string={}",
        Bytes(unsafe { connection.bytes() })
    );
    (s.parse_connection)(connection, &raw mut company, resolved);
    if !(session.network_available)() || !(s.validate_client_name)() {
        return false;
    }
    (s.set_join_info)(resolved, company, password);
    if crate::session::mode() == GameMode::MENU {
        join_game(s, f, work);
    } else {
        crate::session::set_pending(SwitchMode::JOIN_GAME);
    }
    true
}

fn init_game_info(s: &Services, f: &FrameServices, session: &SessionServices) {
    (s.fill_game_info)();
    unsafe {
        f.connection
            .clients_on
            .write(u8::from(!((*(session.world)()).dedicated)()));
    };
    let info = (s.allocate_server_info)();
    (s.auth_client_key)();
    (s.set_server_info_key)(info);
}

pub(crate) fn server_start(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    frame: *mut crate::session::Frame,
    work: Actions,
) -> Result<bool, ()> {
    if !(session.network_available)() {
        return Ok(false);
    }
    if !(s.pre_server)(frame) {
        return Err(());
    }
    if (unsafe { &*(session.world)() }.dedicated)() && !(s.pre_dedicated)(frame) {
        return Err(());
    }
    if (unsafe { &*(session.world)() }.dedicated)() {
        (s.check_names)();
    }
    disconnect(s, f, false, work);
    if work.pending() {
        work.push(Resume::ServerStart);
        return Err(());
    }
    server_start_after_close(s, f, session, frame)
}

pub(crate) fn server_start_after_close(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    frame: *mut crate::session::Frame,
) -> Result<bool, ()> {
    initialize(s, f, false);
    (s.udp_initialize)();
    debug!(net, 5, "Starting listeners for clients");
    if !(s.listen_clients)((s.server_port)()) {
        return Ok(false);
    }
    if (s.admin_auth_configured)() {
        debug!(net, 5, "Starting listeners for admins");
        if !(s.listen_admins)((s.admin_port)()) {
            return Ok(false);
        }
    }
    debug!(net, 5, "Starting listeners for incoming server queries");
    (s.udp_listen)();
    (s.set_network_server)(true);
    (s.set_networking)(true);
    crate::network_frame::server_started();
    unsafe {
        f.client.own_client_id.write(CLIENT_ID_SERVER);
        f.connection.clients_connected.write(0);
    }
    init_game_info(s, f, session);
    if (s.server_game_type)() != SERVER_GAME_TYPE_LOCAL {
        (s.coordinator_register)();
    }
    if !(s.on_server)(frame) {
        return Err(());
    }
    if (unsafe { &*(session.world)() }.dedicated)() && !(s.on_dedicated)(frame) {
        return Err(());
    }
    Ok(true)
}

pub(crate) fn on_game_start(s: &Services, f: &FrameServices, session: &SessionServices) {
    if !(f.command.company.network_server)() {
        return;
    }
    (s.update_game_info)();
    (s.reset_restart_time)();
    if (unsafe { &*(session.world)() }.dedicated)() {
        (s.admins_welcome)();
    } else {
        let company_services = f.command.company;
        let id = crate::company::first_playable(company_services);
        let company = if (company_services.shared.company_valid)(id.0) {
            (company_services.identity.company)(id)
        } else {
            std::ptr::null_mut()
        };
        let info = (f.command.company.shared.network_client_info)(CLIENT_ID_SERVER);
        if !company.is_null() && !info.is_null() {
            (s.set_info_company)(info, (company_services.identity.company_id)(company).0);
            if (company_services.identity.name_id)(company)
                == company_services.identity.strings.unnamed
                && (company_services.identity.name)(company).size == 0
            {
                new_company(s, f, company, info);
            }
        }
        (f.client.show_client_list)();
    }
}

pub(crate) fn reconnect_due(f: &FrameServices) -> bool {
    let count = unsafe { f.client.reconnect.read() };
    if count == 0 {
        return false;
    }
    let next = count.wrapping_sub(1);
    unsafe { f.client.reconnect.write(next) };
    next == 0
}

pub(crate) fn reconnect(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    frame: *mut crate::session::Frame,
    work: Actions,
) {
    let password = (s.connect_text_begin)(frame, false);
    let resolved = (s.connect_text_begin)(frame, true);
    connect_game(
        s,
        f,
        session,
        (s.last_joined)(),
        crate::company_type::COMPANY_SPECTATOR,
        password,
        resolved,
        work,
    );
    if work.pending() {
        work.push(Resume::ReconnectEnd);
        return;
    }
    reconnect_end(s, frame);
}

pub(crate) fn reconnect_end(s: &Services, frame: *mut crate::session::Frame) {
    (s.connect_text_end)(frame);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_initialize(
    s: &Services,
    f: &FrameServices,
    close_admins: bool,
) {
    initialize(s, f, close_admins);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_disconnect(
    s: &Services,
    f: &FrameServices,
    close_admins: bool,
    work: Actions,
) {
    work.network(f);
    disconnect(s, f, close_admins, work);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_connect_game(
    s: &Services,
    f: &FrameServices,
    session: &SessionServices,
    connection: ByteSpan,
    company: u8,
    password: *const c_void,
    resolved: *mut c_void,
    work: Actions,
) -> u8 {
    work.network(f);
    u8::from(connect_game(
        s, f, session, connection, company, password, resolved, work,
    ))
}

/// `NetworkServerNewCompany`: info publication precedes the two ordered command sends.
pub(crate) fn new_company(
    s: &Services,
    f: &FrameServices,
    company: *mut crate::company_type::OpenTTDCompany,
    info: *mut c_void,
) {
    crate::network_frame::packet_check(f)(!company.is_null());
    if !(f.command.company.network_server)() || info.is_null() {
        return;
    }
    let id = (f.command.company.identity.company_id)(company).0;
    (s.set_info_company)(info, id);
    (s.update_client_info)(info);
    let allow = (s.pack_company_allow)(info);
    crate::network_command::send_command(
        f.command,
        crate::command_type::Commands::CMD_COMPANY_ALLOW_LIST_CTRL,
        crate::strings_type::StringID::STR_NULL,
        std::ptr::null(),
        crate::company_type::CompanyID(id),
        allow.span(),
    );
    drop(allow);
    let president = (s.pack_president)(info);
    crate::network_command::send_command(
        f.command,
        crate::command_type::Commands::CMD_RENAME_PRESIDENT,
        crate::strings_type::StringID::STR_NULL,
        std::ptr::null(),
        crate::company_type::CompanyID(id),
        president.span(),
    );
    drop(president);
    (s.announce_company)(info, id);
}

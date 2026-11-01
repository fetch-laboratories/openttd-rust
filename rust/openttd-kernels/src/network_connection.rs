/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Connection admission and map pacing. Socket and client-info pools and save filters stay
//! native. Fields are accessed at their original observation points; no borrow survives a leaf.
#![allow(unsafe_code, missing_docs)]
use crate::network_command::next_client;
use crate::network_frame::{
    Services as FrameServices, SocketSync,
    server_status::{
        ACTIVE, AUTHORIZED, DONE_MAP, IDENTIFY, MAP, MAP_WAIT, NEWGRFS_CHECK, PRE_ACTIVE,
    },
};
use crate::network_type::{
    NETWORK_ERROR_NOT_AUTHORIZED, NETWORK_ERROR_NOT_EXPECTED, NETWORK_RECV_STATUS_MALFORMED_PACKET,
    NETWORK_RECV_STATUS_OKAY,
};
use crate::openttd::PauseMode;
use crate::raw_field::write_field;
use crate::tcp_game::{
    PACKET_SERVER_JOIN, PACKET_SERVER_MAP_BEGIN, PACKET_SERVER_WAIT, PACKET_SERVER_WELCOME,
};
use std::ffi::c_void;

#[repr(C)]
pub struct Services {
    pub admin_joined: extern "C" fn(*mut c_void),
    pub client_company: extern "C" fn(*mut c_void) -> u8,
    pub client_id: extern "C" fn(*mut c_void) -> u32,
    pub clients_on: *mut u8,
    pub close: extern "C" fn(*mut c_void, u8) -> u8,
    pub debug_authorized: extern "C" fn(*mut c_void),
    pub debug_done_map: extern "C" fn(*mut c_void),
    pub debug_game_info: extern "C" fn(*mut c_void),
    pub debug_getmap: extern "C" fn(*mut c_void),
    pub debug_map: extern "C" fn(*mut c_void),
    pub debug_map_first: extern "C" fn(*mut c_void),
    pub debug_map_last: extern "C" fn(*mut c_void),
    pub debug_map_ok: extern "C" fn(*mut c_void),
    pub debug_map_wait: extern "C" fn(*mut c_void),
    pub debug_newgrf_check: extern "C" fn(*mut c_void),
    pub debug_newgrfs: extern "C" fn(*mut c_void),
    pub debug_newgrfs_checked: extern "C" fn(*mut c_void),
    pub debug_next_map: extern "C" fn(*mut c_void),
    pub debug_pre_active: extern "C" fn(*mut c_void),
    pub debug_send_join: extern "C" fn(*mut c_void, u32),
    pub debug_send_wait: extern "C" fn(*mut c_void),
    pub debug_send_welcome: extern "C" fn(*mut c_void),
    pub game: extern "C" fn(*mut c_void) -> *mut c_void,
    pub grfs_empty: extern "C" fn() -> bool,
    pub grfs_packet: extern "C" fn(*mut c_void) -> *mut c_void,
    pub has_quit: extern "C" fn(*mut c_void) -> bool,
    pub info: extern "C" fn(*mut c_void) -> *mut c_void,
    pub join_date: extern "C" fn(*mut c_void) -> i32,
    pub joined: extern "C" fn(*mut c_void),
    pub map_create: extern "C" fn(*mut c_void),
    pub map_destroy: extern "C" fn(*mut c_void),
    pub map_save: extern "C" fn(*mut c_void),
    pub map_transfer: extern "C" fn(*mut c_void) -> bool,
    pub min_active_clients: extern "C" fn() -> u8,
    pub outgoing_queue: extern "C" fn(*mut c_void) -> *mut crate::network_command::CommandQueue,
    pub pause_on_join: extern "C" fn() -> bool,
    pub send_client_info: extern "C" fn(*mut c_void, *mut c_void) -> u8,
    pub send_config: extern "C" fn(*mut c_void) -> u8,
    pub send_game_info: extern "C" fn(*mut c_void) -> u8,
    pub server_info: extern "C" fn() -> *mut c_void,
    pub status: extern "C" fn(*mut c_void) -> *mut u8,
    pub sync: extern "C" fn(*mut c_void) -> *mut SocketSync,
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkConnectionServices" ["probe"] => Services {
        admin_joined,
        client_company,
        client_id,
        clients_on,
        close,
        debug_authorized,
        debug_done_map,
        debug_game_info,
        debug_getmap,
        debug_map,
        debug_map_first,
        debug_map_last,
        debug_map_ok,
        debug_map_wait,
        debug_newgrf_check,
        debug_newgrfs,
        debug_newgrfs_checked,
        debug_next_map,
        debug_pre_active,
        debug_send_join,
        debug_send_wait,
        debug_send_welcome,
        game,
        grfs_empty,
        grfs_packet,
        has_quit,
        info,
        join_date,
        joined,
        map_create,
        map_destroy,
        map_save,
        map_transfer,
        min_active_clients,
        outgoing_queue,
        pause_on_join,
        send_client_info,
        send_config,
        send_game_info,
        server_info,
        status,
        sync,
    };
}

// Native sockets and their fields remain live until their original deferred-deletion point.
fn status(s: &Services, socket: *mut c_void) -> u8 {
    unsafe { (s.status)(socket).read() }
}
fn set_status(s: &Services, socket: *mut c_void, value: u8) {
    unsafe { (s.status)(socket).write(value) };
}
fn reset_lag(s: &Services, socket: *mut c_void) {
    let sync = (s.sync)(socket);
    write_field!(sync, last_frame, crate::network_frame::frame_counter());
    write_field!(
        sync,
        last_frame_server,
        crate::network_frame::frame_counter()
    );
}

pub(crate) fn check_pause_on_join(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause = crate::session::pause();
    if pause & PauseMode::ERROR.bit() != 0
        || (!(s.pause_on_join)() && pause & PauseMode::JOIN.bit() == 0)
    {
        return;
    }
    let mut cursor = 0;
    let mut joining = false;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status >= AUTHORIZED && client.status < ACTIVE {
            joining = true;
            break;
        }
    }
    if joining != (pause & PauseMode::JOIN.bit() != 0) {
        (session.post_pause)(PauseMode::JOIN, joining);
    }
}

pub(crate) fn check_min_active_clients(session: &crate::session::Services, f: &FrameServices) {
    let s = f.connection;
    let pause = crate::session::pause();
    if pause & PauseMode::ERROR.bit() != 0
        || !(session.dedicated)()
        || ((s.min_active_clients)() == 0 && pause & PauseMode::ACTIVE_CLIENTS.bit() == 0)
    {
        return;
    }
    let mut count = 0_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status != ACTIVE {
            continue;
        }
        if !(f.command.company.shared.company_valid)((s.client_company)(client.socket)) {
            continue;
        }
        count += 1;
    }
    let required = count < u32::from((s.min_active_clients)());
    if required != (pause & PauseMode::ACTIVE_CLIENTS.bit() != 0) {
        (session.post_pause)(PauseMode::ACTIVE_CLIENTS, required);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_welcome(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    (s.debug_send_welcome)(socket);
    if status(s, socket) != NEWGRFS_CHECK {
        return (s.close)(socket, NETWORK_RECV_STATUS_MALFORMED_PACKET);
    }
    (s.debug_authorized)(socket);
    set_status(s, socket, AUTHORIZED);
    reset_lag(s, socket);
    unsafe { s.clients_on.write(s.clients_on.read().wrapping_add(1)) };
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_WELCOME); // PACKET_SERVER_WELCOME
    (f.packet_send_uint32)(p, (s.client_id)(socket));
    (f.packet_send)(game, p);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket != socket && client.status >= AUTHORIZED {
            (s.send_client_info)(socket, (s.info)(client.socket));
        }
    }
    (s.send_client_info)(socket, (s.server_info)())
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_wait(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    (s.debug_send_wait)(socket);
    let mut waiting = 1_u32;
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status != MAP_WAIT {
            continue;
        }
        if (s.join_date)(client.socket) < (s.join_date)(socket)
            || ((s.join_date)(client.socket) == (s.join_date)(socket)
                && (s.client_id)(client.socket) < (s.client_id)(socket))
        {
            waiting += 1;
        }
    }
    let game = (s.game)(socket);
    let p = (f.packet_new)(game, PACKET_SERVER_WAIT); // PACKET_SERVER_WAIT
    #[allow(clippy::cast_possible_truncation)]
    (f.packet_send_uint8)(p, waiting as u8);
    (f.packet_send)(game, p);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_next_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
    ignore: *mut c_void,
) {
    (s.debug_next_map)(socket);
    let mut best: *mut c_void = std::ptr::null_mut();
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.socket == ignore {
            continue;
        }
        if client.status == MAP_WAIT
            && (best.is_null()
                || (s.join_date)(best) > (s.join_date)(client.socket)
                || ((s.join_date)(best) == (s.join_date)(client.socket)
                    && (s.client_id)(best) > (s.client_id)(client.socket)))
        {
            best = client.socket;
        }
    }
    if !best.is_null() {
        set_status(s, best, AUTHORIZED);
        openttd_rust_network_send_map(s, f, best);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status == MAP_WAIT {
                openttd_rust_network_send_wait(s, f, client.socket);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_map(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) < AUTHORIZED {
        return (f.send_error)(socket, NETWORK_ERROR_NOT_AUTHORIZED);
    }
    if status(s, socket) == AUTHORIZED {
        (s.debug_map_first)(socket);
        (s.map_create)(socket);
        let game = (s.game)(socket);
        let p = (f.packet_new)(game, PACKET_SERVER_MAP_BEGIN); // PACKET_SERVER_MAP_BEGIN
        (f.packet_send_uint32)(p, crate::network_frame::frame_counter());
        (f.packet_send)(game, p);
        unsafe {
            crate::network_command::sync_command_queue((s.outgoing_queue)(socket));
        };
        (s.debug_map)(socket);
        set_status(s, socket, MAP);
        reset_lag(s, socket);
        (s.map_save)(socket);
    }
    if status(s, socket) == MAP && (s.map_transfer)(socket) {
        (s.debug_map_last)(socket);
        (s.map_destroy)(socket);
        (s.debug_done_map)(socket);
        set_status(s, socket, DONE_MAP);
        openttd_rust_network_next_map(s, f, socket, std::ptr::null_mut());
    }
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_getmap(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) < AUTHORIZED || (s.has_quit)(socket) {
        return (f.send_error)(socket, NETWORK_ERROR_NOT_AUTHORIZED);
    }
    (s.debug_getmap)(socket);
    let mut cursor = 0;
    while let Some(client) = next_client(f.command, &mut cursor) {
        if client.status == MAP {
            (s.debug_map_wait)(socket);
            set_status(s, socket, MAP_WAIT);
            return openttd_rust_network_send_wait(s, f, socket);
        }
    }
    openttd_rust_network_send_map(s, f, socket)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_map_ok(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) == DONE_MAP && !(s.has_quit)(socket) {
        (s.debug_map_ok)(socket);
        (s.joined)(socket);
        (s.debug_pre_active)(socket);
        set_status(s, socket, PRE_ACTIVE);
        unsafe {
            crate::network_command::handle_command_queue(
                f.command,
                socket,
                (s.outgoing_queue)(socket),
            );
        };
        unsafe {
            crate::network_frame::openttd_rust_network_send_frame(
                f,
                (s.game)(socket),
                (s.sync)(socket),
            )
        };
        crate::network_frame::openttd_rust_network_send_sync(f, (s.game)(socket));
        reset_lag(s, socket);
        let mut cursor = 0;
        while let Some(client) = next_client(f.command, &mut cursor) {
            if client.status >= AUTHORIZED {
                (s.send_client_info)(client.socket, (s.info)(socket));
                (s.debug_send_join)(client.socket, (s.client_id)(socket));
                // PACKET_SERVER_JOIN
                let p = (f.packet_new)(client.game, PACKET_SERVER_JOIN);
                (f.packet_send_uint32)(p, (s.client_id)(socket));
                (f.packet_send)(client.game, p);
            }
        }
        (s.admin_joined)(socket);
        return (s.send_config)(socket);
    }
    (f.send_error)(socket, NETWORK_ERROR_NOT_EXPECTED)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_newgrf_check(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    (s.debug_newgrf_check)(socket);
    if status(s, socket) != IDENTIFY {
        return (s.close)(socket, NETWORK_RECV_STATUS_MALFORMED_PACKET);
    }
    (s.debug_newgrfs)(socket);
    set_status(s, socket, NEWGRFS_CHECK);
    if (s.grfs_empty)() {
        return openttd_rust_network_send_welcome(s, f, socket);
    }
    let p = (s.grfs_packet)(socket);
    (f.packet_send)((s.game)(socket), p);
    NETWORK_RECV_STATUS_OKAY
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_newgrfs_checked(
    s: &Services,
    f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    if status(s, socket) != NEWGRFS_CHECK {
        return (f.send_error)(socket, NETWORK_ERROR_NOT_EXPECTED);
    }
    (s.debug_newgrfs_checked)(socket);
    openttd_rust_network_send_welcome(s, f, socket)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_receive_game_info(
    s: &Services,
    _f: &FrameServices,
    socket: *mut c_void,
) -> u8 {
    (s.debug_game_info)(socket);
    (s.send_game_info)(socket)
}

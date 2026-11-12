/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Call-local map-save actions. Native callers save after Rust returns, then resume the
//! interrupted map, close, tick or session entry. No borrowed game state crosses a save.
#![allow(unsafe_code, missing_docs)]
use crate::network_frame::Services as Network;
use crate::network_type::NetworkRecvStatus;
use crate::openttd::{GameMode, SwitchMode};
use crate::session::{Frame, Services as Session};
use std::ffi::c_void;

crate::abi::shared_records! {
    fn abi_record;
    /// Native stack capacity for the Rust continuation owner; its contents stay opaque.
    "OpenTTDNetworkActionsStorage" ["probe"] => pub struct Storage {
        pub words: [u64; 64] => "std::array<uint64_t, 64>",
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) enum Resume {
    None,
    MapTransfer(*mut crate::network_socket::GameSocket),
    Waits(usize),
    CloseTail(*mut crate::network_socket::GameSocket, NetworkRecvStatus),
    CloseClients(usize, bool),
    DisconnectSend(usize, bool),
    RebootSend(usize),
    DisconnectEnd,
    JoinEnd,
    ServerStart,
    ReconnectEnd,
    SwitchNetwork(SwitchMode),
    SwitchStartup(SwitchMode),
    LoadRestore(GameMode),
    JoiningSocial,
    SwitchLoad(GameMode),
    MapLoaded(*mut crate::network_socket::GameSocket),
    LoopSwitch,
    LoopState,
    LoopEnd,
    FrameReceived,
    ReceiveClients(usize),
    ClientReceived,
    FrameSend,
    SendClients(usize),
    ReceivedPacket(*mut crate::network_socket::GameSocket),
    ReceiveEnd,
    ServerTick(usize, bool, bool),
}

// Coalesced waits and the fixed session caller chain use fewer than 16 slots.
// Reentrant native entries have separate frames.
struct Owner {
    network: *const Network,
    session: *const Session,
    frame: *mut Frame,
    save: *mut crate::network_socket::GameSocket,
    packet: *mut crate::network_tcp::Packet,
    /// The driver's result byte: a `NetworkRecvStatus` after packet handling and map loads, and
    /// `NetworkReceive`'s boolean after a receive.
    result: u8,
    queue: [Resume; 16],
    len: usize,
    insert: usize,
}

/// Call-local owner pointer; field borrows end before a native or Rust continuation call.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Actions(*mut Owner);

const _: () = assert!(std::mem::size_of::<Owner>() <= std::mem::size_of::<Storage>());
const _: () = assert!(std::mem::align_of::<Owner>() <= std::mem::align_of::<Storage>());

impl Actions {
    pub(crate) unsafe fn from_storage(storage: *mut Storage) -> Self {
        Self(storage.cast())
    }
    /// Initializes an empty owner, with no services bound, in unused native storage.
    unsafe fn init(storage: *mut Storage) {
        unsafe {
            storage.cast::<Owner>().write(Owner {
                network: std::ptr::null(),
                session: std::ptr::null(),
                frame: std::ptr::null_mut(),
                save: std::ptr::null_mut(),
                packet: std::ptr::null_mut(),
                result: 0,
                queue: [Resume::None; 16],
                len: 0,
                insert: 0,
            });
        }
    }
    /// The native server shell whose map save is pending, or null when none is.
    fn save_shell(self) -> *mut c_void {
        let socket = unsafe { (*self.0).save };
        if socket.is_null() {
            std::ptr::null_mut()
        } else {
            crate::network_socket::server(socket)
        }
    }
    /// Stores the `NetworkRecvStatus` the native caller returns.
    pub(crate) fn set_status(self, status: NetworkRecvStatus) {
        unsafe {
            (*self.0).result = status.0;
        }
    }
    /// The stored `NetworkRecvStatus`.
    pub(crate) fn status(self) -> NetworkRecvStatus {
        NetworkRecvStatus(unsafe { (*self.0).result })
    }
    /// Stores `NetworkReceive`'s result.
    pub(crate) fn set_received(self, received: bool) {
        unsafe {
            (*self.0).result = u8::from(received);
        }
    }
    /// The stored `NetworkReceive` result.
    pub(crate) fn received(self) -> bool {
        unsafe { (*self.0).result != 0 }
    }
    pub(crate) fn hold_packet(self, packet: *mut crate::network_tcp::Packet) {
        unsafe {
            (*self.0).packet = packet;
        }
    }
    pub(crate) fn drop_packet(self) {
        let packet = unsafe { std::mem::replace(&mut (*self.0).packet, std::ptr::null_mut()) };
        if !packet.is_null() {
            let network = unsafe { &*(*self.0).network };
            crate::network_tcp::destroy_packet(network.tcp, packet);
        }
    }
    pub(crate) fn network(self, network: &Network) {
        unsafe {
            (*self.0).network = network;
        }
    }
    pub(crate) fn bind(self, network: &Network, session: *const Session, frame: *mut Frame) {
        unsafe {
            (*self.0).network = network;
            (*self.0).session = session;
            (*self.0).frame = frame;
        }
    }
    pub(crate) fn pending(self) -> bool {
        unsafe { !(*self.0).save.is_null() }
    }
    pub(crate) fn save(self, socket: *mut crate::network_socket::GameSocket) {
        unsafe {
            (*self.0).save = socket;
        }
        self.push(Resume::MapTransfer(socket));
    }
    pub(crate) fn push(self, resume: Resume) {
        if !self.pending() {
            return;
        }
        unsafe {
            let owner = &mut *self.0;
            if let Resume::Waits(count) = resume
                && let Some(Resume::Waits(previous)) = owner.queue.get_mut(owner.insert)
            {
                *previous += count;
                owner.insert += 1;
                return;
            }
            owner
                .queue
                .copy_within(owner.insert..owner.len, owner.insert + 1);
            owner.queue[owner.insert] = resume;
            owner.len += 1;
            owner.insert += 1;
        }
    }
    fn pop(self) -> Resume {
        unsafe {
            let owner = &mut *self.0;
            let resume = owner.queue[0];
            owner.len -= 1;
            owner.queue.copy_within(1..=owner.len, 0);
            owner.queue[owner.len] = Resume::None;
            owner.insert = 0;
            resume
        }
    }
    #[allow(clippy::too_many_lines)] // One branch per native-frame continuation.
    unsafe fn resume(self) {
        unsafe {
            (*self.0).save = std::ptr::null_mut();
        }
        // SAFETY: Service tables and the native frame live until its action driver returns.
        let (network, session, frame) = unsafe {
            (
                &*(*self.0).network,
                (*self.0).session.as_ref(),
                (*self.0).frame,
            )
        };
        while unsafe { (*self.0).len } != 0 && !self.pending() {
            let next = self.pop();
            match next {
                Resume::None => (),
                Resume::MapTransfer(socket) => {
                    crate::network_connection::map_transfer(network, socket, self);
                }
                Resume::Waits(count) => crate::network_connection::update_waiters(network, count),
                Resume::CloseTail(socket, status) => {
                    crate::network_connection::close_tail(
                        network.connection,
                        network,
                        socket,
                        status,
                    );
                }
                Resume::CloseClients(cursor, admins) => {
                    crate::network_lifecycle::close_clients(
                        network.lifecycle,
                        network,
                        admins,
                        cursor,
                        self,
                    );
                }
                Resume::DisconnectSend(cursor, admins) => {
                    crate::network_lifecycle::disconnect_send(
                        network.lifecycle,
                        network,
                        admins,
                        cursor,
                        self,
                    );
                }
                Resume::RebootSend(cursor) => crate::network_lifecycle::reboot_send(
                    network.lifecycle,
                    network,
                    session.unwrap(),
                    cursor,
                    self,
                ),
                Resume::DisconnectEnd => {
                    crate::network_lifecycle::disconnect_end(network.lifecycle);
                }
                Resume::JoinEnd => crate::network_lifecycle::join_end(network.lifecycle, network),
                Resume::ServerStart => {
                    let _ = crate::network_lifecycle::server_start_after_close(
                        network.lifecycle,
                        network,
                        session.unwrap(),
                        frame,
                    );
                }
                Resume::ReconnectEnd => {
                    crate::network_lifecycle::reconnect_end(network.lifecycle, frame);
                }
                Resume::SwitchNetwork(mode) => {
                    let _ =
                        crate::session::resume_switch_network(session.unwrap(), frame, mode, self);
                }
                Resume::SwitchStartup(mode) => {
                    let _ = crate::session::resume_switch_startup(session.unwrap(), frame, mode);
                }
                Resume::LoadRestore(mode) => {
                    let _ = crate::session::resume_load_restore(session.unwrap(), frame, mode);
                }
                Resume::JoiningSocial => (session.unwrap().joining_social)(),
                Resume::SwitchLoad(mode) => {
                    crate::session::resume_switch_load(session.unwrap(), mode);
                }
                Resume::MapLoaded(game) => {
                    let status = crate::network_client::map_loaded(
                        network.client,
                        network,
                        session.unwrap(),
                        game,
                        false,
                    );
                    self.set_status(status);
                }
                Resume::LoopSwitch => {
                    let _ = crate::session::resume_loop_switch(session.unwrap(), frame, self);
                }
                Resume::LoopState => {
                    let _ = crate::session::resume_loop_state(session.unwrap(), frame);
                }
                Resume::LoopEnd => crate::session::resume_loop_end(session.unwrap()),
                Resume::FrameReceived => {
                    let _ = crate::network_frame::game_loop_received(
                        session.unwrap(),
                        network,
                        frame,
                        self.received(),
                        self,
                    );
                }
                Resume::ReceiveClients(cursor) => {
                    let _ = crate::network_socket::receive_clients(
                        session.unwrap(),
                        network,
                        frame,
                        cursor,
                        self,
                    );
                }
                Resume::ClientReceived => {
                    let _ = crate::network_socket::client_received(network, frame, self);
                }
                Resume::FrameSend => {
                    let _ = crate::network_frame::send(network, frame, self);
                }
                Resume::ReceivedPacket(game) => {
                    let _ = crate::network_dispatch::received_packet(
                        session.unwrap(),
                        network,
                        game,
                        frame,
                        self,
                    );
                }
                Resume::ReceiveEnd => self.drop_packet(),
                Resume::SendClients(cursor) => {
                    crate::network_socket::send_clients(network, cursor, self);
                }
                Resume::ServerTick(cursor, send_frame, send_sync) => {
                    crate::network_frame::server_tick_clients(
                        network, send_frame, send_sync, cursor, self,
                    );
                }
            }
            if let Some(session) = session
                && (session.failed)(frame)
            {
                if unsafe { !(*self.0).packet.is_null() } {
                    (network.dispatch.packet_end)(frame);
                    self.drop_packet();
                }
                unsafe {
                    (*self.0).len = 0;
                }
                unsafe {
                    (*self.0).save = std::ptr::null_mut();
                }
            }
        }
    }
}

/// # Safety
/// `storage` is aligned native stack capacity, unused until this initialization completes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_actions_init(storage: *mut Storage) {
    unsafe { Actions::init(storage) };
}
/// # Safety
/// `storage` is a live initialized native frame's action owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_actions_save(storage: *mut Storage) -> *mut c_void {
    unsafe { Actions::from_storage(storage) }.save_shell()
}
/// # Safety
/// The requested native save completed and all entry arguments and services remain live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_actions_resume(storage: *mut Storage) {
    unsafe {
        Actions::from_storage(storage).resume();
    }
}

/// # Safety
/// The native frame owns initialized action storage and is ending its lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_actions_drop(storage: *mut Storage) {
    unsafe {
        Actions::from_storage(storage).drop_packet();
    }
}
/// # Safety
/// `storage` is a live native frame's initialized action owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_network_actions_result(storage: *const Storage) -> u8 {
    // The result byte is the ABI form of both result kinds.
    unsafe { Actions::from_storage(storage.cast_mut()) }
        .status()
        .0
}

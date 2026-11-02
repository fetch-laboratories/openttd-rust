/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical game-socket protocol fields. Native shells install stable identity and resource
//! links at construction. They live until deferred destruction; raw field access ends before
//! callbacks or deletion. The game-state mutex serializes visits and pool mutation.
#![allow(unsafe_code, missing_docs)]
use crate::network_command::CommandQueue;
use crate::network_frame::SocketSync;
use crate::network_tcp::NetworkTCPSocketHandler;
use crate::packet::NetworkSocketHandler;
use crate::raw_field::{read_field, write_field};
use std::ffi::c_void;
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDNetworkGameSocket" ["probe"] => pub struct GameSocket {
        pub base: *mut NetworkSocketHandler => class,
        pub game: *mut c_void,
        pub incoming: *mut CommandQueue,
        pub last_packet_ns: i64,
        pub outgoing: *mut CommandQueue,
        pub pending: bool,
        pub role: u8,
        pub server: *mut c_void,
        pub status: u8,
        pub sync: *mut SocketSync,
        pub tcp: *mut NetworkTCPSocketHandler => class,
    }
}
pub(crate) fn shell(socket: *mut GameSocket) -> *mut c_void {
    read_field!(socket, game)
}
pub(crate) fn server(socket: *mut GameSocket) -> *mut c_void {
    read_field!(socket, server)
}
pub(crate) fn base(socket: *mut GameSocket) -> *mut NetworkSocketHandler {
    read_field!(socket, base)
}
pub(crate) fn tcp(socket: *mut GameSocket) -> *mut NetworkTCPSocketHandler {
    read_field!(socket, tcp)
}
pub(crate) fn sync(socket: *mut GameSocket) -> *mut SocketSync {
    read_field!(socket, sync)
}
pub(crate) fn incoming(socket: *mut GameSocket) -> *mut CommandQueue {
    read_field!(socket, incoming)
}
pub(crate) fn outgoing(socket: *mut GameSocket) -> *mut CommandQueue {
    read_field!(socket, outgoing)
}
pub(crate) fn status(socket: *mut GameSocket) -> u8 {
    read_field!(socket, status)
}
pub(crate) fn pending(socket: *mut GameSocket) -> bool {
    read_field!(socket, pending)
}
pub(crate) fn last_packet(socket: *mut GameSocket) -> i64 {
    read_field!(socket, last_packet_ns)
}
pub(crate) fn set_last_packet(socket: *mut GameSocket, time: i64) {
    write_field!(socket, last_packet_ns, time);
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `network_command.cpp` command queues with their send, distribute, sync and execute decisions.
//! Command arguments stay the encoded bytes of the native `CommandTraits` templates, which alone
//! decode them through typed leaves. Queues and records have C layout: the local wait and
//! execution queues are process-wide state, and each socket shell holds its own queues in place.
//! No queue borrow survives a native leaf; entries address queued packets by raw pointer.
#![allow(unsafe_code, missing_docs)]

use crate::command_type::{CommandFlag, CommandFlags, Commands};
use crate::company_type::CompanyID;
use crate::debug::debug;
use crate::game_state::GameState;
use crate::network_server::{self as server, ClientStatus};
use crate::network_socket as socket;
use crate::network_type::{NetworkErrorCode, NetworkRecvStatus};
use crate::openttd::PauseModes;
use crate::services::ByteSpan;
use crate::strings_type::StringID;
use crate::tcp_game::PacketGameType;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    /// `CommandDataBuffer` bytes as the raw parts of a Rust `Vec<u8>`; null when never allocated.
    "OpenTTDCommandData" ["probe"] => pub struct CommandData {
        pub data: *mut u8,
        pub size: usize,
        pub capacity: usize,
    }
    /// `CommandPacket`.
    #[derive(Clone)]
    "OpenTTDCommandPacket" ["probe"] => pub struct CommandPacket {
        /// command parameters.
        pub data: CommandData,
        /// any callback function executed upon successful completion of the command; a
        /// `CommandCallback *`, compared and called only by native leaves.
        pub callback: *const c_void,
        /// the frame in which this packet is executed
        pub frame: u32,
        /// string ID of error message to use.
        pub err_msg: u32,
        /// company that is executing the command
        pub company: u8,
        /// command being executed.
        pub cmd: u8,
        /// did the command originate from "me"
        pub my_cmd: bool,
    }
    /// `CommandQueue` as the raw parts of a Rust `Vec<CommandPacket>`; null when never allocated.
    "OpenTTDCommandQueue" ["probe"] => pub struct CommandQueue {
        pub data: *mut CommandPacket,
        pub size: usize,
        pub capacity: usize,
    }
    /// One `NetworkClientSocket` resolved once per visit, with its pool index; queues and the
    /// sync record live in the socket shell.
    "OpenTTDNetworkCommandClient" ["probe"] => pub struct Client {
        pub socket: *mut socket::GameSocket,
        pub index: usize,
    }
}

impl CommandData {
    pub(crate) const EMPTY: Self = Self {
        data: null_mut(),
        size: 0,
        capacity: 0,
    };
    fn from_vec(bytes: Vec<u8>) -> Self {
        let mut bytes = ManuallyDrop::new(bytes);
        Self {
            data: bytes.as_mut_ptr(),
            size: bytes.len(),
            capacity: bytes.capacity(),
        }
    }
    fn take(&mut self) -> Vec<u8> {
        let parts = std::mem::replace(self, Self::EMPTY);
        let parts = ManuallyDrop::new(parts);
        if parts.data.is_null() {
            Vec::new()
        } else {
            // SAFETY: Non-null parts always come from `from_vec`.
            unsafe { Vec::from_raw_parts(parts.data, parts.size, parts.capacity) }
        }
    }
    pub(crate) fn span(&self) -> ByteSpan {
        ByteSpan {
            data: self.data,
            size: self.size,
        }
    }
    pub(crate) fn as_slice(&self) -> &[u8] {
        if self.data.is_null() {
            &[]
        } else {
            // SAFETY: Non-null parts always come from `from_vec`.
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
    /// Grows the bytes by at least one, as `std::vector::push_back`.
    fn grow(&mut self) {
        let mut bytes = self.take();
        bytes.reserve(1);
        *self = Self::from_vec(bytes);
    }
}

impl Drop for CommandData {
    fn drop(&mut self) {
        drop(self.take());
    }
}

impl Clone for CommandData {
    /// `std::vector` copy: exactly `size` bytes, no allocation when empty.
    fn clone(&self) -> Self {
        Self::from_vec(self.as_slice().to_vec())
    }
}

impl CommandQueue {
    pub(crate) const EMPTY: Self = Self {
        data: null_mut(),
        size: 0,
        capacity: 0,
    };
    /// Edits the queue as a `Vec`; panics abort, so the parts are always written back.
    ///
    /// # Safety
    /// `queue` is live and holds parts from this function or `EMPTY`; `edit` calls no leaf.
    unsafe fn edit<R>(queue: *mut Self, edit: impl FnOnce(&mut Vec<CommandPacket>) -> R) -> R {
        unsafe {
            let mut packets = if (*queue).data.is_null() {
                Vec::new()
            } else {
                Vec::from_raw_parts((*queue).data, (*queue).size, (*queue).capacity)
            };
            let result = edit(&mut packets);
            let mut packets = ManuallyDrop::new(packets);
            (*queue).data = packets.as_mut_ptr();
            (*queue).size = packets.len();
            (*queue).capacity = packets.capacity();
            result
        }
    }
    /// Frees the queue and leaves it empty.
    ///
    /// # Safety
    /// `queue` is live and holds parts from `edit` or `EMPTY`.
    unsafe fn free(queue: *mut Self) {
        unsafe {
            drop(Self::edit(queue, std::mem::take));
            *queue = Self::EMPTY;
        }
    }
}

/// Native leaves for `network_command.cpp`; see `network_command_ffi.h`.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    None,
    InvalidCommand,
    Offline,
    InvalidCallback,
}
#[repr(C)]
pub struct Services {
    pub admin_log: extern "C" fn(*const c_void, *const CommandPacket),
    pub allow_list_key: extern "C" fn(ByteSpan, *mut c_void),
    pub callback: extern "C" fn(usize) -> *const c_void,
    pub callback_count: usize,
    pub commands_per_frame: extern "C" fn() -> u16,
    pub commands_per_frame_server: extern "C" fn() -> u16,
    pub company: &'static crate::company::Services,
    pub console_bad_client: extern "C" fn(*mut c_void, DecodeError),
    pub console_bad_server: extern "C" fn(DecodeError),
    pub console_company_mismatch: extern "C" fn(*mut c_void, u8),
    pub console_server_only: extern "C" fn(*mut c_void, u8),
    pub console_spectator: extern "C" fn(*mut c_void, u8),
    pub control_action: extern "C" fn(ByteSpan) -> u8,
    pub execute: extern "C" fn(*const CommandPacket, usize),
    pub execute_past: extern "C" fn(),
    pub has_unpack: extern "C" fn(u8, usize) -> bool,
    pub info_key_equals: extern "C" fn(*mut c_void, *mut c_void) -> bool,
    pub max_commands_in_queue: extern "C" fn() -> u16,
    pub network: extern "C" fn() -> &'static crate::network_frame::Services,
    pub next_client: extern "C" fn(usize, *mut Client) -> bool,
    pub next_info: extern "C" fn(*mut u32) -> *mut c_void,
    pub replace_client_id: extern "C" fn(u8, ByteSpan, u32) -> CommandData,
    pub sanitize_strings: extern "C" fn(u8, ByteSpan) -> CommandData,
    pub server_full: extern "C" fn(u32),
}

static WAIT: GameState<CommandQueue> = GameState::new(CommandQueue::EMPTY);
static EXECUTION: GameState<CommandQueue> = GameState::new(CommandQueue::EMPTY);

/// `_local_wait_queue` for the commands console's native-test entry.
#[cfg(feature = "native-test-support")]
pub(crate) fn wait_queue() -> *const CommandQueue {
    WAIT.get()
}

/// `_local_wait_queue.clear()`, as `free_local_command_queue` clears it.
#[cfg(feature = "native-test-support")]
pub(crate) fn clear_wait_queue() {
    // SAFETY: The queue is live and clearing calls no leaf.
    unsafe { CommandQueue::edit(WAIT.get(), Vec::clear) };
}

pub(crate) fn next_client(s: &Services, cursor: &mut usize) -> Option<Client> {
    let mut client = Client {
        socket: null_mut(),
        index: 0,
    };
    (s.next_client)(*cursor, &raw mut client).then(|| {
        *cursor = client.index + 1;
        client
    })
}

/// `NetworkSendCommand` for the native command facade.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_send_command(
    s: &Services,
    cmd: u8,
    err_msg: u32,
    callback: *const c_void,
    company: u8,
    data: ByteSpan,
) {
    send_command(
        s,
        Commands(cmd),
        StringID(err_msg),
        callback,
        CompanyID(company),
        data,
    );
}

/// `NetworkSendCommand` with raw command, string and company integers, for held callers.
pub(crate) fn send_command_raw(
    s: &Services,
    cmd: u8,
    err_msg: u32,
    callback: *const c_void,
    company: u8,
    data: ByteSpan,
) {
    send_command(
        s,
        Commands(cmd),
        StringID(err_msg),
        callback,
        CompanyID(company),
        data,
    );
}

/// `NetworkSendCommand`: prepare a `DoCommand` to be send over the network.
/// `cmd` is the command to execute (a CMD_* value), `err_message` the message prefix to show on
/// error, `callback` a callback function to call after the command is finished, `company` the
/// company that wants to send the command and `cmd_data` the command proc arguments.
pub(crate) fn send_command(
    s: &Services,
    cmd: Commands,
    err_message: StringID,
    callback: *const c_void,
    company: CompanyID,
    cmd_data: ByteSpan,
) {
    let bytes = if cmd_data.size == 0 {
        Vec::new()
    } else {
        // SAFETY: The caller's buffer is live for this entry.
        unsafe { std::slice::from_raw_parts(cmd_data.data, cmd_data.size) }.to_vec()
    };
    let mut c = CommandPacket {
        data: CommandData::from_vec(bytes),
        callback,
        frame: 0,
        err_msg: err_message.0,
        company: company.0,
        cmd: cmd.0,
        my_cmd: false,
    };

    if (s.company.network_server)() {
        /* If we are the server, we queue the command in our 'special' queue.
         *   In theory, we could execute the command right away, but then the
         *   client on the server can do everything 1 tick faster than others.
         *   So to keep the game fair, we delay the command with 1 tick
         *   which gives about the same speed as most clients.
         */
        c.frame = crate::network_frame::frame_counter_max().wrapping_add(1);
        c.my_cmd = true;

        unsafe { CommandQueue::edit(WAIT.get(), |queue| queue.push(c)) };
        return;
    }

    c.frame = 0; // The client can't tell which frame, so just make it 0

    /* Clients send their command to the server and forget all about the packet */
    client_send_command((s.network)(), &c);
}

/// `NetworkSyncCommandQueue`: sync our local command queue to the command queue of the given
/// socket. This is needed for the case where we receive a command
/// before saving the game for a joining client, but without the
/// execution of those commands. Not syncing those commands means
/// that the client will never get them and as such will be in a
/// desynced state from the time it started with joining.
///
/// # Safety
/// `outgoing` is the live queue of a client socket.
pub(crate) unsafe fn sync_command_queue(outgoing: *mut CommandQueue) {
    unsafe {
        let execution = EXECUTION.get();
        for index in 0..(*execution).size {
            let mut c = (*(*execution).data.add(index)).clone();
            c.callback = std::ptr::null();
            CommandQueue::edit(outgoing, |queue| queue.push(c));
        }
    }
}

/// `NetworkExecuteLocalCommandQueue`: execute all commands on the local command queue that ought
/// to be executed this frame.
pub(crate) fn execute_local_command_queue(session: &crate::session::Services, s: &Services) {
    (session.assert_local_company)();

    let company = s.company;
    let queue = if (company.network_server)() {
        EXECUTION.get()
    } else {
        socket::incoming(((s.network)().client_game)())
    };

    let mut executed = 0;
    // SAFETY: Execution never adds to or frees the queue it executes.
    unsafe {
        while executed < (*queue).size {
            let cp = (*queue).data.add(executed);
            /* The queue is always in order, which means
             * that the first element will be executed first. */
            if crate::network_frame::frame_counter() < (*cp).frame {
                break;
            }

            if crate::network_frame::frame_counter() > (*cp).frame {
                /* If we reach here, it means for whatever reason, we've already executed
                 * past the command we need to execute. */
                (s.execute_past)();
            }

            /* We can execute this command */
            (company.finance.set_current_company)((*cp).company);
            (s.execute)(cp, find_callback_index(s, (*cp).callback));
            executed += 1;
        }
        CommandQueue::edit(queue, |queue| drop(queue.drain(..executed)));
    }

    /* Local company may have changed, so we should not restore the old value */
    (company.finance.set_current_company)((company.local_company)());
}

/// `NetworkFreeLocalCommandQueue`: free the local command queues.
pub(crate) fn free_local_command_queue() {
    unsafe {
        CommandQueue::edit(WAIT.get(), Vec::clear);
        CommandQueue::edit(EXECUTION.get(), Vec::clear);
    }
}

/// `DistributeCommandPacket`: "Send" a particular `CommandPacket` to all clients.
/// `cp` is the command that has to be distributed and `owner` the client that owns the command.
unsafe fn distribute_command_packet(
    s: &Services,
    cp: *mut CommandPacket,
    owner: *const socket::GameSocket,
) {
    unsafe {
        let callback = (*cp).callback;
        (*cp).frame = crate::network_frame::frame_counter_max().wrapping_add(1);

        let mut cursor = 0;
        while let Some(client) = next_client(s, &mut cursor) {
            if server::status(client.socket) >= ClientStatus::STATUS_MAP {
                /* Callbacks are only send back to the client who sent them in the
                 *  first place. This filters that out. */
                let own = client.socket.cast_const() == owner;
                (*cp).callback = if own { callback } else { std::ptr::null() };
                (*cp).my_cmd = own;
                let c = (*cp).clone();
                CommandQueue::edit(socket::outgoing(client.socket), |queue| queue.push(c));
            }
        }

        (*cp).callback = if owner.is_null() {
            callback
        } else {
            std::ptr::null()
        };
        (*cp).my_cmd = owner.is_null();
        let c = (*cp).clone();
        CommandQueue::edit(EXECUTION.get(), |queue| queue.push(c));
    }
}

/// `DistributeQueue`: "Send" a particular `CommandQueue` to all clients.
/// `queue` is the queue of commands that has to be distributed and `owner` the client that owns
/// the commands.
unsafe fn distribute_queue(
    s: &Services,
    queue: *mut CommandQueue,
    owner: *const socket::GameSocket,
) {
    let mut to_go = i32::from((s.commands_per_frame)());
    if owner.is_null() {
        /* This is the server, use the commands_per_frame_server setting if higher */
        to_go = to_go.max(i32::from((s.commands_per_frame_server)()));
    }

    /* Not technically the most performant way, but consider clients rarely click more than once per tick. */
    let mut index = 0;
    unsafe {
        while index < (*queue).size {
            let cp = (*queue).data.add(index);
            /* Do not distribute commands when paused and the command is not allowed while paused. */
            let commands = commands(s);
            if PauseModes(crate::session::pause()) != PauseModes::EMPTY
                && !crate::command::allowed_while_paused(
                    commands,
                    (commands.command_type)((*cp).cmd),
                )
            {
                index += 1;
                continue;
            }

            /* Limit the number of commands per client per tick. */
            to_go -= 1;
            if to_go < 0 {
                break;
            }

            distribute_command_packet(s, cp, owner);
            (s.admin_log)(
                if owner.is_null() {
                    std::ptr::null()
                } else {
                    socket::server(owner.cast_mut())
                },
                cp,
            );
            CommandQueue::edit(queue, |queue| drop(queue.remove(index)));
        }
    }
}

/// `NetworkDistributeCommands`: distribute the commands of ourself and the clients.
pub(crate) fn distribute_commands(s: &Services) {
    /* First send the server's commands. */
    unsafe { distribute_queue(s, WAIT.get(), std::ptr::null()) };

    /* Then send the queues of the others. */
    let mut cursor = 0;
    while let Some(client) = next_client(s, &mut cursor) {
        unsafe { distribute_queue(s, socket::incoming(client.socket), client.socket) };
    }
}

/// `NetworkHandleCommandQueue`: handle the command-queue of a socket.
///
/// # Safety
/// `outgoing` is the live queue of `cs`.
pub(crate) unsafe fn handle_command_queue(
    s: &Services,
    cs: *mut socket::GameSocket,
    outgoing: *mut CommandQueue,
) {
    unsafe {
        for index in 0..(*outgoing).size {
            server_send_command((s.network)(), cs, &*(*outgoing).data.add(index));
        }
        CommandQueue::edit(outgoing, Vec::clear);
    }
}

/// `FindCallbackIndex`: the callback index of a callback pointer, or `usize::MAX` if the
/// function wasn't found in the callback list.
fn find_callback_index(s: &Services, callback: *const c_void) -> usize {
    (0..s.callback_count)
        .find(|&i| (s.callback)(i) == callback)
        .unwrap_or(usize::MAX)
}

/// `IsNetworkRegisteredCallback`: ensure that callbacks used when Posting commands are actually
/// registered for the network calls.
fn is_network_registered_callback(s: &Services, callback: *const c_void) -> bool {
    find_callback_index(s, callback) != usize::MAX
}

/// `IsNetworkRegisteredCallback` for the native command facade.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_network_registered_callback(
    s: &Services,
    callback: *const c_void,
) -> u8 {
    u8::from(is_network_registered_callback(s, callback))
}

fn commands(s: &Services) -> &crate::services::CommandServices {
    ((s.company.shared.command_post)().command)()
}

/// `NetworkGameSocketHandler::SendCommand`: sends a command over the network.
/// `p` is the packet to send it in and `cp` the packet to actually send.
#[allow(clippy::cast_possible_truncation)] // Original u16 string ID and u8 callback encoding.
fn send_command_packet(
    f: &crate::network_frame::Services,
    p: &mut crate::packet::State,
    cp: &CommandPacket,
) {
    let s = f.command;
    let check = crate::network_frame::packet_check(f);
    p.send_uint8(cp.company, check);
    p.send_uint16(u16::from(cp.cmd), check);
    p.send_uint16(cp.err_msg as u16, check);
    p.send_buffer(cp.data.as_slice(), check);

    let mut callback = find_callback_index(s, cp.callback);
    if callback > usize::from(u8::MAX) || !(s.has_unpack)(cp.cmd, callback) {
        debug!(
            net,
            0, "Unknown callback for command; no callback sent (command: {})", cp.cmd
        );
        callback = 0; // _callback_table[0] == nullptr
    }
    p.send_uint8(callback as u8, check);
}

/// `ServerNetworkGameSocketHandler::SendCommand`: send a command to the client to execute.
fn server_send_command(
    f: &crate::network_frame::Services,
    cs: *mut socket::GameSocket,
    cp: &CommandPacket,
) -> NetworkRecvStatus {
    use crate::network_frame::{packet_check, packet_new, packet_send};
    debug!(
        net,
        9,
        "client[{}] SendCommand(): cmd={}",
        (f.dispatch.client_id)(cs),
        cp.cmd
    );

    let mut p = packet_new(f, cs, PacketGameType::PACKET_SERVER_COMMAND);

    send_command_packet(f, &mut p, cp);
    p.send_uint32(cp.frame, packet_check(f));
    p.send_bool(cp.my_cmd, packet_check(f));

    packet_send(f, cs, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ClientNetworkGameSocketHandler::SendCommand`: send a command to the server.
fn client_send_command(
    f: &crate::network_frame::Services,
    cp: &CommandPacket,
) -> NetworkRecvStatus {
    use crate::network_frame::{packet_new, packet_send};
    debug!(net, 9, "Client::SendCommand(): cmd={}", cp.cmd);

    let my_client = (f.client_game)();
    let mut p = packet_new(f, my_client, PacketGameType::PACKET_CLIENT_COMMAND);
    send_command_packet(f, &mut p, cp);

    packet_send(f, my_client, &mut p);
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `NetworkGameSocketHandler::ReceiveCommand`: receives a command from the network.
/// `p` is the packet to read from and `cp` the struct to write the data to; the wire's u16
/// command narrows to the native enum's u8 first.
#[allow(clippy::cast_possible_truncation)] // Native Commands narrows the wire u16 to u8.
fn receive_command(
    f: &crate::network_frame::Services,
    p: *mut crate::packet::State,
    cp: &mut CommandPacket,
) -> DecodeError {
    use crate::packet;
    let s = f.command;
    let commands = commands(s);
    unsafe {
        cp.company = packet::recv_uint8(p, f.packet);
        cp.cmd = packet::recv_uint16(p, f.packet) as u8;
        if !(commands.command_valid)(cp.cmd) {
            return DecodeError::InvalidCommand;
        }
        if CommandFlags((commands.command_flags)(cp.cmd)).test(CommandFlag::Offline) {
            return DecodeError::Offline;
        }
        cp.err_msg = u32::from(packet::recv_uint16(p, f.packet));
        let size = packet::recv_buffer_size(p, f.packet);
        let mut bytes = vec![0; size];
        (*p).read_buffer_data(&mut bytes);
        cp.data = (s.sanitize_strings)(
            cp.cmd,
            ByteSpan {
                data: bytes.as_ptr(),
                size,
            },
        );
        drop(bytes);

        let callback = usize::from(packet::recv_uint8(p, f.packet));
        if callback >= s.callback_count || !(s.has_unpack)(cp.cmd, callback) {
            return DecodeError::InvalidCallback;
        }

        cp.callback = (s.callback)(callback);
    }
    DecodeError::None
}

/// A default-constructed `CommandPacket`.
fn empty_packet() -> CommandPacket {
    CommandPacket {
        data: CommandData::EMPTY,
        callback: std::ptr::null(),
        frame: 0,
        err_msg: 0,
        company: CompanyID::INVALID.0,
        cmd: 0,
        my_cmd: false,
    }
}

/// `ClientNetworkGameSocketHandler::Receive_SERVER_COMMAND`.
pub(crate) fn receive_server(
    f: &crate::network_frame::Services,
    game: *mut socket::GameSocket,
    p: *mut crate::packet::State,
) -> NetworkRecvStatus {
    use crate::network_client::ServerStatus;
    if ServerStatus(socket::status(game)) != ServerStatus::STATUS_ACTIVE {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    let mut cp = empty_packet();
    let err = receive_command(f, p, &mut cp);
    cp.frame = unsafe { crate::packet::recv_uint32(p, f.packet) };
    cp.my_cmd = unsafe { crate::packet::recv_bool(p, f.packet) };

    debug!(
        net,
        9, "Client::Receive_SERVER_COMMAND(): cmd={}, frame={}", cp.cmd, cp.frame
    );

    if err != DecodeError::None {
        (f.command.console_bad_server)(err);
        return NetworkRecvStatus::NETWORK_RECV_STATUS_MALFORMED_PACKET;
    }

    unsafe {
        CommandQueue::edit(socket::incoming(game), |q| q.push(cp));
    }

    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// `ServerNetworkGameSocketHandler::Receive_CLIENT_COMMAND`: the client has done a command and
/// wants us to handle it.
#[allow(clippy::too_many_lines)] // The original ordered command admission/error precedence.
pub(crate) fn receive_client(
    f: &crate::network_frame::Services,
    game: *mut socket::GameSocket,
    p: *mut crate::packet::State,
    frame: *mut crate::session::Frame,
    work: crate::network_action::Actions,
) -> NetworkRecvStatus {
    use crate::company_type::{CCA_NEW, COMPANY_NEW_COMPANY, COMPANY_SPECTATOR};
    use crate::network_type::CLIENT_ID_SERVER;
    let send_error = |error| {
        crate::network_connection::send_error(
            f.connection,
            f,
            game,
            error,
            crate::network_connection::EMPTY_TEXT,
            work,
        )
    };
    /* The client was never joined.. so this is impossible, right?
     *  Ignore the packet, give the client a warning, and close the connection */
    if server::status(game) < ClientStatus::STATUS_DONE_MAP
        || crate::network_frame::socket_has_quit(f, game)
    {
        return send_error(NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED);
    }

    let s = f.command;
    if unsafe { (*socket::incoming(game)).size } >= usize::from((s.max_commands_in_queue)()) {
        return send_error(NetworkErrorCode::NETWORK_ERROR_TOO_MANY_COMMANDS);
    }

    let shell = socket::server(game);
    debug!(
        net,
        9,
        "client[{}] Receive_CLIENT_COMMAND()",
        (f.dispatch.client_id)(game)
    );

    let mut cp = empty_packet();
    let err = receive_command(f, p, &mut cp);

    if crate::network_frame::socket_has_quit(f, game) {
        return NetworkRecvStatus::NETWORK_RECV_STATUS_CLIENT_QUIT;
    }

    if err != DecodeError::None {
        (s.console_bad_client)(shell, err);
        return send_error(NetworkErrorCode::NETWORK_ERROR_NOT_EXPECTED);
    }

    let commands = commands(s);
    if CommandFlags((commands.command_flags)(cp.cmd)).test(CommandFlag::Server)
        && (f.dispatch.client_id)(game) != CLIENT_ID_SERVER
    {
        (s.console_server_only)(shell, cp.cmd);
        return send_error(NetworkErrorCode::NETWORK_ERROR_KICKED);
    }

    if !CommandFlags((commands.command_flags)(cp.cmd)).test(CommandFlag::Spectator)
        && !(s.company.shared.company_valid)(cp.company)
        && (f.dispatch.client_id)(game) != CLIENT_ID_SERVER
    {
        (s.console_spectator)(shell, cp.cmd);
        return send_error(NetworkErrorCode::NETWORK_ERROR_KICKED);
    }

    /*
     * Only CMD_COMPANY_CTRL is always allowed, for the rest, playas needs
     * to match the company in the packet. If it doesn't, the client has done
     * something pretty naughty (or a bug), and will be kicked
     */
    let cca = if Commands(cp.cmd) == Commands::CMD_COMPANY_CTRL {
        (s.control_action)(cp.data.span())
    } else {
        CCA_NEW
    };
    if !(Commands(cp.cmd) == Commands::CMD_COMPANY_CTRL
        && cca == CCA_NEW
        && (f.connection.client_company)(shell) == COMPANY_NEW_COMPANY)
        && (f.connection.client_company)(shell) != cp.company
    {
        (s.console_company_mismatch)(shell, cp.company);
        return send_error(NetworkErrorCode::NETWORK_ERROR_COMPANY_MISMATCH);
    }

    if Commands(cp.cmd) == Commands::CMD_COMPANY_CTRL {
        if cca != CCA_NEW || cp.company != COMPANY_SPECTATOR {
            return send_error(NetworkErrorCode::NETWORK_ERROR_CHEATER);
        }

        /* Check if we are full - else it's possible for spectators to send a CMD_COMPANY_CTRL and the company is created regardless of max_companies! */
        if (s.company.company_count)() >= usize::from((f.connection.max_companies)()) {
            (s.server_full)((f.dispatch.client_id)(game));
            return NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY;
        }
    }

    if Commands(cp.cmd) == Commands::CMD_COMPANY_ALLOW_LIST_CTRL {
        /* Maybe the client just got moved before allowing? */
        if (f.dispatch.client_id)(game) != CLIENT_ID_SERVER
            && (f.connection.client_company)(shell) != cp.company
        {
            return NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY;
        }

        /* Only allow clients to add/remove currently joined clients. The server owner does not go via this method, so is allowed to do more. */
        let public_key = (f.dispatch.text_begin)(frame);
        (s.allow_list_key)(cp.data.span(), public_key);
        let mut cursor = 0;
        let found = loop {
            let info = (s.next_info)(&raw mut cursor);
            if info.is_null() {
                break false;
            }
            if (s.info_key_equals)(info, public_key) {
                break true;
            }
        };
        // The original key temporary ends before client-ID replacement and queue insertion.
        (f.dispatch.packet_end)(frame);

        /* Maybe the client just left? */
        if !found {
            return NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY;
        }
    }

    if CommandFlags((commands.command_flags)(cp.cmd)).test(CommandFlag::ClientID) {
        cp.data = (s.replace_client_id)(cp.cmd, cp.data.span(), (f.dispatch.client_id)(game));
    }
    unsafe {
        CommandQueue::edit(socket::incoming(game), |q| q.push(cp));
    }
    NetworkRecvStatus::NETWORK_RECV_STATUS_OKAY
}

/// Grows `data` by at least one byte, as `std::vector::push_back`.
///
/// # Safety
/// `data` is live and holds parts from this module or zeroes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_data_grow(data: *mut CommandData) {
    unsafe { (*data).grow() };
}

/// # Safety
/// `data` is live and holds parts from this module or zeroes; it is left empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_data_free(data: *mut CommandData) {
    unsafe { drop((*data).take()) };
}

/// # Safety
/// `queue` is live and holds parts from this module or zeroes; it is left empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_queue_free(queue: *mut CommandQueue) {
    unsafe { CommandQueue::free(queue) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNetworkCommandServices" ["probe"] => Services {
        admin_log,
        allow_list_key,
        callback,
        callback_count,
        commands_per_frame,
        commands_per_frame_server,
        company,
        console_bad_client,
        console_bad_server,
        console_company_mismatch,
        console_server_only,
        console_spectator,
        control_action,
        execute,
        execute_past,
        has_unpack,
        info_key_equals,
        max_commands_in_queue,
        network,
        next_client,
        next_info,
        replace_client_id,
        sanitize_strings,
        server_full,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(frame: u32, bytes: &[u8]) -> CommandPacket {
        CommandPacket {
            data: CommandData::from_vec(bytes.to_vec()),
            callback: std::ptr::null(),
            frame,
            err_msg: 0,
            company: 0,
            cmd: 0,
            my_cmd: false,
        }
    }

    #[test]
    fn queue_parts_survive_edits_and_moves() {
        let mut queue = CommandQueue::EMPTY;
        let moved = packet(7, &[1, 2, 3]);
        unsafe {
            CommandQueue::edit(&raw mut queue, |q| q.push(moved));
            CommandQueue::edit(&raw mut queue, |q| q.push(packet(8, &[])));
            assert_eq!(queue.size, 2);
            assert_eq!((*queue.data).data.as_slice(), &[1, 2, 3]);
            assert_eq!((*queue.data.add(1)).frame, 8);
            CommandQueue::edit(&raw mut queue, |q| drop(q.remove(0)));
            assert_eq!((*queue.data).frame, 8);
            CommandQueue::free(&raw mut queue);
        }
        assert!(queue.data.is_null() && queue.size == 0 && queue.capacity == 0);
    }

    #[test]
    fn data_grows_from_zeroed_parts_like_push_back() {
        let mut data = CommandData::EMPTY;
        for byte in 0..40u8 {
            unsafe {
                if data.size == data.capacity {
                    data.grow();
                }
                *data.data.add(data.size) = byte;
            }
            data.size += 1;
        }
        let expected: Vec<u8> = (0..40).collect();
        assert_eq!(data.as_slice(), expected.as_slice());
        let copy = data.clone();
        assert_eq!(copy.capacity, 40);
        assert_eq!(copy.as_slice(), data.as_slice());
    }
}

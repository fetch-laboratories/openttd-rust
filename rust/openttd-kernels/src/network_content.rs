/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The content client from `network_content.cpp` and `tcp_content.cpp`: metadata, dependency
//! resolution, request/cancellation/completion and download state, packet dispatch and the
//! HTTP response decisions. Native leaves keep the socket shell, `ContentInfo` text and lists,
//! files, decompression, HTTP transfers, timers and windows.
//!
//! The process-wide state is borrowed only inside `with`; every leaf runs after that borrow
//! ends, because sockets, callbacks and errors can reenter these entries. Record pointers are
//! the `ContentInfo` base records the native owner allocates and deletes through leaves.
#![allow(unsafe_code)]

use crate::debug::{Bytes, debug};
use crate::game_state::GameState;
use crate::network_config::{
    NETWORK_CONTENT_DESC_LENGTH, NETWORK_CONTENT_FILENAME_LENGTH, NETWORK_CONTENT_NAME_LENGTH,
    NETWORK_CONTENT_TAG_LENGTH, NETWORK_CONTENT_URL_LENGTH, NETWORK_CONTENT_VERSION_LENGTH,
    TCP_MTU,
};
use crate::network_tcp::{self as tcp, NetworkTCPSocketHandler, Packet};
use crate::packet::{self, State as PacketState};
use crate::services::ByteSpan;
use crate::services_assert::check;
use crate::string_services::append;
use crate::tcp_content_type::{self as ct, ContentInfo, state};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ContentCallback {
    _private: [u8; 0],
}
pub struct ContentVector {
    _private: [u8; 0],
}
pub struct ConstContentVector {
    _private: [u8; 0],
}

/// `GetHasContentProcforContentType` and the procedure's answer.
#[allow(dead_code)] // Native leaves construct every variant.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HasContent {
    NoProc,
    Missing,
    Present,
}

/// A native `std::vector<ContentID>` viewed for the producing call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IdSpan {
    pub data: *const u32,
    pub size: usize,
}

#[repr(C)]
pub struct Services {
    pub callback_complete: extern "C" fn(*mut ContentCallback, u32),
    pub callback_connect: extern "C" fn(*mut ContentCallback, bool),
    pub callback_disconnect: extern "C" fn(*mut ContentCallback),
    pub callback_progress: extern "C" fn(*mut ContentCallback, *const ContentInfo, i32),
    pub callback_receive: extern "C" fn(*mut ContentCallback, *const ContentInfo),
    /// `_network_content_client`.
    pub client: *mut NetworkTCPSocketHandler,
    /// `CloseSocket`.
    pub close_socket: extern "C" fn(),
    /// `TCPConnecter::Create<NetworkContentConnecter>` for the configured server.
    pub connect: extern "C" fn(),
    /// `_openttd_content_version`.
    pub content_version: extern "C" fn() -> ByteSpan,
    pub dependencies: extern "C" fn(*const ContentInfo) -> IdSpan,
    pub dependency_push: extern "C" fn(*mut ContentInfo, u32),
    pub description: extern "C" fn(*mut ContentInfo) -> *mut c_void,
    /// `ShowErrorMessage(STR_CONTENT_ERROR_COULD_NOT_EXTRACT)`.
    pub error_extract: extern "C" fn(),
    /// Close the download window and show the not-writable error.
    pub error_not_writable: extern "C" fn(),
    /// `GunzipFile`, then remove the archive, scan the tar and extract music.
    pub extract: extern "C" fn(*const ContentInfo) -> bool,
    pub file_close: extern "C" fn(),
    /// Open the compressed download file for writing.
    pub file_create: extern "C" fn(*const ContentInfo) -> bool,
    pub file_open: extern "C" fn() -> bool,
    /// `fwrite` to the current file.
    pub file_write: extern "C" fn(*const u8, usize) -> usize,
    pub filename: extern "C" fn(*mut ContentInfo) -> *mut c_void,
    pub has_content: extern "C" fn(*const ContentInfo, bool) -> HasContent,
    /// `NetworkHTTPSocketHandler::Connect` for a file URL.
    pub http_get: extern "C" fn(ByteSpan),
    /// `NetworkHTTPSocketHandler::Connect` to the mirror with a request body.
    pub http_mirror: extern "C" fn(ByteSpan),
    pub info_assign: extern "C" fn(*mut ContentInfo, *const ContentInfo),
    pub info_delete: extern "C" fn(*mut ContentInfo),
    pub info_new: extern "C" fn() -> *mut ContentInfo,
    pub list_get: extern "C" fn(*mut ContentVector, usize) -> *mut ContentInfo,
    pub list_release: extern "C" fn(*mut ContentVector, usize) -> *mut ContentInfo,
    pub list_size: extern "C" fn(*const ContentVector) -> usize,
    pub name: extern "C" fn(*mut ContentInfo) -> *mut c_void,
    /// `_request_queue_timeout.Reset()`.
    pub queue_reset: extern "C" fn(),
    pub string_assign: extern "C" fn(*mut c_void, *const c_void),
    pub string_empty: extern "C" fn(*const c_void) -> bool,
    /// Append an empty tag and return it.
    pub tag: extern "C" fn(*mut ContentInfo) -> *mut c_void,
    pub tcp: &'static tcp::Services,
    /// `NetworkTCPSocketHandler::CloseConnection`.
    pub tcp_close: extern "C" fn(),
    pub tree_push: extern "C" fn(*mut ConstContentVector, *const ContentInfo),
    pub url: extern "C" fn(*mut ContentInfo) -> *mut c_void,
    pub version: extern "C" fn(*mut ContentInfo) -> *mut c_void,
}

/// `ClientNetworkContentSocketHandler::IDLE_TIMEOUT`.
const IDLE_TIMEOUT: i64 = 60_000_000_000;
/// `CONTENT_QUEUE_TIMEOUT`.
const CONTENT_QUEUE_TIMEOUT: i64 = 100_000_000;
/// `MAX_PACKETS_TO_RECEIVE`.
const MAX_PACKETS_TO_RECEIVE: i32 = 42;
/// IDs per `CLIENT_INFO_ID` or `CLIENT_CONTENT` packet after size, type and count.
const MAX_CONTENT_IDS_PER_PACKET: usize = (TCP_MTU - 2 - 1 - 2) / 4;

struct Client {
    callbacks: Vec<*mut ContentCallback>,
    requested: Vec<u32>,
    queued: Vec<u32>,
    infos: Vec<*mut ContentInfo>,
    /// `reverse_dependency_map`: libstdc++'s `unordered_multimap::emplace` places a new
    /// element before its equal keys, so `equal_range` visits each key's parents newest first.
    reverse: BTreeMap<u32, Vec<u32>>,
    http_response: Vec<u8>,
    http_response_index: i32,
    cur_info: *mut ContentInfo,
    is_connecting: bool,
    last_activity: i64,
}

static CLIENT: GameState<Client> = GameState::new(Client {
    callbacks: Vec::new(),
    requested: Vec::new(),
    queued: Vec::new(),
    infos: Vec::new(),
    reverse: BTreeMap::new(),
    http_response: Vec::new(),
    http_response_index: -2,
    cur_info: std::ptr::null_mut(),
    is_connecting: false,
    last_activity: 0,
});
/// `is_cancelled`; the HTTP owner also reads it through `IsCancelled`.
static CANCELLED: AtomicBool = AtomicBool::new(false);

fn with<R>(f: impl FnOnce(&mut Client) -> R) -> R {
    // SAFETY: Serialized by the game-state mutex; callers end the borrow before any leaf.
    f(unsafe { &mut *CLIENT.get() })
}

/// A live record; the original dereferences a null parent or dependency without a check.
fn info<'a>(ci: *const ContentInfo) -> &'a ContentInfo {
    // SAFETY: Records stay live until `Clear` or a stub merge deletes them, as in the original.
    unsafe { ci.as_ref() }.expect("content record")
}
fn info_mut<'a>(ci: *mut ContentInfo) -> &'a mut ContentInfo {
    // SAFETY: As `info`; no other Rust reference to the record is live.
    unsafe { ci.as_mut() }.expect("content record")
}

fn socket(s: &Services) -> *mut packet::NetworkSocketHandler {
    (s.tcp.base_socket)(s.client)
}

fn get_content(cid: u32) -> *mut ContentInfo {
    with(|c| {
        c.infos
            .iter()
            .copied()
            .find(|&ci| info(ci).id == cid)
            .unwrap_or(std::ptr::null_mut())
    })
}

fn notify(each: impl Fn(*mut ContentCallback)) {
    // A callback may add callbacks or remove itself.
    let mut i = 0;
    while let Some(cb) = with(|c| c.callbacks.get(i).copied()) {
        each(cb);
        if with(|c| c.callbacks.get(i) == Some(&cb)) {
            i += 1;
        }
    }
}

fn on_connect(s: &Services, success: bool) {
    notify(|cb| (s.callback_connect)(cb, success));
}
fn on_disconnect(s: &Services) {
    notify(|cb| (s.callback_disconnect)(cb));
}
fn on_receive_content_info(s: &Services, ci: *const ContentInfo) {
    notify(|cb| (s.callback_receive)(cb, ci));
}
fn on_download_progress(s: &Services, ci: *const ContentInfo, bytes: i32) {
    notify(|cb| (s.callback_progress)(cb, ci, bytes));
}
fn on_download_complete(s: &Services, cid: u32) {
    let ci = get_content(cid);
    if !ci.is_null() {
        info_mut(ci).state = state::ALREADY_HERE;
    }
    notify(|cb| (s.callback_complete)(cb, cid));
}

fn new_packet(s: &Services, kind: u8, limit: usize) -> PacketState {
    unsafe { packet::new_outgoing(socket(s), s.tcp.packet, check, kind, limit) }
}
fn send_packet(s: &Services, mut p: PacketState) {
    unsafe { packet::prepare_to_send(&raw mut p, s.tcp.packet, check) };
    (s.tcp.packet_enqueue)(s.client, &raw mut p);
}

fn recv_string(s: &Services, p: *mut PacketState, text: *mut c_void, length: usize, settings: u8) {
    unsafe {
        packet::recv_string_into(
            packet::ReceiveContext {
                state: p,
                services: s.tcp.packet,
            },
            check,
            length,
            crate::string_type::StringValidationSettings(settings),
            |bytes| {
                append(text, bytes);
            },
        );
    }
}

fn connect(s: &Services) {
    if (s.tcp.valid)(s.client) || with(|c| c.is_connecting) {
        return;
    }
    CANCELLED.store(false, Ordering::Relaxed);
    with(|c| c.is_connecting = true);
    (s.connect)();
}

fn close_connection(s: &Services) {
    (s.tcp_close)();
    if !(s.tcp.valid)(s.client) {
        return;
    }
    (s.close_socket)();
    on_disconnect(s);
}

fn handle_packet(s: &Services, p: *mut PacketState) -> bool {
    let kind = unsafe { packet::recv_uint8(p, s.tcp.packet) };
    let quit = unsafe { (s.tcp.packet.has_quit)(socket(s)) };
    match if quit { ct::PACKET_CONTENT_END } else { kind } {
        ct::PACKET_CONTENT_SERVER_INFO => receive_server_info(s, p),
        ct::PACKET_CONTENT_SERVER_CONTENT => receive_server_content(s, p),
        ct::PACKET_CONTENT_CLIENT_INFO_LIST
        | ct::PACKET_CONTENT_CLIENT_INFO_ID
        | ct::PACKET_CONTENT_CLIENT_INFO_EXTID
        | ct::PACKET_CONTENT_CLIENT_INFO_EXTID_MD5
        | ct::PACKET_CONTENT_CLIENT_CONTENT => {
            debug!(
                net,
                0, "[tcp/content] Received illegal packet type {}", kind
            );
            false
        }
        _ => {
            if quit {
                debug!(
                    net,
                    0, "[tcp/content] Received invalid packet type {}", kind
                );
            } else {
                debug!(net, 0, "[tcp/content] Received illegal packet");
            }
            false
        }
    }
}

/// `NetworkTCPSocketHandler::ReceivePacket` using the integrated TCP and packet owners.
fn receive_packet(s: &Services) -> *mut Packet {
    match unsafe {
        tcp::receive_packet(
            tcp::Context {
                services: s.tcp,
                socket: s.client,
            },
            check,
        )
    } {
        tcp::Receive::None => std::ptr::null_mut(),
        tcp::Receive::Close => {
            close_connection(s);
            std::ptr::null_mut()
        }
        tcp::Receive::Ready => {
            let p = (s.tcp.receive_packet)(s.client);
            if !unsafe { packet::prepare_to_read(p, s.tcp.packet) } {
                debug!(
                    net,
                    0, "Invalid packet received (too small / decryption error)"
                );
                close_connection(s);
                return std::ptr::null_mut();
            }
            (s.tcp.receive_take)(s.client)
        }
    }
}

/// `NetworkContentSocketHandler::ReceivePackets`: a bounded batch per call.
fn receive_packets(s: &Services) -> bool {
    let mut i = MAX_PACKETS_TO_RECEIVE;
    loop {
        i -= 1;
        if i == 0 {
            break;
        }
        let p = receive_packet(s);
        if p.is_null() {
            break;
        }
        let cont = handle_packet(s, (s.tcp.packet_record)(p));
        tcp::destroy_packet(s.tcp, p);
        if !cont {
            return true;
        }
    }
    i != MAX_PACKETS_TO_RECEIVE - 1
}

fn receive_server_info(s: &Services, p: *mut PacketState) -> bool {
    let packets = s.tcp.packet;
    let ci = (s.info_new)();
    unsafe {
        info_mut(ci).r#type = packet::recv_uint8(p, packets);
        info_mut(ci).id = packet::recv_uint32(p, packets);
        info_mut(ci).filesize = packet::recv_uint32(p, packets);
    }
    let plain = crate::string_type::REPLACE_WITH_QUESTION_MARK;
    recv_string(s, p, (s.name)(ci), NETWORK_CONTENT_NAME_LENGTH, plain);
    recv_string(s, p, (s.version)(ci), NETWORK_CONTENT_VERSION_LENGTH, plain);
    recv_string(s, p, (s.url)(ci), NETWORK_CONTENT_URL_LENGTH, plain);
    let multiline = plain | crate::string_type::ALLOW_NEWLINE;
    recv_string(
        s,
        p,
        (s.description)(ci),
        NETWORK_CONTENT_DESC_LENGTH,
        multiline,
    );
    unsafe {
        info_mut(ci).unique_id = packet::recv_uint32(p, packets);
        (&mut *p).recv_bytes(&mut info_mut(ci).md5sum);
    }
    let dependency_count = unsafe { packet::recv_uint8(p, packets) };
    for _ in 0..dependency_count {
        let dependency = unsafe { packet::recv_uint32(p, packets) };
        (s.dependency_push)(ci, dependency);
        let parent = info(ci).id;
        with(|c| c.reverse.entry(dependency).or_default().push(parent));
    }
    let tag_count = unsafe { packet::recv_uint8(p, packets) };
    for _ in 0..tag_count {
        recv_string(s, p, (s.tag)(ci), NETWORK_CONTENT_TAG_LENGTH, plain);
    }

    if !info(ci).is_valid() {
        close_connection(s);
        (s.info_delete)(ci);
        return false;
    }

    match (s.has_content)(ci, true) {
        HasContent::Present => info_mut(ci).state = state::ALREADY_HERE,
        HasContent::Missing => {
            info_mut(ci).state = state::UNSELECTED;
            if (s.has_content)(ci, false) == HasContent::Present {
                info_mut(ci).upgrade = true;
            }
        }
        HasContent::NoProc => info_mut(ci).state = state::UNSELECTED,
    }

    // Something we do not have with file size 0 does not exist in the system.
    if info(ci).state == state::UNSELECTED && info(ci).filesize == 0 {
        info_mut(ci).state = state::DOES_NOT_EXIST;
    }

    let stub = with(|c| {
        c.infos.iter().copied().find(|&ici| {
            let (old, new) = (info(ici), info(ci));
            old.r#type == new.r#type && old.unique_id == new.unique_id && new.md5sum == old.md5sum
        })
    });
    if let Some(ici) = stub {
        if (s.string_empty)((s.name)(ci)) {
            (s.string_assign)((s.name)(ci), (s.name)(ici));
        }
        if info(ici).is_selected() {
            info_mut(ci).state = info(ici).state;
        }
        // A content window may hold the stub, so it keeps its address and takes the new values.
        (s.info_assign)(ici, ci);
        on_receive_content_info(s, ici);
        (s.info_delete)(ci);
        return true;
    }

    // Missing content info is not listed.
    if info(ci).filesize == 0 {
        (s.info_delete)(ci);
        return true;
    }

    with(|c| c.infos.push(ci));

    // Incoming data means that dependencies might need reconsideration.
    for parent in reverse_lookup_tree_dependency(ci) {
        check_dependency_state(s, parent);
    }

    on_receive_content_info(s, ci);
    true
}

fn request_content_list_type(s: &Services, kind: u8) {
    if kind == ct::CONTENT_TYPE_END {
        for kind in [
            ct::CONTENT_TYPE_BASE_GRAPHICS,
            ct::CONTENT_TYPE_BASE_MUSIC,
            ct::CONTENT_TYPE_BASE_SOUNDS,
            ct::CONTENT_TYPE_SCENARIO,
            ct::CONTENT_TYPE_HEIGHTMAP,
            ct::CONTENT_TYPE_AI,
            ct::CONTENT_TYPE_AI_LIBRARY,
            ct::CONTENT_TYPE_GAME,
            ct::CONTENT_TYPE_GAME_LIBRARY,
            ct::CONTENT_TYPE_NEWGRF,
        ] {
            request_content_list_type(s, kind);
        }
        return;
    }

    connect(s);

    let mut p = new_packet(
        s,
        ct::PACKET_CONTENT_CLIENT_INFO_LIST,
        crate::network_config::COMPAT_MTU,
    );
    p.send_uint8(kind, check);
    p.send_uint32(0xffff_ffff, check);
    p.send_uint8(1, check);
    p.send_string(b"vanilla", check);
    p.send_string(unsafe { (s.content_version)().bytes() }, check);
    send_packet(s, p);
}

fn request_content_list_ids(s: &Services, content_ids: &[u32]) {
    if content_ids.is_empty() {
        return;
    }

    connect(s);

    for chunk in content_ids.chunks(MAX_CONTENT_IDS_PER_PACKET) {
        let mut p = new_packet(s, ct::PACKET_CONTENT_CLIENT_INFO_ID, TCP_MTU);
        #[allow(clippy::cast_possible_truncation)]
        p.send_uint16(chunk.len() as u16, check);
        for &id in chunk {
            p.send_uint32(id, check);
        }
        send_packet(s, p);
    }
}

fn request_content_list_vector(s: &Services, cv: *mut ContentVector, send_md5sum: bool) {
    if cv.is_null() {
        return;
    }

    connect(s);

    let count = (s.list_size)(cv);
    check(count < 255);
    check(count < (TCP_MTU - 2 - 1 - 1) / (1 + 4 + if send_md5sum { 16 } else { 0 }));

    let kind = if send_md5sum {
        ct::PACKET_CONTENT_CLIENT_INFO_EXTID_MD5
    } else {
        ct::PACKET_CONTENT_CLIENT_INFO_EXTID
    };
    let mut p = new_packet(s, kind, TCP_MTU);
    #[allow(clippy::cast_possible_truncation)]
    p.send_uint8(count as u8, check);
    for i in 0..count {
        let ci = info((s.list_get)(cv, i));
        p.send_uint8(ci.r#type, check);
        p.send_uint32(ci.unique_id, check);
        if send_md5sum {
            p.send_bytes(&ci.md5sum);
        }
    }
    send_packet(s, p);

    for i in 0..count {
        let ci = info((s.list_get)(cv, i));
        let found = with(|c| {
            c.infos.iter().any(|&ci2| {
                let ci2 = info(ci2);
                ci.r#type == ci2.r#type
                    && ci.unique_id == ci2.unique_id
                    && (!send_md5sum || ci.md5sum == ci2.md5sum)
            })
        });
        if !found {
            let owned = (s.list_release)(cv, i);
            with(|c| c.infos.push(owned));
        }
    }
}

fn download_selected_content(s: &Services, files: &mut u32, bytes: &mut u32, fallback: bool) {
    *bytes = 0;

    let mut content = Vec::new();
    with(|c| {
        for &ci in &c.infos {
            let ci = info(ci);
            if !ci.is_selected() || ci.state == state::ALREADY_HERE {
                continue;
            }
            content.push(ci.id);
            *bytes = bytes.wrapping_add(ci.filesize);
        }
    });

    #[allow(clippy::cast_possible_truncation)]
    {
        *files = content.len() as u32;
    }

    // If there is nothing to download, do nothing.
    if *files == 0 {
        return;
    }

    CANCELLED.store(false, Ordering::Relaxed);

    if fallback {
        download_selected_content_fallback(s, &content);
    } else {
        download_selected_content_http(s, &content);
    }
}

fn download_selected_content_http(s: &Services, content: &[u32]) {
    let mut content_request = Vec::new();
    for id in content {
        content_request.extend_from_slice(id.to_string().as_bytes());
        content_request.push(b'\n');
    }

    with(|c| c.http_response_index = -1);

    (s.http_mirror)(ByteSpan {
        data: content_request.as_ptr(),
        size: content_request.len(),
    });
}

fn download_selected_content_fallback(s: &Services, content: &[u32]) {
    connect(s);

    for chunk in content.chunks(MAX_CONTENT_IDS_PER_PACKET) {
        let mut p = new_packet(s, ct::PACKET_CONTENT_CLIENT_CONTENT, TCP_MTU);
        #[allow(clippy::cast_possible_truncation)]
        p.send_uint16(chunk.len() as u16, check);
        for &id in chunk {
            p.send_uint32(id, check);
        }
        send_packet(s, p);
    }
}

fn replace_cur_info(s: &Services) -> *mut ContentInfo {
    let info = (s.info_new)();
    let old = with(|c| std::mem::replace(&mut c.cur_info, info));
    if !old.is_null() {
        (s.info_delete)(old);
    }
    info
}

fn receive_server_content(s: &Services, p: *mut PacketState) -> bool {
    if (s.file_open)() {
        return receive_content_data(s, p);
    }
    {
        // Without an open file this is the first packet, with metadata.
        let packets = s.tcp.packet;
        let ci = replace_cur_info(s);
        unsafe {
            info_mut(ci).r#type = packet::recv_uint8(p, packets);
            info_mut(ci).id = packet::recv_uint32(p, packets);
            info_mut(ci).filesize = packet::recv_uint32(p, packets);
        }
        let plain = crate::string_type::REPLACE_WITH_QUESTION_MARK;
        recv_string(
            s,
            p,
            (s.filename)(ci),
            NETWORK_CONTENT_FILENAME_LENGTH,
            plain,
        );

        if !before_download(s) {
            close_connection(s);
            return false;
        }
    }
    true
}

/// A data packet of the open download.
fn receive_content_data(s: &Services, p: *mut PacketState) -> bool {
    {
        // With an open file the packet holds downloaded data.
        let (position, size) = unsafe { ((*p).position, (*p).size) };
        #[allow(clippy::cast_possible_wrap)]
        let to_read = size.wrapping_sub(usize::from(position)) as isize;
        if to_read != 0 && transfer_out(s, p) != to_read {
            (s.error_not_writable)();
            close_connection(s);
            (s.file_close)();
            return false;
        }

        let ci = with(|c| c.cur_info);
        #[allow(clippy::cast_possible_truncation)]
        on_download_progress(s, ci, to_read as i32);

        if to_read == 0 {
            after_download(s);
        }
    }

    true
}

/// `Packet::TransferOut` into the current file.
fn transfer_out(s: &Services, p: *mut PacketState) -> isize {
    let amount = unsafe { (*p).transfer_amount(usize::MAX) };
    if amount == 0 {
        return 0;
    }
    let (position, size, data) = unsafe { ((*p).position, (*p).size, (*p).data) };
    check(usize::from(position) < size);
    check(usize::from(position).wrapping_add(amount) <= size);
    #[allow(clippy::cast_possible_wrap)]
    let bytes = (s.file_write)(unsafe { data.add(usize::from(position)) }, amount) as isize;
    unsafe { (*p).transfer_commit(bytes) };
    bytes
}

fn before_download(s: &Services) -> bool {
    let ci = with(|c| c.cur_info);
    if !info(ci).is_valid() {
        with(|c| c.cur_info = std::ptr::null_mut());
        (s.info_delete)(ci);
        return false;
    }

    // A nonzero file size means a download.
    if info(ci).filesize != 0 && !(s.file_create)(ci) {
        (s.error_not_writable)();
        return false;
    }
    true
}

fn after_download(s: &Services) {
    // Reading nothing marks the end of the stream; extract the archive and make it known.
    (s.file_close)();

    let ci = with(|c| c.cur_info);
    if (s.extract)(ci) {
        on_download_complete(s, info(ci).id);
    } else {
        (s.error_extract)();
    }
}

/// `OnFailure`, also called to clean up after the last HTTP download.
fn on_failure(s: &Services) {
    with(|c| {
        c.http_response = Vec::new();
        c.http_response_index = -2;
    });

    if (s.file_open)() {
        let ci = with(|c| c.cur_info);
        on_download_progress(s, ci, -1);
        (s.file_close)();
    }

    // On failure the rest downloads through the fallback protocol.
    if !CANCELLED.load(Ordering::Relaxed) {
        let (mut files, mut bytes) = (0, 0);
        download_selected_content(s, &mut files, &mut bytes, true);
    }
}

fn on_receive_data(s: &Services, data: &[u8], has_data: bool) {
    // Latent data arrives from a connection that was closed.
    let index = with(|c| c.http_response_index);
    if index == -2 {
        return;
    }

    if index == -1 {
        if has_data {
            // Append the rest of the response.
            with(|c| c.http_response.extend_from_slice(data));
            return;
        }
        // Terminate the response and prepare for the file data.
        with(|c| {
            c.http_response.push(0);
            c.http_response_index = 0;
        });
    }

    if has_data {
        // Data belongs to the current file.
        if (s.file_write)(data.as_ptr(), data.len()) == data.len() {
            let ci = with(|c| c.cur_info);
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            on_download_progress(s, ci, data.len() as i32);
        } else {
            // A failed write retries through the fallback protocol.
            on_failure(s);
        }
        return;
    }

    if (s.file_open)() {
        // A file finished downloading.
        after_download(s);
    }

    #[allow(clippy::cast_sign_loss)]
    if with(|c| c.http_response_index as u32 as usize >= c.http_response.len()) {
        // Not a real failure, but it cleans up when there is nothing more to download.
        on_failure(s);
        return;
    }

    let ci = replace_cur_info(s);
    if parse_response_line(s, ci).is_none() {
        on_failure(s);
    }
}

/// The `StringConsumer` reads of one mirror response line.
struct Line<'a> {
    src: &'a [u8],
    position: usize,
}
impl<'a> Line<'a> {
    fn left(&self) -> &'a [u8] {
        &self.src[self.position..]
    }
    /// `ReadIntegerBase<T>(10)`: logs a parse error, then always skips the integer.
    fn read_integer(&mut self, width: u8) -> u32 {
        let src = self.left();
        let result = crate::integer::parse(src, 10, width, false, false);
        if result.error_kind != 0 {
            let part = &src[result.error_offset..];
            let head = &part[..result.error_length.min(part.len())];
            if result.error_kind == 3 {
                debug!(misc, 0, "Integer out of range: '{}'", Bytes(head));
            } else {
                let tail = &part[head.len()..];
                debug!(
                    misc,
                    0,
                    "{}: '{}'+'{}'",
                    if result.error_kind == 2 {
                        "Integer out of range"
                    } else {
                        "Cannot parse integer"
                    },
                    Bytes(head),
                    Bytes(&tail[..tail.len().min(4)])
                );
            }
        }
        self.position += crate::integer::skip(src, 10);
        #[allow(clippy::cast_possible_truncation)]
        if result.length > 0 {
            result.value_bits as u32
        } else {
            0
        }
    }
    fn read_if(&mut self, prefix: &[u8]) -> bool {
        let found = self.left().starts_with(prefix);
        if found {
            self.position += prefix.len();
        }
        found
    }
    /// `SkipUntilChar(c, KEEP_SEPARATOR)`.
    fn skip_until(&mut self, c: u8) {
        self.position += self
            .left()
            .iter()
            .position(|&b| b == c)
            .unwrap_or(self.left().len());
    }
    /// `ReadUntilChar(c, KEEP_SEPARATOR)`.
    fn read_until(&mut self, c: u8) -> &'a [u8] {
        let left = self.left();
        let length = left.iter().position(|&b| b == c).unwrap_or(left.len());
        self.position += length;
        &left[..length]
    }
}

/// The mirror response loop; `None` is the original's thrown failure.
fn parse_response_line(s: &Services, ci: *mut ContentInfo) -> Option<()> {
    // Leaves below never touch the response, so its buffer stays put for the loop.
    let (data, size) = with(|c| (c.http_response.as_ptr(), c.http_response.len()));
    // SAFETY: The response stays allocated and unmodified until this loop ends.
    let response = unsafe { std::slice::from_raw_parts(data, size) };
    loop {
        #[allow(clippy::cast_sign_loss)]
        let index = with(|c| c.http_response_index) as usize;
        let buffer = &response[index..];
        let len = buffer.iter().position(|&b| b == b'\n')?;
        // Update the index for the next line.
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        with(|c| {
            c.http_response_index = c.http_response_index.wrapping_add((len + 1) as i32);
        });

        let mut line = Line {
            src: &buffer[..len],
            position: 0,
        };

        info_mut(ci).id = line.read_integer(32);
        if !line.read_if(b",") {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)]
        {
            info_mut(ci).r#type = line.read_integer(32) as u8;
        }
        if !line.read_if(b",") {
            return None;
        }
        info_mut(ci).filesize = line.read_integer(32);
        if !line.read_if(b",") {
            return None;
        }

        let url = line.left();

        // A fallback URL continues with the next line.
        if line.read_if(b"ottd") {
            #[allow(clippy::cast_sign_loss)]
            if with(|c| c.http_response_index as usize) >= size {
                return None;
            }
            continue;
        }

        line.skip_until(b'/');
        // Keep only the last part; there must be at least one slash.
        let mut filename;
        loop {
            if !line.read_if(b"/") {
                return None;
            }
            filename = line.read_until(b'/');
            if line.left().is_empty() {
                break;
            }
        }

        // Remove the two extensions.
        for _ in 0..2 {
            let pos = filename.iter().rposition(|&b| b == b'.')?;
            filename = &filename[..pos];
        }

        unsafe { append((s.filename)(ci), filename) };

        // Request the next file.
        if !before_download(s) {
            return None;
        }

        (s.http_get)(ByteSpan {
            data: url.as_ptr(),
            size: url.len(),
        });
        return Some(());
    }
}

fn download_content_info(s: &Services, cid: u32) {
    // Content already tried is not requested again.
    if with(|c| {
        if c.requested.contains(&cid) {
            return true;
        }
        c.requested.push(cid);
        c.queued.push(cid);
        false
    }) {
        return;
    }
    (s.queue_reset)();
}

fn request_queued_content_info(s: &Services) {
    if with(|c| c.queued.is_empty()) {
        return;
    }

    // Wait until data briefly stops arriving before making the request.
    if (s.tcp.now)() <= with(|c| c.last_activity).wrapping_add(CONTENT_QUEUE_TIMEOUT) {
        (s.queue_reset)();
        return;
    }

    // Take the queue so more IDs can be queued, then drop IDs received since.
    let queue = with(|c| {
        let mut queue = std::mem::take(&mut c.queued);
        queue.retain(|&id| !c.infos.iter().any(|&ci| info(ci).id == id));
        queue
    });

    request_content_list_ids(s, &queue);
}

fn select(s: &Services, cid: u32) {
    let ci = get_content(cid);
    if ci.is_null() || info(ci).state != state::UNSELECTED {
        return;
    }
    info_mut(ci).state = state::SELECTED;
    check_dependency_state(s, ci);
}

fn unselect(s: &Services, cid: u32) {
    let ci = get_content(cid);
    if ci.is_null() || !info(ci).is_selected() {
        return;
    }
    info_mut(ci).state = state::UNSELECTED;
    check_dependency_state(s, ci);
}

/// `SelectAll` and `SelectUpgrade`.
fn select_where(s: &Services, upgrade_only: bool) {
    let mut i = 0;
    while let Some(ci) = with(|c| c.infos.get(i).copied()) {
        i += 1;
        if info(ci).state == state::UNSELECTED && (!upgrade_only || info(ci).upgrade) {
            info_mut(ci).state = state::SELECTED;
            check_dependency_state(s, ci);
        }
    }
}

fn reverse_lookup_dependency(parents: &mut Vec<*mut ContentInfo>, child: *const ContentInfo) {
    let id = info(child).id;
    let ids = with(|c| c.reverse.get(&id).cloned().unwrap_or_default());
    for parent in ids.into_iter().rev() {
        parents.push(get_content(parent));
    }
}

fn reverse_lookup_tree_dependency(child: *mut ContentInfo) -> Vec<*mut ContentInfo> {
    let mut tree = vec![child];
    // Indices, since the tree grows while it is walked.
    let mut i = 0;
    while i < tree.len() {
        let mut parents = Vec::new();
        reverse_lookup_dependency(&mut parents, tree[i]);
        for ci in parents {
            if !tree.contains(&ci) {
                tree.push(ci);
            }
        }
        i += 1;
    }
    tree
}

fn dependency_at(s: &Services, ci: *const ContentInfo, index: usize) -> Option<u32> {
    let span = (s.dependencies)(ci);
    // SAFETY: The native vector is live and unmodified for this read.
    (index < span.size).then(|| unsafe { *span.data.add(index) })
}

fn check_dependency_state(s: &Services, ci: *mut ContentInfo) {
    if info(ci).is_selected() || info(ci).state == state::ALREADY_HERE {
        // Mark unselected children autoselected, so removing a dependency can unselect them.
        let mut k = 0;
        while let Some(dependency) = dependency_at(s, ci, k) {
            k += 1;
            let c = get_content(dependency);
            if c.is_null() {
                download_content_info(s, dependency);
            } else if info(c).state == state::UNSELECTED {
                info_mut(c).state = state::AUTOSELECTED;
                check_dependency_state(s, c);
            }
        }
        return;
    }

    if info(ci).state != state::UNSELECTED {
        return;
    }

    // Unselect the parents, then the autoselected children only this content needs.
    let mut parents = Vec::new();
    reverse_lookup_dependency(&mut parents, ci);
    for &c in &parents {
        if !info(c).is_selected() {
            continue;
        }
        unselect(s, info(c).id);
    }

    let mut k = 0;
    while let Some(dependency) = dependency_at(s, ci, k) {
        k += 1;
        let c = get_content(dependency);
        if c.is_null() {
            download_content_info(s, dependency);
            continue;
        }
        if info(c).state != state::AUTOSELECTED {
            continue;
        }

        // Only unselect when this content is the only parent.
        parents.clear();
        reverse_lookup_dependency(&mut parents, c);

        let mut sel_count = 0;
        let mut force_selection = false;
        for &parent_ci in &parents {
            if info(parent_ci).is_selected() {
                sel_count += 1;
            }
            if info(parent_ci).state == state::SELECTED {
                force_selection = true;
            }
        }
        if sel_count == 0 {
            // Nothing depends on it.
            unselect(s, info(c).id);
            continue;
        }
        // Something manually selected depends directly on it.
        if force_selection {
            continue;
        }

        // Flood the dependency graph.
        parents = reverse_lookup_tree_dependency(c);

        // Anything manually selected keeps the graph.
        if parents
            .iter()
            .any(|&parent_ci| info(parent_ci).state == state::SELECTED)
        {
            continue;
        }

        // Unselect the whole graph first, then check the children of each member.
        for &parent in &parents {
            if info(parent).state == state::AUTOSELECTED {
                unselect(s, info(parent).id);
            }
        }
        for &parent in &parents {
            check_dependency_state(s, get_content(info(parent).id));
        }
    }
}

/// Construction of `_network_content_client`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_init(s: &Services) {
    let now = (s.tcp.now)();
    with(|c| c.last_activity = now);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_connect(s: &Services) {
    connect(s);
}

/// `NetworkContentConnecter`; on success the native socket is already open.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_connected(s: &Services, success: bool) {
    if success {
        let now = (s.tcp.now)();
        with(|c| c.last_activity = now);
    }
    with(|c| c.is_connecting = false);
    on_connect(s, success);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_close(s: &Services) {
    close_connection(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_cancel(s: &Services) {
    CANCELLED.store(true, Ordering::Relaxed);
    close_connection(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_send_receive(s: &Services) {
    if !(s.tcp.valid)(s.client) || with(|c| c.is_connecting) {
        return;
    }

    // Close the connection after inactivity; HTTP downloads can still be pending.
    if (s.tcp.now)() > with(|c| c.last_activity).wrapping_add(IDLE_TIMEOUT) {
        close_connection(s);
        return;
    }

    // Activity counts once a packet arrives, not on every attempt.
    if (s.tcp.can_send_receive)(s.client) && receive_packets(s) {
        let now = (s.tcp.now)();
        with(|c| c.last_activity = now);
    }

    if unsafe {
        tcp::send_packets(
            tcp::Context {
                services: s.tcp,
                socket: s.client,
            },
            false,
            check,
        )
    } == tcp::Send::Close
    {
        close_connection(s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_request_type(s: &Services, kind: u8) {
    request_content_list_type(s, kind);
}

/// # Safety
/// `ids` holds `count` readable IDs for the call; it may be null when empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_content_request_ids(
    s: &Services,
    ids: *const u32,
    count: usize,
) {
    let ids = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(ids, count) }
    };
    request_content_list_ids(s, ids);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_request_vector(
    s: &Services,
    cv: *mut ContentVector,
    send_md5sum: bool,
) {
    request_content_list_vector(s, cv, send_md5sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_download(
    s: &Services,
    files: &mut u32,
    bytes: &mut u32,
    fallback: bool,
) {
    download_selected_content(s, files, bytes, fallback);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_request_queued(s: &Services) {
    request_queued_content_info(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_select(s: &Services, cid: u32) {
    select(s, cid);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_unselect(s: &Services, cid: u32) {
    unselect(s, cid);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_select_all(s: &Services, upgrade_only: bool) {
    select_where(s, upgrade_only);
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_unselect_all() {
    with(|c| {
        for &ci in &c.infos {
            let ci = info_mut(ci);
            if ci.is_selected() && ci.state != state::ALREADY_HERE {
                ci.state = state::UNSELECTED;
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_toggle(s: &Services, ci: *const ContentInfo) {
    match info(ci).state {
        state::SELECTED | state::AUTOSELECTED => unselect(s, info(ci).id),
        state::UNSELECTED => select(s, info(ci).id),
        _ => {}
    }
}

/// `ReverseLookupTreeDependency` into a native vector; its only caller passes it empty.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_tree(
    s: &Services,
    tree: *mut ConstContentVector,
    child: *const ContentInfo,
) {
    for ci in reverse_lookup_tree_dependency(child.cast_mut()) {
        (s.tree_push)(tree, ci);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_clear(s: &Services) {
    let infos = with(|c| {
        c.requested.clear();
        c.queued.clear();
        c.reverse.clear();
        std::mem::take(&mut c.infos)
    });
    for ci in infos {
        (s.info_delete)(ci);
    }
}

/// The received records in order, for native iteration until the next mutation.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_infos(count: &mut usize) -> *const *mut ContentInfo {
    with(|c| {
        *count = c.infos.len();
        c.infos.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_add_callback(cb: *mut ContentCallback) {
    with(|c| {
        if !c.callbacks.contains(&cb) {
            c.callbacks.push(cb);
        }
    });
}

/// `callbacks.erase(find(cb))`; libstdc++ erasing `end()` drops the last element.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_remove_callback(cb: *mut ContentCallback) {
    with(|c| match c.callbacks.iter().position(|&x| x == cb) {
        Some(index) => {
            c.callbacks.remove(index);
        }
        None => {
            c.callbacks.pop();
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_http_failure(s: &Services) {
    on_failure(s);
}

/// # Safety
/// Non-null `data` holds `length` readable bytes for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_content_http_data(
    s: &Services,
    data: *const u8,
    length: usize,
) {
    check(data.is_null() || length != 0);
    let bytes = if data.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, length) }
    };
    on_receive_data(s, bytes, !data.is_null());
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_content_http_cancelled() -> u8 {
    u8::from(CANCELLED.load(Ordering::Relaxed))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDContentServices" ["probe"] => Services {
        callback_complete, callback_connect, callback_disconnect, callback_progress,
        callback_receive, client, close_socket, connect, content_version, dependencies,
        dependency_push, description, error_extract, error_not_writable, extract, file_close,
        file_create, file_open, file_write, filename, has_content, http_get, http_mirror,
        info_assign, info_delete, info_new, list_get, list_release, list_size, name,
        queue_reset, string_assign,
        string_empty, tag, tcp, tcp_close, tree_push, url, version,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &[u8]) -> Line<'_> {
        Line {
            src: text,
            position: 0,
        }
    }

    #[test]
    fn response_line_reads_follow_string_consumer() {
        let mut l = line(b"12,3,40,http://h/a/b.tar.gz");
        assert_eq!(l.read_integer(32), 12);
        assert!(l.read_if(b","));
        assert_eq!(l.read_integer(32), 3);
        assert!(l.read_if(b","));
        assert_eq!(l.read_integer(32), 40);
        assert!(l.read_if(b","));
        assert_eq!(l.left(), b"http://h/a/b.tar.gz");
        l.skip_until(b'/');
        assert!(l.read_if(b"/"));
        assert_eq!(l.read_until(b'/'), b"");
        assert!(l.read_if(b"/"));
        assert_eq!(l.read_until(b'/'), b"h");
    }

    #[test]
    fn id_packets_split_at_the_original_capacity() {
        assert_eq!(MAX_CONTENT_IDS_PER_PACKET, 8190);
    }
}

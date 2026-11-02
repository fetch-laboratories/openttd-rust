/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Authentication handlers, method fallback, sessions and incremental encryption.
//! Calls are serialized. Native providers and packet services remain live for
//! their handler. No Rust borrow spans a callback; password requests may reenter
//! the active handler synchronously. Panics abort and callbacks cannot unwind.
#![allow(unsafe_code)]

use crate::{
    crypto::{self, Aead},
    packet,
};
use std::ptr;
use zeroize::{Zeroize, Zeroizing};

pub(crate) struct NetworkAuthenticationPasswordProvider {
    _private: [u8; 0],
}
pub(crate) struct NetworkAuthenticationAuthorizedKeyHandler {
    _private: [u8; 0],
}
pub(crate) struct RustAuthenticationClientHandler {
    _private: [u8; 0],
}

#[repr(C)]
pub(crate) struct Services {
    allowed: unsafe extern "C" fn(
        *const NetworkAuthenticationAuthorizedKeyHandler,
        *const u8,
        usize,
    ) -> u8,
    ask_password: unsafe extern "C" fn(*mut RustAuthenticationClientHandler),
    authorized_available:
        unsafe extern "C" fn(*const NetworkAuthenticationAuthorizedKeyHandler) -> u8,
    client_password:
        unsafe extern "C" fn(*const RustAuthenticationClientHandler, *mut usize) -> *const u8,
    illegal_public_key: unsafe extern "C" fn(),
    illegal_size: unsafe extern "C" fn(),
    packet: *const packet::Services,
    random_bytes: unsafe extern "C" fn(*mut u8, usize),
    received_request: unsafe extern "C" fn(*const u8, usize),
    receiving_response: unsafe extern "C" fn(*const u8, usize),
    sending_request: unsafe extern "C" fn(*const u8, usize),
    sending_response: unsafe extern "C" fn(*const u8, usize),
    server_password:
        unsafe extern "C" fn(*const NetworkAuthenticationPasswordProvider, *mut usize) -> *const u8,
}

unsafe fn bytes<'a>(data: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, length) }
    }
}
unsafe fn bytes_mut<'a>(data: *mut u8, length: usize) -> &'a mut [u8] {
    if length == 0 {
        &mut []
    } else {
        unsafe { std::slice::from_raw_parts_mut(data, length) }
    }
}

#[derive(Clone)]
struct Session {
    secret: [u8; 32],
    public: [u8; 32],
    exchange_nonce: [u8; 24],
    keys: [u8; 64],
    peer: [u8; 32],
    encryption_nonce: [u8; 24],
}
impl Drop for Session {
    fn drop(&mut self) {
        self.encryption_nonce.zeroize();
        self.peer.zeroize();
        self.keys.zeroize();
        self.exchange_nonce.zeroize();
        self.public.zeroize();
        self.secret.zeroize();
    }
}

#[derive(Clone)]
struct Single {
    session: Session,
    method: u8,
    server: bool,
    client_password: *mut RustAuthenticationClientHandler,
    server_password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)] // Each original single handler embeds fixed key and nonce state.
enum Kind {
    Single(Single),
    Client {
        handlers: Vec<Handler>,
        current: Option<usize>,
    },
    Server {
        handlers: Vec<Handler>,
    },
}
#[derive(Clone)]
pub(crate) struct Handler {
    services: *const Services,
    kind: Kind,
}
pub(crate) struct Stream {
    context: Aead,
}

impl Handler {
    unsafe fn single(&self) -> &Single {
        match &self.kind {
            Kind::Single(single) => single,
            Kind::Client { handlers, current } => unsafe { handlers[current.unwrap()].single() },
            Kind::Server { handlers } => unsafe { handlers.last().unwrap().single() },
        }
    }
    unsafe fn active(owner: *mut Self) -> *mut Self {
        match unsafe { &mut (*owner).kind } {
            Kind::Single(_) => owner,
            Kind::Client { handlers, current } => unsafe {
                Self::active(&raw mut handlers[current.unwrap()])
            },
            Kind::Server { handlers } => unsafe { Self::active(handlers.last_mut().unwrap()) },
        }
    }
    fn name(&self) -> &'static str {
        match &self.kind {
            Kind::Single(single) => match (single.method, single.server) {
                (0, false) => "X25519-KeyExchangeOnly-client",
                (0, true) => "X25519-KeyExchangeOnly-server",
                (1, false) => "X25519-PAKE-client",
                (1, true) => "X25519-PAKE-server",
                (2, false) => "X25519-AuthorizedKey-client",
                (2, true) => "X25519-AuthorizedKey-server",
                _ => unreachable!(),
            },
            Kind::Client { handlers, current } => current.map_or("Unknown", |i| handlers[i].name()),
            Kind::Server { handlers } => handlers.last().map_or("Unknown", Self::name),
        }
    }
    fn method(&self) -> u8 {
        match &self.kind {
            Kind::Single(single) => single.method,
            Kind::Client { handlers, current } => current.map_or(3, |i| handlers[i].method()),
            Kind::Server { handlers } => handlers.last().map_or(3, Self::method),
        }
    }
}

unsafe fn single_mut<'a>(owner: *mut Handler) -> &'a mut Single {
    let Kind::Single(single) = (unsafe { &mut (*owner).kind }) else {
        unreachable!();
    };
    single
}

unsafe fn session_new(secret: &[u8; 32], services: *const Services) -> Session {
    let mut session = Session {
        secret: *secret,
        public: crypto::public_key(secret),
        exchange_nonce: [0; 24],
        keys: [0; 64],
        peer: [0; 32],
        encryption_nonce: [0; 24],
    };
    unsafe {
        ((*services).random_bytes)(session.exchange_nonce.as_mut_ptr(), 24);
        ((*services).random_bytes)(session.encryption_nonce.as_mut_ptr(), 24);
    }
    session
}

#[allow(clippy::too_many_arguments)] // Typed constructor inputs follow the native handler interfaces.
unsafe fn new_single(
    services: *const Services,
    method: u8,
    server: bool,
    secret: &[u8; 32],
    client_password: *mut RustAuthenticationClientHandler,
    server_password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
) -> Handler {
    Handler {
        services,
        kind: Kind::Single(Single {
            session: unsafe { session_new(secret, services) },
            method,
            server,
            client_password,
            server_password,
            authorized,
        }),
    }
}

unsafe fn can_be_used(owner: *const Handler) -> bool {
    match unsafe { &(*owner).kind } {
        Kind::Single(single) => unsafe {
            match single.method {
                0 => true,
                1 => {
                    let mut size = 0;
                    ((*(*owner).services).server_password)(single.server_password, &raw mut size);
                    size != 0
                }
                2 => ((*(*owner).services).authorized_available)(single.authorized) != 0,
                _ => unreachable!(),
            }
        },
        Kind::Server { handlers } => !handlers.is_empty(),
        Kind::Client { .. } => unreachable!(),
    }
}

unsafe fn log(owner: *const Handler, callback: unsafe extern "C" fn(*const u8, usize)) {
    let name = unsafe { (*owner).name() };
    unsafe {
        callback(name.as_ptr(), name.len());
    }
}

unsafe fn receive_request(owner: *mut Handler, packet: *mut packet::State) -> u8 {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Client { .. }) {
        let method = unsafe { packet::recv_uint8(packet, &*(*services).packet) };
        let Kind::Client { handlers, current } = (unsafe { &mut (*owner).kind }) else {
            unreachable!();
        };
        let Some(index) = handlers
            .iter()
            .position(|handler| handler.method() == method)
        else {
            return 2;
        };
        *current = Some(index);
        unsafe {
            log(owner, (*services).received_request);
        }
        let active = unsafe { Handler::active(owner) };
        return unsafe { receive_request(active, packet) };
    }
    if unsafe { (*packet).remaining() } != 56 {
        unsafe {
            ((*services).illegal_size)();
        }
        return 2;
    }
    unsafe {
        let single = single_mut(owner);
        (*packet).recv_bytes(&mut single.session.peer);
        (*packet).recv_bytes(&mut single.session.exchange_nonce);
    }
    let (method, password_handler) = unsafe {
        let single = single_mut(owner);
        (single.method, single.client_password)
    };
    if method == 1 {
        unsafe {
            ((*services).ask_password)(password_handler);
        }
        0
    } else {
        1
    }
}

unsafe fn send_response(owner: *mut Handler, packet: *mut packet::State) -> bool {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Client { .. }) {
        unsafe {
            log(owner, (*services).sending_response);
        }
        return unsafe { send_response(Handler::active(owner), packet) };
    }
    let mut size = 0;
    let password = if unsafe { (*owner).method() } == 1 {
        let handler = unsafe { single_mut(owner).client_password };
        unsafe { ((*services).client_password)(handler, &raw mut size) }
    } else {
        ptr::null()
    };
    let valid = unsafe {
        let single = single_mut(owner);
        crypto::derive(
            &mut single.session.keys,
            &single.session.secret,
            &single.session.public,
            &single.session.peer,
            false,
            bytes(password, size),
        )
    };
    if !valid {
        unsafe {
            ((*services).illegal_public_key)();
        }
        return false;
    }
    let mut message = Zeroizing::new([0; 8]);
    unsafe {
        ((*services).random_bytes)(message.as_mut_ptr(), 8);
    }
    unsafe {
        let single = single_mut(owner);
        let mut context = Aead::new(
            single.session.keys[..32].try_into().unwrap(),
            &single.session.exchange_nonce,
        );
        let mac = context.write(message.as_mut(), &single.session.public);
        (*packet).send_bytes(&single.session.public);
        (*packet).send_bytes(&mac);
        (*packet).send_bytes(message.as_ref());
    }
    true
}

unsafe fn receive_response(owner: *mut Handler, packet: *mut packet::State) -> u8 {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Server { .. }) {
        unsafe {
            log(owner, (*services).receiving_response);
        }
        let result = unsafe { receive_response(Handler::active(owner), packet) };
        if result != 1 {
            return result;
        }
        let Kind::Server { handlers } = (unsafe { &mut (*owner).kind }) else {
            unreachable!();
        };
        handlers.pop();
        return if handlers.is_empty() { 1 } else { 2 };
    }
    // The original PAKE handler queries the provider before checking packet size.
    let mut size = 0;
    let password = if unsafe { (*owner).method() } == 1 {
        let provider = unsafe { single_mut(owner).server_password };
        unsafe { ((*services).server_password)(provider, &raw mut size) }
    } else {
        ptr::null()
    };
    if unsafe { (*packet).remaining() } != 56 {
        unsafe {
            ((*services).illegal_size)();
        }
        return 1;
    }
    let mut message = Zeroizing::new([0; 8]);
    let mut mac = [0; 16];
    let valid = unsafe {
        let single = single_mut(owner);
        (*packet).recv_bytes(&mut single.session.peer);
        (*packet).recv_bytes(&mut mac);
        (*packet).recv_bytes(message.as_mut());
        crypto::derive(
            &mut single.session.keys,
            &single.session.secret,
            &single.session.public,
            &single.session.peer,
            true,
            bytes(password, size),
        )
    };
    if !valid {
        unsafe {
            ((*services).illegal_public_key)();
        }
        return 1;
    }
    let valid = unsafe {
        let single = single_mut(owner);
        Aead::new(
            single.session.keys[..32].try_into().unwrap(),
            &single.session.exchange_nonce,
        )
        .read(message.as_mut(), &mac, &single.session.peer)
    };
    if !valid {
        return 1;
    }
    if unsafe { (*owner).method() } == 2 {
        let (authorized, peer) = unsafe {
            let single = single_mut(owner);
            (single.authorized, single.session.peer.as_ptr())
        };
        let mut encoded = [0; 64];
        unsafe {
            crate::byte_strings::openttd_rust_bytes_hex_encode(peer, 32, encoded.as_mut_ptr());
        }
        if unsafe { ((*services).allowed)(authorized, encoded.as_ptr(), encoded.len()) } == 0 {
            return 1;
        }
    }
    0
}

// Entries share the module contract. Native success values are 0-or-1 u8 (#644).
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_single_new(
    services: *const Services,
    method: u8,
    server: u8,
    secret: *const u8,
    client_password: *mut RustAuthenticationClientHandler,
    server_password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
) -> *mut Handler {
    Box::into_raw(Box::new(unsafe {
        new_single(
            services,
            method,
            server != 0,
            &*secret.cast::<[u8; 32]>(),
            client_password,
            server_password,
            authorized,
        )
    }))
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_client_new(
    services: *const Services,
    secret: *const u8,
    password: *mut RustAuthenticationClientHandler,
) -> *mut Handler {
    let mut handlers = Vec::new();
    for method in 0..3 {
        handlers.push(unsafe {
            new_single(
                services,
                method,
                false,
                &*secret.cast::<[u8; 32]>(),
                password,
                ptr::null(),
                ptr::null(),
            )
        });
    }
    Box::into_raw(Box::new(Handler {
        services,
        kind: Kind::Client {
            handlers,
            current: None,
        },
    }))
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_server_new(
    services: *const Services,
    password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
    mask: u16,
) -> *mut Handler {
    let mut secret = Zeroizing::new([0; 32]);
    unsafe {
        ((*services).random_bytes)(secret.as_mut_ptr(), 32);
    }
    let mut handlers = Vec::new();
    if !password.is_null() && mask & 2 != 0 {
        let handler = unsafe {
            new_single(
                services,
                1,
                true,
                &secret,
                ptr::null_mut(),
                password,
                ptr::null(),
            )
        };
        if unsafe { can_be_used(&raw const handler) } {
            handlers.push(handler);
        }
    }
    if !authorized.is_null() && mask & 4 != 0 {
        let handler = unsafe {
            new_single(
                services,
                2,
                true,
                &secret,
                ptr::null_mut(),
                ptr::null(),
                authorized,
            )
        };
        if unsafe { can_be_used(&raw const handler) } {
            handlers.push(handler);
        }
    }
    if handlers.is_empty() && mask & 1 != 0 {
        handlers.push(unsafe {
            new_single(
                services,
                0,
                true,
                &secret,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
            )
        });
    }
    Box::into_raw(Box::new(Handler {
        services,
        kind: Kind::Server { handlers },
    }))
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_clone(
    owner: *const Handler,
) -> *mut Handler {
    Box::into_raw(Box::new(unsafe { (*owner).clone() }))
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_assign(
    dst: *mut Handler,
    src: *const Handler,
) {
    if !ptr::eq(dst, src) {
        unsafe {
            (*dst).clone_from(&*src);
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_destroy(owner: *mut Handler) {
    unsafe {
        drop(Box::from_raw(owner));
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_name(
    owner: *const Handler,
    size: *mut usize,
) -> *const u8 {
    let name = unsafe { (*owner).name() };
    unsafe {
        *size = name.len();
    }
    name.as_ptr()
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_method(owner: *const Handler) -> u8 {
    unsafe { (*owner).method() }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_peer(owner: *const Handler) -> *const u8 {
    unsafe { (*owner).single().session.peer.as_ptr() }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_can_be_used(owner: *const Handler) -> u8 {
    u8::from(unsafe { can_be_used(owner) })
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_receive_request(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> u8 {
    unsafe { receive_request(owner, packet) }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_send_response(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> u8 {
    u8::from(unsafe { send_response(owner, packet) })
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_receive_response(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> u8 {
    unsafe { receive_response(owner, packet) }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_send_request(
    owner: *mut Handler,
    packet: *mut packet::State,
    check: extern "C" fn(bool),
) {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Server { .. }) {
        unsafe {
            log(owner, (*services).sending_request);
            (*packet).send_uint8((*owner).method(), check);
        }
    }
    unsafe {
        let active = Handler::active(owner);
        let session = &single_mut(active).session;
        (*packet).send_bytes(&session.public);
        (*packet).send_bytes(&session.exchange_nonce);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_send_enable(
    owner: *mut Handler,
    packet: *mut packet::State,
) {
    unsafe {
        let session = &single_mut(Handler::active(owner)).session;
        (*packet).send_bytes(&session.encryption_nonce);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_receive_enable(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> u8 {
    unsafe {
        let session = &mut single_mut(Handler::active(owner)).session;
        u8::from((*packet).recv_bytes(&mut session.encryption_nonce) == 24)
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_new(
    owner: *const Handler,
    side: u8,
) -> *mut Stream {
    unsafe {
        let session = &(*owner).single().session;
        Box::into_raw(Box::new(Stream {
            context: Aead::new(
                session.keys[usize::from(side) * 32..usize::from(side) * 32 + 32]
                    .try_into()
                    .unwrap(),
                &session.encryption_nonce,
            ),
        }))
    }
}

unsafe fn allocate<T>() -> *mut T {
    let layout = std::alloc::Layout::new::<T>();
    let result = unsafe { std::alloc::alloc_zeroed(layout) }.cast::<T>();
    if result.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    result
}
unsafe fn free<T>(owner: *mut T) {
    unsafe {
        ptr::drop_in_place(owner);
        std::alloc::dealloc(owner.cast(), std::alloc::Layout::new::<T>());
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_clone(src: *const Stream) -> *mut Stream {
    unsafe {
        let dst = allocate::<Stream>();
        ptr::addr_of_mut!((*dst).context).write((*src).context.clone());
        dst
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_assign(
    dst: *mut Stream,
    src: *const Stream,
) {
    unsafe {
        if !ptr::eq(dst, src) {
            (*dst).context.clone_from(&(*src).context);
        }
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_destroy(owner: *mut Stream) {
    unsafe {
        free(owner);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_encrypt(
    owner: *mut Stream,
    mac: *mut u8,
    message: *mut u8,
    length: usize,
) {
    assert!(isize::try_from(length).is_ok());
    unsafe {
        let tag = (*owner).context.write(bytes_mut(message, length), &[]);
        ptr::copy_nonoverlapping(tag.as_ptr(), mac, 16);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_decrypt(
    owner: *mut Stream,
    mac: *const u8,
    message: *mut u8,
    length: usize,
) -> u8 {
    assert!(isize::try_from(length).is_ok());
    unsafe {
        u8::from(
            (*owner)
                .context
                .read(bytes_mut(message, length), &*mac.cast::<[u8; 16]>(), &[]),
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAuthHandlerServices" ["probe"] => Services {
        allowed, ask_password, authorized_available, client_password,
        illegal_public_key, illegal_size, packet, random_bytes, received_request,
        receiving_response, sending_request, sending_response, server_password,
    };
}

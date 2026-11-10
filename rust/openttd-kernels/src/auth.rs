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

use crate::debug::debug;
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
    packet: *const packet::Services,
    random_bytes: unsafe extern "C" fn(*mut u8, usize),
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

struct Single {
    session: Session,
    method: u8,
    server: bool,
    client_password: *mut RustAuthenticationClientHandler,
    server_password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
}

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
pub(crate) struct Handler {
    services: *const Services,
    kind: Kind,
}
pub(crate) struct Stream {
    context: Aead,
}

impl Stream {
    /// `NetworkEncryptionHandler::Encrypt`: encrypt the message and then write its MAC.
    /// # Safety
    /// `mac` is writable for the native 16-byte authentication tag.
    pub(crate) unsafe fn encrypt(&mut self, mac: *mut u8, message: &mut [u8]) {
        let tag = self.context.write(message, &[]);
        unsafe { ptr::copy_nonoverlapping(tag.as_ptr(), mac, 16) };
    }

    /// `NetworkEncryptionHandler::Decrypt`: authenticate and decrypt the message.
    pub(crate) fn decrypt(&mut self, mac: &[u8; 16], message: &mut [u8]) -> bool {
        self.context.read(message, mac, &[])
    }
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
        debug!(net, 9, "Received {} authentication request", unsafe {
            (*owner).name()
        });
        let active = unsafe { Handler::active(owner) };
        return unsafe { receive_request(active, packet) };
    }
    if unsafe { (*packet).remaining() } != 56 {
        debug!(
            net,
            1, "[crypto] Received auth response of illegal size; authentication aborted."
        );
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
        debug!(net, 9, "Sending {} authentication response", unsafe {
            (*owner).name()
        });
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
        debug!(
            net,
            0, "[crypto] Peer sent an illegal public key; authentication aborted."
        );
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
        debug!(net, 9, "Receiving {} authentication response", unsafe {
            (*owner).name()
        });
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
        debug!(
            net,
            1, "[crypto] Received auth response of illegal size; authentication aborted."
        );
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
        debug!(
            net,
            0, "[crypto] Peer sent an illegal public key; authentication aborted."
        );
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
            // SAFETY: The live fixed-size public key is disjoint from encoded output.
            crate::byte_strings::format_array_as_hex(
                std::slice::from_raw_parts(peer, 32),
                &mut encoded,
            );
        }
        if unsafe { ((*services).allowed)(authorized, encoded.as_ptr(), encoded.len()) } == 0 {
            return 1;
        }
    }
    0
}

// Entries share the module contract. Native success values are 0-or-1 u8 (#644).
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
    if matches!(unsafe { &(*owner).kind }, Kind::Server { .. }) {
        unsafe {
            debug!(net, 9, "Sending {} authentication request", (*owner).name());
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

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_destroy(owner: *mut Stream) {
    unsafe {
        drop(Box::from_raw(owner));
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
        (*owner).encrypt(mac, bytes_mut(message, length));
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
    unsafe { u8::from((*owner).decrypt(&*mac.cast::<[u8; 16]>(), bytes_mut(message, length))) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAuthHandlerServices" ["probe"] => Services {
        allowed, ask_password, authorized_available, client_password, packet, random_bytes,
        server_password,
    };
}

#[cfg(test)]
mod tests {
    use super::{
        Handler, NetworkAuthenticationAuthorizedKeyHandler, NetworkAuthenticationPasswordProvider,
        RustAuthenticationClientHandler, Services, new_single,
        openttd_rust_auth_handler_send_request, receive_request, receive_response, send_response,
    };
    use crate::packet;
    use std::ptr;

    extern "C" fn unused_allowed(
        _: *const NetworkAuthenticationAuthorizedKeyHandler,
        _: *const u8,
        _: usize,
    ) -> u8 {
        unreachable!()
    }
    extern "C" fn unused_available(_: *const NetworkAuthenticationAuthorizedKeyHandler) -> u8 {
        unreachable!()
    }
    extern "C" fn ask(_: *mut RustAuthenticationClientHandler) {}
    /// Fixture providers point at a `&[u8]` password.
    extern "C" fn client_password(
        handler: *const RustAuthenticationClientHandler,
        size: *mut usize,
    ) -> *const u8 {
        let password = unsafe { *handler.cast::<&[u8]>() };
        unsafe { size.write(password.len()) };
        password.as_ptr()
    }
    extern "C" fn server_password(
        provider: *const NetworkAuthenticationPasswordProvider,
        size: *mut usize,
    ) -> *const u8 {
        let password = unsafe { *provider.cast::<&[u8]>() };
        unsafe { size.write(password.len()) };
        password.as_ptr()
    }
    extern "C" fn random(data: *mut u8, size: usize) {
        let bytes = unsafe { std::slice::from_raw_parts_mut(data, size) };
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = i.to_le_bytes()[0].wrapping_mul(37).wrapping_add(11);
        }
    }
    extern "C" fn check(condition: bool) {
        assert!(condition);
    }

    /// Upstream `Authentication_PAKE` sections on the single PAKE handlers, including the
    /// empty server password that the production server factory never offers.
    fn pake(server: &'static [u8], client: &'static [u8]) -> u8 {
        let services = Services {
            allowed: unused_allowed,
            ask_password: ask,
            authorized_available: unused_available,
            client_password,
            packet: ptr::null(),
            random_bytes: random,
            server_password,
        };
        let server_password: &[u8] = server;
        let client_password: &[u8] = client;
        let new = |is_server: bool, secret: u8| -> Handler {
            unsafe {
                new_single(
                    &raw const services,
                    1,
                    is_server,
                    &[secret; 32],
                    (&raw const client_password).cast_mut().cast(),
                    (&raw const server_password).cast(),
                    ptr::null(),
                )
            }
        };
        let mut server = new(true, 1);
        let mut client = new(false, 2);
        let mut request = packet::State::new(1460, ptr::null_mut());
        let mut response = packet::State::new(1460, ptr::null_mut());
        unsafe {
            openttd_rust_auth_handler_send_request(&raw mut server, &raw mut request, check);
            // RequestResult::AwaitUserInput.
            assert_eq!(receive_request(&raw mut client, &raw mut request), 0);
            assert!(send_response(&raw mut client, &raw mut response));
            receive_response(&raw mut server, &raw mut response)
        }
    }

    #[test]
    fn pake_handlers_match_upstream_sections() {
        if crate::services_assert::isolate_test_check(
            "auth::tests::pake_handlers_match_upstream_sections",
            check,
        ) {
            return;
        }
        // ResponseResult::Authenticated, then NotAuthenticated.
        assert_eq!(pake(b"", b""), 0);
        assert_eq!(pake(b"sikrit", b"sikrit"), 0);
        assert_eq!(pake(b"sikrit", b"secret"), 1);
        assert_eq!(pake(b"", b"secret"), 1);
    }
}

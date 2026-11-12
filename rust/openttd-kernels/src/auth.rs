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
use crate::network_crypto::{
    NetworkAuthenticationMethod, NetworkAuthenticationMethodMask, RequestResult, ResponseResult,
};
use crate::network_crypto_internal::{
    X25519_KEY_EXCHANGE_MESSAGE_SIZE, X25519_KEY_SIZE, X25519_MAC_SIZE, X25519_NONCE_SIZE,
    X25519DerivedKeys, X25519KeyExchangeMessage, X25519KeyExchangeSide, X25519Mac, X25519Nonce,
    X25519PublicKey, X25519SecretKey, client_to_server, create_random_nonce,
    create_random_secret_key, server_to_client,
};
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

/// `X25519AuthenticationHandler`: both peers exchange public keys and derive two
/// `BLAKE2b` keys from the shared secret, server public key, client public key and
/// optional password, in that order. The first key encrypts client-to-server
/// traffic; the second encrypts server-to-client traffic.
///
/// The server sends its nonce: admin-protocol clients have many implementations,
/// so the server cannot rely on them to generate a sufficiently good nonce.
/// It generates new keys for each session. The client encrypts eight random bytes
/// with XChaCha20-Poly1305, using its public key as additional authenticated data,
/// and returns its public key, MAC and encrypted message. The server derives the
/// same keys, verifies the MAC and message, then performs the method-specific check.
struct Session {
    secret: X25519SecretKey,
    public: X25519PublicKey,
    exchange_nonce: X25519Nonce,
    keys: X25519DerivedKeys,
    peer: X25519PublicKey,
    encryption_nonce: X25519Nonce,
}
impl Drop for Session {
    fn drop(&mut self) {
        // Ensure the nonce does not get leaked when we are done with it.
        self.encryption_nonce.zeroize();
        // Ensure the key and derived keys do not get leaked when we are done.
        self.peer.zeroize();
        self.keys.zeroize();
        // Ensure the nonce does not get leaked when we are done with it.
        self.exchange_nonce.zeroize();
        self.public.zeroize();
        self.secret.zeroize();
    }
}

struct Single {
    session: Session,
    method: NetworkAuthenticationMethod,
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
        unsafe { ptr::copy_nonoverlapping(tag.as_ptr(), mac, X25519_MAC_SIZE) };
    }

    /// `NetworkEncryptionHandler::Decrypt`: authenticate and decrypt the message.
    pub(crate) fn decrypt(&mut self, mac: &X25519Mac, message: &mut [u8]) -> bool {
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
                (NetworkAuthenticationMethod::X25519_KeyExchangeOnly, false) => {
                    "X25519-KeyExchangeOnly-client"
                }
                (NetworkAuthenticationMethod::X25519_KeyExchangeOnly, true) => {
                    "X25519-KeyExchangeOnly-server"
                }
                (NetworkAuthenticationMethod::X25519_PAKE, false) => "X25519-PAKE-client",
                (NetworkAuthenticationMethod::X25519_PAKE, true) => "X25519-PAKE-server",
                (NetworkAuthenticationMethod::X25519_AuthorizedKey, false) => {
                    "X25519-AuthorizedKey-client"
                }
                (NetworkAuthenticationMethod::X25519_AuthorizedKey, true) => {
                    "X25519-AuthorizedKey-server"
                }
                _ => unreachable!(),
            },
            Kind::Client { handlers, current } => current.map_or("Unknown", |i| handlers[i].name()),
            Kind::Server { handlers } => handlers.last().map_or("Unknown", Self::name),
        }
    }
    /// Public key supplied by the peer; owner remains live across native adoption.
    pub(crate) fn peer(&self) -> *const u8 {
        unsafe { self.single().session.peer.as_ptr() }
    }

    pub(crate) fn method(&self) -> NetworkAuthenticationMethod {
        match &self.kind {
            Kind::Single(single) => single.method,
            Kind::Client { handlers, current } => {
                current.map_or(NetworkAuthenticationMethod::End, |i| handlers[i].method())
            }
            Kind::Server { handlers } => handlers
                .last()
                .map_or(NetworkAuthenticationMethod::End, Self::method),
        }
    }
}

unsafe fn single_mut<'a>(owner: *mut Handler) -> &'a mut Single {
    let Kind::Single(single) = (unsafe { &mut (*owner).kind }) else {
        unreachable!();
    };
    single
}

unsafe fn session_new(secret: &X25519SecretKey, services: *const Services) -> Session {
    let mut session = Session {
        secret: *secret,
        public: crypto::public_key(secret),
        exchange_nonce: [0; X25519_NONCE_SIZE],
        keys: [0; X25519_KEY_SIZE + X25519_KEY_SIZE],
        peer: [0; X25519_KEY_SIZE],
        encryption_nonce: [0; X25519_NONCE_SIZE],
    };
    unsafe {
        create_random_nonce(&mut session.exchange_nonce, |nonce| {
            ((*services).random_bytes)(nonce.as_mut_ptr(), X25519_NONCE_SIZE);
        });
        create_random_nonce(&mut session.encryption_nonce, |nonce| {
            ((*services).random_bytes)(nonce.as_mut_ptr(), X25519_NONCE_SIZE);
        });
    }
    session
}

/// Constructor inputs for one original authentication sub-handler.
#[derive(Clone, Copy)]
struct SingleArgs<'a> {
    method: NetworkAuthenticationMethod,
    server: bool,
    secret: &'a X25519SecretKey,
    client_password: *mut RustAuthenticationClientHandler,
    server_password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
}

unsafe fn new_single(services: *const Services, args: SingleArgs<'_>) -> Handler {
    let SingleArgs {
        method,
        server,
        secret,
        client_password,
        server_password,
        authorized,
    } = args;
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
                NetworkAuthenticationMethod::X25519_KeyExchangeOnly => true,
                NetworkAuthenticationMethod::X25519_PAKE => {
                    let mut size = 0;
                    ((*(*owner).services).server_password)(single.server_password, &raw mut size);
                    size != 0
                }
                NetworkAuthenticationMethod::X25519_AuthorizedKey => {
                    ((*(*owner).services).authorized_available)(single.authorized) != 0
                }
                _ => unreachable!(),
            }
        },
        Kind::Server { handlers } => !handlers.is_empty(),
        Kind::Client { .. } => unreachable!(),
    }
}

/// Read server key-exchange data, then request user input if needed.
/// # Safety
/// Handler, packet and synchronous leaves follow the module lifetime contract.
pub(crate) unsafe fn receive_request(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> RequestResult {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Client { .. }) {
        let method = NetworkAuthenticationMethod(unsafe {
            packet::recv_uint8(packet, &*(*services).packet)
        });
        let Kind::Client { handlers, current } = (unsafe { &mut (*owner).kind }) else {
            unreachable!();
        };
        let Some(index) = handlers
            .iter()
            .position(|handler| handler.method() == method)
        else {
            return RequestResult::Invalid;
        };
        *current = Some(index);
        debug!(net, 9, "Received {} authentication request", unsafe {
            (*owner).name()
        });
        let active = unsafe { Handler::active(owner) };
        return unsafe { receive_request(active, packet) };
    }
    if unsafe { (*packet).remaining() } != X25519_KEY_SIZE + X25519_NONCE_SIZE {
        debug!(
            net,
            1, "[crypto] Received auth response of illegal size; authentication aborted."
        );
        return RequestResult::Invalid;
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
    if method == NetworkAuthenticationMethod::X25519_PAKE {
        unsafe {
            ((*services).ask_password)(password_handler);
        }
        RequestResult::AwaitUserInput
    } else {
        RequestResult::ReadyForResponse
    }
}

/// Perform key exchange and fill the outgoing response when it succeeds.
/// # Safety
/// Handler, packet and synchronous leaves follow the module lifetime contract.
pub(crate) unsafe fn send_response(owner: *mut Handler, packet: *mut packet::State) -> bool {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Client { .. }) {
        debug!(net, 9, "Sending {} authentication response", unsafe {
            (*owner).name()
        });
        return unsafe { send_response(Handler::active(owner), packet) };
    }
    let mut size = 0;
    let password = if unsafe { (*owner).method() } == NetworkAuthenticationMethod::X25519_PAKE {
        let handler = unsafe { single_mut(owner).client_password };
        unsafe { ((*services).client_password)(handler, &raw mut size) }
    } else {
        ptr::null()
    };
    let valid = unsafe {
        let single = single_mut(owner);
        crypto::exchange(
            &mut single.session.keys,
            &single.session.secret,
            &single.session.public,
            &single.session.peer,
            X25519KeyExchangeSide::CLIENT,
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
    let mut message: Zeroizing<X25519KeyExchangeMessage> =
        Zeroizing::new([0; X25519_KEY_EXCHANGE_MESSAGE_SIZE]);
    unsafe {
        ((*services).random_bytes)(message.as_mut_ptr(), X25519_KEY_EXCHANGE_MESSAGE_SIZE);
    }
    unsafe {
        let single = single_mut(owner);
        let mut context = Aead::new(
            client_to_server(&single.session.keys),
            &single.session.exchange_nonce,
        );
        let mac = context.write(message.as_mut(), &single.session.public);
        (*packet).send_bytes(&single.session.public);
        (*packet).send_bytes(&mac);
        (*packet).send_bytes(message.as_ref());
    }
    true
}

/// Read client key-exchange data and check authentication.
/// # Safety
/// Handler, packet and synchronous leaves follow the module lifetime contract.
pub(crate) unsafe fn receive_response(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> ResponseResult {
    let services = unsafe { (*owner).services };
    if matches!(unsafe { &(*owner).kind }, Kind::Server { .. }) {
        debug!(net, 9, "Receiving {} authentication response", unsafe {
            (*owner).name()
        });
        let result = unsafe { receive_response(Handler::active(owner), packet) };
        if result != ResponseResult::NotAuthenticated {
            return result;
        }
        let Kind::Server { handlers } = (unsafe { &mut (*owner).kind }) else {
            unreachable!();
        };
        handlers.pop();
        return if handlers.is_empty() {
            ResponseResult::NotAuthenticated
        } else {
            ResponseResult::RetryNextMethod
        };
    }
    // The original PAKE handler queries the provider before checking packet size.
    let mut size = 0;
    let password = if unsafe { (*owner).method() } == NetworkAuthenticationMethod::X25519_PAKE {
        let provider = unsafe { single_mut(owner).server_password };
        unsafe { ((*services).server_password)(provider, &raw mut size) }
    } else {
        ptr::null()
    };
    if unsafe { (*packet).remaining() }
        != X25519_KEY_SIZE + X25519_MAC_SIZE + X25519_KEY_EXCHANGE_MESSAGE_SIZE
    {
        debug!(
            net,
            1, "[crypto] Received auth response of illegal size; authentication aborted."
        );
        return ResponseResult::NotAuthenticated;
    }
    let mut message: Zeroizing<X25519KeyExchangeMessage> =
        Zeroizing::new([0; X25519_KEY_EXCHANGE_MESSAGE_SIZE]);
    let mut mac: X25519Mac = [0; X25519_MAC_SIZE];
    let valid = unsafe {
        let single = single_mut(owner);
        (*packet).recv_bytes(&mut single.session.peer);
        (*packet).recv_bytes(&mut mac);
        (*packet).recv_bytes(message.as_mut());
        crypto::exchange(
            &mut single.session.keys,
            &single.session.secret,
            &single.session.public,
            &single.session.peer,
            X25519KeyExchangeSide::SERVER,
            bytes(password, size),
        )
    };
    if !valid {
        debug!(
            net,
            0, "[crypto] Peer sent an illegal public key; authentication aborted."
        );
        return ResponseResult::NotAuthenticated;
    }
    let valid = unsafe {
        let single = single_mut(owner);
        Aead::new(
            client_to_server(&single.session.keys),
            &single.session.exchange_nonce,
        )
        .read(message.as_mut(), &mac, &single.session.peer)
    };
    if !valid {
        /* The ciphertext and MAC do not match the encryption key. Most likely
         * the password is invalid, or possibly the client has a bug. */
        return ResponseResult::NotAuthenticated;
    }
    if unsafe { (*owner).method() } == NetworkAuthenticationMethod::X25519_AuthorizedKey {
        let (authorized, peer) = unsafe {
            let single = single_mut(owner);
            (single.authorized, single.session.peer.as_ptr())
        };
        let mut encoded = [0; X25519_KEY_SIZE + X25519_KEY_SIZE];
        unsafe {
            // SAFETY: The live fixed-size public key is disjoint from encoded output.
            crate::byte_strings::format_array_as_hex(
                std::slice::from_raw_parts(peer, X25519_KEY_SIZE),
                &mut encoded,
            );
        }
        if unsafe { ((*services).allowed)(authorized, encoded.as_ptr(), encoded.len()) } == 0 {
            return ResponseResult::NotAuthenticated;
        }
    }
    ResponseResult::Authenticated
}

// Entries share the module contract. Native success values are 0-or-1 u8 (#644).
/// Create the client authentication handlers in method order.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn client_new(
    services: *const Services,
    secret: &X25519SecretKey,
    password: *mut RustAuthenticationClientHandler,
) -> *mut Handler {
    let mut handlers = Vec::new();
    for method in 0..NetworkAuthenticationMethod::End.0 {
        handlers.push(unsafe {
            new_single(
                services,
                SingleArgs {
                    method: NetworkAuthenticationMethod(method),
                    server: false,
                    secret,
                    client_password: password,
                    server_password: ptr::null(),
                    authorized: ptr::null(),
                },
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
pub(crate) unsafe extern "C" fn openttd_rust_auth_client_new(
    services: *const Services,
    secret: *const u8,
    password: *mut RustAuthenticationClientHandler,
) -> *mut Handler {
    unsafe { client_new(services, &*secret.cast::<X25519SecretKey>(), password) }
}
/// Create the available server handlers, preserving provider and fallback order.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn server_new(
    services: *const Services,
    password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
    mask: NetworkAuthenticationMethodMask,
) -> *mut Handler {
    let mut secret = Zeroizing::new([0; X25519_KEY_SIZE]);
    unsafe {
        create_random_secret_key(&mut secret, |key| {
            ((*services).random_bytes)(key.as_mut_ptr(), X25519_KEY_SIZE);
        });
    }
    let mut handlers = Vec::new();
    if !password.is_null() && mask.test(NetworkAuthenticationMethod::X25519_PAKE) {
        let handler = unsafe {
            new_single(
                services,
                SingleArgs {
                    method: NetworkAuthenticationMethod::X25519_PAKE,
                    server: true,
                    secret: &secret,
                    client_password: ptr::null_mut(),
                    server_password: password,
                    authorized: ptr::null(),
                },
            )
        };
        /* Is the handler configured correctly, e.g. does it have a password? */
        if unsafe { can_be_used(&raw const handler) } {
            handlers.push(handler);
        }
    }
    if !authorized.is_null() && mask.test(NetworkAuthenticationMethod::X25519_AuthorizedKey) {
        let handler = unsafe {
            new_single(
                services,
                SingleArgs {
                    method: NetworkAuthenticationMethod::X25519_AuthorizedKey,
                    server: true,
                    secret: &secret,
                    client_password: ptr::null_mut(),
                    server_password: ptr::null(),
                    authorized,
                },
            )
        };
        /* Is the handler configured correctly, e.g. does it have a password? */
        if unsafe { can_be_used(&raw const handler) } {
            handlers.push(handler);
        }
    }
    if handlers.is_empty() && mask.test(NetworkAuthenticationMethod::X25519_KeyExchangeOnly) {
        /* Fall back to the plain handler when neither passwords nor authorized
         * keys are configured. */
        handlers.push(unsafe {
            new_single(
                services,
                SingleArgs {
                    method: NetworkAuthenticationMethod::X25519_KeyExchangeOnly,
                    server: true,
                    secret: &secret,
                    client_password: ptr::null_mut(),
                    server_password: ptr::null(),
                    authorized: ptr::null(),
                },
            )
        });
    }
    Box::into_raw(Box::new(Handler {
        services,
        kind: Kind::Server { handlers },
    }))
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_server_new(
    services: *const Services,
    password: *const NetworkAuthenticationPasswordProvider,
    authorized: *const NetworkAuthenticationAuthorizedKeyHandler,
    mask: u16,
) -> *mut Handler {
    unsafe {
        server_new(
            services,
            password,
            authorized,
            NetworkAuthenticationMethodMask(mask),
        )
    }
}
/// Release the original handler owner and its destruction-time wipes.
/// # Safety
/// `owner` is live, uniquely owned and was allocated by its constructor.
pub(crate) unsafe fn destroy_handler(owner: *mut Handler) {
    unsafe {
        drop(Box::from_raw(owner));
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_destroy(owner: *mut Handler) {
    unsafe {
        destroy_handler(owner);
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
    unsafe { (*owner).method().0 }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_peer(owner: *const Handler) -> *const u8 {
    unsafe { (*owner).peer() }
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
    unsafe { receive_request(owner, packet).0 }
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
    unsafe { receive_response(owner, packet).0 }
}
/// Send the active server request, with its method prefix for combined handlers.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn send_request(
    owner: *mut Handler,
    packet: *mut packet::State,
    check: extern "C" fn(bool),
) {
    if matches!(unsafe { &(*owner).kind }, Kind::Server { .. }) {
        unsafe {
            debug!(net, 9, "Sending {} authentication request", (*owner).name());
            (*packet).send_uint8((*owner).method().0, check);
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
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_send_request(
    owner: *mut Handler,
    packet: *mut packet::State,
    check: extern "C" fn(bool),
) {
    unsafe { send_request(owner, packet, check) }
}
/// Send the active session nonce to enable encryption.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn send_enable(owner: *mut Handler, packet: *mut packet::State) {
    unsafe {
        let session = &single_mut(Handler::active(owner)).session;
        (*packet).send_bytes(&session.encryption_nonce);
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_send_enable(
    owner: *mut Handler,
    packet: *mut packet::State,
) {
    unsafe { send_enable(owner, packet) }
}
/// Read the active session nonce for enabling encryption.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn receive_enable(owner: *mut Handler, packet: *mut packet::State) -> bool {
    unsafe {
        let session = &mut single_mut(Handler::active(owner)).session;
        (*packet).recv_bytes(&mut session.encryption_nonce) == X25519_NONCE_SIZE
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_handler_receive_enable(
    owner: *mut Handler,
    packet: *mut packet::State,
) -> u8 {
    u8::from(unsafe { receive_enable(owner, packet) })
}
/// Create an encryption stream for the selected exchange direction.
/// # Safety
/// Owners, providers, packets and synchronous leaves obey this module's lifetime contract.
pub(crate) unsafe fn stream_new(owner: *const Handler, side: X25519KeyExchangeSide) -> *mut Stream {
    unsafe {
        let session = &(*owner).single().session;
        Box::into_raw(Box::new(Stream {
            context: Aead::new(
                [
                    client_to_server(&session.keys),
                    server_to_client(&session.keys),
                ][usize::from(side.0)],
                &session.encryption_nonce,
            ),
        }))
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_new(
    owner: *const Handler,
    side: u8,
) -> *mut Stream {
    unsafe { stream_new(owner, X25519KeyExchangeSide(side)) }
}

/// Release the original stream owner and its destruction-time wipes.
/// # Safety
/// `owner` is live, uniquely owned and was allocated by its constructor.
pub(crate) unsafe fn destroy_stream(owner: *mut Stream) {
    unsafe {
        drop(Box::from_raw(owner));
    }
}

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_auth_stream_destroy(owner: *mut Stream) {
    unsafe {
        destroy_stream(owner);
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
    unsafe { u8::from((*owner).decrypt(&*mac.cast::<X25519Mac>(), bytes_mut(message, length))) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAuthHandlerServices" ["probe"] => Services {
        allowed, ask_password, authorized_available, client_password, packet, random_bytes,
        server_password,
    };
}

#[cfg(test)]
mod evidence_tests;

#[cfg(test)]
mod tests {
    use super::{
        Handler, NetworkAuthenticationAuthorizedKeyHandler, NetworkAuthenticationPasswordProvider,
        RustAuthenticationClientHandler, Services, SingleArgs, new_single, receive_request,
        receive_response, send_request, send_response,
    };
    use crate::network_config::COMPAT_MTU;
    use crate::network_crypto::{NetworkAuthenticationMethod, RequestResult, ResponseResult};
    use crate::network_crypto_internal::X25519_KEY_SIZE;
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
    fn pake(server: &'static [u8], client: &'static [u8]) -> ResponseResult {
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
                    SingleArgs {
                        method: NetworkAuthenticationMethod::X25519_PAKE,
                        server: is_server,
                        secret: &[secret; X25519_KEY_SIZE],
                        client_password: (&raw const client_password).cast_mut().cast(),
                        server_password: (&raw const server_password).cast(),
                        authorized: ptr::null(),
                    },
                )
            }
        };
        let mut server = new(true, 1);
        let mut client = new(false, 2);
        let mut request = packet::State::new(COMPAT_MTU, ptr::null_mut());
        let mut response = packet::State::new(COMPAT_MTU, ptr::null_mut());
        unsafe {
            send_request(&raw mut server, &raw mut request, check);
            assert!(
                receive_request(&raw mut client, &raw mut request) == RequestResult::AwaitUserInput
            );
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
        assert!(pake(b"", b"") == ResponseResult::Authenticated);
        assert!(pake(b"sikrit", b"sikrit") == ResponseResult::Authenticated);
        assert!(pake(b"sikrit", b"secret") == ResponseResult::NotAuthenticated);
        assert!(pake(b"", b"secret") == ResponseResult::NotAuthenticated);
    }
}

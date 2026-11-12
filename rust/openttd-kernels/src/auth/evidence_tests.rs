/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Public authentication endpoints under prescribed entropy (#736).
//!
//! Moved from `tools/migration/auth-comparison.cpp` with `tools/auth-comparison.py`. The probe
//! built one endpoint program against the pinned original's `network_crypto.cpp` and against the
//! fork's adapters, ran each of 18 cases as a server and a client process, and compared the
//! transcripts of three pairings with the original's own. The original's results are frozen in
//! `endpoints-oracle.txt` (`python3 tools/oracle-capture.py auth_endpoints`), one section per case.
//!
//! This test regenerates the driver's commands and replays each case in the same three pairings
//! (original server with candidate client, the reverse, and candidate with candidate). A recorded
//! result stands for the original endpoint; a candidate endpoint is the probe's command
//! interpreter over the production `openttd_rust_auth_*` entries, with fixture-owned provider,
//! password-request, entropy and debug tables in place of the program's globals.
//!
//! Not observed here: the C++ adapter classes in `src/network/network_crypto.cpp` and the native
//! `Packet` facade, which the probe's candidate called and this test does not.
use super::*;
use crate::network_config::COMPAT_MTU;
use std::cell::Cell;
use std::fmt::Write as _;
use std::ptr::{NonNull, null, null_mut};

/// The probe's `SECRET_CLIENT`: bytes 32 to 63.
const SECRET_CLIENT: [u8; X25519_KEY_SIZE] = {
    let mut secret = [0; X25519_KEY_SIZE];
    let mut i = 0;
    while i < X25519_KEY_SIZE {
        secret[i] = 32 + i.to_le_bytes()[0];
        i += 1;
    }
    secret
};

/// The server's authorized key provider: none, empty, the client's key or 32 zero bytes.
#[derive(Clone, Copy, PartialEq)]
enum Keys {
    None,
    Empty,
    Valid,
    Invalid,
}

/// The driver's `low_order` and `damage`: a request with a zero server key, or one response
/// byte flipped.
#[derive(Clone, Copy)]
enum Fault {
    None,
    LowOrder,
    Damage(usize),
}

/// One driver case, with the driver's defaults.
struct Case {
    name: &'static str,
    password: Option<&'static [u8]>,
    keys: Keys,
    client_password: &'static [u8],
    methods: &'static [u8],
    success: bool,
    mask: u16,
    failures: bool,
    reentry: bool,
    fault: Fault,
}

const DEFAULT: Case = Case {
    name: "",
    password: None,
    keys: Keys::None,
    client_password: b"",
    methods: &[0],
    success: true,
    mask: 7,
    failures: false,
    reentry: false,
    fault: Fault::None,
};

const CASES: [Case; 18] = [
    Case {
        name: "no-password",
        failures: true,
        ..DEFAULT
    },
    Case {
        name: "empty-providers",
        password: Some(b""),
        keys: Keys::Empty,
        ..DEFAULT
    },
    Case {
        name: "password",
        password: Some(b"password"),
        client_password: b"password",
        methods: &[1],
        ..DEFAULT
    },
    Case {
        name: "binary-password",
        password: Some(b"a\x00b\xff"),
        client_password: b"a\x00b\xff",
        methods: &[1],
        ..DEFAULT
    },
    Case {
        name: "wrong-password",
        password: Some(b"password"),
        client_password: b"wrong",
        methods: &[1],
        success: false,
        ..DEFAULT
    },
    Case {
        name: "authorized",
        keys: Keys::Valid,
        methods: &[2],
        ..DEFAULT
    },
    Case {
        name: "unauthorized",
        keys: Keys::Invalid,
        methods: &[2],
        success: false,
        ..DEFAULT
    },
    Case {
        name: "key-before-password",
        password: Some(b"password"),
        keys: Keys::Valid,
        methods: &[2],
        ..DEFAULT
    },
    Case {
        name: "password-fallback",
        password: Some(b"password"),
        keys: Keys::Invalid,
        client_password: b"password",
        methods: &[2, 1],
        ..DEFAULT
    },
    Case {
        name: "fallback-exhausted",
        password: Some(b"password"),
        keys: Keys::Invalid,
        client_password: b"wrong",
        methods: &[2, 1],
        success: false,
        ..DEFAULT
    },
    Case {
        name: "no-supported-methods",
        password: Some(b"password"),
        keys: Keys::Valid,
        mask: 0,
        methods: &[],
        success: false,
        ..DEFAULT
    },
    Case {
        name: "only-plain-supported",
        password: Some(b"password"),
        keys: Keys::Valid,
        mask: 1,
        ..DEFAULT
    },
    Case {
        name: "only-password-supported",
        password: Some(b"password"),
        keys: Keys::Valid,
        client_password: b"password",
        mask: 2,
        methods: &[1],
        ..DEFAULT
    },
    Case {
        name: "only-key-supported",
        password: Some(b"password"),
        keys: Keys::Valid,
        mask: 4,
        methods: &[2],
        ..DEFAULT
    },
    Case {
        name: "password-reentry",
        password: Some(b"password"),
        client_password: b"password",
        methods: &[1],
        reentry: true,
        ..DEFAULT
    },
    Case {
        name: "low-order-server",
        fault: Fault::LowOrder,
        success: false,
        ..DEFAULT
    },
    Case {
        name: "damaged-response-key",
        fault: Fault::Damage(3),
        success: false,
        ..DEFAULT
    },
    Case {
        name: "damaged-response-mac",
        fault: Fault::Damage(35),
        success: false,
        ..DEFAULT
    },
];

/// The probe's `Hex`: lowercase, `-` when empty.
fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".into();
    }
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}
/// The probe's `Unhex`.
fn unhex(text: &str) -> Vec<u8> {
    if text == "-" {
        return Vec::new();
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}
/// The driver's `wire`: a size-prefixed packet of type 0.
fn wire(payload: &[u8]) -> String {
    let size = u16::try_from(payload.len() + 3).unwrap().to_le_bytes();
    hex(&[&size[..], &[0], payload].concat())
}
/// `FormatArrayAsHex`, as the probe's `public` and `peer` print keys.
fn format_hex(bytes: &[u8]) -> String {
    let mut text = vec![0; bytes.len() * 2];
    crate::byte_strings::format_array_as_hex(bytes, &mut text);
    String::from_utf8(text).unwrap()
}

extern "C" fn check(condition: bool) {
    assert!(condition);
}

/// One endpoint process: the probe's globals, handlers and stream.
struct Endpoint {
    random_sizes: Vec<usize>,
    random_seed: u8,
    logs: usize,
    asks: usize,
    replies: usize,
    password_reads: usize,
    authorized_checks: usize,
    allowed_checks: usize,
    last_log: Vec<u8>,
    server_password: Vec<u8>,
    authorized_key: Vec<u8>,
    reentrant_response: String,
    reenter: bool,
    /// The probe's `PasswordRequest::reply`.
    reply: Vec<u8>,
    /// `NetworkAuthenticationPasswordRequestHandler::password`, set by `Reply`.
    password: Vec<u8>,
    services: Services,
    client: *mut Handler,
    server: *mut Handler,
    stream: *mut Stream,
}

thread_local! {
    /// The endpoint whose command runs; `RandomBytesWithFallback` has no context argument.
    static ACTIVE: Cell<*mut Endpoint> = const { Cell::new(null_mut()) };
}

/// The probe's `RandomBytesWithFallback`.
extern "C" fn random_bytes(data: *mut u8, size: usize) {
    let endpoint = ACTIVE.get();
    unsafe {
        for i in 0..size {
            let value =
                usize::from((*endpoint).random_seed) + i + (*endpoint).random_sizes.len() * 31;
            data.add(i).write(value.to_le_bytes()[0]);
        }
        (*endpoint).random_sizes.push(size);
    }
}
/// `PasswordProvider::GetPassword`.
extern "C" fn server_password(
    provider: *const NetworkAuthenticationPasswordProvider,
    size: *mut usize,
) -> *const u8 {
    let endpoint = provider.cast::<Endpoint>().cast_mut();
    unsafe {
        (*endpoint).password_reads += 1;
        size.write((*endpoint).server_password.len());
        (*endpoint).server_password.as_ptr()
    }
}
/// `AuthorizedKeys::CanBeUsed`.
extern "C" fn authorized_available(
    handler: *const NetworkAuthenticationAuthorizedKeyHandler,
) -> u8 {
    let endpoint = handler.cast::<Endpoint>().cast_mut();
    unsafe {
        (*endpoint).authorized_checks += 1;
        u8::from(!(*endpoint).authorized_key.is_empty())
    }
}
/// `AuthorizedKeys::IsAllowed`.
extern "C" fn allowed(
    handler: *const NetworkAuthenticationAuthorizedKeyHandler,
    peer: *const u8,
    size: usize,
) -> u8 {
    let endpoint = handler.cast::<Endpoint>().cast_mut();
    unsafe {
        (*endpoint).allowed_checks += 1;
        u8::from(bytes(peer, size) == (*endpoint).authorized_key.as_slice())
    }
}
/// `PasswordRequest::AskUserForPassword`, `Reply` and the reentrant `SendResponse`.
extern "C" fn ask_password(shell: *mut RustAuthenticationClientHandler) {
    let endpoint = shell.cast::<Endpoint>();
    unsafe {
        (*endpoint).asks += 1;
        (*endpoint).password.clone_from(&(*endpoint).reply);
        (*endpoint).replies += 1;
        if (*endpoint).reenter {
            let mut packet = outgoing();
            assert!(
                openttd_rust_auth_handler_send_response((*endpoint).client, &raw mut packet) != 0,
                "reentrant response"
            );
            (*endpoint).reentrant_response = send(&mut packet);
        }
    }
}
/// `NetworkAuthenticationPasswordRequestHandler::password`, read by the client handler.
extern "C" fn client_password(
    shell: *const RustAuthenticationClientHandler,
    size: *mut usize,
) -> *const u8 {
    let endpoint = shell.cast::<Endpoint>();
    unsafe {
        size.write((*endpoint).password.len());
        (*endpoint).password.as_ptr()
    }
}

/// `Packet(&socket_handler, PacketType{})`.
fn outgoing() -> packet::State {
    unsafe { packet::new_outgoing(null_mut(), packet::test_services(), check, 0, COMPAT_MTU) }
}
/// The probe's `Wire`: prepare the packet and transfer it out.
fn send(packet: &mut packet::State) -> String {
    unsafe {
        packet::prepare_to_send(packet, packet::test_services(), check);
        let amount = packet.transfer_amount(usize::MAX);
        let out = bytes(packet.data.add(usize::from(packet.position)), amount).to_vec();
        packet.transfer_commit(isize::try_from(amount).unwrap());
        hex(&out)
    }
}
/// The probe's `Read`: transfer the wire in, prepare it and read its type.
fn receive(text: &str) -> packet::State {
    let wire = unhex(text);
    let socket = NonNull::dangling().as_ptr();
    let mut packet = packet::State::new_receive(COMPAT_MTU, wire.len(), socket, check);
    unsafe {
        let amount = packet.transfer_amount(usize::MAX);
        std::ptr::copy_nonoverlapping(
            wire.as_ptr(),
            packet.data.add(usize::from(packet.position)),
            amount,
        );
        packet.transfer_commit(isize::try_from(wire.len()).unwrap());
        assert!(
            packet::prepare_to_read(&raw mut packet, packet::test_services()),
            "invalid wire"
        );
        packet::recv_uint8(&raw mut packet, packet::test_services());
    }
    packet
}

impl Endpoint {
    fn new() -> Box<Self> {
        Box::new(Self {
            random_sizes: Vec::new(),
            random_seed: 0,
            logs: 0,
            asks: 0,
            replies: 0,
            password_reads: 0,
            authorized_checks: 0,
            allowed_checks: 0,
            last_log: Vec::new(),
            server_password: Vec::new(),
            authorized_key: Vec::new(),
            reentrant_response: String::new(),
            reenter: false,
            reply: Vec::new(),
            password: Vec::new(),
            services: Services {
                allowed,
                ask_password,
                authorized_available,
                client_password,
                packet: packet::test_services(),
                random_bytes,
                server_password,
            },
            client: null_mut(),
            server: null_mut(),
            stream: null_mut(),
        })
    }
    /// `stream.reset(); server.reset(); client.reset();`
    fn reset(&mut self) {
        unsafe {
            if !self.stream.is_null() {
                openttd_rust_auth_stream_destroy(self.stream);
            }
            if !self.server.is_null() {
                openttd_rust_auth_handler_destroy(self.server);
            }
            if !self.client.is_null() {
                openttd_rust_auth_handler_destroy(self.client);
            }
        }
        (self.stream, self.server, self.client) = (null_mut(), null_mut(), null_mut());
    }
    /// The client if any, else the server, as `state` and `init-stream` choose.
    fn handler(&self) -> *mut Handler {
        if self.client.is_null() {
            self.server
        } else {
            self.client
        }
    }
    /// One command line of the probe's `main`, with its printed result.
    fn call(self: &mut Box<Self>, line: &str) -> String {
        let endpoint: *mut Self = &raw mut **self;
        ACTIVE.set(endpoint);
        let result = unsafe { Self::run(endpoint, line) };
        // `DebugPrint`, through the Rust debug table bound to net level 9.
        for (category, level, message) in crate::debug::tests::take() {
            let line: &[u8] = &[format!("{category}:{level}:").as_bytes(), &message].concat();
            unsafe {
                (*endpoint).logs += 1;
                (*endpoint).last_log = line.to_vec();
            }
        }
        ACTIVE.set(null_mut());
        result.trim_end().to_owned()
    }
    /// The probe's `server` and `client` commands: reset, then create the handler.
    /// # Safety
    /// As `run`.
    unsafe fn start(this: *mut Self, server: bool, [a, b, c, d]: [&str; 4]) -> String {
        let number = |text: &str| text.parse::<u16>().expect("number");
        unsafe {
            (*this).reset();
            (*this).random_sizes.clear();
            (*this).logs = 0;
            (*this).asks = 0;
            (*this).replies = 0;
            (*this).password_reads = 0;
            (*this).authorized_checks = 0;
            (*this).allowed_checks = 0;
            (*this).last_log.clear();
            (*this).reentrant_response.clear();
            let services = &raw const (*this).services;
            if server {
                (*this).server_password = if a == "none" { Vec::new() } else { unhex(a) };
                (*this).authorized_key = if b == "none" || b == "-" {
                    Vec::new()
                } else {
                    b.as_bytes().to_vec()
                };
                (*this).random_seed = u8::try_from(number(d)).unwrap();
                let provider = if a == "none" {
                    null()
                } else {
                    this.cast_const().cast()
                };
                let authorized = if b == "none" {
                    null()
                } else {
                    this.cast_const().cast()
                };
                (*this).server =
                    openttd_rust_auth_server_new(services, provider, authorized, number(c));
            } else {
                (*this).random_seed = u8::try_from(number(c)).unwrap();
                (*this).reenter = d == "1";
                (*this).reply = unhex(b);
                let secret: X25519SecretKey = unhex(a).try_into().unwrap();
                (*this).client =
                    openttd_rust_auth_client_new(services, secret.as_ptr(), this.cast());
            }
        }
        "ok".into()
    }
    /// The probe's `encrypt` and `decrypt` commands on the current stream.
    /// # Safety
    /// As `run`.
    unsafe fn crypt(this: *mut Self, encrypt: bool, a: &str, b: &str) -> String {
        unsafe {
            let mut message = unhex(a);
            let mut mac = [0; X25519_MAC_SIZE];
            let mut ok = 1;
            if encrypt {
                openttd_rust_auth_stream_encrypt(
                    (*this).stream,
                    mac.as_mut_ptr(),
                    message.as_mut_ptr(),
                    message.len(),
                );
            } else {
                let tag = unhex(b);
                mac[..tag.len()].copy_from_slice(&tag);
                ok = openttd_rust_auth_stream_decrypt(
                    (*this).stream,
                    mac.as_ptr(),
                    message.as_mut_ptr(),
                    message.len(),
                );
            }
            format!("{ok} {} {}", hex(&mac), hex(&message))
        }
    }
    /// # Safety
    /// `this` is the active endpoint; no reference into it is held across a production call.
    unsafe fn run(this: *mut Self, line: &str) -> String {
        let mut words = line.split(' ');
        let command = words.next().unwrap();
        let [a, b, c, d] = std::array::from_fn(|_| words.next().unwrap_or(""));
        unsafe {
            match command {
                "server" | "client" => Self::start(this, command == "server", [a, b, c, d]),
                "public" => {
                    let secret: X25519SecretKey = unhex(a).try_into().unwrap();
                    format_hex(&crypto::public_key(&secret))
                }
                "request" | "response" | "nonce" => {
                    let mut packet = outgoing();
                    let mut ok = true;
                    match command {
                        "request" => {
                            openttd_rust_auth_handler_send_request(
                                (*this).server,
                                &raw mut packet,
                                check,
                            );
                        }
                        "response" => {
                            ok = openttd_rust_auth_handler_send_response(
                                (*this).client,
                                &raw mut packet,
                            ) != 0;
                        }
                        _ => openttd_rust_auth_handler_send_enable((*this).server, &raw mut packet),
                    }
                    format!("{} {}", u8::from(ok), send(&mut packet))
                }
                "recv-request" | "recv-response" | "recv-nonce" => {
                    let mut packet = receive(a);
                    let result = match command {
                        "recv-request" => openttd_rust_auth_handler_receive_request(
                            (*this).client,
                            &raw mut packet,
                        ),
                        "recv-response" => openttd_rust_auth_handler_receive_response(
                            (*this).server,
                            &raw mut packet,
                        ),
                        _ => openttd_rust_auth_handler_receive_enable(
                            (*this).client,
                            &raw mut packet,
                        ),
                    };
                    format!("{result} {}", packet.remaining())
                }
                "state" => {
                    let handler = (*this).handler();
                    let mut size = 0;
                    let name = bytes(openttd_rust_auth_handler_name(handler, &raw mut size), size);
                    let sizes: String =
                        (*this)
                            .random_sizes
                            .iter()
                            .fold(String::new(), |mut text, size| {
                                let _ = write!(text, "{size},");
                                text
                            });
                    format!(
                        "{} {} {} {} {} {} {} {} {} {sizes}",
                        std::str::from_utf8(name).unwrap(),
                        openttd_rust_auth_handler_method(handler),
                        (*this).logs,
                        (*this).asks,
                        (*this).replies,
                        (*this).password_reads,
                        (*this).authorized_checks,
                        (*this).allowed_checks,
                        hex(&(*this).last_log),
                    )
                }
                "usable" => openttd_rust_auth_handler_can_be_used((*this).server).to_string(),
                "peer" => format_hex(bytes(
                    openttd_rust_auth_handler_peer((*this).server),
                    X25519_KEY_SIZE,
                )),
                "reentrant" => {
                    let response = &(*this).reentrant_response;
                    if response.is_empty() {
                        "-".into()
                    } else {
                        response.clone()
                    }
                }
                "init-stream" => {
                    // The new stream is created before the old one is released.
                    let side = u8::from(a != "0");
                    let stream = openttd_rust_auth_stream_new((*this).handler(), side);
                    if !(*this).stream.is_null() {
                        openttd_rust_auth_stream_destroy((*this).stream);
                    }
                    (*this).stream = stream;
                    X25519_MAC_SIZE.to_string()
                }
                "encrypt" | "decrypt" => Self::crypt(this, command == "encrypt", a, b),
                _ => panic!("unknown command {command}"),
            }
        }
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.reset();
    }
}

/// Which endpoints of a pairing are candidates; the others replay recorded results.
#[derive(Clone, Copy)]
struct Pairing {
    name: &'static str,
    server: bool,
    client: bool,
}
const PAIRINGS: [Pairing; 3] = [
    Pairing {
        name: "reference-candidate",
        server: false,
        client: true,
    },
    Pairing {
        name: "candidate-reference",
        server: true,
        client: false,
    },
    Pairing {
        name: "candidate-candidate",
        server: true,
        client: true,
    },
];

/// One pairing's run of one case: the recorded results and each call as compared.
struct Run<'a> {
    pairing: Pairing,
    recorded: &'a crate::oracle::Section<'a>,
    server: Box<Endpoint>,
    client: Box<Endpoint>,
    calls: Vec<(String, String)>,
}
impl Run<'_> {
    /// The driver's `Endpoint.call`; a recorded result stands for an original endpoint.
    fn call(&mut self, server: bool, command: &str) -> String {
        let recorded = self
            .recorded
            .case(self.calls.len())
            .expect("call beyond the recorded transcript")
            .to_owned();
        let (role, candidate) = if server {
            ("server", self.pairing.server)
        } else {
            ("client", self.pairing.client)
        };
        let actual = match (candidate, server) {
            (false, _) => recorded.clone(),
            (true, true) => self.server.call(command),
            (true, false) => self.client.call(command),
        };
        let label = format!("{} {role}: {command}", self.pairing.name);
        self.calls.push((label, actual));
        recorded
    }
    fn server(&mut self, command: &str) -> String {
        self.call(true, command)
    }
    fn client(&mut self, command: &str) -> String {
        self.call(false, command)
    }
}

/// The driver's `scenario`, with its assertions on the original's results.
fn scenario(run: &mut Run<'_>, case: &Case) {
    let secret = hex(&SECRET_CLIENT);
    let public = run.client(&format!("public {secret}"));
    let keys = match case.keys {
        Keys::None => "none".to_owned(),
        Keys::Empty => "-".to_owned(),
        Keys::Valid => public.clone(),
        Keys::Invalid => "00".repeat(32),
    };
    let password = case.password.map_or("none".to_owned(), hex);
    assert_eq!(
        run.server(&format!("server {password} {keys} {} 73", case.mask)),
        "ok"
    );
    let client_password = hex(case.client_password);
    let reentry = u8::from(case.reentry);
    assert_eq!(
        run.client(&format!("client {secret} {client_password} 191 {reentry}")),
        "ok"
    );
    assert_eq!(
        run.server("usable"),
        u8::from(!case.methods.is_empty()).to_string()
    );
    run.server("state");
    run.client("state");
    for (index, &method) in case.methods.iter().enumerate() {
        let mut request = run.server("request").split(' ').nth(1).unwrap().to_owned();
        assert_eq!(unhex(&request)[3], method);
        if case.failures {
            for unknown in [3, 255] {
                assert_eq!(
                    run.client(&format!("recv-request {}", wire(&[unknown]))),
                    "2 0"
                );
                run.client("state");
            }
            for length in [0, 55, 57] {
                let payload = [&[method][..], &vec![0; length]].concat();
                let result = run.client(&format!("recv-request {}", wire(&payload)));
                assert_eq!(result, format!("2 {length}"));
                run.client("state");
            }
        }
        if matches!(case.fault, Fault::LowOrder) {
            let mut changed = unhex(&request);
            changed[4..36].fill(0);
            request = hex(&changed);
        }
        let ready = u8::from(method != 1);
        assert_eq!(
            run.client(&format!("recv-request {request}")),
            format!("{ready} 0")
        );
        run.client("state");
        let mut response;
        if case.reentry {
            response = run.client("reentrant");
            assert_ne!(response, "-");
        } else {
            let result = run.client("response");
            let (ok, sent) = result.split_once(' ').unwrap();
            response = sent.to_owned();
            if matches!(case.fault, Fault::LowOrder) {
                assert_eq!(ok, "0");
                run.client("state");
                return;
            }
            assert_eq!(ok, "1");
        }
        if let Fault::Damage(damage) = case.fault {
            let mut changed = unhex(&response);
            changed[damage] ^= 1;
            response = hex(&changed);
        }
        let expected = if index + 1 < case.methods.len() {
            2
        } else {
            u8::from(!case.success)
        };
        assert_eq!(
            run.server(&format!("recv-response {response}")),
            format!("{expected} 0")
        );
        run.server("state");
    }
    if case.success {
        encrypted(run, case, &public);
    }
}

/// The driver's `scenario` after a successful authentication: the peer key, the
/// enable-encryption nonce and both streams.
fn encrypted(run: &mut Run<'_>, case: &Case, public: &str) {
    assert_eq!(run.server("peer"), public);
    let nonce = run.server("nonce").split(' ').nth(1).unwrap().to_owned();
    assert_eq!(run.client(&format!("recv-nonce {}", wire(b""))), "0 0");
    assert_eq!(run.client(&format!("recv-nonce {nonce}")), "1 0");
    if case.failures {
        let extended = [&unhex(&nonce)[3..], b"xyz"].concat();
        assert_eq!(
            run.client(&format!("recv-nonce {}", wire(&extended))),
            "1 3"
        );
    }
    for side in [false, true] {
        // Side 0 sends client to server; side 1 server to client.
        let command = format!("init-stream {}", u8::from(side));
        assert_eq!(run.call(side, &command), "16");
        assert_eq!(run.call(!side, &command), "16");
        for length in [0usize, 1, 8, 15, 16, 17, 63, 64, 65, 127, 256] {
            let plaintext: Vec<u8> = (0..length)
                .map(|i| (i * 17 + length).to_le_bytes()[0])
                .collect();
            let encrypted = run.call(side, &format!("encrypt {}", hex(&plaintext)));
            let [_, mac, ciphertext] =
                <[&str; 3]>::try_from(encrypted.split(' ').collect::<Vec<_>>()).unwrap();
            if case.failures {
                let mut tampered = unhex(mac);
                tampered[0] ^= 128;
                let failed = run.call(!side, &format!("decrypt {ciphertext} {}", hex(&tampered)));
                let failed: Vec<_> = failed.split(' ').collect();
                assert!(failed[0] == "0" && failed[2] == ciphertext);
            }
            let decrypted = run.call(!side, &format!("decrypt {ciphertext} {mac}"));
            let decrypted: Vec<_> = decrypted.split(' ').collect();
            assert!(decrypted[0] == "1" && decrypted[2] == hex(&plaintext));
            if case.failures {
                let again = run.call(!side, &format!("decrypt {ciphertext} {mac}"));
                assert!(again.starts_with("0 "));
            }
        }
    }
}

#[test]
fn endpoints_match_original_transcripts() {
    if crate::services_assert::isolate_test_check(
        "auth::evidence_tests::endpoints_match_original_transcripts",
        check,
    ) {
        return;
    }
    crate::debug::tests::take();
    let oracle = crate::oracle::parse(include_str!("endpoints-oracle.txt"));
    let mut records = 0;
    for case in &CASES {
        for pairing in PAIRINGS {
            let mut run = Run {
                pairing,
                recorded: &oracle[case.name],
                server: Endpoint::new(),
                client: Endpoint::new(),
                calls: Vec::new(),
            };
            scenario(&mut run, case);
            let calls = std::mem::take(&mut run.calls);
            drop(run);
            records += crate::oracle::check(&oracle, case.name, calls);
        }
    }
    // The probe's `mixed_endpoint_records`.
    assert_eq!(records, 2_544);
}

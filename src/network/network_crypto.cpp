/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_crypto.cpp Implementation of the network specific cryptography helpers. */

#include "../stdafx.h"

#include "network_crypto_internal.h"
#include "core/packet.h"

#include "../3rdparty/monocypher/monocypher.h"
#include "../core/random_func.hpp"
#include "../debug.h"
#include "../string_func.h"

#include "../safeguards.h"

/**
 * Call \c crypto_wipe for all the data in the given span.
 * @param span The span to cryptographically wipe.
 */
static void crypto_wipe(std::span<uint8_t> span)
{
	crypto_wipe(span.data(), span.size());
}

#ifdef WITH_RUST
/* These callbacks are primitive-only and cannot throw. They start the actual
 * trivial vendor object lifetime in Rust-allocated, correctly aligned storage.
 * The table keeps vendor symbols out of the Rust archive's other consumers. */
static_assert(X25519_KEY_SIZE == 32 && X25519_NONCE_SIZE == 24 && X25519_MAC_SIZE == 16 && X25519_KEY_EXCHANGE_MESSAGE_SIZE == 8);
static_assert(static_cast<uint8_t>(X25519KeyExchangeSide::CLIENT) == 0 && static_cast<uint8_t>(X25519KeyExchangeSide::SERVER) == 1);
static const OpenTTDAuthPrimitives &GetAuthPrimitives()
{
	static const OpenTTDAuthPrimitives primitives = {
		1, 0, sizeof(crypto_aead_ctx), alignof(crypto_aead_ctx), sizeof(crypto_blake2b_ctx), alignof(crypto_blake2b_ctx),
		[](void *data, size_t size) noexcept { ::crypto_wipe(data, size); },
		[](uint8_t *shared, const uint8_t *secret, const uint8_t *peer) noexcept { crypto_x25519(shared, secret, peer); },
		[](uint8_t *public_key, const uint8_t *secret) noexcept { crypto_x25519_public_key(public_key, secret); },
		[](void *context, size_t size) noexcept { crypto_blake2b_init(::new (context) crypto_blake2b_ctx, size); },
		[](void *context, const uint8_t *data, size_t size) noexcept { crypto_blake2b_update(static_cast<crypto_blake2b_ctx *>(context), data, size); },
		[](void *context, uint8_t *keys) noexcept { crypto_blake2b_final(static_cast<crypto_blake2b_ctx *>(context), keys); },
		[](void *context, const uint8_t *key, const uint8_t *nonce) noexcept { crypto_aead_init_x(::new (context) crypto_aead_ctx, key, nonce); },
		[](void *destination, const void *source) noexcept { ::new (destination) crypto_aead_ctx(*static_cast<const crypto_aead_ctx *>(source)); },
		[](void *context, uint8_t *cipher, uint8_t *mac, const uint8_t *ad, size_t ad_size, const uint8_t *message, size_t size) noexcept { crypto_aead_write(static_cast<crypto_aead_ctx *>(context), cipher, mac, ad, ad_size, message, size); },
		[](void *context, uint8_t *message, const uint8_t *mac, const uint8_t *ad, size_t ad_size, const uint8_t *cipher, size_t size) noexcept -> int32_t { return crypto_aead_read(static_cast<crypto_aead_ctx *>(context), message, mac, ad, ad_size, cipher, size); },
		[](uint8_t *cipher, uint8_t *mac, const uint8_t *key, const uint8_t *nonce, const uint8_t *ad, size_t ad_size, const uint8_t *message, size_t size) noexcept { crypto_aead_lock(cipher, mac, key, nonce, ad, ad_size, message, size); },
		[](uint8_t *message, const uint8_t *mac, const uint8_t *key, const uint8_t *nonce, const uint8_t *ad, size_t ad_size, const uint8_t *cipher, size_t size) noexcept -> int32_t { return crypto_aead_unlock(message, mac, key, nonce, ad, ad_size, cipher, size); },
	};
	return primitives;
}

X25519DerivedKeys::X25519DerivedKeys() : keys(openttd_rust_auth_keys_new(&GetAuthPrimitives())) {}
#endif

/** Ensure the derived keys do not get leaked when we're done with it. */
X25519DerivedKeys::~X25519DerivedKeys()
{
#ifndef WITH_RUST
	crypto_wipe(keys);
#endif
}

/**
 * Get the key to encrypt or decrypt a message sent from the client to the server.
 * @return The raw bytes of the key.
 */
std::span<const uint8_t> X25519DerivedKeys::ClientToServer() const
{
#ifdef WITH_RUST
	return std::span(openttd_rust_auth_keys_data(this->keys.get(), 0), X25519_KEY_SIZE);
#else
	return std::span(this->keys.data(), X25519_KEY_SIZE);
#endif
}

/**
 * Get the key to encrypt or decrypt a message sent from the server to the client.
 * @return The raw bytes of the key.
 */
std::span<const uint8_t> X25519DerivedKeys::ServerToClient() const
{
#ifdef WITH_RUST
	return std::span(openttd_rust_auth_keys_data(this->keys.get(), 1), X25519_KEY_SIZE);
#else
	return std::span(this->keys.data() + X25519_KEY_SIZE, X25519_KEY_SIZE);
#endif
}

/**
 * Perform the actual key exchange.
 * @param peer_public_key The public key chosen by the other participant of the key exchange.
 * @param side Whether we are the client or server; used to hash the public key of us and the peer in the right order.
 * @param our_secret_key The secret key of us.
 * @param our_public_key The public key of us.
 * @param extra_payload Extra payload to put into the hash function to create the derived keys.
 * @return True when the key exchange has succeeded, false when an illegal public key was given.
 */
bool X25519DerivedKeys::Exchange(const X25519PublicKey &peer_public_key, X25519KeyExchangeSide side,
		const X25519SecretKey &our_secret_key, const X25519PublicKey &our_public_key, std::string_view extra_payload)
{
#ifdef WITH_RUST
	return openttd_rust_auth_keys_exchange(this->keys.get(), peer_public_key.data(), static_cast<uint8_t>(side),
			our_secret_key.data(), our_public_key.data(), reinterpret_cast<const uint8_t *>(extra_payload.data()), extra_payload.size()) != 0;
#else
	X25519Key shared_secret;
	crypto_x25519(shared_secret.data(), our_secret_key.data(), peer_public_key.data());
	if (std::all_of(shared_secret.begin(), shared_secret.end(), [](auto v) { return v == 0; })) {
		/* A shared secret of all zeros means that the peer tried to force the shared secret to a known constant. */
		return false;
	}

	crypto_blake2b_ctx ctx;
	crypto_blake2b_init(&ctx, this->keys.size());
	crypto_blake2b_update(&ctx, shared_secret.data(), shared_secret.size());
	switch (side) {
		case X25519KeyExchangeSide::SERVER:
			crypto_blake2b_update(&ctx, our_public_key.data(), our_public_key.size());
			crypto_blake2b_update(&ctx, peer_public_key.data(), peer_public_key.size());
			break;
		case X25519KeyExchangeSide::CLIENT:
			crypto_blake2b_update(&ctx, peer_public_key.data(), peer_public_key.size());
			crypto_blake2b_update(&ctx, our_public_key.data(), our_public_key.size());
			break;
		default:
			NOT_REACHED();
	}
	crypto_blake2b_update(&ctx, reinterpret_cast<const uint8_t *>(extra_payload.data()), extra_payload.size());
	crypto_blake2b_final(&ctx, this->keys.data());
	return true;
#endif
}

/**
 * Encryption handler implementation for monocypher encryption after a X25519 key exchange.
 */
#ifdef WITH_RUST
class X25519EncryptionHandler : public NetworkEncryptionHandler {
private:
	RustAuthStream context; ///< Independently owned opaque vendor context in Rust.
public:
	/* C++ allocation succeeds before this initializer allocates the Rust owner. */
	X25519EncryptionHandler(const OpenTTDAuthSession *session, uint8_t side) : context(openttd_rust_auth_stream_new(session, side)) {}
	~X25519EncryptionHandler() = default;
	size_t MACSize() const override { return X25519_MAC_SIZE; }
	bool Decrypt(std::span<std::uint8_t> mac, std::span<std::uint8_t> message) override
	{
		return openttd_rust_auth_stream_decrypt(this->context.get(), mac.data(), message.data(), message.size()) != 0;
	}
	void Encrypt(std::span<std::uint8_t> mac, std::span<std::uint8_t> message) override
	{
		openttd_rust_auth_stream_encrypt(this->context.get(), mac.data(), message.data(), message.size());
	}
};
#else
class X25519EncryptionHandler : public NetworkEncryptionHandler {
private:
	crypto_aead_ctx context; ///< The actual encryption context.

public:
	/**
	 * Create the encryption handler.
	 * @param key The key used for the encryption.
	 * @param nonce The nonce used for the encryption.
	 */
	X25519EncryptionHandler(const std::span<const uint8_t> key, const X25519Nonce &nonce)
	{
		assert(key.size() == X25519_KEY_SIZE);
		crypto_aead_init_x(&this->context, key.data(), nonce.data());
	}

	/** Ensure the encryption context is wiped! */
	~X25519EncryptionHandler()
	{
		crypto_wipe(&this->context, sizeof(this->context));
	}

	size_t MACSize() const override
	{
		return X25519_MAC_SIZE;
	}

	bool Decrypt(std::span<std::uint8_t> mac, std::span<std::uint8_t> message) override
	{
		return crypto_aead_read(&this->context, message.data(), mac.data(), nullptr, 0, message.data(), message.size()) == 0;
	}

	void Encrypt(std::span<std::uint8_t> mac, std::span<std::uint8_t> message) override
	{
		crypto_aead_write(&this->context, message.data(), mac.data(), nullptr, 0, message.data(), message.size());
	}
};
#endif

/** Ensure the key does not get leaked when we're done with it. */
X25519Key::~X25519Key()
{
	crypto_wipe(*this);
}

/**
 * Create a new secret key that's filled with random bytes.
 * @return The randomly filled key.
 */
/* static */ X25519SecretKey X25519SecretKey::CreateRandom()
{
	X25519SecretKey secret_key;
	RandomBytesWithFallback(secret_key);
	return secret_key;
}

/**
 * Create the public key associated with this secret key.
 * @return The public key.
 */
X25519PublicKey X25519SecretKey::CreatePublicKey() const
{
	X25519PublicKey public_key;
	crypto_x25519_public_key(public_key.data(), this->data());
	return public_key;
}

/**
 * Create a new nonce that's filled with random bytes.
 * @return The randomly filled nonce.
 */
/* static */ X25519Nonce X25519Nonce::CreateRandom()
{
	X25519Nonce nonce;
	RandomBytesWithFallback(nonce);
	return nonce;
}

/** Ensure the nonce does not get leaked when we're done with it. */
X25519Nonce::~X25519Nonce()
{
	crypto_wipe(*this);
}

/**
 * Create the handler, and generate the public keys accordingly.
 * @param secret_key The secret key to use for this handler. Defaults to secure random data.
 */
#ifdef WITH_RUST
X25519AuthenticationHandler::X25519AuthenticationHandler(const X25519SecretKey &secret_key) :
	session(openttd_rust_auth_session_new(&GetAuthPrimitives(), secret_key.data()))
{
	RandomBytesWithFallback(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 2), X25519_NONCE_SIZE));
	RandomBytesWithFallback(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 3), X25519_NONCE_SIZE));
}
#else
X25519AuthenticationHandler::X25519AuthenticationHandler(const X25519SecretKey &secret_key) :
	our_secret_key(secret_key), our_public_key(secret_key.CreatePublicKey()),
	key_exchange_nonce(X25519Nonce::CreateRandom()), encryption_nonce(X25519Nonce::CreateRandom())
{
}
#endif

/* virtual */ void X25519AuthenticationHandler::SendRequest(Packet &p)
{
#ifdef WITH_RUST
	p.Send_bytes(std::span(openttd_rust_auth_session_view(this->session.get(), 0), X25519_KEY_SIZE));
	p.Send_bytes(std::span(openttd_rust_auth_session_view(this->session.get(), 2), X25519_NONCE_SIZE));
#else
	p.Send_bytes(this->our_public_key);
	p.Send_bytes(this->key_exchange_nonce);
#endif
}

/**
 * Read the key exchange data from a \c Packet that came from the server,
 * @param p The packet that has been received.
 * @return True when the data seems correct.
 */
bool X25519AuthenticationHandler::ReceiveRequest(Packet &p)
{
	if (p.RemainingBytesToTransfer() != X25519_KEY_SIZE + X25519_NONCE_SIZE) {
		Debug(net, 1, "[crypto] Received auth response of illegal size; authentication aborted.");
		return false;
	}

#ifdef WITH_RUST
	p.Recv_bytes(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 1), X25519_KEY_SIZE));
	p.Recv_bytes(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 2), X25519_NONCE_SIZE));
#else
	p.Recv_bytes(this->peer_public_key);
	p.Recv_bytes(this->key_exchange_nonce);
#endif
	return true;
}

/**
 * Perform the key exchange, and when that is correct fill the \c Packet with the appropriate data.
 * @param p The packet that has to be sent.
 * @param derived_key_extra_payload The extra payload to pass to the key exchange.
 * @return Whether the key exchange was successful or not.
 */
bool X25519AuthenticationHandler::SendResponse(Packet &p, std::string_view derived_key_extra_payload)
{
#ifdef WITH_RUST
	if (openttd_rust_auth_session_exchange(this->session.get(), static_cast<uint8_t>(X25519KeyExchangeSide::CLIENT),
			reinterpret_cast<const uint8_t *>(derived_key_extra_payload.data()), derived_key_extra_payload.size()) == 0) {
#else
	if (!this->derived_keys.Exchange(this->peer_public_key, X25519KeyExchangeSide::CLIENT,
			this->our_secret_key, this->our_public_key, derived_key_extra_payload)) {
#endif
		Debug(net, 0, "[crypto] Peer sent an illegal public key; authentication aborted.");
		return false;
	}

	X25519KeyExchangeMessage message;
	RandomBytesWithFallback(message);
	X25519Mac mac;

#ifdef WITH_RUST
	openttd_rust_auth_session_encrypt_response(this->session.get(), message.data(), mac.data());
	p.Send_bytes(std::span(openttd_rust_auth_session_view(this->session.get(), 0), X25519_KEY_SIZE));
#else
	crypto_aead_lock(message.data(), mac.data(), this->derived_keys.ClientToServer().data(), this->key_exchange_nonce.data(),
			this->our_public_key.data(), this->our_public_key.size(), message.data(), message.size());

	p.Send_bytes(this->our_public_key);
#endif
	p.Send_bytes(mac);
	p.Send_bytes(message);
	return true;
}

/**
 * Get the public key the peer provided for the key exchange.
 * @return The hexadecimal string representation of the peer's public key.
 */
std::string X25519AuthenticationHandler::GetPeerPublicKey() const
{
#ifdef WITH_RUST
	return FormatArrayAsHex(std::span(openttd_rust_auth_session_view(this->session.get(), 1), X25519_KEY_SIZE));
#else
	return FormatArrayAsHex(this->peer_public_key);
#endif
}

/**
 * Send the initial nonce for the encrypted connection.
 * @param p The packet to send the data in.
 */
void X25519AuthenticationHandler::SendEnableEncryption(struct Packet &p) const
{
#ifdef WITH_RUST
	p.Send_bytes(std::span(openttd_rust_auth_session_view(this->session.get(), 3), X25519_NONCE_SIZE));
#else
	p.Send_bytes(this->encryption_nonce);
#endif
}

/**
 * Receive the initial nonce for the encrypted connection.
 * @param p The packet to read the data from.
 * @return \c true when enough bytes could be read for the nonce, otherwise \c false.
 */
bool X25519AuthenticationHandler::ReceiveEnableEncryption(struct Packet &p)
{
#ifdef WITH_RUST
	return p.Recv_bytes(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 3), X25519_NONCE_SIZE)) == X25519_NONCE_SIZE;
#else
	return p.Recv_bytes(this->encryption_nonce) == this->encryption_nonce.size();
#endif
}

std::unique_ptr<NetworkEncryptionHandler> X25519AuthenticationHandler::CreateClientToServerEncryptionHandler() const
{
#ifdef WITH_RUST
	return std::make_unique<X25519EncryptionHandler>(this->session.get(), 0);
#else
	return std::make_unique<X25519EncryptionHandler>(this->derived_keys.ClientToServer(), this->encryption_nonce);
#endif
}

std::unique_ptr<NetworkEncryptionHandler> X25519AuthenticationHandler::CreateServerToClientEncryptionHandler() const
{
#ifdef WITH_RUST
	return std::make_unique<X25519EncryptionHandler>(this->session.get(), 1);
#else
	return std::make_unique<X25519EncryptionHandler>(this->derived_keys.ServerToClient(), this->encryption_nonce);
#endif
}

/**
 * Read the key exchange data from a \c Packet that came from the client, and check whether the client passes the key
 * exchange successfully.
 * @param p The packet that has been received.
 * @param derived_key_extra_payload The extra payload to pass to the key exchange.
 * @return Whether the authentication was successful or not.
 */
NetworkAuthenticationServerHandler::ResponseResult X25519AuthenticationHandler::ReceiveResponse(Packet &p, std::string_view derived_key_extra_payload)
{
	if (p.RemainingBytesToTransfer() != X25519_KEY_SIZE + X25519_MAC_SIZE + X25519_KEY_EXCHANGE_MESSAGE_SIZE) {
		Debug(net, 1, "[crypto] Received auth response of illegal size; authentication aborted.");
		return NetworkAuthenticationServerHandler::ResponseResult::NotAuthenticated;
	}

	X25519KeyExchangeMessage message{};
	X25519Mac mac;

#ifdef WITH_RUST
	p.Recv_bytes(std::span(openttd_rust_auth_session_mut_view(this->session.get(), 1), X25519_KEY_SIZE));
#else
	p.Recv_bytes(this->peer_public_key);
#endif
	p.Recv_bytes(mac);
	p.Recv_bytes(message);

#ifdef WITH_RUST
	if (openttd_rust_auth_session_exchange(this->session.get(), static_cast<uint8_t>(X25519KeyExchangeSide::SERVER),
			reinterpret_cast<const uint8_t *>(derived_key_extra_payload.data()), derived_key_extra_payload.size()) == 0) {
#else
	if (!this->derived_keys.Exchange(this->peer_public_key, X25519KeyExchangeSide::SERVER,
			this->our_secret_key, this->our_public_key, derived_key_extra_payload)) {
#endif
		Debug(net, 0, "[crypto] Peer sent an illegal public key; authentication aborted.");
		return NetworkAuthenticationServerHandler::ResponseResult::NotAuthenticated;
	}

#ifdef WITH_RUST
	if (openttd_rust_auth_session_decrypt_response(this->session.get(), message.data(), mac.data()) == 0) {
#else
	if (crypto_aead_unlock(message.data(), mac.data(), this->derived_keys.ClientToServer().data(), this->key_exchange_nonce.data(),
			this->peer_public_key.data(), this->peer_public_key.size(), message.data(), message.size()) != 0) {
#endif
		/*
		 * The ciphertext and the message authentication code do not match with the encryption key.
		 * This is most likely an invalid password, or possibly a bug in the client.
		 */
		return NetworkAuthenticationServerHandler::ResponseResult::NotAuthenticated;
	}

	return NetworkAuthenticationServerHandler::ResponseResult::Authenticated;
}


/* virtual */ NetworkAuthenticationClientHandler::RequestResult X25519PAKEClientHandler::ReceiveRequest(struct Packet &p)
{
	bool success = this->X25519AuthenticationHandler::ReceiveRequest(p);
	if (!success) return NetworkAuthenticationClientHandler::RequestResult::Invalid;

	this->handler->AskUserForPassword(this->handler);
	return NetworkAuthenticationClientHandler::RequestResult::AwaitUserInput;
}

/**
 * Get the secret key from the given string. If that is not a valid secret key, reset it with a random one.
 * Furthermore update the public key so it is always in sync with the private key.
 * @param secret_key The secret key to read/validate/fix.
 * @param public_key The public key to update.
 * @return The valid secret key.
 */
/* static */ X25519SecretKey X25519AuthorizedKeyClientHandler::GetValidSecretKeyAndUpdatePublicKey(std::string &secret_key, std::string &public_key)
{
	X25519SecretKey key{};
	if (!ConvertHexToBytes(secret_key, key)) {
		if (secret_key.empty()) {
			Debug(net, 3, "[crypto] Creating a new random key");
		} else {
			Debug(net, 0, "[crypto] Found invalid secret key, creating a new random key");
		}
		key = X25519SecretKey::CreateRandom();
		secret_key = FormatArrayAsHex(key);
	}

	public_key = FormatArrayAsHex(key.CreatePublicKey());
	return key;
}

/* virtual */ NetworkAuthenticationServerHandler::ResponseResult X25519AuthorizedKeyServerHandler::ReceiveResponse(Packet &p)
{
	ResponseResult result = this->X25519AuthenticationHandler::ReceiveResponse(p, {});
	if (result != ResponseResult::Authenticated) return result;

	std::string peer_public_key = this->GetPeerPublicKey();
	return this->authorized_key_handler->IsAllowed(peer_public_key) ? ResponseResult::Authenticated : ResponseResult::NotAuthenticated;
}


/* virtual */ NetworkAuthenticationClientHandler::RequestResult CombinedAuthenticationClientHandler::ReceiveRequest(struct Packet &p)
{
	NetworkAuthenticationMethod method = static_cast<NetworkAuthenticationMethod>(p.Recv_uint8());

	auto is_of_method = [method](Handler &handler) { return handler->GetAuthenticationMethod() == method; };
	auto it = std::ranges::find_if(handlers, is_of_method);
	if (it == handlers.end()) return RequestResult::Invalid;

	this->current_handler = it->get();

	Debug(net, 9, "Received {} authentication request", this->GetName());
	return this->current_handler->ReceiveRequest(p);
}

/* virtual */ bool CombinedAuthenticationClientHandler::SendResponse(struct Packet &p)
{
	Debug(net, 9, "Sending {} authentication response", this->GetName());

	return this->current_handler->SendResponse(p);
}

/* virtual */ std::string_view CombinedAuthenticationClientHandler::GetName() const
{
	return this->current_handler != nullptr ? this->current_handler->GetName() : "Unknown";
}

/* virtual */ NetworkAuthenticationMethod CombinedAuthenticationClientHandler::GetAuthenticationMethod() const
{
	return this->current_handler != nullptr ? this->current_handler->GetAuthenticationMethod() : NetworkAuthenticationMethod::End;
}


/**
 * Add the given sub-handler to this handler, if the handler can be used (e.g. there are authorized keys or there is a password).
 * @param handler The handler to add.
 */
void CombinedAuthenticationServerHandler::Add(CombinedAuthenticationServerHandler::Handler &&handler)
{
	/* Is the handler configured correctly, e.g. does it have a password? */
	if (!handler->CanBeUsed()) return;

	this->handlers.push_back(std::move(handler));
}

/* virtual */ void CombinedAuthenticationServerHandler::SendRequest(struct Packet &p)
{
	Debug(net, 9, "Sending {} authentication request", this->GetName());

	p.Send_uint8(to_underlying(this->handlers.back()->GetAuthenticationMethod()));
	this->handlers.back()->SendRequest(p);
}

/* virtual */ NetworkAuthenticationServerHandler::ResponseResult CombinedAuthenticationServerHandler::ReceiveResponse(struct Packet &p)
{
	Debug(net, 9, "Receiving {} authentication response", this->GetName());

	ResponseResult result = this->handlers.back()->ReceiveResponse(p);
	if (result != ResponseResult::NotAuthenticated) return result;

	this->handlers.pop_back();
	return this->CanBeUsed() ? ResponseResult::RetryNextMethod : ResponseResult::NotAuthenticated;
}

/* virtual */ std::string_view CombinedAuthenticationServerHandler::GetName() const
{
	return this->CanBeUsed() ? this->handlers.back()->GetName() : "Unknown";
}

/* virtual */ NetworkAuthenticationMethod CombinedAuthenticationServerHandler::GetAuthenticationMethod() const
{
	return this->CanBeUsed() ? this->handlers.back()->GetAuthenticationMethod() : NetworkAuthenticationMethod::End;
}

/* virtual */ bool CombinedAuthenticationServerHandler::CanBeUsed() const
{
	return !this->handlers.empty();
}


/* virtual */ void NetworkAuthenticationPasswordRequestHandler::Reply(const std::string &password)
{
	this->password = password;
	this->SendResponse();
}

/**
 * Ensures that the given secret key is valid, and when not overwrite it with a valid secret key. Then update the public key to be associated with the secret key.
 * @param secret_key The location where the secret key is stored; can be overwritten when invalid.
 * @param public_key The location where the public key is stored; can be overwritten when invalid.
 */
/* static */ void NetworkAuthenticationClientHandler::EnsureValidSecretKeyAndUpdatePublicKey(std::string &secret_key, std::string &public_key)
{
	X25519AuthorizedKeyClientHandler::GetValidSecretKeyAndUpdatePublicKey(secret_key, public_key);
}

/**
 * Create a NetworkAuthenticationClientHandler.
 * @param password_handler The handler for when a request for password needs to be passed on to the user.
 * @param secret_key The location where the secret key is stored; can be overwritten when invalid.
 * @param public_key The location where the public key is stored; can be overwritten when invalid.
 */
/* static */ std::unique_ptr<NetworkAuthenticationClientHandler> NetworkAuthenticationClientHandler::Create(std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password_handler, std::string &secret_key, std::string &public_key)
{
	auto secret = X25519AuthorizedKeyClientHandler::GetValidSecretKeyAndUpdatePublicKey(secret_key, public_key);
	auto handler = std::make_unique<CombinedAuthenticationClientHandler>();
	handler->Add(std::make_unique<X25519KeyExchangeOnlyClientHandler>(secret));
	handler->Add(std::make_unique<X25519PAKEClientHandler>(secret, std::move(password_handler)));
	handler->Add(std::make_unique<X25519AuthorizedKeyClientHandler>(secret));
	return handler;
}

/**
 * Create a NetworkAuthenticationServerHandler.
 * @param password_provider Callback to provide the password handling. Must remain valid until the authentication has succeeded or failed. Can be \c nullptr to skip password checks.
 * @param authorized_key_handler Callback to provide the authorized key handling. Must remain valid until the authentication has succeeded or failed. Can be \c nullptr to skip authorized key checks.
 * @param client_supported_method_mask Bitmask of the methods that are supported by the client. Defaults to support of all methods.
 */
std::unique_ptr<NetworkAuthenticationServerHandler> NetworkAuthenticationServerHandler::Create(const NetworkAuthenticationPasswordProvider *password_provider, const NetworkAuthenticationAuthorizedKeyHandler *authorized_key_handler, NetworkAuthenticationMethodMask client_supported_method_mask)
{
	auto secret = X25519SecretKey::CreateRandom();
	auto handler = std::make_unique<CombinedAuthenticationServerHandler>();
	if (password_provider != nullptr && client_supported_method_mask.Test(NetworkAuthenticationMethod::X25519_PAKE)) {
		handler->Add(std::make_unique<X25519PAKEServerHandler>(secret, password_provider));
	}

	if (authorized_key_handler != nullptr && client_supported_method_mask.Test(NetworkAuthenticationMethod::X25519_AuthorizedKey)) {
		handler->Add(std::make_unique<X25519AuthorizedKeyServerHandler>(secret, authorized_key_handler));
	}

	if (!handler->CanBeUsed() && client_supported_method_mask.Test(NetworkAuthenticationMethod::X25519_KeyExchangeOnly)) {
		/* Fall back to the plain handler when neither password, nor authorized keys are configured. */
		handler->Add(std::make_unique<X25519KeyExchangeOnlyServerHandler>(secret));
	}
	return handler;
}

#if defined(WITH_RUST) && defined(WITH_RUST_ABI_CHECKS)
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDAuthPrimitives.hpp"
bool CheckAbi_network_crypto() { return AbiServicesComplete(GetAuthPrimitives()); }
#endif

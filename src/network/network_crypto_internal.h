/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_crypto_internal.h Native facades for Rust authentication owners. */
#ifndef NETWORK_CRYPTO_INTERNAL_H
#define NETWORK_CRYPTO_INTERNAL_H
#include "network_crypto.h"
#include "../rust/auth_owner.hpp"

constexpr size_t X25519_KEY_SIZE = 32;
constexpr size_t X25519_NONCE_SIZE = 24;
constexpr size_t X25519_MAC_SIZE = 16;
constexpr size_t X25519_KEY_EXCHANGE_MESSAGE_SIZE = 8;

struct X25519Key : std::array<uint8_t, X25519_KEY_SIZE> { ~X25519Key(); };
struct X25519PublicKey : X25519Key {};
struct X25519SecretKey : X25519Key {
	static X25519SecretKey CreateRandom();
	X25519PublicKey CreatePublicKey() const;
};

class RustAuthenticationClientHandler : public NetworkAuthenticationClientHandler {
	friend const OpenTTDAuthHandlerServices &GetAuthHandlerServices();
	std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password_handler;
protected:
	RustAuthHandler handler;
	RustAuthenticationClientHandler(const X25519SecretKey &secret, uint8_t method, std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password = {});
public:
	RustAuthenticationClientHandler(const X25519SecretKey &secret, std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password);
	RequestResult ReceiveRequest(Packet &p) override;
	bool SendResponse(Packet &p) override;
	std::string_view GetName() const override;
	NetworkAuthenticationMethod GetAuthenticationMethod() const override;
	bool ReceiveEnableEncryption(Packet &p) override;
	std::unique_ptr<NetworkEncryptionHandler> CreateClientToServerEncryptionHandler() const override;
	std::unique_ptr<NetworkEncryptionHandler> CreateServerToClientEncryptionHandler() const override;
};

class RustAuthenticationServerHandler : public NetworkAuthenticationServerHandler {
protected:
	RustAuthHandler handler;
	RustAuthenticationServerHandler(const X25519SecretKey &secret, uint8_t method, const NetworkAuthenticationPasswordProvider *password = nullptr, const NetworkAuthenticationAuthorizedKeyHandler *authorized = nullptr);
public:
	RustAuthenticationServerHandler(const NetworkAuthenticationPasswordProvider *password, const NetworkAuthenticationAuthorizedKeyHandler *authorized, NetworkAuthenticationMethodMask mask);
	void SendRequest(Packet &p) override;
	ResponseResult ReceiveResponse(Packet &p) override;
	std::string_view GetName() const override;
	NetworkAuthenticationMethod GetAuthenticationMethod() const override;
	bool CanBeUsed() const override;
	std::string GetPeerPublicKey() const override;
	void SendEnableEncryption(Packet &p) override;
	std::unique_ptr<NetworkEncryptionHandler> CreateClientToServerEncryptionHandler() const override;
	std::unique_ptr<NetworkEncryptionHandler> CreateServerToClientEncryptionHandler() const override;
};

class X25519KeyExchangeOnlyClientHandler : public RustAuthenticationClientHandler {
public:
	X25519KeyExchangeOnlyClientHandler(const X25519SecretKey &secret) : RustAuthenticationClientHandler(secret, 0) {}
};
class X25519KeyExchangeOnlyServerHandler : public RustAuthenticationServerHandler {
public:
	X25519KeyExchangeOnlyServerHandler(const X25519SecretKey &secret) : RustAuthenticationServerHandler(secret, 0) {}
};
class X25519PAKEClientHandler : public RustAuthenticationClientHandler {
public:
	X25519PAKEClientHandler(const X25519SecretKey &secret, std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password) : RustAuthenticationClientHandler(secret, 1, std::move(password)) {}
};
class X25519PAKEServerHandler : public RustAuthenticationServerHandler {
public:
	X25519PAKEServerHandler(const X25519SecretKey &secret, const NetworkAuthenticationPasswordProvider *password) : RustAuthenticationServerHandler(secret, 1, password) {}
};
class X25519AuthorizedKeyClientHandler : public RustAuthenticationClientHandler {
public:
	X25519AuthorizedKeyClientHandler(const X25519SecretKey &secret) : RustAuthenticationClientHandler(secret, 2) {}
	static X25519SecretKey GetValidSecretKeyAndUpdatePublicKey(std::string &secret_key, std::string &public_key);
};
class X25519AuthorizedKeyServerHandler : public RustAuthenticationServerHandler {
public:
	X25519AuthorizedKeyServerHandler(const X25519SecretKey &secret, const NetworkAuthenticationAuthorizedKeyHandler *authorized) : RustAuthenticationServerHandler(secret, 2, nullptr, authorized) {}
};
#endif /* NETWORK_CRYPTO_INTERNAL_H */

/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file network_crypto.cpp Native interfaces for Rust network authentication. */
#include "../stdafx.h"
#include "network_crypto_internal.h"
#include "core/packet.h"
#include "../core/random_func.hpp"
#include "../debug.h"
#include "../rust/crypto_ffi.h"
#include "../rust/services_assert.h"
#include "../string_func.h"
#include "../safeguards.h"

const OpenTTDAuthHandlerServices &GetAuthHandlerServices()
{
	static const OpenTTDAuthHandlerServices services{
		.allowed = [](const NetworkAuthenticationAuthorizedKeyHandler *handler, const uint8_t *peer, size_t size) noexcept -> uint8_t { return handler->IsAllowed(std::string_view(reinterpret_cast<const char *>(peer), size)); },
		.ask_password = [](RustAuthenticationClientHandler *shell) noexcept { shell->password_handler->AskUserForPassword(shell->password_handler); },
		.authorized_available = [](const NetworkAuthenticationAuthorizedKeyHandler *handler) noexcept -> uint8_t { return handler->CanBeUsed(); },
		.client_password = [](const RustAuthenticationClientHandler *shell, size_t *size) noexcept { const auto &password = shell->password_handler->password; *size = password.size(); return reinterpret_cast<const uint8_t *>(password.data()); },
		.packet = &Packet::Services(),
		.random_bytes = [](uint8_t *data, size_t size) noexcept { RandomBytesWithFallback(std::span(data, size)); },
		.server_password = [](const NetworkAuthenticationPasswordProvider *provider, size_t *size) noexcept { auto password = provider->GetPassword(); *size = password.size(); return reinterpret_cast<const uint8_t *>(password.data()); },
	};
	return services;
}

class X25519EncryptionHandler : public NetworkEncryptionHandler {
	RustAuthStream context;
public:
	explicit X25519EncryptionHandler(OpenTTDAuthStream *stream) : context(stream) {}
	X25519EncryptionHandler(const OpenTTDAuthHandler *handler, uint8_t side) : context(openttd_rust_auth_stream_new(handler, side)) {}
	OpenTTDAuthStream *GetRustStream() const noexcept override { return this->context.get(); }
	size_t MACSize() const override { return X25519_MAC_SIZE; }
	bool Decrypt(std::span<uint8_t> mac, std::span<uint8_t> message) override { return openttd_rust_auth_stream_decrypt(this->context.get(), mac.data(), message.data(), message.size()) != 0; }
	void Encrypt(std::span<uint8_t> mac, std::span<uint8_t> message) override { openttd_rust_auth_stream_encrypt(this->context.get(), mac.data(), message.data(), message.size()); }
};

X25519Key::~X25519Key() { openttd_rust_crypto_wipe(this->data(), this->size()); }
X25519SecretKey X25519SecretKey::CreateRandom() { X25519SecretKey secret; RandomBytesWithFallback(secret); return secret; }
X25519PublicKey X25519SecretKey::CreatePublicKey() const { X25519PublicKey key; openttd_rust_crypto_public_key(key.data(), this->data()); return key; }

std::string FormatNetworkPublicKey(const uint8_t *key)
{
	return FormatArrayAsHex(std::span(key, X25519_KEY_SIZE));
}

std::unique_ptr<NetworkEncryptionHandler> MakeNetworkEncryptionHandler(OpenTTDAuthStream *stream)
{
	return std::make_unique<X25519EncryptionHandler>(stream);
}

RustAuthenticationClientHandler::RustAuthenticationClientHandler(const X25519SecretKey &secret, std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password) :
	password_handler(std::move(password)), handler(openttd_rust_auth_client_new(&GetAuthHandlerServices(), secret.data(), this)) {}
RustAuthenticationServerHandler::RustAuthenticationServerHandler(const NetworkAuthenticationPasswordProvider *password, const NetworkAuthenticationAuthorizedKeyHandler *authorized, NetworkAuthenticationMethodMask mask) :
	handler(openttd_rust_auth_server_new(&GetAuthHandlerServices(), password, authorized, mask.base())) {}

NetworkAuthenticationClientHandler::RequestResult RustAuthenticationClientHandler::ReceiveRequest(Packet &p) { return static_cast<RequestResult>(openttd_rust_auth_handler_receive_request(this->handler.get(), p.Record())); }
bool RustAuthenticationClientHandler::SendResponse(Packet &p) { return openttd_rust_auth_handler_send_response(this->handler.get(), p.Record()) != 0; }
bool RustAuthenticationClientHandler::ReceiveEnableEncryption(Packet &p) { return openttd_rust_auth_handler_receive_enable(this->handler.get(), p.Record()) != 0; }
void RustAuthenticationServerHandler::SendRequest(Packet &p) { openttd_rust_auth_handler_send_request(this->handler.get(), p.Record(), RustAssertCondition); }
NetworkAuthenticationServerHandler::ResponseResult RustAuthenticationServerHandler::ReceiveResponse(Packet &p) { return static_cast<ResponseResult>(openttd_rust_auth_handler_receive_response(this->handler.get(), p.Record())); }
void RustAuthenticationServerHandler::SendEnableEncryption(Packet &p) { openttd_rust_auth_handler_send_enable(this->handler.get(), p.Record()); }
bool RustAuthenticationServerHandler::CanBeUsed() const { return openttd_rust_auth_handler_can_be_used(this->handler.get()) != 0; }
std::string RustAuthenticationServerHandler::GetPeerPublicKey() const { return FormatNetworkPublicKey(openttd_rust_auth_handler_peer(this->handler.get())); }

std::string_view RustAuthenticationClientHandler::GetName() const { size_t size; auto name = openttd_rust_auth_handler_name(this->handler.get(), &size); return {reinterpret_cast<const char *>(name), size}; }
std::unique_ptr<NetworkEncryptionHandler> RustAuthenticationClientHandler::CreateClientToServerEncryptionHandler() const { return std::make_unique<X25519EncryptionHandler>(this->handler.get(), 0); }
std::unique_ptr<NetworkEncryptionHandler> RustAuthenticationClientHandler::CreateServerToClientEncryptionHandler() const { return std::make_unique<X25519EncryptionHandler>(this->handler.get(), 1); }

std::string_view RustAuthenticationServerHandler::GetName() const { size_t size; auto name = openttd_rust_auth_handler_name(this->handler.get(), &size); return {reinterpret_cast<const char *>(name), size}; }
std::unique_ptr<NetworkEncryptionHandler> RustAuthenticationServerHandler::CreateClientToServerEncryptionHandler() const { return std::make_unique<X25519EncryptionHandler>(this->handler.get(), 0); }
std::unique_ptr<NetworkEncryptionHandler> RustAuthenticationServerHandler::CreateServerToClientEncryptionHandler() const { return std::make_unique<X25519EncryptionHandler>(this->handler.get(), 1); }

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

void NetworkAuthenticationPasswordRequestHandler::Reply(const std::string &password) { this->password = password; this->SendResponse(); }
void NetworkAuthenticationClientHandler::EnsureValidSecretKeyAndUpdatePublicKey(std::string &secret_key, std::string &public_key) { X25519AuthorizedKeyClientHandler::GetValidSecretKeyAndUpdatePublicKey(secret_key, public_key); }
std::unique_ptr<NetworkAuthenticationClientHandler> NetworkAuthenticationClientHandler::Create(std::shared_ptr<NetworkAuthenticationPasswordRequestHandler> password_handler, std::string &secret_key, std::string &public_key)
{
	auto secret = X25519AuthorizedKeyClientHandler::GetValidSecretKeyAndUpdatePublicKey(secret_key, public_key);
	return std::make_unique<RustAuthenticationClientHandler>(secret, std::move(password_handler));
}
std::unique_ptr<NetworkAuthenticationServerHandler> NetworkAuthenticationServerHandler::Create(const NetworkAuthenticationPasswordProvider *password, const NetworkAuthenticationAuthorizedKeyHandler *authorized, NetworkAuthenticationMethodMask mask)
{
	return std::make_unique<RustAuthenticationServerHandler>(password, authorized, mask);
}

#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDAuthHandlerServices.hpp"
bool CheckAbi_network_crypto() { return AbiServicesComplete(GetAuthHandlerServices()); }

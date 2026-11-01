/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file network_services.h Shared native access to network socket shells. */
#ifndef RUST_NETWORK_SERVICES_H
#define RUST_NETWORK_SERVICES_H
#include "../company_base.h"
#include "../network/network_server.h"
inline bool RustNetworkCompanyAllowsClient(uint8_t company, void *socket) noexcept
{
	return Company::Get(company)->allow_list.Contains(static_cast<NetworkClientSocket *>(socket)->GetPeerPublicKey());
}
#endif /* RUST_NETWORK_SERVICES_H */

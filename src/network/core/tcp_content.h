/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tcp_content.h Basic functions to receive and send TCP packets to/from the content server.
 */

#ifndef NETWORK_CORE_TCP_CONTENT_H
#define NETWORK_CORE_TCP_CONTENT_H

#include "os_abstraction.h"
#include "tcp.h"
#include "packet.h"
#include "../../debug.h"
#include "tcp_content_type.h"

/** Base socket handler for all Content TCP sockets; Rust dispatches its packets. */
class NetworkContentSocketHandler : public NetworkTCPSocketHandler {
public:
	/**
	 * Create a new cs socket handler for a given cs
	 * @param s  the socket we are connected with
	 */
	NetworkContentSocketHandler(SOCKET s = INVALID_SOCKET) :
		NetworkTCPSocketHandler(s)
	{
	}

	/** On destructing of this class, the socket needs to be closed */
	virtual ~NetworkContentSocketHandler()
	{
		/* Virtual functions get called statically in destructors, so make it explicit to remove any confusion. */
		this->CloseSocket();
	}
};

Subdirectory GetContentInfoSubDir(ContentType type);

#endif /* NETWORK_CORE_TCP_CONTENT_H */

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `PacketGameType` from `network/core/tcp_game.h`.

/// Enum with all types of TCP packets, retaining every underlying byte.
/// For the exact meaning, look at `NetworkGameSocketHandler`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketGameType(pub u8);

#[allow(dead_code)] // Complete original enumerator vocabulary.
impl PacketGameType {
    /*
     * These first ten packets must remain in this order for backward and forward compatibility
     * between clients that are trying to join directly. These packets can be received and/or sent
     * by the server before the server has processed the 'join' packet from the client.
     */

    /* Packets sent by socket accepting code without ever constructing a client socket instance. */
    /// The server is full and has no place for you.
    pub const PACKET_SERVER_FULL: Self = Self(0);
    /// The server has banned you.
    pub const PACKET_SERVER_BANNED: Self = Self(1);

    /* Packets used by the client to join and an error message when the revision is wrong. */
    /// The client telling the server it wants to join.
    pub const PACKET_CLIENT_JOIN: Self = Self(2);
    /// Server sending an error message to the client.
    pub const PACKET_SERVER_ERROR: Self = Self(3);

    /* Unused packet types, formerly used for the pre-game lobby. */
    /// Unused.
    pub const PACKET_CLIENT_UNUSED: Self = Self(4);
    /// Unused.
    pub const PACKET_SERVER_UNUSED: Self = Self(5);

    /* Packets used to get the game info. */
    /// Information about the server.
    pub const PACKET_SERVER_GAME_INFO: Self = Self(6);
    /// Request information about the server.
    pub const PACKET_CLIENT_GAME_INFO: Self = Self(7);

    /* A server quitting this game. */
    /// The server is preparing to start a new game.
    pub const PACKET_SERVER_NEWGAME: Self = Self(8);
    /// The server is shutting down.
    pub const PACKET_SERVER_SHUTDOWN: Self = Self(9);

    /*
     * Packets after here assume that the client
     * and server are running the same version. As
     * such ordering is unimportant from here on.
     *
     * The following is the remainder of the packets
     * sent as part of authenticating and getting
     * the map and other important data.
     */

    /* After the join step, the first perform game authentication and enabling encryption. */
    /// The server requests the client to authenticate using a number of methods.
    pub const PACKET_SERVER_AUTH_REQUEST: Self = Self(10);
    /// The client responds to the authentication request.
    pub const PACKET_CLIENT_AUTH_RESPONSE: Self = Self(11);
    /// The server tells that authentication has completed and requests to enable encryption with
    /// the keys of the last `PACKET_CLIENT_AUTH_RESPONSE`.
    pub const PACKET_SERVER_ENABLE_ENCRYPTION: Self = Self(12);

    /* After the authentication is done, the next step is identification. */
    /// Client telling the server the client's name and requested company.
    pub const PACKET_CLIENT_IDENTIFY: Self = Self(13);

    /* After the identify step, the next is checking NewGRFs. */
    /// Server sends `NewGRF` IDs and MD5 checksums for the client to check.
    pub const PACKET_SERVER_CHECK_NEWGRFS: Self = Self(14);
    /// Client acknowledges that it has all required `NewGRFs`.
    pub const PACKET_CLIENT_NEWGRFS_CHECKED: Self = Self(15);

    /* The server welcomes the authenticated client and sends information of other clients. */
    /// Server welcomes you and gives you your `ClientID`.
    pub const PACKET_SERVER_WELCOME: Self = Self(16);
    /// Server sends you information about a client.
    pub const PACKET_SERVER_CLIENT_INFO: Self = Self(17);

    /* Getting the savegame/map. */
    /// Client requests the actual map.
    pub const PACKET_CLIENT_GETMAP: Self = Self(18);
    /// Server tells the client there are some people waiting for the map as well.
    pub const PACKET_SERVER_WAIT: Self = Self(19);
    /// Server tells the client that it is beginning to send the map.
    pub const PACKET_SERVER_MAP_BEGIN: Self = Self(20);
    /// Server tells the client what the (compressed) size of the map is.
    pub const PACKET_SERVER_MAP_SIZE: Self = Self(21);
    /// Server sends bits of the map to the client.
    pub const PACKET_SERVER_MAP_DATA: Self = Self(22);
    /// Server tells it has just sent the last bits of the map to the client.
    pub const PACKET_SERVER_MAP_DONE: Self = Self(23);
    /// Client tells the server that it received the whole map.
    pub const PACKET_CLIENT_MAP_OK: Self = Self(24);

    /// Tells clients that a new client has joined.
    pub const PACKET_SERVER_JOIN: Self = Self(25);

    /*
     * At this moment the client has the map and
     * the client is fully authenticated. Now the
     * normal communication starts.
     */

    /* Game progress monitoring. */
    /// Server tells the client what frame it is in, and thus to where the client may progress.
    pub const PACKET_SERVER_FRAME: Self = Self(26);
    /// The client tells the server which frame it has executed.
    pub const PACKET_CLIENT_ACK: Self = Self(27);
    /// Server tells the client what the random state should be.
    pub const PACKET_SERVER_SYNC: Self = Self(28);

    /* Sending commands around. */
    /// Client executed a command and sends it to the server.
    pub const PACKET_CLIENT_COMMAND: Self = Self(29);
    /// Server distributes a command to (all) the clients.
    pub const PACKET_SERVER_COMMAND: Self = Self(30);

    /* Human communication! */
    /// Client said something that should be distributed.
    pub const PACKET_CLIENT_CHAT: Self = Self(31);
    /// Server distributing the message of a client (or itself).
    pub const PACKET_SERVER_CHAT: Self = Self(32);
    /// Server distributing the message from external source.
    pub const PACKET_SERVER_EXTERNAL_CHAT: Self = Self(33);

    /* Remote console. */
    /// Client asks the server to execute some command.
    pub const PACKET_CLIENT_RCON: Self = Self(34);
    /// Response of the executed command on the server.
    pub const PACKET_SERVER_RCON: Self = Self(35);

    /* Moving a client.*/
    /// A client would like to be moved to another company.
    pub const PACKET_CLIENT_MOVE: Self = Self(36);
    /// Server tells everyone that someone is moved to another company.
    pub const PACKET_SERVER_MOVE: Self = Self(37);

    /* Configuration updates. */
    /// A client changes its name.
    pub const PACKET_CLIENT_SET_NAME: Self = Self(38);
    /// Some network configuration important to the client changed.
    pub const PACKET_SERVER_CONFIG_UPDATE: Self = Self(39);

    /* A client quitting. */
    /// A client tells the server it is going to quit.
    pub const PACKET_CLIENT_QUIT: Self = Self(40);
    /// A server tells that a client has quit.
    pub const PACKET_SERVER_QUIT: Self = Self(41);
    /// A client reports an error to the server.
    pub const PACKET_CLIENT_ERROR: Self = Self(42);
    /// A server tells that a client has hit an error and did quit.
    pub const PACKET_SERVER_ERROR_QUIT: Self = Self(43);

    /// Must ALWAYS be on the end of this list!! (period)
    pub const PACKET_END: Self = Self(44);
}

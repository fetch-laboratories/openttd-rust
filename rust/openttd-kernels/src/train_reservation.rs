/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The path reservation bodies of `train_cmd.cpp`: following, extending, choosing and
//! freeing a train's reserved path, and the order lookahead of `VehicleOrderSaver`.
//!
//! Every body runs on a train entry's tables, borrowed for the synchronous call. A train
//! part is a [`Handle`], live for that call; fields are read and written field-sized at the
//! original's observation points, and no Rust reference into a record or owner crosses a
//! callback.
#![allow(unsafe_code, clippy::too_many_lines)]
use crate::bitmath_func::{assign_bit16, has_bit, kill_first_bit, kill_first_bit_u16};
use crate::direction_func::reverse_diag_dir;
use crate::direction_type::DiagDirection;
use crate::follow_track::{CFollowTrackT, ErrorCode};
use crate::ground_vehicle::{GroundVehicleFlags, Owner as GroundOwner};
use crate::map_func::World;
use crate::order_type::{
    DestinationID, OrderDepotActionFlag, OrderDepotTypeFlag, OrderType, VehicleOrderID,
};
use crate::orders::{Order, OrderView};
use crate::pbs::{PBSTileInfo, Pbs};
use crate::rail_type::{RailType, RailTypes};
use crate::services::Services;
use crate::services_assert::assert_with;
use crate::signal_type::{SignalState, SignalType};
use crate::station_type::{StationFacilities, StationFacility, StationID};
use crate::tile_type::{TileIndex, TileType};
use crate::track_func::{
    diagdir_reaches_trackdirs, diagdir_reaches_tracks, find_first_track, find_first_trackdir,
    is_valid_trackdir, remove_first_trackdir, reverse_trackdir, track_bits_to_track,
    track_bits_to_trackdir_bits, track_crosses_tracks, track_enterdir_to_trackdir,
    track_status_to_trackdir_bits, track_to_track_bits, trackdir_bits_to_track_bits,
    trackdir_crosses_trackdirs, trackdir_reaches_trackdirs, trackdir_to_exitdir, trackdir_to_track,
    tracks_overlap,
};
use crate::track_type::{Track, TrackBits, TrackStatus, Trackdir, TrackdirBits};
use crate::train::{Handle, VehicleRailFlag};
use crate::transport_type::TransportType;
use crate::vehicle::{VehicleServices, maintenance};
use crate::vehicle_type::VehicleID;
use crate::widgets::vehicle_widget::VehicleViewWidgets;
use crate::window_func::set_window_widget_dirty;
use crate::window_type::{WindowClass, WindowNumber};

/// The tables of one reservation entry: a train entry's pair and the map descriptor. It
/// holds no vehicle field.
pub(crate) struct Game<'a> {
    leaves: &'a crate::train::Leaves,
    services: &'a Services,
    vs: &'a VehicleServices,
    /// The canonical map descriptor, loaded once at the entry.
    world: World<'a>,
}

impl<'a> Game<'a> {
    /// The context of a train entry's `leaves` and `services` tables.
    pub(crate) fn new(leaves: &'a crate::train::Leaves, services: &'a Services) -> Self {
        Self {
            leaves,
            services,
            // SAFETY: The native table names the immutable process-wide base table.
            vs: unsafe { &*leaves.vehicle },
            world: World::new(services),
        }
    }
    /// The native assertion handler, as a header helper's precondition check.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.services, ok)
    }
    /// The PBS routines.
    fn pbs(&self) -> Pbs<'a> {
        Pbs::acquire(self.services)
    }
    /// The rail map services the PBS routines name.
    fn rail(&self) -> &'a crate::services::RailMapServices {
        // SAFETY: The PBS table names the immutable process-lifetime rail map table.
        unsafe { &*self.pbs().0.rail }
    }
    /// `v->index`.
    fn index(&self, v: Handle) -> VehicleID {
        v.vehicle().index(self.vs.newgrf())
    }
    /// `v->GetVehicleTrackdir()`.
    fn get_vehicle_trackdir(&self, v: Handle) -> Trackdir {
        crate::train::get_vehicle_trackdir(self.leaves, self.services, v)
    }
    /// `MarkTrainAsStuck(v)`.
    fn mark_train_as_stuck(&self, v: Handle) {
        crate::train::mark_train_as_stuck(self.leaves, self.services, v);
    }
    /// `CheckIfTrainNeedsService(v)`.
    fn check_if_train_needs_service(&self, v: Handle) {
        crate::train::check_if_train_needs_service(self.leaves, self.services, v);
    }
    /// `v->NeedsServicing()`.
    fn needs_servicing(&self, v: Handle) -> bool {
        maintenance::needs_servicing(v.vehicle(), self.vs)
    }
    /// `v->HandlePathfindingResult(path_found)`.
    fn handle_pathfinding_result(&self, v: Handle, path_found: bool) {
        (self.vs.pathfinding_result)(v.vehicle(), path_found);
    }
    /// `ProcessOrders(v)`.
    fn process_orders(&self, v: Handle) -> bool {
        crate::orders::process_orders(v.vehicle(), self.vs.orders())
    }
    /// The orders owner's route view.
    fn orders(&self) -> crate::orders::VisitOrders {
        crate::orders::VisitOrders::new(self.vs.orders())
    }
    /// `v->GetNumOrders()`.
    fn get_num_orders(&self, v: Handle) -> VehicleOrderID {
        self.orders().get_num_orders(v.vehicle())
    }
    /// `v->GetOrder(index)`.
    fn get_order(&self, v: Handle, index: VehicleOrderID) -> *const Order {
        self.orders().get_order(v.vehicle(), index)
    }
    /// `UpdateOrderDest(v, order, conditional_depth, pbs_look_ahead)`.
    fn update_order_dest(
        &self,
        v: Handle,
        order: OrderView<'_>,
        conditional_depth: i32,
        pbs_look_ahead: bool,
    ) -> bool {
        crate::orders::update_order_dest(
            v.vehicle(),
            order,
            conditional_depth,
            pbs_look_ahead,
            self.vs.orders(),
        )
    }
    /// `ProcessConditionalOrder(order, v)`.
    fn process_conditional_order(&self, order: OrderView<'_>, v: Handle) -> VehicleOrderID {
        crate::orders::process_conditional_order(order, v.vehicle(), self.vs.orders())
    }
    /// `v->IncrementRealOrderIndex()`.
    fn increment_real_order_index(&self, v: Handle) {
        crate::orders::increment_real_order_index(v.vehicle(), self.vs.orders());
    }
    /// `v->current_order.ShouldStopAtStation(v, station)`.
    fn should_stop_at_station(&self, v: Handle, station: StationID) -> bool {
        crate::movement::should_stop_at_station(self.services, v.vehicle(), station)
    }
    /// `Station::Get(station)`, resolving both the station and its base.
    fn station_get(&self, station: StationID) -> crate::station_service::LoadingStation {
        // SAFETY: The process-lifetime tables resolve the original `Station::Get` once.
        let stations = unsafe { &*(*(self.services.station_catchment)()).station };
        (stations.station)(station.0)
    }
    /// `st->facilities` of a station `station_get` resolved.
    fn facilities(&self, st: *mut crate::station::BaseStation) -> StationFacilities {
        // SAFETY: As `station_get`.
        let stations = unsafe { &*(*(self.services.station_catchment)()).station };
        StationFacilities((stations.facilities)(st))
    }
    /// `SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP)`.
    fn set_start_stop_dirty(&self, v: Handle) {
        set_window_widget_dirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(self.index(v)),
            VehicleViewWidgets::WID_VV_START_STOP.0,
        );
    }
    /// `_settings_game.pf.reserve_paths`.
    fn reserve_paths(&self) -> bool {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { self.services.settings().reserve_paths.read() }
    }
    /// `_settings_game.pf.path_backoff_interval`.
    fn path_backoff_interval(&self) -> u8 {
        // SAFETY: As above.
        unsafe { self.services.settings().path_backoff_interval.read() }
    }
    /// `_settings_game.difficulty.line_reverse_mode`.
    fn line_reverse_mode(&self) -> bool {
        // SAFETY: As above.
        unsafe { self.services.settings().line_reverse_mode.read() }
    }
    /// `_settings_game.pf.forbid_90_deg`.
    fn forbid_90_deg(&self) -> bool {
        (self.services.forbid_90_deg)()
    }
    /// `_settings_client.gui.show_track_reservation`.
    fn show_track_reservation(&self) -> bool {
        (self.pbs().0.show_reservation)()
    }

    /// `CFollowTrackRail ft(v)`.
    fn follower(&self, v: Handle) -> CFollowTrackT {
        self.follower_override(v, RailTypes::INVALID_RAILTYPES)
    }
    /// `CFollowTrackRail ft(v, railtype_override)`.
    fn follower_override(&self, v: Handle, railtype_override: RailTypes) -> CFollowTrackT {
        CFollowTrackT::vehicle(
            self.services,
            TransportType::TRANSPORT_RAIL,
            Some(v.vehicle()),
            railtype_override,
            true,
        )
    }
    /// `GetAllCompatibleRailTypes(railtypes)`.
    fn get_all_compatible_rail_types(&self, railtypes: RailTypes) -> RailTypes {
        RailTypes((self.services.all_compatible_railtypes)(railtypes.0))
    }
    /// `IsTileType(tile, type)`.
    fn is_tile_type(&self, tile: TileIndex, kind: TileType) -> bool {
        self.world.is_tile_type(tile, kind)
    }
    /// `IsRailDepotTile(tile)`.
    fn is_rail_depot_tile(&self, tile: TileIndex) -> bool {
        self.world.is_rail_depot_tile(tile)
    }
    /// `IsPlainRail(tile)`.
    fn is_plain_rail(&self, tile: TileIndex) -> bool {
        self.world.is_plain_rail(tile)
    }
    /// `GetRailDepotDirection(tile)`.
    fn get_rail_depot_direction(&self, tile: TileIndex) -> DiagDirection {
        self.world.get_rail_depot_direction(tile)
    }
    /// `HasSignalOnTrackdir(tile, trackdir)`.
    fn has_signal_on_trackdir(&self, tile: TileIndex, trackdir: Trackdir) -> bool {
        self.world.has_signal_on_trackdir(tile, trackdir)
    }
    /// `GetSignalType(tile, track)`.
    fn get_signal_type(&self, tile: TileIndex, track: Track) -> SignalType {
        self.world.get_signal_type(tile, track)
    }
    /// `IsPbsSignal(s)`.
    fn is_pbs_signal(s: SignalType) -> bool {
        crate::rail_map::is_pbs_signal(s)
    }
    /// `IsOnewaySignal(tile, track)`.
    fn is_oneway_signal(&self, tile: TileIndex, track: Track) -> bool {
        self.world.is_oneway_signal(tile, track)
    }
    /// `GetSignalStateByTrackdir(tile, trackdir)`.
    fn get_signal_state_by_trackdir(&self, tile: TileIndex, trackdir: Trackdir) -> SignalState {
        self.world.get_signal_state_by_trackdir(tile, trackdir)
    }
    /// `SetSignalStateByTrackdir(tile, trackdir, state)`.
    fn set_signal_state_by_trackdir(
        &self,
        tile: TileIndex,
        trackdir: Trackdir,
        state: SignalState,
    ) {
        (self.rail().set_signal_state)(tile.0, trackdir.0, state.0);
    }
    /// `HasPbsSignalOnTrackdir(tile, td)`.
    fn has_pbs_signal_on_trackdir(&self, tile: TileIndex, td: Trackdir) -> bool {
        (self.pbs().0.has_pbs)(tile.0, td.0)
    }
    /// `HasOnewaySignalBlockingTrackdir(tile, td)`.
    fn has_oneway_signal_blocking_trackdir(&self, tile: TileIndex, td: Trackdir) -> bool {
        (self.pbs().0.blocking)(tile.0, td.0)
    }
    /// `GetTileRailType(tile)`.
    fn get_tile_rail_type(&self, tile: TileIndex) -> RailType {
        RailType(self.world.tile_rail_type(tile.0))
    }
    /// `Rail90DegTurnDisallowed(rt1, rt2)` with the default `_settings_game.pf.forbid_90_deg`.
    fn rail_90_deg_turn_disallowed(&self, rt1: RailType, rt2: RailType) -> bool {
        (self.pbs().0.rail90)(rt1.0, rt2.0, self.forbid_90_deg())
    }
    /// `HasDepotReservation(tile)`.
    fn has_depot_reservation(&self, tile: TileIndex) -> bool {
        (self.rail().depot_reserved)(tile.0)
    }
    /// `SetDepotReservation(tile, b)`.
    fn set_depot_reservation(&self, tile: TileIndex, b: bool) {
        (self.pbs().0.set_depot)(tile.0, b);
    }
    /// `SetTunnelBridgeReservation(tile, b)`.
    fn set_tunnel_bridge_reservation(&self, tile: TileIndex, b: bool) {
        (self.pbs().0.set_wormhole)(tile.0, b);
    }
    /// `IsBridge(tile)`.
    fn is_bridge(&self, tile: TileIndex) -> bool {
        self.world.bridge(tile.0)
    }
    /// `GetTunnelBridgeDirection(tile)`.
    fn get_tunnel_bridge_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection(self.world.tunnel_bridge_dir(tile.0))
    }
    /// `GetOtherTunnelBridgeEnd(tile)`.
    fn get_other_tunnel_bridge_end(&self, tile: TileIndex) -> TileIndex {
        TileIndex((self.services.other_tunnel_end)(tile.0))
    }
    /// `TunnelBridgeIsFree(tile, endtile, ignore).Succeeded()`.
    fn tunnel_bridge_is_free(&self, tile: TileIndex, endtile: TileIndex, ignore: Handle) -> bool {
        // SAFETY: The current train and every vehicle on both tiles survive this query.
        unsafe {
            crate::vehicle::occupancy::wormhole_blocker(
                self.vs,
                self.world.dimensions,
                tile.0,
                endtile.0,
                ignore.vehicle().0.as_ptr(),
            )
            .is_none()
        }
    }
    /// `IsRailStationTile(tile)`.
    fn is_rail_station_tile(&self, tile: TileIndex) -> bool {
        self.world.is_rail_station_tile(tile)
    }
    /// `IsRailWaypointTile(tile)`.
    fn is_rail_waypoint_tile(&self, tile: TileIndex) -> bool {
        self.world.is_rail_waypoint_tile(tile)
    }
    /// `GetStationIndex(tile)`.
    fn get_station_index(&self, tile: TileIndex) -> StationID {
        self.world.get_station_index(tile)
    }
    /// `IsCompatibleTrainStationTile(test_tile, station_tile)`.
    fn is_compatible_train_station_tile(
        &self,
        test_tile: TileIndex,
        station_tile: TileIndex,
    ) -> bool {
        self.world
            .is_compatible_train_station_tile(test_tile, station_tile)
    }
    /// `GetDepotIndex(tile)`.
    fn get_depot_index(&self, tile: TileIndex) -> crate::depot_type::DepotID {
        crate::depot_type::DepotID((self.services.depot_index)(tile.0))
    }
    /// `TileAddByDiagDir(tile, dir)`.
    fn tile_add_by_diag_dir(&self, tile: TileIndex, dir: DiagDirection) -> TileIndex {
        self.world
            .dimensions
            .tile_add_by_diag_dir(tile, dir, self.check())
    }
    /// `TileOffsByDiagDir(dir)`.
    fn tile_offs_by_diag_dir(&self, dir: DiagDirection) -> crate::map_type::TileIndexDiff {
        self.world
            .dimensions
            .tile_offs_by_diag_dir(dir, self.check())
    }
    /// `GetTileTrackStatus(tile, TRANSPORT_RAIL, 0)`.
    fn get_tile_track_status(&self, tile: TileIndex) -> TrackStatus {
        crate::movement::table(self.services).get_tile_track_status_in_world(
            &self.world,
            tile,
            TransportType::TRANSPORT_RAIL,
            0,
            DiagDirection::INVALID_DIAGDIR,
        )
    }
    /// `MarkTileDirtyByTile(tile)`.
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.services.mark_dirty)(tile.0);
    }
    /// `MarkBridgeDirty(tile)`.
    fn mark_bridge_dirty(&self, tile: TileIndex) {
        (self.services.mark_bridge_dirty)(tile.0);
    }
    /// `AddSideToSignalBuffer(tile, side, owner)`.
    fn add_side_to_signal_buffer(
        &self,
        tile: TileIndex,
        side: DiagDirection,
        owner: crate::company_type::Owner,
    ) {
        crate::signal::context(self.services).add_side_to_signal_buffer(tile, side, owner);
    }
    /// `UpdateSignalsInBuffer()`.
    fn update_signals_in_buffer(&self) {
        crate::signal::context(self.services).update_signals_in_buffer();
    }
    /// `GetReservedTrackbits(t)`.
    fn get_reserved_trackbits(&self, t: TileIndex) -> TrackBits {
        self.pbs().get_reserved_trackbits(t)
    }
    /// `HasReservedTracks(tile, tracks)`.
    fn has_reserved_tracks(&self, tile: TileIndex, tracks: TrackBits) -> bool {
        self.pbs().has_reserved_tracks(tile, tracks)
    }
    /// `TryReserveRailTrack(tile, t)`, which triggers the stations by default.
    fn try_reserve_rail_track(&self, tile: TileIndex, t: Track) -> bool {
        self.pbs().try_reserve_rail_track(tile, t, true)
    }
    /// `UnreserveRailTrack(tile, t)`.
    fn unreserve_rail_track(&self, tile: TileIndex, t: Track) {
        self.pbs().unreserve_rail_track(tile, t);
    }
    /// `SetRailStationPlatformReservation(start, dir, b)`.
    fn set_rail_station_platform_reservation(&self, start: TileIndex, dir: DiagDirection, b: bool) {
        self.pbs()
            .set_rail_station_platform_reservation(start, dir, b);
    }
    /// `FollowTrainReservation(v, train_on_res)`.
    fn follow_train_reservation(
        &self,
        v: Handle,
        train_on_res: Option<&mut Handle>,
    ) -> PBSTileInfo {
        self.pbs().follow_train_reservation(v, train_on_res)
    }
    /// `IsSafeWaitingPosition(v, tile, trackdir, include_line_end, forbid_90deg)`.
    fn is_safe_waiting_position(
        &self,
        v: Handle,
        tile: TileIndex,
        trackdir: Trackdir,
        include_line_end: bool,
        forbid_90deg: bool,
    ) -> bool {
        self.pbs()
            .is_safe_waiting_position(v, tile, trackdir, include_line_end, forbid_90deg)
    }
    /// `IsWaitingPositionFree(v, tile, trackdir, forbid_90deg)`.
    fn is_waiting_position_free(
        &self,
        v: Handle,
        tile: TileIndex,
        trackdir: Trackdir,
        forbid_90deg: bool,
    ) -> bool {
        self.pbs()
            .is_waiting_position_free(v, tile, trackdir, forbid_90deg)
    }
    /// `TryReserveSafeTrack(v, tile, td, override_railtype)`: try to reserve any path to a
    /// safe tile, ignoring the vehicle's destination. Safe tiles are tiles in front of a
    /// signal, depots and station tiles at end of line. `override_railtype` says whether all
    /// physically compatible railtypes should be followed. Returns true if a path to a safe
    /// stopping tile could be reserved.
    fn try_reserve_safe_track(
        &self,
        v: Handle,
        tile: TileIndex,
        td: Trackdir,
        override_railtype: bool,
    ) -> bool {
        crate::rail_yapf::yapf_train_find_nearest_safe_tile(
            self.services,
            v,
            tile,
            td,
            override_railtype,
        )
    }
}

/// `CheckNextTrainTile`: check if the train is on the last reserved tile and try to extend
/// the path then.
pub(crate) fn check_next_train_tile(g: &Game, v: Handle) {
    /* Don't do any look-ahead if path_backoff_interval is 255. */
    if g.path_backoff_interval() == 255 {
        return;
    }

    /* Exit if we are inside a depot. */
    if v.track() == TrackBits::TRACK_BIT_DEPOT {
        return;
    }

    match v.vehicle().current_order().kind() {
        /* Exit if we reached our destination depot. */
        OrderType::OT_GOTO_DEPOT if v.vehicle().tile() == v.vehicle().dest_tile() => return,

        OrderType::OT_GOTO_WAYPOINT => {
            /* If we reached our waypoint, make sure we see that. */
            if g.is_rail_waypoint_tile(v.vehicle().tile())
                && g.get_station_index(v.vehicle().tile())
                    == v.vehicle().current_order().destination().to_station_id()
            {
                g.process_orders(v);
            }
        }

        /* Exit if the current order doesn't have a destination, but the train has orders. */
        OrderType::OT_NOTHING | OrderType::OT_LEAVESTATION | OrderType::OT_LOADING
            if g.get_num_orders(v).0 > 0 =>
        {
            return;
        }

        _ => {}
    }
    /* Exit if we are on a station tile and are going to stop. */
    if g.is_rail_station_tile(v.vehicle().tile())
        && g.should_stop_at_station(v, g.get_station_index(v.vehicle().tile()))
    {
        return;
    }

    let td = g.get_vehicle_trackdir(v);

    /* On a tile with a red non-pbs signal, don't look ahead. */
    if g.is_tile_type(v.vehicle().tile(), TileType::MP_RAILWAY)
        && g.has_signal_on_trackdir(v.vehicle().tile(), td)
        && !Game::is_pbs_signal(
            g.get_signal_type(v.vehicle().tile(), trackdir_to_track(td, g.check())),
        )
        && g.get_signal_state_by_trackdir(v.vehicle().tile(), td) == SignalState::SIGNAL_STATE_RED
    {
        return;
    }

    let mut ft = g.follower(v);
    if !ft.follow(g.services, v.vehicle().tile(), td) {
        return;
    }

    if !g.has_reserved_tracks(ft.new_tile, trackdir_bits_to_track_bits(ft.new_td_bits)) {
        /* Next tile is not reserved. */
        if kill_first_bit_u16(ft.new_td_bits.0) == TrackdirBits::TRACKDIR_BIT_NONE.0
            && g.has_pbs_signal_on_trackdir(
                ft.new_tile,
                find_first_trackdir(ft.new_td_bits, g.check()),
            )
        {
            /* If the next tile is a PBS signal, try to make a reservation. */
            let mut tracks = trackdir_bits_to_track_bits(ft.new_td_bits);
            if g.rail_90_deg_turn_disallowed(
                g.get_tile_rail_type(ft.old_tile),
                g.get_tile_rail_type(ft.new_tile),
            ) {
                tracks &= !track_crosses_tracks(trackdir_to_track(ft.old_td, g.check()), g.check());
            }
            choose_train_track(g, v, ft.new_tile, ft.exitdir, tracks, false, false);
        }
    }
}

/// `ClearPathReservation`: clear the reservation of `tile` that was just left by a wagon on
/// `track_dir`. `v` is the train owning the reservation.
pub(crate) fn clear_path_reservation(g: &Game, v: Handle, tile: TileIndex, track_dir: Trackdir) {
    let dir = trackdir_to_exitdir(track_dir, g.check());

    if g.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
        /* Are we just leaving a tunnel/bridge? */
        if g.get_tunnel_bridge_direction(tile) == reverse_diag_dir(dir, g.check()) {
            let end = g.get_other_tunnel_bridge_end(tile);

            if g.tunnel_bridge_is_free(tile, end, v) {
                /* Free the reservation only if no other train is on the tiles. */
                g.set_tunnel_bridge_reservation(tile, false);
                g.set_tunnel_bridge_reservation(end, false);

                if g.show_track_reservation() {
                    if g.is_bridge(tile) {
                        g.mark_bridge_dirty(tile);
                    } else {
                        g.mark_tile_dirty_by_tile(tile);
                        g.mark_tile_dirty_by_tile(end);
                    }
                }
            }
        }
    } else if g.is_rail_station_tile(tile) {
        let new_tile = g.tile_add_by_diag_dir(tile, dir);
        /* If the new tile is not a further tile of the same station, we
         * clear the reservation for the whole platform. */
        if !g.is_compatible_train_station_tile(new_tile, tile) {
            g.set_rail_station_platform_reservation(tile, reverse_diag_dir(dir, g.check()), false);
        }
    } else {
        /* Any other tile */
        g.unreserve_rail_track(tile, trackdir_to_track(track_dir, g.check()));
    }
}

/// `FreeTrainTrackReservation`: free the reserved path in front of a vehicle. `v` is the
/// train owning the reserved path.
#[allow(clippy::nonminimal_bool)] // `FreeTrainTrackReservation`'s condition as written.
pub(crate) fn free_train_track_reservation(g: &Game, v: Handle) {
    crate::witness::hit(crate::witness::TRAIN_FREE_PATH);
    assert_with(|| v.is_front_engine());

    let mut tile = v.vehicle().tile();
    let mut td = g.get_vehicle_trackdir(v);
    let mut free_tile = !(g.is_rail_station_tile(v.vehicle().tile())
        || g.is_tile_type(v.vehicle().tile(), TileType::MP_TUNNELBRIDGE));
    let station_id = if g.is_rail_station_tile(v.vehicle().tile()) {
        g.get_station_index(v.vehicle().tile())
    } else {
        StationID::INVALID
    };

    /* Can't be holding a reservation if we enter a depot. */
    if g.is_rail_depot_tile(tile)
        && trackdir_to_exitdir(td, g.check()) != g.get_rail_depot_direction(tile)
    {
        return;
    }
    if v.track() == TrackBits::TRACK_BIT_DEPOT {
        /* Front engine is in a depot. We enter if some part is not in the depot. */
        let mut u = v;
        while u != Handle::NONE {
            if u.track() != TrackBits::TRACK_BIT_DEPOT || u.vehicle().tile() != v.vehicle().tile() {
                return;
            }
            u = u.next();
        }
    }
    /* Don't free reservation if it's not ours. */
    if tracks_overlap(
        g.get_reserved_trackbits(tile)
            | track_to_track_bits(trackdir_to_track(td, g.check()), g.check()),
    ) {
        return;
    }

    let mut ft = g.follower_override(v, g.get_all_compatible_rail_types(v.railtypes()));
    while ft.follow(g.services, tile, td) {
        tile = ft.new_tile;
        let mut bits = ft.new_td_bits & track_bits_to_trackdir_bits(g.get_reserved_trackbits(tile));
        td = remove_first_trackdir(&mut bits, g.check());
        assert_with(|| bits == TrackdirBits::TRACKDIR_BIT_NONE);

        if !is_valid_trackdir(td) {
            break;
        }

        if g.is_tile_type(tile, TileType::MP_RAILWAY) {
            if g.has_signal_on_trackdir(tile, td)
                && !Game::is_pbs_signal(g.get_signal_type(tile, trackdir_to_track(td, g.check())))
            {
                /* Conventional signal along trackdir: remove reservation and stop. */
                g.unreserve_rail_track(tile, trackdir_to_track(td, g.check()));
                break;
            }
            if g.has_pbs_signal_on_trackdir(tile, td) {
                if g.get_signal_state_by_trackdir(tile, td) == SignalState::SIGNAL_STATE_RED {
                    /* Red PBS signal? Can't be our reservation, would be green then. */
                    break;
                }
                /* Turn the signal back to red. */
                g.set_signal_state_by_trackdir(tile, td, SignalState::SIGNAL_STATE_RED);
                g.mark_tile_dirty_by_tile(tile);
            } else if g.has_pbs_signal_on_trackdir(tile, reverse_trackdir(td, g.check())) {
                /* Reservation passes an opposing path signal. Mark signal for update to re-establish the proper default state. */
                g.add_side_to_signal_buffer(
                    tile,
                    trackdir_to_exitdir(reverse_trackdir(td, g.check()), g.check()),
                    v.vehicle().owner(),
                );
            } else if g.has_signal_on_trackdir(tile, reverse_trackdir(td, g.check()))
                && g.is_oneway_signal(tile, trackdir_to_track(td, g.check()))
            {
                break;
            }
        }

        /* Don't free first station/bridge/tunnel if we are on it. */
        if free_tile
            || (!(ft.is_station && g.get_station_index(ft.new_tile) == station_id)
                && !ft.is_tunnel
                && !ft.is_bridge)
        {
            clear_path_reservation(g, v, tile, td);
        }

        free_tile = true;
    }

    g.update_signals_in_buffer();
}

/// `ExtendTrainReservation`: extend a train path as far as possible. Stops on encountering
/// a safe tile, another reservation or a track choice. A tile of `INVALID_TILE` indicates
/// that the reservation failed. On a choice, `new_tracks` and `enterdir` receive the
/// choice tile's tracks and entry direction.
fn extend_train_reservation(
    g: &Game,
    v: Handle,
    new_tracks: &mut TrackBits,
    enterdir: &mut DiagDirection,
) -> PBSTileInfo {
    crate::witness::hit(crate::witness::TRAIN_CONTROLLER_EXTENSION);
    let origin = g.follow_train_reservation(v, None);

    let mut ft = g.follower(v);

    let mut signals_set_to_red: Vec<(TileIndex, Trackdir)> = Vec::new();

    let mut tile = origin.tile;
    let mut cur_td = origin.trackdir;
    while ft.follow(g.services, tile, cur_td) {
        /* Possible signal tile. */
        if kill_first_bit_u16(ft.new_td_bits.0) == TrackdirBits::TRACKDIR_BIT_NONE.0
            && g.has_oneway_signal_blocking_trackdir(
                ft.new_tile,
                find_first_trackdir(ft.new_td_bits, g.check()),
            )
        {
            break;
        }

        if g.rail_90_deg_turn_disallowed(
            g.get_tile_rail_type(ft.old_tile),
            g.get_tile_rail_type(ft.new_tile),
        ) {
            ft.new_td_bits &= !trackdir_crosses_trackdirs(ft.old_td, g.check());
            if ft.new_td_bits == TrackdirBits::TRACKDIR_BIT_NONE {
                break;
            }
        }

        /* Station, depot or waypoint are a possible target. */
        let target_seen = ft.is_station
            || (g.is_tile_type(ft.new_tile, TileType::MP_RAILWAY) && !g.is_plain_rail(ft.new_tile));
        if target_seen || kill_first_bit_u16(ft.new_td_bits.0) != TrackdirBits::TRACKDIR_BIT_NONE.0
        {
            /* Choice found or possible target encountered.
             * On finding a possible target, we need to stop and let the pathfinder handle the
             * remaining path. This is because we don't know if this target is in one of our
             * orders, so we might cause pathfinding to fail later on if we find a choice.
             * This failure would cause a bogous call to TryReserveSafePath which might reserve
             * a wrong path not leading to our next destination. */
            if g.has_reserved_tracks(
                ft.new_tile,
                trackdir_bits_to_track_bits(trackdir_reaches_trackdirs(ft.old_td, g.check())),
            ) {
                break;
            }

            /* If we did skip some tiles, backtrack to the first skipped tile so the pathfinder
             * actually starts its search at the first unreserved tile. */
            if ft.tiles_skipped != 0 {
                ft.new_tile = ft.new_tile
                    - g.tile_offs_by_diag_dir(ft.exitdir)
                        .wrapping_mul(ft.tiles_skipped);
            }

            /* Choice found, path valid but not okay. Save info about the choice tile as well. */
            *new_tracks = trackdir_bits_to_track_bits(ft.new_td_bits);
            *enterdir = ft.exitdir;
            return PBSTileInfo {
                tile: ft.new_tile,
                trackdir: ft.old_td,
                okay: false,
            };
        }

        tile = ft.new_tile;
        cur_td = find_first_trackdir(ft.new_td_bits, g.check());

        let rev_td = reverse_trackdir(cur_td, g.check());
        if g.is_safe_waiting_position(v, tile, cur_td, true, g.forbid_90_deg()) {
            let wp_free = g.is_waiting_position_free(v, tile, cur_td, g.forbid_90_deg());
            if !(wp_free && g.try_reserve_rail_track(tile, trackdir_to_track(cur_td, g.check()))) {
                break;
            }
            /* Green path signal opposing the path? Turn to red. */
            if g.has_pbs_signal_on_trackdir(tile, rev_td)
                && g.get_signal_state_by_trackdir(tile, rev_td) == SignalState::SIGNAL_STATE_GREEN
            {
                crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
                signals_set_to_red.push((tile, rev_td));
                g.set_signal_state_by_trackdir(tile, rev_td, SignalState::SIGNAL_STATE_RED);
                g.mark_tile_dirty_by_tile(tile);
            }
            /* Safe position is all good, path valid and okay. */
            return PBSTileInfo {
                tile,
                trackdir: cur_td,
                okay: true,
            };
        }

        if !g.try_reserve_rail_track(tile, trackdir_to_track(cur_td, g.check())) {
            break;
        }

        /* Green path signal opposing the path? Turn to red. */
        if g.has_pbs_signal_on_trackdir(tile, rev_td)
            && g.get_signal_state_by_trackdir(tile, rev_td) == SignalState::SIGNAL_STATE_GREEN
        {
            crate::witness::hit(crate::witness::TRAIN_EXTENSION_OPPOSING_RED);
            signals_set_to_red.push((tile, rev_td));
            g.set_signal_state_by_trackdir(tile, rev_td, SignalState::SIGNAL_STATE_RED);
            g.mark_tile_dirty_by_tile(tile);
        }
    }

    if ft.err == ErrorCode::EC_OWNER || ft.err == ErrorCode::EC_NO_WAY {
        /* End of line, path valid and okay. */
        return PBSTileInfo {
            tile: ft.old_tile,
            trackdir: ft.old_td,
            okay: true,
        };
    }

    crate::witness::hit(crate::witness::TRAIN_EXTENSION_FAIL);
    /* Sorry, can't reserve path, back out. */
    tile = origin.tile;
    cur_td = origin.trackdir;
    let stopped = ft.old_tile;
    let stopped_td = ft.old_td;
    while tile != stopped || cur_td != stopped_td {
        if !ft.follow(g.services, tile, cur_td) {
            break;
        }

        if g.rail_90_deg_turn_disallowed(
            g.get_tile_rail_type(ft.old_tile),
            g.get_tile_rail_type(ft.new_tile),
        ) {
            ft.new_td_bits &= !trackdir_crosses_trackdirs(ft.old_td, g.check());
            assert_with(|| ft.new_td_bits != TrackdirBits::TRACKDIR_BIT_NONE);
        }
        assert_with(|| kill_first_bit_u16(ft.new_td_bits.0) == TrackdirBits::TRACKDIR_BIT_NONE.0);

        tile = ft.new_tile;
        cur_td = find_first_trackdir(ft.new_td_bits, g.check());

        crate::witness::hit(crate::witness::TRAIN_EXTENSION_ROLLBACK);
        g.unreserve_rail_track(tile, trackdir_to_track(cur_td, g.check()));
    }

    /* Re-instate green signals we turned to red. */
    for (sig_tile, td) in signals_set_to_red {
        crate::witness::hit(crate::witness::TRAIN_EXTENSION_RESTORE);
        g.set_signal_state_by_trackdir(sig_tile, td, SignalState::SIGNAL_STATE_GREEN);
    }

    /* Path invalid. */
    PBSTileInfo::default()
}

/// `VehicleOrderSaver`: this class will save the current order of a vehicle and restore it
/// on destruction.
struct VehicleOrderSaver<'g, 'a> {
    g: &'g Game<'a>,
    v: Handle,
    old_order: Order,
    old_dest_tile: TileIndex,
    old_last_station_visited: StationID,
    index: VehicleOrderID,
    suppress_implicit_orders: bool,
    restored: bool,
}

impl<'g, 'a> VehicleOrderSaver<'g, 'a> {
    fn new(g: &'g Game<'a>, v: Handle) -> Self {
        Self {
            g,
            v,
            old_order: v.vehicle().current_order().copied(),
            old_dest_tile: v.vehicle().dest_tile(),
            old_last_station_visited: v.vehicle().last_station_visited(),
            index: v.vehicle().cur_real_order_index(),
            suppress_implicit_orders: has_bit(
                GroundOwner::of(v.vehicle()).gv_flags(),
                GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0,
            ),
            restored: false,
        }
    }

    /// `Restore`: restore the saved order to the vehicle.
    fn restore(&mut self) {
        crate::witness::hit(crate::witness::TRAIN_ORDER_RESTORE);
        // SAFETY: The vehicle's live current order; the copy creates no reference.
        unsafe {
            self.v
                .vehicle()
                .current_order_pointer()
                .write(self.old_order);
        }
        self.v.vehicle().set_dest_tile(self.old_dest_tile);
        self.v
            .vehicle()
            .set_last_station_visited(self.old_last_station_visited);
        let ground = GroundOwner::of(self.v.vehicle());
        let mut gv_flags = ground.gv_flags();
        assign_bit16(
            &mut gv_flags,
            GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0,
            self.suppress_implicit_orders,
        );
        ground.set_gv_flags(gv_flags);
        self.restored = true;
    }

    /// `SwitchToNextOrder`: set the current vehicle order to the next order in the order
    /// list. `skip_first` says whether the first (i.e. active) order shall be skipped.
    /// Returns true if a suitable next order could be found.
    fn switch_to_next_order(&mut self, skip_first: bool) -> bool {
        crate::witness::hit(crate::witness::TRAIN_ORDER_LOOKAHEAD);
        let g = self.g;
        let v = self.v;
        if g.get_num_orders(v).0 == 0 {
            return false;
        }

        if skip_first {
            self.index = VehicleOrderID(self.index.0.wrapping_add(1));
        }

        let mut depth = 0_i32;

        loop {
            /* Wrap around. */
            if self.index >= g.get_num_orders(v) {
                self.index = VehicleOrderID(0);
            }

            let order = g.get_order(v, self.index);
            assert_with(|| !order.is_null());

            // SAFETY: The list has orders and the index is below their count, so `order`
            // names one of them, live until the list changes; each read below ends before
            // the call that follows it.
            let view = unsafe { OrderView::of(order) };
            let goto = match view.kind() {
                /* Skip service in depot orders when the train doesn't need service. */
                OrderType::OT_GOTO_DEPOT
                    if view.depot_type().test(OrderDepotTypeFlag::Service)
                        && !g.needs_servicing(v) =>
                {
                    false
                }
                OrderType::OT_GOTO_DEPOT
                | OrderType::OT_GOTO_STATION
                | OrderType::OT_GOTO_WAYPOINT => true,
                OrderType::OT_CONDITIONAL => {
                    let next = g.process_conditional_order(view, v);
                    if next != VehicleOrderID::INVALID_VEH_ORDER_ID {
                        depth += 1;
                        self.index = next;
                        /* Don't increment next, so no break here. */
                        if self.index != v.vehicle().cur_real_order_index()
                            && depth < i32::from(g.get_num_orders(v).0)
                        {
                            continue;
                        }
                        return false;
                    }
                    false
                }
                _ => false,
            };
            if goto {
                // SAFETY: As above; the copy creates no reference.
                unsafe { v.vehicle().current_order_pointer().write(view.copied()) };
                return g.update_order_dest(v, view, 0, true);
            }
            /* Don't increment inside the while because otherwise conditional
             * orders can lead to an infinite loop. */
            self.index = VehicleOrderID(self.index.0.wrapping_add(1));
            depth += 1;
            if !(self.index != v.vehicle().cur_real_order_index()
                && depth < i32::from(g.get_num_orders(v).0))
            {
                return false;
            }
        }
    }
}

/// `~VehicleOrderSaver`: restore the saved order to the vehicle, if `Restore()` has not
/// already been called.
impl Drop for VehicleOrderSaver<'_, '_> {
    fn drop(&mut self) {
        if !self.restored {
            self.restore();
        }
    }
}

/// `ChooseTrainTrack`: choose a track. Returns the track and whether a reservation was
/// made, in place of the original's `got_reservation` output, whose writes no reader sees
/// before the return.
pub(crate) fn choose_train_track(
    g: &Game,
    v: Handle,
    tile: TileIndex,
    enterdir: DiagDirection,
    mut tracks: TrackBits,
    force_res: bool,
    mark_stuck: bool,
) -> (Track, bool) {
    crate::witness::hit(crate::witness::TRAIN_CHOOSE_TRACK);
    let mut best_track = Track::INVALID_TRACK;
    let mut do_track_reservation = g.reserve_paths() || force_res;
    let mut changed_signal = false;
    let mut final_dest = TileIndex::INVALID;

    assert_with(|| tracks & !TrackBits::TRACK_BIT_MASK == TrackBits::TRACK_BIT_NONE);

    let mut got_reservation = false;

    /* Don't use tracks here as the setting to forbid 90 deg turns might have been switched between reservation and now. */
    let res_tracks = g.get_reserved_trackbits(tile) & diagdir_reaches_tracks(enterdir, g.check());
    /* Do we have a suitable reserved track? */
    if res_tracks != TrackBits::TRACK_BIT_NONE {
        return (find_first_track(res_tracks), got_reservation);
    }

    /* Quick return in case only one possible track is available */
    if kill_first_bit(tracks.0) == TrackBits::TRACK_BIT_NONE.0 {
        let track = find_first_track(tracks);
        /* We need to check for signals only here, as a junction tile can't have signals. */
        if track != Track::INVALID_TRACK
            && g.has_pbs_signal_on_trackdir(
                tile,
                track_enterdir_to_trackdir(track, enterdir, g.check()),
            )
        {
            do_track_reservation = true;
            changed_signal = true;
            g.set_signal_state_by_trackdir(
                tile,
                track_enterdir_to_trackdir(track, enterdir, g.check()),
                SignalState::SIGNAL_STATE_GREEN,
            );
        } else if !do_track_reservation {
            return (track, got_reservation);
        }
        best_track = track;
    }

    let mut res_dest = PBSTileInfo {
        tile,
        trackdir: Trackdir::INVALID_TRACKDIR,
        okay: false,
    };
    let mut dest_enterdir = enterdir;
    if do_track_reservation {
        res_dest = extend_train_reservation(g, v, &mut tracks, &mut dest_enterdir);
        if res_dest.tile == TileIndex::INVALID {
            /* Reservation failed? */
            if mark_stuck {
                g.mark_train_as_stuck(v);
            }
            if changed_signal {
                g.set_signal_state_by_trackdir(
                    tile,
                    track_enterdir_to_trackdir(best_track, enterdir, g.check()),
                    SignalState::SIGNAL_STATE_RED,
                );
            }
            return (find_first_track(tracks), got_reservation);
        }
        if res_dest.okay {
            /* Got a valid reservation that ends at a safe target, quick exit. */
            got_reservation = true;
            if changed_signal {
                g.mark_tile_dirty_by_tile(tile);
            }
            g.try_reserve_rail_track(
                v.vehicle().tile(),
                trackdir_to_track(g.get_vehicle_trackdir(v), g.check()),
            );
            return (best_track, got_reservation);
        }

        /* Check if the train needs service here, so it has a chance to always find a depot.
         * Also check if the current order is a service order so we don't reserve a path to
         * the destination but instead to the next one if service isn't needed. */
        g.check_if_train_needs_service(v);
        if v.vehicle().current_order().is_type(OrderType::OT_DUMMY)
            || v.vehicle()
                .current_order()
                .is_type(OrderType::OT_CONDITIONAL)
            || v.vehicle()
                .current_order()
                .is_type(OrderType::OT_GOTO_DEPOT)
        {
            g.process_orders(v);
        }
    }

    /* Save the current train order. The destructor will restore the old order on function exit. */
    let mut orders = VehicleOrderSaver::new(g, v);

    /* If the current tile is the destination of the current order and
     * a reservation was requested, advance to the next order.
     * Don't advance on a depot order as depots are always safe end points
     * for a path and no look-ahead is necessary. This also avoids a
     * problem with depot orders not part of the order list when the
     * order list itself is empty. */
    if v.vehicle()
        .current_order()
        .is_type(OrderType::OT_LEAVESTATION)
    {
        orders.switch_to_next_order(false);
    } else if v.vehicle().current_order().is_type(OrderType::OT_LOADING)
        || (!v
            .vehicle()
            .current_order()
            .is_type(OrderType::OT_GOTO_DEPOT)
            && if v
                .vehicle()
                .current_order()
                .is_type(OrderType::OT_GOTO_STATION)
            {
                g.is_rail_station_tile(v.vehicle().tile())
                    && v.vehicle().current_order().destination().to_station_id()
                        == g.get_station_index(v.vehicle().tile())
            } else {
                v.vehicle().tile() == v.vehicle().dest_tile()
            })
    {
        orders.switch_to_next_order(true);
    }

    if res_dest.tile != TileIndex::INVALID && !res_dest.okay {
        /* Pathfinders are able to tell that route was only 'guessed'. */
        let mut path_found = true;
        let new_tile = res_dest.tile;

        /* `DoTrainPathfind`: initialise the final destination, then perform the
         * pathfinding. */
        final_dest = TileIndex::INVALID;
        let next_track = crate::rail_yapf::yapf_train_choose_track(
            g.services,
            v,
            new_tile,
            dest_enterdir,
            tracks,
            &mut path_found,
            do_track_reservation,
            Some(&mut res_dest),
            Some(&mut final_dest),
        );
        if new_tile == tile {
            best_track = next_track;
        }
        g.handle_pathfinding_result(v, path_found);
    }

    /* No track reservation requested -> finished. */
    if !do_track_reservation {
        return (best_track, got_reservation);
    }

    /* A path was found, but could not be reserved. */
    if res_dest.tile != TileIndex::INVALID && !res_dest.okay {
        if mark_stuck {
            g.mark_train_as_stuck(v);
        }
        free_train_track_reservation(g, v);
        return (best_track, got_reservation);
    }

    /* No possible reservation target found, we are probably lost. */
    if res_dest.tile == TileIndex::INVALID {
        /* Try to find any safe destination. */
        let origin = g.follow_train_reservation(v, None);
        if g.try_reserve_safe_track(v, origin.tile, origin.trackdir, false) {
            let res = g.get_reserved_trackbits(tile) & diagdir_reaches_tracks(enterdir, g.check());
            best_track = find_first_track(res);
            g.try_reserve_rail_track(
                v.vehicle().tile(),
                trackdir_to_track(g.get_vehicle_trackdir(v), g.check()),
            );
            got_reservation = true;
            if changed_signal {
                g.mark_tile_dirty_by_tile(tile);
            }
        } else {
            free_train_track_reservation(g, v);
            if mark_stuck {
                g.mark_train_as_stuck(v);
            }
        }
        return (best_track, got_reservation);
    }

    got_reservation = true;

    /* Reservation target found and free, check if it is safe. */
    while !g.is_safe_waiting_position(v, res_dest.tile, res_dest.trackdir, true, g.forbid_90_deg())
    {
        /* Extend reservation until we have found a safe position. */
        let exitdir = trackdir_to_exitdir(res_dest.trackdir, g.check());
        let next_tile = g.tile_add_by_diag_dir(res_dest.tile, exitdir);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the original's (TrackdirBits) cast of the track status"
        )]
        let status = TrackdirBits(g.get_tile_track_status(next_tile) as u16);
        let mut reachable =
            trackdir_bits_to_track_bits(status) & diagdir_reaches_tracks(exitdir, g.check());
        if g.rail_90_deg_turn_disallowed(
            g.get_tile_rail_type(res_dest.tile),
            g.get_tile_rail_type(next_tile),
        ) {
            reachable &=
                !track_crosses_tracks(trackdir_to_track(res_dest.trackdir, g.check()), g.check());
        }

        /* Get next order with destination. */
        if orders.switch_to_next_order(true) {
            let mut cur_dest = PBSTileInfo::default();
            /* `DoTrainPathfind` with no final destination: the original leaves `path_found`
             * uninitialised and never reads it. */
            let mut path_found = true;
            crate::rail_yapf::yapf_train_choose_track(
                g.services,
                v,
                next_tile,
                exitdir,
                reachable,
                &mut path_found,
                true,
                Some(&mut cur_dest),
                None,
            );
            if cur_dest.tile != TileIndex::INVALID {
                res_dest = cur_dest;
                if res_dest.okay {
                    continue;
                }
                /* Path found, but could not be reserved. */
                free_train_track_reservation(g, v);
                if mark_stuck {
                    g.mark_train_as_stuck(v);
                }
                got_reservation = false;
                changed_signal = false;
                break;
            }
        }
        /* No order or no safe position found, try any position. */
        if !g.try_reserve_safe_track(v, res_dest.tile, res_dest.trackdir, true) {
            free_train_track_reservation(g, v);
            if mark_stuck {
                g.mark_train_as_stuck(v);
            }
            got_reservation = false;
            changed_signal = false;
        }
        break;
    }

    g.try_reserve_rail_track(
        v.vehicle().tile(),
        trackdir_to_track(g.get_vehicle_trackdir(v), g.check()),
    );

    if changed_signal {
        g.mark_tile_dirty_by_tile(tile);
    }

    orders.restore();
    if v.vehicle()
        .current_order()
        .is_type(OrderType::OT_GOTO_DEPOT)
        && v.vehicle()
            .current_order()
            .depot_action_flags()
            .test(OrderDepotActionFlag::NearestDepot)
        && final_dest != TileIndex::INVALID
        && g.is_rail_depot_tile(final_dest)
    {
        // SAFETY: The vehicle's live current order.
        unsafe {
            crate::orders::set_destination(
                v.vehicle().current_order_pointer(),
                DestinationID(g.get_depot_index(final_dest).0),
            );
        };
        v.vehicle().set_dest_tile(final_dest);
        g.set_start_stop_dirty(v);
    }

    (best_track, got_reservation)
}

/// `TryPathReserve`: try to reserve a path to a safe position. `mark_as_stuck` says whether
/// the train should be marked as stuck on a failed reservation; `first_tile_okay` that no
/// path should be reserved if the current tile is a safe position. Returns true if a path
/// could be reserved.
pub(crate) fn try_path_reserve(
    g: &Game,
    v: Handle,
    mark_as_stuck: bool,
    first_tile_okay: bool,
) -> bool {
    assert_with(|| v.is_front_engine());

    /* We have to handle depots specially as the track follower won't look
     * at the depot tile itself but starts from the next tile. If we are still
     * inside the depot, a depot reservation can never be ours. */
    if v.track() == TrackBits::TRACK_BIT_DEPOT {
        if g.has_depot_reservation(v.vehicle().tile()) {
            if mark_as_stuck {
                g.mark_train_as_stuck(v);
            }
            return false;
        }
        /* Depot not reserved, but the next tile might be. */
        let next_tile = g.tile_add_by_diag_dir(
            v.vehicle().tile(),
            g.get_rail_depot_direction(v.vehicle().tile()),
        );
        if g.has_reserved_tracks(
            next_tile,
            diagdir_reaches_tracks(g.get_rail_depot_direction(v.vehicle().tile()), g.check()),
        ) {
            return false;
        }
    }

    let mut other_train = Handle::NONE;
    let origin = g.follow_train_reservation(v, Some(&mut other_train));
    /* The path we are driving on is already blocked by some other train.
     * This can only happen in certain situations when mixing path and
     * block signals or when changing tracks and/or signals.
     * Exit here as doing any further reservations will probably just
     * make matters worse. */
    if other_train != Handle::NONE && g.index(other_train) != g.index(v) {
        if mark_as_stuck {
            g.mark_train_as_stuck(v);
        }
        return false;
    }
    /* If we have a reserved path and the path ends at a safe tile, we are finished already. */
    if origin.okay && (v.vehicle().tile() != origin.tile || first_tile_okay) {
        /* Can't be stuck then. */
        if v.flags().test(VehicleRailFlag::Stuck) {
            g.set_start_stop_dirty(v);
        }
        v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
        return true;
    }

    /* If we are in a depot, tentatively reserve the depot. */
    if v.track() == TrackBits::TRACK_BIT_DEPOT {
        g.set_depot_reservation(v.vehicle().tile(), true);
        if g.show_track_reservation() {
            g.mark_tile_dirty_by_tile(v.vehicle().tile());
        }
    }

    let exitdir = trackdir_to_exitdir(origin.trackdir, g.check());
    let new_tile = g.tile_add_by_diag_dir(origin.tile, exitdir);
    let mut reachable = trackdir_bits_to_track_bits(
        track_status_to_trackdir_bits(g.get_tile_track_status(new_tile))
            & diagdir_reaches_trackdirs(exitdir, g.check()),
    );

    if g.rail_90_deg_turn_disallowed(
        g.get_tile_rail_type(origin.tile),
        g.get_tile_rail_type(new_tile),
    ) {
        reachable &=
            !track_crosses_tracks(trackdir_to_track(origin.trackdir, g.check()), g.check());
    }

    let (_, res_made) = choose_train_track(g, v, new_tile, exitdir, reachable, true, mark_as_stuck);

    if !res_made {
        /* Free the depot reservation as well. */
        if v.track() == TrackBits::TRACK_BIT_DEPOT {
            g.set_depot_reservation(v.vehicle().tile(), false);
        }
        return false;
    }

    if v.flags().test(VehicleRailFlag::Stuck) {
        v.set_wait_counter(0);
        g.set_start_stop_dirty(v);
    }
    v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
    true
}

/// `CheckReverseTrain`.
pub(crate) fn check_reverse_train(g: &Game, v: Handle) -> bool {
    if g.line_reverse_mode()
        || v.track() == TrackBits::TRACK_BIT_DEPOT
        || v.track() == TrackBits::TRACK_BIT_WORMHOLE
        || v.vehicle().direction().0 & 1 == 0
    {
        return false;
    }

    assert_with(|| v.track() != TrackBits::TRACK_BIT_NONE);

    crate::rail_yapf::yapf_train_check_reverse(g.services, v)
}

/// `Train::GetOrderStationLocation`: get the location of the next station to visit.
/// `station` is the next station to visit. Returns the location of the new station.
fn get_order_station_location(g: &Game, this: Handle, station: StationID) -> TileIndex {
    if station == this.vehicle().last_station_visited() {
        this.vehicle().set_last_station_visited(StationID::INVALID);
    }

    let st = g.station_get(station);
    if !g.facilities(st.base).test(StationFacility::Train) {
        /* The destination station has no trainstation tiles. */
        g.increment_real_order_index(this);
        return TileIndex(0);
    }

    crate::base_station_base::xy(st.base)
}

/// `Train::ReserveTrackUnderConsist`: tries to reserve track under whole train consist.
pub(crate) fn reserve_track_under_consist(g: &Game, this: Handle) {
    let mut u = this;
    while u != Handle::NONE {
        match u.track() {
            TrackBits::TRACK_BIT_WORMHOLE => {
                g.try_reserve_rail_track(
                    u.vehicle().tile(),
                    crate::track_func::diag_dir_to_diag_track(
                        g.get_tunnel_bridge_direction(u.vehicle().tile()),
                        g.check(),
                    ),
                );
            }
            TrackBits::TRACK_BIT_DEPOT => {}
            _ => {
                g.try_reserve_rail_track(
                    u.vehicle().tile(),
                    track_bits_to_track(u.track(), g.check()),
                );
            }
        }
        u = u.next();
    }
}

/// The C++ callers of these entries hold the game-state mutex and pass the live train part
/// and the immutable process-lifetime train and shared tables, which survive the call.
/// Callbacks may reenter; no Rust borrow of a record spans one.
macro_rules! entry_game {
    ($leaves:expr, $services:expr) => {
        // SAFETY: The entry's contract keeps both immutable tables live until it returns.
        Game::new(unsafe { &*$leaves }, unsafe { &*$services })
    };
}

/// `CheckNextTrainTile`, for the kept native test.
/// # Safety
/// The tables and the train part are live until the call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_check_next(
    v: Handle,
    leaves: *const crate::train::Leaves,
    services: *const Services,
) {
    check_next_train_tile(&entry_game!(leaves, services), v);
}

/// `FreeTrainTrackReservation`.
/// # Safety
/// As [`openttd_rust_train_reservation_check_next`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_free(
    v: Handle,
    leaves: *const crate::train::Leaves,
    services: *const Services,
) {
    free_train_track_reservation(&entry_game!(leaves, services), v);
}

/// `TryPathReserve`.
/// # Safety
/// As [`openttd_rust_train_reservation_check_next`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_try_path(
    v: Handle,
    mark_as_stuck: u8,
    first_tile_okay: u8,
    leaves: *const crate::train::Leaves,
    services: *const Services,
) -> u8 {
    u8::from(try_path_reserve(
        &entry_game!(leaves, services),
        v,
        mark_as_stuck != 0,
        first_tile_okay != 0,
    ))
}

/// `Train::GetOrderStationLocation`.
/// # Safety
/// As [`openttd_rust_train_reservation_check_next`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_station_location(
    v: Handle,
    station: u16,
    leaves: *const crate::train::Leaves,
    services: *const Services,
) -> u32 {
    get_order_station_location(&entry_game!(leaves, services), v, StationID(station)).0
}

/// `Train::ReserveTrackUnderConsist`.
/// # Safety
/// As [`openttd_rust_train_reservation_check_next`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reservation_reserve_under(
    v: Handle,
    leaves: *const crate::train::Leaves,
    services: *const Services,
) {
    reserve_track_under_consist(&entry_game!(leaves, services), v);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout};
    use crate::pbs::PbsServices;
    use crate::services::RailMapServices;
    use crate::test_tables::{MOVEMENT, ORDERS, PBS, RAIL_MAP, TRAIN_LEAVES, VEHICLE};
    use crate::train_state::State;
    use std::cell::Cell;
    use std::ffi::c_void;

    extern "C" fn resolve_record(shell: *mut c_void) -> *mut crate::vehicle_base::VehicleRecord {
        // SAFETY: Test handles retain their exclusive stack Mock for each query.
        unsafe { &raw mut *(*shell.cast::<Mock>()).record }
    }
    /// A train record and the owners it links, with a log of the recording services.
    struct Mock {
        record: Box<crate::vehicle_base::VehicleRecord>,
        state: State,
        stop: u8,
        log: Vec<(&'static str, u32, u8)>,
        ground: crate::ground_vehicle::GroundState,
        consist: crate::orders::Consist,
        orders: crate::orders::VehicleOrders,
    }
    impl Mock {
        fn new(stop: u8) -> Self {
            Self {
                // SAFETY: Transfer the private record allocation to this test owner.
                record: unsafe {
                    Box::from_raw(crate::vehicle_base::openttd_rust_vehicle_record_new(
                        std::ptr::null_mut(),
                        crate::vehicle::VehicleType::TRAIN,
                    ))
                },
                state: State::default(),
                stop,
                log: Vec::new(),
                ground: crate::ground_vehicle::GroundState {
                    flags: 4,
                    ..Default::default()
                },
                consist: crate::orders::Consist {
                    time: 0,
                    lateness: 0,
                    start: 0,
                    last_departure: 0,
                    next_departure: 0,
                    round_trip: 0,
                    real: 254,
                    implicit: 0,
                    flags: 0,
                },
                orders: crate::orders::VehicleOrders {
                    current: std::mem::MaybeUninit::new(crate::orders::Order {
                        kind: 0x12,
                        flags: 0x34,
                        destination: 1,
                        refit: 0x56,
                        wait: 0,
                        travel: 0x789a,
                        speed: 0xbcde,
                    }),
                    orders: std::ptr::null_mut(),
                    next: std::ptr::null_mut(),
                    previous: std::ptr::null_mut(),
                },
            }
        }
        fn set_order_token(&mut self, token: u32) {
            // SAFETY: The fixture initializes this Order before every access.
            let order = unsafe { self.orders.current.assume_init_mut() };
            order.destination = u16::try_from(token & 0xffff).unwrap();
            order.wait = u16::try_from(token >> 16).unwrap();
        }
        fn order_token(&self) -> u32 {
            // SAFETY: The fixture initializes this Order before every access.
            let order = unsafe { self.orders.current.assume_init_ref() };
            u32::from(order.destination) | (u32::from(order.wait) << 16)
        }
        /// The handle of the train part, standing on the line's first tile and facing
        /// north-east on `TRACK_X`.
        fn handle(&mut self) -> Handle {
            self.record.dest_tile = TileIndex(9);
            self.record.last_station_visited = StationID(7);
            self.record.tile = index(0);
            self.record.direction = crate::direction_type::Direction::DIR_NE;
            self.record.ground = &raw mut self.ground;
            self.record.consist = &raw mut self.consist;
            self.record.orders = &raw mut self.orders;
            self.record.train = &raw mut self.state;
            self.record.owner = crate::company_type::CompanyID(0);
            self.record.shell = std::ptr::from_mut(self).cast();
            self.state.track = TrackBits::TRACK_BIT_X;
            Handle {
                record: &raw mut *self.record,
                owner: &raw mut self.state,
            }
        }
    }
    unsafe fn mock<'a>(h: Handle) -> &'a mut Mock {
        // SAFETY: Every test installs this handle to its live exclusive stack mock.
        unsafe { &mut *h.vehicle().shell().cast::<Mock>() }
    }

    // A PBS service has no vehicle argument; each test thread names its line here.
    thread_local! {
        static TRAIN: Cell<Handle> = const { Cell::new(Handle::NONE) };
        static PBS_TABLE: Cell<*const PbsServices> = const { Cell::new(std::ptr::null()) };
        static TILES: Cell<*const BaseTileLayout> = const { Cell::new(std::ptr::null()) };
    }
    fn current<'a>() -> &'a mut Mock {
        // SAFETY: The installing test keeps this mock live; callbacks end each access.
        unsafe { mock(TRAIN.get()) }
    }
    /// The tile `ordinal` steps north-east of the train's tile on a 64 by 64 map.
    const fn index(ordinal: u32) -> TileIndex {
        TileIndex(72 - ordinal)
    }
    const fn ordinal(tile: u32) -> u32 {
        72 - tile
    }
    extern "C" fn pbs_services(_: *const Services) -> *const PbsServices {
        PBS_TABLE.get()
    }
    extern "C" fn every_type(_: u64) -> u64 {
        1
    }
    extern "C" fn no_rail90(_: u8, _: u8, _: bool) -> bool {
        false
    }
    extern "C" fn no_signal(_: u32, _: u8) -> bool {
        false
    }
    /// `GetRailReservationTrackBits` over the line's map: `m2` bits 8 to 10 hold the
    /// reserved track plus one, and bit 11 the opposite track.
    extern "C" fn rail_reservation(tile: u32) -> TrackBits {
        // SAFETY: The installing test keeps its line's map live.
        let m2 = unsafe { (*TILES.get().add(tile as usize)).m2 };
        let track_b = (m2 >> 8) & 7;
        if track_b == 0 {
            return TrackBits::TRACK_BIT_NONE;
        }
        let track = track_b - 1;
        TrackBits(
            (1 << track)
                | if m2 & (1 << 11) != 0 {
                    1 << (track ^ 1)
                } else {
                    0
                },
        )
    }
    extern "C" fn no_show() -> bool {
        false
    }
    extern "C" fn no_random(_: *mut c_void) -> u32 {
        std::process::abort()
    }
    /// `HasPbsSignalOnTrackdir` over the line's map.
    extern "C" fn has_pbs(tile: u32, td: u8) -> bool {
        // SAFETY: The installing test keeps its line live.
        let w = World::new(unsafe { &*(*PBS_TABLE.get()).shared });
        w.is_tile_type(TileIndex(tile), TileType::MP_RAILWAY)
            && w.has_signal_on_trackdir(TileIndex(tile), Trackdir(td))
            && crate::rail_map::is_pbs_signal(w.get_signal_type(TileIndex(tile), Track(td & 7)))
    }
    extern "C" fn try_track(tile: u32, track: u8) -> bool {
        let m = current();
        m.log.push(("try", ordinal(tile), track));
        ordinal(tile) != 3 || m.stop == 1
    }
    extern "C" fn unreserve_track(tile: u32, track: u8) {
        current().log.push(("unreserve", ordinal(tile), track));
    }
    extern "C" fn set_signal(tile: u32, _: u8, state: u8) {
        current().log.push(("signal", ordinal(tile), state));
    }
    extern "C" fn mark(tile: u32) {
        current().log.push(("mark", ordinal(tile), 0));
    }

    /// A straight line of rail north-east of the train: the train's tile is plain rail on
    /// `TRACK_X`, and the four tiles ahead carry a two-way path signal on `TRACK_X` facing
    /// south-west, green. No track is reserved until `reserve` reserves one.
    struct Line {
        shared: Services,
        rail: RailMapServices,
        pbs: PbsServices,
        movement: crate::movement::MovementServices,
        vehicle: VehicleServices,
        train: crate::train::Leaves,
        base: Vec<BaseTileLayout>,
        extended: Vec<ExtendedTileLayout>,
    }
    impl Line {
        fn new() -> Box<Self> {
            let mut line = Box::new(Self {
                shared: Services {
                    pbs_services,
                    all_compatible_railtypes: every_type,
                    mark_dirty: mark,
                    ..crate::services::fixture(std::ptr::null_mut(), no_random)
                },
                rail: RailMapServices {
                    rail_reservation,
                    set_signal_state: set_signal,
                    ..RAIL_MAP
                },
                pbs: PbsServices {
                    try_track,
                    unreserve_track,
                    show_reservation: no_show,
                    rail90: no_rail90,
                    blocking: no_signal,
                    has_pbs,
                    ..PBS
                },
                movement: MOVEMENT,
                vehicle: VehicleServices {
                    record_from_shell: resolve_record,
                    ..VEHICLE
                },
                train: TRAIN_LEAVES,
                base: (0..4096)
                    .map(|_| BaseTileLayout {
                        kind: 0,
                        height: 0,
                        m2: 0,
                        m1: 0,
                        m3: 0,
                        m4: 0,
                        m5: 0,
                    })
                    .collect(),
                extended: (0..4096)
                    .map(|_| ExtendedTileLayout {
                        m6: 0,
                        m7: 0,
                        m8: 0,
                    })
                    .collect(),
            });
            for ordinal in 0..=4 {
                let tile = &mut line.base[index(ordinal).0 as usize];
                tile.kind = TileType::MP_RAILWAY.0 << 4;
                tile.m5 = TrackBits::TRACK_BIT_X.0;
                if ordinal > 0 {
                    // `RailTileType::Signals`; the signal along `TRACKDIR_X_SW` is present
                    // and green, and it is a two-way path signal.
                    tile.m5 |= crate::rail_map::RailTileType::Signals.0 << 6;
                    tile.m3 =
                        crate::signal_func::signal_along_trackdir(Trackdir::TRACKDIR_X_SW) << 4;
                    tile.m4 = tile.m3;
                    tile.m2 = u16::from(SignalType::SIGTYPE_PBS.0);
                }
            }
            line.movement.vehicle = &raw const line.vehicle;
            line.movement.shared = &raw const line.shared;
            line.shared.movement = &raw const line.movement;
            line.train.vehicle = &raw const line.vehicle;
            line.pbs.shared = &raw const line.shared;
            line.pbs.rail = &raw const line.rail;
            line.pbs.train = &raw const line.train;
            line
        }
        /// Route this thread's PBS queries to the line and to `train`'s mock; the line ends
        /// at its fourth tile, or its third when the train's mock stops at `2`.
        fn install(&mut self, train: Handle) {
            // SAFETY: The test's live mock.
            let stop = unsafe { mock(train) }.stop;
            let end = &mut self.base[index(if stop == 2 { 3 } else { 4 }).0 as usize];
            *end = BaseTileLayout {
                kind: TileType::MP_RAILWAY.0 << 4,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            };
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base: self.base.as_ptr(),
                    extended: self.extended.as_ptr(),
                    count: u32::try_from(self.base.len()).unwrap(),
                    assertions: false,
                },
                6,
            );
            TRAIN.set(train);
            PBS_TABLE.set(&raw const self.pbs);
            TILES.set(self.base.as_ptr());
        }
        /// Reserve `TRACK_X` on the tile `ordinal` steps ahead of the train.
        fn reserve(&mut self, ordinal: u32) {
            self.base[index(ordinal).0 as usize].m2 |= u16::from(Track::TRACK_X.0 + 1) << 8;
        }
        fn game(&self) -> Game<'_> {
            Game::new(&self.train, &self.shared)
        }
    }

    /// `ExtendTrainReservation` follows the train's reservation to its end, then reserves
    /// each tile from there, turning each opposing green path signal red, until a
    /// reservation fails; it then unreserves only the tracks it reserved, from the end of
    /// the train's reservation, and turns the signals green again in the order it turned
    /// them red. Without a reservation it starts on the train's own tile.
    #[test]
    fn controller_extension_rolls_back_only_new_tracks_and_restores_signal_order() {
        /* The train holds its own tile and the next one. */
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut line = Line::new();
        line.reserve(0);
        line.reserve(1);
        line.install(h);
        let mut tracks = TrackBits::TRACK_BIT_NONE;
        let mut dir = DiagDirection::INVALID_DIAGDIR;
        let result = extend_train_reservation(&line.game(), h, &mut tracks, &mut dir);
        assert_eq!(result.tile, TileIndex::INVALID);
        assert_eq!(
            m.log,
            vec![
                ("try", 2, 0),
                ("signal", 2, 0),
                ("mark", 2, 0),
                ("try", 3, 0),
                ("unreserve", 2, 0),
                ("signal", 2, 1)
            ]
        );

        /* No reservation: the extension starts on the train's tile. */
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut line = Line::new();
        line.install(h);
        let mut tracks = TrackBits::TRACK_BIT_NONE;
        let mut dir = DiagDirection::INVALID_DIAGDIR;
        let result = extend_train_reservation(&line.game(), h, &mut tracks, &mut dir);
        assert_eq!(result.tile, TileIndex::INVALID);
        assert_eq!(
            m.log,
            vec![
                ("try", 1, 0),
                ("signal", 1, 0),
                ("mark", 1, 0),
                ("try", 2, 0),
                ("signal", 2, 0),
                ("mark", 2, 0),
                ("try", 3, 0),
                ("unreserve", 1, 0),
                ("unreserve", 2, 0),
                ("signal", 1, 1),
                ("signal", 2, 1)
            ]
        );
    }

    /// A safe waiting position or the end of the line ends the extension with a valid path
    /// and rolls nothing back.
    #[test]
    fn controller_extension_accepts_safe_position_or_end_of_line_without_rollback() {
        for stop in [1, 2] {
            let mut m = Mock::new(stop);
            let h = m.handle();
            let mut line = Line::new();
            line.install(h);
            let mut tracks = TrackBits::TRACK_BIT_NONE;
            let mut dir = DiagDirection::INVALID_DIAGDIR;
            let result = extend_train_reservation(&line.game(), h, &mut tracks, &mut dir);
            assert_eq!(
                (result.tile, result.trackdir, result.okay),
                (
                    index(if stop == 1 { 3 } else { 2 }),
                    Trackdir::TRACKDIR_X_NE,
                    true
                )
            );
            assert!(
                !m.log
                    .iter()
                    .any(|&(op, _, value)| op == "unreserve" || op == "signal" && value == 1)
            );
        }
    }

    /// `VehicleOrderSaver` restores the current order, destination, last station and
    /// implicit-order flag once, whether `Restore` or its destructor runs it, and a nested
    /// saver restores what it saw.
    #[test]
    fn explicit_or_drop_order_restoration_runs_once_with_nested_snapshots() {
        for explicit in [false, true] {
            let mut m = Mock::new(0);
            m.set_order_token(0x89ab_cdef);
            let h = m.handle();
            let line = Line::new();
            let g = line.game();
            {
                let mut outer = VehicleOrderSaver::new(&g, h);
                m.set_order_token(0x7654_3210);
                m.record.dest_tile = TileIndex(42);
                m.record.last_station_visited = StationID(8);
                m.ground.flags &= !4;
                {
                    let mut inner = VehicleOrderSaver::new(&g, h);
                    m.set_order_token(3);
                    m.record.dest_tile = TileIndex(99);
                    inner.restore();
                }
                assert_eq!(
                    (
                        m.record.dest_tile.0,
                        m.record.last_station_visited.0,
                        u8::from(m.ground.flags & 4 != 0),
                        m.order_token()
                    ),
                    (42, 8, 0, 0x7654_3210)
                );
                assert_eq!(outer.index, VehicleOrderID(254));
                if explicit {
                    outer.restore();
                    m.set_order_token(0x1357_2468);
                }
            }
            assert_eq!(
                (
                    m.record.dest_tile.0,
                    m.record.last_station_visited.0,
                    u8::from(m.ground.flags & 4 != 0),
                    m.order_token()
                ),
                (9, 7, 1, if explicit { 0x1357_2468 } else { 0x89ab_cdef })
            );
        }
    }

    /// `TryReserveRailTrack`'s plain-rail reservation callback can reenter the train's own
    /// entries: a nested direct entry on the same train sees and makes ordinary writes, and
    /// `ReserveTrackUnderConsist` continues on the state the nested entry left.
    #[test]
    fn direct_station_reservation_can_reenter_the_same_owner() {
        extern "C" fn nested(_: u32, _: u8) -> bool {
            let line_reverse = false;
            let settings = crate::settings_type::SettingAddresses {
                line_reverse_mode: &raw const line_reverse,
                ..crate::settings_type::SettingAddresses::NULL
            };
            let shared = Services {
                settings: &raw const settings,
                ..crate::services::fixture(std::ptr::null_mut(), no_random)
            };
            let h = TRAIN.get();
            h.set_track(TrackBits::TRACK_BIT_DEPOT);
            assert!(!check_reverse_train(&Game::new(&TRAIN_LEAVES, &shared), h));
            h.set_flags(crate::train::VehicleRailFlags(1 << 8));
            h.set_flags(h.flags().reset(VehicleRailFlag::Stuck));
            h.set_wait_counter(0);
            h.set_flags(crate::train::VehicleRailFlags(7));
            true
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut line = Line::new();
        line.pbs.try_track = nested;
        // The train's tile, which its track names, is plain rail with `TRACK_BIT_UPPER`.
        line.base[index(0).0 as usize].m5 = TrackBits::TRACK_BIT_UPPER.0;
        line.install(h);
        h.set_track(TrackBits::TRACK_BIT_UPPER);
        reserve_track_under_consist(&line.game(), h);
        assert_eq!(h.flags().0, 7);
        assert_eq!(m.state.wait_counter, 0);
        assert_eq!(h.track(), TrackBits::TRACK_BIT_DEPOT);
    }

    /// The tables of an entry whose every PBS, rail map and pathfinder service aborts,
    /// with the settings `catalog`.
    fn unreachable_services(catalog: &crate::settings_type::SettingAddresses) -> Services {
        extern "C" fn unreachable_pbs(_: *const Services) -> *const PbsServices {
            std::process::abort()
        }
        Services {
            pbs_services: unreachable_pbs,
            settings: catalog,
            ..crate::services::fixture(std::ptr::null_mut(), no_random)
        }
    }

    /// `TryPathReserve` asserts `v->IsFrontEngine()` first: a wagon ends the child there,
    /// before the depot check reads the depot reservation.
    #[test]
    fn try_path_reserve_asserts_a_front_engine_before_the_depot_check() {
        if crate::services_assert::isolate_test_exit(
            "train_reservation::tests::try_path_reserve_asserts_a_front_engine_before_the_depot_check",
        ) {
            return;
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        h.set_track(TrackBits::TRACK_BIT_DEPOT);
        let shared = unreachable_services(&crate::settings_type::SettingAddresses::NULL);
        try_path_reserve(&Game::new(&TRAIN_LEAVES, &shared), h, false, false);
    }

    /// `ChooseTrainTrack` reads `reserve_paths`, then asserts that the tracks hold no bit
    /// outside `TRACK_BIT_MASK`: `TRACK_BIT_DEPOT` ends the child there, before the
    /// reserved track bits are read.
    #[test]
    fn choose_train_track_asserts_the_track_mask_after_reading_reserve_paths() {
        static RESERVE_PATHS: bool = false;
        if crate::services_assert::isolate_test_exit(
            "train_reservation::tests::choose_train_track_asserts_the_track_mask_after_reading_reserve_paths",
        ) {
            return;
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        let catalog = crate::settings_type::SettingAddresses {
            reserve_paths: &raw const RESERVE_PATHS,
            ..crate::settings_type::SettingAddresses::NULL
        };
        let shared = unreachable_services(&catalog);
        choose_train_track(
            &Game::new(&TRAIN_LEAVES, &shared),
            h,
            index(1),
            DiagDirection::DIAGDIR_NE,
            TrackBits::TRACK_BIT_X | TrackBits::TRACK_BIT_DEPOT,
            false,
            false,
        );
    }

    /// `CheckReverseTrain` asserts `v->track != TRACK_BIT_NONE` once the train may reverse:
    /// a train on no track and on a diagonal direction ends the child there, before the
    /// pathfinder runs.
    #[test]
    fn check_reverse_train_asserts_a_track_before_the_pathfinder() {
        static LINE_REVERSE_MODE: bool = false;
        if crate::services_assert::isolate_test_exit(
            "train_reservation::tests::check_reverse_train_asserts_a_track_before_the_pathfinder",
        ) {
            return;
        }
        let mut m = Mock::new(0);
        let h = m.handle();
        h.set_track(TrackBits::TRACK_BIT_NONE);
        let catalog = crate::settings_type::SettingAddresses {
            line_reverse_mode: &raw const LINE_REVERSE_MODE,
            ..crate::settings_type::SettingAddresses::NULL
        };
        let shared = unreachable_services(&catalog);
        check_reverse_train(&Game::new(&TRAIN_LEAVES, &shared), h);
    }

    unsafe extern "C" fn vehicle_orders(shell: *mut c_void) -> *mut crate::orders::VehicleOrders {
        // SAFETY: Test handles retain their exclusive stack Mock for each query.
        unsafe { &raw mut (*shell.cast::<Mock>()).orders }
    }
    unsafe extern "C" fn consist(shell: *mut c_void) -> *mut crate::orders::Consist {
        // SAFETY: As above.
        unsafe { &raw mut (*shell.cast::<Mock>()).consist }
    }
    unsafe extern "C" fn invalidate_order(shell: *mut c_void, data: i32) {
        // SAFETY: As above.
        unsafe {
            (*shell.cast::<Mock>())
                .log
                .push(("invalidate", 0, u8::try_from(data).unwrap()));
        }
    }
    /// The orders table of a train without an order list: the vehicle's orders owner and
    /// consist, and a recording order invalidation; every list access aborts.
    extern "C" fn orders_table() -> *const crate::orders::Leaves {
        thread_local! {
            static TABLE: &'static crate::orders::Leaves = Box::leak(Box::new(crate::orders::Leaves {
                vehicle: vehicle_orders,
                consist,
                invalidate_order,
                ..ORDERS
            }));
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }

    /// `SwitchToNextOrder` of a train without an order list returns false at once, as
    /// `v->GetNumOrders()` is zero there, and reads no order and no index.
    #[test]
    fn switch_to_next_order_without_an_order_list_finds_none() {
        let mut m = Mock::new(0);
        let h = m.handle();
        let mut line = Line::new();
        line.vehicle.orders = orders_table;
        let g = line.game();
        let mut orders = VehicleOrderSaver::new(&g, h);
        assert!(!orders.switch_to_next_order(true));
        assert_eq!(orders.index, VehicleOrderID(254));
        drop(orders);
        assert_eq!(m.order_token(), 1);
    }

    thread_local! {
        static STATION_LOOKUPS: Cell<u32> = const { Cell::new(0) };
        static FACILITIES: Cell<u8> = const { Cell::new(0) };
    }
    /// The base station the recording table resolves.
    const BASE: *mut crate::station::BaseStation = std::ptr::dangling_mut();
    const STATION: StationID = StationID(5);
    extern "C" fn station(id: u16) -> crate::station_service::LoadingStation {
        assert_eq!(id, STATION.0, "Station::Get(station)");
        STATION_LOOKUPS.set(STATION_LOOKUPS.get() + 1);
        crate::station_service::LoadingStation {
            native: std::ptr::dangling_mut(),
            base: BASE,
            state: std::ptr::null_mut(),
        }
    }
    extern "C" fn facilities(st: *mut crate::station::BaseStation) -> u8 {
        assert_eq!(st, BASE);
        FACILITIES.get()
    }
    extern "C" fn station_catchment() -> *const crate::station::CatchmentServices {
        thread_local! {
            static TABLE: &'static crate::station::CatchmentServices = Box::leak(Box::new(
                crate::station::CatchmentServices {
                    station: Box::leak(Box::new(crate::station_service::World {
                        station,
                        facilities,
                        ..crate::test_tables::STATION_WORLD
                    })),
                    ..crate::test_tables::CATCHMENT
                },
            ));
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }

    /// `Train::GetOrderStationLocation` forgets a last visit to the station and resolves
    /// it once: a station with train facilities gives its `xy`; one without advances the
    /// real order index, which without manual orders returns it to the first order and
    /// invalidates the order window, and gives tile 0.
    #[test]
    fn get_order_station_location_resolves_the_station_once() {
        crate::base_station_base::testing::set_xy(|st| {
            assert_eq!(st, BASE.cast_const());
            TileIndex(1234)
        });
        for (train_facility, location, real, log) in [
            (true, TileIndex(1234), 254, vec![]),
            (false, TileIndex(0), 0, vec![("invalidate", 0, 0)]),
        ] {
            let mut m = Mock::new(0);
            let h = m.handle();
            m.record.last_station_visited = STATION;
            m.consist.implicit = 3;
            let mut line = Line::new();
            line.shared.station_catchment = station_catchment;
            line.vehicle.orders = orders_table;
            STATION_LOOKUPS.set(0);
            FACILITIES.set(if train_facility {
                1 << StationFacility::Train.0
            } else {
                0
            });
            assert_eq!(
                get_order_station_location(&line.game(), h, STATION),
                location
            );
            assert_eq!(STATION_LOOKUPS.get(), 1);
            assert_eq!(m.record.last_station_visited, StationID::INVALID);
            assert_eq!((m.consist.real, m.log.clone()), (real, log));
        }
    }
}

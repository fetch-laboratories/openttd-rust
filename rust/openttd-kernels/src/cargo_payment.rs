/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The cargo payment of `economy.cpp`: `GetTransportedGoodsIncome`, the delivery of goods to
//! industries and towns, `CargoPayment` and the production triggered by the industries that
//! received cargo.
//!
//! C++ keeps the payment pool and its shell, whose save aliases address [`CargoPayment::fields`]
//! in place between Rust calls, and the world records the services read. A payment or packet
//! pointer names a live record. A payment's front is a live vehicle from the constructor to the
//! destructor, and unresolved between loading and `AfterLoad`. The game-state mutex serialises
//! every access, and no reference to a payment, the delivery destinations or a station's list
//! is alive across a callback, a leaf or a delivery.
use crate::bitmath_func::{gb, has_bit};
use crate::cargo_type::CargoType;
use crate::company_type::{CompanyID, Owner};
use crate::industry_type::{Industry, IndustryID};
use crate::math::{clamp_to_u8, clamp_to_u16};
use crate::newgrf_callbacks::{CALLBACK_FAILED, CBID_CARGO_PROFIT_CALC, CargoCallbackMask};
use crate::source_type::{Source, SourceType};
use crate::station_service::LoadingStation;
use crate::station_type::{Station, StationID};
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleRef;
use crate::vehicle_type::OpenTTDVehicle;

/// Opaque live native `CargoPayment`; never dereferenced by Rust.
#[repr(C)]
pub struct NativePayment {
    _private: [u8; 0],
}

/// The members of `CargoPayment` the save code aliases.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fields {
    /// The front vehicle to do the payment of.
    pub(crate) front: *mut OpenTTDVehicle,
    /// The amount of money to add/remove from the bank account.
    pub(crate) route_profit: i64,
    /// The visual profit to show.
    pub(crate) visual_profit: i64,
    /// The transfer credits to be shown.
    pub(crate) visual_transfer: i64,
}

/// The C++ services and tables the payment reaches.
#[repr(C)]
pub struct Services {
    /// `this->front->cargo_payment = nullptr`: the store into the vehicle's C++ shell, whose
    /// pool #634 owns.
    pub(crate) detach_payment: extern "C" fn(*mut OpenTTDVehicle),
    /// `ShowFeederIncomeAnimation`.
    pub(crate) feeder_animation: extern "C" fn(i32, i32, i32, i64, i64),
    pub(crate) finance: *const crate::company::FinanceServices,
    pub(crate) industry_delivery: *const crate::industry::DeliveryServices,
    pub(crate) registry: *const crate::cargo_registry::Services,
    pub(crate) resolver: *const crate::industry::resolver::ResolverServices,
    pub(crate) shared: *const crate::services::Services,
    /// The station table, for `Station::Get`.
    pub(crate) stations: *const crate::station_service::World,
    pub(crate) subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    /// `BaseStation::town`.
    pub(crate) town: extern "C" fn(*const Station) -> *const crate::town_type::TownHandle,
    pub(crate) vehicles: *const crate::vehicle::VehicleServices,
}

/// `CargoPayment`: helper class to perform the cargo payment. Rust allocates it with the
/// `OpenTTDCargoPayment` C layout (ABI layout 60).
#[repr(C)]
pub struct CargoPayment {
    pub(crate) fields: Fields,
    /// NOSAVE: The current station.
    pub(crate) current_station: StationID,
}

/// `SmallIndustryList`.
type SmallIndustryList = Vec<*mut Industry>;

/// `_cargo_delivery_destinations`: the industries we've currently brought cargo to.
static CARGO_DELIVERY_DESTINATIONS: crate::game_state::GameState<SmallIndustryList> =
    crate::game_state::GameState::new(Vec::new());

/// `_cargo_delivery_destinations`, for one operation that ends before any callback.
#[allow(
    unsafe_code,
    reason = "borrows the process-wide list under the module's contract"
)]
fn cargo_delivery_destinations() -> &'static mut SmallIndustryList {
    // SAFETY: The game-state mutex serialises every access, and each caller ends the borrow
    // before a callback.
    unsafe { &mut *CARGO_DELIVERY_DESTINATIONS.get() }
}

/// Whether `_cargo_delivery_destinations` is empty.
#[cfg(test)]
pub(crate) fn cargo_delivery_destinations_is_empty() -> bool {
    cargo_delivery_destinations().is_empty()
}

/// `BigMulS`: multiply two integer values and shift the results to right. The result of `a`
/// times `b` is shifted by `shift` to the right.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`(int32_t)((int64_t)a * (int64_t)b >> shift)`"
)]
fn big_mul_s(a: i32, b: i32, shift: u8) -> i32 {
    ((i64::from(a) * i64::from(b)) >> shift) as i32
}

/// `GetTransportedGoodsIncome`: the income for `num_pieces` of `cargo_type` carried `dist` in
/// `transit_periods`.
#[allow(unsafe_code, reason = "reads the process-lifetime registry table")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the original's `int` and `uint` conversions of `GB(callback, 0, 14)`, `result * num_pieces`, `dist * time_factor * num_pieces` and `BigMulS`' `int32_t` payment"
)]
pub(crate) fn get_transported_goods_income(
    s: &Services,
    num_pieces: u32,
    dist: u32,
    transit_periods: u16,
    cargo_type: CargoType,
) -> i64 {
    // SAFETY: The table names the immutable process-lifetime registry table.
    let cs = crate::cargotype::cargo_spec_get(unsafe { &*s.registry }, cargo_type);
    if !cs.is_valid() {
        /* User changed newgrfs and some vehicle still carries some cargo which is no longer available. */
        return 0;
    }

    /* Use callback to calculate cargo profit, if available */
    if cs.callback_mask().test(CargoCallbackMask::ProfitCalc) {
        let var18 = u32::from(clamp_to_u16(i64::from(dist)))
            | (u32::from(clamp_to_u8(i64::from(num_pieces))) << 16)
            | (u32::from(clamp_to_u8(i64::from(transit_periods))) << 24);
        let callback = crate::cargo_registry::get_cargo_callback(
            CBID_CARGO_PROFIT_CALC,
            0,
            var18,
            cs,
            &mut [],
        );
        if callback != CALLBACK_FAILED {
            let mut result = gb(u32::from(callback), 0, 14) as i32;

            /* Simulate a 15 bit signed value */
            if has_bit(callback, 14) {
                result -= 0x4000;
            }

            /* "The result should be a signed multiplier that gets multiplied
             * by the amount of cargo moved and the price factor, then gets
             * divided by 8192." */
            // `result * num_pieces` promotes to `uint` and wraps; the `uint * Money` product
            // passes through `int64` and then `operator*=(int)`, which narrows it.
            let factor = (result as u32).wrapping_mul(num_pieces) as i32;
            return cs.current_payment().saturating_mul(i64::from(factor)) / 8192;
        }
    }

    const MIN_TIME_FACTOR: i32 = 31;
    const MAX_TIME_FACTOR: i32 = 255;
    const TIME_FACTOR_FRAC_BITS: u8 = 4;
    const TIME_FACTOR_FRAC: i32 = 1 << TIME_FACTOR_FRAC_BITS;

    let [periods1, periods2] = cs.transit_periods().map(i32::from);
    let transit_periods = i32::from(transit_periods);
    let periods_over_periods1 = (transit_periods - periods1).max(0);
    let periods_over_periods2 = (periods_over_periods1 - periods2).max(0);
    let mut periods_over_max = MIN_TIME_FACTOR - MAX_TIME_FACTOR;
    if periods2 > -periods_over_max {
        periods_over_max += transit_periods - periods1;
    } else {
        periods_over_max += 2 * (transit_periods - periods1) - periods2;
    }

    /*
     * The time factor is calculated based on the time it took
     * (transit_periods) compared two cargo-depending values. The
     * range is divided into four parts:
     *
     *  - constant for fast transits
     *  - linear decreasing with time with a slope of -1 for medium transports
     *  - linear decreasing with time with a slope of -2 for slow transports
     *  - after hitting MIN_TIME_FACTOR, the time factor will be asymptotically decreased to a limit of 1 with a scaled 1/(x+1) function.
     *
     */
    if periods_over_max > 0 {
        // MIN_TIME_FACTOR / (x/(2 * TIME_FACTOR_FRAC) + 1) + 1, expressed as fixed point with TIME_FACTOR_FRAC_BITS.
        let time_factor = (2 * MIN_TIME_FACTOR * TIME_FACTOR_FRAC * TIME_FACTOR_FRAC
            / (periods_over_max + 2 * TIME_FACTOR_FRAC))
            .max(1);
        i64::from(big_mul_s(
            dist.wrapping_mul(time_factor as u32)
                .wrapping_mul(num_pieces) as i32,
            cs.current_payment() as i32,
            21 + TIME_FACTOR_FRAC_BITS,
        ))
    } else {
        let time_factor =
            (MAX_TIME_FACTOR - periods_over_periods1 - periods_over_periods2).max(MIN_TIME_FACTOR);
        i64::from(big_mul_s(
            dist.wrapping_mul(time_factor as u32)
                .wrapping_mul(num_pieces) as i32,
            cs.current_payment() as i32,
            21,
        ))
    }
}

/// `DeliverGoodsToIndustry`: transfer goods from station to industry. All cargo is delivered
/// to the nearest (Manhattan) industry to the station sign, which is inside the acceptance
/// rectangle and actually accepts the cargo. `st` is the station that accepted `num_pieces` of
/// `cargo_type` from `source`, delivered by `company`. Returns the actually accepted pieces of
/// cargo.
#[allow(
    unsafe_code,
    reason = "walks the live station's nearby industries and updates their acceptance"
)]
fn deliver_goods_to_industry(
    ctx: &FinalDelivery<'_>,
    st: LoadingStation,
    cargo_type: CargoType,
    num_pieces: u32,
    source: IndustryID,
    company: CompanyID,
) -> u32 {
    /* Find the nearest industrytile to the station sign inside the catchment area, whose industry accepts the cargo.
     * This fails in three cases:
     *  1) The station accepts the cargo because there are enough houses around it accepting the cargo.
     *  2) The industries in the catchment area temporarily reject the cargo, and the daily station loop has not yet updated station acceptance.
     *  3) The results of callbacks CBID_INDUSTRY_REFUSE_CARGO and CBID_INDTILE_CARGO_ACCEPTANCE are inconsistent. (documented behaviour)
     */

    let s = ctx.services;
    // SAFETY: The table names the immutable process-lifetime station and resolver tables.
    let (stations, resolver) = unsafe { (&*s.stations, &*s.resolver) };
    let catchment = crate::station::Ctx::from_shared(ctx.shared);
    let mut num_pieces = num_pieces;
    let mut accepted = 0_u32;

    // SAFETY: The live station owner; each entry is copied before its visit, whose callbacks
    // cannot change the list.
    unsafe {
        crate::station::for_each_industry_near(st.state, |i| {
            if num_pieces == 0 {
                return false;
            }

            let ind = i.industry;
            if i.id == source {
                return true;
            }

            let owner = (catchment.c.industry_owner)(ind);
            let it = crate::industry::get_cargo_accepted(owner, cargo_type);
            /* Check if matching cargo has been found */
            let Some(it) = it else {
                return true;
            };

            /* Check if industry temporarily refuses acceptance */
            if crate::industry::resolver::industry_temporarily_refuses_cargo(
                ind.cast(),
                owner,
                cargo_type,
                resolver,
            ) {
                return true;
            }

            let exclusive_supplier = crate::industry::exclusive_supplier(owner);
            if exclusive_supplier != Owner::INVALID
                && exclusive_supplier != CompanyID((stations.station_owner)(st.base))
            {
                return true;
            }

            /* Insert the industry into _cargo_delivery_destinations, if not yet contained */
            crate::container_func::include(cargo_delivery_destinations(), &ind);

            let amount = crate::industry::accept_delivery(owner, it, num_pieces, || {
                crate::timer_game_common::economy_date()
            });
            num_pieces -= amount;
            accepted += amount;

            /* Update the cargo monitor. */
            crate::cargomonitor::add_cargo_delivery(
                s,
                cargo_type,
                company,
                amount,
                Source::industry(source.0),
                st,
                i.id,
            );
            true
        });
    }

    accepted
}

/// `DeliverGoods`' parameters.
struct DeliverGoods {
    /// Amount of cargo delivered.
    num_pieces: i32,
    /// The type of cargo that is delivered.
    cargo_type: CargoType,
    /// Station the cargo has been unloaded.
    dest: StationID,
    /// The distance the cargo has traveled.
    distance: u32,
    /// Travel time in cargo aging periods.
    periods_in_transit: u16,
    /// The company delivering the cargo.
    company: CompanyID,
    /// Source of cargo.
    src: Source,
}

/// `DeliverGoods`: delivers goods to industries/towns and calculates the payment. Returns the
/// revenue for delivering cargo. The cargo is just added to the stockpile of the industry. It
/// is due to the caller to trigger the industry's production machinery.
#[allow(
    unsafe_code,
    reason = "updates the live station, company and town records of the delivery"
)]
#[allow(
    clippy::cast_sign_loss,
    reason = "`DeliverGoods`' `int num_pieces` passed and compared as `uint`"
)]
fn deliver_goods(ctx: &FinalDelivery<'_>, args: DeliverGoods) -> i64 {
    let DeliverGoods {
        num_pieces,
        cargo_type,
        dest,
        distance,
        periods_in_transit,
        company,
        src,
    } = args;
    crate::services_assert::assert_with(|| num_pieces > 0);

    let s = ctx.services;
    // SAFETY: The table names the immutable process-lifetime tables.
    let (stations, finance, registry) = unsafe { (&*s.stations, &*s.finance, &*s.registry) };
    let st = (stations.station)(dest.0);
    let finances = (finance.company_finances)(company.0);

    /* Give the goods to the industry. */
    let accepted_ind = deliver_goods_to_industry(
        ctx,
        st,
        cargo_type,
        num_pieces as u32,
        if SourceType(src.kind) == SourceType::Industry {
            src.to_industry_id()
        } else {
            IndustryID::INVALID
        },
        company,
    );

    /* If this cargo type is always accepted, accept all */
    // SAFETY: The station's live owner.
    let accepted_total = if unsafe { crate::station_service::always_accepts(st.state, cargo_type) }
    {
        num_pieces as u32
    } else {
        accepted_ind
    };

    /* Update station statistics */
    if accepted_total > 0 {
        // SAFETY: As above.
        unsafe { crate::station_service::record_delivery(st.state, cargo_type) };
    }

    /* Update company statistics */
    // SAFETY: `Company::Get(this->front->owner)`'s live record.
    unsafe { crate::company::record_delivery(finances, cargo_type, accepted_total) };

    /* Increase town's counter for town effects */
    let cs = crate::cargotype::cargo_spec_get(registry, cargo_type);
    let town = (ctx.shared.town_owner)((s.town)(st.native));
    // SAFETY: The station's live town.
    unsafe { crate::town::add_received(town, cs.town_acceptance_effect(), accepted_total) };

    /* Determine profit */
    let mut profit =
        get_transported_goods_income(s, accepted_total, distance, periods_in_transit, cargo_type);

    /* Update the cargo monitor. */
    crate::cargomonitor::add_cargo_delivery(
        s,
        cargo_type,
        company,
        accepted_total - accepted_ind,
        src,
        st,
        IndustryID::INVALID,
    );

    /* Modify profit if a subsidy is in effect */
    // SAFETY: The immutable process-lifetime subsidy table.
    let subsidies = unsafe { &*(s.subsidies)() };
    if subsidies.check_subsidised(cargo_type, company, src, || st.state) {
        profit = match (subsidies.multiplier)() {
            0 => profit.saturating_add(profit >> 1),
            1 => profit.saturating_mul(2),
            2 => profit.saturating_mul(3),
            _ => profit.saturating_mul(4),
        };
    }

    profit
}

/// The production machinery at the end of `LoadUnloadStation`: call the production machinery
/// of the industries that received cargo, then forget them.
#[allow(
    unsafe_code,
    reason = "runs the production of the live industries in the destination list"
)]
pub(crate) fn trigger_industry_production_of_deliveries(s: &Services) {
    // SAFETY: The table names the immutable process-lifetime tables.
    let (resolver, industry_delivery, shared) =
        unsafe { (&*s.resolver, &*s.industry_delivery, &*s.shared) };
    let catchment = crate::station::Ctx::from_shared(shared);
    // SAFETY: As above.
    let subsidies = unsafe { &*(s.subsidies)() };

    /* Call the production machinery of industries */
    // The list is read again at each step: the production callbacks run between them.
    let mut index = 0;
    while let Some(&iid) = cargo_delivery_destinations().get(index) {
        // SAFETY: A live industry of the list; no borrow of the list is alive across the call.
        unsafe {
            crate::industry::trigger_industry_production(
                iid,
                (catchment.c.industry_owner)(iid),
                (catchment.c.industry_id)(iid),
                resolver,
                &*subsidies.industry,
                industry_delivery,
            );
        }
        index += 1;
    }
    cargo_delivery_destinations().clear();
}

impl CargoPayment {
    /// `CargoPayment::CargoPayment(Vehicle *front)`: makes us a new cargo payment helper for
    /// the front of the train.
    fn new(front: VehicleRef) -> Self {
        Self {
            fields: Fields {
                front: front.0.as_ptr(),
                ..Fields::default()
            },
            current_station: front.last_station_visited(),
        }
    }
}

/// `CargoPayment::~CargoPayment`, after its `CleaningPool()` return.
#[allow(unsafe_code, reason = "settles the payment of its live front vehicle")]
unsafe fn settle(p: *mut CargoPayment, s: &Services) {
    // SAFETY: The caller's live payment; each read ends before the next call.
    (s.detach_payment)(unsafe { (*p).fields.front });

    if unsafe { (*p).fields.visual_profit == 0 && (*p).fields.visual_transfer == 0 } {
        return;
    }

    // SAFETY: The table names the immutable process-lifetime tables.
    let (finance, vehicles, shared) = unsafe { (&*s.finance, &*s.vehicles, &*s.shared) };
    // SAFETY: The front is the payment's live vehicle until the destructor returns.
    let front = VehicleRef(unsafe { std::ptr::NonNull::new_unchecked((*p).fields.front) });

    let cur_company = (finance.current_company)();
    (finance.set_current_company)(front.owner().0);

    crate::company::subtract_money_from_company(
        shared,
        finance,
        unsafe { (*p).fields.route_profit }.saturating_neg(),
        front.get_expense_type(true),
    );
    let (visual_profit, visual_transfer) =
        unsafe { ((*p).fields.visual_profit, (*p).fields.visual_transfer) };
    front.set_profit_this_year(
        vehicles,
        front.profit_this_year(vehicles).saturating_add(
            visual_profit
                .saturating_add(visual_transfer)
                .wrapping_shl(8),
        ),
    );

    if unsafe { (*p).fields.route_profit } != 0
        && crate::command::is_local_company_of(shared.local_company, shared.current_company)
        && !(vehicles.play_vehicle_sound)(front, crate::newgrf_sound::VSE_LOAD_UNLOAD)
    {
        (vehicles.play_sound)(front, crate::sound_type::SND_14_CASHTILL);
    }

    if visual_transfer != 0 {
        (s.feeder_animation)(
            front.x_pos(),
            front.y_pos(),
            front.z_pos(),
            visual_transfer,
            visual_profit.saturating_neg(),
        );
    } else {
        (shared.cost_animation)(
            front.x_pos(),
            front.y_pos(),
            front.z_pos(),
            visual_profit.saturating_neg(),
        );
    }

    (finance.set_current_company)(cur_company);
}

/// `CargoPayment::PayTransfer`: handle payment for transfer of `count` items of the given
/// cargo packet `cp` of `cargo` on `current_tile`; actual payment won't be made. Returns the
/// amount of money paid for the transfer.
///
/// # Safety
/// `payment` is the live payment and `cp` a live packet record.
#[allow(
    unsafe_code,
    reason = "reads the live packet and accumulates into the live payment"
)]
pub(crate) unsafe fn pay_transfer(
    payment: *mut CargoPayment,
    s: &Services,
    shared: &crate::services::Services,
    cargo: CargoType,
    cp: *const crate::cargo_storage::CargoPacket,
    count: u32,
    current_tile: TileIndex,
) -> i64 {
    /* Pay transfer vehicle the difference between the payment for the journey from
     * the source to the current point, and the sum of the previous transfer payments */
    // SAFETY: The caller's live record; the reads end before the income callback.
    let (feeder_share, distance, periods_in_transit) = unsafe {
        (
            (*cp).get_feeder_share_part(count),
            (*cp).get_distance(shared, current_tile),
            (*cp).get_periods_in_transit(),
        )
    };
    let profit = feeder_share
        .saturating_neg()
        .saturating_add(get_transported_goods_income(
            s,
            count,
            distance,
            periods_in_transit,
            cargo,
        ));

    let profit = profit.saturating_mul(i64::from(shared.settings().feeder_payment_share())) / 100;

    // SAFETY: The caller's live payment.
    unsafe {
        // Accumulate transfer profits for whole vehicle.
        (*payment).fields.visual_transfer =
            (*payment).fields.visual_transfer.saturating_add(profit);
    }
    profit // account for the (virtual) profit already made for the cargo packet
}

/// The payment and tables `CargoPayment::PayFinalDelivery` reaches.
pub(crate) struct FinalDelivery<'a> {
    /// `CargoPayment::rust_state` of the paying vehicle.
    pub(crate) payment: *mut CargoPayment,
    pub(crate) services: &'a Services,
    pub(crate) shared: &'a crate::services::Services,
}
impl<'a> FinalDelivery<'a> {
    /// The context of the native payment `payment`, from the storage table's slots.
    pub(crate) fn of(s: &'a crate::cargo_storage::Services, payment: *mut NativePayment) -> Self {
        Self {
            payment: (s.payment_state)(payment),
            services: s.payment(),
            shared: s.shared(),
        }
    }
}

/// `CargoPayment::PayFinalDelivery`: handle payment for final delivery of `count` items of the
/// given cargo packet `cp` of `cargo` on `current_tile`.
///
/// # Safety
/// `ctx.payment` is the live payment and `cp` a live packet record.
#[allow(
    unsafe_code,
    reason = "reads the live payment and packet and accumulates into the payment"
)]
#[allow(
    clippy::cast_possible_wrap,
    reason = "`DeliverGoods(count, ...)` converts `uint count` to its `int num_pieces`"
)]
pub(crate) unsafe fn pay_final_delivery(
    ctx: &FinalDelivery<'_>,
    cargo: CargoType,
    cp: *const crate::cargo_storage::CargoPacket,
    count: u32,
    current_tile: TileIndex,
) {
    // SAFETY: The caller's live payment and record; the reads end before the delivery.
    let (current_station, front, distance, periods_in_transit, source) = unsafe {
        (
            (*ctx.payment).current_station,
            (*ctx.payment).fields.front,
            (*cp).get_distance(ctx.shared, current_tile),
            (*cp).get_periods_in_transit(),
            (*cp).get_source(),
        )
    };
    // SAFETY: The payment's live front.
    let company = VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(front) }).owner();

    /* Handle end of route payment */
    let profit = deliver_goods(
        ctx,
        DeliverGoods {
            num_pieces: count as i32,
            cargo_type: cargo,
            dest: current_station,
            distance,
            periods_in_transit,
            company,
            src: source,
        },
    );
    // SAFETY: As above.
    unsafe {
        (*ctx.payment).fields.route_profit =
            (*ctx.payment).fields.route_profit.saturating_add(profit);

        /* The vehicle's profit is whatever route profit there is minus feeder shares. */
        let visual = profit.saturating_sub((*cp).get_feeder_share_part(count));
        (*ctx.payment).fields.visual_profit =
            (*ctx.payment).fields.visual_profit.saturating_add(visual);
    }
}

/// `CargoPayment::CargoPayment`: the payment of the live `front`, or for null the default
/// constructor the loader calls.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_cargo_payment_new(front: *mut OpenTTDVehicle) -> *mut CargoPayment {
    let payment = match std::ptr::NonNull::new(front) {
        Some(front) => CargoPayment::new(VehicleRef(front)),
        None => CargoPayment {
            fields: Fields::default(),
            current_station: StationID::INVALID,
        },
    };
    Box::into_raw(Box::new(payment))
}
/// `CargoPayment::front`.
///
/// # Safety
/// `p` is a live payment.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's payment")]
pub unsafe extern "C" fn openttd_rust_cargo_payment_front(
    p: *const CargoPayment,
) -> *mut OpenTTDVehicle {
    // SAFETY: The caller's live payment.
    unsafe { (*p).fields.front }
}
/// `CargoPayment::AfterLoad`'s `this->current_station = this->front->last_station_visited`.
///
/// # Safety
/// `p` is a live payment whose front the loader has resolved.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's payment")]
pub unsafe extern "C" fn openttd_rust_cargo_payment_afterload(p: *mut CargoPayment) {
    // SAFETY: The caller's live payment and its resolved front.
    unsafe {
        let front = VehicleRef(std::ptr::NonNull::new_unchecked((*p).fields.front));
        (*p).current_station = front.last_station_visited();
    }
}
/// `CargoPayment::~CargoPayment`: `cleaning` is `CleaningPool()`, which settles nothing; `s` may
/// be null only then.
///
/// # Safety
/// `p` is a live payment the constructor allocated, released here.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's payment")]
pub unsafe extern "C" fn openttd_rust_cargo_payment_destroy(
    p: *mut CargoPayment,
    s: *const Services,
    cleaning: u8,
) {
    if cleaning == 0 {
        // SAFETY: The caller's live payment and process-lifetime table.
        unsafe { settle(p, &*s) };
    }
    // SAFETY: The constructor's allocation.
    drop(unsafe { Box::from_raw(p) });
}
/// `CargoPayment::PayFinalDelivery`, for the consoles: `payment` pays for `count` items of the
/// packet shell `cp` on `tile`.
///
/// # Safety
/// `s` is the live storage table, `payment` a live payment and `cp` a live packet shell.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's payment and packet"
)]
pub unsafe extern "C" fn openttd_rust_cargo_payment_final(
    s: *const crate::cargo_storage::Services,
    payment: *mut NativePayment,
    cargo: u8,
    cp: *mut crate::cargo_storage::NativeCargoPacket,
    count: u32,
    tile: u32,
) {
    // SAFETY: The caller's live table, payment and packet.
    let s = unsafe { &*s };
    unsafe {
        pay_final_delivery(
            &FinalDelivery::of(s, payment),
            CargoType(cargo),
            (s.packet)(cp),
            count,
            TileIndex(tile),
        );
    }
}
/// `GetTransportedGoodsIncome`, for its C++ callers.
///
/// # Safety
/// `s` is the live payment table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's table")]
pub unsafe extern "C" fn openttd_rust_cargo_income(
    s: *const Services,
    num_pieces: u32,
    dist: u32,
    transit_periods: u16,
    cargo_type: u8,
) -> i64 {
    // SAFETY: The caller's process-lifetime table.
    get_transported_goods_income(
        unsafe { &*s },
        num_pieces,
        dist,
        transit_periods,
        CargoType(cargo_type),
    )
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoPaymentFields" ["probe"] => Fields {
        front,
        route_profit,
        visual_profit,
        visual_transfer,
    };
    "OpenTTDCargoPayment" ["probe"] => CargoPayment {
        fields,
        current_station => station,
    };
    "OpenTTDCargoServices" ["probe"] => Services {
        detach_payment,
        feeder_animation,
        finance,
        industry_delivery,
        registry,
        resolver,
        shared,
        stations,
        subsidies,
        town,
        vehicles,
    };
}

/// `GetTransportedGoodsIncome` and `CargoPayment::PayTransfer` over the retired
/// `tools/migration/cargo-income-gap.cpp` domain (#736 F), against its original-only capture
/// in `cargo_payment/income-oracle.txt`.
#[cfg(test)]
mod income_tests {
    use super::*;
    use crate::cargotype::CargoSpec;
    use crate::oracle;
    use crate::test_tables::CARGO;
    use std::cell::Cell;
    use std::ptr;

    const ORACLE: &str = include_str!("cargo_payment/income-oracle.txt");
    const PAYMENTS: [i64; 7] = [
        i64::MIN,
        i32::MIN as i64,
        -1,
        0,
        6000,
        i32::MAX as i64,
        i64::MAX,
    ];
    const PERIODS1: [u8; 3] = [0, 20, 255];
    const PERIODS2: [u8; 5] = [0, 20, 224, 225, 255];
    const PIECES: [u32; 7] = [0, 1, 255, 256, 65535, 0x8000_0000, u32::MAX];
    const DISTANCES: [u32; 6] = [0, 1, 65535, 65536, 0x8000_0000, u32::MAX];
    const PERIODS: [u16; 7] = [0, 20, 244, 245, 255, 256, 65535];
    const CALLBACKS: [u16; 6] = [0, 0x3FFF, 0x4000, 0x7FFF, 0x8000, 0xFFFF];
    const EXTREMES: [i64; 4] = [i64::MIN, -1, 0, i64::MAX];
    const SHARES: [u8; 4] = [0, 75, 100, 255];

    thread_local! {
        /// The fixture's spec: payment, valid, transit periods; its profit callback is enabled.
        static SPEC: Cell<(i64, u8, u8, u8)> = const { Cell::new((0, 0, 0, 0)) };
        /// The spec's `transit_periods` member, which its reader addresses.
        static TRANSIT_PERIODS: Cell<[u8; 2]> = const { Cell::new([0, 0]) };
        static CALLBACK: Cell<u16> = const { Cell::new(0) };
        static CALLS: Cell<u32> = const { Cell::new(0) };
        /// The last callback's `var18`; like the fixture's global, never reset.
        static ARGUMENT: Cell<u32> = const { Cell::new(0) };
        static SHARE: Cell<u8> = const { Cell::new(0) };
    }
    extern "C" fn registry_spec(_: u8) -> *const CargoSpec {
        ptr::null()
    }
    extern "C" fn bitnum(_: *const CargoSpec) -> u8 {
        if SPEC.get().1 == 0 {
            crate::cargotype::INVALID_CARGO_BITNUM
        } else {
            0
        }
    }
    extern "C" fn callback_mask(_: *const CargoSpec) -> u8 {
        crate::newgrf_callbacks::CargoCallbackMasks::from_element(CargoCallbackMask::ProfitCalc).0
    }
    extern "C" fn current_payment(_: *const CargoSpec) -> i64 {
        SPEC.get().0
    }
    extern "C" fn transit_periods(_: *const CargoSpec) -> *const [u8; 2] {
        let (_, _, periods1, periods2) = SPEC.get();
        TRANSIT_PERIODS.set([periods1, periods2]);
        TRANSIT_PERIODS.with(Cell::as_ptr)
    }
    extern "C" fn callback_frame(
        _: *const CargoSpec,
        _: u16,
        _: u32,
        callback_param2: u32,
        _: *mut i32,
        _: usize,
    ) -> u16 {
        CALLS.set(CALLS.get() + 1);
        ARGUMENT.set(callback_param2);
        CALLBACK.get()
    }

    /// Split a case index of nested loops into its digits, innermost loop first.
    fn digits<const N: usize>(mut index: usize, radices: [usize; N]) -> [usize; N] {
        radices.map(|radix| {
            let digit = index % radix;
            index /= radix;
            digit
        })
    }

    /// Income for each case of the fixture's nested domain: (inputs, "income calls argument").
    #[allow(
        unsafe_code,
        reason = "the fixture builds, reads or frees raw payments, packets and records"
    )]
    fn income_cases(s: &Services) -> Vec<(String, String)> {
        let radices = [
            CALLBACKS.len(),
            PERIODS.len(),
            DISTANCES.len(),
            PIECES.len(),
            PERIODS2.len(),
            PERIODS1.len(),
            PAYMENTS.len(),
        ];
        (0..radices.iter().product())
            .map(|index| {
                let [callback, periods, distance, pieces, p2, p1, payment] = digits(index, radices);
                let (callback, periods, distance, pieces) = (
                    CALLBACKS[callback],
                    PERIODS[periods],
                    DISTANCES[distance],
                    PIECES[pieces],
                );
                let (p1, p2, payment) = (PERIODS1[p1], PERIODS2[p2], PAYMENTS[payment]);
                CALLBACK.set(callback);
                CALLS.set(0);
                SPEC.set((payment, 1, p1, p2));
                // SAFETY: The test's live services.
                let income = unsafe { openttd_rust_cargo_income(s, pieces, distance, periods, 0) };
                (
                    format!(
                        "payment {payment} periods {p1}/{p2} pieces {pieces} distance {distance} transit {periods} callback {callback:#x}"
                    ),
                    format!("{income} {} {}", CALLS.get(), ARGUMENT.get()),
                )
            })
            .collect()
    }

    /// Original-order case indices of the income domain enumerated in `order`, outermost
    /// first, over dimensions 0 payment, 1 `transit_periods[0]`, 2 `transit_periods[1]`,
    /// 3 pieces, 4 distance, 5 periods and 6 callback.
    fn in_order(order: [usize; 7]) -> Vec<usize> {
        let lengths = [
            PAYMENTS.len(),
            PERIODS1.len(),
            PERIODS2.len(),
            PIECES.len(),
            DISTANCES.len(),
            PERIODS.len(),
            CALLBACKS.len(),
        ];
        let mut strides = [1_usize; 7];
        for d in (0..6).rev() {
            strides[d] = strides[d + 1] * lengths[d + 1];
        }
        (0..lengths.iter().product())
            .map(|mut rest: usize| {
                let mut index = 0;
                for &d in order.iter().rev() {
                    index += (rest % lengths[d]) * strides[d];
                    rest /= lengths[d];
                }
                index
            })
            .collect()
    }

    /// One column of `cases`: the space-separated output fields `fields`, in `order`.
    fn column(
        cases: &[(String, String)],
        order: Option<[usize; 7]>,
        fields: std::ops::Range<usize>,
    ) -> Vec<(String, String)> {
        let indices = order.map_or_else(|| (0..cases.len()).collect(), in_order);
        indices
            .into_iter()
            .map(|index| {
                let (inputs, output) = &cases[index];
                let output: Vec<&str> = output.split(' ').collect();
                (inputs.clone(), output[fields.clone()].join(" "))
            })
            .collect()
    }

    /// `PayTransfer` of 65535 of a 65535-piece packet, distance 250 and 45 periods, with the
    /// income loop's last spec: (inputs, "profit `visual_transfer` calls argument").
    fn transfer_cases(s: &Services) -> Vec<(String, String)> {
        extern "C" fn draw(_: *mut std::ffi::c_void) -> u32 {
            0
        }
        // A 256 by 256 map: the packet was loaded on tile (0, 0) and is paid on (250, 0).
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: ptr::null(),
                extended: ptr::null(),
                count: 256 * 256,
                assertions: true,
            },
            8,
        );
        let settings = crate::settings_type::SettingAddresses {
            feeder_payment_share: SHARE.with(Cell::as_ptr),
            ..crate::settings_type::SettingAddresses::NULL
        };
        let mut shared = crate::services::fixture(ptr::null_mut(), draw);
        shared.settings = &raw const settings;
        let radices = [SHARES.len(), EXTREMES.len(), EXTREMES.len()];
        (0..radices.iter().product())
            .map(|index| {
                let [share, feeder, initial] = digits(index, radices);
                transfer_case(
                    s,
                    &shared,
                    EXTREMES[initial],
                    EXTREMES[feeder],
                    SHARES[share],
                )
            })
            .collect()
    }
    #[allow(
        unsafe_code,
        reason = "the fixture builds, reads or frees raw payments, packets and records"
    )]
    fn transfer_case(
        s: &Services,
        shared: &crate::services::Services,
        initial: i64,
        feeder: i64,
        share: u8,
    ) -> (String, String) {
        SHARE.set(share);
        CALLS.set(0);
        let payment = openttd_rust_cargo_payment_new(ptr::null_mut());
        // Loaded on tile 0 (`travelled` holds its coordinates, zero) and in transit 45 periods,
        // so `GetDistance` on tile 250 is 250.
        let packet = crate::cargo_storage::CargoPacket {
            feeder_share: feeder,
            source_xy: 0,
            count: 65535,
            periods_in_transit: 45,
            in_vehicle: 1,
            ..crate::cargo_storage::CargoPacket::default()
        };
        // SAFETY: Allocated above and released below; the packet is a local.
        let (profit, fields) = unsafe {
            (*payment).fields = Fields {
                front: ptr::null_mut(),
                route_profit: i64::MIN,
                visual_profit: i64::MAX,
                visual_transfer: initial,
            };
            let profit = pay_transfer(
                payment,
                s,
                shared,
                CargoType(0),
                &raw const packet,
                65535,
                crate::tile_type::TileIndex(250),
            );
            let fields = (*payment).fields;
            openttd_rust_cargo_payment_destroy(payment, ptr::null(), 1);
            (profit, fields)
        };
        let inputs = format!("visual_transfer {initial} feeder_share {feeder} share {share}");
        // The fixture's own check: a transfer leaves the route and visual profits alone.
        assert_eq!(
            (fields.route_profit, fields.visual_profit),
            (i64::MIN, i64::MAX),
            "{inputs}: route and visual profit"
        );
        let output = format!(
            "{profit} {} {} {}",
            fields.visual_transfer,
            CALLS.get(),
            ARGUMENT.get()
        );
        (inputs, output)
    }

    #[allow(
        unsafe_code,
        reason = "the fixture builds, reads or frees raw payments, packets and records"
    )]
    #[test]
    fn income_and_transfer_match_original() {
        let mut registry = crate::cargo_registry::TEST_TABLE;
        registry.spec = registry_spec;
        registry.bitnum = bitnum;
        registry.callback_mask = callback_mask;
        registry.current_payment = current_payment;
        registry.transit_periods = transit_periods;
        registry.callback_frame = callback_frame;
        let mut s = CARGO;
        s.registry = &raw const registry;
        let sections = oracle::parse(ORACLE);

        // The cases run in the fixture's order: its callback argument and spec carry over.
        // Each column is then compared in its own stored order.
        let income = income_cases(&s);
        let transfer = transfer_cases(&s);
        let income_order = Some([3, 6, 0, 1, 2, 4, 5]);
        let var18_order = Some([5, 4, 3, 6, 1, 2, 0]);
        for (name, cases, order, fields) in [
            ("income", &income, income_order, 0..2),
            ("var18", &income, var18_order, 2..3),
            ("transfer", &transfer, None, 0..2),
            ("transfer-callback", &transfer, None, 2..4),
        ] {
            assert_eq!(
                oracle::check(&sections, name, column(cases, order, fields)),
                cases.len(),
                "{name}"
            );
        }
        assert_eq!(income.len(), 185_220);
        assert_eq!(transfer.len(), 64);
        let (payment, _, periods1, periods2) = SPEC.get();
        SPEC.set((payment, 0, periods1, periods2));
        CALLS.set(0);
        // SAFETY: The test's live services.
        let income =
            unsafe { openttd_rust_cargo_income(&raw const s, u32::MAX, u32::MAX, 65535, 0) };
        let invalid = [(
            "invalid spec, pieces and distance UINT32_MAX, transit 65535".to_owned(),
            format!("{income} {}", CALLS.get()),
        )];
        assert_eq!(oracle::check(&sections, "invalid", invalid), 1);
    }
}

/// `CargoPayment::~CargoPayment` settlement from the retired `src/tests/cargo_payment.cpp`
/// (#736 F): the fixture's operation order, `Money` boundaries and cleanup, with its own
/// expected values.
#[cfg(test)]
mod settlement_tests {
    use super::*;
    use crate::economy_type::ExpensesType;
    use crate::test_tables::{CARGO, FINANCE, SHARED, VEHICLE};
    use crate::vehicle::VehicleRef;
    use std::cell::{Cell, RefCell};
    use std::ptr;

    /// The fixture's settlement log: (operation, first value, second value). Operations:
    /// 0 detach, 1 current company, 2 select the owner, 3 invalidate its windows, 4 write the
    /// profit, 5 local company, 6 vehicle sound, 7 cash-till sound, 8 feeder animation,
    /// 9 cost animation, 10 restore the company.
    type Settlement = Vec<(u8, i64, i64)>;
    thread_local! {
        static SETTLEMENT: RefCell<Settlement> = const { RefCell::new(Vec::new()) };
        static FINANCES: Cell<*mut crate::company::Finances> = const { Cell::new(ptr::null_mut()) };
        static LOCAL: Cell<bool> = const { Cell::new(false) };
        static CUSTOM_SOUND: Cell<bool> = const { Cell::new(false) };
        static VALID_COMPANY: Cell<bool> = const { Cell::new(false) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
    }
    fn log(operation: u8, first: i64, second: i64) {
        SETTLEMENT.with(|s| s.borrow_mut().push((operation, first, second)));
    }
    extern "C" fn current_company() -> u8 {
        log(1, 0, 0);
        CURRENT.get()
    }
    extern "C" fn set_current_company(id: u8) {
        log(if id == 3 { 2 } else { 10 }, i64::from(id), 0);
        CURRENT.set(id);
    }
    extern "C" fn company_finances(id: u8) -> *mut crate::company::Finances {
        assert_eq!(id, 3);
        if VALID_COMPANY.get() {
            FINANCES.get()
        } else {
            ptr::null_mut()
        }
    }
    extern "C" fn invalidate_company_windows(id: u8) {
        log(3, i64::from(id), 0);
    }
    extern "C" fn write_profit(_: VehicleRef, value: i64) {
        log(4, value, 0);
    }
    extern "C" fn play_vehicle_sound(
        _: VehicleRef,
        event: crate::newgrf_sound::VehicleSoundEvent,
    ) -> bool {
        assert_eq!(event.0, crate::newgrf_sound::VSE_LOAD_UNLOAD.0);
        log(6, 0, 0);
        CUSTOM_SOUND.get()
    }
    extern "C" fn play_sound(_: VehicleRef, sound: u16) {
        assert_eq!(sound, crate::sound_type::SND_14_CASHTILL);
        log(7, 0, 0);
    }
    extern "C" fn detach_payment(_: *mut OpenTTDVehicle) {
        log(0, 0, 0);
    }
    /// `_local_company`: equal to `_current_company` exactly for a local case.
    extern "C" fn local_company() -> u8 {
        log(5, 0, 0);
        if LOCAL.get() {
            CURRENT.get()
        } else {
            CURRENT.get().wrapping_add(1)
        }
    }
    /// `_current_company`, as `SubtractMoneyFromCompany` and `IsLocalCompany` read it.
    extern "C" fn shared_current_company() -> u8 {
        CURRENT.get()
    }
    extern "C" fn profit(_: VehicleRef) -> i64 {
        1000
    }
    extern "C" fn cost_animation(x: i32, y: i32, z: i32, income: i64) {
        assert_eq!((x, y, z), (11, 22, 33));
        log(9, 0, income);
    }
    extern "C" fn feeder_animation(x: i32, y: i32, z: i32, transfer: i64, income: i64) {
        assert_eq!((x, y, z), (11, 22, 33));
        log(8, transfer, income);
    }

    /// One fixture case: cleanup or settlement, a maximal or no transfer, a local company,
    /// a custom vehicle sound and a valid company.
    /// The fixture's nested loop index: bit 4 cleaning, then transfer, local, custom and bit 0
    /// valid, so counting up follows its loop order.
    #[derive(Clone, Copy)]
    struct Case(u8);
    impl Case {
        fn bit(self, n: u8) -> bool {
            self.0 & (16 >> n) != 0
        }
        fn cleaning(self) -> bool {
            self.bit(0)
        }
        fn transfer(self) -> bool {
            self.bit(1)
        }
        fn local(self) -> bool {
            self.bit(2)
        }
        fn custom(self) -> bool {
            self.bit(3)
        }
        fn valid(self) -> bool {
            self.bit(4)
        }
    }

    /// The fixture's expected operation order for one settled payment.
    fn expected_order(case: Case) -> Vec<u8> {
        let mut order = vec![0, 1, 2];
        if case.valid() {
            order.push(3);
        }
        order.extend([4, 5]);
        if case.local() {
            order.push(6);
            if !case.custom() {
                order.push(7);
            }
        }
        order.push(if case.transfer() { 8 } else { 9 });
        order.push(10);
        order
    }

    /// One fixture case: settle (or clean up) a payment whose route and visual profits are
    /// `INT64_MIN`, with a maximal or no transfer.
    #[allow(
        unsafe_code,
        reason = "the fixture builds, reads or frees raw payments, packets and records"
    )]
    fn settle(s: &Services, case: Case) {
        let (cleaning, transfer, local, custom, valid) = (
            case.cleaning(),
            case.transfer(),
            case.local(),
            case.custom(),
            case.valid(),
        );
        let inputs = format!(
            "cleaning {cleaning} transfer {transfer} local {local} custom {custom} valid {valid}"
        );
        SETTLEMENT.with(|s| s.borrow_mut().clear());
        // SAFETY: The case's zeroed finance record, owned by this thread.
        unsafe { FINANCES.get().write(std::mem::zeroed()) };
        CURRENT.set(17);
        LOCAL.set(local);
        CUSTOM_SOUND.set(custom);
        VALID_COMPANY.set(valid);
        let front = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            crate::vehicle_type::VehicleType::TRAIN,
        );
        // SAFETY: The record allocated above, released at the end of the case.
        unsafe {
            (*front).owner = crate::company_type::CompanyID(3);
            ((*front).x_pos, (*front).y_pos, (*front).z_pos) = (11, 22, 33);
        }
        let payment = openttd_rust_cargo_payment_new(front);
        // SAFETY: The payment allocated above; destroy releases it.
        unsafe {
            (*payment).fields = Fields {
                front,
                route_profit: i64::MIN,
                visual_profit: i64::MIN,
                visual_transfer: if transfer { i64::MAX } else { 0 },
            };
            assert_eq!(
                openttd_rust_cargo_payment_front(payment),
                front,
                "{inputs}: front"
            );
            openttd_rust_cargo_payment_destroy(payment, s, u8::from(cleaning));
        }
        let settlement = SETTLEMENT.with(|s| s.borrow().clone());
        // SAFETY: The settlement has ended; nothing else holds the record or the finances.
        let (money, revenue) = unsafe {
            crate::vehicle_base::openttd_rust_vehicle_record_delete(front);
            let finances = FINANCES.get();
            (
                (*finances).money,
                (*finances).yearly_expenses[0][usize::from(ExpensesType::EXPENSES_TRAIN_REVENUE.0)],
            )
        };
        if cleaning {
            assert!(settlement.is_empty(), "{inputs}: cleanup settles nothing");
            return;
        }
        let order: Vec<u8> = settlement
            .iter()
            .map(|&(operation, ..)| operation)
            .collect();
        assert_eq!(order, expected_order(case), "{inputs}: order");
        let expected = if valid { (-i64::MAX, i64::MAX) } else { (0, 0) };
        assert_eq!((money, revenue), expected, "{inputs}: money, revenue");
        assert_eq!(
            settlement[if valid { 4 } else { 3 }].1,
            if transfer { 744 } else { 1000 },
            "{inputs}: written profit"
        );
        assert_eq!(
            settlement[settlement.len() - 2].2,
            i64::MAX,
            "{inputs}: animated income"
        );
        assert_eq!(CURRENT.get(), 17, "{inputs}: restored company");
    }

    #[allow(
        unsafe_code,
        reason = "the fixture builds, reads or frees raw payments, packets and records"
    )]
    #[test]
    fn settlement_order_money_boundaries_and_cleanup() {
        let mut finance = FINANCE;
        finance.current_company = current_company;
        finance.set_current_company = set_current_company;
        finance.company_finances = company_finances;
        finance.invalidate_company_windows = invalidate_company_windows;
        let mut vehicles = VEHICLE;
        vehicles.profit = profit;
        vehicles.write_profit = write_profit;
        vehicles.play_vehicle_sound = play_vehicle_sound;
        vehicles.play_sound = play_sound;
        let mut shared = SHARED;
        shared.cost_animation = cost_animation;
        shared.current_company = shared_current_company;
        shared.local_company = local_company;
        let mut s = CARGO;
        s.finance = &raw const finance;
        s.vehicles = &raw const vehicles;
        s.detach_payment = detach_payment;
        s.shared = &raw const shared;
        s.feeder_animation = feeder_animation;
        // SAFETY: Plain integer record; every case rewrites it before use.
        let mut finances: Box<crate::company::Finances> = Box::new(unsafe { std::mem::zeroed() });
        FINANCES.set(&raw mut *finances);

        // The fixture's nested loops, outermost first.
        for case in 0..32 {
            settle(&s, Case(case));
        }
        // A payment with neither visual profit nor transfer only detaches.
        SETTLEMENT.with(|s| s.borrow_mut().clear());
        let payment = openttd_rust_cargo_payment_new(ptr::null_mut());
        // SAFETY: The payment allocated above; destroy releases it.
        unsafe {
            (*payment).fields = Fields {
                front: ptr::null_mut(),
                route_profit: i64::MAX,
                visual_profit: 0,
                visual_transfer: 0,
            };
            openttd_rust_cargo_payment_destroy(payment, &raw const s, 0);
        }
        assert_eq!(SETTLEMENT.with(|s| s.borrow().clone()), [(0, 0, 0)]);
        FINANCES.set(ptr::null_mut());
    }
}

/// `Vehicle::GetExpenseType` and its four overrides, which the settlement charges with
/// `income` and refit with a running cost.
#[cfg(test)]
mod expense_type_tests {
    use crate::economy_type::ExpensesType;
    use crate::vehicle_type::VehicleType;

    #[test]
    #[allow(
        unsafe_code,
        reason = "the test allocates and frees raw vehicle records"
    )]
    fn get_expense_type_over_every_vehicle_type_and_both_flags() {
        // Each type's (`GetExpenseType(true)`, `GetExpenseType(false)`), from `train.h`,
        // `roadveh.h`, `ship.h`, `aircraft.h` and, for the rest, `vehicle_base.h`.
        for (kind, expected) in [
            (
                VehicleType::TRAIN,
                (
                    ExpensesType::EXPENSES_TRAIN_REVENUE,
                    ExpensesType::EXPENSES_TRAIN_RUN,
                ),
            ),
            (
                VehicleType::ROAD,
                (
                    ExpensesType::EXPENSES_ROADVEH_REVENUE,
                    ExpensesType::EXPENSES_ROADVEH_RUN,
                ),
            ),
            (
                VehicleType::SHIP,
                (
                    ExpensesType::EXPENSES_SHIP_REVENUE,
                    ExpensesType::EXPENSES_SHIP_RUN,
                ),
            ),
            (
                VehicleType::AIRCRAFT,
                (
                    ExpensesType::EXPENSES_AIRCRAFT_REVENUE,
                    ExpensesType::EXPENSES_AIRCRAFT_RUN,
                ),
            ),
            (
                VehicleType::EFFECT,
                (ExpensesType::EXPENSES_OTHER, ExpensesType::EXPENSES_OTHER),
            ),
            (
                VehicleType::DISASTER,
                (ExpensesType::EXPENSES_OTHER, ExpensesType::EXPENSES_OTHER),
            ),
        ] {
            let record =
                crate::vehicle_base::openttd_rust_vehicle_record_new(std::ptr::null_mut(), kind);
            let v = crate::vehicle::VehicleRef(std::ptr::NonNull::new(record).unwrap());
            assert_eq!(
                (v.get_expense_type(true), v.get_expense_type(false)),
                expected,
                "{kind:?}"
            );
            // SAFETY: The record allocated above; nothing else holds it.
            unsafe { crate::vehicle_base::openttd_rust_vehicle_record_delete(record) };
        }
    }
}

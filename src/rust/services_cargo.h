/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_cargo.h Shared cargo leaves for synchronous Rust services. */
#ifndef RUST_SERVICES_CARGO_H
#define RUST_SERVICES_CARGO_H
#include "../cargotype.h"
#include "../economy_func.h"
#include "../newgrf_cargo.h"
#include "../industrytype.h"
struct OpenTTDIndustrySpecHandle;

/* Keep native types and source observation points. Scaling may draw Random;
 * delivery may reenter owners, so callers end their owner access first.
 * Environmental failures terminate inside these noexcept leaves. */
inline uint OpenTTDScaleCargo(uint amount, bool town) noexcept { return ScaleByCargoScale(amount, town); }
inline CargoType OpenTTDCargoTranslation(uint8_t cargo, const GRFFile *grf) noexcept { return GetCargoTranslation(cargo, grf); }
inline CargoType OpenTTDCargoByLabel(CargoLabel label) noexcept { return GetCargoTypeByLabel(label); }
inline uint OpenTTDMoveGoodsToStation(CargoType cargo, uint amount, Source source, const StationList &stations, Owner exclusivity = INVALID_OWNER) noexcept
{
	return MoveGoodsToStation(cargo, amount, source, stations, exclusivity);
}
/* Legacy tables specialize the shared meanings without another native copy. */
inline uint OpenTTDScaleTownCargo(uint amount) noexcept { return OpenTTDScaleCargo(amount, true); }
inline uint OpenTTDScaleIndustryCargo(uint amount) noexcept { return OpenTTDScaleCargo(amount, false); }
inline CargoType OpenTTDFoodCargo() noexcept { return OpenTTDCargoByLabel(CT_FOOD); }
inline CargoType OpenTTDPassengerCargo() noexcept { return OpenTTDCargoByLabel(CT_PASSENGERS); }
inline CargoType OpenTTDIndustryCargoTranslation(const OpenTTDIndustrySpecHandle *spec, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, reinterpret_cast<const IndustrySpec *>(spec)->grf_prop.grffile); }
inline CargoType OpenTTDIndustryTileCargoTranslation(const IndustryTileSpec *spec, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, spec->grf_prop.grffile); }
#endif /* RUST_SERVICES_CARGO_H */

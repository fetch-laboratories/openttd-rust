/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_base.h Base class for engines. */

#ifndef ENGINE_BASE_H
#define ENGINE_BASE_H

#include "engine_type.h"
#include "vehicle_type.h"
#include "core/enum_type.hpp"
#include "core/pool_type.hpp"
#include "newgrf_commons.h"
#include "timer/timer_game_calendar.h"
#include "rust/engine_lifecycle.hpp"
#include "rust/owned_bytes.h"

/** Flags used client-side in the purchase/autorenew engine list. */
enum class EngineDisplayFlag : uint8_t {
	HasVariants, ///< Set if engine has variants.
	IsFolded, ///< Set if display of variants should be folded (hidden).
	Shaded, ///< Set if engine should be masked.
};

using EngineDisplayFlags = EnumBitSet<EngineDisplayFlag, uint8_t>;

typedef Pool<Engine, EngineID, 64> EnginePool;
extern EnginePool _engine_pool;

class Engine : public EnginePool::PoolItem<&_engine_pool> {
public:
	EngineLifecycleOwner rust_lifecycle{this};
	OpenTTDEngineLifecycle &Record() { return *this->rust_lifecycle.state; }
	const OpenTTDEngineLifecycle &Record() const { return *this->rust_lifecycle.state; }
	std::string_view Name() const { return RustBytesView(this->Record().name); }
	void SetName(std::string_view text) { RustBytesAssign(this->Record().name, text); }

	EngineDisplayFlags display_flags{}; ///< NOSAVE client-side-only display flags for build engine list.
	EngineID display_last_variant = EngineID::Invalid(); ///< NOSAVE client-side-only last variant selected.


	Engine() {}
	Engine(Engine &&other) noexcept
		: rust_lifecycle(std::move(other.rust_lifecycle)), display_flags(other.display_flags), display_last_variant(other.display_last_variant)
	{
		this->Record().shell = this;
	}
	Engine(const Engine &) = delete;
	Engine &operator=(const Engine &) = delete;
	Engine(VehicleType type, uint16_t local_id);
	bool IsEnabled() const;

	/**
	 * Determines the default cargo type of an engine.
	 *
	 * Usually a valid cargo is returned, even though the vehicle has zero capacity, and can therefore not carry anything. But the cargotype is still used
	 * for livery selection etc..
	 *
	 * Vehicles with INVALID_CARGO as default cargo are usually not available, but it can appear as default cargo of articulated parts.
	 *
	 * @return The default cargo type.
	 * @see CanCarryCargo
	 */
	CargoType GetDefaultCargoType() const
	{
		return this->Record().info.cargo_type;
	}

	uint DetermineCapacity(const Vehicle *v, uint16_t *mail_capacity = nullptr) const;

	bool CanCarryCargo() const;

	/**
	 * Determines the default cargo capacity of an engine for display purposes.
	 *
	 * For planes carrying both passenger and mail this is the passenger capacity.
	 * For multiheaded engines this is the capacity of both heads.
	 * For articulated engines use GetCapacityOfArticulatedParts
	 *
	 * @param mail_capacity returns secondary cargo (mail) capacity of aircraft
	 * @return The default capacity
	 * @see GetDefaultCargoType
	 */
	uint GetDisplayDefaultCapacity(uint16_t *mail_capacity = nullptr) const
	{
		return this->DetermineCapacity(nullptr, mail_capacity);
	}

	Money GetRunningCost() const;
	Money GetCost() const;
	uint GetDisplayMaxSpeed() const;
	uint GetPower() const;
	uint GetDisplayWeight() const;
	uint GetDisplayMaxTractiveEffort() const;
	TimerGameCalendar::Date GetLifeLengthInDays() const;
	uint16_t GetRange() const;
	StringID GetAircraftTypeText() const;

	/**
	 * Check whether the engine is hidden in the GUI for the given company.
	 * @param c Company to check.
	 * @return \c true iff the engine is hidden in the GUI for the given company.
	 */
	inline bool IsHidden(CompanyID c) const
	{
		return c < MAX_COMPANIES && this->Record().company_hidden.Test(c);
	}

	/**
	 * Get the last display variant for an engine.
	 * @return Engine's last display variant or engine itself if no last display variant is set.
	 */
	const Engine *GetDisplayVariant() const
	{
		if (this->display_last_variant == this->index || this->display_last_variant == EngineID::Invalid()) return this;
		return Engine::Get(this->display_last_variant);
	}

	bool IsVariantHidden(CompanyID c) const;

	/**
	 * Check if the engine is a ground vehicle.
	 * @return True iff the engine is a train or a road vehicle.
	 */
	inline bool IsGroundVehicle() const
	{
		return this->Record().type == VEH_TRAIN || this->Record().type == VEH_ROAD;
	}

	/**
	 * Retrieve the NewGRF the engine is tied to.
	 * This is the GRF providing the Action 3.
	 * @return NewGRF associated to the engine.
	 */
	const GRFFile *GetGRF() const
	{
		return this->Record().bindings.grf_prop.grffile;
	}

	uint32_t GetGRFID() const;

	struct EngineTypeFilter {
		VehicleType vt;

		bool operator() (size_t index) { return Engine::Get(index)->Record().type == this->vt; }
	};

	template <typename T>
	inline T &VehInfo()
	{
		return const_cast<T &>(std::as_const(*this).VehInfo<T>());
	}

	template <typename T>
	inline const T &VehInfo() const
	{
		const auto &record = this->Record();
		if constexpr (std::is_same_v<T, RailVehicleInfo>) {
			if (record.vehicle_info_type == VEH_TRAIN) return record.vehicle_info.rail;
		} else if constexpr (std::is_same_v<T, RoadVehicleInfo>) {
			if (record.vehicle_info_type == VEH_ROAD) return record.vehicle_info.road;
		} else if constexpr (std::is_same_v<T, ShipVehicleInfo>) {
			if (record.vehicle_info_type == VEH_SHIP) return record.vehicle_info.ship;
		} else if constexpr (std::is_same_v<T, AircraftVehicleInfo>) {
			if (record.vehicle_info_type == VEH_AIRCRAFT) return record.vehicle_info.aircraft;
		}
		throw std::bad_variant_access{};
	}

	/**
	 * Returns an iterable ensemble of all valid engines of the given type
	 * @param vt the VehicleType for engines to be valid
	 * @param from index of the first engine to consider
	 * @return an iterable ensemble of all valid engines of the given type
	 */
	static Pool::IterateWrapperFiltered<Engine, EngineTypeFilter> IterateType(VehicleType vt, size_t from = 0)
	{
		return Pool::IterateWrapperFiltered<Engine, EngineTypeFilter>(from, EngineTypeFilter{ vt });
	}
};

struct EngineIDMapping {
	uint32_t grfid = 0; ///< The GRF ID of the file the entity belongs to
	uint16_t internal_id = 0; ///< The internal ID within the GRF file
	VehicleType type{}; ///< The engine type
	uint8_t substitute_id = 0; ///< The (original) entity ID to use if this GRF is not available (currently not used)
	EngineID engine{};

	static inline uint64_t Key(uint32_t grfid, uint16_t internal_id) { return static_cast<uint64_t>(grfid) << 32 | internal_id; }

	inline uint64_t Key() const { return Key(this->grfid, this->internal_id); }

	EngineIDMapping() {}
	EngineIDMapping(uint32_t grfid, uint16_t internal_id, VehicleType type, uint8_t substitute_id, EngineID engine)
		: grfid(grfid), internal_id(internal_id), type(type), substitute_id(substitute_id), engine(engine) {}
};

/** Projection to get a unique key of an EngineIDMapping, used for sorting in EngineOverrideManager. */
struct EngineIDMappingKeyProjection {
	inline uint64_t operator()(const EngineIDMapping &eid) const { return eid.Key(); }
};

/**
 * Stores the mapping of EngineID to the internal id of newgrfs.
 * Note: This is not part of Engine, as the data in the EngineOverrideManager and the engine pool get reset in different cases.
 */
struct EngineOverrideManager {
	std::span<const EngineIDMapping> Mappings(VehicleType type) const;
	void ClearMapping();

	void ResetToDefaultMapping();
	EngineID GetID(VehicleType type, uint16_t grf_local_id, uint32_t grfid);
	void SetID(VehicleType type, uint16_t grf_local_id, uint32_t grfid, uint8_t substitute_id, EngineID engine);

	static bool ResetToCurrentNewGRFConfig();
};

extern EngineOverrideManager _engine_mngr;

inline const EngineInfo *EngInfo(EngineID e)
{
	return &Engine::Get(e)->Record().info;
}

inline const RailVehicleInfo *RailVehInfo(EngineID e)
{
	return &Engine::Get(e)->VehInfo<RailVehicleInfo>();
}

inline const RoadVehicleInfo *RoadVehInfo(EngineID e)
{
	return &Engine::Get(e)->VehInfo<RoadVehicleInfo>();
}

inline const ShipVehicleInfo *ShipVehInfo(EngineID e)
{
	return &Engine::Get(e)->VehInfo<ShipVehicleInfo>();
}

inline const AircraftVehicleInfo *AircraftVehInfo(EngineID e)
{
	return &Engine::Get(e)->VehInfo<AircraftVehicleInfo>();
}

#endif /* ENGINE_BASE_H */

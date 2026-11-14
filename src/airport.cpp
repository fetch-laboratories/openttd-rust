/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file airport.cpp Functions related to airports. */

#include "stdafx.h"
#include "station_base.h"

#include "table/strings.h"
#include "table/airport_movement.h"
#include "table/airporttile_ids.h"

#include "safeguards.h"


/**
 * Define a generic airport.
 * @param name Suffix of the names of the airport data.
 * @param terminals The terminals.
 * @param num_helipads Number of heli pads.
 * @param flags Information about the class of FTA.
 * @param delta_z Height of the airport above the land.
 */
#define AIRPORT_GENERIC(name, terminals, num_helipads, flags, delta_z) \
	static const AirportFTAClass _airportfta_ ## name(_airport_moving_data_ ## name, terminals, \
			num_helipads, _airport_entries_ ## name, flags, _airport_fta_ ## name, delta_z);

/**
 * Define an airport.
 * @param name Suffix of the names of the airport data.
 * @param num_helipads Number of heli pads.
 * @param short_strip Airport has a short land/take-off strip.
 */
#define AIRPORT(name, num_helipads, short_strip) \
	AIRPORT_GENERIC(name, _airport_terminal_ ## name, num_helipads, AirportFTAClass::Flags({AirportFTAClass::Flag::Airplanes, AirportFTAClass::Flag::Helicopters}) | (short_strip ? AirportFTAClass::Flags{AirportFTAClass::Flag::ShortStrip} : AirportFTAClass::Flags{}), 0)

/**
 * Define a heliport.
 * @param name Suffix of the names of the helipad data.
 * @param num_helipads Number of heli pads.
 * @param delta_z Height of the airport above the land.
 */
#define HELIPORT(name, num_helipads, delta_z) \
	AIRPORT_GENERIC(name, nullptr, num_helipads, AirportFTAClass::Flag::Helicopters, delta_z)

AIRPORT(country, 0, true)
AIRPORT(city, 0, false)
HELIPORT(heliport, 1, 60)
AIRPORT(metropolitan, 0, false)
AIRPORT(international, 2, false)
AIRPORT(commuter, 2, true)
HELIPORT(helidepot, 1, 0)
AIRPORT(intercontinental, 2, false)
HELIPORT(helistation, 3, 0)
HELIPORT(oilrig, 1, 54)
AIRPORT_GENERIC(dummy, nullptr, 0, AirportFTAClass::Flags({AirportFTAClass::Flag::Airplanes, AirportFTAClass::Flag::Helicopters}), 0)

#undef HELIPORT
#undef AIRPORT
#undef AIRPORT_GENERIC

#include "table/airport_defaults.h"


static uint16_t AirportGetNofElements(const AirportFTAbuildup *apFA);
static void AirportBuildAutomata(std::vector<AirportFTA> &layout, uint8_t nofelements, const AirportFTAbuildup *apFA);


AirportFTAClass::AirportFTAClass(
	const AirportMovingData *moving_data_,
	const uint8_t *terminals_,
	const uint8_t num_helipads_,
	const uint8_t *entry_points_,
	Flags flags_,
	const AirportFTAbuildup *apFA,
	uint8_t delta_z_
) :
	moving_data(moving_data_),
	terminals(terminals_),
	num_helipads(num_helipads_),
	flags(flags_),
	nofelements(AirportGetNofElements(apFA)),
	entry_points(entry_points_),
	delta_z(delta_z_)
{
	/* Build the state machine itself */
	AirportBuildAutomata(this->layout, this->nofelements, apFA);
}

/**
 * Get the number of elements of a source Airport state automata
 * Since it is actually just a big array of AirportFTA types, we only
 * know one element from the other by differing 'position' identifiers
 */
static uint16_t AirportGetNofElements(const AirportFTAbuildup *apFA)
{
	uint16_t nofelements = 0;
	int temp = apFA[0].position;

	for (uint i = 0; i < MAX_ELEMENTS; i++) {
		if (temp != apFA[i].position) {
			nofelements++;
			temp = apFA[i].position;
		}
		if (apFA[i].position == MAX_ELEMENTS) break;
	}
	return nofelements;
}

AirportFTA::AirportFTA(const AirportFTAbuildup &buildup) : blocks(buildup.blocks), position(buildup.position), next_position(buildup.next), heading(buildup.heading)
{
}

/**
 * Construct the FTA given a description.
 * @param layout The vector to write the automata to.
 * @param nofelements The number of elements in the FTA.
 * @param apFA The description of the FTA.
 */
static void AirportBuildAutomata(std::vector<AirportFTA> &layout, uint8_t nofelements, const AirportFTAbuildup *apFA)
{
	uint16_t internalcounter = 0;

	layout.reserve(nofelements);
	for (uint i = 0; i < nofelements; i++) {
		AirportFTA *current = &layout.emplace_back(apFA[internalcounter]);

		/* outgoing nodes from the same position, create linked list */
		while (current->position == apFA[internalcounter + 1].position) {
			current->next = std::make_unique<AirportFTA>(apFA[internalcounter + 1]);
			current = current->next.get();
			internalcounter++;
		}
		internalcounter++;
	}
}

const AirportFTAClass *RustDummyAirportFTA() noexcept { return &_airportfta_dummy; }


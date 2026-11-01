/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_object.cpp Handling of object NewGRFs. */

#include "stdafx.h"
#include "company_base.h"
#include "company_func.h"
#include "debug.h"
#include "genworld.h"
#include "newgrf_badge.h"
#include "newgrf_object.h"
#include "newgrf_sound.h"
#include "object_base.h"
#include "object_map.h"
#include "timer/timer_game_calendar.h"
#include "tile_cmd.h"
#include "town.h"
#include "water.h"
#include "newgrf_animation_base.h"
#include "rust/animation_binding.hpp"
#include "rust/services_object.h"

#include "table/strings.h"

#include "newgrf_class_func.h"

#include "rust/object_ffi.h"
#include "rust/services_newgrf.h"
#include "rust/tile_animation_services.h"
#include "rust/services_map.h"
#include "rust/environment_services.h"

#include "safeguards.h"

/** The override manager for our objects. */
ObjectOverrideManager _object_mngr(NEW_OBJECT_OFFSET, NUM_OBJECTS, INVALID_OBJECT_TYPE);

extern const ObjectSpec _original_objects[NEW_OBJECT_OFFSET];
/** All the object specifications. */
std::vector<ObjectSpec> _object_specs;

const std::vector<ObjectSpec> &ObjectSpec::Specs()
{
	return _object_specs;
}

size_t ObjectSpec::Count()
{
	return _object_specs.size();
}

/**
 * Get the specification associated with a specific ObjectType.
 * @param index The object type to fetch.
 * @return The specification.
 */
/* static */ const ObjectSpec *ObjectSpec::Get(ObjectType index)
{
	/* Empty object if index is out of range -- this might happen if NewGRFs are changed. */
	static ObjectSpec empty = {};

	assert(index < NUM_OBJECTS);
	if (index >= _object_specs.size()) return &empty;
	return &_object_specs[index];
}

/**
 * Get the specification associated with a tile.
 * @param tile The tile to fetch the data for.
 * @return The specification.
 */
/* static */ const ObjectSpec *ObjectSpec::GetByTile(TileIndex tile)
{
	return ObjectSpec::Get(GetObjectType(tile));
}

/**
 * Check whether the object might be available at some point in this game with the current game mode.
 * @return true if it might be available.
 */
bool ObjectSpec::IsEverAvailable() const
{
	return openttd_rust_object_ever_available(&GetRustObjectServices(), this);
}

/**
 * Check whether the object was available at some point in the past or present in this game with the current game mode.
 * @return true if it was ever or is available.
 */
bool ObjectSpec::WasEverAvailable() const
{
	return openttd_rust_object_was_available(&GetRustObjectServices(), this);
}

/**
 * Check whether the object is available at this time.
 * @return true if it is available.
 */
bool ObjectSpec::IsAvailable() const
{
	return openttd_rust_object_available(&GetRustObjectServices(), this);
}

/**
 * Gets the index of this spec.
 * @return The index.
 */
uint ObjectSpec::Index() const
{
	return this - _object_specs.data();
}

/**
 * Tie all ObjectSpecs to their class.
 */
/* static */ void ObjectSpec::BindToClasses()
{
	for (auto &spec : _object_specs) {
		if (spec.IsEnabled() && spec.class_index != INVALID_OBJECT_CLASS) {
			ObjectClass::Assign(&spec);
		}
	}
}

/** This function initialize the spec arrays of objects. */
void ResetObjects()
{
	openttd_rust_object_services_exchange(&GetRustObjectServices());
	/* Clean the pool. */
	_object_specs.clear();

	/* And add our originals. */
	_object_specs.reserve(lengthof(_original_objects));

	for (uint16_t i = 0; i < lengthof(_original_objects); i++) {
		ObjectSpec &spec = _object_specs.emplace_back(_original_objects[i]);
		spec.grf_prop.local_id = i;
	}

	/* Set class for originals. */
	_object_specs[OBJECT_LIGHTHOUSE].class_index = ObjectClass::Allocate('LTHS');
	_object_specs[OBJECT_TRANSMITTER].class_index = ObjectClass::Allocate('TRNS');
}

template <>
std::span<const OpenTTDGRFClassDefault> ObjectClass::Defaults()
{
	static const OpenTTDGRFClassDefault defaults[]{
		{'LTHS', STR_OBJECT_CLASS_LTHS},
		{'TRNS', STR_OBJECT_CLASS_TRNS},
	};
	return defaults;
}


/* Instantiate ObjectClass. */
template class NewGRFClass<ObjectSpec, ObjectClassID, OBJECT_CLASS_MAX>;

/* virtual */ uint32_t ObjectScopeResolver::GetRandomBits() const
{
	return openttd_rust_object_random_bits(&GetRustObjectServices(), this->context.tile);
}


/** Used by the resolver to get values for feature 0F deterministic spritegroups. */
/* virtual */ uint32_t ObjectScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_object_variable(&GetRustObjectServices(), &this->ro, this->context.object, this->context.spec, this->context.tile, this->context.view, variable, parameter, &available);
}

/**
 * Constructor of the object resolver.
 * @param obj Object being resolved.
 * @param tile %Tile of the object.
 * @param view View of the object.
 * @param callback Callback ID.
 * @param param1 First parameter (var 10) of the callback.
 * @param param2 Second parameter (var 18) of the callback.
 */
ObjectResolverObject::ObjectResolverObject(const ObjectSpec *spec, Object *obj, TileIndex tile, uint8_t view,
		CallbackID callback, uint32_t param1, uint32_t param2)
	: ResolverObject(
		openttd_rust_object_file(spec),
		callback, param1, param2), object_scope(*this, obj, spec, tile, view)
{
	this->root_spritegroup = openttd_rust_object_root(&this->object_scope.context);
}

/**
 * Get the town resolver scope that belongs to this object resolver.
 * On the first call, the town scope is created (if possible).
 * @return Town scope, if available.
 */

GrfSpecFeature ObjectResolverObject::GetFeature() const
{
	return static_cast<GrfSpecFeature>(openttd_rust_object_feature());
}

uint32_t ObjectResolverObject::GetDebugID() const
{
	return openttd_rust_object_debug_id(&this->object_scope.context);
}

/**
 * Perform a callback for an object.
 * @param callback The callback to perform.
 * @param param1   The first parameter to pass to the NewGRF.
 * @param param2   The second parameter to pass to the NewGRF.
 * @param spec     The specification of the object / the entry point.
 * @param o        The object to call the callback for.
 * @param tile     The tile the callback is called for.
 * @param view     The view of the object (only used when o == nullptr).
 * @param[out] regs100 Additional result values from registers 100+
 * @return The result of the callback.
 */
uint16_t GetObjectCallback(CallbackID callback, uint32_t param1, uint32_t param2, const ObjectSpec *spec, Object *o, TileIndex tile, std::span<int32_t> regs100, uint8_t view)
{
	return openttd_rust_object_callback(&GetRustObjectServices(), callback, param1, param2, spec, o, tile.base(), regs100.data(), regs100.size(), view);
}

/**
 * Draw an group of sprites on the map.
 * @param ti    Information about the tile to draw on.
 * @param dts   The sprite layout to draw.
 * @param spec  Object spec to draw.
 */
static void DrawTileLayout(const TileInfo *ti, const DrawTileSpriteSpan &dts, const ObjectSpec *spec)
{
	PaletteID palette = (spec->flags.Test(ObjectFlag::Uses2CC) ? SPR_2CCMAP_BASE : PALETTE_RECOLOUR_START) + Object::GetByTile(ti->tile)->colour;

	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		/* If the ground sprite is the default flat water sprite, draw also canal/river borders
		 * Do not do this if the tile's WaterClass is 'land'. */
		if ((image == SPR_FLAT_WATER_TILE || spec->flags.Test(ObjectFlag::DrawWater)) && IsTileOnWater(ti->tile)) {
			DrawWaterClassGround(ti);
		} else {
			DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, palette));
		}
	}

	DrawNewGRFTileSeq(ti, &dts, TO_STRUCTURES, 0, palette);
}

/**
 * Draw an object on the map.
 * @param ti   Information about the tile to draw on.
 * @param spec Object spec to draw.
 */
void DrawNewObjectTile(TileInfo *ti, const ObjectSpec *spec)
{
	Object *o = Object::GetByTile(ti->tile);
	ObjectResolverObject object(spec, o, ti->tile);

	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return;

	auto processor = group->ProcessRegisters(object, nullptr);
	auto dts = processor.GetLayout();
	DrawTileLayout(ti, dts, spec);
}

/**
 * Draw representation of an object (tile) for GUI purposes.
 * @param x    Position x of image.
 * @param y    Position y of image.
 * @param spec Object spec to draw.
 * @param view The object's view.
 */
void DrawNewObjectTileInGUI(int x, int y, const ObjectSpec *spec, uint8_t view)
{
	ObjectResolverObject object(spec, nullptr, INVALID_TILE, view);
	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return;

	auto processor = group->ProcessRegisters(object, nullptr);
	auto dts = processor.GetLayout();

	PaletteID palette;
	if (Company::IsValidID(_local_company)) {
		/* Get the colours of our company! */
		if (spec->flags.Test(ObjectFlag::Uses2CC)) {
			const Livery &l = Company::Get(_local_company)->livery[0];
			palette = SPR_2CCMAP_BASE + l.colour1 + l.colour2 * 16;
		} else {
			palette = GetCompanyPalette(_local_company);
		}
	} else {
		/* There's no company, so just take the base palette. */
		palette = spec->flags.Test(ObjectFlag::Uses2CC) ? SPR_2CCMAP_BASE : PALETTE_RECOLOUR_START;
	}

	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		DrawSprite(image, GroundSpritePaletteTransform(image, pal, palette), x, y);
	}

	DrawNewGRFTileSeqInGUI(x, y, &dts, 0, palette);
}

/**
 * Perform a callback for an object.
 * @param callback The callback to perform.
 * @param param1   The first parameter to pass to the NewGRF.
 * @param param2   The second parameter to pass to the NewGRF.
 * @param spec     The specification of the object / the entry point.
 * @param o        The object to call the callback for.
 * @param tile     The tile the callback is called for.
 * @return The result of the callback.
 */

/** Helper class for animation control. */
const OpenTTDObjectAnimation &GetObjectAnimationBinding() noexcept
{
	static const auto binding = MakeRustAnimationBindingDirect<ObjectSpec, Object, int>(
		[](uint32_t t) noexcept -> OpenTTDObjectAnimationEntry { const auto *s = RustObjectSpecByTile(t); if (s == nullptr || !s->flags.Test(ObjectFlag::Animation)) return {nullptr, nullptr}; return {s, RustObjectByTile(t)}; },
		[](const ObjectSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(ObjectCallbackMask::AnimationSpeed), s->callback_mask.Test(ObjectCallbackMask::AnimationNextFrame), s->flags.Test(ObjectFlag::AnimRandomBits), true}; },
		openttd_rust_object_animation_callback);
	return binding;
}

/**
 * Handle the animation of the object tile.
 * @param tile The tile to animate.
 */
void AnimateNewObjectTile(TileIndex tile)
{
	openttd_rust_animate_object(tile.base());
}


/**
 * Trigger the update of animation on a single tile.
 * @param o       The object that got triggered.
 * @param tile    The location of the triggered tile.
 * @param trigger The trigger that is triggered.
 * @param spec    The spec associated with the object.
 */
bool TriggerObjectTileAnimation(Object *o, TileIndex tile, ObjectAnimationTrigger trigger, const ObjectSpec *spec)
{
	return openttd_rust_object_trigger_tile(&GetRustObjectServices(), o, tile.base(), trigger, spec);
}

/**
 * Trigger the update of animation on a whole object.
 * @param o       The object that got triggered.
 * @param trigger The trigger that is triggered.
 * @param spec    The spec associated with the object.
 */
bool TriggerObjectAnimation(Object *o, ObjectAnimationTrigger trigger, const ObjectSpec *spec)
{
	return openttd_rust_object_trigger(&GetRustObjectServices(), o, trigger, spec);
}

const OpenTTDObjectQueryServices &GetRustObjectQueryServices() noexcept
{
	static const OpenTTDObjectQueryServices services{
		.random_bits = [](uint32_t t) noexcept { return GetObjectRandomBits(TileIndex(t)); },
		.index = [](uint32_t t) noexcept { return GetObjectIndex(TileIndex(t)).base(); },
		.by_tile = RustObjectByTile,
		.definition = [](Object *o) noexcept -> OpenTTDObjectDefinition { const auto *s = ObjectSpec::Get(o->type); return {s->grf_prop.grfid, s->grf_prop.local_id, o->view, s->grf_prop.HasGrfFile()}; },
		.object_index = [](Object *o) noexcept { return o->index.base(); },
		.kind = [](Object *o) noexcept { return o->type; },
		.north = [](Object *o) noexcept { return o->location.tile.base(); },
		.town = [](Object *o) noexcept -> const Town * { return o->town; },
		.town_tile = OpenTTDTownTile,
		.date = [](Object *o) noexcept { return o->build_date.base(); },
		.colour = [](Object *o) noexcept { return o->colour; },
		.view = [](Object *o) noexcept { return o->view; },
		.nearby = RustSignedNearbyTile,
		.nearby_info = RustNearbyTileInformation,
		.terrain = RustTerrainType,
		.badges = RustSpecBadges<ObjectSpec>,
		.grfid = RustResolverGRFFileID,
		.version8 = RustResolverGRFVersion8,
		.override_id = [](uint16_t id, uint32_t grfid) noexcept { return _object_mngr.GetID(id, grfid); },
		.manhattan = OpenTTDDistanceManhattan,
		.square = OpenTTDDistanceSquare,
		.triggers = [](const ObjectSpec *s) noexcept { return s->animation.triggers.base(); },

		.unhandled = [](uint8_t variable) noexcept { Debug(grf, 1, "Unhandled object variable 0x{:X}", variable); },
	};
	return services;
}

const OpenTTDObjectResolverServices &GetObjectResolverServices() noexcept
{
	static_assert(GSF_OBJECTS == 0x0F && VSG_SCOPE_SELF == 0 && VSG_SCOPE_PARENT == 1 && CBID_OBJECT_LAND_SLOPE_CHECK == 0x157 && CBID_OBJECT_COLOUR == 0x15B);
	static const OpenTTDObjectResolverServices services{
		.file = RustSpecGRFFile<ObjectSpec>,
		.root = RustSpecSpriteGroup<ObjectSpec>,
		.local_id = RustSpecLocalID<ObjectSpec>,
		.version = RustGRFFileVersion,
		.views = RustObjectViews,
		.eligibility = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectEligibility { return {RustObjectSpecFlags(spec), spec->climate.base(), RustObjectViews(spec)}; },
		.introduction = [](const ObjectSpec *spec) noexcept -> int32_t { return spec->introduction_date.base(); },
		.lifetime = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectLifetime { return {spec->introduction_date.base(), spec->end_of_life_date.base()}; },
		.town_scope = RustResolverTownScope<ObjectResolverObject>,
		.base_scope = RustResolverBaseScope,
		.evaluate = [](const OpenTTDObjectScope *context, uint16_t callback, uint32_t param1, uint32_t param2, int32_t *regs, size_t count) noexcept -> uint16_t {
			ObjectResolverObject ro(context->spec, context->object, TileIndex{context->tile}, context->view, static_cast<CallbackID>(callback), param1, param2);
			return ro.ResolveCallback({regs, count});
		},
	};
	return services;
}

/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Engine fields address the canonical owner in the candidate and original fields in the oracle. */
#ifndef ENGINE_RECORD_FIXTURE_HPP
#define ENGINE_RECORD_FIXTURE_HPP
#include "engine_base.h"
#include "core/backup_type.hpp"
#include "newgrf_engine.h"
template <class E>
static decltype(auto) EngineFields(E *engine)
{
#ifdef WITH_RUST
	return (engine->Record());
#else
	return (*engine);
#endif
}

template <class E>
static decltype(auto) EngineBindings(E *engine)
{
#ifdef WITH_RUST
	return (engine->Record().bindings);
#else
	return (*engine);
#endif
}
#ifdef WITH_RUST
#include "rust/services_ffi.h"
extern "C" {
OpenTTDEngineBindings *openttd_test_engine_bindings_clone(const OpenTTDEngineBindings *);
void openttd_test_engine_bindings_drop(OpenTTDEngineBindings *);
void openttd_test_engine_sprites_clear(RustVector<OpenTTDEngineCargoBinding> *);
void openttd_test_engine_overrides_clear(RustVector<OpenTTDEngineWagonOverride> *);
void openttd_test_engine_badges_assign(RustVector<BadgeID> *, const BadgeID *, size_t);
void openttd_test_engine_sprite(OpenTTDEngineGRFProps *, CargoType, const SpriteGroup *);
void openttd_test_engine_override(const OpenTTDSharedServices *, OpenTTDEngineBindings *, CargoType, const SpriteGroup *, const EngineID *, size_t);
}
using FixtureEngineGRFProps = OpenTTDEngineGRFProps;
inline bool operator==(const OpenTTDEngineCargoBinding &a, const OpenTTDEngineCargoBinding &b) { return a.cargo == b.cargo && a.group == b.group; }
template <typename T, typename R> inline bool operator==(const RustVector<T> &a, const R &b) { return std::ranges::equal(a, b); }
/** Deep copies only fixture snapshots; shell and pool ownership never enter the snapshot. */
template <typename T> struct FixtureEngineBackup {
	T &original;
	OpenTTDEngineBindings *snapshot;
	bool valid = true;
	static constexpr size_t Offset()
	{
		if constexpr (std::is_same_v<T, OpenTTDEngineGRFProps>) return offsetof(OpenTTDEngineBindings, grf_prop);
		else if constexpr (std::is_same_v<T, RustVector<OpenTTDEngineCargoBinding>>) return offsetof(OpenTTDEngineBindings, grf_prop) + offsetof(OpenTTDEngineGRFProps, spritegroups);
		else if constexpr (std::is_same_v<T, RustVector<OpenTTDEngineWagonOverride>>) return offsetof(OpenTTDEngineBindings, overrides);
		else { static_assert(std::is_same_v<T, RustVector<BadgeID>>); return offsetof(OpenTTDEngineBindings, badges); }
	}
	static T &Value(OpenTTDEngineBindings *bindings) { return *reinterpret_cast<T *>(reinterpret_cast<char *>(bindings) + Offset()); }
	explicit FixtureEngineBackup(T &value) : original(value), snapshot(openttd_test_engine_bindings_clone(reinterpret_cast<OpenTTDEngineBindings *>(reinterpret_cast<char *>(&value) - Offset()))) {}
	FixtureEngineBackup(T &value, const std::vector<BadgeID> &replacement) : FixtureEngineBackup(value) { openttd_test_engine_badges_assign(&value, replacement.data(), replacement.size()); }
	FixtureEngineBackup(const FixtureEngineBackup &) = delete;
	~FixtureEngineBackup() { if (this->valid) this->Restore(); openttd_test_engine_bindings_drop(this->snapshot); }
	const T &GetOriginalValue() const { assert(this->valid); return Value(this->snapshot); }
	bool Verify() const { assert(this->valid); return Value(this->snapshot) == this->original; }
	void Revert() { assert(this->valid); auto *copy = openttd_test_engine_bindings_clone(this->snapshot); std::swap(this->original, Value(copy)); openttd_test_engine_bindings_drop(copy); }
	void Trash() { assert(this->valid); this->valid = false; }
	void Restore() { this->Revert(); this->Trash(); }
};
inline void FixtureClearEngineSprites(Engine *e) { openttd_test_engine_sprites_clear(&EngineBindings(e).grf_prop.spritegroups); }
inline void FixtureClearEngineOverrides(Engine *e) { openttd_test_engine_overrides_clear(&EngineBindings(e).overrides); }
inline void FixtureSetEngineSprite(Engine *e, CargoType cargo, const SpriteGroup *group) { openttd_test_engine_sprite(&EngineBindings(e).grf_prop, cargo, group); }
inline void FixtureSetEngineBadges(Engine *e, std::span<const BadgeID> badges) { openttd_test_engine_badges_assign(&EngineBindings(e).badges, badges.data(), badges.size()); }
inline void FixtureWagonOverride(EngineID engine, CargoType cargo, const SpriteGroup *group, std::span<EngineID> engines) { openttd_test_engine_override(&GetRustSharedServices(), &EngineBindings(Engine::Get(engine)), cargo, group, engines.data(), engines.size()); }
#else
using FixtureEngineGRFProps = CargoGRFFileProps;
template <typename T> using FixtureEngineBackup = Backup<T>;
inline void FixtureClearEngineSprites(Engine *e) { e->grf_prop.spritegroups.clear(); }
inline void FixtureClearEngineOverrides(Engine *e) { e->overrides.clear(); }
inline void FixtureSetEngineSprite(Engine *e, CargoType cargo, const SpriteGroup *group) { e->grf_prop.SetSpriteGroup(cargo, group); }
inline void FixtureSetEngineBadges(Engine *e, std::span<const BadgeID> badges) { e->badges.assign(badges.begin(), badges.end()); }
inline void FixtureWagonOverride(EngineID engine, CargoType cargo, const SpriteGroup *group, std::span<EngineID> engines) { SetWagonOverrideSprites(engine, cargo, group, engines); }
#endif
inline const SpriteGroup *FixtureEngineSprite(const Engine *engine, CargoType cargo)
{
#ifdef WITH_RUST
	const auto &groups = EngineBindings(engine).grf_prop.spritegroups;
	const auto it = std::ranges::lower_bound(groups, cargo, {}, &OpenTTDEngineCargoBinding::cargo);
	return it != groups.end() && it->cargo == cargo ? it->group : nullptr;
#else
	return engine->grf_prop.GetSpriteGroup(cargo);
#endif
}
#endif

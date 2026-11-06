/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** Engine fields address the canonical owner in the candidate and original fields in the oracle. */
#ifndef ENGINE_RECORD_FIXTURE_HPP
#define ENGINE_RECORD_FIXTURE_HPP
#include "engine_base.h"
template <class E>
static decltype(auto) EngineFields(E *engine)
{
#ifdef WITH_RUST
	return (engine->Record());
#else
	return (*engine);
#endif
}
#endif

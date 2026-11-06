/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tcp_content.cpp Basic functions to receive and send Content packets.
 */

#include "../../stdafx.h"
#include "../../textfile_gui.h"
#include "../../newgrf_config.h"
#include "../../base_media_base.h"
#include "../../base_media_graphics.h"
#include "../../base_media_music.h"
#include "../../base_media_sounds.h"
#include "../../ai/ai.hpp"
#include "../../game/game.hpp"
#include "../../fios.h"
#include "tcp_content.h"

#include "table/strings.h"

#include "../../safeguards.h"

/**
 * Search a textfile file next to this file in the content list.
 * @param type The type of the textfile to search for.
 * @return The filename for the textfile.
 */
std::optional<std::string> ContentInfo::GetTextfile(TextfileType type) const
{
	if (this->state == ContentInfo::State::Invalid) return std::nullopt;
	std::optional<std::string_view> tmp;
	switch (this->type) {
		default: NOT_REACHED();
		case CONTENT_TYPE_AI:
			tmp = AI::GetScannerInfo()->FindMainScript(*this, true);
			break;
		case CONTENT_TYPE_AI_LIBRARY:
			tmp = AI::GetScannerLibrary()->FindMainScript(*this, true);
			break;
		case CONTENT_TYPE_GAME:
			tmp = Game::GetScannerInfo()->FindMainScript(*this, true);
			break;
		case CONTENT_TYPE_GAME_LIBRARY:
			tmp = Game::GetScannerLibrary()->FindMainScript(*this, true);
			break;
		case CONTENT_TYPE_NEWGRF: {
			const GRFConfig *gc = FindGRFConfig(std::byteswap(this->unique_id), FGCM_EXACT, &this->md5sum);
			if (gc != nullptr) tmp = gc->filename;
			break;
		}
		case CONTENT_TYPE_BASE_GRAPHICS:
			tmp = TryGetBaseSetFile(*this, true, BaseGraphics::GetAvailableSets());
			break;
		case CONTENT_TYPE_BASE_SOUNDS:
			tmp = TryGetBaseSetFile(*this, true, BaseSounds::GetAvailableSets());
			break;
		case CONTENT_TYPE_BASE_MUSIC:
			tmp = TryGetBaseSetFile(*this, true, BaseMusic::GetAvailableSets());
			break;
		case CONTENT_TYPE_SCENARIO:
		case CONTENT_TYPE_HEIGHTMAP:
			tmp = FindScenario(*this, true);
			break;
	}
	if (!tmp.has_value()) return std::nullopt;
	return ::GetTextfile(type, GetContentInfoSubDir(this->type), *tmp);
}

/**
 * Helper to get the subdirectory a #ContentInfo is located in.
 * @param type The type of content.
 * @return The subdirectory the content is located in.
 */
Subdirectory GetContentInfoSubDir(ContentType type)
{
	switch (type) {
		default: return NO_DIRECTORY;
		case CONTENT_TYPE_AI:           return AI_DIR;
		case CONTENT_TYPE_AI_LIBRARY:   return AI_LIBRARY_DIR;
		case CONTENT_TYPE_GAME:         return GAME_DIR;
		case CONTENT_TYPE_GAME_LIBRARY: return GAME_LIBRARY_DIR;
		case CONTENT_TYPE_NEWGRF:       return NEWGRF_DIR;

		case CONTENT_TYPE_BASE_GRAPHICS:
		case CONTENT_TYPE_BASE_SOUNDS:
		case CONTENT_TYPE_BASE_MUSIC:
			return BASESET_DIR;

		case CONTENT_TYPE_SCENARIO:     return SCENARIO_DIR;
		case CONTENT_TYPE_HEIGHTMAP:    return HEIGHTMAP_DIR;
	}
}

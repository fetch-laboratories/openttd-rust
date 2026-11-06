/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file story_sl.cpp Code handling saving and loading of story pages */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/story_sl_compat.h"

#include "../story_base.h"

#include "../safeguards.h"

/** Called after load to trash broken pages. */
void AfterLoadStoryBook()
{
	if (IsSavegameVersionBefore(SLV_185)) {
		/* Trash all story pages and page elements because
		 * they were saved with wrong data types.
		 */
		_story_page_element_pool.CleanPool();
		_story_page_pool.CleanPool();
	}
}

static EncodedString _storypageelement_text;
struct StoryPageElementText {
	StoryPageElement *item;
	~StoryPageElementText() { this->item->Record().text = _storypageelement_text; }
};

static const SaveLoad _story_page_elements_desc[] = {
	SLE_CONDVARNAME(StoryPageElement, Record().sort_value, "sort_value",    SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION,   SLV_185),
	SLE_CONDVARNAME(StoryPageElement, Record().sort_value, "sort_value",    SLE_UINT32,                 SLV_185, SL_MAX_VERSION),
	    SLE_VARNAME(StoryPageElement, Record().page, "page",          SLE_UINT16),
	SLE_CONDVARNAME(StoryPageElement, Record().type, "type",          SLE_FILE_U16 | SLE_VAR_U8,  SL_MIN_VERSION,   SLV_185),
	SLE_CONDVARNAME(StoryPageElement, Record().type, "type",          SLE_UINT8,                  SLV_185, SL_MAX_VERSION),
	    SLE_VARNAME(StoryPageElement, Record().referenced_id, "referenced_id", SLE_UINT32),
	   SLEG_SSTR("text", _storypageelement_text,          SLE_STR | SLF_ALLOW_CONTROL),
};

struct STPEChunkHandler : ChunkHandler {
	STPEChunkHandler() : ChunkHandler('STPE', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_story_page_elements_desc);

		for (StoryPageElement *s : StoryPageElement::Iterate()) {
			SlSetArrayIndex(s->index);
			_storypageelement_text = s->Record().text;
			SlObject(s, _story_page_elements_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_story_page_elements_desc, _story_page_elements_sl_compat);

		int index;
		uint32_t max_sort_value = 0;
		while ((index = SlIterateArray()) != -1) {
			StoryPageElement *s = new (StoryPageElementID(index)) StoryPageElement();
			_storypageelement_text.clear();
			StoryPageElementText staged{s};
			SlObject(s, slt);
			if (s->Record().sort_value > max_sort_value) {
				max_sort_value = s->Record().sort_value;
			}
		}
		/* Update the next sort value, so that the next
		 * created page is shown after all existing pages.
		 */
		_story_page_element_next_sort_value = max_sort_value + 1;
	}
};

static EncodedString _storypage_title;
struct StoryPageText {
	StoryPage *item;
	~StoryPageText() { this->item->Record().title = _storypage_title; }
};

static const SaveLoad _story_pages_desc[] = {
	SLE_CONDVARNAME(StoryPage, Record().sort_value, "sort_value", SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION,   SLV_185),
	SLE_CONDVARNAME(StoryPage, Record().sort_value, "sort_value", SLE_UINT32,                 SLV_185, SL_MAX_VERSION),
	    SLE_VARNAME(StoryPage, Record().date, "date",       SLE_UINT32),
	SLE_CONDVARNAME(StoryPage, Record().company, "company",    SLE_FILE_U16 | SLE_VAR_U8,  SL_MIN_VERSION,   SLV_185),
	SLE_CONDVARNAME(StoryPage, Record().company, "company",    SLE_UINT8,                  SLV_185, SL_MAX_VERSION),
	   SLEG_SSTR("title", _storypage_title,      SLE_STR | SLF_ALLOW_CONTROL),
};

struct STPAChunkHandler : ChunkHandler {
	STPAChunkHandler() : ChunkHandler('STPA', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_story_pages_desc);

		for (StoryPage *s : StoryPage::Iterate()) {
			SlSetArrayIndex(s->index);
			_storypage_title = s->Record().title;
			SlObject(s, _story_pages_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_story_pages_desc, _story_pages_sl_compat);

		int index;
		uint32_t max_sort_value = 0;
		while ((index = SlIterateArray()) != -1) {
			StoryPage *s = new (StoryPageID(index)) StoryPage();
			_storypage_title.clear();
			StoryPageText staged{s};
			SlObject(s, slt);
			if (s->Record().sort_value > max_sort_value) {
				max_sort_value = s->Record().sort_value;
			}
		}
		/* Update the next sort value, so that the next
		 * created page is shown after all existing pages.
		 */
		_story_page_next_sort_value = max_sort_value + 1;
	}
};

static const STPEChunkHandler STPE;
static const STPAChunkHandler STPA;
static const ChunkHandlerRef story_page_chunk_handlers[] = {
	STPE,
	STPA,
};

extern const ChunkHandlerTable _story_page_chunk_handlers(story_page_chunk_handlers);

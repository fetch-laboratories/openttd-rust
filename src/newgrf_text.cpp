/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file newgrf_text.cpp
 * Implementation of  Action 04 "universal holder" structure and functions.
 * This file implements a linked-lists of strings,
 * holding everything that the newgrf action 04 will send over to OpenTTD.
 * One of the biggest problems is that Dynamic lang Array uses ISO codes
 * as way to identifying current user lang, while newgrf uses bit shift codes
 * not related to ISO.  So equivalence functionality had to be set.
 */

#include "stdafx.h"

#include "debug.h"
#include "rust/newgrf_text_ffi.h"
#include "rust/services_ffi.h"
#include "newgrf.h"
#include "strings_internal.h"
#include "newgrf_storage.h"
#include "newgrf_text.h"
#include "newgrf/newgrf_stringmapping.h"
#include "newgrf_cargo.h"
#include "string_func.h"
#include "timer/timer_game_calendar.h"
#include "debug.h"
#include "language.h"
#include <ranges>

#include "table/strings.h"
#include "table/control_codes.h"

#include "safeguards.h"

static_assert(SCC_BIGFONT == 0xE006 && SCC_BLACK == 0xE052 && SCC_BLUE == 0xE042 && SCC_BROWN == 0xE04D && SCC_BUS == 0xE2B6);
static_assert(SCC_CHECKMARK == 0xE2AC && SCC_CITY == 0xE29C && SCC_CREAM == 0xE04C && SCC_CROSS == 0xE2AD && SCC_DKBLUE == 0xE051);
static_assert(SCC_DKGREEN == 0xE04B && SCC_DOWN_ARROW == 0xE2AA && SCC_GENDER_INDEX == 0xE03E && SCC_GENDER_LIST == 0xE03D && SCC_GOLD == 0xE044);
static_assert(SCC_GRAY == 0xE050 && SCC_GREEN == 0xE049 && SCC_LORRY == 0xE2B5 && SCC_LTBLUE == 0xE04F && SCC_LTBROWN == 0xE047);
static_assert(SCC_NEWGRF_DISCARD_WORD == 0xE072 && SCC_NEWGRF_PRINT_BYTE_HEX == 0xE063 && SCC_NEWGRF_PRINT_DWORD_DATE_LONG == 0xE067 && SCC_NEWGRF_PRINT_DWORD_FORCE == 0xE070 && SCC_NEWGRF_PRINT_DWORD_HEX == 0xE065);
static_assert(SCC_NEWGRF_PRINT_DWORD_SIGNED == 0xE056 && SCC_NEWGRF_PRINT_QWORD_CURRENCY == 0xE062 && SCC_NEWGRF_PRINT_QWORD_HEX == 0xE066 && SCC_NEWGRF_PRINT_WORD_DATE_LONG == 0xE05C && SCC_NEWGRF_PRINT_WORD_HEX == 0xE064);
static_assert(SCC_NEWGRF_PRINT_WORD_STATION_NAME == 0xE061 && SCC_NEWGRF_PRINT_WORD_STRING_ID == 0xE05B && SCC_NEWGRF_PRINT_WORD_VOLUME_LONG == 0xE05F && SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG == 0xE060 && SCC_NEWGRF_PUSH_WORD == 0xE071);
static_assert(SCC_NEWGRF_ROTATE_TOP_4_WORDS == 0xE073 && SCC_NEWGRF_STRINL == 0xE074 && SCC_ORANGE == 0xE048 && SCC_PLANE == 0xE2B7 && SCC_PLURAL_LIST == 0xE03C);
static_assert(SCC_POP_COLOUR == 0xE055 && SCC_PURPLE == 0xE046 && SCC_PUSH_COLOUR == 0xE054 && SCC_RED == 0xE045 && SCC_RIGHT_ARROW == 0xE2AF);
static_assert(SCC_SET_CASE == 0xE040 && SCC_SHIP == 0xE2B8 && SCC_SILVER == 0xE043 && SCC_SMALL_DOWN_ARROW == 0xE2BD && SCC_SMALL_UP_ARROW == 0xE2BC);
static_assert(SCC_SUPERSCRIPT_M1 == 0xE2B9 && SCC_SWITCH_CASE == 0xE041 && SCC_TINYFONT == 0xE005 && SCC_TOWN == 0xE29B && SCC_TRAIN == 0xE2B4);
static_assert(SCC_UP_ARROW == 0xE2A0 && SCC_WHITE == 0xE04E && SCC_YELLOW == 0xE04A);

const OpenTTDGrfTextServices &GetRustGrfTextServices() noexcept
{
	static const OpenTTDGrfTextServices services{
		.empty = STR_EMPTY,
		.language_map = [](uint32_t grfid, uint8_t language) noexcept { return LanguageMap::GetLanguageMap(grfid, language); },
		.legacy_string = [](uint32_t stringid) noexcept { return TTDPStringIDToOTTDStringIDMapping(GRFStringID{stringid}); },
		.mapping = [](const LanguageMap *lm, int32_t id, bool gender) noexcept { return lm->GetMapping(id, gender); },
		.max_plural_forms = LANGUAGE_MAX_PLURAL_FORMS,
		.num_cases = []() noexcept { return _current_language->num_cases; },
		.num_genders = []() noexcept { return _current_language->num_genders; },
		.plural_form = [](const LanguageMap *lm) noexcept { return lm->plural_form; },
		.reverse_mapping = [](const LanguageMap *lm, int32_t id, bool gender) noexcept { return lm->GetReverseMapping(id, gender); },
		.undefined = STR_UNDEFINED,
	};
	return services;
}

static_assert(TEXT_TAB_NEWGRF_START == 64 && TAB_SIZE_BITS == 11 && TAB_SIZE_NEWGRF == 524288);

/**
 * Get the mapping from the NewGRF supplied ID to OpenTTD's internal ID.
 * @param newgrf_id The NewGRF ID to map.
 * @param gender    Whether to map genders or cases.
 * @return The, to OpenTTD's internal ID, mapped index, or -1 if there is no mapping.
 */
int LanguageMap::GetMapping(int newgrf_id, bool gender) const
{
	const std::vector<Mapping> &map = gender ? this->gender_map : this->case_map;
	for (const Mapping &m : map) {
		if (m.newgrf_id == newgrf_id) return m.openttd_id;
	}
	return -1;
}

/**
 * Get the mapping from OpenTTD's internal ID to the NewGRF supplied ID.
 * @param openttd_id The OpenTTD ID to map.
 * @param gender     Whether to map genders or cases.
 * @return The, to the NewGRF supplied ID, mapped index, or -1 if there is no mapping.
 */
int LanguageMap::GetReverseMapping(int openttd_id, bool gender) const
{
	const std::vector<Mapping> &map = gender ? this->gender_map : this->case_map;
	for (const Mapping &m : map) {
		if (m.openttd_id == openttd_id) return m.newgrf_id;
	}
	return -1;
}

std::string TranslateTTDPatchCodes(uint32_t grfid, uint8_t language_id, bool allow_newlines, std::string_view str, StringControlCode byte80)
{
	std::string result;
	openttd_rust_grf_text_translate(&GetRustGrfTextServices(), grfid, language_id, allow_newlines, reinterpret_cast<const uint8_t *>(str.data()), str.size(), byte80, &result, [](void *output, const uint8_t *data, size_t size) noexcept { static_cast<std::string *>(output)->assign(reinterpret_cast<const char *>(data), size); });
	return result;
}

/**
 * Add a new text to a GRFText list.
 * @param list The list where the text should be added to.
 * @param langid The The language of the new text.
 * @param text_to_add The text to add to the list.
 */
static void AddGRFTextToList(GRFTextList &list, uint8_t langid, std::string_view text_to_add)
{
	/* Loop through all languages and see if we can replace a string */
	for (auto &text : list) {
		if (text.langid == langid) {
			text.text = text_to_add;
			return;
		}
	}

	/* If a string wasn't replaced, then we must append the new string */
	list.emplace_back(langid, std::string{text_to_add});
}

/**
 * Add a string to a GRFText list.
 * @param list The list where the text should be added to.
 * @param langid The language of the new text.
 * @param grfid The grfid where this string is defined.
 * @param allow_newlines Whether newlines are allowed in this string.
 * @param text_to_add The text to add to the list.
 * @note All text-codes will be translated.
 */
void AddGRFTextToList(GRFTextList &list, uint8_t langid, uint32_t grfid, bool allow_newlines, std::string_view text_to_add)
{
	AddGRFTextToList(list, langid, TranslateTTDPatchCodes(grfid, langid, allow_newlines, text_to_add));
}

/**
 * Add a string to a GRFText list.
 * @param list The list where the text should be added to.
 * @param langid The language of the new text.
 * @param grfid The grfid where this string is defined.
 * @param allow_newlines Whether newlines are allowed in this string.
 * @param text_to_add The text to add to the list.
 * @note All text-codes will be translated.
 */
void AddGRFTextToList(GRFTextWrapper &list, uint8_t langid, uint32_t grfid, bool allow_newlines, std::string_view text_to_add)
{
	if (list == nullptr) list = std::make_shared<GRFTextList>();
	AddGRFTextToList(*list, langid, grfid, allow_newlines, text_to_add);
}

/**
 * Add a GRFText to a GRFText list. The text should  not contain any text-codes.
 * The text will be added as a 'default language'-text.
 * @param list The list where the text should be added to.
 * @param text_to_add The text to add to the list.
 */
void AddGRFTextToList(GRFTextWrapper &list, std::string_view text_to_add)
{
	if (list == nullptr) list = std::make_shared<GRFTextList>();
	AddGRFTextToList(*list, 0x7F, text_to_add);
}

/**
 * Add the new read string into our structure.
 */
StringID AddGRFString(uint32_t grfid, GRFStringID stringid, uint8_t langid_to_add, bool new_scheme, bool allow_newlines, std::string_view text_to_add, StringID def_string)
{
	return openttd_rust_grf_text_add(&GetRustGrfTextServices(), grfid, stringid.base(), langid_to_add, new_scheme, allow_newlines, reinterpret_cast<const uint8_t *>(text_to_add.data()), text_to_add.size(), def_string);
}

/**
 * Returns the index for this stringid associated with its grfID
 */
StringID GetGRFStringID(uint32_t grfid, GRFStringID stringid)
{
	return openttd_rust_grf_text_id(grfid, stringid.base(), STR_UNDEFINED);
}


/**
 * Get a C-string from a GRFText-list. If there is a translation for the
 * current language it is returned, otherwise the default translation
 * is returned. If there is neither a default nor a translation for the
 * current language nullptr is returned.
 * @param text_list The GRFTextList to get the string from.
 */
std::optional<std::string_view> GetGRFStringFromGRFText(const GRFTextList &text_list)
{
	static_assert(std::is_standard_layout_v<GRFText> && offsetof(GRFText, langid) == 0);
	size_t index = openttd_rust_grf_text_select(text_list.data(), text_list.size(), sizeof(GRFText));
	return index == SIZE_MAX ? std::nullopt : std::optional<std::string_view>{text_list[index].text};
}

/**
 * Get a C-string from a GRFText-list. If there is a translation for the
 * current language it is returned, otherwise the default translation
 * is returned. If there is neither a default nor a translation for the
 * current language nullptr is returned.
 * @param text The GRFTextList to get the string from.
 */
std::optional<std::string_view> GetGRFStringFromGRFText(const GRFTextWrapper &text)
{
	return text ? GetGRFStringFromGRFText(*text) : std::nullopt;
}

/**
 * Get a C-string from a stringid set by a newgrf.
 */
std::string_view GetGRFStringPtr(StringIndexInTab stringid)
{
	StringID fallback;
	auto text = openttd_rust_grf_text_get(&GetRustSharedServices(), stringid.base(), &fallback);
	return text.data != nullptr ? std::string_view{reinterpret_cast<const char *>(text.data), text.size} : GetStringPtr(fallback);
}

/**
 * Equivalence Setter function between game and newgrf langID.
 * This function will adjust _currentLangID as to what is the LangID
 * of the current language set by the user.
 * This function is called after the user changed language,
 * from strings.cpp:ReadLanguagePack
 * @param language_id iso code of current selection
 */
void SetCurrentGrfLangID(uint8_t language_id)
{
	openttd_rust_grf_text_set_language(language_id);
}

bool CheckGrfLangID(uint8_t lang_id, uint8_t grf_version)
{
	return openttd_rust_grf_text_check_language(lang_id, grf_version);
}

struct TextRefStack {
private:
	std::vector<uint8_t> stack;
public:
	const GRFFile *grffile = nullptr;

	TextRefStack(const GRFFile *grffile, std::span<const int32_t> textstack) : grffile(grffile)
	{
		this->stack.reserve(textstack.size() * 4 + 6); // for translations it is reasonable to push 3 word and rotate them
		for (int32_t value : std::ranges::reverse_view(textstack)) {
			this->PushDWord(value);
		}
	}

	uint8_t PopUnsignedByte()
	{
		if (this->stack.empty()) return 0;
		auto res = this->stack.back();
		this->stack.pop_back();
		return res;
	}
	int8_t   PopSignedByte()    { return (int8_t)this->PopUnsignedByte(); }

	uint16_t PopUnsignedWord()
	{
		uint16_t val = this->PopUnsignedByte();
		return val | (this->PopUnsignedByte() << 8);
	}
	int16_t  PopSignedWord()    { return (int32_t)this->PopUnsignedWord(); }

	uint32_t PopUnsignedDWord()
	{
		uint32_t val = this->PopUnsignedWord();
		return val | (this->PopUnsignedWord() << 16);
	}
	int32_t  PopSignedDWord()   { return (int32_t)this->PopUnsignedDWord(); }

	uint64_t PopUnsignedQWord()
	{
		uint64_t val = this->PopUnsignedDWord();
		return val | (((uint64_t)this->PopUnsignedDWord()) << 32);
	}
	int64_t  PopSignedQWord()   { return (int64_t)this->PopUnsignedQWord(); }

	/** Rotate the top four words down: W1, W2, W3, W4 -> W4, W1, W2, W3 */
	void RotateTop4Words()
	{
		auto w1 = this->PopUnsignedWord();
		auto w2 = this->PopUnsignedWord();
		auto w3 = this->PopUnsignedWord();
		auto w4 = this->PopUnsignedWord();
		this->PushWord(w3);
		this->PushWord(w2);
		this->PushWord(w1);
		this->PushWord(w4);
	}

	void PushByte(uint8_t b)
	{
		this->stack.push_back(b);
	}

	void PushWord(uint16_t w)
	{
		this->PushByte(GB(w, 8, 8));
		this->PushByte(GB(w, 0, 8));
	}

	void PushDWord(uint32_t dw)
	{
		this->PushWord(GB(dw, 16, 16));
		this->PushWord(GB(dw, 0, 16));
	}
};

static void HandleNewGRFStringControlCodes(std::string_view str, TextRefStack &stack, std::vector<StringParameter> &params);

/**
 * Process NewGRF string control code instructions.
 * @param scc The string control code that has been read.
 * @param consumer The string that we are reading from.
 * @param stack The TextRefStack.
 * @param[out] params Output parameters
 */
static void ProcessNewGRFStringControlCode(char32_t scc, StringConsumer &consumer, TextRefStack &stack, std::vector<StringParameter> &params)
{
	/* There is data on the NewGRF text stack, and we want to move them to OpenTTD's string stack.
	 * After this call, a new call is made with `modify_parameters` set to false when the string is finally formatted. */
	switch (scc) {
		default: return;

		case SCC_PLURAL_LIST:
			consumer.SkipUint8(); // plural form
			[[fallthrough]];
		case SCC_GENDER_LIST: {
			consumer.SkipUint8(); // offset
			/* plural and gender choices cannot contain any string commands, so just skip the whole thing */
			uint num = consumer.ReadUint8();
			uint total_len = 0;
			for (uint i = 0; i != num; i++) {
				total_len += consumer.ReadUint8();
			}
			consumer.Skip(total_len);
			break;
		}

		case SCC_SWITCH_CASE: {
			/* skip all cases and continue with default case */
			uint num = consumer.ReadUint8();
			for (uint i = 0; i != num; i++) {
				consumer.SkipUint8();
				auto len = consumer.ReadUint16LE();
				consumer.Skip(len);
			}
			consumer.SkipUint16LE(); // length of default
			break;
		}

		case SCC_GENDER_INDEX:
		case SCC_SET_CASE:
			consumer.SkipUint8();
			break;

		case SCC_ARG_INDEX:
			NOT_REACHED();
			break;

		case SCC_NEWGRF_PRINT_BYTE_SIGNED:      params.emplace_back(stack.PopSignedByte());    break;
		case SCC_NEWGRF_PRINT_QWORD_CURRENCY:   params.emplace_back(stack.PopSignedQWord());   break;

		case SCC_NEWGRF_PRINT_DWORD_CURRENCY:
		case SCC_NEWGRF_PRINT_DWORD_SIGNED:     params.emplace_back(stack.PopSignedDWord());   break;

		case SCC_NEWGRF_PRINT_BYTE_HEX:         params.emplace_back(stack.PopUnsignedByte());  break;
		case SCC_NEWGRF_PRINT_QWORD_HEX:        params.emplace_back(stack.PopUnsignedQWord()); break;

		case SCC_NEWGRF_PRINT_WORD_SPEED:
		case SCC_NEWGRF_PRINT_WORD_VOLUME_LONG:
		case SCC_NEWGRF_PRINT_WORD_VOLUME_SHORT:
		case SCC_NEWGRF_PRINT_WORD_SIGNED:      params.emplace_back(stack.PopSignedWord());    break;

		case SCC_NEWGRF_PRINT_WORD_HEX:
		case SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG:
		case SCC_NEWGRF_PRINT_WORD_WEIGHT_SHORT:
		case SCC_NEWGRF_PRINT_WORD_POWER:
		case SCC_NEWGRF_PRINT_WORD_STATION_NAME:
		case SCC_NEWGRF_PRINT_WORD_UNSIGNED:    params.emplace_back(stack.PopUnsignedWord());  break;

		case SCC_NEWGRF_PRINT_DWORD_FORCE:
		case SCC_NEWGRF_PRINT_DWORD_DATE_LONG:
		case SCC_NEWGRF_PRINT_DWORD_DATE_SHORT:
		case SCC_NEWGRF_PRINT_DWORD_HEX:        params.emplace_back(stack.PopUnsignedDWord()); break;

		/* Dates from NewGRFs have 1920-01-01 as their zero point, convert it to OpenTTD's epoch. */
		case SCC_NEWGRF_PRINT_WORD_DATE_LONG:
		case SCC_NEWGRF_PRINT_WORD_DATE_SHORT:  params.emplace_back(CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR + stack.PopUnsignedWord()); break;

		case SCC_NEWGRF_DISCARD_WORD:           stack.PopUnsignedWord(); break;

		case SCC_NEWGRF_ROTATE_TOP_4_WORDS:     stack.RotateTop4Words(); break;
		case SCC_NEWGRF_PUSH_WORD:              stack.PushWord(consumer.ReadUtf8(0)); break;

		case SCC_NEWGRF_PRINT_WORD_CARGO_LONG:
		case SCC_NEWGRF_PRINT_WORD_CARGO_SHORT:
		case SCC_NEWGRF_PRINT_WORD_CARGO_TINY:
			params.emplace_back(GetCargoTranslation(stack.PopUnsignedWord(), stack.grffile));
			params.emplace_back(stack.PopUnsignedWord());
			break;

		case SCC_NEWGRF_STRINL: {
			StringID stringid = consumer.ReadUtf8(STR_NULL);
			/* We also need to handle the substring's stack usage. */
			HandleNewGRFStringControlCodes(GetStringPtr(stringid), stack, params);
			break;
		}

		case SCC_NEWGRF_PRINT_WORD_STRING_ID: {
			StringID stringid = MapGRFStringID(stack.grffile->grfid, GRFStringID{stack.PopUnsignedWord()});
			params.emplace_back(stringid);
			/* We also need to handle the substring's stack usage. */
			HandleNewGRFStringControlCodes(GetStringPtr(stringid), stack, params);
			break;
		}

		case SCC_NEWGRF_PRINT_WORD_CARGO_NAME: {
			CargoType cargo = GetCargoTranslation(stack.PopUnsignedWord(), stack.grffile);
			params.emplace_back(cargo < NUM_CARGO ? 1ULL << cargo : 0);
			break;
		}
	}
}

/**
 * Emit OpenTTD's internal string code for the different NewGRF string codes.
 * @param scc NewGRF string code.
 * @param consumer String consumer, moved forward if SCC_NEWGRF_PUSH_WORD is found.
 * @returns String code to use.
 */
char32_t RemapNewGRFStringControlCode(char32_t scc, StringConsumer &consumer)
{
	switch (scc) {
		default:
			return scc;

		case SCC_NEWGRF_PRINT_DWORD_SIGNED:
		case SCC_NEWGRF_PRINT_WORD_SIGNED:
		case SCC_NEWGRF_PRINT_BYTE_SIGNED:
		case SCC_NEWGRF_PRINT_WORD_UNSIGNED:
			return SCC_COMMA;

		case SCC_NEWGRF_PRINT_BYTE_HEX:
		case SCC_NEWGRF_PRINT_WORD_HEX:
		case SCC_NEWGRF_PRINT_DWORD_HEX:
		case SCC_NEWGRF_PRINT_QWORD_HEX:
			return SCC_HEX;

		case SCC_NEWGRF_PRINT_DWORD_CURRENCY:
		case SCC_NEWGRF_PRINT_QWORD_CURRENCY:
			return SCC_CURRENCY_LONG;

		case SCC_NEWGRF_PRINT_WORD_DATE_LONG:
		case SCC_NEWGRF_PRINT_DWORD_DATE_LONG:
			return SCC_DATE_LONG;

		case SCC_NEWGRF_PRINT_WORD_DATE_SHORT:
		case SCC_NEWGRF_PRINT_DWORD_DATE_SHORT:
			return SCC_DATE_SHORT;

		case SCC_NEWGRF_PRINT_WORD_SPEED:
			return SCC_VELOCITY;

		case SCC_NEWGRF_PRINT_WORD_VOLUME_LONG:
			return SCC_VOLUME_LONG;

		case SCC_NEWGRF_PRINT_WORD_VOLUME_SHORT:
			return SCC_VOLUME_SHORT;

		case SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG:
			return SCC_WEIGHT_LONG;

		case SCC_NEWGRF_PRINT_WORD_WEIGHT_SHORT:
			return SCC_WEIGHT_SHORT;

		case SCC_NEWGRF_PRINT_WORD_POWER:
			return SCC_POWER;

		case SCC_NEWGRF_PRINT_DWORD_FORCE:
			return SCC_FORCE;

		case SCC_NEWGRF_PRINT_WORD_CARGO_LONG:
			return SCC_CARGO_LONG;

		case SCC_NEWGRF_PRINT_WORD_CARGO_SHORT:
			return SCC_CARGO_SHORT;

		case SCC_NEWGRF_PRINT_WORD_CARGO_TINY:
			return SCC_CARGO_TINY;

		case SCC_NEWGRF_PRINT_WORD_CARGO_NAME:
			return SCC_CARGO_LIST;

		case SCC_NEWGRF_PRINT_WORD_STATION_NAME:
			return SCC_STATION_NAME;

		/* These NewGRF string codes modify the NewGRF stack or otherwise do not map to OpenTTD string codes. */
		case SCC_NEWGRF_PUSH_WORD:
			consumer.SkipUtf8();
			return 0;

		case SCC_NEWGRF_DISCARD_WORD:
		case SCC_NEWGRF_ROTATE_TOP_4_WORDS:
			return 0;
	}
}

/**
 * Handle control codes in a NewGRF string, processing the stack and filling parameters.
 * @param str String to process.
 * @param[in,out] stack Stack to use.
 * @param[out] params Parameters to fill.
 */
static void HandleNewGRFStringControlCodes(std::string_view str, TextRefStack &stack, std::vector<StringParameter> &params)
{
	StringConsumer consumer(str);
	while (consumer.AnyBytesLeft()) {
		char32_t scc = consumer.ReadUtf8();
		ProcessNewGRFStringControlCode(scc, consumer, stack, params);
	}
}

/**
 * Process the text ref stack for a GRF String and return its parameters.
 * @param grffile GRFFile of string.
 * @param stringid StringID of string.
 * @param textstack Text parameter stack.
 * @returns Parameters for GRF string.
 */
std::vector<StringParameter> GetGRFStringTextStackParameters(const GRFFile *grffile, StringID stringid, std::span<const int32_t> textstack)
{
	if (stringid == INVALID_STRING_ID) return {};

	auto str = GetStringPtr(stringid);

	std::vector<StringParameter> params;
	params.reserve(20);

	TextRefStack stack{grffile, textstack};
	HandleNewGRFStringControlCodes(str, stack, params);

	return params;
}

/**
 * Format a GRF string using the text ref stack for parameters.
 * @param grffile GRFFile of string.
 * @param grfstringid GRFStringID of string.
 * @param textstack Text parameter stack.
 * @returns Formatted string.
 */
std::string GetGRFStringWithTextStack(const struct GRFFile *grffile, GRFStringID grfstringid, std::span<const int32_t> textstack)
{
	StringID stringid = GetGRFStringID(grffile->grfid, grfstringid);
	auto params = GetGRFStringTextStackParameters(grffile, stringid, textstack);
	return GetStringWithArgs(stringid, params);
}

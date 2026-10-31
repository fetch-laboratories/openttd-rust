/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file openttd.cpp Functions related to starting OpenTTD. */

#include "stdafx.h"

#include "blitter/factory.hpp"
#include "sound/sound_driver.hpp"
#include "music/music_driver.hpp"
#include "video/video_driver.hpp"
#include "mixer.h"

#include "fontcache.h"
#include "error.h"
#include "error_func.h"
#include "gui.h"

#include "base_media_base.h"
#include "base_media_graphics.h"
#include "base_media_music.h"
#include "base_media_sounds.h"
#include "saveload/saveload.h"
#include "company_cmd.h"
#include "company_func.h"
#include "company_gui.h"
#include "command_func.h"
#include "news_func.h"
#include "fios.h"
#include "aircraft.h"
#include "roadveh.h"
#include "train.h"
#include "ship.h"
#include "console_func.h"
#include "screenshot.h"
#include "network/network.h"
#include "network/network_func.h"
#include "ai/ai.hpp"
#include "ai/ai_config.hpp"
#include "settings_func.h"
#include "genworld.h"
#include "progress.h"
#include "strings_func.h"
#include "vehicle_func.h"
#include "gamelog.h"
#include "animated_tile_func.h"
#include "roadstop_base.h"
#include "elrail_func.h"
#include "rev.h"
#include "highscore.h"
#include "station_base.h"
#include "crashlog.h"
#include "engine_func.h"
#include "core/random_func.hpp"
#include "rail_gui.h"
#include "road_gui.h"
#include "core/backup_type.hpp"
#include "hotkeys.h"
#include "newgrf.h"
#include "misc/getoptdata.h"
#include "game/game.hpp"
#include "game/game_config.hpp"
#include "town.h"
#include "subsidy_func.h"
#include "gfx_layout.h"
#include "viewport_func.h"
#include "viewport_sprite_sorter.h"
#include "framerate_type.h"
#include "industry.h"
#include "network/network_gui.h"
#include "network/network_survey.h"
#include "misc_cmd.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_realtime.h"
#include "timer/timer_game_tick.h"
#include "social_integration.h"
#include "core/string_consumer.hpp"

#include "linkgraph/linkgraphschedule.h"

#include <system_error>

#include "table/strings.h"

#ifdef __EMSCRIPTEN__
#	include <emscripten.h>
#	include <emscripten/html5.h>
#endif

#include "safeguards.h"

void DoPaletteAnimations();
void MusicLoop();
void CallWindowGameTickEvent();
bool HandleBootstrap();

extern void CheckCaches();
extern Company *DoStartupNewCompany(bool is_ai, CompanyID company = CompanyID::Invalid());
extern void OSOpenBrowser(const std::string &url);
extern void ShowOSErrorBox(std::string_view buf, bool system);
extern std::string _config_file;

bool _save_config = false;
#include "rust/session_native.hpp"

/**
 * Error handling for fatal user errors.
 * @param str the string to print.
 * @note Does NEVER return.
 */
void UserErrorI(const std::string &str)
{
	ShowOSErrorBox(str, false);
	if (VideoDriver::GetInstance() != nullptr) VideoDriver::GetInstance()->Stop();

#ifdef __EMSCRIPTEN__
	emscripten_exit_pointerlock();
	/* In effect, the game ends here. As emscripten_set_main_loop() caused
	 * the stack to be unwound, the code after MainLoop() in
	 * openttd_main() is never executed. */
	EM_ASM(if (window["openttd_abort"]) openttd_abort());
#endif

	_exit(1);
}

/**
 * Error handling for fatal non-user errors.
 * @param str the string to print.
 * @note Does NEVER return.
 */
void FatalErrorI(const std::string &str)
{
	if (VideoDriver::GetInstance() == nullptr || VideoDriver::GetInstance()->HasGUI()) {
		ShowOSErrorBox(str, true);
	}

	/* Set the error message for the crash log and then invoke it. */
	CrashLog::SetErrorMessage(str);
	abort();
}

/**
 * Show the help message when someone passed a wrong parameter.
 */
static void ShowHelp()
{
	std::string str;
	str.reserve(8192);

	std::back_insert_iterator<std::string> output_iterator = std::back_inserter(str);
	fmt::format_to(output_iterator, "OpenTTD {}\n", _openttd_revision);
	str +=
		"\n"
		"\n"
		"Command line options:\n"
		"  -v drv              = Set video driver (see below)\n"
		"  -s drv              = Set sound driver (see below)\n"
		"  -m drv              = Set music driver (see below)\n"
		"  -b drv              = Set the blitter to use (see below)\n"
		"  -r res              = Set resolution (for instance 800x600)\n"
		"  -h                  = Display this help text\n"
		"  -t year             = Set starting year\n"
		"  -d [[fac=]lvl[,...]]= Debug mode\n"
		"  -e                  = Start Editor\n"
		"  -g [savegame|scenario|heightmap] = Start new/savegame/scenario/heightmap immediately\n"
		"  -G seed             = Set random seed\n"
		"  -n host[:port][#company]= Join network game\n"
		"  -p password         = Password to join server\n"
		"  -D [host][:port]    = Start dedicated server\n"
#if !defined(_WIN32)
		"  -f                  = Fork into the background (dedicated only)\n"
#endif
		"  -I graphics_set     = Force the graphics set (see below)\n"
		"  -S sounds_set       = Force the sounds set (see below)\n"
		"  -M music_set        = Force the music set (see below)\n"
		"  -c config_file      = Use 'config_file' instead of 'openttd.cfg'\n"
		"  -x                  = Never save configuration changes to disk\n"
		"  -X                  = Don't use global folders to search for files\n"
		"  -q savegame         = Write some information about the savegame and exit\n"
		"  -Q                  = Don't scan for/load NewGRF files on startup\n"
		"  -QQ                 = Disable NewGRF scanning/loading entirely\n"
		"\n";

	/* List the graphics packs */
	BaseGraphics::GetSetsList(output_iterator);

	/* List the sounds packs */
	BaseSounds::GetSetsList(output_iterator);

	/* List the music packs */
	BaseMusic::GetSetsList(output_iterator);

	/* List the drivers */
	DriverFactoryBase::GetDriversInfo(output_iterator);

	/* List the blitters */
	BlitterFactory::GetBlittersInfo(output_iterator);

	/* List the debug facilities. */
	DumpDebugFacilityNames(output_iterator);

	/* We need to initialize the AI, so it finds the AIs */
	AI::Initialize();
	AI::GetConsoleList(output_iterator, true);
	AI::Uninitialize(true);

	/* We need to initialize the GameScript, so it finds the GSs */
	Game::Initialize();
	Game::GetConsoleList(output_iterator, true);
	Game::Uninitialize(true);

	/* ShowInfo put output to stderr, but version information should go
	 * to stdout; this is the only exception */
#if !defined(_WIN32)
	fmt::print("{}\n", str);
#else
	ShowInfoI(str);
#endif
}

static void WriteSavegameInfo(const std::string &name)
{
	extern SaveLoadVersion _sl_version;
	uint32_t last_ottd_rev = 0;
	uint8_t ever_modified = 0;
	bool removed_newgrfs = false;

	_gamelog.Info(&last_ottd_rev, &ever_modified, &removed_newgrfs);

	std::string message;
	message.reserve(1024);
	format_append(message, "Name:         {}\n", name);
	format_append(message, "Savegame ver: {}\n", _sl_version);
	format_append(message, "NewGRF ver:   0x{:08X}\n", last_ottd_rev);
	format_append(message, "Modified:     {}\n", ever_modified);

	if (removed_newgrfs) {
		format_append(message, "NewGRFs have been removed\n");
	}

	message += "NewGRFs:\n";
	if (_load_check_data.HasNewGrfs()) {
		for (const auto &c : _load_check_data.grfconfig) {
			format_append(message, "{:08X} {} {}\n", std::byteswap(c->ident.grfid),
				FormatArrayAsHex(c->flags.Test(GRFConfigFlag::Compatible) ? c->original_md5sum : c->ident.md5sum), c->filename);
		}
	}

	/* ShowInfo put output to stderr, but version information should go
	 * to stdout; this is the only exception */
#if !defined(_WIN32)
	fmt::print("{}\n", message);
#else
	ShowInfoI(message);
#endif
}


/**
 * Extract the resolution from the given string and store
 * it in the 'res' parameter.
 * @param res variable to store the resolution in.
 * @param s   the string to decompose.
 */
static void ParseResolution(Dimension &res, std::string_view s)
{
	StringConsumer consumer(s);
	auto width = consumer.TryReadIntegerBase<uint>(10);
	auto valid = consumer.ReadIf("x");
	auto height = consumer.TryReadIntegerBase<uint>(10);
	if (!width.has_value() || !valid || !height.has_value() || consumer.AnyBytesLeft()) {
		ShowInfo("Invalid resolution '{}'", s);
		return;
	}

	res.width  = std::max<uint>(*width, 64);
	res.height = std::max<uint>(*height, 64);
}


/**
 * Uninitializes drivers, frees allocated memory, cleans pools, ...
 * Generally, prepares the game for shutting down
 */
static void ShutdownGame()
{
	IConsoleFree();

	if (_network_available) NetworkShutDown(); // Shut down the network and close any open connections

	SocialIntegration::Shutdown();
	DriverFactoryBase::ShutdownDrivers();

	UnInitWindowSystem();

	/* stop the scripts */
	AI::Uninitialize(false);
	Game::Uninitialize(false);

	/* Uninitialize variables that are allocated dynamically */
	_gamelog.Reset();

	LinkGraphSchedule::Clear();
	PoolBase::Clean(PT_ALL);

	/* No NewGRFs were loaded when it was still bootstrapping. */
	if (_game_mode != GM_BOOTSTRAP) ResetNewGRFData();

	FontCache::UninitializeFontCaches();
}

/**
 * Load the introduction game.
 * @param load_newgrfs Whether to load the NewGRFs or not.
 */
static void LoadIntroGame(bool load_newgrfs = true)
{
	OpenTTDSessionFrame frame;
	openttd_rust_session_intro(&GetRustSessionServices(), &frame, load_newgrfs);
	frame.Rethrow();
}

void MakeNewgameSettingsLive()
{
	openttd_rust_session_settings(&GetRustSessionServices());
}

void OpenBrowser(const std::string &url)
{
	/* Make sure we only accept urls that are sure to open a browser. */
	if (url.starts_with("http://") || url.starts_with("https://")) {
		OSOpenBrowser(url);
	}
}

/** Callback structure of statements to be executed after the NewGRF scan. */
struct AfterNewGRFScan : NewGRFScanCallback {
	TimerGameCalendar::Year startyear = CalendarTime::INVALID_YEAR; ///< The start year.
	uint32_t generation_seed = GENERATE_NEW_SEED; ///< Seed for the new game.
	std::string dedicated_host;                 ///< Hostname for the dedicated server.
	uint16_t dedicated_port = 0;                  ///< Port for the dedicated server.
	std::string connection_string;              ///< Information about the server to connect to
	std::string join_server_password;           ///< The password to join the server with.
	bool save_config = true;                    ///< The save config setting.

	/**
	 * Create a new callback.
	 */
	AfterNewGRFScan()
	{
		/* Visual C++ 2015 fails compiling this line (AfterNewGRFScan::generation_seed undefined symbol)
		 * if it's placed outside a member function, directly in the struct body. */
		static_assert(sizeof(generation_seed) == sizeof(_settings_game.game_creation.generation_seed));
	}

	void OnNewGRFsScanned() override
	{
		ResetGRFConfig(false);

		TarScanner::DoScan(TarScanner::Mode::Scenario);

		AI::Initialize();
		Game::Initialize();

		/* We want the new (correct) NewGRF count to survive the loading. */
		uint last_newgrf_count = _settings_client.gui.last_newgrf_count;
		LoadFromConfig();
		_settings_client.gui.last_newgrf_count = last_newgrf_count;
		/* Since the default for the palette might have changed due to
		 * reading the configuration file, recalculate that now. */
		UpdateNewGRFConfigPalette();

		Game::Uninitialize(true);
		AI::Uninitialize(true);
		LoadFromHighScore();
		LoadHotkeysFromConfig();
		WindowDesc::LoadFromConfig();

		/* We have loaded the config, so we may possibly save it. */
		_save_config = save_config;

		/* restore saved music and effects volumes */
		MusicDriver::GetInstance()->SetVolume(_settings_client.music.music_vol);
		SetEffectVolume(_settings_client.music.effect_vol);

		if (startyear != CalendarTime::INVALID_YEAR) IConsoleSetSetting("game_creation.starting_year", startyear.base());
		_settings_newgame.game_creation.generation_seed = generation_seed;

		if (!dedicated_host.empty()) {
			_network_bind_list.clear();
			_network_bind_list.emplace_back(dedicated_host);
		}
		if (dedicated_port != 0) _settings_client.network.server_port = dedicated_port;

		/* initialize the ingame console */
		IConsoleInit();
		InitializeGUI();
		IConsoleCmdExec("exec scripts/autoexec.scr 0");

		/* Make sure _settings is filled with _settings_newgame if we switch to a game directly */
		if (_switch_mode != SM_NONE) MakeNewgameSettingsLive();

		if (_network_available && !connection_string.empty()) {
			LoadIntroGame();
			_switch_mode = SM_NONE;

			NetworkClientConnectGame(connection_string, COMPANY_NEW_COMPANY, join_server_password);
		}

		/* After the scan we're not used anymore. */
		delete this;
	}
};

void PostMainLoop()
{
	WaitTillSaved();

	/* only save config if we have to */
	if (_save_config) {
		SaveToConfig();
		SaveHotkeysToConfig();
		WindowDesc::SaveToConfig();
		SaveToHighScore();
	}

	/* Reset windowing system, stop drivers, free used memory, ... */
	ShutdownGame();
}

#if defined(UNIX)
extern void DedicatedFork();
#endif

/**
 * Create all the options that OpenTTD supports. Each option is
 * always a single character with no, an optional or a required value.
 * @return The available options.
 */
static std::vector<OptionData> CreateOptions()
{
	std::vector<OptionData> options;
	/* Options that require a parameter. */
	for (char c : "GIMSbcmnpqrstv") options.push_back({ .type = ODF_HAS_VALUE, .id = c, .shortname = c });

	/* Options with an optional parameter. */
	for (char c : "Ddg") options.push_back({ .type = ODF_OPTIONAL_VALUE, .id = c, .shortname = c });

	/* Options without a parameter. */
	for (char c : "QXehx") options.push_back({ .type = ODF_NO_VALUE, .id = c, .shortname = c });
#if !defined(_WIN32)
	options.push_back({ .type = ODF_NO_VALUE, .id = 'f', .shortname = 'f' });
#endif

	return options;
}

/**
 * Main entry point for this lovely game.
 * @param arguments The command line arguments passed to the application.
 * @return 0 when there is no error.
 */
int openttd_main(std::span<std::string_view> arguments)
{
	_game_session_stats.start_time = std::chrono::steady_clock::now();
	_game_session_stats.savegame_size = std::nullopt;

	std::string musicdriver;
	std::string sounddriver;
	std::string videodriver;
	std::string blitter;
	std::string graphics_set;
	std::string sounds_set;
	std::string music_set;
	Dimension resolution = {0, 0};
	std::unique_ptr<AfterNewGRFScan> scanner = std::make_unique<AfterNewGRFScan>();
	bool dedicated = false;
	bool only_local_path = false;

	extern bool _dedicated_forks;
	_dedicated_forks = false;

	_game_mode = GM_MENU;
	_switch_mode = SM_MENU;

	auto options = CreateOptions();
	GetOptData mgo(arguments.subspan(1), options);

	int i;
	while ((i = mgo.GetOpt()) != -1) {
		switch (i) {
		case 'I': graphics_set = mgo.opt; break;
		case 'S': sounds_set = mgo.opt; break;
		case 'M': music_set = mgo.opt; break;
		case 'm': musicdriver = mgo.opt; break;
		case 's': sounddriver = mgo.opt; break;
		case 'v': videodriver = mgo.opt; break;
		case 'b': blitter = mgo.opt; break;
		case 'D':
			musicdriver = "null";
			sounddriver = "null";
			videodriver = "dedicated";
			blitter = "null";
			dedicated = true;
			SetDebugString("net=4", ShowInfoI);
			if (!mgo.opt.empty()) {
				scanner->dedicated_host = ParseFullConnectionString(mgo.opt, scanner->dedicated_port);
			}
			break;
		case 'f': _dedicated_forks = true; break;
		case 'n':
			scanner->connection_string = mgo.opt; // host:port#company parameter
			break;
		case 'p':
			scanner->join_server_password = mgo.opt;
			break;
		case 'r': ParseResolution(resolution, mgo.opt); break;
		case 't':
			if (auto value = ParseInteger(mgo.opt); value.has_value()) {
				scanner->startyear = TimerGameCalendar::Year(*value);
			} else {
				fmt::print(stderr, "Invalid start year: {}\n", mgo.opt);
			}
			break;
		case 'd': {
#if defined(_WIN32)
				CreateConsole();
#endif
				if (!mgo.opt.empty()) SetDebugString(mgo.opt, ShowInfoI);
				break;
			}
		case 'e':
			/* Allow for '-e' before or after '-g'. */
			switch (_switch_mode) {
				case SM_MENU: _switch_mode = SM_EDITOR; break;
				case SM_LOAD_GAME: _switch_mode = SM_LOAD_SCENARIO; break;
				case SM_START_HEIGHTMAP: _switch_mode = SM_LOAD_HEIGHTMAP; break;
				default: break;
			}
			break;
		case 'g':
			if (!mgo.opt.empty()) {
				_file_to_saveload.name = mgo.opt;

				std::string extension = FS2OTTD(std::filesystem::path(OTTD2FS(_file_to_saveload.name)).extension().native());
				auto [ft, _] = FiosGetSavegameListCallback(SLO_LOAD, _file_to_saveload.name, extension);
				if (ft == FIOS_TYPE_INVALID) {
					std::tie(ft, _) = FiosGetScenarioListCallback(SLO_LOAD, _file_to_saveload.name, extension);
				}
				if (ft == FIOS_TYPE_INVALID) {
					std::tie(ft, _) = FiosGetHeightmapListCallback(SLO_LOAD, _file_to_saveload.name, extension);
				}

				/* Allow for '-e' before or after '-g'. */
				switch (ft.abstract) {
					case FT_SAVEGAME: _switch_mode = (_switch_mode == SM_EDITOR ? SM_LOAD_SCENARIO : SM_LOAD_GAME); break;
					case FT_SCENARIO: _switch_mode = (_switch_mode == SM_EDITOR ? SM_LOAD_SCENARIO : SM_LOAD_GAME); break;
					case FT_HEIGHTMAP: _switch_mode = (_switch_mode == SM_EDITOR ? SM_LOAD_HEIGHTMAP : SM_START_HEIGHTMAP); break;
					default: break;
				}

				_file_to_saveload.SetMode(ft, SLO_LOAD);
				break;
			}

			_switch_mode = SM_NEWGAME;
			/* Give a random map if no seed has been given */
			if (scanner->generation_seed == GENERATE_NEW_SEED) {
				scanner->generation_seed = InteractiveRandom();
			}
			break;
		case 'q': {
			DeterminePaths(arguments[0], only_local_path);
			if (mgo.opt.empty()) {
				return 1;
			}

			std::string extension = FS2OTTD(std::filesystem::path(OTTD2FS(mgo.opt)).extension().native());
			auto [_, title] = FiosGetSavegameListCallback(SLO_LOAD, mgo.opt, extension);

			_load_check_data.Clear();
			SaveOrLoadResult res = SaveOrLoad(mgo.opt, SLO_CHECK, DFT_GAME_FILE, SAVE_DIR, false);
			if (res != SL_OK || _load_check_data.HasErrors()) {
				fmt::print(stderr, "Failed to open savegame\n");
				if (_load_check_data.HasErrors()) {
					InitializeLanguagePacks(); // A language pack is needed for GetString()
					fmt::print(stderr, "{}\n", GetString(_load_check_data.error, _load_check_data.error_msg));
				}
				return 1;
			}

			WriteSavegameInfo(title);
			return 0;
		}
		case 'Q': {
			extern int _skip_all_newgrf_scanning;
			_skip_all_newgrf_scanning += 1;
			break;
		}
		case 'G':
			if (auto value = ParseInteger(mgo.opt); value.has_value()) {
				scanner->generation_seed = *value;
			} else {
				fmt::print(stderr, "Invalid generation seed: {}\n", mgo.opt);
			}
			break;
		case 'c': _config_file = mgo.opt; break;
		case 'x': scanner->save_config = false; break;
		case 'X': only_local_path = true; break;
		case 'h': break; // handled below
		}
		if (i == 'h' || i == -2) break;
	}

	if (i == 'h' || i == -2 || !mgo.arguments.empty()) {
		/* Either the user typed '-h', they made an error, or they added unrecognized command line arguments.
		 * In all cases, print the help, and exit.
		 *
		 * The next two functions are needed to list the graphics sets. We can't do them earlier
		 * because then we cannot show it on the debug console as that hasn't been configured yet. */
		DeterminePaths(arguments[0], only_local_path);
		TarScanner::DoScan(TarScanner::Mode::Baseset);
		BaseGraphics::FindSets();
		BaseSounds::FindSets();
		BaseMusic::FindSets();
		ShowHelp();

		/* Return zero when asked to print help, and 1 for all other cases. */
		if (i == 'h') return 0;
		return 1;
	}

	DeterminePaths(arguments[0], only_local_path);
	TarScanner::DoScan(TarScanner::Mode::Baseset);

	if (dedicated) Debug(net, 3, "Starting dedicated server, version {}", _openttd_revision);
	if (_dedicated_forks && !dedicated) _dedicated_forks = false;

#if defined(UNIX)
	/* We must fork here, or we'll end up without some resources we need (like sockets) */
	if (_dedicated_forks) DedicatedFork();
#endif

	LoadFromConfig(true);

	if (resolution.width != 0) _cur_resolution = resolution;

	/* Limit width times height times bytes per pixel to fit a 32 bit
	 * integer, This way all internal drawing routines work correctly.
	 * A resolution that has one component as 0 is treated as a marker to
	 * auto-detect a good window size. */
	_cur_resolution.width  = std::min(_cur_resolution.width, UINT16_MAX / 2u);
	_cur_resolution.height = std::min(_cur_resolution.height, UINT16_MAX / 2u);

	/* Assume the cursor starts within the game as not all video drivers
	 * get an event that the cursor is within the window when it is opened.
	 * Saying the cursor is there makes no visible difference as it would
	 * just be out of the bounds of the window. */
	_cursor.in_window = true;

	/* enumerate language files */
	InitializeLanguagePacks();

	/* Initialize the font cache */
	FontCache::LoadFontCaches(FONTSIZES_REQUIRED);

	/* This must be done early, since functions use the SetWindowDirty* calls */
	InitWindowSystem();

	BaseGraphics::FindSets();
	bool valid_graphics_set;
	if (!graphics_set.empty()) {
		valid_graphics_set = BaseGraphics::SetSetByName(graphics_set);
	} else if (BaseGraphics::ini_data.shortname != 0) {
		graphics_set = BaseGraphics::ini_data.name;
		valid_graphics_set = BaseGraphics::SetSetByShortname(BaseGraphics::ini_data.shortname);
		if (valid_graphics_set && !BaseGraphics::ini_data.extra_params.empty()) {
			GRFConfig &extra_cfg = BaseGraphics::GetUsedSet()->GetOrCreateExtraConfig();
			if (extra_cfg.IsCompatible(BaseGraphics::ini_data.extra_version)) {
				extra_cfg.SetParams(BaseGraphics::ini_data.extra_params);
			}
		}
	} else if (!BaseGraphics::ini_data.name.empty()) {
		graphics_set = BaseGraphics::ini_data.name;
		valid_graphics_set = BaseGraphics::SetSetByName(BaseGraphics::ini_data.name);
	} else {
		valid_graphics_set = true;
		BaseGraphics::SetSet(nullptr); // ignore error, continue to bootstrap GUI
	}
	if (!valid_graphics_set) {
		BaseGraphics::SetSet(nullptr);

		ErrorMessageData msg(GetEncodedString(STR_CONFIG_ERROR), GetEncodedString(STR_CONFIG_ERROR_INVALID_BASE_GRAPHICS_NOT_FOUND, graphics_set));
		ScheduleErrorMessage(msg);
	}

	/* Initialize game palette */
	GfxInitPalettes();

	Debug(misc, 1, "Loading blitter...");
	if (blitter.empty() && !_ini_blitter.empty()) blitter = _ini_blitter;
	_blitter_autodetected = blitter.empty();
	/* Activate the initial blitter.
	 * This is only some initial guess, after NewGRFs have been loaded SwitchNewGRFBlitter may switch to a different one.
	 *  - Never guess anything, if the user specified a blitter. (_blitter_autodetected)
	 *  - Use 32bpp blitter if baseset or 8bpp-support settings says so.
	 *  - Use 8bpp blitter otherwise.
	 */
	if (!_blitter_autodetected ||
			(_support8bpp != S8BPP_NONE && (BaseGraphics::GetUsedSet() == nullptr || BaseGraphics::GetUsedSet()->blitter == BLT_8BPP)) ||
			BlitterFactory::SelectBlitter("32bpp-anim") == nullptr) {
		if (BlitterFactory::SelectBlitter(blitter) == nullptr) {
			blitter.empty() ?
				UserError("Failed to autoprobe blitter") :
				UserError("Failed to select requested blitter '{}'; does it exist?", blitter);
		}
	}

	if (videodriver.empty() && !_ini_videodriver.empty()) videodriver = _ini_videodriver;
	DriverFactoryBase::SelectDriver(videodriver, Driver::DT_VIDEO);

	InitializeSpriteSorter();

	/* Initialize the zoom level of the screen to normal */
	_screen.zoom = ZoomLevel::Min;

	/* The video driver is now selected, now initialise GUI zoom */
	UpdateGUIZoom();

	SocialIntegration::Initialize();
	NetworkStartUp(); // initialize network-core

	if (!HandleBootstrap()) {
		ShutdownGame();
		return 0;
	}

	VideoDriver::GetInstance()->ClaimMousePointer();

	BaseSounds::FindSets();
	if (sounds_set.empty() && !BaseSounds::ini_set.empty()) sounds_set = BaseSounds::ini_set;
	if (!BaseSounds::SetSetByName(sounds_set)) {
		if (sounds_set.empty() || !BaseSounds::SetSet({})) {
			UserError("Failed to find a sounds set. Please acquire a sounds set for OpenTTD. See section 1.4 of README.md.");
		} else {
			ErrorMessageData msg(GetEncodedString(STR_CONFIG_ERROR), GetEncodedString(STR_CONFIG_ERROR_INVALID_BASE_SOUNDS_NOT_FOUND, sounds_set));
			ScheduleErrorMessage(msg);
		}
	}

	BaseMusic::FindSets();
	if (music_set.empty() && !BaseMusic::ini_set.empty()) music_set = BaseMusic::ini_set;
	if (!BaseMusic::SetSetByName(music_set)) {
		if (music_set.empty() || !BaseMusic::SetSet({})) {
			UserError("Failed to find a music set. Please acquire a music set for OpenTTD. See section 1.4 of README.md.");
		} else {
			ErrorMessageData msg(GetEncodedString(STR_CONFIG_ERROR), GetEncodedString(STR_CONFIG_ERROR_INVALID_BASE_MUSIC_NOT_FOUND, music_set));
			ScheduleErrorMessage(msg);
		}
	}

	if (sounddriver.empty() && !_ini_sounddriver.empty()) sounddriver = _ini_sounddriver;
	DriverFactoryBase::SelectDriver(sounddriver, Driver::DT_SOUND);

	if (musicdriver.empty() && !_ini_musicdriver.empty()) musicdriver = _ini_musicdriver;
	DriverFactoryBase::SelectDriver(musicdriver, Driver::DT_MUSIC);

	GenerateWorld(GWM_EMPTY, 64, 64); // Make the viewport initialization happy
	LoadIntroGame(false);

	/* ScanNewGRFFiles now has control over the scanner. */
	RequestNewGRFScan(scanner.release());

	VideoDriver::GetInstance()->MainLoop();

	PostMainLoop();
	return 0;
}

void HandleExitGameRequest()
{
	if (_game_mode == GM_MENU || _game_mode == GM_BOOTSTRAP) { // do not ask to quit on the main screen
		_exit_game = true;
	} else if (_settings_client.gui.autosave_on_exit) {
		DoExitSave();
		_survey.Transmit(NetworkSurveyHandler::Reason::EXIT, true);
		_exit_game = true;
	} else {
		AskExitGame();
	}
}


/**
 * Load the specified savegame but on error do different things.
 * If loading fails due to corrupt savegame, bad version, etc. go back to
 * a previous correct state. In the menu for example load the intro game again.
 * @param filename file to be loaded
 * @param fop mode of loading, always SLO_LOAD
 * @param newgm switch to this mode of loading fails due to some unknown error
 * @param subdir default directory to look for filename, set to 0 if not needed
 * @param lf Load filter to use, if nullptr: use filename + subdir.
 */
bool SafeLoad(const std::string &filename, SaveLoadOperation fop, DetailedFileType dft, GameMode newgm, Subdirectory subdir, std::shared_ptr<LoadFilter> lf = nullptr)
{
	assert(fop == SLO_LOAD);
	assert(dft == DFT_GAME_FILE || (lf == nullptr && dft == DFT_OLD_GAME_FILE));
	OpenTTDSessionLoad input{filename, fop, dft, subdir, std::move(lf)};
	OpenTTDSessionFrame frame;
	bool result = openttd_rust_session_safe_load(&GetRustSessionServices(), &frame, &input, newgm);
	frame.Rethrow();
	return result;
}

static void UpdateSocialIntegration(GameMode game_mode)
{
	switch (game_mode) {
		case GM_BOOTSTRAP:
		case GM_MENU:
			SocialIntegration::EventEnterMainMenu();
			break;

		case GM_NORMAL:
			if (_networking) {
				SocialIntegration::EventEnterMultiplayer(Map::SizeX(), Map::SizeY());
			} else {
				SocialIntegration::EventEnterSingleplayer(Map::SizeX(), Map::SizeY());
			}
			break;

		case GM_EDITOR:
			SocialIntegration::EventEnterScenarioEditor(Map::SizeX(), Map::SizeY());
			break;
	}
}

void SwitchToMode(SwitchMode new_mode)
{
	OpenTTDSessionFrame frame;
	openttd_rust_session_switch(&GetRustSessionServices(), &frame, new_mode);
	frame.Rethrow();
}



/**
 * State controlling game loop.
 * The state must not be changed from anywhere but here.
 * That check is enforced in DoCommand.
 */
void StateGameLoop()
{
	OpenTTDSessionFrame frame;
	openttd_rust_session_state_loop(&GetRustSessionServices(), &frame);
	frame.Rethrow();
}

/** Interval for regular autosaves. Initialized at zero to disable till settings are loaded. */
static IntervalTimer<TimerGameRealtime> _autosave_interval({std::chrono::milliseconds::zero(), TimerGameRealtime::AUTOSAVE}, [](auto)
{
	/* We reset the command-during-pause mode here, so we don't continue
	 * to make auto-saves when nothing more is changing. */
	_pause_mode.Reset(PauseMode::CommandDuringPause);

	_do_autosave = true;
	SetWindowDirty(WC_STATUS_BAR, 0);

	static FiosNumberedSaveName _autosave_ctr("autosave");
	DoAutoOrNetsave(_autosave_ctr);

	_do_autosave = false;
	SetWindowDirty(WC_STATUS_BAR, 0);
});

/**
 * Reset the interval of the autosave.
 *
 * If reset is not set, this does not set the elapsed time on the timer,
 * so if the interval is smaller, it might result in an autosave being done
 * immediately.
 *
 * @param reset Whether to reset the timer back to zero, or to continue.
 */
void ChangeAutosaveFrequency(bool reset)
{
	_autosave_interval.SetInterval({std::chrono::minutes(_settings_client.gui.autosave_interval), TimerGameRealtime::AUTOSAVE}, reset);
}

/**
 * Request a new NewGRF scan. This will be executed on the next game-tick.
 * This is mostly needed to ensure NewGRF scans (which are blocking) are
 * done in the game-thread, and not in the draw-thread (which most often
 * triggers this request).
 * @param callback Optional callback to call when NewGRF scan is completed.
 * @return True when the NewGRF scan was actually requested, false when the scan was already running.
 */
bool RequestNewGRFScan(NewGRFScanCallback *callback)
{
	return openttd_rust_session_scan(callback);
}

void GameLoop()
{
	OpenTTDSessionFrame frame;
	openttd_rust_session_game_loop(&GetRustSessionServices(), &frame);
	frame.Rethrow();
}

#include "rust/session_services.inc"

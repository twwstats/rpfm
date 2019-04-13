//---------------------------------------------------------------------------//
// Copyright (c) 2017-2019 Ismael Gutiérrez González. All rights reserved.
// 
// This file is part of the Rusted PackFile Manager (RPFM) project,
// which can be found here: https://github.com/Frodo45127/rpfm.
// 
// This file is licensed under the MIT license, which can be found here:
// https://github.com/Frodo45127/rpfm/blob/master/LICENSE.
//---------------------------------------------------------------------------//

// This is the main starting poing of RPFM. The begining of all ends.

// Disabled `Clippy` linters, with the reasons why they were disabled.
#![allow(
    clippy::cast_lossless,                  // Disabled due to useless warnings.
    clippy::cognitive_complexity,           // Disabled due to useless warnings.
    clippy::cyclomatic_complexity,          // Disabled due to useless warnings.
    clippy::doc_markdown,                   // Disabled due to false positives on things that shouldn't be formated in the docs as it says.
    clippy::if_same_then_else,              // Disabled because some of the solutions it provides are freaking hard to read.
    clippy::match_bool,                     // Disabled because the solutions it provides are harder to read than the current code.
    clippy::module_inception,               // Disabled because it's quite useless.
    clippy::needless_bool,                  // Disabled because the solutions it provides are harder to read than the current code.
    clippy::new_ret_no_self,                // Disabled because the reported situations are special cases. So no, I'm not going to rewrite them.
    clippy::redundant_closure,              // Disabled because the solutions it provides doesn't even work.             
    clippy::suspicious_else_formatting,     // Disabled because the errors it gives are actually false positives due to comments.
    clippy::too_many_arguments,             // Disabled because you never have enough arguments.
    clippy::type_complexity,                // Disabled temporarily because there are other things to do before rewriting the types it warns about.
    clippy::useless_format,                 // Disabled due to false positives.
    clippy::match_wild_err_arm              // Disabled because, despite being a bad practice, it's the intended behavior in the code it warns about.
)]

use clap::{App, Arg, SubCommand};
use indexmap::map::IndexMap;
use lazy_static::lazy_static;

use std::sync::{Arc, Mutex};
use std::panic;
use std::path::PathBuf;

use crate::common::communications::*;
use crate::error::logger::Report;
use crate::packfile::packedfile::PackedFile;
use crate::packedfile::*;
use crate::packedfile::db::DB;
use crate::packedfile::db::schemas::Schema;
use crate::packfile::PFHVersion;
use crate::settings::*;
use crate::settings::shortcuts::Shortcuts;
use crate::ui::*;
use crate::ui::packfile_treeview::*;
use crate::ui::settings::*;
use crate::ui::table_state::*;
use crate::ui_thread::*;

/// This macro is used to clone the variables into the closures without the compiler complaining.
/// This should be BEFORE the `mod xxx` stuff, so submodules can use it too.
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

mod background_thread;
mod background_thread_extra;
mod cli_thread;
mod common;
mod error;
mod packedfile;
mod packfile;
mod settings;
mod updater;
mod ui;
mod ui_thread;
mod ui_thread_extra;

// Statics, so we don't need to pass them everywhere to use them.
lazy_static! {

    /// List of supported games and their configuration. Their key is what we know as `folder_name`, used to identify the game and
    /// for "MyMod" folders.
    #[derive(Debug)]
    static ref SUPPORTED_GAMES: IndexMap<&'static str, GameInfo> = {
        let mut map = IndexMap::new();

        // Warhammer 2
        map.insert("warhammer_2", GameInfo {
            display_name: "Warhammer 2".to_owned(),
            id: PFHVersion::PFH5,
            schema: "schema_wh.json".to_owned(),
            db_packs: vec!["data.pack".to_owned()],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(594_570),
            raw_db_version: 2,
            pak_file: Some("wh2.pak".to_owned()),
            ca_types_file: Some("ca_types_wh2".to_owned()),
            supports_editing: true,
            game_selected_icon: "gs_wh2.png".to_owned(),
        });

        // Warhammer
        map.insert("warhammer", GameInfo {
            display_name: "Warhammer".to_owned(),
            id: PFHVersion::PFH4,
            schema: "schema_wh.json".to_owned(),
            db_packs: vec![
                "data.pack".to_owned(),         // Central data PackFile
                "data_bl.pack".to_owned(),      // Blood DLC Data
                "data_bm.pack".to_owned()       // Beastmen DLC Data
            ],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(364_360),
            raw_db_version: 2,
            pak_file: Some("wh.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_wh.png".to_owned(),
        });

        // Thrones of Britannia
        map.insert("thrones_of_britannia", GameInfo {
            display_name: "Thrones of Britannia".to_owned(),
            id: PFHVersion::PFH4,
            schema: "schema_tob.json".to_owned(),
            db_packs: vec!["data.pack".to_owned()],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(712_100),
            raw_db_version: 2,
            pak_file: Some("tob.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_tob.png".to_owned(),
        });

        // Attila
        map.insert("attila", GameInfo {
            display_name: "Attila".to_owned(),
            id: PFHVersion::PFH4,
            schema: "schema_att.json".to_owned(),
            db_packs: vec!["data.pack".to_owned()],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(325_610),
            raw_db_version: 2,
            pak_file: Some("att.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_att.png".to_owned(),
        });

        // Rome 2
        map.insert("rome_2", GameInfo {
            display_name: "Rome 2".to_owned(),
            id: PFHVersion::PFH4,
            schema: "schema_rom2.json".to_owned(),
            db_packs: vec!["data_rome2.pack".to_owned()],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(214_950),
            raw_db_version: 2,
            pak_file: Some("rom2.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_rom2.png".to_owned(),
        });

        // Shogun 2
        map.insert("shogun_2", GameInfo {
            display_name: "Shogun 2".to_owned(),
            id: PFHVersion::PFH3,
            schema: "schema_sho2.json".to_owned(),
            db_packs: vec!["data.pack".to_owned()],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "local_br.pack".to_owned(),     // Brazilian
                "local_cz.pack".to_owned(),     // Czech
                "local_ge.pack".to_owned(),     // German
                "local_sp.pack".to_owned(),     // Spanish
                "local_fr.pack".to_owned(),     // French
                "local_it.pack".to_owned(),     // Italian
                "local_kr.pack".to_owned(),     // Korean
                "local_pl.pack".to_owned(),     // Polish
                "local_ru.pack".to_owned(),     // Russian
                "local_tr.pack".to_owned(),     // Turkish
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "local_zh.pack".to_owned(),     // Traditional Chinese
            ],
            steam_id: Some(34330),
            raw_db_version: 1,
            pak_file: Some("sho2.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_sho2.png".to_owned(),
        });

        // Napoleon
        map.insert("napoleon", GameInfo {
            display_name: "Napoleon".to_owned(),
            id: PFHVersion::PFH0,
            schema: "schema_nap.json".to_owned(),
            db_packs: vec![                     // NOTE: Patches 5 and 7 has no table changes, so they should not be here.
                "data.pack".to_owned(),         // Main DB PackFile
                "patch.pack".to_owned(),        // First Patch
                "patch2.pack".to_owned(),       // Second Patch
                "patch3.pack".to_owned(),       // Third Patch
                "patch4.pack".to_owned(),       // Fourth Patch
                "patch6.pack".to_owned(),       // Six Patch
            ],
            loc_packs: vec![
                "local_en.pack".to_owned(),         // English
                "local_en_patch.pack".to_owned(),   // English Patch
                "local_br.pack".to_owned(),         // Brazilian
                "local_br_patch.pack".to_owned(),   // Brazilian Patch
                "local_cz.pack".to_owned(),         // Czech
                "local_cz_patch.pack".to_owned(),   // Czech Patch
                "local_ge.pack".to_owned(),         // German
                "local_ge_patch.pack".to_owned(),   // German Patch
                "local_sp.pack".to_owned(),         // Spanish
                "local_sp_patch.pack".to_owned(),   // Spanish Patch
                "local_fr.pack".to_owned(),         // French
                "local_fr_patch.pack".to_owned(),   // French Patch
                "local_it.pack".to_owned(),         // Italian
                "local_it_patch.pack".to_owned(),   // Italian Patch
                "local_kr.pack".to_owned(),         // Korean
                "local_kr_patch.pack".to_owned(),   // Korean Patch
                "local_pl.pack".to_owned(),         // Polish
                "local_pl_patch.pack".to_owned(),   // Polish Patch
                "local_ru.pack".to_owned(),         // Russian
                "local_ru_patch.pack".to_owned(),   // Russian Patch
                "local_tr.pack".to_owned(),         // Turkish
                "local_tr_patch.pack".to_owned(),   // Turkish Patch
                "local_cn.pack".to_owned(),         // Simplified Chinese
                "local_cn_patch.pack".to_owned(),   // Simplified Chinese Patch
                "local_zh.pack".to_owned(),         // Traditional Chinese
                "local_zh_patch.pack".to_owned(),   // Traditional Chinese Patch
            ],
            steam_id: Some(34030),
            raw_db_version: 0,
            pak_file: Some("nap.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_nap.png".to_owned(),
        });

        // Empire
        map.insert("empire", GameInfo {
            display_name: "Empire".to_owned(),
            id: PFHVersion::PFH0,
            schema: "schema_emp.json".to_owned(),
            db_packs: vec![
                "main.pack".to_owned(),         // Main DB PackFile
                "models.pack".to_owned(),       // Models PackFile (contains model-related DB Tables)
                "patch.pack".to_owned(),        // First Patch
                "patch2.pack".to_owned(),       // Second Patch
                "patch3.pack".to_owned(),       // Third Patch
                "patch4.pack".to_owned(),       // Fourth Patch
                "patch5.pack".to_owned(),       // Fifth Patch
            ],
            loc_packs: vec![
                "local_en.pack".to_owned(),     // English
                "patch_en.pack".to_owned(),     // English Patch
                "local_br.pack".to_owned(),     // Brazilian
                "patch_br.pack".to_owned(),     // Brazilian Patch
                "local_cz.pack".to_owned(),     // Czech
                "patch_cz.pack".to_owned(),     // Czech Patch
                "local_ge.pack".to_owned(),     // German
                "patch_ge.pack".to_owned(),     // German Patch
                "local_sp.pack".to_owned(),     // Spanish
                "patch_sp.pack".to_owned(),     // Spanish Patch
                "local_fr.pack".to_owned(),     // French
                "patch_fr.pack".to_owned(),     // French Patch
                "local_it.pack".to_owned(),     // Italian
                "patch_it.pack".to_owned(),     // Italian Patch
                "local_kr.pack".to_owned(),     // Korean
                "patch_kr.pack".to_owned(),     // Korean Patch
                "local_pl.pack".to_owned(),     // Polish
                "patch_pl.pack".to_owned(),     // Polish Patch
                "local_ru.pack".to_owned(),     // Russian
                "patch_ru.pack".to_owned(),     // Russian Patch
                "local_tr.pack".to_owned(),     // Turkish
                "patch_tr.pack".to_owned(),     // Turkish Patch
                "local_cn.pack".to_owned(),     // Simplified Chinese
                "patch_cn.pack".to_owned(),     // Simplified Chinese Patch
                "local_zh.pack".to_owned(),     // Traditional Chinese
                "patch_zh.pack".to_owned(),     // Traditional Chinese Patch
            ],
            steam_id: Some(10500),
            raw_db_version: 0,
            pak_file: Some("emp.pak".to_owned()),
            ca_types_file: None,
            supports_editing: true,
            game_selected_icon: "gs_emp.png".to_owned(),
        });

        // NOTE: There are things that depend on the order of this list, and this game must ALWAYS be the last one.
        // Otherwise, stuff that uses this list will probably break.
        // Arena
        map.insert("arena", GameInfo {
            display_name: "Arena".to_owned(),
            id: PFHVersion::PFH5,
            schema: "schema_are.json".to_owned(),
            db_packs: vec!["wad.pack".to_owned()],
            loc_packs: vec!["local_ex.pack".to_owned()],
            steam_id: None,
            raw_db_version: -1,
            pak_file: None,
            ca_types_file: None,
            supports_editing: false,
            game_selected_icon: "gs_are.png".to_owned(),
        });

        map
    };

    /// Path were the stuff used by RPFM (settings, schemas,...) is. In debug mode, we just take the current path
    /// (so we don't break debug builds). In Release mode, we take the `.exe` path.
    #[derive(Debug)]
    static ref RPFM_PATH: PathBuf = if cfg!(debug_assertions) {
        std::env::current_dir().unwrap()
    } else {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path
    };

    /// The current Settings and Shortcuts. To avoid reference and lock issues, this should be edited ONLY in the background thread.
    static ref SETTINGS: Arc<Mutex<Settings>> = Arc::new(Mutex::new(Settings::load().unwrap_or_else(|_|Settings::new())));
    static ref SHORTCUTS: Arc<Mutex<Shortcuts>> = Arc::new(Mutex::new(Shortcuts::load().unwrap_or_else(|_|Shortcuts::new())));

    /// The current GameSelected. Same as the one above, only edited from the background thread.
    static ref GAME_SELECTED: Arc<Mutex<String>> = Arc::new(Mutex::new(SETTINGS.lock().unwrap().settings_string["default_game"].to_owned()));

    /// PackedFiles from the dependencies of the currently open PackFile.
    static ref DEPENDENCY_DATABASE: Mutex<Vec<PackedFile>> = Mutex::new(vec![]);
    
    /// DB Files from the Pak File of the current game. Only for dependency checking.
    static ref FAKE_DEPENDENCY_DATABASE: Mutex<Vec<DB>> = Mutex::new(vec![]);

    /// Currently loaded schema.
    static ref SCHEMA: Arc<Mutex<Option<Schema>>> = Arc::new(Mutex::new(None));
}

/// These constants get RPFM's data, so we can use them everywhere.
const PROGRAM_NAME: &str = "Rusted PackFile Manager";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

/// This constant is used to enable or disable the generation of a new Schema file in compile time.
/// If you don't want to explicity create a new Schema for a game, leave this disabled.
const GENERATE_NEW_SCHEMA: bool = false;

/// This constant is used to check the references of every table in a PackFile and return the errors. For now it's only to check
/// if tables have swapped columns, but it may be expanded in the future.
const SHOW_TABLE_SCHEMA_ERRORS: bool = false;

// Main function. Simple: if we pass it "--cli" we boot in CLI mode. Otherwise, it's always ui.
fn main() {

    // Log the crashes so the user can send them himself.
    if !cfg!(debug_assertions) { panic::set_hook(Box::new(move |info: &panic::PanicInfo| { Report::new(info).save().unwrap(); })); }

    // Get the full argument list, so we can check if it's time for UI or CLI.
    let matches = App::new(PROGRAM_NAME)
      .version(VERSION)
      .author(AUTHOR)
      .about("A modding tool for modern (Post-Empire, Empire Included) Total War Games.")
      .arg(Arg::with_name("cli")
           .long("cli")
           .help("Enable the CLI mode of RPFM. For getting it called by other apps and automatizing tasks."))
      .subcommand(SubCommand::with_name("test")
          .about("controls testing features")
          .version("1.3")
          .author("Someone E. <someone_else@other.com>")
          .arg(Arg::with_name("debug")
              .short("d")
              .help("print debug information verbosely")))
      .get_matches();

    // If we are executing with `--cli` as argument, boot to CLI mode. 
    if matches.is_present("cli") { 
        println!("yay");
        if let Some(matches) = matches.subcommand_matches("test") {
            println!("Printing debug info...");
        } else {
            println!("Printing normally...");
        }
    }
    else { build_ui(); }
}

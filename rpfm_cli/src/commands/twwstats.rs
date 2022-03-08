//---------------------------------------------------------------------------//
// This file is licensed under the MIT license, which can be found here:
// https://github.com/Frodo45127/rpfm/blob/master/LICENSE.
//---------------------------------------------------------------------------//

use std::path::Path;
use std::collections::HashMap;
use bytesize::ByteSize;
use colored::*;
use log::{error, info, warn};
use prettytable::{Table, row, cell};
use std::process::exit;
use std::fs::File;

use std::path::PathBuf;
use std::vec;

use rpfm_error::Result;
use rpfm_lib::packedfile::PackedFileType;
use rpfm_lib::packedfile::DecodedPackedFile;
use rpfm_lib::packfile::*;
use rpfm_lib::schema::Schema;
use rpfm_lib::dependencies::Dependencies;

use crate::config::Config;

//---------------------------------------------------------------------------//
// 							Schema Command Variants
//---------------------------------------------------------------------------//

/// Extensions of images to export
pub const EXTENSIONS: [&str; 3] = [
    ".jpg",
    ".jpeg",
    // ".tga",
    // ".dds",
    ".png",
];

pub fn export(config: &Config, packfile: &str, destination: &str) -> Result<()> {
	if config.verbosity_level > 0 {
		info!("Exporting tables as JSON files to {}...", destination);
	}

    // OLD exporter
    // https://github.com/twwstats/twwstats-dataexporter/blob/master/twwstats_dataexporter/TwwstatsDataExporter.cs

    // DO THE MAGIC HERE!
    // warn!("PATHS: {}", SETTINGS.read().unwrap().paths);

    // let path = match config.game_selected.as_ref().unwrap().get_data_path() {
    //     Ok(path) => path,
    //     Err(error) => { error!("{} {}","Error:".red().bold(), error.to_terminal()); exit(1) }
    // };

    // Preload the default game's dependencies.
    let mut dependencies = Dependencies::default();
    let game_selected = config.game_selected.as_ref().unwrap();
    let version = game_selected.get_raw_db_version();
    let schema = Schema::load(game_selected.get_schema_name())?;

    let cache: Dependencies = dependencies.generate_dependencies_cache(&None, version)?;

    let mut packfile = PackFile::open_all_ca_packfiles().unwrap();

    for file in packfile.get_ref_mut_packed_files_by_type(PackedFileType::Loc, false) {
        if let Ok(DecodedPackedFile::Loc(table)) = file.decode_return_ref_no_locks(&schema) {
            warn!("{}", file.get_path().last().unwrap());
        }
    }

    for file in packfile.get_ref_mut_packed_files_by_type(PackedFileType::DB, false) {
        if let Ok(DecodedPackedFile::DB(table)) = file.decode_return_ref_no_locks(&schema) {
            // Create a file for each table
            let mut out_path = Path::new(&destination).join(table.get_table_name_without_tables());
            out_path.set_extension("json");
            // warn!("{}", out_path.to_str().unwrap());
            let mut entries = table.get_ref_table_data().to_vec();
            let definition = table.get_ref_definition();

            // let first_key = definition.get_fields_sorted().iter().position(|x| x.get_is_key()).unwrap_or(0);

            // ::serde_json::to_writer(&File::create(out_path)?, &entries)?;
            if definition.get_localised_fields().len() > 0 {
                warn!("{}", table.get_table_name_without_tables());
                warn!("{:?}", definition.get_localised_fields());
                exit(1);
            }
        }
    }

    // EXPORT IMAGES - WORKING!!!
    // for file in packfile.get_ref_mut_packed_files_by_path_start(&[String::from("ui")]) {
    //     if let Some(packedfile_name) = file.get_path().last() {
    //         let packedfile_name = packedfile_name.to_lowercase();

    //         if EXTENSIONS.iter().any(|x| packedfile_name.ends_with(x)) {
    //             let out_path = Path::new(&destination);
    //             file.extract_packed_file(out_path, false);
    //         }
    //     }
    // }

    // let mut table = Table::new();
    // table.add_row(row!["PackedFile Path", "Type", "Size"]);
    // for file in packfile.get_ref_packed_files_all() {
    //     let packedfile_type = PackedFileType::get_packed_file_type(file.get_ref_raw(), true);
    //     if packedfile_type == PackedFileType::Image {
    //         if let Some(packedfile_name) = file.get_path().last() {
    //             let packedfile_name = packedfile_name.to_lowercase();

    //             if EXTENSIONS.iter().any(|x| packedfile_name.ends_with(x)) {
    //                 warn!("{}", file.get_path().join("/"));
    //             }
    //         }

    //         // let size = ByteSize::kib((file.get_raw_data_size() / 1024).into());
    //         // table.add_row(row![file.get_path().join("/"), packedfile_type, size]);
    //     }
    // }

    // table.printstd();

	let result = Ok(());
    if config.verbosity_level > 0 {
        info!("Export completed!");
    }
    result
}

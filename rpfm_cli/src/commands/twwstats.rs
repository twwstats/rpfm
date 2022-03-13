//---------------------------------------------------------------------------//
// This file is licensed under the MIT license, which can be found here:
// https://github.com/Frodo45127/rpfm/blob/master/LICENSE.
//---------------------------------------------------------------------------//

use rpfm_lib::common::encoder::Encoder;
use rpfm_lib::common::decoder::Decoder;
use std::path::Path;
use std::collections::HashMap;
use bytesize::ByteSize;
use colored::*;
use log::{error, info, warn};
use prettytable::{Table, row, cell};
use std::process::exit;
use std::fs::File;
use serde_json::json;
use rpfm_lib::schema::Field;

use std::path::PathBuf;
use std::vec;

use rpfm_error::Result;
use rpfm_lib::packedfile::PackedFileType;
use rpfm_lib::packedfile::DecodedPackedFile;
use rpfm_lib::packfile::*;
use rpfm_lib::schema::Schema;
use rpfm_lib::dependencies::Dependencies;
use rpfm_lib::packedfile::table::DecodedData;

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

fn process_string(field: &Field, data: &String) -> String {
    if !data.is_empty() {
        if field.get_is_filename() {
            let mut path = data.replace('\\', "/");

            // If it's a folder, remove the trailing /.
            if path.ends_with('/') {
                path.pop();
            }

            return path;
        }
    }

    return data.clone();
}

pub fn export(config: &Config, destination: &str) -> Result<()> {
    info!("Exporting tables as JSON files to {}...", destination);

    let game_selected = config.game_selected.as_ref().unwrap();
    let version = game_selected.get_raw_db_version();
    let schema = Schema::load(game_selected.get_schema_name())?;

    info!("Opening packfiles for {}...", game_selected.get_display_name());
    let mut packfile = PackFile::open_all_ca_packfiles().unwrap();

    info!("Building locs HashMap...");
    let locs: HashMap<_, _> = packfile.get_ref_mut_packed_files_by_type(PackedFileType::Loc, false).iter_mut().filter_map(|file| {
        let name = file.get_path().last().unwrap().to_string().replace("__.loc", "");
        warn!("Loc file found: {}", name);
        if let Ok(DecodedPackedFile::Loc(table)) = file.decode_return_ref_no_locks(&schema) {
            Some((name, table.clone()))
        }
        else { None }
    }).collect();

    info!("Building DB tables...");
    for file in packfile.get_ref_mut_packed_files_by_type(PackedFileType::DB, false) {
        if let Ok(DecodedPackedFile::DB(table)) = file.decode_return_ref_no_locks(&schema) {
            let definition = table.get_ref_definition();

            let fields_processed = definition.get_fields_processed();

            let localised_fields = definition.get_localised_fields();

            let table_name_without_tables = &table.get_table_name_without_tables();

            // FOR DEBUGGING PURPOSES
            // if table_name_without_tables != "land_units" {
            //     continue;
            // }

            let mut loc_strings = HashMap::<String, String>::new();
            if let Some(loc_table) = locs.get(table_name_without_tables) {
                loc_table.get_ref_table_data().iter().for_each(|cells| {
                    let mut prefix_to_remove = table_name_without_tables.clone();
                    prefix_to_remove.push('_');
                    let key = match &cells[0] {
                        DecodedData::StringU16(data) => data.to_string().strip_prefix(&prefix_to_remove).unwrap().to_string(),
                        _ => String::from("")
                    };
                    let value = match &cells[1] {
                        DecodedData::StringU16(data) => data.to_string(),
                        _ => String::from("")
                    };
                    loc_strings.insert(key, value);
                });
            };

            warn!("DB Table: {}", table.get_table_name());
            let json_data: Vec<serde_json::Map<String, serde_json::value::Value>> = table.get_ref_table_data().iter().map(|cells| {
                let mut json_map = serde_json::Map::new();
                let primary_key_col = fields_processed.iter().position(|f| f.get_is_key()).unwrap();
                let primary_key = match &cells[primary_key_col] {
                    DecodedData::StringU8(data) => data.to_string(),
                    DecodedData::StringU16(data) => data.to_string(),
                    _ => String::from("")
                };
                for (column, field) in fields_processed.iter().enumerate() {
                    let json_key = field.get_name().to_string();

                    match &cells[column] {
                        DecodedData::Boolean(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::F32(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::F64(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::I16(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::I32(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::I64(data) => json_map.insert(json_key, json!(data)),
                        DecodedData::StringU8(data) => json_map.insert(json_key, json!(process_string(field, data))),
                        DecodedData::StringU16(data) => json_map.insert(json_key, json!(process_string(field, data))),
                        DecodedData::OptionalStringU8(data) => json_map.insert(json_key, json!(process_string(field, data))),
                        DecodedData::OptionalStringU16(data) => json_map.insert(json_key, json!(process_string(field, data))),
                        // Special case: we need to convert this into the hex representation of its bytes.
                        DecodedData::ColourRGB(data) => {
                            let mut encoded = Vec::with_capacity(4);
                            encoded.encode_integer_colour_rgb(*data);
                            match encoded.decode_string_colour_rgb(0) {
                                Ok(data) => json_map.insert(json_key, json!(data)),
                                Err(_) => json_map.insert(json_key, json!("000000")),
                            }
                        },
                        DecodedData::SequenceU16(_) => json_map.insert(json_key, json!("SequenceU16")),
                        DecodedData::SequenceU32(_) => json_map.insert(json_key, json!("SequenceU32")),
                    };
                }

                // Localised fields
                localised_fields.iter().for_each(|field| {
                    let json_key = field.get_name().to_string();
                    let mut loc_key = field.get_name().to_string().clone();
                    loc_key.push('_');
                    loc_key.push_str(&primary_key);
                    if let Some(value) = loc_strings.get(&loc_key) {
                        json_map.insert(json_key, json!(value.clone()));
                    }
                });

                // Return the row
                json_map
            }).collect();

            let mut out_path = Path::new(&destination).join(table.get_table_name_without_tables());
            out_path.set_extension("json");
            warn!("Saving {:?}...", out_path);
            ::serde_json::to_writer(&File::create(out_path)?, &json_data)?;
        }
    }

    // info!("Exporting images...");
    // for file in packfile.get_ref_mut_packed_files_by_path_start(&[String::from("ui")]) {
    //     if let Some(packedfile_name) = file.get_path().last() {
    //         let packedfile_name = packedfile_name.to_lowercase();

    //         if EXTENSIONS.iter().any(|x| packedfile_name.ends_with(x)) {
    //             let out_path = Path::new(&destination);
    //             file.extract_packed_file(out_path, false);
    //         }
    //     }
    // }

	let result = Ok(());
    if config.verbosity_level > 0 {
        info!("Export completed!");
    }
    result
}

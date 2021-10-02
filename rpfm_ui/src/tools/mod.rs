//---------------------------------------------------------------------------//
// Copyright (c) 2017-2020 Ismael Gutiérrez González. All rights reserved.
//
// This file is part of the Rusted PackFile Manager (RPFM) project,
// which can be found here: https://github.com/Frodo45127/rpfm.
//
// This file is licensed under the MIT license, which can be found here:
// https://github.com/Frodo45127/rpfm/blob/master/LICENSE.
//---------------------------------------------------------------------------//

/*!
Module with all the code for managing the UI.

This module contains the code to manage the main UI and store all his slots.
!*/

use qt_widgets::QCheckBox;
use qt_widgets::QDialogButtonBox;
use qt_widgets::QDialog;
use qt_widgets::QDoubleSpinBox;
use qt_widgets::QLabel;
use qt_widgets::QLineEdit;
use qt_widgets::QSpinBox;
use qt_widgets::QTextEdit;
use qt_widgets::QWidget;

use qt_core::QBox;
use qt_core::QObject;
use qt_core::QPtr;
use qt_core::QString;

use qt_ui_tools::QUiLoader;

use cpp_core::{CastInto, DynamicCast, Ptr, StaticUpcast};

use rayon::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader};
use std::rc::Rc;

use rpfm_error::{ErrorKind, Result};
use rpfm_macros::*;

use rpfm_lib::GAME_SELECTED;
use rpfm_lib::packfile::PathType;
use rpfm_lib::packfile::packedfile::PackedFile;
use rpfm_lib::packedfile::DecodedPackedFile;
use rpfm_lib::packedfile::table::{db::DB, DecodedData, loc::Loc};
use rpfm_lib::SCHEMA;
use rpfm_lib::schema::{Definition, FieldType};

use crate::AppUI;
use crate::CENTRAL_COMMAND;
use crate::communications::{CentralCommand, Command, Response, THREADS_COMMUNICATION_ERROR};
use crate::dependencies_ui::DependenciesUI;
use crate::diagnostics_ui::DiagnosticsUI;
use crate::ffi::kmessage_widget_close_safe;
use crate::global_search_ui::GlobalSearchUI;
use crate::packedfile_views::DataSource;
use crate::pack_tree::{PackTree, TreePathType, TreeViewOperation};
use crate::packfile_contents_ui::PackFileContentsUI;
use crate::utils::*;
use crate::UI_STATE;
use crate::views::table::utils::clean_column_names;

/// Macro to automatically generate get code from all sources, because it gets big really fast.
macro_rules! get_data_from_all_sources {
    ($funtion:ident, $data:ident, $processed_data:ident) => (
        if let Some(data) = $data.get_mut(&DataSource::GameFiles) {
            Self::$funtion(data, &mut $processed_data)?;
        }
        if let Some(data) = $data.get_mut(&DataSource::ParentFiles) {
            Self::$funtion(data, &mut $processed_data)?;
        }
        if let Some(data) = $data.get_mut(&DataSource::PackFile) {
            Self::$funtion(data, &mut $processed_data)?;
        }
    );
    ($funtion:ident, $data:ident, $processed_data:ident, $use_source:expr) => (
        if let Some(data) = $data.get_mut(&DataSource::GameFiles) {
            Self::$funtion(data, &mut $processed_data, DataSource::GameFiles)?;
        }
        if let Some(data) = $data.get_mut(&DataSource::ParentFiles) {
            Self::$funtion(data, &mut $processed_data, DataSource::ParentFiles)?;
        }
        if let Some(data) = $data.get_mut(&DataSource::PackFile) {
            Self::$funtion(data, &mut $processed_data, DataSource::PackFile)?;
        }
    );
}

pub mod faction_painter;
pub mod unit_editor;

//-------------------------------------------------------------------------------//
//                              Enums & Structs
//-------------------------------------------------------------------------------//

/// This struct represents the common content and behavior shared across Tools.
#[derive(GetRef, GetRefMut)]
pub struct Tool {

    /// Main widget of the tool, built from a Template. Usually, the dialog.
    main_widget: QBox<QWidget>,

    /// Paths which the tool requires data from.
    used_paths: Vec<PathType>,

    /// Stored PackedFiles, for quickly pulling data from them if needed.
    packed_files: Rc<RefCell<HashMap<DataSource, HashMap<Vec<String>, PackedFile>>>>,

    /// KMessageWidget to display messages to the user in the Tool.
    message_widget: QPtr<QWidget>,

    /// Bottom buttonbox of the Tool.
    button_box: QPtr<QDialogButtonBox>,
}

//-------------------------------------------------------------------------------//
//                             Implementations
//-------------------------------------------------------------------------------//

/// Implementation of `Tool`.
impl Tool {

    /// This function creates a Tool with the data it needs.
    pub unsafe fn new(parent: impl CastInto<Ptr<QWidget>>, paths: &[PathType], tool_supported_games: &[&str], template_path: &str) -> Result<Self> {

        // First, some checks to ensure we can actually open a tool.
        // The requeriments for all tools are:
        // - Game Selected supported by the specific tool we want to open.
        // - Schema for the Game Selected.
        // - Dependencies cache generated and up-to-date.
        //
        // These requeriments are common for all tools, so they're checked here.
        if tool_supported_games.iter().all(|x| *x != GAME_SELECTED.read().unwrap().get_game_key_name()) {
            return Err(ErrorKind::GameSelectedNotSupportedForTool.into());
        }

        if SCHEMA.read().unwrap().is_none() {
            return Err(ErrorKind::SchemaNotFound.into());
        }

        let receiver = CENTRAL_COMMAND.send_background(Command::IsThereADependencyDatabase);
        let response = CentralCommand::recv(&receiver);
        match response {
            Response::Bool(it_is) => if !it_is { return Err(ErrorKind::DependenciesCacheNotGeneratedorOutOfDate.into()); },
            _ => panic!("{}{:?}", THREADS_COMMUNICATION_ERROR, response),
        }

        // Load the UI Template.
        let mut data = vec!();
        let mut file = BufReader::new(File::open(template_path)?);
        file.read_to_end(&mut data)?;

        let ui_loader = QUiLoader::new_0a();
        let main_widget = ui_loader.load_bytes_with_parent(&data, parent);

        // Get the common widgets for all tools.
        let message_widget: QPtr<QWidget> = Self::find_widget_no_tool(&main_widget.static_upcast(), "message_widget")?;
        let button_box: QPtr<QDialogButtonBox> = Self::find_widget_no_tool(&main_widget.static_upcast(), "button_box")?;

        // Close the message widget, as by default is open.
        kmessage_widget_close_safe(&message_widget.as_ptr());

        // Dedup the paths.
        let used_paths = PathType::dedup(paths);

        // Then, build the tool.
        Ok(Self{
            main_widget,
            used_paths,
            packed_files: Rc::new(RefCell::new(HashMap::new())),
            message_widget,
            button_box,
        })
    }

    /// This function returns the main widget casted as a QDialog, which should be the type of the widget defined in the UI Template.
    pub unsafe fn get_ref_dialog(&self) -> qt_core::QPtr<QDialog> {
        self.main_widget.static_downcast::<QDialog>()
    }

    /// This function sets the title of the Tool's window.
    pub unsafe fn set_title(&self, title: &str) {
        self.get_ref_dialog().set_window_title(&QString::from_std_str(title));
    }

    /// This function saves the tools data to the PackFile, in a common way across all tools, and triggers the relevant UI updates.
    pub unsafe fn save(
        &self,
        app_ui: &Rc<AppUI>,
        pack_file_contents_ui: &Rc<PackFileContentsUI>,
        global_search_ui: &Rc<GlobalSearchUI>,
        diagnostics_ui: &Rc<DiagnosticsUI>,
        dependencies_ui: &Rc<DependenciesUI>,
        packed_files: &[PackedFile]
    ) -> Result<()> {

        // First, check if we actually have an open PackFile. If we don't have one, we need to generate it and promp a save.
        if pack_file_contents_ui.packfile_contents_tree_model.row_count_0a() == 0 {
            AppUI::new_packfile(app_ui, pack_file_contents_ui, global_search_ui, diagnostics_ui, dependencies_ui);
        }

        // If either the PackFile exists, or it didn't but now it does, then me need to check, file by file, to see if we can merge
        // the data edited by the tool into the current files, or we have to insert the files as new.
        let receiver = CENTRAL_COMMAND.send_background(Command::SavePackedFilesToPackFileAndClean(packed_files.to_vec()));
        let response = CentralCommand::recv(&receiver);
        match response {
            Response::VecVecStringVecVecString((paths_to_add, paths_to_delete)) => {

                // Get the list of paths to add, removing those we "replaced".
                let paths_to_add = paths_to_add.iter().map(|x| TreePathType::File(x.to_vec())).collect::<Vec<TreePathType>>();
                let paths_to_delete = paths_to_delete.iter().map(|x| TreePathType::File(x.to_vec())).collect::<Vec<TreePathType>>();

                // Update the TreeView.
                pack_file_contents_ui.packfile_contents_tree_view.update_treeview(true, TreeViewOperation::Add(paths_to_add.to_vec()), DataSource::PackFile);
                pack_file_contents_ui.packfile_contents_tree_view.update_treeview(true, TreeViewOperation::MarkAlwaysModified(paths_to_add), DataSource::PackFile);
                pack_file_contents_ui.packfile_contents_tree_view.update_treeview(true, TreeViewOperation::Delete(paths_to_delete), DataSource::PackFile);
                UI_STATE.set_is_modified(true, app_ui, pack_file_contents_ui);
            }

            Response::Error(error) => return Err(error),
            _ => panic!("{}{:?}", THREADS_COMMUNICATION_ERROR, response)
        }

        // Reload the paths edited by the tool whose views are open.
        self.reload_used_paths(app_ui, pack_file_contents_ui);
        Ok(())
    }

    /// This function takes care of backing up the open files we need for the tool, so we always have their latest data.
    ///
    /// Really... we backup everything. To be optimized in the future for backing up only specific PathTypes.
    pub unsafe fn backup_used_paths(&self, app_ui: &Rc<AppUI>, pack_file_contents_ui: &Rc<PackFileContentsUI>) -> Result<()> {
        AppUI::back_to_back_end_all(app_ui, pack_file_contents_ui)
    }

    /// This function takes care of reloading open files we have edited with the tool.
    ///
    /// If a view fails to reload, it just closes it. No view should ever fail, but... we're not in a sunshine and rainbow's world.
    pub unsafe fn reload_used_paths(&self, app_ui: &Rc<AppUI>, pack_file_contents_ui: &Rc<PackFileContentsUI>) {
        let mut paths_to_purge = vec![];
        for path_type in &self.used_paths {
            match path_type {
                PathType::File(ref path) => {
                    if let Some(packed_file_view) = UI_STATE.set_open_packedfiles().iter_mut().find(|x| *x.get_ref_path() == *path && x.get_data_source() == DataSource::PackFile) {
                        if packed_file_view.reload(path, pack_file_contents_ui).is_err() {
                            paths_to_purge.push(path.to_vec());
                        }
                    }
                },
                PathType::Folder(ref path) => {
                    for packed_file_view in UI_STATE.set_open_packedfiles().iter_mut().filter(|x| x.get_ref_path().starts_with(path) && x.get_ref_path().len() > path.len() && x.get_data_source() == DataSource::PackFile) {
                        if packed_file_view.reload(path, pack_file_contents_ui).is_err() {
                            paths_to_purge.push(path.to_vec());
                        }
                    }
                },
                PathType::PackFile => {
                    for packed_file_view in &mut *UI_STATE.set_open_packedfiles() {
                        if packed_file_view.reload(&packed_file_view.get_path(), pack_file_contents_ui).is_err() {
                            paths_to_purge.push(packed_file_view.get_path().to_vec());
                        }
                    }
                },
                PathType::None => unimplemented!(),
            }
        }

        for path in &paths_to_purge {
            let _ = AppUI::purge_that_one_specifically(app_ui, pack_file_contents_ui, path, DataSource::PackFile, false);
        }
    }

    /// This function returns the data on a row's column, or an error if said column doesn't exist.
    ///
    /// It's an utility function for tools.
    pub fn get_row_by_column_index(row: &[DecodedData], index: usize) -> Result<&DecodedData> {
        row.get(index).ok_or_else(|| ErrorKind::ToolTableColumnNotFound.into())
    }

    /// This function returns the a widget from the view if it exits, and an error if it doesn't.
    pub unsafe fn find_widget<T: StaticUpcast<QWidget> + cpp_core::StaticUpcast<qt_core::QObject>>(&self, widget_name: &str) -> Result<QPtr<T>>
        where QObject: DynamicCast<T> {
        Self::find_widget_no_tool(&self.get_ref_main_widget().static_upcast(), widget_name)
    }

    /// This function returns the a widget from the view if it exits, and an error if it doesn't.
    ///
    /// For local use when no Tool has yet been created.
    unsafe fn find_widget_no_tool<T: StaticUpcast<QWidget> + cpp_core::StaticUpcast<qt_core::QObject>>(main_widget: &QPtr<QWidget>, widget_name: &str) -> Result<QPtr<T>>
        where QObject: DynamicCast<T> {
        main_widget.find_child(widget_name).map_err(|_| ErrorKind::TemplateUIWidgetNotFound(widget_name.to_owned()).into())
    }

    /// This function gets the data needed for the tool from a DB table in a generic way.
    ///
    /// Useful for tables of which we can modify any of its columns. If you need to only change some of their columns, use a custom function.
    unsafe fn get_table_data(
        data: &mut HashMap<Vec<String>, PackedFile>,
        processed_data: &mut HashMap<String, HashMap<String, String>>,
        table_name: &str,
        key_name: &str,
        linked_table: Option<(String, String)>,
    ) -> Result<()> {

        // Prepare all the different name variations we need.
        let table_name_end_underscore = format!("{}_", table_name);
        let table_name_end_tables = format!("{}_tables", table_name);
        let definition_key = format!("{}_definition", table_name);
        let linked_key_name = linked_table.map(|(table, column)| format!("{}_{}", table, column));

        for (path, packed_file) in data.iter_mut() {
            if path.len() > 2 && path[0].to_lowercase() == "db" && path[1] == table_name_end_tables {
                if let Ok(DecodedPackedFile::DB(table)) = packed_file.decode_return_ref() {

                    // First, get the key column.
                    let key_column = table.get_column_position_by_name(key_name)?;
                    let fields = table.get_ref_definition().get_fields_processed();

                    // Depending of if it's a linked table or not, we get it as full new entries, or filling existing entries.
                    match linked_key_name {
                        Some(ref linked_key_name) => {

                            // If it's a linked table, we iterate over our current data, and for each of our entries, find the equivalent entry on this table.
                            // If no link is found, skip the entry.
                            for values in processed_data.values_mut() {
                                let linked_key = if let Some(linked_key) = values.get(linked_key_name) { linked_key.to_owned() } else { continue };
                                let row = table.get_ref_table_data().par_iter().find_first(|row| {
                                    match Tool::get_row_by_column_index(row, key_column) {
                                        Ok(data) => match data {
                                            DecodedData::StringU8(data) |
                                            DecodedData::StringU16(data) |
                                            DecodedData::OptionalStringU8(data) |
                                            DecodedData::OptionalStringU16(data) => data == &linked_key,
                                            _ => false,
                                        },
                                        Err(_) => false,
                                    }
                                });

                                // If it has data, add it of the rest of the fields.
                                if let Some(row) = row {
                                    for (index, cell) in row.iter().enumerate() {
                                        let cell_data = cell.data_to_string();
                                        let cell_name = table_name_end_underscore.to_owned() + fields[index].get_name();
                                        values.insert(cell_name, cell_data);
                                    }
                                }

                                // Store the definition, so we can re-use it later to recreate the table.
                                if values.get(&definition_key).is_none() {
                                    let definition = serde_json::to_string(table.get_ref_definition())?;
                                    values.insert(definition_key.to_owned(), definition);
                                }
                            }
                        },
                        None => {

                            // If it's not a linked table... just add each row to our data.
                            for row in table.get_ref_table_data() {
                                let mut data = HashMap::new();
                                let key = Tool::get_row_by_column_index(row, key_column)?.data_to_string();

                                for (index, cell) in row.iter().enumerate() {
                                    let cell_data = cell.data_to_string();
                                    let cell_name = table_name_end_underscore.to_owned() + fields[index].get_name();
                                    data.insert(cell_name, cell_data);
                                }

                                // Store the definition, so we can re-use it later to recreate the table.
                                if data.get(&definition_key).is_none() {
                                    let definition = serde_json::to_string(table.get_ref_definition())?;
                                    data.insert(definition_key.to_owned(), definition);
                                }

                                processed_data.insert(key.to_owned(), data);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// This function takes care of saving a DB table in a generic way into a PackedFile.
    ///
    /// Useful for tables of which we can modify any of its columns. If you need to only change some of their columns, use a custom function.
    ///
    /// TODO: Make this work for tables that admit multiple rows per relation.
    unsafe fn save_table_data(&self, data: &[HashMap<String, String>], table_name: &str, file_name: &str) -> Result<PackedFile> {

        // Prepare all the different name variations we need.
        let table_name_end_underscore = format!("{}_", table_name);
        let table_name_end_tables = format!("{}_tables", table_name);
        let definition_key = format!("{}_definition", table_name);

        // Get the table definition from its first entry, if there is one.
        if let Some(first) = data.first() {
            if let Some(definition) = first.get(&definition_key) {
                let mut table = DB::new(&table_name_end_tables, None, &serde_json::from_str(definition)?);

                // Generate the table's data from empty rows + our data.
                let table_fields = table.get_ref_definition().get_fields_processed();
                let table_data = data.par_iter()
                    .filter_map(|row_data| {
                        let mut row = table.get_new_row();
                        for (index, field) in table_fields.iter().enumerate() {

                            // If the field is a reference to another, try to get the source instead. Only use the current table's value if that fails.
                            let field_source_table_name = match field.get_is_reference() {
                                Some((source_table, _)) => source_table.to_owned() + "_",
                                None => table_name_end_underscore.to_owned(),
                            };

                            // For each field, check if we have data for it, and replace the "empty" row's data with it. Skip invalid values
                            if let Some(value) = row_data.get(&format!("{}{}", field_source_table_name, field.get_name())) {
                                row[index] = match field.get_field_type() {
                                    FieldType::Boolean => DecodedData::Boolean(value.parse().ok()?),
                                    FieldType::F32 => DecodedData::F32(value.parse().ok()?),
                                    FieldType::I16 => DecodedData::I16(value.parse().ok()?),
                                    FieldType::I32 => DecodedData::I32(value.parse().ok()?),
                                    FieldType::I64 => DecodedData::I64(value.parse().ok()?),
                                    FieldType::StringU8 => DecodedData::StringU8(value.parse().ok()?),
                                    FieldType::StringU16 => DecodedData::StringU16(value.parse().ok()?),
                                    FieldType::OptionalStringU8 => DecodedData::OptionalStringU8(value.parse().ok()?),
                                    FieldType::OptionalStringU16 => DecodedData::OptionalStringU16(value.parse().ok()?),
                                    _ => unimplemented!()
                                };
                            }
                        }

                        Some(row)
                    }).collect::<Vec<Vec<DecodedData>>>();

                table.set_table_data(&table_data)?;
                let path = vec!["db".to_owned(), table_name_end_tables.to_owned(), file_name.to_owned()];
                Ok(PackedFile::new_from_decoded(&DecodedPackedFile::DB(table), &path))
            } else { Err(ErrorKind::Impossibru.into()) }
        } else { Err(ErrorKind::Impossibru.into()) }
    }

    /// This function gets the data needed for the tool from the locs in a generic way.
    unsafe fn get_loc_data(
        data: &mut HashMap<Vec<String>, PackedFile>,
        processed_data: &mut HashMap<String, HashMap<String, String>>,
        loc_keys: &[(&str, &str)],
    ) -> Result<()> {

        for (path, packed_file) in data.iter_mut() {
            if path.len() > 1 && path[0].to_lowercase() == "text" && path.last().unwrap().ends_with(".loc") {
                if let Ok(DecodedPackedFile::Loc(table)) = packed_file.decode_return_ref() {
                    let table = table.get_ref_table_data().par_iter()
                        .filter_map(|row| {
                            let key = if let DecodedData::StringU16(key) = &row[0] { key.to_owned() } else { None? };
                            let value = if let DecodedData::StringU16(value) = &row[1] { value.to_owned() } else { None? };
                            Some((key, value))
                        }).collect::<HashMap<String, String>>();

                    // For each entry on our list, check the provided loc keys we expect.
                    //
                    // TODO: Make this work with multi-key columns.
                    for values in processed_data.values_mut() {
                        let loc_keys = loc_keys.iter()
                            .filter_map(|(table_and_column, key)|
                                Some((*table_and_column, format!("{}_{}", table_and_column, values.get(*key)?)))
                            ).collect::<Vec<(&str, String)>>();

                        for (partial_key, full_key) in loc_keys {
                            if let Some(value) = table.get(&full_key) {
                                values.insert(format!("loc_{}", partial_key), value.to_owned());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// This function takes care of saving all the loc-related data in a generic way into a PackedFile.
    unsafe fn save_loc_data(
        &self,
        data: &[HashMap<String, String>],
        file_name: &str,
        loc_keys: &[(&str, &str)]
    ) -> Result<PackedFile> {
        if let Some(schema) = &*SCHEMA.read().unwrap() {
            if let Ok(definition) = schema.get_ref_last_definition_loc() {
                let mut table = Loc::new(&definition);

                // Generate the table's data from empty rows + our data.
                let table_data = data.par_iter()
                    .filter_map(|row_data| {
                        let mut rows = vec![];

                        for (key, value) in row_data {
                            let loc_keys = loc_keys.iter().filter_map(|(table_and_column, key)| {
                                Some((*table_and_column, format!("{}_{}", table_and_column, row_data.get(key.to_owned())?)))
                            }).collect::<Vec<(&str, String)>>();

                            if key.starts_with("loc_") {
                                let mut key = key.to_owned();
                                key.remove(0);
                                key.remove(0);
                                key.remove(0);
                                key.remove(0);

                                if let Some(loc_key) = loc_keys.iter().find_map(|(tool_key, loc_key)| if *tool_key == &key { Some(loc_key) } else { None }) {

                                    let mut row = table.get_new_row();
                                    row[0] = DecodedData::StringU16(loc_key.to_owned());
                                    row[1] = DecodedData::StringU16(value.to_owned());
                                    rows.push(row);
                                }
                            }
                        }

                        Some(rows)
                    })
                    .flatten()
                    .collect::<Vec<Vec<DecodedData>>>();

                table.set_table_data(&table_data)?;
                let path = vec!["text".to_owned(), "db".to_owned(), file_name.to_owned()];
                Ok(PackedFile::new_from_decoded(&DecodedPackedFile::Loc(table), &path))
            } else { Err(ErrorKind::Impossibru.into()) }
        } else { Err(ErrorKind::SchemaNotFound.into()) }
    }

    /// This function is an utility function to get the most relevant file for a tool from the dependencies.
    unsafe fn get_most_relevant_file(data: &HashMap<DataSource, HashMap<Vec<String>, PackedFile>>, path: &[String]) -> Option<PackedFile> {
        if let Some(data) = data.get(&DataSource::PackFile) {
            if let Some(packed_file) = data.get(path) {
                return Some(packed_file.to_owned());
            }
        }

        if let Some(data) = data.get(&DataSource::ParentFiles) {
            if let Some(packed_file) = data.get(path) {
                return Some(packed_file.to_owned());
            }
        }

        if let Some(data) = data.get(&DataSource::GameFiles) {
            if let Some(packed_file) = data.get(path) {
                return Some(packed_file.to_owned());
            }
        }

        None
    }

    //-------------------------------------------------------------------------------//
    //                                Data loaders
    //-------------------------------------------------------------------------------//

    /// This function takes care of loading on-mass data from a specific table, including label name,
    /// dependency data, default values, and current data, into the detailed view.
    ///
    /// Fields that fail to load due to missing widgets are returned on error.
    unsafe fn load_definition_to_detailed_view_editor(&self, data: &HashMap<String, String>, table_name: &str, fields_to_ignore: &[&str]) -> Result<()> {

        let mut load_field_errors = vec![];

        // Try to get the table's definition.
        let definition_name = format!("{}_definition", table_name);
        match data.get(&definition_name) {
            Some(definition) => {
                let definition: Definition = serde_json::from_str(&definition).unwrap();
                definition.get_fields_processed()
                    .iter()
                    .filter(|field| !fields_to_ignore.contains(&field.get_name()))
                    .for_each(|field| {

                        // First, load the field's label. If it uses a custom one, set it after this function.
                        let label_name = format!("{}_{}_label", table_name, field.get_name());
                        let label_widget: Result<QPtr<QLabel>> = self.find_widget(&label_name);
                        match label_widget {
                            Ok(label) => label.set_text(&QString::from_std_str(&clean_column_names(field.get_name()))),
                            Err(_) => load_field_errors.push(label_name),
                        };

                        // Next, setup the data in the widget's depending on the type of the data.
                        match field.get_field_type() {
                            FieldType::Boolean => {
                                let widget_name = format!("{}_{}_checkbox", table_name, field.get_name());
                                let widget: Result<QPtr<QCheckBox>> = self.find_widget(&widget_name);
                                match widget {
                                    Ok(widget) => {

                                        // Check if we have data for the widget. If not, fill it with default data
                                        let field_key_name = format!("{}_{}", table_name, field.get_name());
                                        match data.get(&field_key_name) {
                                            Some(data) => {
                                                if let Ok(value) = data.parse::<bool>() {
                                                    widget.set_checked(value);
                                                }
                                            },
                                            None => {
                                                if let Some(default_value) = field.get_default_value() {
                                                    if let Ok(value) = default_value.parse::<bool>() {
                                                        widget.set_checked(value);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => load_field_errors.push(widget_name),
                                };
                            },
                            FieldType::I16 |
                            FieldType::I32 |
                            FieldType::I64 => {
                                let widget_name = format!("{}_{}_spinbox", table_name, field.get_name());
                                let widget: Result<QPtr<QSpinBox>> = self.find_widget(&widget_name);
                                match widget {
                                    Ok(widget) => {

                                        // Check if we have data for the widget. If not, fill it with default data
                                        let field_key_name = format!("{}_{}", table_name, field.get_name());
                                        match data.get(&field_key_name) {
                                            Some(data) => {
                                                if let Ok(value) = data.parse::<i32>() {
                                                    widget.set_value(value);
                                                }
                                            },
                                            None => {
                                                if let Some(default_value) = field.get_default_value() {
                                                    if let Ok(value) = default_value.parse::<i32>() {
                                                        widget.set_value(value);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => load_field_errors.push(widget_name),
                                };
                            },
                            FieldType::F32 => {
                                let widget_name = format!("{}_{}_double_spinbox", table_name, field.get_name());
                                let widget: Result<QPtr<QDoubleSpinBox>> = self.find_widget(&widget_name);
                                match widget {
                                    Ok(widget) => {

                                        // Check if we have data for the widget. If not, fill it with default data
                                        let field_key_name = format!("{}_{}", table_name, field.get_name());
                                        match data.get(&field_key_name) {
                                            Some(data) => {
                                                if let Ok(value) = data.parse::<f64>() {
                                                    widget.set_value(value);
                                                }
                                            },
                                            None => {
                                                if let Some(default_value) = field.get_default_value() {
                                                    if let Ok(value) = default_value.parse::<f64>() {
                                                        widget.set_value(value);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => load_field_errors.push(widget_name),
                                };
                            },
                            FieldType::StringU8 |
                            FieldType::StringU16 |
                            FieldType::OptionalStringU8 |
                            FieldType::OptionalStringU16 => {
                                let widget_name = format!("{}_{}_line_edit", table_name, field.get_name());
                                let widget: Result<QPtr<QLineEdit>> = self.find_widget(&widget_name);
                                match widget {
                                    Ok(widget) => {

                                        // Check if we have data for the widget. If not, fill it with default data
                                        let field_key_name = format!("{}_{}", table_name, field.get_name());
                                        match data.get(&field_key_name) {
                                            Some(data) => widget.set_text(&QString::from_std_str(data)),
                                            None => {
                                                if let Some(default_value) = field.get_default_value() {
                                                    widget.set_text(&QString::from_std_str(default_value));
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => load_field_errors.push(widget_name),
                                };
                            },
                            _ => unimplemented!()
                        };
                    }
                );
            }

            // If we fail to find a definition... tbd.
            None => {}
        }

        if !load_field_errors.is_empty() {
            Err(ErrorKind::TemplateUIWidgetNotFound(load_field_errors.join(", ")).into())
        } else {
            Ok(())
        }
    }

    /// This function tries to load data from a bool value into a QCheckBox.
    unsafe fn load_field_to_detailed_view_editor_bool(&self, processed_data: &HashMap<String, String>, field_editor: &QPtr<QCheckBox>, field_name: &str) {
        match processed_data.get(field_name) {
            Some(data) => match data.parse::<bool>() {
                Ok(data) => field_editor.set_checked(data),
                Err(error) => show_message_warning(&self.message_widget, error.to_string()),
            }
            None => field_editor.set_checked(false),
        }
    }

    /// This function tries to load data from a i32 value into a QSpinBox.
    unsafe fn load_field_to_detailed_view_editor_i32(&self, processed_data: &HashMap<String, String>, field_editor: &QPtr<QSpinBox>, field_name: &str) {
        match processed_data.get(field_name) {
            Some(data) => match data.parse::<i32>() {
                Ok(data) => field_editor.set_value(data.into()),
                Err(error) => {
                    field_editor.set_value(0);
                    show_message_warning(&self.message_widget, error.to_string());
                }
            }
            None => field_editor.set_value(0),
        }
    }

    /// This function tries to load data from a f32 value into a QDoubleSpinBox.
    unsafe fn load_field_to_detailed_view_editor_f32(&self, processed_data: &HashMap<String, String>, field_editor: &QPtr<QDoubleSpinBox>, field_name: &str) {
        match processed_data.get(field_name) {
            Some(data) => match data.parse::<f32>() {
                Ok(data) => field_editor.set_value(data.into()),
                Err(error) => {
                    field_editor.set_value(0.0);
                    show_message_warning(&self.message_widget, error.to_string());
                }
            }
            None => field_editor.set_value(0.0),
        }
    }

    /// This function tries to load data from a string into a QLineEdit.
    unsafe fn load_field_to_detailed_view_editor_string_short(&self, processed_data: &HashMap<String, String>, field_editor: &QPtr<QLineEdit>, field_name: &str) {
        match processed_data.get(field_name) {
            Some(data) => field_editor.set_text(&QString::from_std_str(data)),
            None => field_editor.set_text(&QString::new()),
        }
    }

    /// This function tries to load data from a long string into a QTextEdit.
    unsafe fn load_field_to_detailed_view_editor_string_long(&self, processed_data: &HashMap<String, String>, field_editor: &QPtr<QTextEdit>, field_name: &str) {
        match processed_data.get(field_name) {
            Some(data) => field_editor.set_text(&QString::from_std_str(data)),
            None => field_editor.set_text(&QString::new()),
        }
    }

    //-------------------------------------------------------------------------------//
    //                               Data retrievers
    //-------------------------------------------------------------------------------//
}

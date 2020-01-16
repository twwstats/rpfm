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
This module contains the code to build/use the ***Shortcuts*** UI.
!*/

use qt_widgets::dialog::Dialog;
use qt_widgets::dialog_button_box;
use qt_widgets::dialog_button_box::DialogButtonBox;
use qt_widgets::header_view::ResizeMode;
use qt_widgets::push_button::PushButton;
use qt_widgets::tree_view::TreeView;
use qt_widgets::widget::Widget;

use qt_gui::list::ListStandardItemMutPtr;
use qt_gui::standard_item::StandardItem;
use qt_gui::standard_item_model::StandardItemModel;

use qt_core::abstract_item_model::AbstractItemModel;
use qt_core::object::Object;
use qt_core::sort_filter_proxy_model::SortFilterProxyModel;
use qt_core::qt::Orientation;
use qt_core::variant::Variant;

use crate::QString;
use crate::ffi::new_treeview_filter;
use crate::locale::{qtr, tr};
use crate::ui_state::shortcuts::Shortcuts;
use crate::utils::create_grid_layout_unsafe;
use crate::UI_STATE;
use self::slots::ShortcutsUISlots;

mod connections;
mod slots;

//-------------------------------------------------------------------------------//
//                              Enums & Structs
//-------------------------------------------------------------------------------//

/// This struct holds all the widgets used in the Shortcuts Window.
#[derive(Clone)]
pub struct ShortcutsUI {
    dialog: *mut Dialog,

    shortcuts_table: *mut TreeView,
    shortcuts_model: *mut StandardItemModel,
    shortcuts_filter: *mut SortFilterProxyModel,

    restore_default_button: *mut PushButton,
    cancel_button: *mut PushButton,
    accept_button: *mut PushButton,
}

//-------------------------------------------------------------------------------//
//                             Implementations
//-------------------------------------------------------------------------------//

/// Implementation of `ShortcutsUI`.
impl ShortcutsUI {

    /// This function creates a ***ShortcutsUI*** dialog, execute it, and returns a new `Shortcuts`, or `None` if you close/cancel the dialog.
    pub fn new(parent: *mut Widget) -> Option<Shortcuts> {
        let mut ui = Self::new_with_parent(parent);
        let slots = ShortcutsUISlots::new(&ui);
        connections::set_connections(&ui, &slots);
        ui.load(&UI_STATE.get_shortcuts());

        if unsafe { ui.dialog.as_mut().unwrap().exec() == 1 } { Some(ui.save()) }
        else { None }
    }

    /// This function creates the entire `ShortcutsUI` Window and shows it.
    pub fn new_with_parent(parent: *mut Widget) -> Self {

        // Create the Shortcuts Dialog and configure it.
        let mut dialog = unsafe { Dialog::new_unsafe(parent) };
        dialog.set_window_title(&qtr("shortcut_title"));
        dialog.set_modal(true);
        dialog.resize((1100, 700));

        // Create the main Grid and add the shortcuts TreeView.
        let main_grid = create_grid_layout_unsafe(dialog.as_mut_ptr() as *mut Widget);
        let mut shortcuts_table = TreeView::new();
        let shortcuts_filter = unsafe { new_treeview_filter(shortcuts_table.as_mut_ptr() as *mut Object) };
        let shortcuts_model = StandardItemModel::new(()).into_raw();

        unsafe { shortcuts_table.set_model(shortcuts_filter as *mut AbstractItemModel); }
        unsafe { shortcuts_filter.as_mut().unwrap().set_source_model(shortcuts_model as *mut AbstractItemModel); }

        shortcuts_table.set_sorting_enabled(false);
        unsafe { shortcuts_table.header().as_mut().unwrap().set_stretch_last_section(true); }
        unsafe { main_grid.as_mut().unwrap().add_widget((shortcuts_table.as_mut_ptr() as *mut Widget, 0, 0, 1, 1)); }

        // Create the bottom buttons and add them to the Dialog.
        let mut button_box = DialogButtonBox::new(());
        let restore_default_button = button_box.add_button(dialog_button_box::StandardButton::RestoreDefaults);
        let cancel_button = button_box.add_button(dialog_button_box::StandardButton::Cancel);
        let accept_button = button_box.add_button(dialog_button_box::StandardButton::Save);
        unsafe { main_grid.as_mut().unwrap().add_widget((button_box.into_raw() as *mut Widget, 1, 0, 1, 1)); }

        Self {
            dialog: dialog.into_raw(),
            shortcuts_table: shortcuts_table.into_raw(),
            shortcuts_model,
            shortcuts_filter,
            restore_default_button,
            cancel_button,
            accept_button,
        }
    }

    /// This function loads the data from the `Shortcuts` struct to the `ShortcutsUI`.
    pub fn load(&mut self, shortcuts: &Shortcuts) {
        let shortcuts_model = unsafe { self.shortcuts_model.as_mut().unwrap() };
        let shortcuts_table = unsafe { self.shortcuts_table.as_mut().unwrap() };

        // Clear all the models, just in case this is a restore default operation.
        shortcuts_model.clear();

        // Just add in mass the shortcuts to the Model, separated in sections.
        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("menu_bar_packfile_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.menu_bar_packfile.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("menu_bar_mymod_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.menu_bar_mymod.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("menu_bar_game_selected_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.menu_bar_game_selected.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("menu_bar_about_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.menu_bar_about.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("packfile_contents_tree_view_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.packfile_contents_tree_view.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("packed_file_table_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.packed_file_table.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        {
            let mut menu_bar_packfile_parent = ListStandardItemMutPtr::new(());
            let mut section = StandardItem::new(());
            let mut fill1 = StandardItem::new(());
            section.set_text(&qtr("packed_file_decoder_section"));
            section.set_editable(false);
            fill1.set_editable(false);
            for (key, value) in shortcuts.packed_file_decoder.iter() {
                let mut row_list = ListStandardItemMutPtr::new(());
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(key)).into_raw()); }
                unsafe { row_list.append_unsafe(&StandardItem::new(&QString::from_std_str(value)).into_raw()); }
                unsafe { row_list.at(0).as_mut().unwrap().set_editable(false); }
                section.append_row(&row_list);
            }

            unsafe { menu_bar_packfile_parent.append_unsafe(&section.into_raw()); }
            unsafe { menu_bar_packfile_parent.append_unsafe(&fill1.into_raw()); }
            shortcuts_model.append_row(&menu_bar_packfile_parent);
        }

        // Rename the columns and expand all.
        shortcuts_model.set_header_data((0, Orientation::Horizontal, &Variant::new0(&qtr("shortcut_section_action"))));
        shortcuts_model.set_header_data((1, Orientation::Horizontal, &Variant::new0(&qtr("shortcut_text"))));
        shortcuts_table.expand_all();
        unsafe { shortcuts_table.header().as_mut().unwrap().resize_sections(ResizeMode::ResizeToContents); }
    }

    /// This function gets the data from the `ShortcutsUI` and returns a `Shortcuts` struct with that data in it.
    pub fn save(&self) -> Shortcuts {

        // Create a new Shortcuts struct to populate it wit the contents of the model.
        let mut shortcuts = Shortcuts::new();
        let shortcuts_model = unsafe { self.shortcuts_model.as_ref().unwrap() };
        let root = unsafe { shortcuts_model.invisible_root_item().as_ref().unwrap() };

        let menu_bar_packfile_section_title = tr("menu_bar_packfile_section");
        let menu_bar_mymod_section_title = tr("menu_bar_mymod_section");
        let menu_bar_game_selected_section_title = tr("menu_bar_game_selected_section");
        let menu_bar_about_section_title = tr("menu_bar_about_section");
        let packfile_contents_tree_view_section_title = tr("packfile_contents_tree_view_section");
        let packed_file_table_section_title = tr("packed_file_table_section");
        let packed_file_decoder_section_title = tr("packed_file_decoder_section");

        for index in 0..root.row_count() {
            let section = unsafe { root.child(index).as_ref().unwrap() };
            let section_text = section.text().to_std_string();
            let map = if section_text == menu_bar_packfile_section_title { &mut shortcuts.menu_bar_packfile }
                else if section_text == menu_bar_mymod_section_title { &mut shortcuts.menu_bar_mymod }
                else if section_text == menu_bar_game_selected_section_title { &mut shortcuts.menu_bar_game_selected }
                else if section_text == menu_bar_about_section_title { &mut shortcuts.menu_bar_about }
                else if section_text == packfile_contents_tree_view_section_title { &mut shortcuts.packfile_contents_tree_view }
                else if section_text == packed_file_table_section_title { &mut shortcuts.packed_file_table }
                else if section_text == packed_file_decoder_section_title { &mut shortcuts.packed_file_decoder }
                else { panic!("WTF?!! YOU ARE NOT SUPPOSED TO MANUALLY DO WEIRD STUFF WITH THE RON FILE!!!") };

            for index in 0..section.row_count() {
                let key = unsafe { section.child((index, 0)).as_ref().unwrap().text().to_std_string() };
                let value = unsafe { section.child((index, 1)).as_ref().unwrap().text().to_std_string() };
                map.insert(key, value);
            }
        }

        shortcuts
    }
}

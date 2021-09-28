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
Module with all the code related to `ToolUnitEditorSlots`.
!*/

use qt_core::QBox;
use qt_core::SlotNoArgs;
use qt_core::SlotOfQItemSelectionQItemSelection;

use std::rc::Rc;

use super::*;

//-------------------------------------------------------------------------------//
//                              Enums & Structs
//-------------------------------------------------------------------------------//

/// This struct contains all the slots we need to respond to signals of EVERY widget/action in the `ToolUnitEditor` struct.
///
/// This means everything you can do with the stuff you have in the `ToolUnitEditor` goes here.
pub struct ToolUnitEditorSlots {
    pub delayed_updates: QBox<SlotNoArgs>,
    pub load_data_to_detailed_view: QBox<SlotOfQItemSelectionQItemSelection>,
    pub filter_edited: QBox<SlotNoArgs>,
}

//-------------------------------------------------------------------------------//
//                             Implementations
//-------------------------------------------------------------------------------//

/// Implementation of `ToolUnitEditorSlots`.
impl ToolUnitEditorSlots {

    /// This function creates a new `ToolUnitEditorSlots`.
    pub unsafe fn new(ui: &Rc<ToolUnitEditor>) -> Self {

        let delayed_updates = SlotNoArgs::new(ui.tool.get_ref_main_widget(), clone!(
            ui => move || {
                ui.filter_list();
            }
        ));

        let load_data_to_detailed_view = SlotOfQItemSelectionQItemSelection::new(ui.tool.get_ref_main_widget(), clone!(
            ui => move |after, before| {

                // Save the previous data if needed.
                if before.count_0a() == 1 {
                    let filter_index = before.take_at(0).indexes().take_at(0);
                    let index = ui.get_ref_unit_list_filter().map_to_source(filter_index.as_ref());
                    ui.save_from_detailed_view(index.as_ref());
                }

                // Load the new data.
                if after.count_0a() == 1 {
                    let filter_index = after.take_at(0).indexes().take_at(0);
                    let index = ui.get_ref_unit_list_filter().map_to_source(filter_index.as_ref());
                    ui.load_to_detailed_view(index.as_ref());
                }
            }
        ));

        let filter_edited = SlotNoArgs::new(ui.tool.get_ref_main_widget(), clone!(
            ui => move || {
                ui.start_delayed_updates_timer();
            }
        ));

        ToolUnitEditorSlots {
            delayed_updates,
            load_data_to_detailed_view,
            filter_edited,
        }
    }
}

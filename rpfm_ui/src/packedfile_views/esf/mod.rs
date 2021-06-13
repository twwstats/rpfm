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
Module with all the code for managing the ESF Views.
!*/

use qt_widgets::q_abstract_item_view::SelectionMode;
use qt_widgets::QLineEdit;
use qt_widgets::QPushButton;
use qt_widgets::QGridLayout;
use qt_widgets::QWidget;

use qt_core::ContextMenuPolicy;
use qt_core::QBox;
use qt_core::QPtr;
use qt_core::QTimer;

use qt_core::QSortFilterProxyModel;
use qt_gui::QStandardItemModel;
use qt_widgets::QTreeView;

use std::sync::{Arc, RwLock};

use rpfm_error::{ErrorKind, Result};

use rpfm_lib::packedfile::esf::ESF;
use rpfm_lib::packedfile::PackedFileType;
use rpfm_lib::packfile::packedfile::PackedFileInfo;

use crate::CENTRAL_COMMAND;
use crate::communications::*;
use crate::ffi::*;
use crate::locale::qtr;
use crate::packedfile_views::PackedFileView;
use crate::packedfile_views::esf::esftree::*;
use crate::utils::create_grid_layout;

use super::{ViewType, View};

mod esftree;

//-------------------------------------------------------------------------------//
//                              Enums & Structs
//-------------------------------------------------------------------------------//

/// This struct contains the view of the ESF PackedFile.
pub struct PackedFileESFView {
    tree_view: QBox<QTreeView>,
    tree_model: QBox<QStandardItemModel>,
    tree_filter: QBox<QSortFilterProxyModel>,

    filter_line_edit: QBox<QLineEdit>,
    filter_autoexpand_matches_button: QBox<QPushButton>,
    filter_case_sensitive_button: QBox<QPushButton>,
    filter_timer_delayed_updates: QBox<QTimer>,

    path: Arc<RwLock<Vec<String>>>,
}

//-------------------------------------------------------------------------------//
//                             Implementations
//-------------------------------------------------------------------------------//

/// Implementation for `PackedFileESFView`.
impl PackedFileESFView {

    /// This function creates a new PackedFileESFView, and sets up his slots and connections.
    pub unsafe fn new_view(
        packed_file_view: &mut PackedFileView,
    ) -> Result<Option<PackedFileInfo>> {

        CENTRAL_COMMAND.send_message_qt(Command::DecodePackedFile(packed_file_view.get_path(), packed_file_view.get_data_source()));
        let response = CENTRAL_COMMAND.recv_message_qt();
        let (data, packed_file_info) = match response {
            Response::ESFPackedFileInfo((data, packed_file_info)) => (data, packed_file_info),
            Response::Error(error) => return Err(error),
            Response::Unknown => return Err(ErrorKind::PackedFileTypeUnknown.into()),
            _ => panic!("{}{:?}", THREADS_COMMUNICATION_ERROR, response),
        };

        // Create the TreeView for the ESF PackedFile.
        let tree_view = QTreeView::new_1a(packed_file_view.get_mut_widget());
        let tree_model = new_packed_file_model_safe();
        let tree_filter = new_treeview_filter_safe(tree_view.static_upcast());
        tree_filter.set_source_model(&tree_model);
        tree_model.set_parent(&tree_view);
        tree_view.set_model(&tree_filter);
        tree_view.set_header_hidden(true);
        tree_view.set_animated(true);
        tree_view.set_uniform_row_heights(true);
        tree_view.set_selection_mode(SelectionMode::ExtendedSelection);
        tree_view.set_context_menu_policy(ContextMenuPolicy::CustomContextMenu);
        tree_view.set_expands_on_double_click(true);
        tree_view.header().set_stretch_last_section(false);

        // Create and configure the widgets to control the `TreeView`s filter.
        let filter_timer_delayed_updates = QTimer::new_1a(packed_file_view.get_mut_widget());
        let filter_line_edit = QLineEdit::from_q_widget(packed_file_view.get_mut_widget());
        let filter_autoexpand_matches_button = QPushButton::from_q_string_q_widget(&qtr("treeview_autoexpand"), packed_file_view.get_mut_widget());
        let filter_case_sensitive_button = QPushButton::from_q_string_q_widget(&qtr("treeview_aai"), packed_file_view.get_mut_widget());
        filter_timer_delayed_updates.set_single_shot(true);
        filter_line_edit.set_placeholder_text(&qtr("packedfile_filter"));
        filter_line_edit.set_clear_button_enabled(true);
        filter_autoexpand_matches_button.set_checkable(true);
        filter_case_sensitive_button.set_checkable(true);

        let node_data_panel = QWidget::new_1a(packed_file_view.get_mut_widget());
        create_grid_layout(node_data_panel.static_upcast());

        // Add everything to the `TreeView`s Layout.
        let layout: QPtr<QGridLayout> = packed_file_view.get_mut_widget().layout().static_downcast();
        layout.add_widget_5a(&tree_view, 0, 0, 1, 2);
        layout.add_widget_5a(&filter_line_edit, 1, 0, 1, 2);
        layout.add_widget_5a(&filter_autoexpand_matches_button, 2, 0, 1, 1);
        layout.add_widget_5a(&filter_case_sensitive_button, 2, 1, 1, 1);
        layout.add_widget_5a(&node_data_panel, 0, 2, 3, 1);

        let view = Self {
            tree_view,
            tree_model,
            tree_filter,

            filter_line_edit,
            filter_autoexpand_matches_button,
            filter_case_sensitive_button,
            filter_timer_delayed_updates,

            path: packed_file_view.get_path_raw()
        };

        view.tree_view.update_treeview(true, ESFTreeViewOperation::Build(data));

        packed_file_view.view = ViewType::Internal(View::ESF(Arc::new(view)));
        packed_file_view.packed_file_type = PackedFileType::ESF;

        Ok(Some(packed_file_info))
    }

    /// This function tries to reload the current view with the provided data.
    pub unsafe fn reload_view(&self, data: &ESF) {
        //let text = serde_json::to_string_pretty(&data).unwrap();
        //self.reload_view(&text);
    }
}

use std::path::PathBuf;

use egui::Widget;
use serde::{Deserialize, Serialize};

use crate::file_entry::FileEntry;

#[derive(Serialize, Deserialize, Clone)]
pub struct Folder {
    pub path: PathBuf,
    pub files: Vec<FileEntry>,
    pub expanded: bool,
    pub to_be_deleted: bool,
}

impl Folder {
    pub fn list_files_ui(
        &mut self,
        ui: &mut egui::Ui,
        search_phrase: &str,
        error_log: &mut Vec<String>,
    ) {
        for file_entry in self.files.iter_mut() {
            if !file_entry.should_be_listed(search_phrase, self.expanded) {
                continue;
            }

            let file_label = file_entry
                .get_file_label()
                .truncate()
                .ui(ui)
                .on_hover_ui(|ui| {
                    ui.label(&file_entry.preview);
                });

            if file_label.clicked() {
                // lazily load the data
                // TODO: if file was updated, it should be reloaded
                file_entry.clicked(&self.path, error_log);
            };

            // toggle plotted or active
            if file_label.secondary_clicked() {
                file_entry.secondary_clicked()
            }
        }
    }
}

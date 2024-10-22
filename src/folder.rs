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
    pub fn new(path: PathBuf, id_counter: &mut usize) -> Self {
        let mut files = vec![];
        if let Ok(read_dir) = path.read_dir() {
            // flatten pulls out the Ok variants of the `read_dir` elements
            for entry in read_dir.into_iter().flatten() {
                // only list csv files
                let filename = entry.file_name().to_string_lossy().into_owned();
                files.push(FileEntry::new(filename, id_counter, entry));
                *id_counter += 1;
            }
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Self {
            path,
            files,
            expanded: true,
            to_be_deleted: false,
        }
    }
    pub fn refresh(&mut self, id_counter: &mut usize) {
        let entries: Vec<_> = self
            .path
            .read_dir()
            .iter_mut()
            .flatten()
            .flatten()
            .collect();
        if entries.is_empty() {
            return;
        }
        let filenames: Vec<_> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        // filter out entries where the file does not appear in the folder anymore
        // (was deleted between last and current loading of folder)
        self.files = self
            .files
            .iter()
            .filter_map(|e| {
                if filenames.contains(&e.filename) {
                    Some(e.to_owned())
                } else {
                    None
                }
            })
            .collect();
        let mut new_files = vec![];
        'outer: for entry in entries {
            // only list csv files
            let filename = entry.file_name().to_string_lossy().into_owned();
            for f in self.files.iter() {
                if f.filename == filename {
                    continue 'outer;
                }
            }
            new_files.push(FileEntry::new(filename, id_counter, entry));
            *id_counter += 1;
        }
        self.files.append(&mut new_files);
        self.files.sort_by(|a, b| a.filename.cmp(&b.filename));
    }

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

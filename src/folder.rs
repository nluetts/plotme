use std::path::PathBuf;

use egui::Widget;
use serde::{Deserialize, Serialize};

use crate::{
    app::FloatInput,
    csvfile::CSVFile,
    file_entry::{FileEntry, FileEntryState},
};

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
                let data_file = CSVFile {
                    filepath: filename.clone().into(),
                    ..Default::default()
                };
                let file_entry = FileEntry {
                    filename,
                    data_file,
                    state: FileEntryState::Idle,
                    scale: FloatInput {
                        input: "1.0".to_string(),
                    },
                    offset: FloatInput {
                        input: "0.0".to_string(),
                    },
                    xoffset: FloatInput {
                        input: "0.0".to_string(),
                    },
                    color: egui::Color32::TRANSPARENT,
                    id: *id_counter,
                    preview: utils::read_first_lines(&entry.path(), 20).unwrap_or_default(),
                };
                *id_counter += 1;
                files.push(file_entry)
            }
        }
        Self {
            path,
            files,
            expanded: true,
            to_be_deleted: false,
        }
    }
    pub fn refresh(&mut self) {}

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

mod utils {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    pub(super) fn read_first_lines(
        filepath: &Path,
        num_lines: usize,
    ) -> Result<String, std::io::Error> {
        let file = File::open(filepath)?;
        let buf_reader = BufReader::new(file);
        let mut lines = String::new();

        for line in buf_reader.lines().take(num_lines).flatten() {
            lines.push_str(&line);
        }

        Ok(lines)
    }
}

use std::path::PathBuf;

use egui::Widget;
use serde::{Deserialize, Serialize};

use crate::{
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
    pub fn list_files_ui(
        &mut self,
        ui: &mut egui::Ui,
        search_phrase: &str,
        error_log: &mut Vec<String>,
    ) {
        for file_entry in self.files.iter_mut() {
            // exclude files which do not match search pattern
            if !(search_phrase
                .split(" ")
                .all(|phrase| file_entry.filename.contains(phrase) && self.expanded)
                // but list spectra that are or were just plotted
                || file_entry.is_plotted()
                || (file_entry.was_just_plotted() && self.expanded))
            {
                continue;
            }

            // style file label, based on currently plotted/active or not
            let file_label = file_entry.get_file_label().truncate().ui(ui);

            if file_label.clicked() {
                // lazily load the data
                // TODO: if file was updated, it should be reloaded
                if file_entry.data_file.data.is_empty()
                    && file_entry.state != FileEntryState::NeedsConfig
                {
                    let filepath = {
                        let path = self.path.clone();
                        path.join(file_entry.filename.clone())
                    };
                    if let Some(csvfile) = CSVFile::new(
                        filepath,
                        file_entry.data_file.xcol,
                        file_entry.data_file.ycol,
                        file_entry.data_file.delimiter,
                        file_entry.data_file.comment_char,
                        file_entry.data_file.skip_header,
                        file_entry.data_file.skip_footer,
                        error_log,
                    ) {
                        // immediately plot freshly loaded csv
                        file_entry.state = FileEntryState::Plotted;
                        file_entry.data_file = csvfile;
                    } else {
                        file_entry.state = FileEntryState::NeedsConfig;
                    }
                } else {
                    file_entry.state = match file_entry.state {
                        FileEntryState::Active | FileEntryState::Plotted => {
                            FileEntryState::PreviouslyPlotted
                        }
                        FileEntryState::Idle | FileEntryState::PreviouslyPlotted => {
                            FileEntryState::Plotted
                        }
                        FileEntryState::NeedsConfig => FileEntryState::Idle,
                    }
                }
            };

            // toggle plotted or active
            if file_label.secondary_clicked() {
                match file_entry.state {
                    FileEntryState::Plotted => file_entry.state = FileEntryState::Active,
                    FileEntryState::Active => file_entry.state = FileEntryState::Plotted,
                    _ => (),
                }
            }
        }
    }
}

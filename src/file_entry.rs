use std::iter::Iterator;
use std::path::Path;

use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::{app::FloatInput, csvfile::CSVFile};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub filename: String,
    pub data_file: CSVFile,
    pub scale: FloatInput,
    pub offset: FloatInput,
    pub xoffset: FloatInput,
    pub color: Color32,
    state: FileEntryState,
    pub id: usize,
    pub preview: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
enum FileEntryState {
    Idle,
    Plotted,
    PreviouslyPlotted,
    Active,
    NeedsConfig,
}

impl FileEntryState {}

impl FileEntry {
    pub fn get_file_label_text(&mut self) -> egui::RichText {
        use FileEntryState::*;
        let text = egui::RichText::new(&self.filename);
        match self.state {
            Idle | PreviouslyPlotted => text,
            Plotted => text.color(Color32::BLACK).background_color(self.color),
            Active => text
                .color(Color32::BLACK.gamma_multiply(0.5))
                .background_color(self.color),
            NeedsConfig => text.color(Color32::RED),
        }
    }
    pub fn get_file_label(&mut self) -> egui::Label {
        egui::Label::new(self.get_file_label_text())
    }
    pub fn reload_csv(&mut self, folder_path: &Path, error_log: &mut Vec<String>) {
        let filepath = { folder_path.join(self.filename.clone()) };
        if let Some(csvfile) = CSVFile::new(
            filepath,
            self.data_file.xcol,
            self.data_file.ycol,
            self.data_file.delimiter,
            self.data_file.comment_char,
            self.data_file.skip_header,
            self.data_file.skip_footer,
            error_log,
        ) {
            self.data_file = csvfile;
        }
    }
    pub fn should_be_listed(&self, search_phrase: &str, folder_is_expanded: bool) -> bool {
        use FileEntryState::*;
        let contains_search_phrase = search_phrase
            .split(" ")
            .all(|phrase| self.filename.contains(phrase));
        match (contains_search_phrase, folder_is_expanded, &self.state) {
            (true, true, _) => true,
            (_, _, Idle) => false,
            (_, _, Plotted | PreviouslyPlotted | Active | NeedsConfig) => true,
        }
    }
    pub fn is_active(&self) -> bool {
        self.state == FileEntryState::Active
    }
    pub fn set_active(&mut self) {
        self.state = FileEntryState::Active
    }
    pub fn is_plotted(&self) -> bool {
        use FileEntryState::*;
        match self.state {
            Plotted | Active | NeedsConfig => true,
            Idle | PreviouslyPlotted => false,
        }
    }
    pub fn was_just_plotted(&self) -> bool {
        use FileEntryState::*;
        match self.state {
            Idle | Plotted | Active | NeedsConfig => true,
            PreviouslyPlotted => true,
        }
    }
}

// transitions
impl FileEntry {
    pub fn clicked(&mut self, path: &Path, error_log: &mut Vec<String>) {
        if self.data_file.data.is_empty() && self.state != FileEntryState::NeedsConfig {
            let filepath = { path.join(self.filename.clone()) };
            if let Some(csvfile) = CSVFile::new(
                filepath,
                self.data_file.xcol,
                self.data_file.ycol,
                self.data_file.delimiter,
                self.data_file.comment_char,
                self.data_file.skip_header,
                self.data_file.skip_footer,
                error_log,
            ) {
                // immediately plot freshly loaded csv
                self.state = FileEntryState::Plotted;
                self.data_file = csvfile;
            } else {
                self.state = FileEntryState::NeedsConfig;
            }
        } else {
            self.state = match self.state {
                FileEntryState::Active | FileEntryState::Plotted => {
                    FileEntryState::PreviouslyPlotted
                }
                FileEntryState::Idle | FileEntryState::PreviouslyPlotted => FileEntryState::Plotted,
                FileEntryState::NeedsConfig => FileEntryState::Idle,
            }
        }
    }
    pub fn secondary_clicked(&mut self) {
        match self.state {
            FileEntryState::Plotted => self.state = FileEntryState::Active,
            FileEntryState::Active => self.state = FileEntryState::Plotted,
            _ => (),
        }
    }
    pub fn search_phrase_changed(&mut self) {
        use FileEntryState::*;
        self.state = match self.state {
            PreviouslyPlotted => Idle,
            Idle => Idle,
            Plotted => Plotted,
            Active => Active,
            NeedsConfig => NeedsConfig,
        }
    }
}

pub fn get_file_entries(folder: &Path, id_counter: &mut usize) -> Vec<FileEntry> {
    let mut file_entries = vec![];
    if let Ok(read_dir) = folder.read_dir() {
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
                color: Color32::TRANSPARENT,
                id: *id_counter,
                preview: utils::read_first_lines(&entry.path(), 20).unwrap_or_default(),
            };
            *id_counter += 1;
            file_entries.push(file_entry)
        }
    }
    file_entries
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

        for line in buf_reader.lines().take(num_lines) {
            if let Ok(line) = line {
                lines.push_str(&line);
            }
        }

        Ok(lines)
    }
}

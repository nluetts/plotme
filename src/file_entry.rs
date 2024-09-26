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
    pub state: FileEntryState,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum FileEntryState {
    Idle,
    Plotted,
    PreviouslyPlotted,
    Active,
    NeedsConfig,
}

impl FileEntry {
    pub fn get_file_label_text(&mut self) -> egui::RichText {
        let mut text = egui::RichText::new(&self.filename);
        if self.is_plotted() {
            let mut textcolor = Color32::BLACK;
            if self.is_active() {
                textcolor = textcolor.gamma_multiply(0.5)
            };
            text = text.background_color(self.color).color(textcolor);
        }
        if self.is_active() {
            text = text.strong();
        }
        text
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
    pub fn is_active(&self) -> bool {
        self.state == FileEntryState::Active
    }
    pub fn is_plotted(&self) -> bool {
        self.state == FileEntryState::Plotted
            || self.state == FileEntryState::NeedsConfig
            || self.is_active()
    }
    pub fn was_just_plotted(&self) -> bool {
        self.state == FileEntryState::PreviouslyPlotted
    }
}

pub fn get_file_entries(folder: &Path) -> Vec<FileEntry> {
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
            };
            file_entries.push(file_entry)
        }
    }
    file_entries
}

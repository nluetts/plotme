#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{fs::DirEntry, path::PathBuf, str::FromStr};

use csv::ReaderBuilder as CSVReaderBuilder;
use eframe::egui::{self, TextBuffer, Widget};

fn main() -> eframe::Result {
    let data_files = if let Some(data_file) = CSVFile::new(
        PathBuf::from_str("sample_spectrum.csv").unwrap(),
        ',' as u8,
        '#' as u8,
    ) {
        vec![data_file]
    } else {
        Vec::new()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Native file dialogs and drag-and-drop files",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MyApp {
                // data_files,
                ..Default::default()
            }))
        }),
    )
}

impl CSVFile {
    fn new(filepath: PathBuf, delimiter: u8, comment_char: u8) -> Option<Self> {
        let rdr = CSVReaderBuilder::new()
            .comment(Some(comment_char))
            .delimiter(delimiter)
            .from_path(filepath.clone());

        let data = match rdr {
            Ok(mut rdr) => {
                let mut data = Vec::<[f64; 2]>::new();
                'record: for entry in rdr.records() {
                    if let Ok(entry) = entry {
                        let mut point = [0.0, 0.0];
                        for (i, pt) in entry.iter().enumerate() {
                            if i > 1 {
                                break;
                            }
                            if let Ok(num) = pt.parse::<f64>() {
                                point[i] = num;
                            } else {
                                // skips rows with unreadable numbers
                                continue 'record;
                            }
                        }
                        data.push(point);
                    } else {
                        // skips unreadable rows
                        continue;
                    }
                }
                data
            }
            Err(err) => {
                eprintln!(
                    "WARNING: Data from file {} could not be read: {}",
                    filepath.to_string_lossy(),
                    err
                );
                return None;
            }
        };
        Some(CSVFile {
            filepath,
            data,
            delimiter,
            comment_char,
        })
    }
}

#[derive(Default)]
struct MyApp {
    picked_path: Option<String>,
    folders: Vec<Folder>,
}

struct CSVFile {
    filepath: PathBuf,
    data: Vec<[f64; 2]>,
    delimiter: u8,
    comment_char: u8,
}

impl Default for CSVFile {
    fn default() -> Self {
        Self {
            filepath: "".into(),
            data: vec![],
            delimiter: ',' as u8,
            comment_char: '#' as u8,
        }
    }
}

struct FileEntry {
    filename: String,
    data_file: CSVFile,
    selected: bool,
}

struct Folder {
    path: PathBuf,
    files: Vec<FileEntry>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            if ui.button("Open folder…").clicked() {
                for folder in rfd::FileDialog::new().pick_folders().unwrap_or_default() {
                    let files = get_file_entries(&folder);
                    self.folders.push(Folder {
                        path: folder,
                        files,
                    })
                }
            }

            if let Some(picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }

            for folder in self.folders.iter_mut() {
                ui.label(folder.path.to_str().unwrap());
                for file_entry in folder.files.iter_mut() {
                    let label = if file_entry.selected {
                        ui.label(&file_entry.filename).highlight()
                    } else {
                        ui.label(&file_entry.filename)
                    };
                    if label.clicked() {
                        let filepath = {
                            let path = folder.path.clone();
                            path.join(file_entry.filename.clone())
                        };
                        if let Some(csvfile) = CSVFile::new(filepath, ',' as u8, '#' as u8) {
                            file_entry.selected = !file_entry.selected;
                            file_entry.data_file = csvfile
                        }
                    };
                }
            }

            egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                .show(ui, |plot_ui| {
                    for entry in self.folders.iter().flat_map(|folder| &folder.files) {
                        if !entry.selected {
                            continue;
                        }
                        plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::new(
                            entry.data_file.data.clone(),
                        )))
                    }
                });
        });
    }
}

fn get_file_entries(folder: &PathBuf) -> Vec<FileEntry> {
    let mut file_entries = vec![];
    match folder.read_dir() {
        Ok(read_dir) => {
            for entry in read_dir.into_iter() {
                if let Ok(entry) = entry {
                    // only list csv files
                    let filename = entry.file_name().to_string_lossy().take();
                    if !filename.ends_with(".csv") {
                        continue;
                    }
                    let data_file = CSVFile {
                        filepath: filename.clone().into(),
                        ..Default::default()
                    };
                    let file_entry = FileEntry {
                        filename,
                        data_file,
                        selected: false,
                    };
                    file_entries.push(file_entry)
                };
            }
        }
        Err(_) => (),
    }
    file_entries
}

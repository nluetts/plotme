#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::path::PathBuf;

use csv::ReaderBuilder as CSVReaderBuilder;
use eframe::{
    egui::{self, Color32, TextBuffer},
    epaint::Hsva,
};
use egui_plot::PlotItem;

fn main() -> eframe::Result {
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
            Ok(Box::new(App {
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
struct App {
    folders: Vec<Folder>,
    search_phrase: String,
    plot_xspan: f64,
    plot_yspan: f64,
    color_idx: usize,
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
    displayed: bool,
    expanded: bool,
    scale: FloatInput,
    offset: FloatInput,
    xoffset: FloatInput,
    active: bool,
    color: Option<Color32>,
}

struct FloatInput {
    input: String,
}

impl FloatInput {
    fn parse(&self) -> Option<f64> {
        self.input.parse().ok()
    }
}

struct Folder {
    path: PathBuf,
    files: Vec<FileEntry>,
    expanded: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::panel::SidePanel::left("File Tree")
            .min_width(300.0)
            .show(ctx, |ui| {
                let lab = ui.label("Filter:");
                ui.text_edit_singleline(&mut self.search_phrase)
                    .labelled_by(lab.id);
                if ui.button("Open folder…").clicked() {
                    for folder in rfd::FileDialog::new().pick_folders().unwrap_or_default() {
                        let files = get_file_entries(&folder);
                        self.folders.push(Folder {
                            path: folder,
                            files,
                            expanded: true,
                        })
                    }
                }
                self.list_folders(ui);
            });

        egui::panel::CentralPanel::default().show(ctx, |ui| {
            // read input events
            let (d_down, f_down, g_down, mouse_delta) = ctx.input(|i| {
                (
                    i.key_down(egui::Key::D),
                    i.key_down(egui::Key::F),
                    i.key_down(egui::Key::G),
                    i.pointer.delta(),
                )
            });
            // scale active plots along y
            if !d_down && f_down && mouse_delta.y != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if !file_entry.active {
                        continue;
                    }
                    if let Some(scale) = file_entry.scale.parse() {
                        let scale = scale as f32;
                        // we just modify the string ... hacky
                        file_entry.scale.input =
                            format!("{}", scale + mouse_delta.y.signum() * scale * 0.01);
                    }
                }
            }
            // offset active plots along y
            if d_down && !f_down && mouse_delta.y != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if !file_entry.active {
                        continue;
                    }
                    if let Some(offset) = file_entry.offset.parse() {
                        let offset = offset as f32;
                        // we just modify the string ... hacky
                        file_entry.offset.input = format!(
                            "{}",
                            offset + mouse_delta.y.signum() * self.plot_yspan as f32 * 0.001
                        );
                    }
                }
            }
            // offset active plots along x
            if g_down && mouse_delta.x != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if !file_entry.active {
                        continue;
                    }
                    if let Some(xoffset) = file_entry.xoffset.parse() {
                        let xoffset = xoffset as f32;
                        // we just modify the string ... hacky
                        file_entry.xoffset.input = format!(
                            "{}",
                            xoffset + mouse_delta.x.signum() * self.plot_xspan as f32 * 0.001
                        );
                    }
                }
            }
            let plot = egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                // .allow_drag(false)
                .allow_drag(!(f_down || d_down))
                .show(ui, |plot_ui| {
                    self.plot_xspan = plot_ui.plot_bounds().width();
                    self.plot_yspan = plot_ui.plot_bounds().height();
                    let mut color_index = 0;
                    for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                        if !file_entry.displayed {
                            continue;
                        }
                        let scale = file_entry.scale.parse().unwrap_or(1.0);
                        let offset = file_entry.offset.parse().unwrap_or(0.0);
                        let xoffset = file_entry.xoffset.parse().unwrap_or(0.0);
                        let input_data = file_entry
                            .data_file
                            .data
                            .iter()
                            .map(|[x, y]| [*x + xoffset, *y * scale + offset])
                            .collect();
                        let color = auto_color(color_index);
                        color_index += 1;
                        let line = egui_plot::Line::new(egui_plot::PlotPoints::new(input_data))
                            .color(color.clone());
                        file_entry.color = Some(color);
                        plot_ui.line(line);
                    }
                });
        });
    }
}

impl App {
    fn list_folders(&mut self, ui: &mut egui::Ui) {
        for folder in self.folders.iter_mut() {
            let folder_label = ui.label(folder.path.to_str().unwrap());
            if folder_label.clicked() {
                folder.expanded = !folder.expanded
            }
            if !folder.expanded {
                continue;
            } else {
                folder_label.highlight();
            }
            for file_entry in folder.files.iter_mut() {
                if !self
                    .search_phrase
                    .clone()
                    .split(" ")
                    .all(|phrase| file_entry.filename.contains(phrase))
                {
                    continue;
                }
                let label = if file_entry.displayed {
                    let color = file_entry.color.unwrap_or(egui::Color32::WHITE);
                    ui.label(egui::RichText::new(&file_entry.filename).color(color))
                        .highlight()
                } else {
                    ui.label(&file_entry.filename)
                };
                if file_entry.expanded {
                    let lab = ui.label("Delimiter");
                    let mut delimiter = ",";
                    ui.text_edit_singleline(&mut delimiter).labelled_by(lab.id);
                    let lab = ui.label("Comment character");
                    let mut char = "#";
                    ui.text_edit_singleline(&mut char).labelled_by(lab.id);
                    let lab = ui.label("Scale");
                    ui.text_edit_singleline(&mut file_entry.scale.input)
                        .labelled_by(lab.id);
                    let lab = ui.label("Offset");
                    ui.text_edit_singleline(&mut file_entry.offset.input)
                        .labelled_by(lab.id);
                    ui.checkbox(&mut file_entry.active, "Modify in plot window");
                    if file_entry.displayed {
                        if ui.button("Remove from plot").clicked() {
                            file_entry.displayed = false;
                            file_entry.color = None;
                        }
                    } else {
                        if ui.button("Add to plot").clicked() {
                            file_entry.displayed = true;
                        }
                    }
                }
                if label.clicked() {
                    file_entry.expanded = !file_entry.expanded;
                    // lazily load the data
                    // TODO: if file was updated, it should be reloaded
                    if file_entry.data_file.data.is_empty() {
                        let filepath = {
                            let path = folder.path.clone();
                            path.join(file_entry.filename.clone())
                        };
                        if let Some(csvfile) = CSVFile::new(filepath, ',' as u8, '#' as u8) {
                            file_entry.displayed = true;
                            file_entry.data_file = csvfile;
                        }
                    }
                };
            }
        }
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
                        displayed: false,
                        expanded: false,
                        active: false,
                        scale: FloatInput {
                            input: "1.0".to_string(),
                        },
                        offset: FloatInput {
                            input: "0.0".to_string(),
                        },
                        xoffset: FloatInput {
                            input: "0.0".to_string(),
                        },
                        color: None,
                    };
                    file_entries.push(file_entry)
                };
            }
        }
        Err(_) => (),
    }
    file_entries
}

fn auto_color(color_idx: usize) -> Color32 {
    // analog to egui_plot
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = color_idx as f32 * golden_ratio;
    eframe::epaint::Hsva::new(h, 0.85, 0.5, 1.0).into()
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use serde::{Deserialize, Serialize};
use std::{
    env::{join_paths, VarError},
    fs,
    path::PathBuf,
};

use csv::ReaderBuilder as CSVReaderBuilder;
use eframe::{
    egui::{self, Color32, Id, TextBuffer, Widget},
    epaint::Hsva,
};

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
                ..Default::default()
            }))
        }),
    )
}

#[derive(Serialize, Deserialize, Default)]
struct App {
    folders: Vec<Folder>,
    search_phrase: String,
    plot_xspan: f64,
    plot_yspan: f64,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct FileEntry {
    filename: String,
    data_file: CSVFile,
    is_plotted: bool,
    expanded: bool,
    scale: FloatInput,
    offset: FloatInput,
    xoffset: FloatInput,
    active: bool,
    color: Color32,
}

#[derive(Serialize, Deserialize)]
struct Folder {
    path: PathBuf,
    files: Vec<FileEntry>,
    expanded: bool,
}

#[derive(Serialize, Deserialize)]
struct FloatInput {
    input: String,
}

impl FloatInput {
    fn parse(&self) -> Option<f64> {
        self.input.parse().ok()
    }
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
                self.list_folders(ui, ctx);
                ui.horizontal(|ui| {
                    if ui.button("Save Session").clicked() {
                        self.save_state(None)
                    }
                    if ui.button("Load Session").clicked() {
                        self.load_state(None);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Save Session As ...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("plotme_session.json")
                            .save_file()
                        {
                            self.save_state(Some(path))
                        }
                    }
                    if ui.button("Load Session From ...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_state(Some(path))
                        }
                    }
                })
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
            egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                .allow_drag(!(f_down || d_down))
                .show(ui, |plot_ui| {
                    self.plot_xspan = plot_ui.plot_bounds().width();
                    self.plot_yspan = plot_ui.plot_bounds().height();
                    for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                        if !file_entry.is_plotted {
                            continue;
                        }
                        if file_entry.color == Color32::TRANSPARENT {
                            {
                                // if no color was assigned to file yet, generate
                                // it from the running color index
                                let color_idx = ctx.data_mut(|map| {
                                    let idx =
                                        map.get_temp_mut_or_insert_with(Id::new("color_idx"), || 0);
                                    *idx += 1;
                                    *idx
                                });
                                file_entry.color = auto_color(color_idx);
                            }
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
                        let line = egui_plot::Line::new(egui_plot::PlotPoints::new(input_data))
                            .color(file_entry.color);
                        plot_ui.line(line);
                    }
                });
        });
    }
}

impl App {
    fn list_folders(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        for folder in self.folders.iter_mut() {
            let folder_label = {
                let text = egui::RichText::new(folder.path.to_str().unwrap());
                if folder.expanded {
                    ui.label(text.strong())
                } else {
                    ui.label(text.weak())
                }
            };

            if folder_label.clicked() {
                folder.expanded = !folder.expanded
            }

            if folder.expanded {
                folder.list_files_ui(ui, ctx, &self.search_phrase);
            }
        }
    }

    fn load_state(&mut self, path: Option<PathBuf>) {
        // if no path is given, load from home directory
        let path = if path.is_none() {
            // load config from home
            match default_config_path() {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("ERROR: could not find default config file path: {}", err);
                    return;
                }
            }
        } else {
            path.unwrap()
        };
        fs::read_to_string(&path)
            .and_then(|config_string| {
                *self = serde_json::from_str::<App>(&config_string)?;
                Ok(())
            })
            .map_err(|err| {
                eprintln!(
                    "ERROR: could not read config file {}: {}",
                    path.to_string_lossy(),
                    err
                );
                err
            });
    }

    fn save_state(&self, path: Option<PathBuf>) {
        let path = if path.is_none() {
            // write config file to home directory
            match default_config_path() {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("ERROR: could not find default config file path: {}", err);
                    return;
                }
            }
        } else {
            path.unwrap()
        };

        let state = serde_json::to_string(&self).unwrap();
        if let Err(err) = fs::write(path, state) {
            eprintln!("ERROR: could not write config: {}", err);
        }
    }
}

impl Folder {
    fn list_files_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, search_phrase: &str) {
        for file_entry in self.files.iter_mut() {
            // exclude files which do not match search pattern
            if !search_phrase
                .split(" ")
                .all(|phrase| file_entry.filename.contains(phrase))
            {
                continue;
            }

            // style file label, based on currently expanded or not
            let file_label = {
                let mut text = egui::RichText::new(&file_entry.filename).color(Color32::GRAY);
                if file_entry.is_plotted {
                    text = text.background_color(file_entry.color);
                }
                egui::Label::new(text)
                    .truncate()
                    .ui(ui)
                    .on_hover_text_at_pointer(&file_entry.filename)
            };

            // toggle popup window with file settings
            if file_label.clicked() {
                file_entry.expanded = !file_entry.expanded;
                // lazily load the data
                // TODO: if file was updated, it should be reloaded
                if file_entry.data_file.data.is_empty() {
                    let filepath = {
                        let path = self.path.clone();
                        path.join(file_entry.filename.clone())
                    };
                    if let Some(csvfile) = CSVFile::new(filepath, ',' as u8, '#' as u8) {
                        // this makes it show the data on the first click
                        file_entry.is_plotted = true;
                        file_entry.data_file = csvfile;
                    }
                }
            };

            // toggle plotted
            if file_label.secondary_clicked() {
                file_entry.is_plotted = !file_entry.is_plotted;
            }

            if file_entry.expanded {
                egui::Window::new(&file_entry.filename)
                    .default_width(300.0)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
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
                        if file_entry.is_plotted {
                            if ui.button("Remove from plot").clicked() {
                                file_entry.is_plotted = false;
                                file_entry.color = Color32::TRANSPARENT;
                            }
                        } else {
                            if ui.button("Add to plot").clicked() {
                                file_entry.is_plotted = true;
                            }
                        }
                        egui::color_picker::color_picker_color32(
                            ui,
                            &mut file_entry.color,
                            egui::color_picker::Alpha::BlendOrAdditive,
                        );
                        if ui.button("Close").clicked() {
                            file_entry.expanded = false;
                        }
                    });
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
                        is_plotted: false,
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
                        color: Color32::TRANSPARENT,
                    };
                    file_entries.push(file_entry)
                };
            }
        }
        Err(_) => (),
    }
    file_entries
}

fn auto_color(color_idx: i32) -> Color32 {
    // analog to egui_plot
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = color_idx as f32 * golden_ratio;
    // also updates the color index
    Hsva::new(h, 0.85, 0.5, 1.0).into()
}

fn default_config_path() -> Result<PathBuf, std::env::VarError> {
    let home_path = std::env::var("HOME")?;
    Ok(PathBuf::from(home_path).join(".plotme.json"))
}

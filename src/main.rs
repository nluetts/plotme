#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui::menu::menu_button;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
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
        "PlotMe CSV File Plotter",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App {
                search_phrase: String::from(".csv"),
                ..Default::default()
            }))
        }),
    )
}

#[derive(Serialize, Deserialize, Default)]
struct App {
    folders: Vec<Folder>,
    search_phrase: String,
    //FIXME: plot dimensions are not loaded when restoring session
    plot_dims: PlotDimensions,
    #[serde(skip)]
    errors: Vec<String>,
    #[serde(skip)]
    acceleration: Option<f64>,
    #[serde(skip)]
    copied_csvoptions: Option<CSVFile>,
}

#[derive(Serialize, Deserialize, Default)]
struct PlotDimensions {
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
}

impl PlotDimensions {
    fn xspan(&self) -> f32 {
        (self.x1 - self.x0).abs()
    }
    fn yspan(&self) -> f32 {
        (self.y1 - self.y0).abs()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct CSVFile {
    filepath: PathBuf,
    data: Vec<[f64; 2]>,
    delimiter: u8,
    comment_char: u8,
    xcol: usize,
    ycol: usize,
    skip_header: usize,
    skip_footer: usize,
}

impl Default for CSVFile {
    fn default() -> Self {
        Self {
            filepath: "".into(),
            data: vec![],
            delimiter: b',',
            comment_char: b'#',
            xcol: 1,
            ycol: 2,
            skip_header: 0,
            skip_footer: 0,
        }
    }
}

impl CSVFile {
    fn new(
        filepath: PathBuf,
        xcol: usize,
        ycol: usize,
        delimiter: u8,
        comment_char: u8,
        skip_header: usize,
        skip_footer: usize,
        error_log: &mut Vec<String>,
    ) -> Option<Self> {
        let rdr = CSVReaderBuilder::new()
            .comment(Some(comment_char))
            .delimiter(delimiter)
            .from_path(filepath.clone())
            .map_err(|err| {
                error_log.push(format!(
                    "ERROR: could not read CSV file {filepath:?}: {}",
                    err
                ))
            });
        if rdr.is_err() {
            return None;
        }

        let rdr = rdr.unwrap();

        let data = parse_rows(rdr, xcol, ycol, &filepath, error_log);
        if data.is_empty() {
            return None;
        }
        Some(CSVFile {
            filepath,
            data,
            delimiter,
            comment_char,
            xcol,
            ycol,
            skip_header,
            skip_footer,
        })
    }
}

fn parse_rows(
    mut rdr: csv::Reader<fs::File>,
    xcol: usize,
    ycol: usize,
    filepath: &Path,
    error_log: &mut Vec<String>,
) -> Vec<[f64; 2]> {
    let mut data = Vec::<[f64; 2]>::new();
    for (i, entry) in rdr.records().enumerate() {
        if let Err(e) = entry {
            error_log.push(format!(
                "WARNING: could not parse row {} of file {filepath:?}: {}",
                i + 1,
                e
            ));
            continue;
        }
        let entry = entry.unwrap();
        let x = entry.iter().nth(xcol).map(|x| x.parse::<f64>());
        let y = entry.iter().nth(ycol).map(|y| y.parse::<f64>());
        match (x, y) {
            (Some(Ok(x)), Some(Ok(y))) => {
                data.push([x, y]);
            }
            (Some(Ok(_)), Some(Err(e))) => {
                error_log.push(format!(
                    "WARNING: y-column {ycol} could not be parsed in entry {} for file {filepath:?}: {}",
                    i + 1,
                    e
                ));
                continue;
            }
            (Some(Err(e)), Some(Ok(_))) => {
                error_log.push(format!(
                    "WARNING: x-column {xcol} could not be parsed in entry {} for file {filepath:?}: {}",
                    i + 1,
                    e
                ));
                continue;
            }
            _ => {
                error_log.push(format!(
                    "WARNING: could not parse columns {xcol}, {ycol} in entry {} for file {filepath:?}",
                    i + 1
                ));
                continue;
            }
        }
    }
    data
}

#[derive(Serialize, Deserialize, Clone)]
struct FileEntry {
    filename: String,
    data_file: CSVFile,
    scale: FloatInput,
    offset: FloatInput,
    xoffset: FloatInput,
    color: Color32,
    state: FileEntryState,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
enum FileEntryState {
    Idle,
    Plotted,
    PreviouslyPlotted,
    Active,
    NeedsConfig,
}

#[derive(Serialize, Deserialize, Clone)]
struct Folder {
    path: PathBuf,
    files: Vec<FileEntry>,
    expanded: bool,
    to_be_deleted: bool,
}

#[derive(Serialize, Deserialize, Clone)]
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
        egui::panel::TopBottomPanel::top("Menu").show(ctx, |ui| self.menu(ui));
        egui::panel::TopBottomPanel::bottom("Error Log")
            .exact_height(100.0)
            .show(ctx, |ui| {
                // only retain the last 10 errors
                if self.errors.len() > 10 {
                    let n = self.errors.len().saturating_sub(10);
                    self.errors = self.errors[n..].to_vec();
                };
                ui.label("Error log:");
                ui.label(self.errors.join("\n"));
            });

        egui::panel::CentralPanel::default().show(ctx, |ui| {
            // read input events
            let (d_down, f_down, g_down, mouse_delta) = ctx.input(|i| {
                // set acceleration if mouse is pressed
                if i.pointer.primary_pressed() {
                    self.acceleration = Some(1.0)
                };
                // increase acceleration by x % per frame if mouse button is down
                if i.pointer.primary_down() {
                    self.acceleration = self.acceleration.map(|acc| acc * 1.03);
                }
                (
                    i.key_down(egui::Key::D) && i.pointer.primary_down(), // pan y
                    i.key_down(egui::Key::F) && i.pointer.primary_down(), // scale y
                    i.key_down(egui::Key::G) && i.pointer.primary_down(), // pan x
                    i.pointer.delta(),
                )
            });
            // scale active plots along y
            if !d_down && f_down && mouse_delta.y != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if file_entry.state != FileEntryState::Active {
                        continue;
                    }
                    if let Some(scale) = file_entry.scale.parse() {
                        let acceleration = self.acceleration.unwrap_or(1.0) as f32;
                        let scale = scale as f32;
                        // we just modify the string ... hacky
                        file_entry.scale.input = format!(
                            "{}",
                            scale - mouse_delta.y.signum() * scale * 0.01 * acceleration
                        );
                    }
                }
            }
            // offset active plots along y
            if d_down && !f_down && mouse_delta.y != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if file_entry.state != FileEntryState::Active {
                        continue;
                    }
                    if let Some(offset) = file_entry.offset.parse() {
                        let acceleration = self.acceleration.unwrap_or(1.0) as f32;
                        let offset = offset as f32;
                        let span = self.plot_dims.yspan();
                        // we just modify the string ... hacky
                        file_entry.offset.input = format!(
                            "{}",
                            offset - mouse_delta.y.signum() * span * 0.001 * acceleration
                        );
                    }
                }
            }
            // offset active plots along x
            if g_down && mouse_delta.x != 0.0 {
                for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                    if file_entry.state != FileEntryState::Active {
                        continue;
                    }
                    if let Some(xoffset) = file_entry.xoffset.parse() {
                        let acceleration = self.acceleration.unwrap_or(1.0) as f32;
                        let xoffset = xoffset as f32;
                        let span = self.plot_dims.xspan();
                        // we just modify the string ... hacky
                        file_entry.xoffset.input = format!(
                            "{}",
                            xoffset + mouse_delta.x.signum() * span * 0.001 * acceleration
                        );
                    }
                }
            }
            egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                .allow_drag(!(f_down || d_down || g_down))
                .show(ui, |plot_ui| {
                    // update plot dimensions in App state
                    let [x0, y0] = plot_ui.plot_bounds().min();
                    let [x1, y1] = plot_ui.plot_bounds().max();
                    self.plot_dims.x0 = x0 as f32;
                    self.plot_dims.x1 = x1 as f32;
                    self.plot_dims.y0 = y0 as f32;
                    self.plot_dims.y1 = y1 as f32;
                    for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                        if !file_entry.is_plotted() {
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
                            .color(file_entry.color)
                            .highlight(file_entry.state == FileEntryState::Active);
                        plot_ui.line(line);
                    }
                });
        });
    }
}

impl App {
    fn list_folders(&mut self, ui: &mut egui::Ui) {
        for folder in self.folders.iter_mut() {
            ui.horizontal(|ui| {
                if ui.small_button("x").clicked() {
                    folder.to_be_deleted = true;
                }
                let folder_label = {
                    let text = egui::RichText::new(folder.path.to_str().unwrap());
                    if folder.expanded {
                        ui.label(text.strong())
                    } else {
                        ui.label(text.weak())
                    }
                };

                if folder_label.clicked() {
                    folder.expanded = !folder.expanded;
                }
            });
            folder.list_files_ui(ui, &self.search_phrase, &mut self.errors);
        }
    }

    fn delete_folders(&mut self) {
        self.folders = self
            .folders
            .iter()
            .filter_map(|f| match f.to_be_deleted {
                true => None,
                false => Some(f.to_owned()),
            })
            .collect();
    }

    fn load_state(&mut self, path: Option<PathBuf>) -> Result<(), String> {
        // if no path is given, load from home directory
        let path = match path {
            Some(path) => path,
            None => {
                // load config from home
                default_config_path()
                    .err_to_string("ERROR: could not find default config file path")?
            }
        };
        let config_raw = fs::read_to_string(&path).err_to_string(&format!(
            "Could not read contents of config file {}",
            path.to_string_lossy()
        ))?;
        let state = serde_json::from_str::<App>(&config_raw).err_to_string(&format!(
            "ERROR: could not read config file {}",
            path.to_string_lossy(),
        ))?;
        *self = state;
        Ok(())
    }

    fn save_state(&self, path: Option<PathBuf>) {
        let path = match path {
            Some(path) => path,
            None => {
                // write config to home directory
                match default_config_path() {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("ERROR: could not find default config file path: {}", err);
                        return;
                    }
                }
            }
        };

        let state = serde_json::to_string(&self).unwrap();
        if let Err(err) = fs::write(path, state) {
            eprintln!("ERROR: could not write config: {}", err);
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) -> egui::InnerResponse<()> {
        egui::menu::bar(ui, |ui| {
            menu_button(ui, "Folder", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .min_scrolled_height(800.0)
                    .show(ui, |ui| self.file_tree_ui(ui));
            });
            menu_button(ui, "Session", |ui| {
                if ui.button("Save Session").clicked() {
                    self.save_state(None)
                }
                if ui.button("Load Session").clicked() {
                    if let Err(msg) = self.load_state(None) {
                        self.errors.push(msg);
                    };
                }
                if ui.button("Save Session As ...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("plotme_session.json")
                        .save_file()
                    {
                        self.save_state(Some(path))
                    } else {
                        self.errors
                            .push("WARNING: No path given to save the session.".to_string())
                    }
                }
                if ui.button("Load Session From ...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Err(msg) = self.load_state(Some(path)) {
                            self.errors.push(msg);
                        }
                    }
                }
            });
            menu_button(ui, "File Settings", |ui| {
                ui.set_min_width(400.0);
                let mut files_plotted = false;
                for folder in self.folders.iter_mut() {
                    for file_entry in folder.files.iter_mut() {
                        if !file_entry.is_plotted() {
                            continue; // only list files that are plotted
                        }
                        ui.menu_button(file_entry.get_file_label_text(), |ui| {
                            file_settings_menu(
                                ui,
                                file_entry,
                                &folder.path,
                                &mut self.copied_csvoptions,
                                &mut self.errors,
                            )
                        });
                        if !files_plotted {
                            files_plotted = true;
                        }
                    }
                }
                if !files_plotted {
                    ui.label("Settings for plotted files will appear here.");
                }
            });
            if ui.button("Save Plot").clicked() {
                if let Err(msg) = self.save_svg() {
                    self.errors.push(msg);
                };
            }
        })
    }

    fn file_tree_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("Open Folder").clicked() {
            for folder in rfd::FileDialog::new().pick_folders().unwrap_or_default() {
                let files = get_file_entries(&folder);
                self.folders.push(Folder {
                    path: folder,
                    files,
                    expanded: true,
                    to_be_deleted: false,
                })
            }
        }

        if self.folders.is_empty() {
            ui.label("Opened folders will appear here ...");
            return;
        }

        let lab = ui.label("Filter:");
        let prev_search_phrase = self.search_phrase.clone();
        ui.text_edit_singleline(&mut self.search_phrase)
            .labelled_by(lab.id);
        // if search phrase has changed, release previously plotted file entries
        // from being shown
        if prev_search_phrase != self.search_phrase {
            for file_entry in self.folders.iter_mut().flat_map(|folder| &mut folder.files) {
                if file_entry.state == FileEntryState::PreviouslyPlotted {
                    file_entry.state = FileEntryState::Idle
                }
            }
        }
        self.list_folders(ui);
        // delete folders that were marked to be deleted
        self.delete_folders();
    }

    fn save_svg(&self) -> Result<(), String> {
        use plotters::prelude::*;
        let filepath = if let Some(path) = rfd::FileDialog::new().save_file() {
            path
        } else {
            return Err("ERROR: selected path unvalid.".to_string());
        };
        let root = SVGBackend::new(&filepath, (1024, 768)).into_drawing_area();
        // let font: FontDesc = ("sans-serif", 20.0).into();

        root.fill(&WHITE)
            .err_to_string("ERROR: to prepare canvas for SVG export")?;

        let mut chart = ChartBuilder::on(&root)
            .margin(20u32)
            // .caption(format!("y=x^{}", 2), font)
            .x_label_area_size(30u32)
            .y_label_area_size(30u32)
            .build_cartesian_2d(
                self.plot_dims.x0..self.plot_dims.x1,
                self.plot_dims.y0..self.plot_dims.y1,
            )
            .err_to_string("ERROR: unable to build chart for SVG export")?;

        chart
            .configure_mesh()
            .x_labels(3)
            .y_labels(3)
            .draw()
            .err_to_string("ERROR: unable to prepare labels for SVG export")?;

        for file_entry in self.folders.iter().flat_map(|folder| &folder.files) {
            if !file_entry.is_plotted() || file_entry.color == Color32::TRANSPARENT {
                continue;
            }
            let scale = file_entry.scale.parse().unwrap_or(1.0);
            let offset = file_entry.offset.parse().unwrap_or(0.0);
            let xoffset = file_entry.xoffset.parse().unwrap_or(0.0);
            let color = {
                let (r, g, b, a) = file_entry.color.to_tuple();
                RGBAColor(r, g, b, a as f64 / 255.).stroke_width(2)
            };

            chart
                .draw_series(LineSeries::new(
                    file_entry
                        .data_file
                        .data
                        .iter()
                        .map(|[x, y]| (*x + xoffset, *y * scale + offset))
                        .map(|(x, y)| (x as f32, y as f32)),
                    color,
                ))
                .err_to_string("ERROR: unable to draw data for SVG export")?
                .label(&file_entry.filename)
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
        }

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .position(SeriesLabelPosition::UpperRight)
            .draw()
            .err_to_string("ERROR: unable to configure labels for SVG export")?;

        root.present()
            .err_to_string("ERROR: unable to write SVG output")?;
        Ok(())
    }
}

impl Folder {
    fn list_files_ui(
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

impl FileEntry {
    fn get_file_label_text(&mut self) -> egui::RichText {
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
    fn get_file_label(&mut self) -> egui::Label {
        egui::Label::new(self.get_file_label_text())
    }
    fn reload_csv(&mut self, folder_path: &Path, error_log: &mut Vec<String>) {
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
    fn is_active(&self) -> bool {
        self.state == FileEntryState::Active
    }
    fn is_plotted(&self) -> bool {
        self.state == FileEntryState::Plotted
            || self.state == FileEntryState::NeedsConfig
            || self.is_active()
    }
    fn was_just_plotted(&self) -> bool {
        self.state == FileEntryState::PreviouslyPlotted
    }
}

fn file_settings_menu(
    ui: &mut egui::Ui,
    file_entry: &mut FileEntry,
    folder_path: &Path,
    csv_options: &mut Option<CSVFile>,
    error_log: &mut Vec<String>,
) {
    ui.heading("CSV Settings");

    ui.label("x-Column:");
    integer_edit_field(ui, &mut file_entry.data_file.xcol);
    ui.label("y-Column:");
    integer_edit_field(ui, &mut file_entry.data_file.ycol);

    ui.label("Skip header lines:");
    integer_edit_field(ui, &mut file_entry.data_file.skip_header);
    ui.label("Skip footer files:");
    integer_edit_field(ui, &mut file_entry.data_file.skip_footer);

    let lab = ui.label("Delimiter");
    let mut delimiter =
        String::from_utf8(vec![file_entry.data_file.delimiter]).unwrap_or("#".into());
    ui.text_edit_singleline(&mut delimiter).labelled_by(lab.id);
    if let Some(ch) = delimiter.as_bytes().first() {
        file_entry.data_file.delimiter = *ch;
    }
    let lab = ui.label("Comment character");
    let mut char = String::from_utf8(vec![file_entry.data_file.comment_char]).unwrap_or("#".into());
    ui.text_edit_singleline(&mut char).labelled_by(lab.id);
    if let Some(ch) = char.as_bytes().first() {
        file_entry.data_file.comment_char = *ch;
    }

    ui.horizontal(|ui| {
        if ui.button("Copy Options").clicked() {
            let csv_tempate = CSVFile {
                delimiter: file_entry.data_file.delimiter,
                comment_char: file_entry.data_file.comment_char,
                xcol: file_entry.data_file.xcol,
                ycol: file_entry.data_file.ycol,
                skip_header: file_entry.data_file.skip_header,
                skip_footer: file_entry.data_file.skip_footer,
                ..Default::default()
            };
            *csv_options = Some(csv_tempate);
        }

        match csv_options {
            Some(opts) => {
                if ui.button("Paste Options").clicked() {
                    file_entry.data_file.delimiter = opts.delimiter;
                    file_entry.data_file.comment_char = opts.comment_char;
                    file_entry.data_file.xcol = opts.xcol;
                    file_entry.data_file.ycol = opts.ycol;
                    file_entry.data_file.skip_header = opts.skip_header;
                    file_entry.data_file.skip_footer = opts.skip_footer;
                }
            }
            None => {
                ui.add_enabled(false, egui::Button::new("Paste Options"));
            }
        }
    });

    ui.heading("Manipulation");
    ui.label("Scale");
    ui.text_edit_singleline(&mut file_entry.scale.input);
    ui.label("y-Offset");
    ui.text_edit_singleline(&mut file_entry.offset.input);
    ui.label("x-Offset");
    ui.text_edit_singleline(&mut file_entry.xoffset.input);

    if ui.button("Reload CSV").clicked() {
        return file_entry.reload_csv(folder_path, error_log);
    }

    ui.menu_button("Color", |ui| {
        egui::color_picker::color_picker_color32(
            ui,
            &mut file_entry.color,
            egui::color_picker::Alpha::BlendOrAdditive,
        );
    });
}

fn get_file_entries(folder: &Path) -> Vec<FileEntry> {
    let mut file_entries = vec![];
    if let Ok(read_dir) = folder.read_dir() {
        // flatten pulls out the Ok variants of the `read_dir` elements
        for entry in read_dir.into_iter().flatten() {
            // only list csv files
            let filename = entry.file_name().to_string_lossy().take();
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

// extending the Result type to convert error into strings
trait ErrorStringExt<S> {
    fn err_to_string(self, base_message: &str) -> Result<S, String>;
}

impl<T: std::fmt::Display + std::error::Error, S> ErrorStringExt<S> for Result<S, T> {
    fn err_to_string(self, base_message: &str) -> Result<S, String> {
        self.map_err(|err| format!("{}: {}", base_message, err))
    }
}

fn integer_edit_field(ui: &mut egui::Ui, value: &mut usize) -> egui::Response {
    let mut tmp_value = format!("{}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}

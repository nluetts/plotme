#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::str::FromStr;

use csv::ReaderBuilder as CSVReaderBuilder;
use eframe::egui;

fn main() -> eframe::Result {
    let data_file = CSVFile::new(
        String::from_str("sample_spectrum.csv").unwrap(),
        ',' as u8,
        '#' as u8,
    );

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
                data_file,
                ..Default::default()
            }))
        }),
    )
}

impl CSVFile {
    fn new(filepath: String, delimiter: u8, comment_char: u8) -> Option<Self> {
        let rdr = CSVReaderBuilder::new()
            .comment(Some(comment_char))
            .delimiter(delimiter)
            .from_path(filepath.clone());

        let data = if let Ok(mut rdr) = rdr {
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
        } else {
            eprintln!("WARNING: Data from file {} could not be read!", filepath);
            return None;
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
    dropped_files: Vec<egui::DroppedFile>,
    picked_path: Option<String>,
    data_file: Option<CSVFile>,
}

struct CSVFile {
    filepath: String,
    data: Vec<[f64; 2]>,
    delimiter: u8,
    comment_char: u8,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Drag-and-drop files onto the window!");

            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());
                    self.data_file = CSVFile::new(path.display().to_string(), ',' as u8, '#' as u8);
                }
            }

            if let Some(picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }

            // Show dropped files (if any):
            if !self.dropped_files.is_empty() {
                ui.group(|ui| {
                    ui.label("Dropped files:");

                    for file in &self.dropped_files {
                        let mut info = if let Some(path) = &file.path {
                            path.display().to_string()
                        } else if !file.name.is_empty() {
                            file.name.clone()
                        } else {
                            "???".to_owned()
                        };

                        let mut additional_info = vec![];
                        if !file.mime.is_empty() {
                            additional_info.push(format!("type: {}", file.mime));
                        }
                        if let Some(bytes) = &file.bytes {
                            additional_info.push(format!("{} bytes", bytes.len()));
                        }
                        if !additional_info.is_empty() {
                            info += &format!(" ({})", additional_info.join(", "));
                        }

                        ui.label(info);
                    }
                });
            }
            egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                .show(ui, |plot_ui| {
                    if let Some(data_file) = &self.data_file {
                        plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::new(
                            data_file.data.clone(),
                        )))
                    }
                });
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files.clone_from(&i.raw.dropped_files);
            }
        });
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}

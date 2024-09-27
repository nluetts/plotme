use egui::{epaint::Hsva, Color32, Id};
use serde::{Deserialize, Serialize};

use crate::App;

#[derive(Serialize, Deserialize, Default)]
pub struct PlotDimensions {
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
}

impl PlotDimensions {
    pub fn xspan(&self) -> f32 {
        (self.x1 - self.x0).abs()
    }
    pub fn yspan(&self) -> f32 {
        (self.y1 - self.y0).abs()
    }
}

impl App {
    pub fn plot_panel_ui(&mut self, ctx: &egui::Context) {
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
                    if !file_entry.is_active() {
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
                    if file_entry.is_active() {
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
                    if file_entry.is_active() {
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
                            .highlight(file_entry.is_active());
                        plot_ui.line(line);
                    }
                });
        });
    }
}

pub fn auto_color(color_idx: i32) -> Color32 {
    // analog to egui_plot
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = color_idx as f32 * golden_ratio;
    // also updates the color index
    Hsva::new(h, 0.85, 0.5, 1.0).into()
}

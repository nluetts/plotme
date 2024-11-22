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
            let acc_id = Id::new("acceleration");
            // dispatch mouse and keyboard interactions

            // the current acceleration is stateful and must be kept between
            // updates, thus we store it in the context and load it here
            if ctx.input(|i| i.pointer.primary_released()) {
                ctx.data_mut(|map| {
                    map.remove_temp::<f64>(acc_id);
                });
            }
            let acc = ctx.data_mut(|map| {
                let acc = map.get_temp_mut_or_insert_with(acc_id, || 1.0);
                *acc *= 1.01;
                acc.to_owned()
            });
            let allow_drag = ctx.input(|i| {
                if !i.pointer.primary_down() {
                    return true;
                }
                let mut allow_drag = false;
                if i.modifiers.shift {
                    self.queue_event(Box::new(crate::event::TransformPlot::new_scale_y(
                        acc,
                        i.pointer.delta().y as f64,
                    )))
                } else if i.modifiers.ctrl {
                    self.queue_event(Box::new(crate::event::TransformPlot::new_shift_y(
                        acc,
                        i.pointer.delta().y as f64,
                        self.plot_dims.yspan() as f64,
                    )))
                } else if i.modifiers.alt {
                    self.queue_event(Box::new(crate::event::TransformPlot::new_shift_x(
                        acc,
                        i.pointer.delta().x as f64,
                        self.plot_dims.xspan() as f64,
                    )))
                } else {
                    allow_drag = true;
                }
                allow_drag
            });

            let _respone = egui_plot::Plot::new(1)
                .min_size(egui::Vec2 { x: 640.0, y: 480.0 })
                .allow_drag(allow_drag)
                .allow_zoom(allow_drag)
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

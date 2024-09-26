use egui::{epaint::Hsva, Color32};
use serde::{Deserialize, Serialize};

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

pub fn auto_color(color_idx: i32) -> Color32 {
    // analog to egui_plot
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = color_idx as f32 * golden_ratio;
    // also updates the color index
    Hsva::new(h, 0.85, 0.5, 1.0).into()
}

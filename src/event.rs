use crate::App;

pub trait AppEvent {
    fn apply(&mut self, app: &mut App) -> Vec<String>;
    fn run(&mut self, app: &mut App) {
        let errors = self.apply(app);
        app.errors.extend(errors);
    }
}

pub struct TransformPlot {
    acceleration: f64,
    transform: TransfromKind,
}

enum TransfromKind {
    ScaleY(f64),
    ShiftX { delta: f64, span: f64 },
    ShiftY { delta: f64, span: f64 },
}

impl TransformPlot {
    pub fn new_scale_y(acceleration: f64, delta: f64) -> Self {
        Self {
            acceleration,
            transform: TransfromKind::ScaleY(delta),
        }
    }
    pub fn new_shift_y(acceleration: f64, delta: f64, span: f64) -> Self {
        Self {
            acceleration,
            transform: TransfromKind::ShiftY { delta, span },
        }
    }
    pub fn new_shift_x(acceleration: f64, delta: f64, span: f64) -> Self {
        Self {
            acceleration,
            transform: TransfromKind::ShiftX { delta, span },
        }
    }
}

impl AppEvent for TransformPlot {
    fn apply(&mut self, app: &mut App) -> Vec<String> {
        for file_entry in app.iter_files_mut().filter(|entry| entry.is_active()) {
            match self.transform {
                TransfromKind::ScaleY(delta_y) => {
                    if let Some(scale) = file_entry.scale.parse() {
                        // we just modify the string ... hacky
                        file_entry.scale.input = format!("{}", scale - delta_y * scale * 0.01);
                    }
                }
                TransfromKind::ShiftX { delta, span } => {
                    if let Some(xoffset) = file_entry.xoffset.parse() {
                        // we just modify the string ... hacky
                        file_entry.xoffset.input = format!("{}", xoffset + delta * span * 0.001);
                    }
                }
                TransfromKind::ShiftY { delta, span } => {
                    if let Some(offset) = file_entry.offset.parse() {
                        // we just modify the string ... hacky
                        file_entry.offset.input = format!("{}", offset - delta * span * 0.001);
                    }
                }
            }
        }
        Vec::new()
    }
}

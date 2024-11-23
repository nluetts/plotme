use crate::App;

// TODO: It would be nice if these methods could consume self,
// but then the trait would need to be a subtrait of Sized which
// does not play nice together with Serde
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

pub enum GroupEvent {
    RemoveFromGroup {
        element: crate::app::FileIndex,
        from_group: usize,
    },
    AddToGroup {
        element: crate::app::FileIndex,
        to_group: usize,
    },
    ChangeVisible {
        group: usize,
        unhide: bool,
    },
}

impl AppEvent for GroupEvent {
    fn apply(&mut self, app: &mut App) -> Vec<String> {
        let mut errors = Vec::new();
        match self {
            GroupEvent::RemoveFromGroup {
                element,
                from_group,
            } => match app.groups.1.get_mut(*from_group) {
                Some(group) => {
                    group.entries.remove(element);
                }
                None => {
                    errors.push(format!(
                        "ERROR: could not add to group with index {}, group does not exist!",
                        from_group
                    ));
                }
            },
            GroupEvent::AddToGroup { element, to_group } => match app.groups.1.get_mut(*to_group) {
                Some(group) => {
                    group.entries.insert(*element);
                }
                None => {
                    errors.push(format!(
                        "ERROR: could not add to group with index {}, group does not exist!",
                        to_group
                    ));
                }
            },
            GroupEvent::ChangeVisible { group, unhide } => match app.groups.1.get_mut(*group) {
                Some(group) => {
                    for e in group.entries.iter() {
                        let file_entry = &mut app.folders[e.folder_index].files[e.file_index];
                        if file_entry.is_plotted() ^ *unhide {
                            file_entry.toggle_plotted(&mut errors);
                        }
                    }
                }
                None => {
                    errors.push(format!(
                        "ERROR: could not change visibility of group with index {}, group does not exist!",
                        group
                    ));
                }
            },
        }
        errors
    }
}

use crate::App;

pub enum AppEvent {
    PlotTransformEvent(PlotTransformKind),
    GroupEvent(GroupEventKind),
}

pub trait AppEventRunner: Sized {
    fn apply(self, app: &mut App) -> Vec<String>;
    fn run(self, app: &mut App) {
        let errors = self.apply(app);
        app.errors.extend(errors);
    }
}

impl AppEventRunner for AppEvent {
    fn apply(self, app: &mut App) -> Vec<String> {
        match self {
            AppEvent::PlotTransformEvent(event) => event.apply(app),
            AppEvent::GroupEvent(event) => event.apply(app),
        }
    }
}

#[derive(Default)]
pub struct EventQueue(Vec<AppEvent>);

impl EventQueue {
    pub fn push_event(&mut self, event: crate::event::AppEvent) {
        self.0.push(event);
    }
    pub fn take_events(&mut self) -> Vec<AppEvent> {
        let new = Vec::new();
        std::mem::replace(&mut self.0, new)
    }
}

pub enum PlotTransformKind {
    ScaleY(f64),
    ShiftX { delta: f64, span: f64 },
    ShiftY { delta: f64, span: f64 },
}

impl PlotTransformKind {
    pub fn new_scale_y(delta: f64) -> Self {
        PlotTransformKind::ScaleY(delta)
    }
    pub fn new_shift_y(delta: f64, span: f64) -> Self {
        PlotTransformKind::ShiftY { delta, span }
    }
    pub fn new_shift_x(delta: f64, span: f64) -> Self {
        PlotTransformKind::ShiftX { delta, span }
    }
}

impl AppEventRunner for PlotTransformKind {
    fn apply(self, app: &mut App) -> Vec<String> {
        for file_entry in app.iter_files_mut().filter(|entry| entry.is_active()) {
            match self {
                PlotTransformKind::ScaleY(delta) => {
                    if let Some(scale) = file_entry.scale.parse() {
                        // we just modify the string ... hacky
                        file_entry.scale.input = format!("{}", scale - delta * scale * 0.01);
                    }
                }
                PlotTransformKind::ShiftX { delta, span } => {
                    if let Some(xoffset) = file_entry.xoffset.parse() {
                        // we just modify the string ... hacky
                        file_entry.xoffset.input = format!("{}", xoffset + delta * span * 0.001);
                    }
                }
                PlotTransformKind::ShiftY { delta, span } => {
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

pub enum GroupEventKind {
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

impl AppEventRunner for GroupEventKind {
    fn apply(self, app: &mut App) -> Vec<String> {
        let mut errors = Vec::new();
        match self {
            GroupEventKind::RemoveFromGroup {
                element,
                from_group,
            } => match app.groups.1.get_mut(from_group) {
                Some(group) => {
                    group.entries.remove(&element);
                }
                None => {
                    errors.push(format!(
                        "ERROR: could not add to group with index {}, group does not exist!",
                        from_group
                    ));
                }
            },
            GroupEventKind::AddToGroup { element, to_group } => {
                match app.groups.1.get_mut(to_group) {
                    Some(group) => {
                        group.entries.insert(element);
                    }
                    None => {
                        errors.push(format!(
                            "ERROR: could not add to group with index {}, group does not exist!",
                            to_group
                        ));
                    }
                }
            }
            GroupEventKind::ChangeVisible { group, unhide } => match app.groups.1.get_mut(group) {
                Some(group) => {
                    for e in group.entries.iter() {
                        let file_entry = &mut app.folders[e.folder_index].files[e.file_index];
                        if file_entry.is_plotted() ^ unhide {
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

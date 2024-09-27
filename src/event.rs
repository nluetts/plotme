use crate::App;

pub trait AppEvent {
    fn apply(&mut self, app: &mut App) -> Vec<String>;
    fn run(&mut self, app: &mut App) {
        let errors = self.apply(app);
        app.errors.extend(errors);
    }
}

struct SetActive {
    file_id: usize,
}

impl SetActive {
    fn new(file_id: usize) -> Self {
        Self { file_id }
    }
}

impl AppEvent for SetActive {
    fn apply(&mut self, app: &mut App) -> Vec<String> {
        for folder in app.folders.iter_mut() {
            for file_entry in folder.files.iter_mut() {
                if file_entry.id == self.file_id {
                    file_entry.set_active();
                    return Vec::new();
                }
            }
        }
        let err_msg = format!("ERROR: file with id {} not found", self.file_id);
        return vec![err_msg];
    }
}

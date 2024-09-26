// extending the Result type to convert error into strings
pub trait ErrorStringExt<S> {
    fn err_to_string(self, base_message: &str) -> Result<S, String>;
}

impl<T: std::fmt::Display + std::error::Error, S> ErrorStringExt<S> for Result<S, T> {
    fn err_to_string(self, base_message: &str) -> Result<S, String> {
        self.map_err(|err| format!("{}: {}", base_message, err))
    }
}

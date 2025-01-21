pub const APPLICATION_NAME: &str = "CoNode";
pub const APPLICATION_VERSION: &str = "v0.0.1";
pub const APPLICATION_MAIN_TEXT: &str = "Decentralized Labor Market Node";

/// Returns the application title text.
pub fn application_title_text() -> String {
    format!(
        "{}",
        APPLICATION_NAME.to_string()
    )
}

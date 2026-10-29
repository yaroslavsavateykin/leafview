//! Never write diagnostics over the alternate-screen UI, including over SSH.
pub fn log(message: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let Some(path) = std::env::var_os("LEAFVIEW_LOG") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

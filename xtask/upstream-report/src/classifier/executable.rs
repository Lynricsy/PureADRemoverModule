pub fn is_executable(display_path: &str, content: &[u8]) -> bool {
    let path = display_path.to_lowercase();
    is_script_path(&path)
        || has_extension(&path, &["so", "bin", "exe", "jar"])
        || content.starts_with(b"#!")
}

fn is_script_path(path: &str) -> bool {
    has_extension(path, &["sh", "bash", "zsh", "py", "rb", "pl"])
        || path.contains("/scripts/")
        || path.contains("service.sh")
        || path.contains("customize.sh")
}

fn has_extension(path: &str, expected: &[&str]) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| {
            expected
                .iter()
                .any(|value| extension.eq_ignore_ascii_case(value))
        })
}

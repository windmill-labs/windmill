fn main() {
    // rust-embed requires the embedded folder to exist at compile time.
    // When building without a prior frontend build, create the directory
    // so the derive macro doesn't panic.
    let dir = std::env::var("FRONTEND_BUILD_DIR")
        .unwrap_or_else(|_| "../../frontend/build/".to_string());
    let path = std::path::Path::new(&dir);
    if !path.exists() {
        std::fs::create_dir_all(path).ok();
    }
}

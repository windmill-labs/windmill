fn main() {
    // Duckdb requires Windows Restart Manager library on Windows
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=Rstrtmgr");
}

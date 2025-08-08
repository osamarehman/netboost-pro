use chrono::Utc;

fn main() {
    // Set the BUILD_DATE environment variable to the current UTC time
    let build_date = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Re-run this build script if the build script itself changes
    println!("cargo:rerun-if-changed=build.rs");

    // Conditionally run the tauri build
    #[cfg(feature = "gui")]
    tauri_build::build();
}
// src-tauri/build.rs
// This file should be placed in the src-tauri directory (same level as Cargo.toml)
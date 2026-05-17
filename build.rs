use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // Get the output directory where artifacts will be placed
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Get the project root directory (parent of the Cargo project)
    let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();

    println!("[cortex-debug-zed] Build script running");
    println!(
        "[cortex-debug-zed] Project root: {}",
        project_root.display()
    );
    println!("[cortex-debug-zed] Output directory: {}", out_dir.display());

    // Source paths
    let cortex_debug_dir = project_root.join("cortex-debug");
    let dist_src = cortex_debug_dir.join("dist");

    // Destination paths in the extension bundle
    let bundle_dir = out_dir.join("cortex-debug-bundle");
    let dist_dst = bundle_dir.join("dist");

    // Create bundle directory
    if !bundle_dir.exists() {
        fs::create_dir_all(&bundle_dir).expect("Failed to create bundle directory");
        println!(
            "[cortex-debug-zed] Created bundle directory: {}",
            bundle_dir.display()
        );
    }

    // Copy dist directory
    if dist_src.exists() {
        copy_dir_all(&dist_src, &dist_dst).expect("Failed to copy dist directory");
        println!("[cortex-debug-zed] Copied dist directory to bundle");
    } else {
        eprintln!(
            "[cortex-debug-zed] WARNING: dist directory not found at {}",
            dist_src.display()
        );
    }

    // Print cargo directives to make the bundle available at runtime
    println!(
        "cargo:rustc-env=CORTEX_DEBUG_BUNDLE_DIR={}",
        bundle_dir.display()
    );

    // Rebuild if these files change
    println!("cargo:rerun-if-changed={}", dist_src.display());
}

/// Recursively copy a directory and all its contents
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

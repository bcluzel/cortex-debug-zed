use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Get the output directory where artifacts will be placed
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Get the project root directory (the Cargo project directory)
    let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    println!("[cortex-debug-zed] Build script running");
    println!(
        "[cortex-debug-zed] Project root: {}",
        project_root.display()
    );
    println!("[cortex-debug-zed] Output directory: {}", out_dir.display());

    // Source paths
    let cortex_debug_dir = project_root.join("cortex-debug");

    let dist_src = cortex_debug_dir.join("dist");

    // Build the JS project if dist is missing or sources have changed
    build_js(&project_root, &cortex_debug_dir);

    // Generate the Zed debug adapter schema from cortex-debug's package.json
    let schema_out = project_root
        .join("debug_adapter_schemas")
        .join("cortex-debug-zed.json");
    generate_schema(&project_root, &cortex_debug_dir, &schema_out);

    // Rebuild when JS sources change
    println!(
        "cargo:rerun-if-changed={}",
        cortex_debug_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cortex_debug_dir.join("webpack.config.js").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        cortex_debug_dir.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("patches").join("cortex-debug-no-vscode-console.patch").display()
    );

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

}

/// Run `npm install` (if needed) and `npm run compile` in the cortex-debug directory.
fn build_js(project_root: &Path, cortex_debug_dir: &Path) {
    println!("[cortex-debug-zed] Running npm install in cortex-debug...");
    let status = npm_command()
        .arg("install")
        .current_dir(cortex_debug_dir)
        .status()
        .expect("Failed to run npm install — is Node.js installed?");
    if !status.success() {
        panic!("npm install failed with status: {}", status);
    }

    // Patch cortex-debug source files to work outside VS Code (no GDBServerConsole proxy).
    // The patch is applied before compilation and reversed afterwards so cortex-debug stays clean.
    let patch_file = project_root.join("patches").join("cortex-debug-no-vscode-console.patch");
    git_apply(&patch_file, cortex_debug_dir);

    println!("[cortex-debug-zed] Running npm run compile in cortex-debug...");
    let compile_status = npm_command()
        .args(["run", "compile"])
        .current_dir(cortex_debug_dir)
        .status()
        .expect("Failed to run npm run compile — is Node.js installed?");

    // Always reverse the patch before checking compile status.
    git_apply_reverse(&patch_file, cortex_debug_dir);

    if !compile_status.success() {
        panic!("npm run compile failed with status: {}", compile_status);
    }
}

/// Apply a git patch file to the given directory (idempotent — skips if already applied).
fn git_apply(patch_file: &Path, dir: &Path) {
    let patch_path = patch_file.to_str().expect("patch path is valid UTF-8");
    // If the patch is already applied (e.g. from an interrupted previous build), skip.
    let already_applied = Command::new("git")
        .args(["apply", "--check", "--reverse", "--ignore-whitespace", patch_path])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if already_applied {
        println!("[cortex-debug-zed] Patch already applied, skipping forward apply.");
        return;
    }
    println!("[cortex-debug-zed] Applying patch {}...", patch_file.display());
    let status = Command::new("git")
        .args(["apply", "--ignore-whitespace", patch_path])
        .current_dir(dir)
        .status()
        .expect("Failed to run git apply — is git installed?");
    if !status.success() {
        panic!("git apply failed for {}", patch_file.display());
    }
}

/// Reverse a previously applied git patch (idempotent — skips if not currently applied).
fn git_apply_reverse(patch_file: &Path, dir: &Path) {
    let patch_path = patch_file.to_str().expect("patch path is valid UTF-8");
    // If the patch is not currently applied (e.g. apply was skipped above), nothing to do.
    let is_applied = Command::new("git")
        .args(["apply", "--check", "--reverse", "--ignore-whitespace", patch_path])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !is_applied {
        println!("[cortex-debug-zed] Patch not applied, skipping reverse.");
        return;
    }
    println!("[cortex-debug-zed] Reversing patch {}...", patch_file.display());
    let status = Command::new("git")
        .args(["apply", "--reverse", "--ignore-whitespace", patch_path])
        .current_dir(dir)
        .status()
        .expect("Failed to run git apply --reverse — is git installed?");
    if !status.success() {
        panic!("git apply --reverse failed for {}", patch_file.display());
    }
}

/// Returns the correct npm command for the current OS.
fn npm_command() -> Command {
    if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "npm"]);
        cmd
    } else {
        Command::new("npm")
    }
}

/// Generate debug_adapter_schemas/cortex-debug-zed.json from cortex-debug's package.json.
fn generate_schema(project_root: &Path, cortex_debug_dir: &Path, output: &Path) {
    let script = project_root.join("scripts").join("generate_schema.js");
    let package_json = cortex_debug_dir.join("package.json");

    println!("[cortex-debug-zed] Generating debug adapter schema...");
    let status = npm_command()
        .args([
            "exec",
            "--",
            "node",
            script.to_str().expect("script path is valid UTF-8"),
            package_json.to_str().expect("package.json path is valid UTF-8"),
            output.to_str().expect("output path is valid UTF-8"),
        ])
        .current_dir(cortex_debug_dir)
        .status();

    // Fall back to calling node directly (npm exec not always available)
    let ok = match status {
        Ok(s) if s.success() => true,
        _ => {
            let s = Command::new("node")
                .args([
                    script.to_str().expect("script path is valid UTF-8"),
                    package_json.to_str().expect("package.json path is valid UTF-8"),
                    output.to_str().expect("output path is valid UTF-8"),
                ])
                .current_dir(cortex_debug_dir)
                .status()
                .expect("Failed to run node — is Node.js installed?");
            s.success()
        }
    };

    if !ok {
        panic!("generate_schema.js failed");
    }
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

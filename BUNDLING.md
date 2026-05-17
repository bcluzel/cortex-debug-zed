# Cortex-Debug DAP Server Bundling

This document explains how the cortex-debug DAP server is bundled with the Zed extension and how to maintain and update it.

## Overview

The Cortex-Debug Zed extension bundles the Node.js-based DAP (Debug Adapter Protocol) server from the cortex-debug project. This allows Zed to spawn the DAP server as a subprocess to debug embedded systems using ARM GDB.

## Bundle Structure

```
cortex-debug-zed/
├── build.rs                          # Rust build script (runs at compile time)
├── Cargo.toml                        # Cargo manifest with build script configuration
├── extension-files/                  # Source files for bundling
│   ├── dap-server.js                # DAP server entry point (copied from cortex-debug/)
│   └── package.json                 # Metadata for the bundled server
├── src/
│   └── lib.rs                        # Zed extension implementation
└── target/
    └── debug/                        # Build artifacts (generated)
        └── cortex-debug-bundle/      # Bundled artifacts (generated)
            ├── dap-server.js
            ├── dist/                 # Compiled debug adapter
            │   ├── debugadapter.js
            │   └── ...
            └── package.json
```

## Build Process

The build process is orchestrated by `build.rs` and runs in the following order:

### 1. **Build Script Execution** (`build.rs`)

When you run `cargo build`, Rust automatically executes `build.rs` before compiling the extension:

```rust
fn main() {
    // Determine paths
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let project_root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    
    // Source files
    let cortex_debug_dir = project_root.join("cortex-debug");
    let dap_server_src = cortex_debug_dir.join("dap-server.js");
    let dist_src = cortex_debug_dir.join("dist");
    
    // Bundle directory
    let bundle_dir = out_dir.join("cortex-debug-bundle");
    
    // Copy files to bundle
    fs::copy(&dap_server_src, &bundle_dir.join("dap-server.js"));
    copy_dir_all(&dist_src, &bundle_dir.join("dist"));
}
```

### 2. **File Copying**

The build script copies:
- **`dap-server.js`**: The Node.js entry point that starts the DAP server
- **`dist/`** directory: The compiled TypeScript debug adapter code

### 3. **Environment Variable Export**

The build script sets the `CORTEX_DEBUG_BUNDLE_DIR` environment variable so the extension can locate bundled files at runtime.

### 4. **Dependency Tracking**

The build script registers file watchers so the build is automatically re-run if source files change:

```rust
println!("cargo:rerun-if-changed={}", dap_server_src.display());
println!("cargo:rerun-if-changed={}", dist_src.display());
```

## Prerequisites for Building

Before building the extension, you need to compile the cortex-debug project:

### Step 1: Build cortex-debug

```bash
cd cortex-debug
npm install
npm run compile
```

This generates:
- `cortex-debug/dist/debugadapter.js` - The compiled debug adapter
- Other compiled files in the `dist/` directory

### Step 2: Build the extension

```bash
cd ../cortex-debug-zed
cargo build --release
```

The build will:
1. Execute `build.rs` to bundle the DAP server files
2. Compile the Rust extension code
3. Generate the extension binary

## Updating the DAP Server

To update the bundled DAP server:

### 1. Update the cortex-debug source

```bash
cd cortex-debug
git pull origin main  # or merge your changes
npm install
npm run compile
```

### 2. Rebuild the extension

```bash
cd ../cortex-debug-zed
cargo clean  # Optional: ensures clean rebuild
cargo build --release
```

The build script will automatically detect changes in the `dist/` directory and re-bundle the files.

## Configuration

The build process is configured in `Cargo.toml`:

```toml
[package]
build = "build.rs"  # Tells cargo to run build.rs before compilation

[lib]
crate-type = ["cdylib"]  # Produces a C-compatible dynamic library for Zed

[build-dependencies]
# build.rs uses only std library (no external dependencies needed)
```

## Troubleshooting

### Issue: "dist directory not found"

**Cause**: The cortex-debug project hasn't been compiled yet.

**Solution**: Build cortex-debug first:
```bash
cd ../cortex-debug
npm run compile
```

### Issue: "dap-server.js not found"

**Cause**: The source file is missing from the cortex-debug directory.

**Solution**: Verify the cortex-debug project is in the correct location:
```bash
ls -la ../cortex-debug/dap-server.js
```

### Issue: Build artifacts not updated after changing cortex-debug

**Cause**: The build cache may be stale.

**Solution**: Clean and rebuild:
```bash
cargo clean
cargo build --release
```

### Issue: Permission denied when running DAP server

**Cause**: The copied `dap-server.js` may not have execute permissions.

**Solution**: This is typically not an issue for Node.js scripts (they're interpreted), but ensure Node.js is installed:
```bash
which node
node --version
```

## Runtime Behavior

At runtime, the Zed extension (`lib.rs`) uses the bundled DAP server:

```rust
fn get_dap_binary(&mut self, adapter_name: String, ...) -> Result<DebugAdapterBinary, String> {
    // Determine the path to the bundled dap-server.js
    let cortex_debug_path = user_provided_debug_adapter_path
        .unwrap_or_else(|| "cortex-debug".to_string());
    
    // Spawn as subprocess with --server=<port>
    let port = 50_000;
    let args = vec!["--server".to_string(), format!("--port={}", port)];
    
    DebugAdapterBinary {
        command: Some(cortex_debug_path),
        arguments: args,
        connection: Some(tcp_arguments),
        ...
    }
}
```

The DAP server then:
1. Receives the command: `node dap-server.js --server=50000`
2. Parses the `--server=50000` argument
3. Starts a TCP server on port 50000
4. Handles DAP protocol messages from Zed
5. Communicates with GDB to debug the target

## Key Files Reference

| File | Purpose |
|------|---------|
| `build.rs` | Build script that bundles cortex-debug files |
| `Cargo.toml` | Cargo manifest with build configuration |
| `extension-files/dap-server.js` | Reference copy of DAP server entry point |
| `extension-files/package.json` | Metadata for bundled server |
| `src/lib.rs` | Zed extension implementation |
| `target/debug/cortex-debug-bundle/` | Generated bundle directory |

## Related Documentation

- [Zed Extension API Documentation](https://zed.dev/docs/extensions)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [Cortex-Debug Project](https://github.com/cfertility/cortex-debug)
- [Cortex-Debug for Zed Setup](../cortex-debug/SETUP_FOR_ZED.md)

## Future Improvements

Potential enhancements to the bundling system:

1. **Selective Bundling**: Only bundle necessary files from dist/ to reduce binary size
2. **Version Management**: Track cortex-debug version in the bundle
3. **Dynamic Loading**: Allow users to specify custom DAP server binaries
4. **Asset Verification**: Add checksums or signatures to verify bundled files
5. **Automated Updates**: Add tooling to update and re-bundle cortex-debug automatically

## Contributing

When modifying the bundling process:

1. Update `build.rs` for changes to the build logic
2. Update `Cargo.toml` for dependency changes
3. Keep `extension-files/` as a reference copy of bundled files
4. Update this documentation if the process changes
5. Test builds on multiple platforms (Linux, macOS, Windows)

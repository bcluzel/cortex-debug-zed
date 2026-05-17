# Cortex-Debug Zed Extension - Quick Reference

## 📋 Setup Checklist

- [x] Build script created (`build.rs`)
- [x] Cargo configuration updated (`Cargo.toml`)
- [x] Extension files directory created (`extension-files/`)
- [x] DAP server entry point copied (`extension-files/dap-server.js`)
- [x] Metadata file created (`extension-files/package.json`)
- [x] Comprehensive documentation created (`BUNDLING.md`)
- [x] Setup report generated (`SETUP_REPORT.md`)

## 🚀 Quick Start

### Build Prerequisites
```bash
# 1. Build the cortex-debug project first
cd cortex-debug
npm install
npm run compile

# 2. Build the Zed extension
cd ../cortex-debug-zed
cargo build --release
```

### Build Output
The build process creates:
```
target/debug/cortex-debug-bundle/
├── dap-server.js          # DAP server entry point
├── dist/                  # Compiled debug adapter
│   ├── debugadapter.js
│   └── ... (other files)
└── package.json
```

## 📁 File Structure

```
cortex-debug-zed/
├── build.rs               # Build script for bundling
├── Cargo.toml             # Configuration (build = "build.rs")
├── BUNDLING.md            # Detailed bundling documentation
├── SETUP_REPORT.md        # This setup summary
├── extension-files/
│   ├── dap-server.js      # DAP server reference copy
│   └── package.json       # Metadata
├── src/lib.rs             # Zed extension implementation
└── target/                # Build artifacts
```

## 🔧 How It Works

### 1. Compile Time (Build Script)
1. `cargo build` runs `build.rs`
2. Script locates `cortex-debug/dap-server.js` and `cortex-debug/dist/`
3. Creates `target/debug/cortex-debug-bundle/`
4. Copies files to bundle directory
5. Exports `CORTEX_DEBUG_BUNDLE_DIR` environment variable

### 2. Runtime (Extension)
1. Zed calls extension with debug configuration
2. Extension spawns: `node dap-server.js --server=<port>`
3. DAP server starts TCP listener on specified port
4. Zed sends DAP protocol messages over TCP
5. Server communicates with GDB

## 🛠️ Common Tasks

### Update Cortex-Debug
```bash
cd cortex-debug
git pull origin main
npm install
npm run compile

cd ../cortex-debug-zed
cargo clean
cargo build --release
```

### Clean Build
```bash
cargo clean
cargo build --release
```

### Check Build Output
```bash
ls -la target/debug/cortex-debug-bundle/
```

## 📖 Documentation

| Document | Purpose |
|----------|---------|
| `BUNDLING.md` | Complete bundling guide with troubleshooting |
| `SETUP_REPORT.md` | Detailed setup summary and status |
| `cortex-debug/SETUP_FOR_ZED.md` | Cortex-debug adaptation for Zed |

## ⚠️ Troubleshooting

### "dist directory not found"
```bash
cd ../cortex-debug
npm run compile
```

### "dap-server.js not found"
```bash
ls -la ../cortex-debug/dap-server.js
```

### Build cache stale
```bash
cargo clean
cargo build --release
```

For more issues, see `BUNDLING.md` Troubleshooting section.

## 🎯 Key Components

### build.rs
- **Lines**: 89
- **Function**: Bundles cortex-debug files during build
- **Key features**: Recursive copying, error handling, dependency tracking

### Cargo.toml
- **Changes**: Added `build = "build.rs"` and `[build-dependencies]`
- **No external dependencies**: Uses Rust stdlib only

### extension-files/
- **dap-server.js**: Node.js script that starts DAP server on a port
- **package.json**: Metadata with version, requirements, keywords

### Documentation
- **BUNDLING.md**: 253 lines covering the complete bundling system
- **SETUP_REPORT.md**: 343-line detailed status report

## ✨ Features

✅ Automatic bundling at build time
✅ Graceful error handling
✅ Recursive directory copying
✅ Incremental rebuilds (only when files change)
✅ Cross-platform compatible (Linux, macOS, Windows)
✅ No external dependencies (uses std lib only)
✅ Comprehensive documentation
✅ Clear troubleshooting guide

## 📊 Build System Overview

```
cortex-debug source code
        ↓
    npm compile
        ↓
cortex-debug/dist/debugadapter.js
        ↓
cargo build (build.rs)
        ↓
target/debug/cortex-debug-bundle/
        ↓
Zed extension binary
```

## 🔗 Related Files

- **Zed extension**: `src/lib.rs` (spawns DAP server)
- **DAP protocol**: Uses standard Microsoft DAP over TCP
- **GDB communication**: DAP server interfaces with arm-none-eabi-gdb

## 💡 Remember

1. **Build cortex-debug first**: `npm run compile` must succeed
2. **Use `--release` mode**: For production builds
3. **Check bundle after build**: `ls target/debug/cortex-debug-bundle/`
4. **Refer to BUNDLING.md**: For detailed explanations

## 📝 Status Summary

✅ **Setup**: Complete
✅ **Build script**: Functional
✅ **Configuration**: Correct
✅ **Documentation**: Comprehensive
✅ **Ready to build**: Yes

For complete details, see `SETUP_REPORT.md` or `BUNDLING.md`.

# cortex-debug-zed

A [Zed](https://zed.dev) extension that brings embedded ARM/Cortex-M debugging to the editor.
It bundles and drives the [cortex-debug](https://github.com/Marus/cortex-debug) VS Code extension's
JavaScript backend as a Debug Adapter Protocol (DAP) server.

## How it works

```
Zed debugger UI  ──DAP (stdio)──►  cortex-debug-zed (Rust/WASM extension)
                                           │
                                           │  spawns
                                           ▼
                                  node dist/debugadapter.js
                                  (cortex-debug JS backend, bundled at build time)
                                           │
                                           │  talks to GDB server
                                           ▼
                               JLink / OpenOCD / ST-LINK / PyOCD / …
                                           │
                                           ▼
                                     Target hardware
```

The extension:
1. Extracts the bundled `debugadapter.js` and GDB support scripts to the extension work directory.
2. Launches it with Node.js using stdio DAP transport.
3. Injects required defaults (e.g. `extensionPath`, `rttConfig`) that the VS Code frontend would normally provide.

The full cortex-debug configuration surface is available — any option supported by the
VS Code extension works here too.

## Prerequisites

- [Node.js](https://nodejs.org) must be available in your `PATH`.
- The GDB server for your probe must be installed and in your `PATH`:
  - **J-Link**: `JLinkGDBServerCL.exe` from the [SEGGER J-Link software](https://www.segger.com/downloads/jlink/)
  - **OpenOCD**: `openocd`
  - **ST-LINK**: `ST-LINK_gdbserver` from STM32CubeIDE
  - **PyOCD**: `pyocd`
  - **BMP** (Black Magic Probe): no server needed, uses GDB directly
- A GDB binary compatible with your target (e.g. `arm-none-eabi-gdb`).

## Installation

Install from the Zed extension marketplace, or install as a dev extension from this repository:

1. Clone the repository.
2. Open Zed → **Extensions** → **Install Dev Extension** → select this folder.

## Configuration

Debug sessions are configured in `.zed/debug.json` at the root of your project.

Cortex-debug fields are placed **at the top level** of the scenario object alongside
`label`, `adapter`, and `request` — there is no nested `config` object.

### Minimal structure

```json
[
  {
    "label": "My Debug Session",
    "adapter": "cortex-debug-zed",
    "request": "launch",
    "servertype": "<jlink|openocd|stlink|pyocd|bmp|qemu|external>",
    "executable": "$ZED_WORKTREE_ROOT/build/firmware.elf",
    "cwd": "$ZED_WORKTREE_ROOT"
  }
]
```

> **`$ZED_WORKTREE_ROOT`** is expanded to the absolute path of the open project.

> **`request`** defaults to `"launch"` if omitted.

---

## Examples

### J-Link — STM32 (launch)

```json
[
  {
    "label": "Debug STM32 (J-Link)",
    "adapter": "cortex-debug-zed",
    "request": "launch",
    "servertype": "jlink",
    "device": "STM32H573VI",
    "interface": "swd",
    "executable": "$ZED_WORKTREE_ROOT/build/firmware.elf",
    "cwd": "$ZED_WORKTREE_ROOT",
    "gdbPath": "C:/Program Files/Arm/GNU Toolchain mingw-w64-x86_64-arm-none-eabi/bin/arm-none-eabi-gdb.exe"
  }
]
```

> **Note:** Use the exact J-Link device name for your chip.
> Run `JLinkGDBServerCL.exe -device ?` or search the [SEGGER supported devices list](https://www.segger.com/downloads/supported-devices.php).
>
> To run a build task before debugging, add `"build": { "label": "your-task-label" }` at the top level.
> This replaces VS Code's `preLaunchTask`.

---

### J-Link — attach to running target

```json
[
  {
    "label": "Attach STM32 (J-Link)",
    "adapter": "cortex-debug-zed",
    "request": "attach",
    "servertype": "jlink",
    "device": "STM32H573VI",
    "interface": "swd",
    "executable": "$ZED_WORKTREE_ROOT/build/firmware.elf",
    "cwd": "$ZED_WORKTREE_ROOT",
    "gdbPath": "C:/Program Files/Arm/GNU Toolchain mingw-w64-x86_64-arm-none-eabi/bin/arm-none-eabi-gdb.exe"
  }
]
```

---

### OpenOCD — STM32 generic

```json
[
  {
    "label": "Debug STM32 (OpenOCD)",
    "adapter": "cortex-debug-zed",
    "request": "launch",
    "servertype": "openocd",
    "executable": "$ZED_WORKTREE_ROOT/build/firmware.elf",
    "cwd": "$ZED_WORKTREE_ROOT",
    "configFiles": [
      "interface/stlink.cfg",
      "target/stm32h4x.cfg"
    ]
  }
]
```

---

### ST-LINK (STM32CubeIDE server)

```json
[
  {
    "label": "Debug STM32 (ST-LINK)",
    "adapter": "cortex-debug-zed",
    "request": "launch",
    "servertype": "stlink",
    "device": "STM32H573VI",
    "executable": "$ZED_WORKTREE_ROOT/build/firmware.elf",
    "cwd": "$ZED_WORKTREE_ROOT",
    "stm32cubeprogrammer": "/opt/STM32CubeIDE/plugins/com.st.stm32cube.ide.mcu.externaltools.cubeprogrammer"
  }
]
```

---

## Common options

| Option | Description |
|---|---|
| `request` | `"launch"` or `"attach"` (default: `"launch"`) |
| `servertype` | GDB server: `jlink`, `openocd`, `stlink`, `pyocd`, `bmp`, `qemu`, `external` |
| `device` | Target device name (required by J-Link / ST-LINK) |
| `executable` | Path to the ELF file to debug |
| `cwd` | Working directory |
| `interface` | Debug interface: `swd` (default) or `jtag` |
| `gdbPath` | GDB binary path (default: `arm-none-eabi-gdb`) |
| `svdFile` | Path to a CMSIS-SVD file for peripheral register display |
| `serialNumber` | Probe serial number when multiple probes are connected |
| `serverpath` | Override the GDB server binary path |
| `configFiles` | OpenOCD config files |
| `searchDir` | OpenOCD search directories |
| `swoConfig` | SWO/ITM trace configuration |
| `rttConfig` | SEGGER RTT configuration |

For the full list of options, see the generated schema at
[`debug_adapter_schemas/cortex-debug-zed.json`](debug_adapter_schemas/cortex-debug-zed.json)
or the upstream [cortex-debug documentation](https://github.com/Marus/cortex-debug/wiki).

## Building from source

Requirements: Rust (via `rustup`), Node.js, npm.

```sh
cargo build --release
```

The build script automatically:
1. Runs `npm install` in `cortex-debug/` if `node_modules` is missing.
2. Runs `npm run compile` (webpack) to produce `cortex-debug/dist/debugadapter.js`.
3. Generates `debug_adapter_schemas/cortex-debug-zed.json` from `cortex-debug/package.json`.

## License

MIT — see [LICENSE](LICENSE).
The bundled cortex-debug backend is © Marus and its contributors, licensed under the MIT License.


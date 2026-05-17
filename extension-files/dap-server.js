#!/usr/bin/env node

/**
 * Cortex-Debug DAP Server Wrapper
 *
 * This script wraps the cortex-debug debug adapter as a standalone DAP server.
 * It accepts --server=<port> argument and listens on that port for DAP connections.
 *
 * The cortex-debug debug adapter (built in dist/debugadapter.js) already includes
 * support for server mode via the @vscode/debugadapter library's runDebugAdapter function.
 * This wrapper simply re-exports the compiled adapter with the appropriate arguments.
 *
 * Usage:
 *   node dap-server.js --server=9000
 *   DEBUG_ADAPTER_PORT=9000 node dap-server.js
 *
 * Example from Zed extension:
 *   spawn("node", ["dap-server.js", "--server=9000"], ...)
 */

const path = require("path");

// Support passing port via environment variable as fallback
if (!process.argv.some((arg) => arg.startsWith("--server"))) {
  const envPort = process.env.DEBUG_ADAPTER_PORT;
  if (envPort) {
    process.argv.push(`--server=${envPort}`);
  }
}

// Log startup info to stderr (DAP protocol uses stdout/socket)
console.error(
  "[cortex-debug DAP Server] Starting with arguments:",
  process.argv.slice(2).join(" "),
);

// The compiled debugadapter.js module contains GDBDebugSession which extends LoggingDebugSession.
// When gdb.ts is executed, it calls LoggingDebugSession.run(GDBDebugSession), which internally
// calls runDebugAdapter(GDBDebugSession). The runDebugAdapter function:
//   1. Parses command-line arguments for --server=<port>
//   2. If port is specified, creates a TCP server on that port
//   3. For each connection, instantiates a new GDBDebugSession and connects it to the socket
//   4. If no port is specified, uses stdin/stdout instead
//
// Therefore, we just need to require the debugadapter module, which will automatically
// handle server mode when --server=<port> is passed.

try {
  require("./dist/debugadapter.js");
} catch (err) {
  console.error("[cortex-debug DAP Server] Error:", err.message);
  if (err.code === "MODULE_NOT_FOUND") {
    console.error(
      "[cortex-debug DAP Server] The dist/debugadapter.js file was not found.",
    );
    console.error("[cortex-debug DAP Server] Please run: npm run compile");
  }
  process.exit(1);
}

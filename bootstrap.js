#!/usr/bin/env node

/**
 * Bootstrap script for cortex-debug DAP server integration with Zed
 *
 * This script loads the cortex-debug JavaScript extension and launches it as a
 * Debug Adapter Protocol (DAP) server. It acts as a bridge between the Rust extension
 * (cortex-debug-zed) and the cortex-debug JavaScript extension.
 *
 * Usage:
 *   node bootstrap.js --server --port=<port>
 *
 * Arguments:
 *   --server      Indicates this should run as a DAP server
 *   --port=<num>  TCP port on which to listen for DAP connections (default: 5000)
 *
 * Environment:
 *   CORTEX_DEBUG_PATH - Path to cortex-debug module (defaul

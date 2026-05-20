use std::{
    fs,
    path::PathBuf,
};

use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
    serde_json,
};

const ADAPTER_NAME: &str = "cortex-debug-zed";

const DEBUG_ADAPTER_JS: &[u8] = include_bytes!("../cortex-debug/dist/debugadapter.js");
const GDB_SUPPORT_INIT: &[u8] = include_bytes!("../cortex-debug/support/gdbsupport.init");
const GDB_SWO_INIT: &[u8] = include_bytes!("../cortex-debug/support/gdb-swo.init");
const OPENOCD_HELPERS_TCL: &[u8] = include_bytes!("../cortex-debug/support/openocd-helpers.tcl");

fn verify_adapter_name(adapter_name: &str) -> Result<(), String> {
    if adapter_name != ADAPTER_NAME {
        Err(format!(
            "Unsupported debug adapter name '{adapter_name}', expected '{ADAPTER_NAME}'"
        ))
    } else {
        Ok(())
    }
}

struct CortexDebugExtension {
    // Extension state
}

impl CortexDebugExtension {
    fn ensure_assets_extracted() -> Result<PathBuf, String> {
        // Zed's WASM host preopens the extension work directory as "." and sets
        // PWD to its real host path.  WASI fs operations must use *relative* paths
        // (rooted at "."); absolute paths are not accessible in the sandbox.
        // We return the absolute PWD-based path so callers can pass it to node,
        // which runs as a normal OS process outside the WASM sandbox.
        let work_dir = std::env::var("PWD")
            .map(PathBuf::from)
            .map_err(|_| "PWD env var not set by Zed WASM host".to_string())?;

        // Always write all bundled assets so that extension updates take effect
        // immediately. The files are written on every session start; the OS page
        // cache makes this cheap in practice.
        fs::create_dir_all("dist")
            .map_err(|e| format!("Failed to create dist directory: {e}"))?;
        fs::write("dist/debugadapter.js", DEBUG_ADAPTER_JS)
            .map_err(|e| format!("Failed to write debugadapter.js: {e}"))?;

        fs::create_dir_all("support")
            .map_err(|e| format!("Failed to create support directory: {e}"))?;
        for (path, bytes) in [
            ("support/gdbsupport.init", GDB_SUPPORT_INIT),
            ("support/gdb-swo.init", GDB_SWO_INIT),
            ("support/openocd-helpers.tcl", OPENOCD_HELPERS_TCL),
        ] {
            fs::write(path, bytes)
                .map_err(|e| format!("Failed to write {path}: {e}"))?;
        }

        Ok(work_dir)
    }
}

impl zed::Extension for CortexDebugExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        println!("Creating new instance of the cortex-debug extension");
        Self {}
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        _worktree: &Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        verify_adapter_name(&adapter_name)?;

        let config_str = config.config.clone();

        let mut json_config: serde_json::Value = serde_json::from_str(&config_str)
            .map_err(|err| format!("Failed to parse JSON config: {}", err))?;

        // cortex-debug's VS Code frontend (configprovider.ts) normally injects these
        // defaults before the debug adapter starts.  We bypass that frontend entirely,
        // so we must supply the same defaults ourselves or cortex-debug crashes on
        // missing fields (e.g. "Cannot read properties of undefined (reading 'enabled')").
        if let Some(obj) = json_config.as_object_mut() {
            obj.entry("rttConfig").or_insert(serde_json::json!({ "enabled": false, "decoders": [] }));
            obj.entry("swoConfig").or_insert(serde_json::json!({
                "enabled": false, "decoders": [], "cpuFrequency": 0, "swoFrequency": 0, "source": "probe"
            }));
            obj.entry("graphConfig").or_insert(serde_json::json!([]));
            obj.entry("debuggerArgs").or_insert(serde_json::json!([]));
            obj.entry("preLaunchCommands").or_insert(serde_json::json!([]));
            obj.entry("postLaunchCommands").or_insert(serde_json::json!([]));
            obj.entry("preAttachCommands").or_insert(serde_json::json!([]));
            obj.entry("postAttachCommands").or_insert(serde_json::json!([]));
            obj.entry("preResetCommands").or_insert(serde_json::json!([]));
            obj.entry("postResetCommands").or_insert(serde_json::json!([]));
            obj.entry("overridePreEndSessionCommands").or_insert(serde_json::Value::Null);
        }

        // Launch cortex-debug as a subprocess using stdio DAP transport.
        // @vscode/debugadapter's LoggingDebugSession.run() uses stdio by default
        // (no --server flag needed). Zed uses stdio when connection is None.
        let assets_dir = Self::ensure_assets_extracted()?;

        // extensionPath tells cortex-debug where to find support/gdbsupport.init etc.
        // GDB requires forward slashes, even on Windows.
        let extension_path = assets_dir.to_string_lossy().replace('\\', "/");
        if let Some(obj) = json_config.as_object_mut() {
            obj.insert("extensionPath".to_string(), serde_json::Value::String(extension_path));
        }

        // Re-serialize after injecting all defaults.
        let config_str = serde_json::to_string(&json_config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        let debug_adapter_path = assets_dir.join("dist/debugadapter.js");
        let script_path = user_provided_debug_adapter_path
            .unwrap_or_else(|| debug_adapter_path.to_string_lossy().to_string());

        println!("Configuration for DAP: {}", config_str);

        let request_kind = match json_config.get("request").and_then(|r| r.as_str()) {
            Some("attach") => StartDebuggingRequestArgumentsRequest::Attach,
            _ => StartDebuggingRequestArgumentsRequest::Launch,
        };

        Ok(DebugAdapterBinary {
            command: Some("node".to_string()),
            arguments: vec![script_path],
            envs: vec![],
            cwd: None,
            connection: None,
            request_args: StartDebuggingRequestArguments {
                // We just pass along the configuration
                configuration: config_str,
                request: request_kind,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        verify_adapter_name(&adapter_name)?;

        // "request" is optional — absent means launch (the common case for cortex-debug).
        let request_value = config
            .get("request")
            .and_then(|f| f.as_str())
            .unwrap_or("launch");

        match request_value {
            "launch" => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            "attach" => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            other => Err(format!(
                "Invalid value for 'request': '{}'. Expected \"launch\" or \"attach\"",
                other
            )),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        debug_config: DebugConfig,
    ) -> Result<DebugScenario, String> {
        verify_adapter_name(&debug_config.adapter)?;

        match debug_config.request {
            DebugRequest::Launch(launch_request) => {
                if !launch_request.args.is_empty() {
                    return Err(
                        "Passing arguments is not supported by this debug adapter".to_string()
                    );
                }

                if !launch_request.envs.is_empty() {
                    return Err(
                        "Setting environment variables is not supported by this debug adapter"
                            .to_string(),
                    );
                }

                // We only get a single program, so we can't create a configuration which would
                // work in a multi-core scenario.
                //
                // We also enable flashing to mimic launching a program.
                let config = serde_json::json!({
                    "cwd": launch_request.cwd,
                    "coreConfigs": [
                        {
                            "programBinary": launch_request.program
                        }
                    ],
                    "flashingConfig": {
                        "flashingEnabled": true,
                        "haltAfterReset": debug_config.stop_on_entry,
                    },
                    "request": "launch",

                });

                let scenario = DebugScenario {
                    label: debug_config.label,
                    adapter: debug_config.adapter,
                    // TODO: Could integrate with cargo
                    build: None,
                    config: config.to_string(),
                    tcp_connection: None,
                };

                Ok(scenario)
            }
            DebugRequest::Attach(_attach_request) => {
                // We can't really support attach in the traditional sense, because we can't attach to a running program on the
                // host
                Err("Attaching to a process is not supported by this debug adapter".to_string())
            }
        }
    }
}

zed::register_extension!(CortexDebugExtension);

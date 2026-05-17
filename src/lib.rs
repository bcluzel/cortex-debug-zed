use std::{
    fs,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    time::Duration,
};

use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, TcpArguments, Worktree,
    serde_json,
};

const ADAPTER_NAME: &str = "cortex-debug-zed";

const DEBUG_ADAPTER_JS: &[u8] = include_bytes!("../../cortex-debug/dist/debugadapter.js");

fn verify_adapter_name(adapter_name: &str) -> Result<(), String> {
    if adapter_name != ADAPTER_NAME {
        Err(format!(
            "Unsupported debug adapter name '{adapter_name}', expected '{ADAPTER_NAME}'"
        ))
    } else {
        Ok(())
    }
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

struct CortexDebugExtension {
    // Extension state
}

impl CortexDebugExtension {
    fn ensure_assets_extracted() -> Result<PathBuf, String> {
        let mut assets_dir = std::env::temp_dir();
        assets_dir.push("zed-cortex-debug-assets");

        if !assets_dir.exists() {
            fs::create_dir_all(&assets_dir)
                .map_err(|e| format!("Failed to create assets directory: {}", e))?;
        }

        let dist_dir = assets_dir.join("dist");
        if !dist_dir.exists() {
            fs::create_dir_all(&dist_dir)
                .map_err(|e| format!("Failed to create dist directory: {}", e))?;
        }

        let debug_adapter_path = dist_dir.join("debugadapter.js");
        if !debug_adapter_path.exists() {
            fs::write(&debug_adapter_path, DEBUG_ADAPTER_JS)
                .map_err(|e| format!("Failed to write debugadapter.js: {}", e))?;
        }

        Ok(assets_dir)
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

        let json_config: serde_json::Value = serde_json::from_str(&config.config)
            .map_err(|err| format!("Failed to parse JSON config: {}", err))?;

        // Check if a TCP connection to an existing cortex-debug instance is specified
        let received_connection =
            if let Some(server_string) = json_config.get("server").and_then(|s| s.as_str()) {
                let mut parsed = parse_server_string(server_string)?;

                // See <https://github.com/zed-industries/zed/blob/834cdc127176228c3c11f1d2cf68a90797a54f15/crates/dap/src/transport.rs#L577>,
                // this seems to be in milliseconds
                parsed.timeout = Some(DEFAULT_TIMEOUT.as_millis() as u64);

                Some(parsed)
            } else {
                None
            };

        // If no TCP connection is specified, launch cortex-debug as a subprocess
        let (command, arguments, connection) = if received_connection.is_none() {
            let assets_dir = Self::ensure_assets_extracted().map_err(|e| e)?;
            let command = "node".to_string();
            let debug_adapter_path = assets_dir.join("dist/debugadapter.js");

            // Use user provided path if available, otherwise use our bundled path
            let script_path = user_provided_debug_adapter_path
                .unwrap_or_else(|| debug_adapter_path.to_string_lossy().to_string());

            // TOOD: Get a port from somewhere
            let port = 50_000;

            let tcp_arguments = TcpArguments {
                port,
                host: Ipv4Addr::LOCALHOST.to_bits(),
                timeout: Some(DEFAULT_TIMEOUT.as_millis() as u64),
            };

            let args = vec![script_path, format!("--server={}", port)];

            (Some(command), args, Some(tcp_arguments))
        } else {
            (None, vec![], received_connection)
        };

        println!("Configuration for DAP: {}", config.config);

        Ok(DebugAdapterBinary {
            command,
            arguments,
            envs: vec![],
            cwd: None,
            connection,
            request_args: StartDebuggingRequestArguments {
                // We just pass along the configuration
                configuration: config.config,
                request: StartDebuggingRequestArgumentsRequest::Launch,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        verify_adapter_name(&adapter_name)?;

        // There should be a request field to indicate if it should be launch or attach
        let Some(request_value) = config.get("request").and_then(|f| f.as_str()) else {
            return Err("Missing 'request' field in configuration".to_string());
        };

        match request_value {
            "launch" => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            "attach" => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            _ => Err(format!(
                "Invalid value for the 'request' field in configuration. Value is {}, but only 'launch' and 'attach' are supported",
                request_value
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

fn parse_server_string(server_string: &str) -> Result<TcpArguments, String> {
    let parts: Vec<&str> = server_string.split(':').collect();

    if parts.len() != 2 {
        return Err(format!(
            "Invalid server string format '{}'. Expected format: 'host:port'",
            server_string
        ));
    }

    let host_str = parts[0];
    let port_str = parts[1];

    // Parse the host IP address
    let host_ip: Ipv4Addr = host_str.parse().map_err(|_| {
        format!(
            "Invalid IP address '{}'. Expected a valid IPv4 address",
            host_str
        )
    })?;

    // Parse the port number
    let port: u16 = port_str.parse().map_err(|_| {
        format!(
            "Invalid port number '{}'. Expected a number between 0 and 65535",
            port_str
        )
    })?;

    Ok(TcpArguments {
        port,
        host: host_ip.to_bits(),
        timeout: None,
    })
}

zed::register_extension!(CortexDebugExtension);

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    #[test]
    fn parse_server_string_invalid_format() {
        // Test missing port
        let result = super::parse_server_string("127.0.0.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid server string format"));

        // Test too many colons
        let result = super::parse_server_string("127.0.0.1:3000:extra");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid server string format"));

        // Test empty string
        let result = super::parse_server_string("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid server string format"));
    }

    #[test]
    fn parse_server_string_invalid_ip() {
        // Test invalid IP address
        let result = super::parse_server_string("999.999.999.999:3000");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP address"));

        // Test non-IP host
        let result = super::parse_server_string("localhost:3000");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP address"));

        // Test empty host
        let result = super::parse_server_string(":3000");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid IP address"));
    }

    #[test]
    fn parse_server_string_invalid_port() {
        // Test non-numeric port
        let result = super::parse_server_string("127.0.0.1:abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid port number"));

        // Test port too large
        let result = super::parse_server_string("127.0.0.1:70000");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid port number"));

        // Test negative port (this will fail parsing as u16)
        let result = super::parse_server_string("127.0.0.1:-1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid port number"));

        // Test empty port
        let result = super::parse_server_string("127.0.0.1:");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid port number"));
    }

    #[test]
    fn parse_server_string_valid_cases() {
        // Test different valid IP addresses and ports
        let test_cases = [
            ("0.0.0.0:80", Ipv4Addr::new(0, 0, 0, 0), 80),
            ("192.168.1.1:8080", Ipv4Addr::new(192, 168, 1, 1), 8080),
            (
                "255.255.255.255:65535",
                Ipv4Addr::new(255, 255, 255, 255),
                65535,
            ),
            ("10.0.0.1:1", Ipv4Addr::new(10, 0, 0, 1), 1),
        ];

        for (server_string, expected_host, expected_port) in test_cases {
            let result = super::parse_server_string(server_string).unwrap();

            assert_eq!(result.port, expected_port);
            assert_eq!(result.host, expected_host.to_bits());
            assert_eq!(result.timeout, None);
        }
    }
}

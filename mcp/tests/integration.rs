use rig_mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct TestModel {
    id: String,
    value: i32,
}

#[tokio::test]
async fn test_mcp_config_formats() {
    let mut env = HashMap::new();
    env.insert("API_KEY".to_string(), "secret".to_string());

    let config = McpConfig {
        name: "test-server".to_string(),
        command: "/path/to/exe".to_string(),
        args: vec!["--flag".to_string()],
        env,
    };

    // Verify Claude JSON
    let claude = config.to_claude_json();
    assert_eq!(
        claude["mcpServers"]["test-server"]["command"],
        "/path/to/exe"
    );
    assert_eq!(claude["mcpServers"]["test-server"]["args"][0], "--flag");
    assert_eq!(
        claude["mcpServers"]["test-server"]["env"]["API_KEY"],
        "secret"
    );

    // Verify Codex TOML
    let codex = config.to_codex_toml();
    assert!(codex.contains("[mcp_servers.test-server]"));
    assert!(codex.contains("command = \"/path/to/exe\""));
    assert!(codex.contains("args = [\"--flag\"]"));
    assert!(codex.contains("[mcp_servers.test-server.env]"));
    assert!(codex.contains("API_KEY = \"secret\""));

    // Verify OpenCode JSON
    let opencode = config.to_opencode_json();
    assert_eq!(
        opencode["mcpServers"]["test-server"]["command"],
        "/path/to/exe"
    );
}

#[tokio::test]
async fn test_toolkit_and_submit_callback() {
    use rig::tool::Tool;

    let (submit, _, _) = JsonSchemaToolkit::<TestModel>::builder()
        .on_submit(|data| format!("Handled: {}", data.id))
        .build()
        .build_tools();

    let input = TestModel {
        id: "123".to_string(),
        value: 42,
    };
    let result = submit.call(input).await.unwrap();

    assert_eq!(result, "Handled: 123");
}

#[tokio::test]
async fn test_toolkit_validation() {
    use rig::tool::Tool;
    use rig_mcp_server::tools::ValidateJsonArgs;

    let (_, validate, _) = JsonSchemaToolkit::<TestModel>::builder()
        .build()
        .build_tools();

    // Valid JSON
    let valid_args = ValidateJsonArgs {
        json: json!({ "id": "test", "value": 10 }),
    };
    let ok_result = validate.call(valid_args).await.unwrap();
    assert!(ok_result.contains("valid"));

    // Invalid JSON (missing field)
    let invalid_args = ValidateJsonArgs {
        json: json!({ "id": "test" }),
    };
    let err_result = validate.call(invalid_args).await.unwrap();
    assert!(err_result.contains("validation failed"));
    assert!(err_result.contains("value"));
}

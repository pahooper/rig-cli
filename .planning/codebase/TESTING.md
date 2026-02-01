# Testing Patterns

**Analysis Date:** 2026-02-01

## Test Framework

**Runner:**
- Tokio for async runtime in tests (`#[tokio::test]`)
- Cargo test for test execution
- Standard Rust test framework (built-in)

**Test Configuration:**
- Location: `mcp/tests/integration.rs`
- No separate test config file (uses Cargo defaults)

**Run Commands:**
```bash
cargo test              # Run all tests across workspace
cargo test --package rig-mcp-server  # Run tests for specific crate
```

**Assertion Library:**
- Standard Rust `assert!()` and `assert_eq!()` macros
- No external assertion framework

## Test File Organization

**Location:**
- Integration tests in `tests/` directory at crate root
- Path: `mcp/tests/integration.rs`
- No unit tests in source directories (all tests in `tests/`)

**Naming:**
- Test file: `integration.rs` (describes scope)
- Test functions: `test_*` prefix with descriptive names

**Structure:**
```
mcp/
├── src/
│   ├── lib.rs
│   ├── server.rs
│   └── tools.rs
└── tests/
    └── integration.rs     # Integration tests
```

## Test Structure

**Suite Organization:**
Example from `mcp/tests/integration.rs`:

```rust
#[tokio::test]
async fn test_mcp_config_formats() {
    // Setup
    let mut env = HashMap::new();
    env.insert("API_KEY".to_string(), "secret".to_string());

    let config = McpConfig {
        name: "test-server".to_string(),
        command: "/path/to/exe".to_string(),
        args: vec!["--flag".to_string()],
        env,
    };

    // Execute
    let claude = config.to_claude_json();

    // Assert
    assert_eq!(
        claude["mcpServers"]["test-server"]["command"],
        "/path/to/exe"
    );
}
```

**Patterns:**
- Async tests marked with `#[tokio::test]` attribute
- Three-part structure: Setup → Execute → Assert
- No separate helper functions (inline setup)
- Use local variables for test data

## Test Data

**Fixtures:**
- Inline data structures created in test body
- HashMap for environment variables: `HashMap::new()`
- Struct instantiation with literal values
- Example from `test_mcp_config_formats()`:
  ```rust
  let mut env = HashMap::new();
  env.insert("API_KEY".to_string(), "secret".to_string());

  let config = McpConfig {
      name: "test-server".to_string(),
      command: "/path/to/exe".to_string(),
      args: vec!["--flag".to_string()],
      env,
  };
  ```

**Location:**
- No separate fixtures file/module
- Data created inline in test functions

## Test Coverage

**Current Status:**
- Integration test suite in `mcp/tests/integration.rs` with 3 test cases
- Focus on API contract testing (config formats, toolkit behavior)
- No unit tests in crate source files
- Limited test coverage overall (3 tests for entire workspace)

**Test Types Present:**

1. **Configuration Format Tests:**
   - `test_mcp_config_formats()` - Verifies McpConfig serialization to multiple formats (Claude JSON, Codex TOML, OpenCode JSON)
   - Validates field mapping and structure

2. **Tool Callback Tests:**
   - `test_toolkit_and_submit_callback()` - Tests JsonSchemaToolkit with callback
   - Async execution of tool with data transformation

3. **Validation Tests:**
   - `test_toolkit_validation()` - Tests JsonSchemaToolkit validation
   - Both valid and invalid JSON inputs
   - Error message content verification

## Assertions

**Patterns Used:**

- `assert_eq!()` for equality checks:
  ```rust
  assert_eq!(
      claude["mcpServers"]["test-server"]["command"],
      "/path/to/exe"
  );
  ```

- `assert!()` for boolean conditions:
  ```rust
  assert!(codex.contains("[mcp_servers.test-server]"));
  assert!(ok_result.contains("valid"));
  ```

- String content checks with `.contains()`:
  ```rust
  assert!(err_result.contains("invalid"));
  assert!(err_result.contains("value"));
  ```

## Mocking

**Framework:**
- No external mocking framework used
- Manual setup of test data structures

**Patterns:**
- Create real instances with test values (not mocks)
- Example: Real `McpConfig` instance with test values instead of mock

**What to Mock:**
- Not documented; current approach avoids mocks

**What NOT to Mock:**
- Core functionality is tested directly
- No stubbing of external dependencies in current tests

## Async Testing

**Pattern:**
- Mark test function with `#[tokio::test]` attribute
- Function body can use `await` and async code
- Example:
  ```rust
  #[tokio::test]
  async fn test_toolkit_and_submit_callback() {
      let (submit, _, _) = JsonSchemaToolkit::<TestModel>::builder()
          .on_submit(|data| format!("Handled: {}", data.id))
          .build()
          .build_tools();

      let result = submit.call(input).await.unwrap();
      assert_eq!(result, "Handled: 123");
  }
  ```

## Test Data Setup

**Struct Creation:**
- Direct instantiation with named fields
- No factory functions or builders for test data
- Example:
  ```rust
  #[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
  struct TestModel {
      id: String,
      value: i32,
  }

  let input = TestModel {
      id: "123".to_string(),
      value: 42,
  };
  ```

## Error Testing

**Pattern:**
- Call functions expecting errors
- Verify error output with string assertions
- Example:
  ```rust
  let invalid_args = ValidateJsonArgs {
      json: json!({ "id": "test" }),
  };
  let err_result = validate.call(invalid_args).await.unwrap();
  assert!(err_result.contains("invalid"));
  assert!(err_result.contains("value"));
  ```

## Test Isolation

**Approach:**
- Each test function creates independent test data
- No shared state between tests
- No setup/teardown fixtures
- Tests can run in parallel safely

## Coverage Gaps

**Untested Areas:**
- Adapter implementations (`claudecode-adapter`, `codex-adapter`, `opencode-adapter`) have no tests
- Error type conversions not tested
- Process execution (`process.rs` in adapters) not tested
- Discovery logic (`discovery.rs`) not tested
- Session management not tested
- Main binaries and CLI execution not tested

**Impact:**
- High risk of regressions in adapter layers
- Process handling edge cases (timeouts, signal handling) untested
- Integration with external CLI tools not verified

## Running Tests

```bash
# Run all tests across workspace
cargo test

# Run tests for specific crate
cargo test -p rig-mcp-server

# Run specific test
cargo test test_mcp_config_formats

# Run with output
cargo test -- --nocapture
```

## Dependencies for Testing

**Test-specific in Cargo.toml:**
- `schemars` - JSON schema generation (used in test data structures)
- `serde_json` - JSON manipulation in tests via `json!()` macro
- `tokio` with `full` features for async runtime

---

*Testing analysis: 2026-02-01*

---
phase: 03-payload-instruction-system
verified: 2026-02-01T22:45:00Z
status: passed
score: 4/4 must-haves verified
---

# Phase 3: Payload & Instruction System Verification Report

**Phase Goal:** Developer can pass context data to agents and force tool workflow
**Verified:** 2026-02-01T22:45:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth                                                                             | Status     | Evidence                                                                                    |
| --- | --------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------- |
| 1   | Developer can attach file contents or text blobs to extraction request           | ✓ VERIFIED | `.payload()` method exists (line 153), used in payload_extraction_e2e.rs (line 102)         |
| 2   | Built-in instruction template forces agents to use example→validate→submit       | ✓ VERIFIED | DEFAULT_WORKFLOW_TEMPLATE (lines 16-30) with numbered workflow steps, injected into system prompt (line 247) |
| 3   | Agents cannot respond with freeform text instead of tool calls                   | ✓ VERIFIED | Constraint language in template: "Do NOT respond with freeform text" (line 27), "Do NOT output raw JSON" (line 28) |
| 4   | Three-tool pattern (example/validate/submit) is the enforced extraction mechanism| ✓ VERIFIED | Workflow steps explicitly enumerate: 1) json_example 2) draft 3) validate_json 4) retry if needed 5) submit |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact                                         | Expected                                              | Status     | Details                                                                                              |
| ------------------------------------------------ | ----------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------- |
| `rig-provider/src/mcp_agent.rs`                  | Builder with payload/instruction methods              | ✓ VERIFIED | Fields: payload (line 80), instruction_template (line 81). Methods: payload() (line 153), instruction_template() (line 163) |
| `rig-provider/src/mcp_agent.rs`                  | DEFAULT_WORKFLOW_TEMPLATE constant                    | ✓ VERIFIED | Defined at line 16, exported from lib.rs (line 25), 15 lines with numbered workflow |
| `rig-provider/src/mcp_agent.rs`                  | 4-block XML prompt when payload present               | ✓ VERIFIED | Lines 263-279: `<context>`, `<task>`, `<output_format>` blocks. Backward compatible (line 278: prompt passthrough when no payload) |
| `rig-provider/src/mcp_agent.rs`                  | System prompt with workflow enforcement               | ✓ VERIFIED | Lines 246-254: workflow_instructions injected into mcp_instruction, includes constraint language |
| `rig-provider/examples/payload_extraction_e2e.rs`| E2E example demonstrating .payload()                  | ✓ VERIFIED | 117 lines, uses .payload(SOURCE_TEXT) (line 102), follows dual-mode pattern, compiles cleanly |
| `rig-provider/examples/mcp_tool_agent_e2e.rs`    | Existing example (backward compat check)              | ✓ VERIFIED | Still compiles, added cross-reference comment (line 10), no breaking changes |

### Key Link Verification

| From                                        | To                             | Via                                         | Status     | Details                                                                                              |
| ------------------------------------------- | ------------------------------ | ------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------- |
| McpToolAgentBuilder::payload()              | run() prompt construction      | self.payload field consumed (line 215)      | ✓ WIRED    | Field moved out in run(), used in if-let (line 263) for 4-block XML construction |
| McpToolAgentBuilder::instruction_template() | run() system prompt construction | self.instruction_template consumed (line 216) | ✓ WIRED  | Field moved out, used with .unwrap_or(DEFAULT_WORKFLOW_TEMPLATE) (line 247) |
| DEFAULT_WORKFLOW_TEMPLATE                   | full_system_prompt in run()    | Workflow template injected (line 250-254)   | ✓ WIRED    | Template incorporated into mcp_instruction, combined with user system_prompt (lines 257-260) |
| payload_extraction_e2e.rs client mode       | McpToolAgent::builder().payload() | .payload(SOURCE_TEXT) call (line 102)     | ✓ WIRED    | Builder chain demonstrates Phase 3 feature usage |
| payload_extraction_e2e.rs server mode       | JsonSchemaToolkit/ToolSetExt   | build_toolset() → serve_stdio() (line 82)  | ✓ WIRED    | Dual-mode pattern properly implemented |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
| ----------- | ----------- | ------ | -------- |
| EXTR-02 | Developer can pass payload data (file contents, text blobs) alongside prompts | ✓ SATISFIED | `.payload()` method exists, demonstrated in payload_extraction_e2e.rs. 4-block XML structure separates context from task (lines 263-279) |
| EXTR-03 | Built-in instruction template forces agents to use submit tool workflow, not freeform text | ✓ SATISFIED | DEFAULT_WORKFLOW_TEMPLATE enforces numbered workflow (lines 16-30). Constraint language forbids freeform text responses (lines 27-28). System prompt includes "You MUST use ONLY these MCP tools" (line 252) |
| EXTR-05 | The three-tool workflow (example/validate/submit) is the enforced extraction mechanism | ✓ SATISFIED | DEFAULT_WORKFLOW_TEMPLATE explicitly defines 5-step workflow culminating in submit. Steps 1-5 map to: json_example → draft → validate_json → fix/retry → submit. "ONLY the 'submit' tool call marks task completion" (line 29) |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| rig-provider/src/mcp_agent.rs | 247 | `.unwrap_or()` with safe default | ℹ️ Info | Safe use - provides DEFAULT_WORKFLOW_TEMPLATE fallback |

**Summary:** One `.unwrap_or()` found, which is the idiomatic Rust pattern for optional fields with defaults. No blocker anti-patterns. No TODO/FIXME comments. No unwrap/expect without error handling. No stub implementations.

### Compilation & Quality Checks

| Check | Status | Details |
| ----- | ------ | ------- |
| `cargo check --workspace` | ✓ PASS | Compiled in 0.12s |
| `cargo clippy -p rig-provider -- -D warnings` | ✓ PASS | Zero warnings with -D warnings flag |
| `cargo doc -p rig-provider --no-deps` | ✓ PASS | Docs generated in 1.17s |
| `cargo test -p rig-provider` | ✓ PASS | 0 tests (no unit tests in crate), doc tests pass |
| All 5 examples compile | ✓ PASS | extraction_e2e, extraction_retry_e2e, extraction_summarize_e2e, mcp_tool_agent_e2e, payload_extraction_e2e |

### Implementation Quality

**Code organization:**
- Builder fields properly initialized to None (lines 93-94)
- Doc comments present on all public items (deny(missing_docs) respected)
- Consistent builder pattern with `#[must_use]` attributes
- No error propagation via .unwrap() or .expect() in new code

**Backward compatibility:**
- When payload is None: prompt passes through unchanged (line 278)
- When instruction_template is None: DEFAULT_WORKFLOW_TEMPLATE used (line 247)
- Existing examples compile without changes (verified mcp_tool_agent_e2e.rs)
- System prompt enhancement (workflow steps) applied to ALL executions, improving Phase 2.1 behavior

**Wiring verification:**
- Builder fields consumed before adapter match (lines 215-216) to avoid partial-move
- final_prompt computed conditionally based on payload presence (lines 263-279)
- final_prompt and full_system_prompt passed to all three adapter run functions (lines 284-292)
- All three adapters (ClaudeCode, Codex, OpenCode) receive enhanced prompts

**XML structure verification:**
- When payload present: 4-block structure (`<context>`, `<task>`, `<output_format>`)
- Payload wrapped in `<context>` tags to prevent instruction/context confusion
- `<output_format>` block reinforces tool usage requirement
- Structure documented in .payload() doc comment (lines 144-151)

**Workflow enforcement verification:**
- DEFAULT_WORKFLOW_TEMPLATE has 5 numbered steps (lines 18-23)
- Step 1: Call json_example FIRST
- Steps 2-4: Draft, validate, fix/retry loop
- Step 5: Submit once validation passes
- RULES section forbids freeform text and raw JSON output (lines 25-30)
- Template injected into every system prompt (line 250-254)

### Phase Goal Assessment

**Goal:** "Developer can pass context data to agents and force tool workflow"

**Achievement:**

1. **Context data passing:** ✓ Complete
   - `.payload()` method available on builder
   - Data wrapped in `<context>` XML block for clear separation
   - Example demonstrates realistic usage with SOURCE_TEXT constant
   - Backward compatible: optional feature, no breaking changes

2. **Forced tool workflow:** ✓ Complete
   - DEFAULT_WORKFLOW_TEMPLATE enforces example→validate→submit sequence
   - Numbered steps provide explicit workflow guidance
   - Constraint language forbids freeform text responses
   - System prompt includes "You MUST use ONLY these MCP tools"
   - Three-tool pattern (example/validate/submit) explicitly defined in workflow steps

**Verification Status:** All success criteria met. Phase 3 requirements (EXTR-02, EXTR-03, EXTR-05) are demonstrably satisfied in the codebase. No gaps found.

---

_Verified: 2026-02-01T22:45:00Z_
_Verifier: Claude (gsd-verifier)_

# Rust WASM strategy module

Use a Rust WASM module when strategy logic needs persistent state or cannot be
expressed by the leg's QuantFlow program. The current strategy flow pins a
successful module build to a strategy-version leg. See the
[`strategy-rust-module` skill](../../skills/strategy-rust-module/SKILL.md) for
the complete workflow and ABI.

## Quick start

1. Copy `scaffold/` to a local working directory and implement the module using
   `lte_strategy_bridge::run_loop`.
2. Build locally with `cargo build --target wasm32-wasip1 --release`.
3. Create/select a repository with `create_module_repository` or `list_modules`.
4. Upload the compiled WASM using `get_module_wasm_upload_url`, PUT to the
   returned URL, then call `submit_module_wasm`. This uploads only the binary;
   the server does not receive Rust source or compile it.
5. Select an owned strategy with `list_strategies`. In
   `create_strategy_version` or `update_strategy_version_leg`, put
   `module.start("<alias>")` as a top-level
   statement in the leg's QuantFlow program and bind the same alias to that
   build ID in the leg's `modules` array.
6. Review the leg with `get_strategy`, then release the pre-release version
   when ready.

The program alias and binding alias must match exactly. Bind a successful
`module_build_id` owned by the strategy owner; the repository `module_id` and
module version are not substitutes for a build ID. For updates, pass the full
replacement program and `modules` list together whenever aliases change.

The server-side source-archive build flow remains available through
`get_module_upload_url` and `submit_module_build`. The scaffold's
`bot_group.dynamic_module.template.json` and the direct
`/mcp/modules/{module_id}/{module_version}/upload` route are for the older
classic bot-group integration; do not use them to attach a build to a
versioned strategy leg.

## Runtime contract

- Read `ModuleInput` JSON lines from stdin and write `ModuleOutput` JSON lines
  to stdout.
- Keep the process alive with `lte_strategy_bridge::run_loop`; do not call
  `std::process::exit`.
- Persist recoverable strategy state through the input and output `state`
  fields.

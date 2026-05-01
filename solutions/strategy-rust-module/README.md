# Strategy Solution 2: Rust WASM Dynamic Module

Use this approach when strategy logic should be implemented in Rust and executed by the DynamicModule bridge.

## When to choose this mode

- Complex stateful logic that is hard to express in settings JSON
- Reusable strategy package with versioned WASM artifacts
- Fine control over entry and exit logic

## Scaffold

A starter scaffold is provided in ./scaffold:

- Cargo.toml
- src/main.rs
- module.manifest.template.json
- bot_group.dynamic_module.template.json

## Quick start

1. Copy scaffold:

```bash
cp -R solutions/strategy-rust-module/scaffold my-strategy
cd my-strategy
```

2. Update module metadata:

- Generate UUID for module_id
- Set version
- Set artifact key to modules/{module_id}/{version}/module.wasm

3. Build WASM:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

4. Upload module bytes via API:

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/modules/<MODULE_ID>/<VERSION>/upload" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @target/wasm32-wasip1/release/module_entry.wasm
```

5. Create or update bot group with DynamicModule settings.

## Runtime contract

- Read ModuleInput JSON from stdin
- Write ModuleOutput JSON to stdout
- Do not call std::process::exit in WASM entrypoint; return from main
- Persist strategy state via state in input and output
# Strategy Solution 2: Rust WASM Dynamic Module

Use this approach when strategy logic should be implemented in Rust and executed by the DynamicModule bridge.

## When to choose this mode

- Complex stateful logic that is hard to express in settings JSON.
- Reusable strategy package with versioned WASM artifacts.
- Fine control over entry/exit logic and internal state.

## Scaffold

A starter scaffold is provided in ./scaffold.

- `Cargo.toml`
- `src/main.rs`
- `module.manifest.template.json`
- `bot_group.dynamic_module.template.json`

## Quick start

1. Copy scaffold:

```bash
cd /path/to/work
cp -R /home/dev/dev/strategy-skill/solutions/strategy-rust-module/scaffold my-strategy
cd my-strategy
```

2. Update module metadata:

- Generate UUID for `module_id`
- Set `version`
- Set artifact key to `modules/{module_id}/{version}/module.wasm`

3. Build WASM:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

4. Upload module bytes via API:

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/modules/<MODULE_ID>/<VERSION>/upload" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @target/wasm32-wasip1/release/module_entry.wasm
```

5. Create/update bot group using `DynamicModule` strategy in settings.

## Runtime contract

- Read `ModuleInput` JSON from stdin.
- Write `ModuleOutput` JSON to stdout.
- Do not call `std::process::exit` inside WASM entrypoint; return from `main` normally.
- Persist strategy state using `state` field in input/output.

## Required env

```bash
export LIVTORGEX_SKILL_URL="https://skill.api.livtorgex.com"
export LIVTORGEX_SKILL_TOKEN="lt_..."
```

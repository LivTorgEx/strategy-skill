---
name: strategy-rust-module
description: >
  Create, build, and deploy a Rust WASM DynamicModule strategy to a LivTorgEx server.
  Use when the user asks to create a new WASM-based strategy module, scaffold a Rust strategy,
  build a WASM binary, upload a module, or configure a bot group with DynamicModule settings.
---

# LivTorgEx — Rust WASM Dynamic Module

## Environment variables

| Variable | Description |
|----------|-------------|
| `LIVTORGEX_SKILL_URL` | Skill API base URL, e.g. `http://localhost:8003` |
| `LIVTORGEX_SKILL_TOKEN` | Personal access token (`lt_<...>`) — get from `/skill/connect` |

---

## Overview

A DynamicModule strategy is a WASM binary compiled from Rust. The bridge
re-executes it on every tick, passing `ModuleInput` via stdin and reading
`ModuleOutput` from stdout. Persistent state is threaded through the `state`
field in both structs.

---

## Workflow

### Step 1 — Scaffold

Clone the scaffold from the official repository:

```bash
git clone https://github.com/LivTorgEx/strategy-skill /tmp/strategy-skill-samples
cp -R /tmp/strategy-skill-samples/solutions/strategy-rust-module/scaffold <my-strategy>
cd <my-strategy>
```

The scaffold contains:

- `Cargo.toml` (binary name: `module_entry`, git dep on `lte_strategy_bridge`)
- `src/main.rs` (reads `ModuleInput` from stdin, writes `ModuleOutput` to stdout)
- `module.manifest.template.json`
- `bot_group.dynamic_module.template.json`

### Step 2 — Module metadata

Generate a UUID for `module_id` and decide a `version` string (e.g. `0.1.0`).

Update `module.manifest.template.json` → save as `module.manifest.json`:

```json
{
  "module_id": "<UUID>",
  "name": "<strategy-name>",
  "version": "<version>",
  "runtime": "wasm",
  "target": "wasm32-wasip1",
  "artifact": {
    "bucket": "strategy-modules",
    "object_key": "modules/<UUID>/<version>/module.wasm"
  }
}
```

Generate a UUID if the user does not provide one:

```bash
uuidgen
```

### Step 3 — Implement strategy logic

Edit `src/main.rs`. Key contract:

- Deserialise `ModuleInput` from stdin (use `serde_json`).
- Write a single `ModuleOutput` JSON line to stdout.
- Do **not** call `std::process::exit` — return from `main` normally.
- Persist cross-tick state in `ModuleOutput.state`; load it from `ModuleInput.state`.

```rust
use std::io::{self, Read};
use lte_strategy_bridge::abi::{ModuleInput, ModuleOutput, ModuleOpenPosition};

fn main() {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw).unwrap_or_default();
    let input: ModuleInput = serde_json::from_str(&raw).unwrap_or_default();

    // --- strategy logic here ---

    let out = ModuleOutput {
        open_positions: vec![],
        ..Default::default()
    };
    println!("{}", serde_json::to_string(&out).unwrap());
}
```

`ModuleOpenPosition` fields: `direction` (`"Long"` | `"Short"`), `amount_ratio` (0..1,
fraction of `max_open_amount`), `enter_price` (optional, None = market), `take_profit`,
`stop_loss`, `mark` (stable unique string per logical entry).

To place or cancel limit orders within an open position, set:
- `ModuleOutput.place_orders: Vec<ModulePlaceOrder>` — upserted by `mark`
- `ModuleOutput.cancel_orders: Vec<String>` — marks to cancel

### Step 4 — Build WASM

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

Artifact: `target/wasm32-wasip1/release/module_entry.wasm`

### Step 5 — Upload via Skill API

Read `module_id` and `version` from `module.manifest.json`, then upload:

```bash
MODULE_ID=$(jq -r .module_id module.manifest.json)
VERSION=$(jq -r .version module.manifest.json)

curl -s -X POST "$LIVTORGEX_SKILL_URL/api/modules/$MODULE_ID/$VERSION/upload" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @target/wasm32-wasip1/release/module_entry.wasm
```

Success response:

```json
{
  "module_id": "<UUID>",
  "module_version": "<version>",
  "bucket": "strategy-modules",
  "object_key": "modules/<UUID>/<version>/module.wasm",
  "size_bytes": 12345
}
```

Constraints:
- `module_id` must be a valid UUID.
- `module_version` must be 1..64 chars, only `[A-Za-z0-9._-]`.
- Max file size: 16 MiB.
- HTTP 403 → `skill_access` for that module record is `"Deny"` or `"Read"`. User must set it to `"Edit"` in **Account → Skill → Module Access**.

### Step 6 — Create or update bot group

Use `bot_group.dynamic_module.template.json` as the base settings, then fill in
module_id, version, max_open_amount, direction, and other bot group fields.

Bot group `strategy` block:

```json
{
  "name": "DynamicModule",
  "module_id": "<UUID>",
  "module_version": "<version>",
  "max_open_amount": 500.0,
  "direction": "BOTH"
}
```

`direction` options: `"BOTH"`, `"Long"`, `"Short"`.

Deploy with the Skill API (same as settings-based strategy):

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/bot_group" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '<FULL_FORM_JSON>'
```

`"created": true` = new group. `"created": false` = updated.

---

## Re-deploying an updated module

1. Bump `version` in `module.manifest.json` (e.g. `0.1.0` → `0.2.0`).
2. Rebuild: `cargo build --target wasm32-wasip1 --release`
3. Upload again (Step 5).
4. Update the bot group's `module_version` (Step 6).

---

## skill_access

Each module record has `skill_access`: `"Edit"` (can upload), `"Read"` (list only),
`"Deny"` (blocked). HTTP 403 → user must set it to `"Edit"` in
**Account → Skill → Module Access**.
  
---

## notes

- The bridge loads the WASM binary from S3/MinIO once at bot startup. To pick up
  a new binary, restart the affected bots (stop + start via the UI or worker).
- Use a stable `mark` string per logical entry (e.g. `"long-dca-t2"`) so the bridge
  can upsert/cancel orders idempotently.
- Keep `module_version` pinned in production; use `"latest"` only during development.

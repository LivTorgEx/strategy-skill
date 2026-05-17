---
name: strategy-rust-module
description: >
  Create, build, and deploy a Rust WASM DynamicModule strategy to a LivTorgEx server.
  Use when the user asks to create a new WASM-based strategy module, scaffold a Rust strategy,
  build a WASM binary, upload a module, or configure a bot group with DynamicModule settings.
---

# LivTorgEx — Rust WASM Dynamic Module

## Overview

A DynamicModule strategy is a WASM binary compiled from Rust. The bridge
re-executes it on every tick, passing `ModuleInput` via stdin and reading
`ModuleOutput` from stdout. Persistent state is threaded through the `state`
field in both structs.

The full lifecycle is:
1. Create a **module repository** (gets a `module_id` UUID).
2. Write Rust strategy code.
3. Package source as a `.tar.gz`, base64-encode it.
4. **Submit a build** — the builder server compiles `wasm32-wasip1` in a container.
5. **Poll for build status** — on failure, read `build_output` compiler errors, fix, and re-submit.
6. On success, deploy the module version to a bot group.

---

## Workflow

### Step 1 — Create a module repository

Call MCP tool: `create_module_repository`
Arguments: `{ "name": "<strategy-name>", "description": "optional" }`

Returns `module_id` (UUID) — the stable identifier for all future versions of this strategy.

To list existing repositories:
Call MCP tool: `list_modules` (no arguments)

---

### Step 2 — Implement strategy logic

Write a Rust binary crate. The scaffold lives at
`strategy-skill/solutions/strategy-rust-module/scaffold/`.

Key contract for `src/main.rs`:
- Read `ModuleInput` from stdin (`serde_json`).
- Write exactly one `ModuleOutput` JSON line to stdout.
- Do **not** call `std::process::exit` — return from `main` normally.
- Persist cross-tick state in `ModuleOutput.state`; load from `ModuleInput.state`.

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

#### `ModulePlaceOrder` — full field reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `Direction` | — | `Long` or `Short` — which side's position this order belongs to |
| `amount_ratio` | `f64` | — | Fraction of `max_open_amount` (0..1) |
| `enter_price` | `f64` | — | Limit price for the order |
| `take_profit` | `Option<f64>` | `None` | TP attached to this order |
| `stop_loss` | `Option<f64>` | `None` | SL attached to this order |
| `mark` | `String` | — | Stable unique identifier — used to upsert/cancel by the host |
| `order_side` | `ModuleOrderSide` | `Buy` | Exchange order side: `Buy` for entries/DCA, `Sell` to reduce a Long, `Buy` to reduce a Short |
| `reduce_only` | `bool` | `false` | When `true` the host applies the order only to an already-open position. **Never buffered** — silently dropped if no position is open. Use for all partial-close orders. |

**`ModuleOrderSide` values:**
```rust
pub enum ModuleOrderSide {
    Buy,  // default — entry / DCA orders
    Sell, // reduce a Long position (or entry when direction = Short)
}
```

**Partial-close (reduce-only) order pattern:**
```rust
use lte_strategy_bridge::abi::{ModulePlaceOrder, ModuleOrderSide, Direction};

// Partial TP that sells a fraction of an open Long position:
ModulePlaceOrder {
    direction: Direction::Long,
    amount_ratio: 0.25,          // close 25% of the position
    enter_price: target_price,
    take_profit: None,
    stop_loss: None,
    mark: "tp-partial-1".to_string(),
    order_side: ModuleOrderSide::Sell,
    reduce_only: true,           // REQUIRED for partial-close orders
}
```

**IMPORTANT rules for `reduce_only`:**
- Always set `reduce_only: true` on partial-close / partial-TP orders.
- Never set it on entry or DCA orders (`reduce_only: false` or omit).
- The host host will NOT buffer a `reduce_only` order if no matching position
  is currently open — it logs a warning and skips it.
- This ensures partial-close orders are direction-agnostic: the module declares
  intent explicitly rather than relying on the host inferring it from the side.

---

### Step 3 — Package source

The project **must** be a valid Cargo workspace or single-crate project.
The builder runs `cargo build --target wasm32-wasip1 --release` inside it.

> **`lte_strategy_bridge` must use a git dependency** — not a `path =` dependency.
> The build server is isolated and has no access to the local workspace.
> Declare it in `Cargo.toml` as:
> ```toml
> lte_strategy_bridge = { git = "https://github.com/LivTorgEx/lte_strategy_bridge.git", rev = "<commit-sha>" }
> ```
> Pin to the exact commit that matches the server's ABI version.

Package the project root:

```bash
# From the directory that contains Cargo.toml:
tar --exclude=./target --exclude=./.git -czf /tmp/module-src.tar.gz .
base64 -w 0 /tmp/module-src.tar.gz
```

Keep the source under **1 MB** (compressed). Remove large test fixtures or generated
files if the source exceeds this limit.

---

### Step 4 — Submit a build

Call MCP tool: `submit_module_build`
Arguments:
```json
{
  "module_id": "<UUID from Step 1>",
  "module_version": "0.1.0",
  "source_tar_gz_base64": "<base64 string from Step 3>"
}
```

Returns `build_id`. `module_version` constraints: 1–64 chars, only `[A-Za-z0-9._-]`.

---

### Step 5 — Poll build status

Call MCP tool: `get_module_build_status`
Arguments: `{ "build_id": "<UUID>" }`

**Status values:**
| Status | Meaning |
|--------|---------|
| `Pending` | Queued, not yet started |
| `Building` | Compilation in progress |
| `Success` | WASM artifact ready |
| `Failed` | Compilation failed — read `build_output` |

Poll every 5–10 seconds until `status` is `Success` or `Failed`.

**On `Failed`:** read `build_output` for compiler errors, fix the Rust source,
re-package (Step 3), and submit a new build (Step 4) with the same or a new version.

To see all builds for a module:
Call MCP tool: `list_module_builds`
Arguments: `{ "module_id": "<UUID>" }`

---

### Step 6 — Deploy to a bot group

Once build status is `Success`, use `upsert_bot_group` (see `create-strategy` skill).
Set the `settings.strategy` block to:

```json
{
  "name": "DynamicModule",
  "module_id": "<UUID>",
  "module_version": "0.1.0",
  "max_open_amount": 500.0,
  "direction": "BOTH"
}
```

`direction` options: `"BOTH"`, `"Long"`, `"Short"`.

---

## Re-deploying an updated module

1. Fix the Rust source.
2. Re-package and re-submit (Steps 4–5) with a **new version string** (e.g. `0.2.0`).
3. Poll until `Success`.
4. Update the bot group's `module_version` via `upsert_bot_group`.

---

## Notes

- The bridge loads the WASM binary from S3/MinIO once at bot startup. To pick up
  a new binary, restart the affected bots (stop + start via the UI or worker).
- Use a stable `mark` string per logical entry (e.g. `"long-dca-t2"`) so the bridge
  can upsert/cancel orders idempotently.
- Keep `module_version` pinned in production.
- Build timeout: 240 seconds. If the build times out, status will be `Failed`
  with `build_output` = `"build timed out"`.
- `build_output` is capped at 32 KB.
- `create_module_repository` and `submit_module_build` are write-scoped tools.
  The server admin must include them in `MCP_ALLOWED_WRITE_TOOLS` for them to
  appear as enabled in the tool list.


# LivTorgEx — Rust WASM Dynamic Module

## Overview

A DynamicModule strategy is a WASM binary compiled from Rust. The bridge
re-executes it on every tick, passing `ModuleInput` via stdin and reading
`ModuleOutput` from stdout. Persistent state is threaded through the `state`
field in both structs.

The full lifecycle is:
1. Create a **module repository** (gets a `module_id` UUID).
2. Write Rust strategy code.
3. Package source as a `.tar.gz`, base64-encode it.
4. **Submit a build** — the builder server compiles `wasm32-wasip1` in a container.
5. **Poll for build status** — on failure, read `build_output` compiler errors, fix, and re-submit.
6. On success, deploy the module version to a bot group.

---

## Workflow

### Step 1 — Create a module repository

```
Call MCP tool: create_module_repository
Arguments: {
  "name": "<strategy-name>",
  "description": "Optional description"
}
```

Response:
```json
{
  "module_id": "<UUID>",
  "name": "<strategy-name>",
  "description": "...",
  "created_at": 1748000000000,
  "updated_at": 1748000000000
}
```

Save `module_id` — it is the stable identifier for all future versions of this strategy.

To list existing repositories:
```
Call MCP tool: list_modules
```

---

### Step 2 — Implement strategy logic

Write a Rust binary crate. The scaffold lives at
`strategy-skill/solutions/strategy-rust-module/scaffold/`.

Key contract for `src/main.rs`:
- Read `ModuleInput` from stdin (`serde_json`).
- Write exactly one `ModuleOutput` JSON line to stdout.
- Do **not** call `std::process::exit` — return from `main` normally.
- Persist cross-tick state in `ModuleOutput.state`; load from `ModuleInput.state`.

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

---

### Step 3 — Package source

The project **must** be a valid Cargo workspace or single-crate project.
The builder runs `cargo build --target wasm32-wasip1 --release` inside it.

> **`lte_strategy_bridge` must use a git dependency** — not a `path =` dependency.
> The build server is isolated and has no access to the local workspace.
> Declare it in `Cargo.toml` as:
> ```toml
> lte_strategy_bridge = { git = "https://github.com/LivTorgEx/lte_strategy_bridge.git", rev = "<commit-sha>" }
> ```

Package the project root:

```bash
# From the directory that contains Cargo.toml:
tar --exclude=./target --exclude=./.git -czf /tmp/module-src.tar.gz .
```

Base64-encode for API submission:
```bash
base64 -w 0 /tmp/module-src.tar.gz
```

Keep the source under **1 MB** (compressed). If the source exceeds this limit, remove
large test fixtures or generated files before packaging.

---

### Step 4 — Submit a build

```
Call MCP tool: submit_module_build
Arguments: {
  "module_id": "<UUID from Step 1>",
  "module_version": "0.1.0",
  "source_tar_gz_base64": "<base64 string>"
}
```

Response:
```json
{
  "build_id": "<UUID>",
  "module_id": "<UUID>",
  "module_version": "0.1.0",
  "status": "Pending"
}
```

`module_version` constraints: 1–64 chars, only `[A-Za-z0-9._-]`.

---

### Step 5 — Poll build status

```
Call MCP tool: get_module_build_status
Arguments: { "build_id": "<UUID>" }
```

Response:
```json
{
  "build_id": "<UUID>",
  "module_id": "<UUID>",
  "module_version": "0.1.0",
  "status": "Building",
  "build_output": null,
  "wasm_s3_key": null,
  "created_at": 1748000000000,
  "started_at": 1748000001000,
  "finished_at": null
}
```

**Status values:**
| Status | Meaning |
|--------|---------|
| `Pending` | Queued, not yet started |
| `Building` | Compilation in progress |
| `Success` | WASM artifact ready |
| `Failed` | Compilation failed — read `build_output` |

Poll every 5–10 seconds until `status` is `Success` or `Failed`.

**On `Failed`:** read `build_output` for compiler errors, fix the Rust source,
re-package (Step 3), and submit a new build (Step 4) with the same or a new version.

To see all builds for a module:
```
Call MCP tool: list_module_builds
Arguments: { "module_id": "<UUID>" }
```

---

### Step 6 — Deploy to a bot group

Once the build status is `Success`, configure the bot group `strategy` block:

```json
{
  "name": "DynamicModule",
  "module_id": "<UUID>",
  "module_version": "0.1.0",
  "max_open_amount": 500.0,
  "direction": "BOTH"
}
```

`direction` options: `"BOTH"`, `"Long"`, `"Short"`.

Deploy with the MCP tool:

```
Call MCP tool: upsert_bot_group
Arguments: <FULL_FORM_JSON>
```

`"created": true` = new group. `"created": false` = updated.

---

## Re-deploying an updated module

1. Fix the Rust source.
2. Re-package and re-submit (Steps 3–4) with a **new version string** (e.g. `0.2.0`).
3. Poll until `Success`.
4. Update the bot group's `module_version` (Step 6).

---

## Notes

- The bridge loads the WASM binary from S3/MinIO once at bot startup. To pick up
  a new binary, restart the affected bots (stop + start via the UI or worker).
- Use a stable `mark` string per logical entry (e.g. `"long-dca-t2"`) so the bridge
  can upsert/cancel orders idempotently.
- Keep `module_version` pinned in production; use `"latest"` only during development.
- Build timeout: 240 seconds. If the build times out, the status will be `Failed`
  with output `"build timed out"`.
- `build_output` is capped at 32 KB. If larger, only the last 32 KB of compiler
  output is stored.


# LivTorgEx — Rust WASM Dynamic Module

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

#### `ModulePlaceOrder` — full field reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `Direction` | — | `Long` or `Short` — which side's position this order belongs to |
| `amount_ratio` | `f64` | — | Fraction of `max_open_amount` (0..1) |
| `enter_price` | `f64` | — | Limit price for the order |
| `take_profit` | `Option<f64>` | `None` | TP attached to this order |
| `stop_loss` | `Option<f64>` | `None` | SL attached to this order |
| `mark` | `String` | — | Stable unique identifier — used to upsert/cancel by the host |
| `order_side` | `ModuleOrderSide` | `Buy` | Exchange order side: `Buy` for entries/DCA, `Sell` to reduce a Long, `Buy` to reduce a Short |
| `reduce_only` | `bool` | `false` | When `true` the host applies the order only to an already-open position. **Never buffered** — silently dropped if no position is open. Use for all partial-close orders. |

**`ModuleOrderSide` values:**
```rust
pub enum ModuleOrderSide {
    Buy,  // default — entry / DCA orders
    Sell, // reduce a Long position (or entry when direction = Short)
}
```

**Partial-close (reduce-only) order pattern:**
```rust
use lte_strategy_bridge::abi::{ModulePlaceOrder, ModuleOrderSide, Direction};

// Partial TP that sells a fraction of an open Long position:
ModulePlaceOrder {
    direction: Direction::Long,
    amount_ratio: 0.25,          // close 25% of the position
    enter_price: target_price,
    take_profit: None,
    stop_loss: None,
    mark: "tp-partial-1".to_string(),
    order_side: ModuleOrderSide::Sell,
    reduce_only: true,           // REQUIRED for partial-close orders
}
```

**IMPORTANT rules for `reduce_only`:**
- Always set `reduce_only: true` on partial-close / partial-TP orders.
- Never set it on entry or DCA orders (`reduce_only: false` or omit).
- The host will NOT buffer a `reduce_only` order if no matching position
  is currently open — it logs a warning and skips it.
- This ensures partial-close orders are direction-agnostic: the module declares
  intent explicitly rather than relying on the host inferring it from the side.

### Step 4 — Submit source for server-side build

Package the project as a `.tar.gz` and submit it to the MCP build service. The server
compiles `wasm32-wasip1` in a container — no local Rust toolchain required.

> **`lte_strategy_bridge` must use a git dependency** in `Cargo.toml` (not `path =`).
> The build server has no access to the local workspace.

```bash
MODULE_ID=$(jq -r .module_id module.manifest.json)
VERSION=$(jq -r .version module.manifest.json)

# Pack source (must stay under 1 MB compressed)
tar --exclude=./target --exclude=./.git -czf /tmp/module-src.tar.gz .
SOURCE_B64=$(base64 -w 0 /tmp/module-src.tar.gz)
```

Then submit via the MCP tool:

```
Call MCP tool: submit_module_build
Arguments: {
  "module_id": "$MODULE_ID",
  "module_version": "$VERSION",
  "source_tar_gz_base64": "$SOURCE_B64"
}
```

Returns `{ "build_id": "<UUID>", "status": "Pending" }`.

### Step 5 — Poll build status

```
Call MCP tool: get_module_build_status
Arguments: { "build_id": "<BUILD_ID>" }
```

**Status values:** `Pending` → `Building` → `Success` / `Failed`.

On `Failed`: read `build_output` for compiler errors, fix the source, re-package
(Step 4) with a new version string.

To see all builds for a module:

```
Call MCP tool: list_module_builds
Arguments: { "module_id": "$MODULE_ID" }
```

Build timeout: 240 seconds. `build_output` is capped at 32 KB.

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

Deploy with the MCP tool (same as settings-based strategy):

```
Call MCP tool: upsert_bot_group
Arguments: <FULL_FORM_JSON>
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

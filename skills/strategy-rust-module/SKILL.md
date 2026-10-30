---
name: strategy-rust-module
description: >
  Create, locally build, upload, and attach a Rust WASM module to a LivTorgEx strategy-version leg.
  Use when the user asks to create a new WASM-based strategy module, scaffold a Rust strategy,
  build a WASM binary, upload a module, or attach a module to a QuantFlow strategy leg.
---

# LivTorgEx — Rust WASM strategy module

## Overview

A Rust WASM module is a pinned strategy-leg capability. The platform starts a
**persistent WASM instance** with the bot — the module runs a
stdin/stdout loop (`lte_strategy_bridge::run_loop`) and stays alive for the
bot's entire lifetime. The bridge feeds events as JSON lines to stdin and reads
responses from stdout. State lives in the module's memory (plain Rust variables)
— no serialization round-trip needed.

Each bot runs as a **tokio task** (green thread), not an OS thread — this scales
to 10k+ bots on a single machine. Between events the module yields to the
runtime (zero CPU while idle).

The active strategy-leg lifecycle is:
1. Create a **module repository** (gets a `module_id` UUID).
2. Write Rust strategy code using `lte_strategy_bridge::run_loop`.
3. Build the WASM locally and upload the binary with the prebuilt-WASM MCP tools.
4. Pin the resulting successful `build_id` to a strategy-version leg with a matching QuantFlow `module.start` alias.
5. Explicitly release the pre-release strategy version when the leg is ready.

Local builds are recommended when Rust source should stay on the developer's machine. The source-archive/server-build flow remains available as an alternative.

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
- Use `lte_strategy_bridge::run_loop` — it handles stdin/stdout framing, JSON parsing, panic recovery, and error reporting.
- Your handler receives a typed `ModuleInput` and returns a `ModuleOutput`.
- Do **not** call `std::process::exit` — the module must keep running. If it exits, the bridge logs an error and all subsequent events fail.
- State lives in-memory as plain Rust variables — no serialization needed.

```rust
use lte_strategy_bridge::{ModuleInput, ModuleOutput};

#[derive(Default)]
struct State {
    // your strategy state here — survives across all events
}

fn main() {
    let mut state = State::default();
    lte_strategy_bridge::run_loop(|input| run(&input, &mut state));
}

fn run(input: &ModuleInput, state: &mut State) -> ModuleOutput {
    // --- strategy logic here ---
    ModuleOutput::default()
}
```

See the **ABI Reference** section below for full type documentation.

---

### Recommended — build and upload WASM locally

Install the WASI target and compile the artifact on the development machine:

```bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
```

The artifact is normally `target/wasm32-wasip1/release/<crate-name>.wasm`. Call `get_module_wasm_upload_url` with the repository `module_id` and `module_version`, PUT the artifact to the returned URL, then call `submit_module_wasm` with the same identifiers. This sends the compiled binary only; it does not upload Rust source or invoke the server builder. Every submission creates an immutable build revision, including when you reuse an existing version label. The result returns the new `build_id`.

To update a module while keeping its version label, repeat this upload flow with the same `module_version`. Use `list_module_builds` to find the newest successful revision and its build ID. Existing strategy legs remain pinned to their current build; update the leg's `module_build_id` explicitly to adopt the new revision.

```bash
curl -fS -X PUT -T target/wasm32-wasip1/release/<crate-name>.wasm "$UPLOAD_URL"
```

The upload limit is 32 MiB. The module version must be 1–64 characters from `[A-Za-z0-9._-]`.

### Optional — build on the server from a source archive

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
mkdir -p dist
tar --exclude=./target --exclude=./.git -czf dist/module-src.tar.gz .
```

Keep the source under **1 MB** (compressed). Remove large test fixtures or generated
files if the source exceeds this limit.

---

### Step 4 — Upload source via presigned URL

**4a.** Get a presigned upload URL:

Call MCP tool: `get_module_upload_url`
Arguments:
```json
{
  "module_id": "<UUID from Step 1>",
  "module_version": "0.1.0"
}
```

Returns `upload_url` (valid for 5 minutes) and `expires_in`.

**4b.** Upload the archive directly to S3:

```bash
curl -X PUT -T dist/module-src.tar.gz "$UPLOAD_URL"
```

The binary goes directly from disk to S3 — no base64 encoding, no JSON serialization.

---

### Step 5 — Submit the build

Call MCP tool: `submit_module_build`
Arguments:
```json
{
  "module_id": "<UUID from Step 1>",
  "module_version": "0.1.0"
}
```

The server validates the uploaded archive (size, tar.gz integrity, Cargo.toml presence),
then queues the build. Returns `build_id`.

`module_version` constraints: 1–64 chars, only `[A-Za-z0-9._-]`.

---

### Step 6 — Poll build status

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
re-package (Step 3), re-upload (Step 4), and submit a new build (Step 5) with the same or a new version.

To see all builds for a module:
Call MCP tool: `list_module_builds`
Arguments: `{ "module_id": "<UUID>" }`

The result lists revisions newest first and includes each revision number and immutable `build_id`.

---

### Step 7 — Attach the build to a strategy-version leg

Use `list_strategies` to select the owned `strategy_id`. Call `create_strategy_version` to add a leg to a new pre-release version, or `update_strategy_version_leg` to edit an existing unreleased leg. Put the alias declaration in the leg's QuantFlow source and pin the successful build ID in `modules`:

The following object is one item in `create_strategy_version.legs`:

```json
{
  "leg_key": "eth_power",
  "name": "ETH Power",
  "provider_code": "<provider code>",
  "program": {
    "schema": "quantflow_bot_v1",
    "program": "module.start(\"eth_power\")"
  },
  "modules": [
    {
      "alias": "eth_power",
      "module_build_id": "<successful build_id>"
    }
  ]
}
```

`module.start` must be a top-level statement. Aliases must be unique, lowercase, start with a letter, and contain only lowercase letters, digits, or underscores (maximum 64 characters). The `modules` aliases must exactly match all `module.start` declarations in that leg's program. `module_build_id` is the ID returned by `submit_module_wasm` or `list_module_builds`, not the repository ID or version string. The build must have status `Success` and belong to the strategy owner.

When updating an existing leg, omit `modules` to preserve its current bindings. If changing any `module.start` aliases, pass the complete new `program` and `modules` array together. Pass `modules: []` to clear all bindings and remove all `module.start` declarations. Use `get_strategy` to verify the leg's `program` and bindings, then call `release_strategy_version` when ready; released versions are immutable.

---

## Updating a pinned module

1. Update the Rust source and build a new WASM artifact locally.
2. Upload and submit it with a new module version to get a new successful `build_id`.
3. Create a new pre-release strategy version or update an unreleased leg to pin that build ID.
4. Review and explicitly release the new version when ready.

---

## MCP access

The module repository and strategy leg must belong to the authenticated user. Repository creation, upload URL generation, upload submission, and leg edits require the MCP `mcp:write` scope.

---

## Notes

- The bridge loads the WASM binary from S3/MinIO once at bot startup and spawns a
  persistent instance. To pick up a new binary, restart the affected bots
  (stop + start via the UI or worker).
- The module **must run a persistent stdin loop** (via `lte_strategy_bridge::run_loop`).
  If `_start` returns or the module calls `process::exit`, the bridge logs an error
  and all subsequent events for that bot will fail.
- Use a stable `mark` string per logical entry (e.g. `"long-dca-t2"`) so the bridge
  can upsert/cancel orders idempotently.
- Strategy legs pin an exact `module_build_id`; they never resolve a floating `latest` build.
- Build timeout: 240 seconds. If the build times out, status will be `Failed`
  with `build_output` = `"build timed out"`.
- `build_output` is capped at 32 KB.
- Both local WASM upload tools and source-archive build tools require the MCP `mcp:write` scope.

---

## Bot Context & Crash Recovery

The bridge persists module state to the **bot context** in the database (throttled to every ~80 seconds). This serves two purposes:

1. **Crash recovery**: On server restart, the saved `state` is passed back to the module via `ModuleInput.state` on the first event. The module can restore its in-memory state from this.
2. **User visibility**: The bot context is visible in the UI, allowing the user to trace strategy decisions in real time. The context includes `realised_pnl`, `auto_max_amount`, the last `debug` message, and the module's `state`.

**To enable crash recovery**, set `ModuleOutput.state` with a JSON-serializable snapshot of your strategy state. The module should check `input.state` on startup and restore from it:

```rust
fn run(input: &ModuleInput, state: &mut State) -> ModuleOutput {
    if let Some(saved) = &input.state {
        if let Ok(restored) = serde_json::from_value::<State>(saved.clone()) {
            *state = restored;
        }
    }
    // ... strategy logic ...
    ModuleOutput {
        state: Some(serde_json::to_value(&state).unwrap()),
        debug: format!("summary of what the strategy is doing"),
        ..Default::default()
    }
}
```

**Key points:**
- `input.state` is only non-`None` on the first event after a restart — use in-memory state for normal operation.
- `output.state` is written to DB every ~80s — keep it small (avoid large arrays or history buffers).
- `output.debug` is saved in the bot context for user tracing — put human-readable strategy status here.

---

## Alerts

The module can emit alert notifications to the user by returning `ModuleAlert` structs in `ModuleOutput.alerts`. Each alert is delivered as a bot notification through the same channel as Pro strategy alerts (push notifications, Telegram, etc.).

Use alerts for important strategy events the user should be aware of — signal triggers, risk warnings, position milestones, etc.

### ModuleAlert

| Field | Type | Description |
|-------|------|-------------|
| `message` | `String` | Alert text delivered to the user |

```rust
use lte_strategy_bridge::ModuleAlert;

fn run(input: &ModuleInput, state: &mut State) -> ModuleOutput {
    let mut alerts = Vec::new();

    if state.detected_reversal {
        alerts.push(ModuleAlert {
            message: format!("Trend reversal detected at {:.2}", input.price),
        });
    }

    if state.drawdown_pct > 5.0 {
        alerts.push(ModuleAlert {
            message: format!("Drawdown warning: {:.1}%", state.drawdown_pct),
        });
    }

    ModuleOutput {
        alerts,
        ..Default::default()
    }
}
```

**Key points:**
- Each `ModuleAlert` becomes a separate notification.
- Alerts are drained after each tick — duplicates are not suppressed, so guard emission in your logic.
- The notification includes the bot name and symbol automatically.

---

## Auto-Reinvest

The platform automatically reinvests realised profit into position sizing. When a position closes, the net PnL (profit minus fees) is accumulated and used to increase the effective max position budget (`auto_max_amount`).

**Formula:**
```
auto_max_amount = max_open_amount + realised_pnl * reinvest_percent * leverage
```

- `max_open_amount` — initial position size from bot group settings (the base budget).
- `realised_pnl` — cumulative net profit (after fees) across all closed positions in this bot run.
- `reinvest_percent` — fraction of profit to reinvest (0.0–1.0). Default: `1.0` (100%).
- `leverage` — exchange leverage set on the bot group.

**Example:** With `max_open_amount = 500`, `reinvest_percent = 1.0`, `leverage = 100`, and $2 net profit earned so far:
```
auto_max_amount = 500 + 2 * 1.0 * 100 = 700
```
The bot can now open positions up to $700 notional instead of the original $500.

**Bot group settings:**
```json
{
  "name": "DynamicModule",
  "module_id": "<UUID>",
  "module_version": "0.1.0",
  "max_open_amount": 500.0,
  "reinvest_percent": 1.0,
  "direction": "BOTH"
}
```

Set `reinvest_percent` to `0.0` to disable compounding entirely — `auto_max_amount` will always equal `max_open_amount`.

**Key points:**
- `auto_max_amount` is passed to the WASM module in every `ModuleInput` — the module can use it for sizing decisions.
- `max_open_amount` is the base guard; `auto_max_amount` only grows above it when profit is positive.
- Realised PnL is net of trading fees (matching Pro strategy behavior).
- The value is persisted to bot context and restored on restart — profit accumulation survives server restarts.
- If cumulative PnL goes negative, `auto_max_amount` drops below `max_open_amount` — the system uses whichever is larger as the effective cap.

---

# ABI Reference — `lte_strategy_bridge`

All types below are from the `lte_strategy_bridge` crate. This is the complete
contract between the WASM module and the host bridge. You do NOT need to read
the source code — everything is documented here.

---

## Direction

```rust
pub enum Direction { Long, Short, Both }
```

JSON serialization: `"LONG"`, `"SHORT"`, `"BOTH"`. Default: `Both`.

---

## ModuleInput

Passed to the WASM module via stdin on every tick.

| Field | Type | Description |
|-------|------|-------------|
| `event` | `ModuleEvent` | What triggered this tick |
| `price` | `f64` | Current market price |
| `market_decision` | `Option<ModuleMarketDecision>` | Market snapshot on `market_decision` events; `None` on other events |
| `symbol` | `String` | Trading pair (e.g. `"BTC-USDT-SWAP"`) |
| `max_amount` | `f64` | Base max position size in USD (from bot group settings) |
| `auto_max_amount` | `f64` | Effective max after auto-reinvest compounding: `max_amount + realised_pnl * reinvest_percent * leverage`. Falls back to `max_amount` when no PnL accumulated. See **Auto-Reinvest** below |
| `leverage` | `i32` | Exchange leverage |
| `symbol_info` | `ModuleSymbolInfo` | Trading rules for the symbol (tick size, lot size, minimums) |
| `klines` | `ModuleKlineSnapshot` | Requested candle history on `init`; new or revised candles on `kline_update` |
| `internal_indicators` | `ModuleInternalIndicatorSnapshot` | Typed platform indicator data. NTPS is currently supported; see **Internal Indicators** below |
| `indicators` | `BTreeMap<i64, Vec<HashMap<String, HashMap<String, ModuleIndicatorValue>>>>` | Reserved for future dynamic indicators. Hosts currently leave it empty |
| `positions` | `ModulePositions` | Current open positions summary |
| `state` | `Option<serde_json::Value>` | On first event after restart: restored from DB (last persisted `ModuleOutput.state`). Otherwise `None` — use in-memory Rust state for normal operation |

---

## Data requirements

Return `ModuleDataRequirements` from the `requirements` event. The host loads
the requested history before sending `init`, then sends only incremental
updates as data becomes available.

```rust
use lte_strategy_bridge::{InternalIndicatorKind, ModuleDataRequirements, MODULE_ABI_VERSION};
use std::collections::BTreeMap;

ModuleDataRequirements {
    abi_version: MODULE_ABI_VERSION,
    klines: BTreeMap::from([(60, 200), (300, 200)]),
    internal_indicators: BTreeMap::from([(
        InternalIndicatorKind::Ntps,
        BTreeMap::from([(60, 10), (300, 10)]),
    )]),
}
```

Kline keys and internal-indicator timeframe keys are seconds; their values
are requested history counts. Internal indicators use the fixed platform list
and typed schemas. Dynamic, parameterized indicators such as MRC are a separate
future feature and are not requested or emitted by the host today.

## Internal Indicators

`ModuleEvent::InternalIndicators { timeframes }` carries only new or revised
records since the previous update for the listed timeframes. `input.internal_indicators`
is keyed by timeframe and each record has a timestamp and a typed indicator
value. `InternalIndicator::Ntps` currently provides `value`, `min`, and `max`.
On `init`, the same field contains the requested recent history. Keep any
additional rolling calculations in the module; the host bounds the delivered
history to the requested count.

---

## ModuleEvent

Discriminated union (serde `snake_case` tag). Tells the module what triggered the current tick.

| Variant | Fields | Description |
|---------|--------|-------------|
| `requirements` | *(none)* | Return `ModuleDataRequirements` for the data this module needs |
| `init` | *(none)* | Initial event with requested kline and internal-indicator history |
| `kline_update` | `timeframes: Vec<i32>` | New or revised klines for requested timeframes |
| `market_decision` | *(none)* | A public market snapshot is available in `input.market_decision` |
| `internal_indicators` | `timeframes: Vec<i64>` | Incremental fixed platform indicator records in `input.internal_indicators` |
| `indicators` | `timeframes: Vec<i64>` | Reserved for future dynamic indicators; the host does not emit it |
| `signal` | `direction`, `code`, `values` | Signal action event |
| `new_position` | `direction: Direction, entry_price: f64, qty: f64` | A position was opened on the exchange |
| `finish_position` | `direction: Direction, pnl: f64` | A position was closed |
| `order_update` | `direction: Direction, order_side: ModuleExchangeOrderSide, role: OrderRole, status: ModuleOrderStatus, fill_price: f64, filled_qty: f64` | An order's status changed on the exchange |
| `action` | `name: String, values: HashMap<String, Value>` | User-triggered named action from the UI. `values` carries any parameters; empty for parameter-less actions |

---

## MarketDecision

The host sends a `market_decision` event when a new market snapshot is ready.
Read it from `input.market_decision`; it is `None` for other event types.

| Field | Type | Description |
|-------|------|-------------|
| `time` | `i64` | Market event timestamp in milliseconds |
| `system_time` | `i64` | Host timestamp in milliseconds |
| `symbol_key` | `String` | Exact platform symbol key |
| `price` | `f64` | Latest trade price |
| `candle` | `ModuleMarketCandle` | Current trade aggregation |
| `fc` | `Option<ModuleMarketCandle>` | Optional fast aggregation |
| `ntp_5s` | `i64` | Trade count in the recent five-second window |
| `tdt` | `Option<i64>` | Optional trade-delta time value |
| `order_book` | `Option<ModuleMarketOrderBook>` | Optional order-book snapshot |
| `oi` | `Option<ModuleMarketOpenInterest>` | Optional open-interest snapshot |

`ModuleMarketCandle` contains `min`, `max`, `enter`, `exit`, `qty`, `asset`,
`qtym`, and `masset`. Order-book sides use `asks` and `bids`; each level has
`price`, `quantity`, `total_amount`, `filled`, `time_start`, and `duration`.

---

## OrderRole

```rust
pub enum OrderRole { Entry, TakeProfit, StopLoss, Other }
```

JSON: `"entry"`, `"take_profit"`, `"stop_loss"`, `"other"` (serde `snake_case`).

---

## ModuleSymbolInfo

Trading rules for the current symbol, populated by the host.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `price_tick_size` | `f64` | `0.0` | Minimum price increment (e.g. `0.00001`) |
| `qty_step_size` | `f64` | `0.0` | Minimum quantity increment per order |
| `min_notional` | `Option<f64>` | `None` | Minimum order value in quote currency (USD) |
| `min_order_qty` | `Option<f64>` | `None` | Minimum order quantity in contracts/coins |

---

## ModulePositions

| Field | Type | Description |
|-------|------|-------------|
| `long` | `Option<ModulePositionSummary>` | Current long position, if any |
| `short` | `Option<ModulePositionSummary>` | Current short position, if any |

### ModulePositionSummary

| Field | Type | Description |
|-------|------|-------------|
| `entry_price` | `f64` | Average entry price |
| `notional` | `f64` | Position notional value in USD |
| `pnl` | `f64` | Unrealised PnL |
| `qty` | `f64` | Position quantity |

---

## ModuleOutput

Written to stdout as a single JSON line. The bridge reads this to execute trading actions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `open_positions` | `Vec<ModuleOpenPosition>` | `[]` | Positions to open |
| `close_positions` | `Vec<ModuleClosePosition>` | `[]` | Positions to close |
| `update_positions` | `Vec<ModuleUpdatePosition>` | `[]` | Amend fields of existing positions (no cancel+replace) |
| `place_orders` | `Vec<ModulePlaceOrder>` | `[]` | Standing limit orders to create or update (matched by `mark`) |
| `cancel_orders` | `Vec<String>` | `[]` | Marks of standing limit orders to cancel |
| `alerts` | `Vec<ModuleAlert>` | `[]` | Alert notifications sent to the user (push/Telegram). See **Alerts** below |
| `stop_bot` | `bool` | `false` | Set `true` to stop the bot after this tick |
| `state` | `Option<serde_json::Value>` | `None` | Opaque state — **persisted to bot context** in DB (every ~80s) and restored on restart via `ModuleInput.state`. Also visible to the user in the UI |
| `debug` | `String` | `""` | Debug message — logged at INFO level and saved in bot context (visible to user for tracing) |
| `error` | `String` | `""` | Error message — logged at ERROR level by the bridge. `run_loop` populates this automatically for parse errors and handler panics |

---

## ModuleOpenPosition

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `Direction` | — | `Long` or `Short` |
| `amount` | `Option<f64>` | `None` | Position size in USD. Provide either `amount` or `qty` |
| `qty` | `Option<f64>` | `None` | Position size in contracts/coins. Provide either `amount` or `qty` |
| `enter_price` | `Option<f64>` | `None` | Limit entry price. `None` = market order |
| `order_type` | `ModuleOrderType` | `Market` | Order type (`Market`, `Limit`) |
| `take_profit` | `Option<f64>` | `None` | TP price |
| `stop_loss` | `Option<f64>` | `None` | SL price |
| `note` | `String` | — | Human-readable note (logged by the bridge) |

If both `amount` and `qty` are zero/None, the entry is treated as a TP/SL-only
update on an existing position (no new position opened).

---

## ModuleClosePosition

| Field | Type | Description |
|-------|------|-------------|
| `direction` | `Direction` | Which position to close (`Long` or `Short`) |
| `reason` | `String` | Human-readable close reason (logged) |

---

## ModuleUpdatePosition

Amend fields of an already-open or pending-limit position. All fields except
`direction` are optional — only non-`None` fields are applied. If the position
does not yet exist, the update is silently dropped.

| Field | Type | Description |
|-------|------|-------------|
| `direction` | `Direction` | Which position to amend |
| `enter_price` | `Option<f64>` | New limit entry price (triggers exchange order amendment) |
| `take_profit` | `Option<f64>` | New TP price |
| `stop_loss` | `Option<f64>` | New SL price |

---

## ModulePlaceOrder

Standing limit order placed by the module. `mark` is the stable unique identifier —
the platform creates the order on first appearance and updates price/amounts on
subsequent ticks.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `Direction` | — | `Long` or `Short` — which side's position this order belongs to |
| `amount` | `Option<f64>` | `None` | Order size in USD. Provide either `amount` or `qty` |
| `qty` | `Option<f64>` | `None` | Order size in contracts/coins. Provide either `amount` or `qty` |
| `enter_price` | `f64` | — | Limit price for the order |
| `take_profit` | `Option<f64>` | `None` | TP attached to this order |
| `stop_loss` | `Option<f64>` | `None` | SL attached to this order |
| `note` | `Option<String>` | `None` | Human-readable note logged by the bridge; omit it to leave the host note empty |
| `mark` | `String` | — | Stable unique identifier — used to upsert/cancel by the host |
| `order_side` | `ModuleOrderSide` | `Buy` | Exchange order side: `Buy` for entries/DCA, `Sell` to reduce a Long, `Buy` to reduce a Short |
| `reduce_only` | `bool` | `false` | When `true`, only applied to an already-open position. Never buffered — silently dropped if no position is open. Use for all partial-close orders |

### ModuleOrderSide

```rust
pub enum ModuleOrderSide {
    Buy,  // default — entry / DCA orders
    Sell, // reduce a Long position (or entry when direction = Short)
}
```

### Partial-close (reduce-only) example

```rust
use lte_strategy_bridge::abi::{ModulePlaceOrder, ModuleOrderSide, Direction};

ModulePlaceOrder {
    direction: Direction::Long,
    amount: Some(125.0),         // close $125 worth of the position
    qty: None,
    enter_price: target_price,
    take_profit: None,
    stop_loss: None,
    note: Some("tp-partial-1".to_string()),
    mark: "tp-partial-1".to_string(),
    order_side: ModuleOrderSide::Sell,
    reduce_only: true,           // REQUIRED for partial-close orders
}
```

**Rules for `reduce_only`:**
- Always set `reduce_only: true` on partial-close / partial-TP orders.
- Never set it on entry or DCA orders (`reduce_only: false` or omit).
- The host will NOT buffer a `reduce_only` order if no matching position
  is currently open — it logs a warning and skips it.

---

---

## Dynamic indicators

`ModuleEvent::Indicators` and `ModuleInput.indicators` are reserved for a future
feature that lets modules request parameterized indicators. The host does not
emit this event or provide this data today. For private TA calculations,
request klines and calculate indicators inside the module. For the fixed
platform NTPS indicator, request `InternalIndicatorKind::Ntps` and consume
`InternalIndicators` updates as described above.

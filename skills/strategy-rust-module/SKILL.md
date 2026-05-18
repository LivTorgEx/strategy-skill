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
3. Package source as a `.tar.gz`.
4. **Get a presigned upload URL** and **upload the archive to S3**.
5. **Submit a build** — the server validates the upload and queues compilation.
6. **Poll for build status** — on failure, read `build_output` compiler errors, fix, and re-submit.
7. On success, deploy the module version to a bot group.

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

See the **ABI Reference** section below for full type documentation.

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
curl -X PUT -T /tmp/module-src.tar.gz "$UPLOAD_URL"
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

---

### Step 7 — Deploy to a bot group

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
2. Re-package, re-upload, and re-submit (Steps 3–5) with a **new version string** (e.g. `0.2.0`).
3. Poll until `Success`.
4. Update the bot group's `module_version` via `upsert_bot_group`.

---

## skill_access

Each module record has `skill_access`: `"Edit"` (can upload), `"Read"` (list only),
`"Deny"` (blocked). HTTP 403 → user must set it to `"Edit"` in
**Account → Skill → Module Access**.

---

## Notes

- The bridge loads the WASM binary from S3/MinIO once at bot startup. To pick up
  a new binary, restart the affected bots (stop + start via the UI or worker).
- Use a stable `mark` string per logical entry (e.g. `"long-dca-t2"`) so the bridge
  can upsert/cancel orders idempotently.
- Keep `module_version` pinned in production; use `"latest"` only during development.
- Build timeout: 240 seconds. If the build times out, status will be `Failed`
  with `build_output` = `"build timed out"`.
- `build_output` is capped at 32 KB.
- `create_module_repository`, `get_module_upload_url`, and `submit_module_build` are write-scoped tools.
  The server admin must include them in `MCP_ALLOWED_WRITE_TOOLS` for them to
  appear as enabled in the tool list.

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
| `symbol` | `String` | Trading pair (e.g. `"BTC-USDT-SWAP"`) |
| `max_amount` | `f64` | Base max position size in USD (from bot group settings) |
| `auto_max_amount` | `f64` | Effective max after compounding: `max_amount + realised_pnl * auto_max_amount_leverage`. Falls back to `max_amount` when no PnL accumulated |
| `leverage` | `i32` | Exchange leverage |
| `symbol_info` | `ModuleSymbolInfo` | Trading rules for the symbol (tick size, lot size, minimums) |
| `indicators` | `BTreeMap<i64, Vec<HashMap<String, HashMap<String, ModuleIndicatorValue>>>>` | Indicator data keyed by timeframe in seconds. See **Indicator Access** below |
| `positions` | `ModulePositions` | Current open positions summary |
| `sug_info` | `Option<SuggestionInfo>` | Real-time projection/suggestion data (present on `SugInfo` events) |
| `state` | `Option<serde_json::Value>` | Opaque state from previous tick's `ModuleOutput.state` |

---

## ModuleEvent

Discriminated union (serde `snake_case` tag). Tells the module what triggered the current tick.

| Variant | Fields | Description |
|---------|--------|-------------|
| `sug_info` | *(none)* | Real-time projection data arrived. `input.sug_info` is populated |
| `indicators` | `timeframes: Vec<i64>` | Indicator candles updated for the listed timeframe(s) |
| `signal` | *(none)* | Signal event |
| `new_position` | `direction: Direction, entry_price: f64, qty: f64` | A position was opened on the exchange |
| `finish_position` | `direction: Direction, pnl: f64` | A position was closed |
| `order_update` | `direction: Direction, order_side: String, role: OrderRole, status: String, fill_price: f64, filled_qty: f64` | An order's status changed on the exchange |
| `action` | `name: String, values: HashMap<String, Value>` | User-triggered named action from the UI. `values` carries any parameters; empty for parameter-less actions |

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
| `stop_bot` | `bool` | `false` | Set `true` to stop the bot after this tick |
| `state` | `Option<serde_json::Value>` | `None` | Opaque state passed back on the next tick's `ModuleInput.state` |
| `debug` | `String` | `""` | Debug message (logged by the bridge, visible in bot logs) |

---

## ModuleOpenPosition

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `Direction` | — | `Long` or `Short` |
| `amount` | `Option<f64>` | `None` | Position size in USD. Provide either `amount` or `qty` |
| `qty` | `Option<f64>` | `None` | Position size in contracts/coins. Provide either `amount` or `qty` |
| `enter_price` | `Option<f64>` | `None` | Limit entry price. `None` = market order |
| `order_type` | `String` | `"Market"` | Order type (`"Market"`, `"Limit"`, `"StopMarket"`) |
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

## SuggestionInfo (projection data)

Present in `ModuleInput.sug_info` when `event` is `sug_info`.

Executed on around 1 second intervals, this is the main data feed for real-time projections and trading suggestions. The host populates it with the latest market data, indicator values,
and projection/suggestion information from the projection engine. Use it to implement real-time reactive strategies that respond to market conditions. The `status` field indicates whether the projection engine suggests normal trading conditions or a fast-trade opportunity when number of trades per 3 second (ntps) exceeds a certain threshold.

| Field | Type | Description |
|-------|------|-------------|
| `time` | `i64` | Timestamp (ms) |
| `system_time` | `i64` | Server system time (ms) |
| `symbol_key` | `String` | Symbol key (e.g. `"OKX#BTC-USDT-SWAP#SWAP"`) |
| `status` | `AlgoSuggestionTradeStatus` | `"Normal"` or `"FastTrade"` |
| `price` | `f64` | Current price |
| `candle` | `SuggestionInfoCandle` | Current candle data for the sug info timeframe |
| `klines` | `HashMap<i64, SuggestionKline>` | Kline data keyed by timeframe (seconds) |
| `fast_candle` | `Option<SuggestionInfoCandle>` | Fast candle (aggregation while in fast-trade mode) |
| `direction` | `Direction` | Current market direction |
| `indicator` | `SuggestionInfoIndicator` | Projection-level indicator data |
| `order_book` | `Option<SuggestionInfoOrderBook>` | Order book snapshot |
| `oi` | `Option<SuggestionInfoOpenInterest>` | Open interest data |

### SuggestionInfoCandle

| Field | Type | Description |
|-------|------|-------------|
| `min` | `f64` | Candle low |
| `max` | `f64` | Candle high |
| `enter` | `f64` | Candle open |
| `exit` | `f64` | Candle close |
| `qty` | `f64` | Total trade quantity |
| `qty_asset` | `f64` | Total trade quantity in asset |
| `qtym` | `f64` | Market-order quantity |
| `qtym_asset` | `f64` | Market-order quantity in asset |

### SuggestionKline

| Field | Type | Description |
|-------|------|-------------|
| `low` | `f64` | Kline low |
| `high` | `f64` | Kline high |
| `open` | `f64` | Kline open |
| `close` | `f64` | Kline close |

### SuggestionInfoOrderBook

| Field | Type | Description |
|-------|------|-------------|
| `long_levels` | `Vec<SuggestionInfoOrderBookLevel>` | Bid side levels |
| `short_levels` | `Vec<SuggestionInfoOrderBookLevel>` | Ask side levels |
| `sell_amount` | `f64` | Total sell-side amount |
| `sell_price` | `Option<f64>` | Best ask price |
| `buy_amount` | `f64` | Total buy-side amount |
| `buy_price` | `Option<f64>` | Best bid price |

### SuggestionInfoOrderBookLevel

| Field | Type | Description |
|-------|------|-------------|
| `price` | `f64` | Price level |
| `quantity` | `f64` | Quantity at this level |
| `total_amount` | `f64` | Total notional amount to that level |
| `filled` | `Option<f64>` | Fill ratio (0.0–1.0) |
| `time_start` | `i64` | When this level appeared (ms) |
| `duration` | `i64` | How long this level has existed (ms) |

### SuggestionInfoOpenInterest

| Field | Type | Description |
|-------|------|-------------|
| `usd` | `f64` | Open interest in USD |
| `change` | `f64` | OI change (percentage) |
| `change_qty` | `f64` | OI change (absolute) |
| `avg` | `f64` | Average OI |

### SuggestionInfoIndicator

| Field | Type | Description |
|-------|------|-------------|
| `ntps` | `i64` | Average number of trades per 3 seconds |
| `ntps_fast_time` | `Option<i64>` | Fast-trade mode trigger time |
| `trandm` | `f64` | Buy/Sell performance during the last 9 days that can be from -100% to +100%. Oversold/Overbought conditions are indicated by extreme values. |
| `asset_01` | `f64` | Asset value per 1% of TrandM |
| `price_1h` | `f64` | Price change % over 1h |
| `price_4h` | `f64` | Price change % over 4h |
| `price_8h` | `f64` | Price change % over 8h |
| `price_24h` | `f64` | Price change % over 24h |

### AlgoSuggestionTradeStatus

```rust
pub enum AlgoSuggestionTradeStatus { Normal, FastTrade }
```

---

## Indicator Access

Indicators are delivered in `ModuleInput.indicators` as a nested map:
`BTreeMap<timeframe_seconds, Vec<candle_map>>` where each `candle_map` is
`HashMap<indicator_name, HashMap<field_name, ModuleIndicatorValue>>`.

The first element (`[0]`) is always the most recent candle.

### ModuleIndicatorValue

```rust
pub enum ModuleIndicatorValue {
    Float(f64),
    String(String),
    Direction(Direction),
    Cross(ModuleIndicatorCross),
}
```

### ModuleIndicatorCross

```rust
pub enum ModuleIndicatorCross { Cross, Body, Shadow, No, Above, Below }
```

### Available timeframes

| Constant | Seconds |
|----------|---------|
| `Tf1m` | `60` |
| `Tf5m` | `300` |
| `Tf15m` | `900` |
| `Tf30m` | `1800` |
| `Tf1h` | `3600` |
| `Tf4h` | `14400` |

### Available indicators and their properties

**Do NOT hardcode indicator names or properties.** Each indicator has its own
specific set of properties. Call the MCP tool `list_indicators` to get the
authoritative list of indicator names, valid periods, and their exact property
names. The map key in `ModuleInput.indicators` is `lowercase_name` + `_` + `period`
(e.g. `rsi_14`, `ema_200`, `bb_20`, `stoch_14,1,3`). Indicators without a
period use just the lowercase name (e.g. `psar`, `supertrend`, `candle`).

Example `list_indicators` response (truncated):
```json
[
  {"name":"Rsi","period":["14"],"properties":["Value"]},
  {"name":"Ema","period":["9","20","21","26","200"],"properties":["Direction","Value"]},
  {"name":"BollingerBands","period":["20"],"properties":["Average","Upper","Lower"]},
  {"name":"Candle","properties":["Direction","BodyChange","Open","Close","High","Low"]}
]
```

The `properties` array lists the exact field names you can use as keys in the
inner `HashMap<String, ModuleIndicatorValue>`. Using a property name not in
that list will return `None`.

### Helper: `get_value()`

```rust
use lte_strategy_bridge::indicator_access::{
    get_value, IndicatorFieldKey, IndicatorKey, IndicatorField, TimeframeSec,
};

let key = IndicatorFieldKey {
    timeframe: TimeframeSec::Tf1h,
    indicator: IndicatorKey::Rsi14,
    field: IndicatorField::Value,
};

if let Some(ModuleIndicatorValue::Float(rsi)) = get_value(&input.indicators, key) {
    // use rsi value
}
```

### Manual access pattern

```rust
if let Some(candles) = input.indicators.get(&3600) {  // 1h timeframe
    if let Some(first_candle) = candles.first() {
        if let Some(fields) = first_candle.get("rsi_14") {
            if let Some(ModuleIndicatorValue::Float(rsi)) = fields.get("Value") {
                // use rsi value
            }
        }
    }
}
```

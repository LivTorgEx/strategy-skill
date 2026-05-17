---
name: create-strategy
description: Design a LivTorgEx trading strategy and deploy it as a bot group. Use when the user asks to create, update, or deploy a trading strategy or bot group.
---

# LivTorgEx — Create / Update Bot Group Strategy

## Workflow

### Step 1 — Fetch account context

Call MCP tool: `list_api_keys`
Call MCP tool: `list_bot_groups`

Present API keys by name. Warn if target bot group has `skill_access: "Read"` or `"Deny"`.

### Step 2 — Clarify

Ask for: bot group name, API key (from list), indicators + timeframe, direction, TP/SL %, max_open_amount, leverage, margin, symbol IDs, max active bots.

### Step 3 — Build JSON

Construct a valid `BotGroupSetting`. For indicator details use `strategy-indicators`. For condition syntax use `strategy-conditions`. For event handler patterns use `strategy-on-actions`.

### Step 4 — Validate

Call MCP tool: `validate_bot_group`
Arguments: `{ "settings": <FULL_SETTINGS_JSON> }`

Response: `{ "valid": true|false, "errors": [...], "warnings": [...] }`.

Fix all `errors`. `warnings` are non-blocking.

### Step 5 — Deploy

`upsert_bot_group` has **two modes**. Pick the right one based on whether the bot group already exists.

#### 5a. CREATE-OR-UPDATE-BY-NAME (full payload)

Use when creating a brand-new bot group, or when you want to fully replace an existing one's config. Send every required field; if a bot group with this `name` already exists for the caller, it is fully overwritten.

Call MCP tool: `upsert_bot_group`
Arguments:
```json
{
  "name": "<BOT_GROUP_NAME>",
  "api_key_id": <ID from list_api_keys>,
  "margin_mode": "Isolated",
  "margin_leverage": 20,
  "margin": 500.0,
  "max_active_bots": 5,
  "symbols": [{ "symbol_key": "OKX#BTC-USDT-SWAP", "leverage": 20 }],
  "settings": { <FULL_SETTINGS_JSON> }
}
```

`"created": true` = new group. `"created": false` = updated.

#### 5b. PARTIAL UPDATE BY ID (GraphQL-style)

Use when changing **only some fields** of an existing bot group. Pass `bot_group_id` and **only** the fields you want to modify. Every field you omit is preserved — including `symbols`, `name`, `api_key_id`, margins, etc.

> **Safety:** updating `settings` alone via this mode will NOT touch the symbols list, margin, or any other field. This is the recommended way to tweak an existing strategy without risking accidental overwrites.

**Rename only:**
```json
{ "bot_group_id": 19, "name": "MRC v2" }
```

**Change leverage only:**
```json
{ "bot_group_id": 19, "margin_leverage": 10 }
```

**Replace the whole `settings` JSON, keep everything else:**
```json
{ "bot_group_id": 19, "settings": { <FULL_SETTINGS_JSON> } }
```

**Patch sub-trees of `settings` without sending the full object** — use `settings_patches: [{path, value}, ...]`. Each entry REPLACES the sub-tree at `path` (dot notation; empty string = whole settings). Patches are applied over the bot group's existing settings and the merged result is re-validated automatically:
```json
{
  "bot_group_id": 19,
  "settings_patches": [
    { "path": "strategy.professional.take_profit.order.price.price.value", "value":  5.0 },
    { "path": "strategy.professional.stop_loss.order.price.price.value",  "value": -2.0 }
  ]
}
```

**Swap a whole sub-tree of `settings` in one shot:**
```json
{
  "bot_group_id": 19,
  "settings_patches": [
    { "path": "strategy.professional", "value": { <NEW_PROFESSIONAL_BLOCK> } }
  ]
}
```

**Notes for partial UPDATE mode:**
- `bot_group_id` must come from `list_bot_groups` or a prior `upsert_bot_group` response.
- Validation still runs whenever `settings` or `settings_patches` is supplied — invalid merged settings are rejected with the standard `Bot schema wrong: <reason> at <path>` error.
- Margin is auto-derived from `max_open_amount / margin_leverage` when settings change, unless you pass an explicit `margin`.
- Works only on **personal** bot groups (not strategy- or worker-attached). Returns 404 otherwise.
- `skill_access` must be `"Edit"` (same rule as before).

### Step 6 — Run backtest (optional feedback)

After deploying, offer the user a backtest to validate the strategy on historical data.
Ask for: `start_time` (Unix ms, e.g. 30 days ago), `end_time` (Unix ms, or omit for open-ended).

Call MCP tool: `run_bot_group_backtest`
Arguments:
```json
{
  "bot_group_id": <ID from upsert_bot_group response>,
  "start_time": 1740787200000,
  "end_time": 1748563200000
}
```

The response contains `version_id`. The user can review the backtest in the UI.

- Uses the bot group's **current** settings, symbols, margin, and leverage automatically.
- A linked backtest container is created once per bot group and reused on subsequent calls.
- Requires `skill_access` = "Edit".

### Step 7 — Run a signal action on a live bot group (optional)

Signal actions let you manually trigger strategy logic on a running bot group (e.g. force-enter a position, set a grid range, change direction).

**First, discover available actions:** `list_bot_groups` returns `signal_actions` on each group. Each entry has:
```json
{
  "name": "Human readable name",
  "code": "action_code",
  "description": "optional description",
  "params": [
    { "name": "Grid lower", "code": "grid_lower", "variant": "Number" },
    { "name": "Grid upper", "code": "grid_upper", "variant": "Number" },
    { "name": "Side",       "code": "side",        "variant": "Direction" },
    { "name": "Type",       "code": "order_type",  "variant": "Select", "options": [{"label":"Market","value":0},{"label":"Limit","value":1}] }
  ]
}
```

**Param variant → value format:**

| Variant | Value format | Example |
|---------|-------------|---------|
| `Number` | `f64` | `45000.0` |
| `Direction` | `"buy"` or `"sell"` | `"buy"` |
| `OrderType` | `"Market"` or `"Limit"` | `"Market"` |
| `PriceRange` | Two keys: `{code}_lower` and `{code}_upper` (both `f64`) | `"range_lower": 44000.0, "range_upper": 46000.0` |
| `Select` | Numeric index of the selected option (`f64`) | `0.0` (first option) |

Call MCP tool: `run_bot_group_action`
Arguments:
```json
{
  "bot_group_id": <ID>,
  "code": "action_code",
  "symbol_key": "OKX#BTC-USDT-SWAP",
  "price": 45000.0,
  "direction": "buy",
  "leverage": null,
  "margin": null,
  "values": {
    "grid_lower": 44000.0,
    "grid_upper": 46000.0
  }
}
```

- `price` is **required** — pass the current market price for the symbol.
- `direction` defaults to `"buy"` if omitted.
- `values` may be empty `{}` if the action has no params.
- Bot group must have `skill_access` = `"Edit"` and must not be linked to a strategy NFT worker.

---

## skill_access

Each bot group has `skill_access`: `"Edit"` (can deploy), `"Read"` (list only), `"Deny"` (blocked). Error 403 → user must set it to `"Edit"` in **Account → Skill → Bot Group Access**.

---

## STRICT TIMEFRAMES

Only these 6 values for `timeframe` in indicators (seconds):

| 1m | 5m | 15m | 30m | 1h | 4h |
|----|----|----|-----|----|-----|
| `60` | `300` | `900` | `1800` | `3600` | `14400` |

No daily. `min_tf` in signal definitions is in **minutes**.

---

## Type rules

| Field | Type | Example |
|-------|------|---------|
| `timeframe` | int seconds (strict list) | `3600` |
| `period` | **string** | `"14"`, `"10,3,3"` |
| `modify` | int (0=Auto, omit for default) | `0` |
| TP/SL percentage | float (+ = profit, − = loss) | `3.0`, `-1.5` |
| `min_tf` in signal | int **seconds** | `60` = 1m |
| `Direction` values | uppercase string | `"LONG"`, `"SHORT"`, `"BOTH"` |

---

## BotGroupSetting structure

```json
{
  "margin_mode": "Isolated" | "Cross",
  "margin_leverage": 20,
  "signal": { "name": "Indicator", "min_tf": 60 },
  "strategy": {
    "type": "Trading",
    "name": "Trading",
    "max_open_amount": 500.0,
    "direction": "Long" | "Short" | "Both",
    /* Long/Short — single direction only. Both — bot can use LONG and SHORT (hedge or reversal). */
    "professional": {
      "type": "Professional",
      "variables": [],
      "enter_price":     { "type": "Force" },
      "enter_amount":    { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
      "enter_direction": { "type": "Default" },
      /* Optional auto-invest / PnL compounding. Omit for no compounding (default).
         Set "auto_max_amount_leverage": { "type": "Number", "value": 1.0 } to reinvest
         100% of closed-position PnL into the next position's budget. See
         `strategy-initialization` skill for the full formula and value types. */
      "auto_max_amount_leverage": { "type": "Number", "value": 1.0 },
      "filters": [ /* search-mode gate — checked before starting a new bot; AND by default */ ],
      "take_profit": { "type": "NextOrder", "order": {
        "type":   "Market",
        "price":  { "type": "Price", "price": { "type": "Percentage", "value": 3.0 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }},
      "stop_loss": { "type": "NextOrder", "order": {
        "type":   "Market",
        "price":  { "type": "Price", "price": { "type": "Percentage", "value": -1.5 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }},
      "on_analysis":   [ /* { "type": "Action", "filters": [], "action": {} } — evaluated every ~1s */ ],
      "on_indicators": [ /* { "timeframe": int, "filters": [], "actions": [{ "type": "Action", ... }] } */ ],
      "on_actions":    [ /* manual user triggers only — omit for auto trading */ ]
    }
  }
}
```

### `professional.filters` — bot spawn gate

**Checked exactly once when the bot group decides to spawn a new bot** (on each `min_tf` tick). If all filters pass → a new bot is spawned. Filters are NOT re-evaluated after the bot is running. Use for quality gates: NTPS, NATR, MRC direction, distance checks. All items are AND by default.

> **`enter_price` controls what happens immediately after spawn:**
> - `Force` → the spawned bot opens a position at market **immediately** (no `on_analysis`/`on_indicators` needed for entry)
> - `Wait` → the spawned bot waits; **you must call `ForceStartPosition` in `on_analysis` or `on_indicators`** to enter
>
> **Important:** `professional.filters` do NOT prevent re-spawning if conditions stay true for multiple `min_tf` ticks. Use `PrevCross`/`CurrentCross` guards or similar one-tick conditions in filters to ensure a single spawn per event.

**`on_analysis`** is the ~1s strategy loop — put entry conditions (when using `Wait`) and exit logic here as `[{ "type": "Action", filters, action }]`.
**`on_indicators`** fires on candle close — `[{ timeframe, filters, actions: [{ "type": "Action", ... }] }]`.
**`on_finished`** fires when a position closes — same item shape as `on_analysis`: `[{ "type": "Action", filters, action }]`.
**`on_actions`** is for manual UI triggers only — omit unless explicitly required.
`take_profit`/`stop_loss` orders require `"type": "Market"` on the inner `order` object.

### `auto_max_amount_leverage` — Auto Invest / PnL compounding

`auto_max_amount_leverage` (UI label: **Auto Invest** / "Auto Max Amount Leverage") is an optional Pro setting that auto-reinvests realised PnL into the next position's budget.

```text
AutoMaxAmount = AutoMaxAmount_prev + position_pnl × auto_max_amount_leverage
```

**Only values ≥ 1 are supported.** Zero and negative values are not valid — to disable compounding, simply omit the field.

| Value | Effect |
|-------|--------|
| omitted | No compounding — every bot uses `max_open_amount` as-is |
| `{ "type": "Number", "value": 1.0 }` | Full PnL reinvested (standard, also the in-UI default when the field is enabled) |
| `{ "type": "Number", "value": 2.0 }` | Aggressive — 2× PnL reinvested |

When the user asks for "auto invest", "compounding", "reinvest profit", or "scaling budget with PnL" → set this field. To size the bot's next entry from the compounded budget, reference `{ "type": "Global", "value": "AutoMaxAmount" }` in `enter_amount` or in a variable's default. Full reference: `strategy-initialization` skill.

### `enter_price` types

| Type | Behaviour after bot spawn |
|------|--------------------------|
| `Force` | Opens position at market price immediately on spawn — no action needed |
| `Wait` | Spawns bot in waiting state — call `ForceStartPosition` from `on_analysis` or `on_indicators` |
| `Signal` | Enters at the signal's suggested price |
| `Indicator` | Enters at a computed indicator price |

### `enter_direction` types

| Type | Description |
|------|-------------|
| `Default` | Use strategy `direction` |
| `Signal` | Follow signal direction |
| `Indicator` | Read from indicator |
| `Value` | Hardcoded direction |

---

## What is a bot?

A **bot** is an instance of real-time market monitoring created by the bot group. It is NOT just a position — a bot can:

- **Monitor the market** to update its own variables or run actions (even with no open position)
- **Start a position** via `ForceStartPosition` (from `on_analysis`, `on_indicators`, or automatic via `enter_price: Force`)
- **Stop a position** via `ForceStopPosition`
- **Wait** — the bot spawns in waiting mode, monitoring conditions until a `ForceStartPosition` action triggers entry. A waiting bot can be stopped (e.g. `ForceStopBot` if conditions no longer apply).

A bot's life: **spawned → (optional waiting) → position open → position closed → finished**. `on_finished` fires on position close; the bot then exits.

### `max_active_bots` and margin

`max_active_bots` caps how many bots can be alive simultaneously. This directly limits capital exposure:

> Example: `margin = 5.0`, `max_active_bots = 3` → maximum simultaneous exposure = $15.

A bot can open positions larger than its initial margin if it has accumulated **live profit** during its lifetime — unrealised gains can be used as extra margin while the bot is running.

---

## signal_actions and on_actions — manual triggers

There are two distinct layers of manual triggers, and both can be used together.

### `signal_actions` — bot-group-level spawn trigger

`signal_actions` are top-level buttons on the bot group. Each one creates (spawns) a **new bot** on the specified symbol when triggered. They are defined on the bot group directly (not inside `settings`):

```json
"signal_actions": [
  { "code": "start", "name": "Start", "params": [] }
]
```

Pass this in `upsert_bot_group` alongside `settings`. For the `signal` field to honour this, set `signal` to `Action` with the matching code:

```json
"signal": { "name": "Action", "code": "start" }
```

**Empty symbols:** When using signal actions you can leave `symbols: []` on the bot group. The token is provided by the caller at trigger time via `symbol_key` in `run_bot_group_action` — no need to pre-configure symbols.

Trigger via `run_bot_group_action`:
```json
{ "bot_group_id": 20, "code": "start", "symbol_key": "OKX#BTC-USDT-SWAP", "price": 45000.0 }
```

### `on_actions` — per-bot in-position trigger

`on_actions` are buttons on an **already-running bot**. They send an event into the strategy logic of that specific bot (e.g. force-enter a position, adjust TP, close). They live inside `settings.strategy`:

**Pro strategy (`name: "Trading"`):**
```json
"strategy": {
  "name": "Trading",
  "professional": {
    "on_actions": [
      {
        "name": "ForceEnter",
        "params": [],
        "actions": [
          { "type": "Action", "filters": [], "action": { "type": "ForceStartPosition" } }
        ]
      }
    ]
  }
}
```
When triggered, the `actions` array runs exactly like `on_analysis` actions — same filter + action syntax.

**DynamicModule strategy (`name: "DynamicModule"`):**
```json
"strategy": {
  "name": "DynamicModule",
  "on_actions": [
    { "name": "ForceEnter", "params": [], "actions": [] }
  ]
}
```
The WASM module receives a `ModuleEvent::Action { name: "ForceEnter" }` and handles it internally — `actions` is unused and should be `[]`.

### Summary

| | `signal_actions` | `on_actions` |
|---|---|---|
| Level | Bot group | Individual bot |
| Effect | Spawns a new bot on the given symbol | Sends event to existing running bot |
| Config location | Top-level `signal_actions` field on bot group | `settings.strategy.professional.on_actions` (Pro) or `settings.strategy.on_actions` (DynamicModule) |
| Trigger tool | `run_bot_group_action` | UI per-bot action button (not via MCP) |
| Symbol | Provided at call time — `symbols: []` is fine | Bot already knows its symbol |

---

## Signal types

The `signal` field defines the **trigger source** that causes the bot group to check `professional.filters` and potentially spawn a new bot. `Indicator` (candle-close timer) is the most common, but other sources react to external market events.

| Name | Extra fields | Notes |
|------|-------------|-------|
| `Indicator` | `min_tf: int (seconds)` | **Default choice.** Rechecks filters on each candle close at `min_tf` interval. Use `min_tf: 60` for 1m. |
| `FastTrade` | — | — |
| `OverTrend` | `min_arrange`, `max_arrange`, `min_4h_wave_prc` | — |
| `ATH4H` | `min_4h_wave_prc`, `reason: "Always"\|"OnlyLine"\|"LineOrMovement"` | — |
| `OrderBigSize` | `min_volume_asset`, `min_duration_minutes` | — |
| `CrossLine` | `timeframe: int (seconds)`, `min_mins: int` | — |
| `QtymSpike` | — | — |
| `Action` | `code: string` | — |
| `External` | `signal_id: int` | External signal source; does not drive filter recheck by timer. |

---

## Full example (RSI + Supertrend Long, 1h)

```json
{
  "name": "RSI Supertrend Long",
  "api_key_id": 1,
  "margin_mode": "Isolated",
  "margin_leverage": 20,
  "margin": 100.0,
  "max_active_bots": 3,
  "symbol_ids": [1],
  "settings": {
    "margin_mode": "Isolated",
    "margin_leverage": 20,
    "signal": { "name": "Indicator", "min_tf": 60 },
    "strategy": {
      "type": "Trading",
      "name": "Trading",
      "max_open_amount": 500.0,
      "direction": "Long",
      "professional": {
        "type": "Professional",
        "variables": [],
        "enter_price":     { "type": "Force" },
        "enter_amount":    { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
        "enter_direction": { "type": "Default" },
        "filters": [],
        "take_profit": { "type": "NextOrder", "order": {
          "type":   "Market",
          "price":  { "type": "Price", "price": { "type": "Percentage", "value": 3.0 } },
          "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
        }},
        "stop_loss": { "type": "NextOrder", "order": {
          "type":   "Market",
          "price":  { "type": "Price", "price": { "type": "Percentage", "value": -1.5 } },
          "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
        }},
        "on_analysis": [
          {
            "type": "Action",
            "filters": [
              {
                "type": "Operation", "operation": "<",
                "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                           "indicator": { "type": "Rsi", "period": "14", "property": "Value" } },
                "right": { "type": "Number", "value": 30.0 }
              },
              {
                "type": "Operation", "operation": "==",
                "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                           "indicator": { "type": "Supertrend", "property": "Direction" } },
                "right": { "type": "Direction", "value": "LONG" }
              }
            ],
            "action": {
              "type": "ForceStartPosition",
              "side": { "type": "Direction", "value": "LONG" },
              "msg": "RSI oversold + Supertrend Long"
            }
          }
        ]
      }
    }
  }
}
```

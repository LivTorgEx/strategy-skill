---
name: strategy-on-actions
description: LivTorgEx event handler and action reference — on_analysis, on_indicators, on_created, on_finished, on_actions, variables, and the complete action type reference (ForceStartPosition, CreateOrder, FireAlert, Break, etc.).
---

# LivTorgEx — Event Handlers, Variables & Actions

---

## 1. Handler structure

`professional` has five event handler arrays and a `variables` array:

```json
"professional": {
  "variables":     [ { "type": "Variable", "key": "k", "name": "Label", "default": {} } ],
  "on_analysis":   [ { "type": "Action", "filters": [], "action": {} } ],
  "on_created":    [ { "type": "Action", "filters": [], "action": {} } ],
  "on_indicators": [ { "timeframe": 300, "filters": [], "actions": [ { "type": "Action", "filters": [], "action": {} } ] } ],
  "on_finished":   [ { "type": "Action", "filters": [], "action": {} } ],
  "on_actions":    [ { "name": "trigger_name", "params": [], "actions": [ { "type": "Action", "filters": [], "action": {} } ] } ]
}
```

**Every item in `on_analysis`, `on_created`, `on_finished`, and `on_indicators[].actions` must include `"type": "Action"`.**

---

## 2. Bot lifecycle

A **bot** is a real-time market monitor — not just a position. It can watch the market, open/close positions, track variables, and stop itself.

**Spawn flow:**
1. On each indicator candle close (controlled by `signal.min_tf`), the bot group evaluates `professional.filters`. If all pass → spawns a new bot (subject to `max_active_bots`).
2. `enter_price` determines what happens next:
   - `Force` — opens a position at market immediately. No action needed.
   - `Wait` — bot spawns without a position. You must call `ForceStartPosition` from `on_analysis` or `on_indicators` to open one.
3. `on_analysis` / `on_indicators` run continuously while the bot is alive — whether or not it has an open position.
4. When a position closes, `on_finished` fires once. If `on_finished` is empty → bot stops automatically. If `on_finished` has any actions → bot keeps running indefinitely without a position. You must use `ForceStopBot` to stop it.

---

## 3. Event handlers

### `on_analysis` — main strategy loop (~1s)

Runs every ~1 second with fresh price and indicator data. **Use by default** for entry/exit logic, trailing stops, PnL checks, variable updates.

Item shape: `{ "type": "Action", "filters": [...], "action": {...} }` — all filters must pass for the action to execute.

```json
"on_analysis": [
  { "type": "Action",
    "filters": [ /* conditions */ ],
    "action": { "type": "ForceStartPosition", "side": { "type": "Direction", "value": "LONG" } } }
]
```

### `on_indicators` — candle close events

Fires when a candle closes at the specified timeframe. Use for indicator-based triggers (EMA flip, Supertrend direction change, RSI threshold) rather than continuous price monitoring.

Item shape: `{ "timeframe": int, "filters": [...], "actions": [...] }` — note `actions` is an **array** (not singular).

```json
"on_indicators": [
  { "timeframe": 300, "filters": [], "actions": [
    { "type": "Action", "filters": [ /* conditions */ ],
      "action": { "type": "SetVariable", "name": "ema_flag", "value": { "type": "Number", "value": 1.0 } } }
  ]}
]
```

**Typical pattern:** use `on_indicators` to store indicator state in variables, then read those variables in `on_analysis` to make trading decisions.

### `on_created` — fires once when a position opens

Fires once immediately after the first entry order fills. Use for one-time setup: saving entry price to a variable, logging, initial order placement.

Item shape: same as `on_analysis`.

```json
"on_created": [
  { "type": "Action", "filters": [],
    "action": { "type": "SetVariable", "name": "entry_price", "value": { "type": "Position", "value": "EntryPrice" } } }
]
```

### `on_finished` — fires when a position closes

**Critical behavior:**
- `on_finished` is **empty** → bot stops automatically.
- `on_finished` has **any actions** → bot enters **waiting mode** and stays alive. You must add `ForceStopBot` explicitly to stop it.

Item shape: same as `on_analysis`.

```json
"on_finished": [
  { "type": "Action", "filters": [],
    "action": { "type": "ForceStopBot", "msg": "Position closed, stopping" } }
]
```

**Revert pattern** (bot stays alive, reverses direction, re-enters):
```json
"on_finished": [
  { "type": "Action", "filters": [],
    "action": { "type": "SetDirection", "value": { "type": "Position", "value": "DirectionOpposite" } } },
  { "type": "Action", "filters": [],
    "action": { "type": "ForceStartPosition", "side": { "type": "Global", "value": "Direction" }, "msg": "Revert" } }
]
```

**Conditional branches** — use `Actions` + `Break` to run only one branch:
```json
"on_finished": [
  { "type": "Action", "filters": [ /* condition A */ ],
    "action": { "type": "Actions", "actions": [
      { "filters": [], "action": { "type": "ForceStopBot", "msg": "SL hit" } },
      { "filters": [], "action": { "type": "Break", "level": 2 } }
    ]}},
  { "type": "Action", "filters": [],
    "action": { "type": "ForceStopBot", "msg": "Fallback — runs only if A didn't match" } }
]
```

### `on_actions` — manual UI triggers only

Fires on a manual user action from the UI. **Not for auto trading.** Omit in most strategies.

Item shape: `{ "name": "...", "params": [...], "actions": [...] }`

`params` defines UI input fields. Each param has `name`, `code`, and `variant` (`"Number"`, `"Direction"`, `"OrderType"`, `"PriceRange"`, or `"Select"`). Access param values in actions via `{ "type": "Parameter", "name": "param_code" }`.

```json
"on_actions": [
  { "name": "manual_close", "params": [], "actions": [
    { "type": "Action", "filters": [],
      "action": { "type": "ForceStopPosition", "side": null, "msg": "Manual close" } }
  ]}
]
```

---

## 4. `direction: "Both"` — LONG + SHORT in a single bot

Enables two patterns:
- **Hedge mode** — hold LONG and SHORT positions simultaneously
- **Reversal mode** — close one side and open the opposite

When using `Both`, specify `side` explicitly on every `ForceStartPosition` / `ForceStopPosition`. Use `Position` value with `side` to check each side independently.

Cross margin is preferred over isolated with `Both`.

---

## 5. Variables

`variables` holds named values that persist across ticks. Use them to share state between handlers, track multi-step conditions, or compute derived values.

```json
"variables": [
  { "type": "Variable", "name": "Display Label", "key": "var_key", "default": { "type": "Number", "value": 0.0 } }
]
```

| Field | Description |
|-------|-------------|
| `type` | **Must** be `"Variable"` |
| `name` | Display label (UI only) |
| `key` | Identifier used in `SetVariable`, `ClearVariable`, and `{ "type": "Variable", "name": "key" }` value references |
| `default` | Initial value — any value expression (Number, Global, Indicator, Math, PriceMeasure, etc.) |

**Evaluation order:** variables initialise sequentially at bot spawn time. Each variable's `default` can reference any **previously defined** variable (lower index). Forward references are not resolved.

**Refreshing variables:** defaults are only evaluated at spawn. Use `SetVariable` in `on_indicators` to update values on each candle close.

---

## 6. Action Reference

Every action below can be used in any handler (`on_analysis`, `on_created`, `on_indicators`, `on_finished`, `on_actions`).

---

### `ForceStartPosition` — open a position

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `side` | value → Direction | **yes** | Position side. Use `{"type":"Global","value":"Direction"}` to follow current bot direction |
| `amount` | value → Float | no | Override `enter_amount`. When omitted, uses `professional.enter_amount` |
| `enter_price` | value → Float | no | Override entry price. When omitted, enters at market |
| `order_type` | value → OrderType | no | Override order type (MARKET/LIMIT/STOP_MARKET/STOP_LIMIT) |
| `modifications` | array | no | Grid modifications for this position (see below) |
| `msg` | string | no | Log message |

**`modifications`:** `professional.modifications` only applies to the initial position opened via `enter_price: Force`. When using `enter_price: Wait`, the top-level modifications do **not** carry over — you must pass `modifications` on each `ForceStartPosition`. The array format is identical to `professional.modifications` (see `strategy-modifications` skill). You can reuse the same config or pass different grids per position.

```json
{ "type": "ForceStartPosition", "side": { "type": "Direction", "value": "LONG" }, "msg": "Open LONG" }
```

---

### `ForceStopPosition` — close a position

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `side` | value → Direction or `null` | no | Which side to close. `null` = close all |
| `msg` | string | no | Log message |

```json
{ "type": "ForceStopPosition", "side": null, "msg": "Close all" }
```

---

### `CreateOrder` — create or update an order

**Important:** if a pending order with the same `mark` and `pside` already exists, `CreateOrder` **updates** it instead of creating a new one. If no matching pending order exists, a new order is created. Filled orders are not tracked — if an order with a given mark has already filled and you call `CreateOrder` with the same mark, a new order will be created. Without a `mark`, every call creates a new order, which can lead to duplicate orders.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `amount` | value → Float | **yes** | Order amount (USDT) |
| `side` | value → Direction | **yes** | Order side |
| `price` | value → Float | no | Limit price. Omit for market orders |
| `order_type` | `"MARKET"` / `"LIMIT"` / `"STOP_MARKET"` / `"STOP_LIMIT"` | no | Auto-detected from price vs current price when omitted |
| `pside` | value → Direction | no | Position side (hedge mode) |
| `mark` | value → String | no | Tag for matching. Orders are matched by `mark` + `pside`. **Always use `mark` to avoid creating duplicate orders** |
| `msg` | string | no | Log message |

```json
{ "type": "CreateOrder", "amount": { "type": "Number", "value": 100 }, "side": { "type": "Direction", "value": "LONG" }, "price": { "type": "Global", "value": "Price" }, "order_type": "LIMIT", "mark": { "type": "String", "value": "DCA1" } }
```

---

### `RemoveOrder` — cancel a marked order

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `mark` | value → String | **yes** | Tag of the order to cancel |
| `pside` | value → Direction | no | Position side (hedge mode) |

```json
{ "type": "RemoveOrder", "mark": { "type": "String", "value": "DCA1" } }
```

---

### `SetVariable` / `ClearVariable` — manage bot state

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | **yes** | Must match the `key` from `variables` array |
| `value` | any value expression | **yes** (SetVariable only) | New value |

```json
{ "type": "SetVariable", "name": "counter", "value": { "type": "Number", "value": 1.0 } }
{ "type": "ClearVariable", "name": "counter" }
```

---

### `SetDirection`, `SetAmount`, `SetEnterPrice`, `ClearEnterPrice`, `SetOrderType` — update global values

These actions update the bot's global values (readable via `{ "type": "Global", "value": "Direction" }` etc.). They do not directly affect open positions or orders — they change what subsequent actions and evaluations will use.

`SetDirection` only affects how direction-aware auto comparisons (`>A`, `<A`) resolve in conditions.

| Action | Field | Type | Description |
|--------|-------|------|-------------|
| `SetDirection` | `value` | value → Direction | Updates `Global.Direction` |
| `SetAmount` | `value` | value → Float | Updates `Global.Amount` |
| `SetEnterPrice` | `value` | value → Float | Updates `Global.EnterPrice` |
| `ClearEnterPrice` | — | — | Resets `Global.EnterPrice` to default |
| `SetOrderType` | `value` | value → OrderType | Updates the default order type |

```json
{ "type": "SetDirection", "value": { "type": "Position", "value": "DirectionOpposite" } }
{ "type": "SetAmount", "value": { "type": "Number", "value": 200.0 } }
{ "type": "SetEnterPrice", "value": { "type": "Global", "value": "Price" } }
{ "type": "ClearEnterPrice" }
{ "type": "SetOrderType", "value": { "type": "OrderType", "value": "LIMIT" } }
```

---

### `SetPositionEnterPrice` — override recorded entry price of an open position

Overrides the average entry price stored on an open position. TP/SL calculations will anchor to this new price instead of the actual fill price.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `value` | value → Float | **yes** | New entry price |
| `side` | value → Direction | no | Target position side in hedge mode. Omit to target current position |

```json
{ "type": "SetPositionEnterPrice", "value": { "type": "Global", "value": "Price" } }
```


---

### `ForceStopBot` — stop the bot entirely

Stops the bot immediately, regardless of open positions.

| Field | Type | Required |
|-------|------|----------|
| `msg` | string | no |

```json
{ "type": "ForceStopBot", "msg": "Conditions expired" }
```

---

### `FireAlert` — send a push notification

Queues a persistent notification (saved to bell feed + broadcast via WebSocket). Bot name and symbol are included automatically.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `msg` | array of parts | **yes** | Message content — array of `Text` and `Value` parts |

**Part types:**

| Part type | Fields | Description |
|-----------|--------|-------------|
| `Text` | `value` (string) | Literal text |
| `Value` | `value` (value expression), `precision` (optional int) | Dynamic value resolved at runtime. `precision` controls decimal places |

```json
{ "type": "FireAlert", "msg": [
  { "type": "Text", "value": "PnL: " },
  { "type": "Value", "value": { "type": "Position", "value": "Pnl" }, "precision": 2 }
]}
```

**Rules:**
- At least one part required. Empty/whitespace-only messages are silently ignored.
- If a `Value` resolves to `None`, that segment is omitted.
- Backward compatible: `"msg": "plain string"` is also accepted.
- Fire-and-forget — does not block subsequent actions or affect bot state.

---

### `Actions` — sequential block of sub-actions

Wraps multiple actions into one. Each sub-action has its own `filters`.

```json
{ "type": "Actions", "actions": [
  { "filters": [], "action": { "type": "SetVariable", "name": "x", "value": { "type": "Number", "value": 1 } } },
  { "filters": [], "action": { "type": "ForceStopBot", "msg": "Done" } }
]}
```

---

### `Break` — early exit from nested action lists

Stops processing the current action list and optionally exits outer levels.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `level` | integer | `1` | How many nesting levels to exit |

- `level: 1` — exits the current `Actions` block or handler array
- `level: 2` — exits 2 levels (e.g., inner `Actions.actions[]` AND the outer `on_finished[]`)

Use with `Actions` in `on_finished` to implement conditional branches where only one branch fires.

```json
{ "type": "Break", "level": 2 }
```

---

### `Wait` — set bot to waiting mode

No fields. Sets the bot into waiting mode (same as when `enter_price: Wait` is used at spawn). Rarely needed since waiting mode is typically set via `enter_price: Wait` at initialization.

```json
{ "type": "Wait" }
```

---

## 7. Notes

**Position amount checks — `IsEmpty` vs `== 0` vs `> 0`:**

These three checks have distinct meanings:

| Check | Meaning |
|-------|---------|
| `IsEmpty(Position.Amount)` | No position exists — no orders placed, nothing in progress |
| `Position.Amount == 0` | Position exists with at least one pending entry order, but not filled yet. Use `SetPositionEnterPrice` to change the entry order price |
| `Position.Amount > 0` | Position is open on the exchange. Use `CreateOrder` to add extra amount or partially close |

```json
// No position at all
{ "type": "IsEmpty", "value": { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Amount" } }

// Position created but not filled yet
{ "type": "Operation", "operation": "==",
  "left": { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Amount" },
  "right": { "type": "Number", "value": 0.0 } }

// Position open on exchange
{ "type": "Operation", "operation": ">",
  "left": { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Amount" },
  "right": { "type": "Number", "value": 0.0 } }
```

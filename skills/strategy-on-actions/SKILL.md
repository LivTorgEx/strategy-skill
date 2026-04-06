---
name: strategy-on-actions
description: LivTorgEx event handler reference — on_analysis, on_indicators, on_actions. Explains when to use each and how variables work for storing strategy state.
---

# LivTorgEx — Event Handlers Reference

## Structure

`on_analysis`, `on_indicators`, and `on_actions` are **separate top-level arrays** inside `professional` — not wrapped in a single `on_actions` array.

```json
"professional": {
  "variables": [],
  "on_analysis":   [ { "type": "Action", "filters": [], "action": {} } ],
  "on_indicators": [ { "timeframe": 300, "filters": [], "actions": [ { "type": "Action", "filters": [], "action": {} } ] } ],
  "on_finished":   [ { "type": "Action", "filters": [], "action": {} } ],
  "on_actions":    [ { "name": "manual_close", "params": [], "actions": [] } ]
}
```

**IMPORTANT:** Every item in `on_analysis`, `on_finished`, and `on_indicators[].actions` **must** include `"type": "Action"` as the first field.

---

## What is a bot?

A **bot** is an instance of real-time market monitoring created by the bot group. It is NOT just a position — a bot can:

- **Monitor the market** with no open position — updating variables, watching conditions
- **Start a position** via `ForceStartPosition` (explicitly in actions, or automatically via `enter_price: Force`)
- **Stop a position** via `ForceStopPosition`
- **Stop itself** via `ForceStopBot` (exits the bot entirely, even with no position)

Bot states: `waiting` (no position) → `active` (has position) → `finished` (position closed). `on_finished` fires on position close.

### `max_active_bots` and margin

`max_active_bots` limits how many bots exist simultaneously. This caps total capital deployment:

> `margin = 5.0`, `max_active_bots = 3` → max simultaneous exposure = **$15**

A bot can open positions larger than its initial margin if it has accumulated **live (unrealised) profit** during its lifetime — that profit can be used as extra margin while the bot is running.

---

## Bot lifecycle and `enter_price`

Understanding when each handler fires requires understanding the bot lifecycle:

1. **Bot group** evaluates `professional.filters` when triggered by the `signal`. If all filters pass → **spawns a new bot** (subject to `max_active_bots`).
2. The `signal` defines *what triggers the filter check* — it is not limited to a timer. `Indicator` (candle-close) is most common, but signals can react to external events, volume spikes, etc.
3. **`enter_price`** determines what the freshly spawned bot does immediately:
   - `Force` — opens a position at market price right away. No `ForceStartPosition` action needed.
   - `Wait` — bot starts in waiting state. **You must call `ForceStartPosition` from `on_analysis` or `on_indicators`** to enter. Use `ForceStopBot` to exit if conditions expire without entering.
4. Once the bot has an open position, `on_analysis` / `on_indicators` run continuously to manage it.
5. When the position closes, **`on_finished`** fires once, then the bot exits.

**Decision rule:**
- Use `enter_price: Force` when spawn conditions in `professional.filters` are sufficient to enter immediately — e.g. fresh MRC cross + NTPS > 50.
- Use `enter_price: Wait` when you need a **two-phase** approach: spawn on one condition, then enter only when a secondary real-time condition passes in `on_analysis`/`on_indicators`.

> **Re-spawn prevention:** `professional.filters` do NOT self-throttle. If conditions stay true across multiple signal ticks, a new bot spawns each tick (up to `max_active_bots`). Use one-candle events (e.g. `MRC PrevCross >= 1 AND CurrentCross < 1`) in filters to make the condition naturally single-fire per event.

---

## `on_analysis` — use this by default

The main strategy loop. Runs every ~1s with fresh price and indicator data. Use it (with `enter_price: Wait`) to place orders, close positions, update variables, and react dynamically to market conditions.

**Use for:** entry logic (when `enter_price: Wait`), exit logic, trailing stops, PnL checks, variable updates.

Each item: `{ "filters": [...], "action": {...} }` — all filters must pass for the action to run.

```json
"on_analysis": [
  {
    "type": "Action",
    "filters": [
      { "type": "Operation", "operation": "==",
        "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                   "indicator": { "type": "Supertrend", "property": "Direction" } },
        "right": { "type": "Direction", "value": "LONG" } }
    ],
    "action": {
      "type": "ForceStartPosition",
      "side": { "type": "Direction", "value": "LONG" },
      "msg": "Supertrend Long"
    }
  }
]
```

---

## `on_indicators` — use when reacting to candle close events

Fires when a new candle closes at the specified timeframe. Use it when the trigger is a candle close (e.g. new EMA value, Supertrend flip) rather than continuous price monitoring.

**Use for:** EMA direction changes, Supertrend flips, RSI threshold breaks, storing flags in variables.
**Avoid for:** general order placement or PnL checks — use `on_analysis` for those.

Each item: `{ "timeframe": int, "filters": [...], "actions": [...] }`

```json
"on_indicators": [
  {
    "timeframe": 300,
    "filters": [
      { "type": "Operation", "operation": "==",
        "left":  { "type": "Indicator", "token": "Chart", "timeframe": 300, "idx": 0,
                   "indicator": { "type": "Ema", "period": "5", "property": "Direction" } },
        "right": { "type": "Direction", "value": "LONG" } }
    ],
    "actions": [
      {
        "type": "Action",
        "filters": [],
        "action": { "type": "SetVariable", "name": "ema5_long", "value": { "type": "Number", "value": 1.0 } }
      }
    ]
  }
]
```

---

## `on_finished` — fires when a position closes

Use to revert direction and re-enter on the opposite side after a close.

```json
"on_finished": [
  {
    "type": "Action",
    "filters": [],
    "action": { "type": "SetDirection", "value": { "type": "Position", "value": "DirectionOpposite" } }
  },
  {
    "type": "Action",
    "filters": [],
    "action": { "type": "ForceStartPosition", "side": { "type": "Global", "value": "Direction" }, "msg": "Revert" }
  }
]
```

**Note:** `ForceStartPosition.side` is **required**. Use `{ "type": "Global", "value": "Direction" }` to follow whatever direction was just set by `SetDirection`.

---

## `on_actions` — avoid; only for very specific manual tasks

Fires on a manual user action from the UI. **This is not for auto trading.** Only use it when the user explicitly needs to trigger something manually.

In almost all auto-trading strategies, `on_actions` should not be present.

Each item: `{ "name": "...", "params": [...], "actions": [...] }`

```json
"on_actions": [
  {
    "name": "manual_close",
    "params": [],
    "actions": [
      {
        "type": "Action",
        "filters": [],
        "action": { "type": "ForceStopPosition", "side": null, "msg": "Manual close" }
      }
    ]
  }
]
```

---

## Variables — storing strategy state

`variables` is declared at the top of `professional` and holds named values that persist across ticks. Use them to share state between `on_indicators` and `on_analysis`, track multi-step conditions, or accumulate counters.

```json
"variables": [
  { "type": "Variable", "name": "EMA5 Direction", "key": "ema5_long",       "default": { "type": "Number", "value": 0.0 } },
  { "type": "Variable", "name": "Entry Confirmed", "key": "entry_confirmed", "default": { "type": "Number", "value": 0.0 } }
]
```

**IMPORTANT:** Every variable entry **must** include `"type": "Variable"`.

- `name` — display label only
- `key` — used everywhere to reference this variable (in SetVariable, ClearVariable, Variable value)
- `default` — initial value (optional)

Read a variable in a filter:
```json
{ "type": "Operation", "operation": "==",
  "left":  { "type": "Variable", "name": "ema5_long" },
  "right": { "type": "Number", "value": 1.0 } }
```

Set / clear a variable in an action:
```json
{ "type": "SetVariable",   "name": "ema5_long", "value": { "type": "Number", "value": 1.0 } }
{ "type": "ClearVariable", "name": "ema5_long" }
```

---

## Combined pattern — on_indicators + on_analysis

Use `on_indicators` to detect an EMA direction change on the 5m chart and store it in a variable. Use `on_analysis` to act on that state along with a live RSI check:

```json
"variables": [
  { "type": "Variable", "name": "EMA5 Long", "key": "ema5_long", "default": { "type": "Number", "value": 0.0 } }
],
"on_indicators": [
  {
    "timeframe": 300,
    "filters": [],
    "actions": [
      {
        "type": "Action",
        "filters": [
          { "type": "Operation", "operation": "==",
            "left":  { "type": "Indicator", "token": "Chart", "timeframe": 300, "idx": 0,
                       "indicator": { "type": "Ema", "period": "5", "property": "Direction" } },
            "right": { "type": "Direction", "value": "LONG" } }
        ],
        "action": { "type": "SetVariable", "name": "ema5_long", "value": { "type": "Number", "value": 1.0 } }
      },
      {
        "type": "Action",
        "filters": [
          { "type": "Operation", "operation": "==",
            "left":  { "type": "Indicator", "token": "Chart", "timeframe": 300, "idx": 0,
                       "indicator": { "type": "Ema", "period": "5", "property": "Direction" } },
            "right": { "type": "Direction", "value": "SHORT" } }
        ],
        "action": { "type": "ClearVariable", "name": "ema5_long" }
      }
    ]
  }
],
"on_analysis": [
  {
    "type": "Action",
    "filters": [
      { "type": "Operation", "operation": "==",
        "left":  { "type": "Variable", "name": "ema5_long" },
        "right": { "type": "Number", "value": 1.0 } },
      { "type": "Operation", "operation": "<",
        "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                   "indicator": { "type": "Rsi", "period": "14", "property": "Value" } },
        "right": { "type": "Number", "value": 35.0 } }
    ],
    "action": {
      "type": "ForceStartPosition",
      "side": { "type": "Direction", "value": "LONG" },
      "msg": "EMA5 Long + RSI oversold"
    }
  },
  {
    "type": "Action",
    "filters": [
      { "type": "Operation", "operation": ">",
        "left":  { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Pnl" },
        "right": { "type": "Number", "value": 150.0 } }
    ],
    "action": {
      "type": "ForceStopPosition",
      "side": { "type": "Direction", "value": "LONG" },
      "msg": "Profit target"
    }
  }
]
```

---

## All available actions

```json
{ "type": "ForceStartPosition", "amount": {}, "side": { /* required — use {"type":"Global","value":"Direction"} to follow current set direction */ }, "enter_price": {}, "order_type": {}, "msg": "" }
{ "type": "ForceStopPosition",  "side": null, "msg": "" }
{ "type": "CreateOrder", "amount": {}, "side": {}, "price": {}, "order_type": "MARKET"|"LIMIT"|"STOP_MARKET"|"STOP_LIMIT", "pside": {}, "mark": {}, "msg": "" }
{ "type": "RemoveOrder",    "mark": {}, "pside": {} }
{ "type": "SetVariable",    "name": "key", "value": {} }
{ "type": "ClearVariable",  "name": "key" }
{ "type": "SetDirection",   "value": {} }
{ "type": "SetAmount",      "value": {} }
{ "type": "SetEnterPrice",  "value": {} }
{ "type": "ClearEnterPrice" }
{ "type": "ForceStopBot",   "msg": "" }
{ "type": "Wait" }
{ "type": "Break", "level": 1 }
```

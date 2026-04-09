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
  "on_created":    [ { "type": "Action", "filters": [], "action": {} } ],
  "on_indicators": [ { "timeframe": 300, "filters": [], "actions": [ { "type": "Action", "filters": [], "action": {} } ] } ],
  "on_finished":   [ { "type": "Action", "filters": [], "action": {} } ],
  "on_actions":    [ { "name": "manual_close", "params": [], "actions": [] } ]
}
```

**IMPORTANT:** Every item in `on_analysis`, `on_created`, `on_finished`, and `on_indicators[].actions` **must** include `"type": "Action"` as the first field.

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
5. When the position closes, **`on_finished`** fires once:
   - If `on_finished` is **empty** → bot stops automatically.
   - If `on_finished` has **any actions** → bot enters **waiting mode** and stays alive. Add `ForceStopBot` explicitly to stop it.

**Decision rule:**
- Use `enter_price: Force` when spawn conditions in `professional.filters` are sufficient to enter immediately — e.g. fresh MRC cross + NTPS > 50.
- Use `enter_price: Wait` when you need a **two-phase** approach: spawn on one condition, then enter only when a secondary real-time condition passes in `on_analysis`/`on_indicators`.

> **Re-spawn prevention:** `professional.filters` do NOT self-throttle. If conditions stay true across multiple signal ticks, a new bot spawns each tick (up to `max_active_bots`). Use one-candle events (e.g. `MRC PrevCross >= 1 AND CurrentCross < 1`) in filters to make the condition naturally single-fire per event.

---

## `on_created` — fires once when a new position opens

`on_created` fires **once** immediately after a position is opened (first entry order fills). Use for one-time position setup that depends on the actual entry price or entry side.

**Use for:** setting variables from entry price, logging entry, one-time setup.
**Prefer over `on_analysis`** for setup to avoid re-running on every tick.

```json
"on_created": [
  {
    "type": "Action",
    "filters": [],
    "action": {
      "type": "SetVariable",
      "name": "initial_entry",
      "value": { "type": "Position", "value": "EntryPrice" }
    }
  }
]
```

---

## `direction: "Both"` — LONG + SHORT in a single bot

`direction: "Both"` allows a single bot to use both LONG and SHORT. This enables two patterns:

- **Hedge mode** — hold LONG and SHORT positions simultaneously (both open at the same time)
- **Reversal mode** — close one side and open the opposite (bot reverses direction)

`Long` and `Short` restrict the bot to a single direction only.

**Settings when using `Both`:**
- `max_active_bots` — controls how many bots (symbols) the group can run; not related to direction
- `enter_price` — any type is valid; describes what the bot does after starting
- `margin_mode` — any; cross margin is preferred over isolated

```json
"on_analysis": [
  {
    "type": "Action",
    "filters": [
      { "type": "Operation", "operation": "==",
        "left":  { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Amount" },
        "right": { "type": "Number", "value": 0 } }
    ],
    "action": { "type": "ForceStartPosition", "side": { "type": "Direction", "value": "LONG" }, "msg": "Open LONG" }
  },
  {
    "type": "Action",
    "filters": [
      { "type": "Operation", "operation": "==",
        "left":  { "type": "Position", "side": { "type": "Direction", "value": "SHORT" }, "value": "Amount" },
        "right": { "type": "Number", "value": 0 } }
    ],
    "action": { "type": "ForceStartPosition", "side": { "type": "Direction", "value": "SHORT" }, "msg": "Open SHORT" }
  }
]
```

> **Re-entry:** Use `on_finished` to re-open the closed side. Check `on_finished` section above for the revert/re-entry pattern.

---

## `on_analysis` — use this by default

The main strategy loop. Runs every ~1s with fresh price and indicator data. Use it to place orders, close positions, update variables, and react dynamically to market conditions.

**Use for:** entry logic, exit logic, trailing stops, PnL checks, variable updates.

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

**Critical behaviour: `on_finished` actions put the bot into waiting mode.**

- **`on_finished` is empty** → bot stops automatically after position closes.
- **`on_finished` has any actions** → bot transitions to **waiting mode** after position closes and stays alive. It will NOT stop on its own.
  - Add `ForceStopBot` as the last action if you want the bot to stop after running `on_finished`.
  - Omit `ForceStopBot` if you want the bot to stay alive and re-enter (e.g. grid, revert strategies).

```json
// Pattern A — run cleanup then stop bot
"on_finished": [
  {
    "type": "Action",
    "filters": [],
    "action": { "type": "SetVariable", "name": "last_side", "value": { "type": "Position", "value": "Direction" } }
  },
  {
    "type": "Action",
    "filters": [],
    "action": { "type": "ForceStopBot", "msg": "Cleanup done, stopping" }
  }
]

// Pattern B — revert direction and re-enter (bot stays alive in waiting, immediately re-enters)
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
- `default` — initial value (optional). Accepts **any `TradeSettingProValue` expression** — not just `Number`.

### Variable evaluation order

Variables are **initialised sequentially** in array order at bot spawn time. Each variable's `default` expression is evaluated before moving to the next. This means a variable can reference **any variable defined before it** (lower index) in its `default` expression — the previous variable's value is already resolved.

```json
"variables": [
  {
    "type": "Variable", "name": "MRC Range %", "key": "mrc_range_pct",
    "default": {
      "type": "PriceMeasure", "is_abs": true,
      "left":  { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
                 "indicator": { "type": "Mrc", "period": "200", "property": "DownInner" } },
      "right": { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
                 "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } }
    }
  },
  {
    "type": "Variable", "name": "SL %", "key": "sl_pct",
    "default": {
      "type": "Math",
      "value": {
        "type": "Operation", "operation": "/",
        "left":  { "type": "Variable", "name": "mrc_range_pct" },
        "right": { "type": "Number", "value": -2.0 }
      }
    }
  }
]
```

Here `sl_pct` reads `mrc_range_pct` in its default because `mrc_range_pct` is at index 0 (already computed).

> **Rule:** only reference variables with a **lower index** in a `default` expression. Forward references (higher index) are not yet resolved.

All defaults are evaluated at **bot spawn time** (before the first candle close). Combine with `on_indicators` `SetVariable` to refresh values on every candle close.

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

### `Break` — early exit from nested action lists

`Break` stops processing the current action list and optionally exits outer levels.

- `level: 1` — exits the **current** action list (the `Actions` block or the handler array you're in)
- `level: 2` — exits **2 levels**: the current block AND the outer handler list (e.g., exits both the inner `Actions.actions[]` and the outer `on_finished[]`)
- Higher levels exit that many levels of nesting

**Use in `on_finished` to prevent a catch-all fallback from running after a conditional branch:**

```json
// on_finished with conditional branches — only one branch fires
"on_finished": [
  {
    "type": "Action",
    "filters": [ { /* condition A — e.g. SL hit */ } ],
    "action": {
      "type": "Actions",
      "actions": [
        { "filters": [], "action": { "type": "ForceStopBot", "msg": "SL — stop" } },
        { "filters": [], "action": { "type": "Break", "level": 2 } }
      ]
    }
  },
  {
    "type": "Action",
    "filters": [ { /* condition B — e.g. TP hit */ } ],
    "action": {
      "type": "Actions",
      "actions": [
        { "filters": [], "action": { "type": "SetVariable", "name": "counter", "value": { "type": "Number", "value": 0 } } },
        { "filters": [], "action": { "type": "Break", "level": 2 } }
      ]
    }
  },
  {
    "type": "Action",
    "filters": [],
    "action": { "type": "ForceStopBot", "msg": "Fallback — should never run after A or B" }
  }
]
```

> **Note on waiting mode:** When a bot is in **waiting mode** (no open position, after `on_finished`), do NOT use `Position.Amount == 0` as a guard in `on_indicators` — in waiting mode `Position.Amount` may not resolve correctly. Use `IsEmpty` with a Position value to reliably detect no active position:
> ```json
> { "type": "IsEmpty", "value": { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Amount" } }
> ```
> Combine with a variable flag (e.g. `position_count == 1`) to track which lifecycle stage the bot is in.

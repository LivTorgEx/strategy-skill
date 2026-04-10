---
name: strategy-initialization
description: LivTorgEx bot spawn initialization reference — enter_price, enter_direction, enter_amount, auto_max_amount_leverage, changes, and professional.filters. Use when configuring how and when a bot enters a position.
---

# LivTorgEx — Bot Initialization Reference

These fields on `professional` control **how a bot enters a position** at spawn time. They are evaluated once when the bot group decides to spawn a new bot.

---

## Initialization evaluation order

When a new bot spawns, these fields are evaluated in this order:

1. `changes` — override direction or amount before variables are set
2. `variables` — initialise named variables (may reference changed direction/amount)
3. `enter_direction` — resolve the initial trade direction
4. `enter_amount` — resolve the initial position size
5. `enter_price` — decide entry mode (Force, Wait, Signal, Indicator)

---

## `enter_price` — entry mode

Controls what the spawned bot does immediately.

```json
{ "type": "Force" }
```

| Type | Behaviour |
|------|-----------|
| `Force` | Opens position at market price immediately on spawn — no `ForceStartPosition` needed |
| `Wait` | Bot spawns in waiting state — **must call `ForceStartPosition`** from `on_analysis` or `on_indicators` to enter |
| `Signal` | Enters at the signal's suggested price with an optional `price` offset |
| `Indicator` | Enters at the value of a specified indicator |

**`Force` example** (immediate market entry):
```json
"enter_price": { "type": "Force" }
```

**`Wait` example** (deferred entry — use with `on_analysis`/`on_created`):
```json
"enter_price": { "type": "Wait" }
```

**`Signal` example** (enter at signal price ± offset):
```json
"enter_price": {
  "type": "Signal",
  "price": { "type": "Percentage", "value": -0.1 }
}
```

**`Indicator` example** (enter at EMA value):
```json
"enter_price": {
  "type": "Indicator",
  "timeframe": 3600,
  "idx": 0,
  "indicator": "Ema",
  "period": "50",
  "property": "Value"
}
```

> **Decision rule:**
> - Use `Force` when the spawn filters are sufficient — enter immediately on match. **Grid strategies can use `Force`: the grid modification activates the moment the position opens, no extra actions needed.**
> - Use `Wait` when you want the bot in monitoring mode first — spawn on one condition, then enter only when a secondary real-time condition passes in `on_analysis`/`on_indicators`.
> - Use `Signal` / `Indicator` when the entry price should track a specific level rather than current market price.
> - For `direction: "Both"`, any `enter_price` type is valid — it just describes what the bot does after starting.

---

## `enter_direction` — initial direction

Resolves the bot's starting trade direction.

```json
{ "type": "Default" }
```

| Type | Description |
|------|-------------|
| `Default` | Uses the bot group's `strategy.direction` |
| `Signal` | Follows or opposes the incoming signal direction — `value: "Follow"` or `"Opposite"` |
| `Indicator` | Reads direction from an indicator's `Direction` property |
| `Value` | Computes direction from any value expression — `value` + `direction: "Follow"\|"Opposite"` |

**`Default` example:**
```json
"enter_direction": { "type": "Default" }
```

**`Signal` example** (follow signal direction):
```json
"enter_direction": { "type": "Signal", "value": "Follow" }
```

**`Signal` opposite** (fade the signal):
```json
"enter_direction": { "type": "Signal", "value": "Opposite" }
```

**`Indicator` example** (enter in Supertrend direction):
```json
"enter_direction": {
  "type": "Indicator",
  "timeframe": 3600,
  "idx": 0,
  "indicator": "Supertrend",
  "direction": "Follow"
}
```

> For `direction: "Both"`, `enter_direction` is typically `Default` — direction is resolved per-position by the explicit `side` field in each `ForceStartPosition` action.

---

## `enter_amount` — position size

Defines the initial margin amount for the bot's first entry order. Currently only `Percentage` is supported.

```json
{ "type": "Percentage", "source": "MaxAmount", "value": 100.0 }
```

| `source` | Description |
|----------|-------------|
| `MaxAmount` | % of the bot's `max_open_amount` (or the current `AutoMaxAmount`) |
| `FirstOrder` | % of the first order size — only valid for subsequent orders, not enter_amount |
| `LastOrder` | % of the last order size — same caveat as above |

**Standard examples:**

```json
// Use full budget
"enter_amount": { "type": "Percentage", "source": "MaxAmount", "value": 100.0 }

// Use 25% of budget for first order (DCA strategy)
"enter_amount": { "type": "Percentage", "source": "MaxAmount", "value": 25.0 }
```

> **Important:** `enter_amount` is the margin allocated to the first entry order. Subsequent DCA orders use `CreateOrder` amounts in `on_analysis`. For grid strategies, the grid computes per-level qty from `enter_amount`.

---

## `auto_max_amount_leverage` — compound PnL multiplier

An optional value that controls how much of a closed position's PnL is reinvested into `AutoMaxAmount` for the next position.

```json
"auto_max_amount_leverage": { "type": "Number", "value": 1.0 }
```

**Formula after each position close:**
```
AutoMaxAmount = AutoMaxAmount_prev + position_pnl × auto_max_amount_leverage
```

When omitted: `AutoMaxAmount = max_open_amount + total_pnl` (default full compounding).

| Value | Effect |
|-------|--------|
| `0.0` | PnL is never added — `AutoMaxAmount` stays at `max_open_amount` (no compounding) |
| `1.0` | Full PnL reinvested (standard compounding) |
| `2.0` | Double PnL reinvested (aggressive compounding) |
| `{ "type": "Number", "value": 8.0 }` | Example seen in multi-limit strategies with leverage |

**Usage in variables:**
```json
"variables": [
  {
    "type": "Variable", "name": "MaxAmount", "key": "mxamt",
    "default": { "type": "Global", "value": "AutoMaxAmount" }
  }
]
```

> `AutoMaxAmount` is the *rolling* amount that accounts for cumulative PnL. Use `{ "type": "Global", "value": "AutoMaxAmount" }` in `enter_amount` or variables to always size positions from the compounded balance.

---

## `filters` — bot spawn gate

`professional.filters` is the AND-list of conditions checked **once when the bot group decides to spawn a new bot** (on each `signal` tick). If all filters pass → a new bot spawns (subject to `max_active_bots`).

```json
"filters": [
  { "type": "Operation", "operation": ">",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
               "indicator": { "type": "Ntps", "property": "Value" } },
    "right": { "type": "Number", "value": 50.0 } },
  { "type": "Operation", "operation": ">",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
               "indicator": { "type": "Natr", "period": "14", "property": "Value" } },
    "right": { "type": "Number", "value": 0.3 } }
]
```

**Key rules:**
- Items are **AND** by default — all must pass
- Evaluated only at spawn time — NOT re-checked while the bot runs
- Do NOT use filters to prevent re-spawning if conditions stay true across multiple ticks — use one-candle events (e.g. `MRC PrevCross >= 1 AND CurrentCross < 1`) instead
- Use `Order` value with `pside` to check if a position side already exists (avoid double-spawning in hedge mode)

**Common spawn gate patterns:**

```json
// Only spawn when NTPS > 50 (quality signal)
{ "type": "Operation", "operation": ">",
  "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
             "indicator": { "type": "Ntps", "property": "Value" } },
  "right": { "type": "Number", "value": 50.0 } }

// Only spawn when NATR > 0.3% (enough volatility)
{ "type": "Operation", "operation": ">",
  "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
             "indicator": { "type": "Natr", "period": "14", "property": "Value" } },
  "right": { "type": "Number", "value": 0.3 } }

// Only spawn once — MRC cross event (fires once per cross)
{ "type": "Operation", "operation": ">=",
  "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
             "indicator": { "type": "Mrc", "period": "200", "property": "PrevCross" } },
  "right": { "type": "Number", "value": 1.0 } }
```

---

## `changes` — pre-spawn context overrides

`changes` runs **before** `variables` are initialised. Use it to override `direction` or `amount` in the bot context based on conditions.

Each entry has an optional `condition` (single filter) and a `value`.

### `Direction` change

Overrides the bot's direction. Currently only `Reverse` is supported as value — it flips the current direction.

```json
"changes": [
  {
    "type": "Direction",
    "condition": {
      "type": "Operation", "operation": "==",
      "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                 "indicator": { "type": "Supertrend", "property": "Direction" } },
      "right": { "type": "Direction", "value": "SHORT" }
    },
    "value": "Reverse"
  }
]
```

> `"value": "Reverse"` flips the current direction: LONG → SHORT, SHORT → LONG.

### `Amount` change

Overrides the bot's initial amount using a `Math` expression.

```json
"changes": [
  {
    "type": "Amount",
    "condition": null,
    "value": {
      "type": "Operation",
      "operation": "*",
      "left":  { "type": "Global", "value": "AutoMaxAmount" },
      "right": { "type": "Number", "value": 0.5 }
    }
  }
]
```

> `condition: null` means the change always applies.

**When to use `changes` vs `variables`:**
- Use `changes` when you need to override `direction` or override `amount` **before** variables are evaluated — because variables can reference these modified values in their `default` expressions.
- Use `variables` for everything else.

---

## Full initialization example (`direction: "Both"`, grid)

```json
"enter_price":     { "type": "Force" },
"enter_direction": { "type": "Default" },
"enter_amount":    { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
"auto_max_amount_leverage": { "type": "Number", "value": 1.0 },
"changes": [],
"filters": [
  { "type": "Operation", "operation": ">",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
               "indicator": { "type": "Ntps", "property": "Value" } },
    "right": { "type": "Number", "value": 40.0 } }
]
```

> `direction: "Both"` on the bot group allows the bot to use both LONG and SHORT — either simultaneously (hedge mode) or by reversing. Use `enter_price: Force` so positions open immediately on spawn; the grid modification activates at the moment each position is initialised. For `margin_mode`, cross margin is preferred over isolated.

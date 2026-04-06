---
name: strategy-conditions
description: LivTorgEx condition, value, and action syntax reference. Use when building strategy filters, entry conditions, or action logic.
---

# LivTorgEx — Conditions & Values

## Conditions (used in `filters` arrays)

```json
{ "type": "Operation", "operation": "==" | "!=" | "<" | ">" | "<A" | ">A",
  "left": { /* value */ }, "right": { /* value */ } }

{ "type": "Expression", "expression": "And" | "Or",
  "values": [ /* condition, ... */ ] }

{ "type": "FewOf", "true_ones": 2,
  "values": [ /* condition, ... */ ] }

{ "type": "IsEmpty", "value": { /* value */ } }
```

### `<A` / `>A` — direction-aware auto-reversing comparisons

These operations flip their meaning based on `global.direction`:

| Operation | When LONG | When SHORT |
|-----------|-----------|------------|
| `>A`      | `>`       | `<`        |
| `<A`      | `<`       | `>`        |

Use `<A`/`>A` when the same condition should work for both long and short entries without duplicating logic. For example, "price has crossed above the target" can be written as one condition with `>A` — for a long it checks `price > target`, for a short it checks `price < target`.

**Root-level `filters` arrays are AND by default** — every item must pass. Use multiple items instead of `Expression: And`. Only use `Expression` when you need OR logic or nesting.

---

## Values

```json
{ "type": "Number",    "value": 30.0 }
{ "type": "Direction", "value": "LONG" | "SHORT" | "BOTH" }

{ "type": "Global", "value":
    "MaxAmount" | "Price" | "Direction" | "DirectionOpposite" |
    "EnterPrice" | "Leverage" | "Margin" | "Factor" |
    "TotalPnL" | "Amount" | "AutoMaxAmount" | "PriceTick" | "PriceTickPrc" }

{ "type": "Variable", "name": "var_key" }

{ "type": "Position",
  "side": { "type": "Direction", "value": "LONG" } | null,
  "value": "Amount" | "EntryPrice" | "Pnl" | "Fee" |
           "Direction" | "DirectionOpposite" | "Factor" }

{ "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
  "indicator": { /* see strategy-indicators skill */ } }

{ "type": "Order",
  "mark":  { "type": "String", "value": "Open" },
  "pside": { "type": "Position", "value": "Direction" },
  "value": "Exist" }

// Percentage distance between two prices: (right - left) / left * 100
// Use is_abs: true for unsigned distance
{ "type": "PriceMeasure",
  "left":  { /* value — base price */ },
  "right": { /* value — target price */ },
  "is_abs": false }

// Arithmetic on any two values: +, -, *, /
{ "type": "Math",
  "value": {
    "type": "Operation",
    "operation": "+" | "-" | "*" | "/",
    "left":  { /* value */ },
    "right": { /* value */ }
  }
}

// Where a value is between two bounds, normalised 0.0–1.0:
// result = (value - left) / (right - left). Use is_abs: true for unsigned result.
{ "type": "PercentageBetween",
  "left":  { /* value — lower bound */ },
  "right": { /* value — upper bound */ },
  "value": { /* value — the probe */ },
  "is_abs": false }
```

**Note:** `Direction` values are always uppercase: `"LONG"`, `"SHORT"`, `"BOTH"`.
**Note:** `Position.side` is a value object (e.g. `{"type":"Direction","value":"LONG"}`), not a string.
**Note:** `Position.value` field is `"EntryPrice"` (not `EnterPrice`).

---

## Actions (used in `on_analysis`, `on_indicators`, `on_actions` — see strategy-on-actions skill)

```json
{ "type": "ForceStartPosition",
  "amount": { /* value */ }, "side": { /* value */ },
  "enter_price": { /* value */ }, "order_type": { /* value */ },
  "msg": "label" }

{ "type": "ForceStopPosition", "side": null, "msg": "label" }

{ "type": "CreateOrder",
  "amount": { /* value */ }, "side": { /* value */ }, "price": { /* value */ },
  "order_type": "MARKET" | "LIMIT" | "STOP_MARKET" | "STOP_LIMIT",
  "pside": { /* value */ }, "mark": { /* value */ }, "msg": "label" }

{ "type": "RemoveOrder", "mark": { /* value */ }, "pside": { /* value */ } }

{ "type": "SetVariable",    "name": "var_key", "value": { /* value */ } }
{ "type": "ClearVariable",  "name": "var_key" }
{ "type": "SetDirection",   "value": { /* value */ } }
{ "type": "SetAmount",      "value": { /* value */ } }
{ "type": "SetEnterPrice",  "value": { /* value */ } }
{ "type": "ClearEnterPrice" }
{ "type": "SetOrderType",   "value": { "type": "OrderType", "value": "LIMIT" } }
{ "type": "ForceStopBot",   "msg": "reason" }
{ "type": "Wait" }
{ "type": "Break", "level": 1 }
```

---

## Common patterns

### AND of two indicator checks (use separate filter items — no Expression needed)
```json
[
  { "type": "Operation", "operation": "<",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
               "indicator": { "type": "Rsi", "period": "14", "property": "Value" } },
    "right": { "type": "Number", "value": 30.0 } },
  { "type": "Operation", "operation": "==",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
               "indicator": { "type": "Supertrend", "property": "Direction" } },
    "right": { "type": "Direction", "value": "LONG" } }
]
```

### OR of two conditions (Expression required)
```json
{ "type": "Expression", "expression": "Or",
  "values": [
    { "type": "Operation", "operation": "<", "left": { /* ... */ }, "right": { /* ... */ } },
    { "type": "Operation", "operation": ">", "left": { /* ... */ }, "right": { /* ... */ } }
  ]
}
```

### At least 2 of 3 conditions
```json
{ "type": "FewOf", "true_ones": 2, "values": [ /* 3 conditions */ ] }
```

### Position PnL check
```json
{ "type": "Operation", "operation": ">",
  "left":  { "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "Pnl" },
  "right": { "type": "Number", "value": 50.0 } }
```

### Variable check
```json
{ "type": "Operation", "operation": "==",
  "left":  { "type": "Variable", "name": "trend_confirmed" },
  "right": { "type": "Number", "value": 1.0 } }
```

### Order existence check (pside is required)
```json
{ "type": "Operation", "operation": "==",
  "left":  { "type": "Order",
             "mark":  { "type": "String", "value": "Open" },
             "pside": { "type": "Position", "value": "Direction" },
             "value": "Exist" },
  "right": { "type": "Number", "value": 0.0 } }
```

### Safe limit order placement — only place when price has crossed the target

A limit order placed at or beyond the current price fills immediately as a market order. To avoid this, guard the `CreateOrder` action with a `>A` check: only place the limit buy when price is **already above** the target (so the order sits below current price). For a short the same condition auto-reverses via `>A`.

```json
{
  "type": "Action",
  "filters": [
    { "type": "Operation", "operation": "==",
      "left":  { "type": "Order",
                 "mark":  { "type": "String", "value": "Open" },
                 "pside": { "type": "Global", "value": "Direction" },
                 "value": "Exist" },
      "right": { "type": "Number", "value": 0.0 } },
    { "type": "Operation", "operation": ">A",
      "left":  { "type": "Global", "value": "Price" },
      "right": { "type": "Variable", "name": "limit_price" } }
  ],
  "action": {
    "type": "CreateOrder",
    "amount": { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
    "side":   { "type": "Global", "value": "Direction" },
    "price":  { "type": "Variable", "name": "limit_price" },
    "order_type": "LIMIT",
    "pside":  { "type": "Global", "value": "Direction" },
    "mark":   { "type": "String", "value": "Open" },
    "msg": "Limit entry after price crossed"
  }
}
```

**Why `>A`:**
- LONG: places the limit order only when `price > limit_price` — order is below current price, will not fill immediately.
- SHORT: auto-reverses to `price < limit_price` — order is above current price, same guarantee.

### MRC-based TP/SL using PriceMeasure

Compute the TP % distance between two MRC bands and store half as the SL %. Use `on_indicators` to update the variable on every candle close, **before** the entry action.

```json
// variables declaration
"variables": [
  { "type": "Variable", "name": "SL %", "key": "sl_pct", "default": { "type": "Number", "value": -1.0 } }
],

// on_indicators action 1 — compute SL % = PriceMeasure(DownInner → UpBig) / -2
// PriceMeasure(DownInner, UpBig) gives a positive %; dividing by -2 makes it negative for SL
{
  "type": "Action",
  "filters": [],
  "action": {
    "type": "SetVariable",
    "name": "sl_pct",
    "value": {
      "type": "Math",
      "value": {
        "type": "Operation",
        "operation": "/",
        "left": {
          "type": "PriceMeasure",
          "left":  { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
                     "indicator": { "type": "Mrc", "period": "200", "property": "DownInner" } },
          "right": { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
                     "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } },
          "is_abs": false
        },
        "right": { "type": "Number", "value": -2.0 }
      }
    }
  }
},

// take_profit — target UpBig price at entry snapshot
"take_profit": { "type": "NextOrder", "order": {
  "order_type": "MARKET",
  "price": {
    "type": "Indicator",
    "source": { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
                "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } },
    "price": { "type": "Percentage", "value": 0.0 }
  },
  "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
}},

// stop_loss — entry price offset by sl_pct variable (negative %)
"stop_loss": { "type": "NextOrder", "only_great": false, "activates": [], "filters": [],
  "order": {
    "order_type": "MARKET",
    "price": { "type": "FirstPrice", "price": { "type": "PercentageVariable", "name": "sl_pct" } },
    "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
  }
}
```

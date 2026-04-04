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

`<A` / `>A` = absolute-value comparison.

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

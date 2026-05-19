---
name: strategy-tp-sl
description: LivTorgEx Take Profit and Stop Loss reference — NextOrder fields (only_great, activates, direction, filters), Optimizer multi-branch selection, TP/SL price base types, and dynamic TP/SL with variables.
---

# LivTorgEx — Take Profit and Stop Loss Reference

`professional.take_profit` and `professional.stop_loss` define how a position is closed when it reaches a target profit or loss. Both support two variants: `NextOrder` (standard) and `Optimizer` (multi-branch selection).

For TP/SL price base types and dynamic variable-based percentages, see also `strategy-conditions` skill.

---

## SL `NextOrder` — full field reference

```json
"stop_loss": {
  "type": "NextOrder",
  "direction": "BOTH",
  "only_great": false,
  "activates": [],
  "filters": [],
  "order": {
    "price": { "type": "Position", "price": { "type": "Percentage", "value": -2.0 } },
    "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
  }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `"LONG"` / `"SHORT"` / `"BOTH"` | `"BOTH"` | Which position side this SL applies to |
| `only_great` | bool | `true` | When `true`, SL only activates when position PnL is positive. **Set to `false` for standard SL.** |
| `activates` | array | `[]` | Activation conditions — SL order is NOT placed until all activate rules pass |
| `filters` | array of conditions | `[]` | Additional gate conditions checked before placing the SL order |
| `order` | order template | required | The SL order (price, amount, order_type, filters) |

> **`only_great` defaults to `true`** — this means a plain `NextOrder` SL without `"only_great": false` will only close when the position is in profit. For a standard stop-loss that fires at a loss, always set `"only_great": false`.

### `activates` — conditional SL activation

Each entry in `activates` defines a condition that must be met before the SL order is placed. This enables trailing-SL-like behavior.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | `"MarkPrice"` / `"SLPrice"` | required | What to compare — `MarkPrice` = current market price, `SLPrice` = the computed SL trigger price |
| `operation` | `">"` / `"<"` | `">"` | Comparison operator |
| `price` | PriceDepends | required | Reference price to compare against |

```json
"activates": [
  {
    "source": "MarkPrice",
    "operation": ">",
    "price": { "type": "Position", "price": { "type": "Percentage", "value": 1.0 } }
  }
]
```

> Example: SL only activates after price moves 1% above entry (break-even lock). Once activated, the SL order is placed at the configured price. Before activation, no SL order exists.

---

## TP `NextOrder` — full field reference

```json
"take_profit": {
  "type": "NextOrder",
  "direction": "BOTH",
  "order": {
    "price": { "type": "Position", "price": { "type": "Percentage", "value": 3.0 } },
    "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
  },
  "filters": []
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | `"LONG"` / `"SHORT"` / `"BOTH"` | `"BOTH"` | Which position side this TP applies to |
| `order` | order template | required | The TP order definition |
| `filters` | array of conditions | `[]` | Gate conditions checked before placing the TP order |

---

## SL `Optimizer` — best-of-multiple stop losses

Evaluates multiple SL definitions and picks the best one based on the `variant` selection rule.

```json
"stop_loss": {
  "type": "Optimizer",
  "variant": "Nearest",
  "stop_losses": [
    {
      "type": "NextOrder",
      "direction": "BOTH",
      "only_great": false,
      "activates": [],
      "filters": [],
      "order": {
        "price": { "type": "Position", "price": { "type": "Percentage", "value": -2.0 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }
    },
    {
      "type": "NextOrder",
      "direction": "BOTH",
      "only_great": false,
      "activates": [],
      "filters": [],
      "order": {
        "price": { "type": "Indicator",
          "source": { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                      "indicator": { "type": "Mrc", "period": "200", "property": "DownBig" } },
          "price": { "type": "Percentage", "value": -0.5 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `variant` | `"Nearest"` / `"Futher"` | Selection rule — `Nearest` picks the SL closest to current price (tightest), `Futher` picks the farthest (widest) |
| `stop_losses` | array of SL entries | Each entry is a full `TradeSettingProSL` (`NextOrder` or nested `Optimizer`) |

> **Spelling note:** the backend uses `"Futher"` (not `"Further"`). JSON must use this exact spelling.

**Use when:** you want to combine a fixed % SL with an indicator-based SL and pick whichever is tighter (Nearest) or more lenient (Futher) at runtime.

---

## TP `Optimizer` — best-of-multiple take profits

Evaluates multiple TP definitions and picks the best one.

```json
"take_profit": {
  "type": "Optimizer",
  "take_profits": [
    {
      "type": "NextOrder",
      "direction": "BOTH",
      "order": {
        "price": { "type": "Position", "price": { "type": "Percentage", "value": 3.0 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      },
      "filters": []
    },
    {
      "type": "NextOrder",
      "direction": "BOTH",
      "order": {
        "price": { "type": "Indicator",
          "source": { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                      "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } },
          "price": { "type": "Percentage", "value": -0.5 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      },
      "filters": []
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `take_profits` | array of TP entries | Each entry is a full `TradeSettingProTP` (`NextOrder` or nested `Optimizer`) |

> TP Optimizer has no `variant` field — it evaluates all entries and selects the optimal one.

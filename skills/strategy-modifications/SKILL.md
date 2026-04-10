---
name: strategy-modifications
description: LivTorgEx modifications reference — how to configure position-level modifications (currently Grid). Use when building strategies that need per-position grid order management.
---

# LivTorgEx — Modifications Reference

`professional.modifications` is an array of modification entries. When a position is initialised (via `enter_price: Force`, `ForceStartPosition`, or any other entry path), each entry is tested in order. The **first matching entry** is applied to the position for its lifetime.

```json
"modifications": [
  {
    "filters": [ /* optional spawn-time conditions */ ],
    "setting": { /* modification config */ }
  }
]
```

- `filters` — optional AND-list of conditions evaluated at position open time. If all pass, this entry is selected.
- `setting` — the modification to apply. Currently only `Grid` is supported.

If no entry matches, the position runs without a modification.

---

## Grid modification — `"type": "Grid"`

A grid fills buy/sell orders across a price range. On each completed order fill, the grid automatically places the next level's order.

```json
{
  "filters": [],
  "setting": {
    "type": "Grid",
    "grid_type": { "type": "Arithmetic", "count": { "type": "Number", "value": 10 } },
    "from_price": { "type": "Price", "price": { "type": "Percentage", "value": -5.0 } },
    "to_price":   { "type": "Price", "price": { "type": "Percentage", "value":  5.0 } }
  }
}
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `grid_type` | ✅ | How level prices are spaced — see table below |
| `from_price` | ✅ | Lower or upper bound of the grid range (resolved at spawn) |
| `to_price` | ✅ | Other bound of the grid range (resolved at spawn) |
| `open_filters` | ❌ | Extra conditions checked before opening each level order |
| `close_filters` | ❌ | Extra conditions checked before closing each level order |

> **Price range direction:** `from_price` and `to_price` are always normalised to `(min, max)` at runtime — you don't need to know which direction the bot is entering. Symmetric offsets (e.g. `−5%` / `+5%`) work for both LONG and SHORT. The direction factor (`+1` for LONG, `−1` for SHORT) is applied when resolving `Percentage` price offsets, so a `−5%` `from_price` means "5% below entry" for LONG and "5% above entry" for SHORT.

---

### `grid_type` variants

#### `Static` — fixed percentage step

Grid levels are spaced by a fixed percentage step. The number of levels is derived from the price range ÷ step.

```json
"grid_type": {
  "type": "Static",
  "step": { "type": "Number", "value": 1.0 }
}
```

- `step` — percentage distance between consecutive levels (e.g. `1.0` = 1%)
- Level count = `floor((max_price − min_price) / (min_price × step / 100))`

**Use when:** you want consistent percentage spacing regardless of absolute price (good for volatile assets).

#### `Arithmetic` — equal absolute spacing

Grid levels are spaced equally in absolute price terms. The number of levels is explicit.

```json
"grid_type": {
  "type": "Arithmetic",
  "count": { "type": "Number", "value": 10 }
}
```

- `count` — total number of grid levels (must be ≥ 1)
- `step = (max_price − min_price) / count`

**Use when:** you want equal dollar spacing between levels (standard for most grid strategies).

#### `Geometric` — equal ratio spacing

Grid levels are spaced by an equal percentage ratio between every level (exponential spacing). The number of levels is explicit.

```json
"grid_type": {
  "type": "Geometric",
  "count": { "type": "Number", "value": 10 }
}
```

- `count` — total number of grid levels (must be ≥ 1)
- Ratio = `(max_price / min_price) ^ (1 / count)` — same percentage change between every level

**Use when:** you want equal percentage returns per level regardless of price (tighter spacing near lower prices).

---

### `from_price` and `to_price` — price range types

Both fields use `TradeSettingProPriceDepends`. Only a subset of types are valid here (grid prices are resolved at spawn, not per-candle):

| Type | Supported | Usage |
|------|-----------|-------|
| `Price` with `Percentage` offset | ✅ | `{ "type": "Price", "price": { "type": "Percentage", "value": -5.0 } }` — % offset from current market price |
| `Price` with `Absolute` offset | ✅ | `{ "type": "Price", "price": { "type": "Absolute", "value": 500.0 } }` — fixed price offset |
| `Indicator` | ✅ | Use indicator value as anchor with optional offset |
| `Variable` | ✅ | Use a named variable value as anchor |
| `Signal`, `FirstPrice`, `Position`, `LastOrder` | ❌ | Not supported for grid price resolution |

**Most common pattern** — symmetric % range around current price:
```json
"from_price": { "type": "Price", "price": { "type": "Percentage", "value": -5.0 } },
"to_price":   { "type": "Price", "price": { "type": "Percentage", "value":  5.0 } }
```

**Indicator-anchored range:**
```json
"from_price": { "type": "Indicator",
  "source": { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
              "indicator": { "type": "Ema", "period": "200", "property": "Value" } },
  "price": { "type": "Percentage", "value": -3.0 } },
"to_price": { "type": "Indicator",
  "source": { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
              "indicator": { "type": "Ema", "period": "200", "property": "Value" } },
  "price": { "type": "Percentage", "value":  3.0 } }
```

---

### How grid orders work at runtime

1. At `ForceStartPosition`, the grid is initialised from the matching modification entry.
2. Level prices are computed and stored in the position state.
3. The current price index (`idx`) is calculated — levels above (for LONG) or below (for SHORT) `idx` get open orders placed.
4. On each completed fill (`on_order_changes`), the grid automatically places the next level's order (up to `visible_size = 4` pending orders at a time).
5. `open_filters` / `close_filters` are evaluated per level before each order is placed.

**Quantity per level:** each level order uses `qty = amount / sum(all level prices)` — so the total capital deployed across all levels equals the position's `enter_amount`.

---

### Grid modification with filters (selective activation)

Use `filters` to select different grid configurations based on market conditions:

```json
"modifications": [
  {
    "filters": [
      { "type": "Operation", "operation": ">",
        "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                   "indicator": { "type": "Natr", "period": "14", "property": "Value" } },
        "right": { "type": "Number", "value": 0.5 } }
    ],
    "setting": {
      "type": "Grid",
      "grid_type": { "type": "Arithmetic", "count": { "type": "Number", "value": 20 } },
      "from_price": { "type": "Price", "price": { "type": "Percentage", "value": -8.0 } },
      "to_price":   { "type": "Price", "price": { "type": "Percentage", "value":  8.0 } }
    }
  },
  {
    "filters": [],
    "setting": {
      "type": "Grid",
      "grid_type": { "type": "Arithmetic", "count": { "type": "Number", "value": 10 } },
      "from_price": { "type": "Price", "price": { "type": "Percentage", "value": -4.0 } },
      "to_price":   { "type": "Price", "price": { "type": "Percentage", "value":  4.0 } }
    }
  }
]
```

> High-volatility (NATR > 0.5%): wide 20-level grid. Default: narrow 10-level grid.

---

### Full arithmetic grid example (`direction: "Both"`, single bot)

```json
"modifications": [
  {
    "filters": [],
    "setting": {
      "type": "Grid",
      "grid_type": {
        "type": "Arithmetic",
        "count": { "type": "Number", "value": 10 }
      },
      "from_price": {
        "type": "Price",
        "price": { "type": "Percentage", "value": -5.0 }
      },
      "to_price": {
        "type": "Price",
        "price": { "type": "Percentage", "value": 5.0 }
      }
    }
  }
]
```

Use `enter_price: Force` to open both LONG and SHORT positions immediately on spawn — no `on_analysis` actions needed. Alternatively, use `enter_price: Wait` with `ForceStartPosition` in `on_analysis` for conditional entry (see `strategy-on-actions` skill).

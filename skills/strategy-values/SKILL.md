---
name: strategy-values
description: LivTorgEx value type reference. Complete guide to every value type usable in filters, conditions, and actions — with JSON syntax, fields, return type, and usage notes.
---

# LivTorgEx — Value Types Reference

Values are used everywhere: in condition `left`/`right` sides, `SetVariable` actions, `ForceStartPosition` amounts, and TP/SL price calculations. Every value resolves to one of: **Float**, **Direction**, **String**, **Chart**, or **OrderType**.

---

## Number

A literal floating-point constant.

```json
{ "type": "Number", "value": 30.0 }
```

Returns: **Float**

---

## String

A literal string constant. Used mainly as `mark` label for orders.

```json
{ "type": "String", "value": "tp_order" }
```

Returns: **String**

---

## Direction

A hardcoded direction constant.

```json
{ "type": "Direction", "value": "LONG" }
{ "type": "Direction", "value": "SHORT" }
{ "type": "Direction", "value": "BOTH" }
```

Returns: **Direction**

---

## OrderType

A hardcoded order type constant. Used in `SetOrderType` actions.

```json
{ "type": "OrderType", "value": "MARKET" }
{ "type": "OrderType", "value": "LIMIT" }
{ "type": "OrderType", "value": "STOP_MARKET" }
{ "type": "OrderType", "value": "STOP_LIMIT" }
```

Returns: **OrderType**

---

## Global

Bot-level state values — amount limits, price, direction, leverage, and PnL.

```json
{ "type": "Global", "value": "<field>" }
```

| Field | Returns | Description |
|-------|---------|-------------|
| `Price` | Float | Current market price of the symbol |
| `Direction` | Direction | Current bot direction (LONG/SHORT) |
| `DirectionOpposite` | Direction | Opposite of current bot direction |
| `EnterPrice` | Float | The bot's configured enter price (if set) |
| `Factor` | Float | `+1.0` for LONG, `-1.0` for SHORT |
| `Leverage` | Float | Bot leverage setting |
| `Margin` | Float | Bot margin setting (USDT) |
| `MaxAmount` | Float | Maximum allowed open amount (USDT) |
| `AutoMaxAmount` | Float | MaxAmount + TotalPnL (auto-compounding); falls back to MaxAmount+TotalPnL if not overridden |
| `Amount` | Float | Current total open amount across all positions |
| `TotalPnL` | Float | Cumulative realised PnL for this bot (USDT) |
| `PriceTick` | Float | Minimum price increment (tick size) for the symbol |
| `PriceTickPrc` | Float | Tick size expressed as % of current price |

---

## Variable

Reads a named bot variable. Returns `None` if the variable has never been set (use `IsEmpty` to check).

```json
{ "type": "Variable", "name": "var_key" }
```

- `name` — the variable `key` as declared in `professional.variables`

Returns: whatever type was stored (typically Float)

> Use `IsEmpty` condition to guard against unset variables:
> ```json
> { "type": "IsEmpty", "value": { "type": "Variable", "name": "sl_pct" } }
> ```

---

## Indicator

Reads a computed indicator value for the current symbol.

```json
{ "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
  "indicator": { "type": "Rsi", "period": "14", "property": "Value" } }
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `token` | no | `"Chart"` | Symbol token. Use `"Chart"` for the bot's own symbol |
| `timeframe` | yes | — | Candle timeframe in seconds (60, 300, 900, 1800, 3600, 14400) |
| `idx` | no | `0` | Candle index: 0 = current/latest, 1 = previous, etc. |
| `indicator` | yes | — | Indicator definition (see `strategy-indicators` skill) |

Returns: Float or Direction depending on the indicator property

---

## Position

Reads a value from an open position.

```json
{ "type": "Position", "side": { "type": "Direction", "value": "LONG" }, "value": "<field>" }
```

- `side` — which position to read. Can be any value resolving to Direction. Use `null` to read the first open position.
- `value` — field to read:

| Field | Returns | Description |
|-------|---------|-------------|
| `Amount` | Float | Open position size in USDT |
| `EntryPrice` | Float | Average entry price of the position |
| `Pnl` | Float | Unrealised PnL in USDT |
| `Fee` | Float | Accumulated fees (always positive) |
| `Direction` | Direction | Position direction |
| `DirectionOpposite` | Direction | Opposite of position direction |
| `Factor` | Float | `+1.0` for LONG, `-1.0` for SHORT |

> Returns `None` if no position exists on that side — condition will fail silently.

---

## Order

Reads information about a tracked order (placed via `CreateOrder` with a `mark`).

```json
{ "type": "Order",
  "mark":  { "type": "String", "value": "my_limit" },
  "pside": { "type": "Direction", "value": "LONG" },
  "value": "Exist" | "Price" }
```

| Field | Description |
|-------|-------------|
| `mark` | Any value resolving to String — the label used in `CreateOrder` |
| `pside` | Any value resolving to Direction — which position's order book to look in |
| `value` | `"Exist"` → Float (`1.0` = exists, `0.0` = not found). `"Price"` → Float (order price) |

---

## Math

Arithmetic on any two values. Supports nested expressions.

```json
{ "type": "Math",
  "value": {
    "type": "Operation",
    "operation": "+" | "-" | "*" | "/",
    "left":  { /* any value */ },
    "right": { /* any value */ }
  }
}
```

Returns: **Float** (returns `None` if either side is not a Float, or division by zero)

Examples:
```json
// Half of MRC band width
{ "type": "Math", "value": { "type": "Operation", "operation": "/",
  "left":  { "type": "PriceMeasure", ... },
  "right": { "type": "Number", "value": 2.0 } } }

// Negate a variable (make it negative for SL)
{ "type": "Math", "value": { "type": "Operation", "operation": "*",
  "left":  { "type": "Variable", "name": "tp_pct" },
  "right": { "type": "Number", "value": -0.5 } } }
```

---

## PriceMeasure

Computes the **percentage change** from `left` price to `right` price.

```
result = (right - left) / left * 100
```

```json
{ "type": "PriceMeasure",
  "left":  { /* any Float value — base price */ },
  "right": { /* any Float value — target price */ },
  "is_abs": false }
```

- `is_abs: true` — returns the absolute value (always positive, regardless of direction)
- Returns `None` if either value is not a Float

Returns: **Float** (%)

Examples:
```json
// Distance from DownInner to UpBig as % (positive when UpBig > DownInner)
{ "type": "PriceMeasure",
  "left":  { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
             "indicator": { "type": "Mrc", "period": "200", "property": "DownInner" } },
  "right": { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
             "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } },
  "is_abs": false }
```

---

## PercentageBetween

Computes the **normalised position** of `value` between `left` (0%) and `right` (100%).

```
result = (value - left) / (right - left)
```

```json
{ "type": "PercentageBetween",
  "left":  { /* any Float value — lower bound */ },
  "right": { /* any Float value — upper bound */ },
  "value": { /* any Float value — the probe */ },
  "is_abs": false }
```

- Result `0.0` = value is at `left`, `1.0` = value is at `right`, `0.5` = midpoint
- `is_abs: true` — returns absolute value
- Returns `None` if `left == right` (division by zero)

Returns: **Float** (ratio, not %)

Example — check price is in the lower 30% of the MRC channel:
```json
{ "type": "Operation", "operation": "<",
  "left": { "type": "PercentageBetween",
    "left":  { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
               "indicator": { "type": "Mrc", "period": "200", "property": "DownBig" } },
    "right": { "type": "Indicator", "token": "Chart", "timeframe": 60, "idx": 0,
               "indicator": { "type": "Mrc", "period": "200", "property": "UpBig" } },
    "value": { "type": "Global", "value": "Price" },
    "is_abs": false },
  "right": { "type": "Number", "value": 0.3 } }
```

---

## Pick

Conditional value selection — like a `switch` statement. Evaluates each branch's filters in order; returns the first matching branch's value. Falls back to `default` if no branch matches.

```json
{ "type": "Pick",
  "values": [
    { "filters": [ /* conditions */ ], "value": { /* value if filters pass */ } },
    { "filters": [ /* conditions */ ], "value": { /* value if filters pass */ } }
  ],
  "default": { /* value if no branch matched */ }
}
```

- `filters` within each branch are AND by default
- `default` is optional — returns `None` if omitted and no branch matches

Example — adaptive SL based on NTPS:
```json
{ "type": "Pick",
  "values": [
    { "filters": [{ "type": "Operation", "operation": ">",
        "left": { "type": "Analysis", "value": { "type": "Indicator", "property": "Ntps" } },
        "right": { "type": "Number", "value": 80.0 } }],
      "value": { "type": "Number", "value": -1.0 } },
    { "filters": [{ "type": "Operation", "operation": ">",
        "left": { "type": "Analysis", "value": { "type": "Indicator", "property": "Ntps" } },
        "right": { "type": "Number", "value": 50.0 } }],
      "value": { "type": "Number", "value": -1.5 } }
  ],
  "default": { "type": "Number", "value": -2.0 }
}
```

---

## Analysis

Real-time market data from the suggestion engine — updated every ~1s. Use in `on_analysis` filters. See `strategy-analysis` skill for full reference.

```json
{ "type": "Analysis", "value": { "type": "<subtype>", ... } }
```

### Analysis subtypes

#### Indicator — signal-level stats

```json
{ "type": "Analysis", "value": { "type": "Indicator", "property": "<field>" } }
```

| Property | Returns | Description |
|----------|---------|-------------|
| `Ntps` | Float | Net taker pressure score (0–100) — buy vs sell momentum |
| `NtpsFastTime` | Float | Short-window NTPS |
| `Trandm` | Float | Trend momentum score |
| `Price1h` | Float | Price change % over last 1h |
| `Price4h` | Float | Price change % over last 4h |
| `Price8h` | Float | Price change % over last 8h (uses 4h value internally) |
| `Price24h` | Float | Price change % over last 24h |
| `Asset01` | Float | Asset-level signal metric |
| `AssetDiff01` | Float | Asset-level signal difference metric |
| `Status` | Float | `1.0` = FastTrade mode, `0.0` = Normal |

#### Candle — current real-time candle (1s or signal update rate)

```json
{ "type": "Analysis", "value": { "type": "Candle", "property": "<field>" } }
```

| Property | Returns | Description |
|----------|---------|-------------|
| `Direction` | Direction | Candle direction (LONG if close > open) |
| `Open` | Float | Candle open price |
| `Close` | Float | Candle close price (current price) |
| `High` | Float | Candle high |
| `Low` | Float | Candle low |
| `Qty` | Float | Quote volume (USDT) |
| `Qtym` | Float | Quote volume — market orders only |
| `QtyAsset` | Float | Base asset volume |
| `QtymAsset` | Float | Base asset volume — market orders only |

#### FastCandle — last fast-trade aggregated candle

Same properties as `Candle`. Use when `Indicator.Status == 1.0` (FastTrade mode).

```json
{ "type": "Analysis", "value": { "type": "FastCandle", "property": "Close" } }
```

#### Kline — closed candle at a specific timeframe

```json
{ "type": "Analysis", "value": { "type": "Kline", "tf": 3600, "property": "<field>" } }
```

- `tf` — timeframe in seconds

| Property | Returns | Description |
|----------|---------|-------------|
| `Direction` | Direction | Candle direction (LONG if close > open) |
| `Open` | Float | Kline open |
| `Close` | Float | Kline close |
| `High` | Float | Kline high |
| `Low` | Float | Kline low |

#### OrderBook — level in the aggregated order book

```json
{ "type": "Analysis", "value": {
  "type": "OrderBookLevel",
  "level": 0,
  "direction": "Long" | "Short" | "Direction" | "DirectionOpposite",
  "property": "<field>"
} }
```

- `level` — 0 = best (tightest), 1 = next, etc.
- `direction` — which side of the book: `Direction` = bot's current direction, `DirectionOpposite` = other side

| Property | Returns | Description |
|----------|---------|-------------|
| `Price` | Float | Level price |
| `Qty` | Float | Level quantity |
| `QtyFilled` | Float | How much has been filled at this level |
| `Amount` | Float | `price × qty` |
| `TotalAmount` | Float | Cumulative amount from level 0 to this level |
| `Duration` | Float | Milliseconds since level appeared |

#### OrderBookInfo — aggregated book summary

```json
{ "type": "Analysis", "value": { "type": "OrderBookInfo", "property": "<field>" } }
```

| Property | Returns | Description |
|----------|---------|-------------|
| `BuyPrice` | Float | Best buy price |
| `BuyAmount` | Float | Total buy-side amount |
| `SellPrice` | Float | Best sell price |
| `SellAmount` | Float | Total sell-side amount |

#### Wave — trend wave analysis at a timeframe

```json
{ "type": "Analysis", "value": { "type": "Wave", "timeframe": 60, "property": "<field>" } }
```

| Property | Returns | Description |
|----------|---------|-------------|
| `Direction` | Direction | Current wave direction |
| `Qtym` | Float | Market order volume in this wave |
| `Percentile` | Float | Wave strength percentile vs history |
| `MinPrice` | Float | Lowest price in this wave |
| `MaxPrice` | Float | Highest price in this wave |
| `TimeFrames` | Float | Duration of wave in number of `timeframe` candles |
| `TimeFrameFract` | Float | Fractional part of `TimeFrames` |

#### Movement — structural price movement

```json
{ "type": "Analysis", "value": { "type": "Movement", "timeframe": 60, "property": "<field>" } }
```

| Property | Returns | Description |
|----------|---------|-------------|
| `Direction` | Direction | Movement direction |
| `Status` | Direction | Movement status as direction |
| `BreakPrice` | Float | Price at which movement broke out |
| `PriceStart` | Float | Movement starting price |
| `Min` | Float | Lowest price in movement |
| `Max` | Float | Highest price in movement |
| `Qtym` | Float | Market order volume during movement |
| `Activates` | Float | Number of price level activations |
| `TimeFrames` | Float | Movement duration in candles at `timeframe` |
| `WaitTimeFrames` | Float | Time since movement ended in candles |

---

## Chart / ChartPrice

Low-level candle position check — tests whether price is crossing, above, or below an indicator level at a specific candle. Rarely needed; prefer `Indicator` values for normal use.

```json
{ "type": "Chart",      "value": "Cross" | "Body" | "Shadow" | "Above" | "Below" | "No",
  "timeframe": 60, "idx": 0 }

{ "type": "ChartPrice", "value": "Cross" | "Body" | "Shadow" | "Above" | "Below" | "No",
  "timeframe": 60, "idx": 0 }
```

Returns: **Chart** (used in conditions, not comparable to Float)

| Value | Meaning |
|-------|---------|
| `Cross` | Price is crossing the level (includes Body and Shadow) |
| `Body` | Candle body is crossing |
| `Shadow` | Only wick is crossing |
| `Above` | Price is above the level |
| `Below` | Price is below the level |
| `No` | No interaction |

---

## Parameter

Reads a parameter passed to a manual `on_actions` action by the user. Only valid inside `on_actions` handlers.

```json
{ "type": "Parameter", "name": "param_key" }
```

Returns: whatever type the parameter holds

---

## Quick cheat sheet

| Use case | Value type |
|----------|-----------|
| Literal number | `Number` |
| Hardcoded direction | `Direction` |
| Current market price | `Global.Price` |
| Bot direction | `Global.Direction` |
| Bot PnL | `Global.TotalPnL` |
| Indicator value | `Indicator` |
| Open position PnL | `Position.Pnl` |
| Open position entry price | `Position.EntryPrice` |
| Bot variable | `Variable` |
| Order exists / price | `Order` |
| % change between prices | `PriceMeasure` |
| Normalised position in range | `PercentageBetween` |
| Arithmetic | `Math` |
| Conditional value | `Pick` |
| Real-time NTPS / candle / wave | `Analysis` |

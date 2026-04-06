---
name: strategy-analysis
description: Reference for "Analysis" values available to strategies (real-time projection data).
---

# LivTorgEx — Analysis value reference

This document lists Analysis value types (from SuggestionInfo / projection) available to strategy settings. Use these when you need real‑time chart, candle, movement, wave, or order book data inside filters, actions, or variables.

## When Analysis is available
- Populated from the projection SuggestionInfo (projection stream).
- Accessible in TradeSetting contexts when options.sug_info is present (bots subscribed to projection).

## Supported Analysis value types
- Indicator: sug_info.indicator fields (ntps, ntps_fast_time, trandm, price_1h/4h/8h/24h, asset_01, asset_diff_01).
  JSON: { "type": "Analysis", "value": { "type": "Indicator", "property": "Ntps" } }

- Candle / FastCandle: live candle fields (Open/Close/High/Low/Qty/Qtym/QtyAsset/QtymAsset, Direction).
  JSON: { "type": "Analysis", "value": { "type": "Candle", "property": "Close" } }

  FastCandle — what it represents and why use it in strategies
  - Purpose: a high-frequency aggregation of trades within a short window. Use when responding to rapid market microstructure events, immediate spikes, or ultra-fast signals where standard candle timeframes (1m+) are too coarse.
  - When to use in strategy:
    - Quick entries/exits on sudden momentum or volume spikes.
    - Volume-based filters that should react faster than regular candle updates.
    - Microstructure checks (did price stay inside a tight range, or did a fast impulse break it?).
    - Guarding limit orders: check if current price sits within the fast candle range before placing a near-market limit.
  - Availability: may be absent for normal ticks; always guard against missing fast data and fall back to the regular Candle value.

  FastCandle property meanings (how to interpret them in strategy logic):
  - enter: opening price of the fast aggregation (acts like 'open'). Use to compute immediate momentum relative to the start of the fast window.
  - exit: latest price / close of the fast aggregation (acts like 'close'). Use for decision thresholds and short-term direction.
  - min / max: observed min and max prices during the fast window. Use to detect breakouts, containment, or to place limit orders safely inside/outside the range.
  - qty: total trade quantity (count or normalized volume measure) in the fast window. Useful for detecting raw activity increases.
  - qty_asset: quantity measured in the asset units. Use when strategy sizing depends on asset volume rather than notional.
  - qtym: monetary-volume (notional) in the fast window (useful to detect large-money activity regardless of asset units).
  - qtym_asset: monetary volume expressed or normalized per-asset unit (helpful when mixing symbol denominations).

  Useful strategy patterns with FastCandle:
  - Volume spike entry: if FastCandle.qtym > X and FastCandle.get_direction() == LONG, consider a market/force entry.
  - Micro breakout: if price > FastCandle.max (break above fast window), trigger aggressive entry or widen stop.
  - Limit safety: only place a near-market limit if FastCandle.include_price(price) == true (ensures the limit will not immediately fill).
  - Volatility filter: use FastCandle.get_prc() or computed (exit-enter) magnitude to avoid trading during noisy micro-churn.

  JSON examples (guard for absence):
  - FastCandle close:
    { "type": "Analysis", "value": { "type": "FastCandle", "property": "Close" } }
  - Compare fast monetary volume:
    { "type": "Operation", "operation": ">", "left": { "type": "Analysis", "value": { "type": "FastCandle", "property": "Qtym" } }, "right": { "type": "Number", "value": 1000.0 } }

  Recommendation: use FastCandle when strategy correctness or safety depends on immediate, high-frequency context. Always provide a fallback path to regular Candle or Indicator values when fast data is missing.

- Kline (closed TF kline): use tf (seconds) and property (Open/Close/High/Low/Direction).
  JSON: { "type": "Analysis", "value": { "type": "Kline", "tf": 3600, "property": "Close" } }

- Wave: timeframe-specific wave stats (Direction, Qtym, Percentile, MinPrice, MaxPrice, TimeFrames).
  JSON: { "type": "Analysis", "value": { "type": "Wave", "timeframe": 3600, "property": "Percentile" } }

- Movement: timeframe-specific movement data (Direction, BreakPrice, Min/Max, PriceStart, Qtym, Activates, Status, etc.).
  JSON: { "type": "Analysis", "value": { "type": "Movement", "timeframe": 900, "property": "BreakPrice" } }

- OrderBook / OrderBookLevel / OrderBookInfo: book top prices, levels, quantities, durations.
  JSON: { "type": "Analysis", "value": { "type": "OrderBookInfo", "property": "SellPrice" } }

## Indicator Vec / detailed indicators
- Detailed indicators live in TFIndicatorVecValues: HashMap<timeframe_seconds, VecDeque<Arc<HashMap<indicator_name, IndicatorMapValues>>>>.
  - VecDeque index semantics: idx = 0 current/updating bar, 1 previous closed, etc.
  - IndicatorMapValue may be Float, Direction (LONG/SHORT/BOTH), or Cross (Cross/Body/Above/Below/No).

## Important notes
- Timeframes are seconds (60, 300, 900, 1800, 3600, 14400).
- Analysis getters return TradeSettingProValueResult variants: Float, Direction, Chart (Cross), or String.
- Use Candle vs FastCandle depending on whether high-frequency aggregation (fast) is required.
- Analysis values require options.sug_info (projection SuggestionInfo). If missing, the value returns None.

## Example combined usage
- Check NTPS and current candle close price:

{
  "type": "Expression",
  "expression": "And",
  "values": [
    { "type": "Operation", "operation": ">", "left": { "type": "Analysis", "value": { "type": "Indicator", "property": "Ntps" } }, "right": { "type": "Number", "value": 50.0 } },
    { "type": "Operation", "operation": ">", "left": { "type": "Analysis", "value": { "type": "Candle", "property": "Close" } }, "right": { "type": "Number", "value": 100.0 } }
  ]
}

---

Reference compiled from trading-platform-rust stream_schema projection and strategy modules (SuggestionInfo, TFIndicatorVecValues, TradeSettingProValueAnalysis).

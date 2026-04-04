---
name: strategy-indicators
description: LivTorgEx indicator reference. Use when building strategy filters or conditions and you need the correct indicator type, properties, or period format.
---

# LivTorgEx — Indicator Reference

## Indicator value wrapper

```json
{
  "type": "Indicator",
  "token": "Chart",
  "timeframe": 3600,
  "idx": 0,
  "indicator": { /* indicator object below */ }
}
```

- `timeframe` — must be one of `60 / 300 / 900 / 1800 / 3600 / 14400` (no other values)
- `idx` — `0` = current candle, `1` = previous, etc.
- `modify` — optional integer: `0` = Auto (default, can omit), `1` = No, `2` = Long, `3` = Short

`Direction` output values: `"LONG"` (bullish) · `"SHORT"` (bearish) · `"BOTH"` (neutral)

---

## Quick reference

| Type | Has period | Key properties |
|------|-----------|----------------|
| `Rsi` | `"14"` | `Value` 0–100 |
| `Stoch` | `"k,smooth_k,d"` e.g. `"14,3,3"` | `K`, `D` 0–100 |
| `Supertrend` | no | `Direction`, `KLines`, `Supertrend` |
| `Ema` | `"50"` | `Direction`, `Value` |
| `BollingerBands` | `"20"` | `Average`, `Upper`, `Lower` |
| `Smi` | `"k,d"` e.g. `"25,3"` | `Direction`, `Smi` −100…100, `SmiBasedEma`, `Cross`, `CrossIndex` |
| `Mfi` | `"14"` | `Value` 0–100 |
| `Cci` | `"20"` | `Value` −200…+200 |
| `Lsma` | `"20"` | `Direction`, `Cross`, `Value` |
| `ATRTralling` | no | `Direction`, `KLines` |
| `Natr` | `"14"` | `Value` %, `Atr`, `Gain`, `Drop` |
| `Psar` | no | `Direction`, `Distance`, `Price`, `NextSar`, `Ep`, `KLines` |
| `EmaCross` | `"fast,slow"` e.g. `"12,26"` | `Direction`, `CrossPrice`, `PCP`, `KLines` |
| `ChandelierExit` | no | `Direction`, `Long`, `Short` |
| `Mrc` | `"20"` | `Direction`, `MeanLine`, `Up/DownInner/Small/Outer/Big`, `CurrentCross` −4…+4, `PrevCross` |
| `Imbalance` | no | `Direction`, `High`, `Low`, `KLines`, `Fill`, `HighFill` |
| `Window` | no | `Buy/Sell/CrossDirection/Price/Cross/KLines/Qtym`, `CrossLineDirection`, `CrossTimes` |
| `Wave` | no | `Direction`, `Qtym`, `TotalQtym`, `Klines`, `CrossKlines`, `Percentile`, `Min`, `Max`, `OP`, `CP` |
| `Volume` | `"20"` | `Direction`, `Gain/Min/Max`, `BuySellGain/Min/Max` |
| `Smc` | no | `Direction`, `DirectionSwing`, `BreakPrice`, `HighPrice`, `LowPrice`, `OBDirection`, `OBHigh/Low`, `SBP`, `FP` |
| `DPSignal` | no | `Direction`, `Zee/1/2`, `ZeeKLines/1/2` |
| `Ntps` | no | `Value`, `Min`, `Max` |
| `Candle` | no | `Direction`, `Open`, `Close`, `High`, `Low`, `BodyChange` |
| `ZigZagTrend` | no | `Direction`, `PrevDirection`, `EndDirection`, `KLines`, `StartPrice`, `FollowPrice`, `EndPrice`, `LastPrice`, `BreakPrice`, `Fibo`, `StartFibo`, `BuyPrice`, `SellPrice` |

---

## Detailed descriptions

### Rsi
```json
{ "type": "Rsi", "period": "14", "property": "Value" }
```
Momentum oscillator. Overbought >70, oversold <30. Typical periods: `7`, `14`, `21`.

### Stoch
```json
{ "type": "Stoch", "period": "14,3,3", "property": "K" }
```
Compares close to range. K/D crossovers. Overbought >80, oversold <20.

### Supertrend
```json
{ "type": "Supertrend", "property": "Direction" }
```
ATR-based trend follower. `KLines` = bars in current trend. No period or multiplier field — configured server-side.

### Ema
```json
{ "type": "Ema", "period": "50", "property": "Direction" }
```
Weighted MA. `Direction` = LONG when price > EMA. Typical: `9`, `20`, `50`, `100`, `200`.

### BollingerBands
```json
{ "type": "BollingerBands", "period": "20", "property": "Upper" }
```
SMA ± 2σ. Use `Upper`/`Lower` as price reference for breakouts or mean reversion.

### Smi
```json
{ "type": "Smi", "period": "25,3", "property": "Smi" }
```
Centered stochastic. `Smi` > 40 = overbought, < −40 = oversold. `Cross` fires on K/D crossover.

### Mfi
```json
{ "type": "Mfi", "period": "14", "property": "Value" }
```
Volume-weighted RSI. Overbought >80, oversold <20.

### Cci
```json
{ "type": "Cci", "period": "20", "property": "Value" }
```
Deviation from mean. Overbought >100, oversold <−100.

### Lsma
```json
{ "type": "Lsma", "period": "20", "property": "Direction" }
```
Linear regression MA. Lower lag than EMA. `Cross` fires when price crosses LSMA.

### ATRTralling
```json
{ "type": "ATRTralling", "property": "Direction" }
```
ATR trailing stop. `KLines` = bars since last flip. No period or multiplier field.

### Natr
```json
{ "type": "Natr", "period": "14", "property": "Value" }
```
ATR as % of price. Use `Value` as a volatility filter (e.g. only enter when NATR > 0.5%).

### Psar
```json
{ "type": "Psar", "property": "Direction" }
```
Accelerating dot stop. `Distance` = gap between price and SAR. No period.

### EmaCross
```json
{ "type": "EmaCross", "period": "12,26", "property": "Direction" }
```
Golden/death cross using fast,slow period pair. `KLines` = bars since last cross. Common pairs: `9,21`, `12,26`, `50,200`.

### ChandelierExit
```json
{ "type": "ChandelierExit", "property": "Direction" }
```
ATR trail from highest high / lowest low. `Long`/`Short` = stop price levels. No period or offset field.

### Mrc
```json
{ "type": "Mrc", "period": "20", "property": "CurrentCross" }
```
Multi-band channel. `CurrentCross` ∈ {−4…+4}: 0 = mean, +4 = extreme upper, −4 = extreme lower.

### Imbalance
```json
{ "type": "Imbalance", "property": "Direction" }
```
Fair Value Gap / imbalance zone. `Fill` > 0 means partially filled. No period.

### Window
```json
{ "type": "Window", "property": "BuyDirection" }
```
Buy/sell structure window. Use `CrossDirection` for net imbalance, `CrossTimes` for activity count.

### Wave
```json
{ "type": "Wave", "property": "Direction" }
```
Market wave tracker. `Percentile` = wave strength. `TotalQtym` accumulates across the wave.

### Volume
```json
{ "type": "Volume", "period": "20", "property": "BuySellGain" }
```
Volume analysis. `BuySellGain` > 1 = buyers dominating. `Gain` vs period average.

### Smc
```json
{ "type": "Smc", "property": "OBDirection" }
```
Smart Money. `OBHigh`/`OBLow` = order block boundaries. `FP` = fair price mid-point.

### DPSignal
```json
{ "type": "DPSignal", "property": "Direction" }
```
Pivot hierarchy. `Zee` = primary level, `Zee1`/`Zee2` = secondary/tertiary.

### Ntps
```json
{ "type": "Ntps", "property": "Value" }
```
Normalized price setup score. Use with `Min`/`Max` for range-relative positioning.

### Candle
```json
{ "type": "Candle", "property": "Direction" }
```
Raw candle data. Use `idx: 1` for the previous candle. `BodyChange` is directional (positive = bullish).

### ZigZagTrend
```json
{ "type": "ZigZagTrend", "property": "Direction" }
```
Structured swing tracker. `Fibo` = current retracement. `KLines` = bars in current leg.

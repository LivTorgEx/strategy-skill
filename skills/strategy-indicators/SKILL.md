---
name: strategy-indicators
description: LivTorgEx indicator reference. Use when building strategy filters or conditions and you need the correct indicator type, properties, or period format.
---

# LivTorgEx — Indicator Reference

Call this endpoint to get all available indicators with their valid `type`, available `period` values, and `property` values. No query parameters.

```
GET /api/indicators
Authorization: Bearer <skill-token>
```

### Response schema

```json
[
  {
    "name": "Ema",
    "period": ["9", "20", "21", "26", "200"],
    "properties": ["Direction", "Value"]
  },
  {
    "name": "Psar",
    "properties": ["Direction", "Distance", "Price", "NextSar", "Ep", "KLines"]
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Exact value for the `"type"` field in an indicator object |
| `period` | `string[]` | All valid period values for this indicator; absent when no period |
| `properties` | `string[]` | All valid values for the `"property"` field |

- Indicators without a `period` field in the response do **not** accept a `period` in the indicator object.
- When `period` is present, use one of the listed values exactly as the `period` string. The first value is the default.

Always call this endpoint before building or validating any indicator usage — it is the canonical, auto-synced source of truth.


---

## Indicator value wrapper

```json
{
  "type": "Indicator",
  "token": "Chart",
  "timeframe": 3600,
  "idx": 0,
  "indicator": { /* indicator object */ }
}
```

- `timeframe` — must be one of `60 / 300 / 900 / 1800 / 3600 / 14400`
- `idx` — `0` = current candle, `1` = previous, etc.
- `modify` — optional integer: `0` = Auto (default, can omit), `1` = No, `2` = Long, `3` = Short

`Direction` output values: `"LONG"` (bullish) · `"SHORT"` (bearish) · `"BOTH"` (neutral)


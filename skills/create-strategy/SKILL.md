---
name: create-strategy
description: Design a LivTorgEx trading strategy and deploy it as a bot group. Use when the user asks to create, update, or deploy a trading strategy or bot group.
---

# LivTorgEx — Create / Update Bot Group Strategy

## Environment variables

| Variable | Description |
|----------|-------------|
| `LIVTORGEX_SKILL_URL` | Skill API base URL, e.g. `https://skill.api.livtorgex.com` |
| `LIVTORGEX_SKILL_TOKEN` | Personal access token (`lt_<...>`) — get from `/skill/connect` |

---

## Workflow

### Step 1 — Fetch account context

```bash
curl -s "$LIVTORGEX_SKILL_URL/api/account/api_keys" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN"

curl -s "$LIVTORGEX_SKILL_URL/api/bot_groups" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN"
```

Present API keys by name. Warn if target bot group has `skill_access: "Read"` or `"Deny"`.

### Step 2 — Clarify

Ask for: bot group name, API key (from list), indicators + timeframe, direction, TP/SL %, max_open_amount, leverage, margin, symbol IDs, max active bots.

### Step 3 — Build JSON

Construct a valid `BotGroupSetting`. For indicator details use `strategy-indicators`. For condition syntax use `strategy-conditions`. For event handler patterns use `strategy-on-actions`.

### Step 4 — Validate



Fix all `errors`. `warnings` are non-blocking.

### Step 5 — Deploy

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/bot_group" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '<FULL_FORM_JSON>'
```

`"created": true` = new group. `"created": false` = updated.

### Step 6 — Run backtest (optional feedback)

After deploying, offer the user a backtest to validate the strategy on historical data.
Ask for: `start_time` (e.g. 30 days ago), `end_time` (now or leave blank for open-ended).

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/bot_groups/<BOT_GROUP_ID>/run_backtest" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_time": "2025-01-01 00:00:00",
    "end_time": "2025-03-31 00:00:00"
  }'
```

The response contains `backtest_id` and `version_id`. The user can review the backtest run in the UI at:
`/backtests/<backtest_id>/versions/<version_id>`

- Uses the bot group's **current** settings, symbols, margin, and leverage automatically.
- A linked backtest container is created once per bot group and reused on subsequent calls.
- Requires `skill_access` = "Edit".

---

## skill_access

Each bot group has `skill_access`: `"Edit"` (can deploy), `"Read"` (list only), `"Deny"` (blocked). HTTP 403 → user must set it to `"Edit"` in **Account → Skill → Bot Group Access**.

---

## STRICT TIMEFRAMES

Only these 6 values for `timeframe` in indicators (seconds):

| 1m | 5m | 15m | 30m | 1h | 4h |
|----|----|----|-----|----|-----|
| `60` | `300` | `900` | `1800` | `3600` | `14400` |

No daily. `min_tf` in signal definitions is in **minutes**.

---

## Type rules

| Field | Type | Example |
|-------|------|---------|
| `timeframe` | int seconds (strict list) | `3600` |
| `period` | **string** | `"14"`, `"10,3,3"` |
| `modify` | int (0=Auto, omit for default) | `0` |
| TP/SL percentage | float (+ = profit, − = loss) | `3.0`, `-1.5` |
| `min_tf` in signal | int **seconds** | `60` = 1m |
| `Direction` values | uppercase string | `"LONG"`, `"SHORT"`, `"BOTH"` |

---

## BotGroupSetting structure

```json
{
  "margin_mode": "Isolated" | "Cross",
  "margin_leverage": 20,
  "signal": { "name": "Indicator", "min_tf": 60 },
  "strategy": {
    "type": "Trading",
    "name": "Trading",
    "max_open_amount": 500.0,
    "direction": "Long" | "Short" | "Both",
    "professional": {
      "type": "Professional",
      "variables": [],
      "enter_price":     { "type": "Force" },
      "enter_amount":    { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
      "enter_direction": { "type": "Default" },
      "filters": [ /* search-mode gate — checked before starting a new bot; AND by default */ ],
      "take_profit": { "type": "NextOrder", "order": {
        "type":   "Market",
        "price":  { "type": "Price", "price": { "type": "Percentage", "value": 3.0 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }},
      "stop_loss": { "type": "NextOrder", "order": {
        "type":   "Market",
        "price":  { "type": "Price", "price": { "type": "Percentage", "value": -1.5 } },
        "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
      }},
      "on_analysis":   [ /* { "type": "Action", "filters": [], "action": {} } — evaluated every ~1s */ ],
      "on_indicators": [ /* { "timeframe": int, "filters": [], "actions": [{ "type": "Action", ... }] } */ ],
      "on_actions":    [ /* manual user triggers only — omit for auto trading */ ]
    }
  }
}
```

### `professional.filters` — bot spawn gate

**Checked exactly once when the bot group decides to spawn a new bot** (on each `min_tf` tick). If all filters pass → a new bot is spawned. Filters are NOT re-evaluated after the bot is running. Use for quality gates: NTPS, NATR, MRC direction, distance checks. All items are AND by default.

> **`enter_price` controls what happens immediately after spawn:**
> - `Force` → the spawned bot opens a position at market **immediately** (no `on_analysis`/`on_indicators` needed for entry)
> - `Wait` → the spawned bot waits; **you must call `ForceStartPosition` in `on_analysis` or `on_indicators`** to enter
>
> **Important:** `professional.filters` do NOT prevent re-spawning if conditions stay true for multiple `min_tf` ticks. Use `PrevCross`/`CurrentCross` guards or similar one-tick conditions in filters to ensure a single spawn per event.

**`on_analysis`** is the ~1s strategy loop — put entry conditions (when using `Wait`) and exit logic here as `[{ "type": "Action", filters, action }]`.
**`on_indicators`** fires on candle close — `[{ timeframe, filters, actions: [{ "type": "Action", ... }] }]`.
**`on_finished`** fires when a position closes — same item shape as `on_analysis`: `[{ "type": "Action", filters, action }]`.
**`on_actions`** is for manual UI triggers only — omit unless explicitly required.
`take_profit`/`stop_loss` orders require `"type": "Market"` on the inner `order` object.

### `enter_price` types

| Type | Behaviour after bot spawn |
|------|--------------------------|
| `Force` | Opens position at market price immediately on spawn — no action needed |
| `Wait` | Spawns bot in waiting state — call `ForceStartPosition` from `on_analysis` or `on_indicators` |
| `Signal` | Enters at the signal's suggested price |
| `Indicator` | Enters at a computed indicator price |

### `enter_direction` types

| Type | Description |
|------|-------------|
| `Default` | Use strategy `direction` |
| `Signal` | Follow signal direction |
| `Indicator` | Read from indicator |
| `Value` | Hardcoded direction |

---

## Signal types

The `signal` field controls how frequently the bot group rechecks `professional.filters` to start a new bot. `min_tf` is in **seconds** — use `min_tf: 60` to recheck every 1 minute.

| Name | Extra fields | Notes |
|------|-------------|-------|
| `Indicator` | `min_tf: int (seconds)` | **Default choice.** Rechecks filters every `min_tf` seconds. Use `min_tf: 60` for 1m recheck. |
| `FastTrade` | — | — |
| `OverTrend` | `min_arrange`, `max_arrange`, `min_4h_wave_prc` | — |
| `ATH4H` | `min_4h_wave_prc`, `reason: "Always"\|"OnlyLine"\|"LineOrMovement"` | — |
| `OrderBigSize` | `min_volume_asset`, `min_duration_minutes` | — |
| `CrossLine` | `timeframe: int (seconds)`, `min_mins: int` | — |
| `QtymSpike` | — | — |
| `Action` | `code: string` | — |
| `External` | `signal_id: int` | External signal source; does not drive filter recheck frequency. |

---

## Full example (RSI + Supertrend Long, 1h)

```json
{
  "name": "RSI Supertrend Long",
  "api_key_id": 1,
  "margin_mode": "Isolated",
  "margin_leverage": 20,
  "margin": 100.0,
  "max_active_bots": 3,
  "symbol_ids": [1],
  "settings": {
    "margin_mode": "Isolated",
    "margin_leverage": 20,
    "signal": { "name": "Indicator", "min_tf": 60 },
    "strategy": {
      "type": "Trading",
      "name": "Trading",
      "max_open_amount": 500.0,
      "direction": "Long",
      "professional": {
        "type": "Professional",
        "variables": [],
        "enter_price":     { "type": "Force" },
        "enter_amount":    { "type": "Percentage", "source": "MaxAmount", "value": 100.0 },
        "enter_direction": { "type": "Default" },
        "filters": [],
        "take_profit": { "type": "NextOrder", "order": {
          "type":   "Market",
          "price":  { "type": "Price", "price": { "type": "Percentage", "value": 3.0 } },
          "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
        }},
        "stop_loss": { "type": "NextOrder", "order": {
          "type":   "Market",
          "price":  { "type": "Price", "price": { "type": "Percentage", "value": -1.5 } },
          "amount": { "type": "Percentage", "source": "FirstOrder", "value": 100.0 }
        }},
        "on_analysis": [
          {
            "type": "Action",
            "filters": [
              {
                "type": "Operation", "operation": "<",
                "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                           "indicator": { "type": "Rsi", "period": "14", "property": "Value" } },
                "right": { "type": "Number", "value": 30.0 }
              },
              {
                "type": "Operation", "operation": "==",
                "left":  { "type": "Indicator", "token": "Chart", "timeframe": 3600, "idx": 0,
                           "indicator": { "type": "Supertrend", "property": "Direction" } },
                "right": { "type": "Direction", "value": "LONG" }
              }
            ],
            "action": {
              "type": "ForceStartPosition",
              "side": { "type": "Direction", "value": "LONG" },
              "msg": "RSI oversold + Supertrend Long"
            }
          }
        ]
      }
    }
  }
}
```

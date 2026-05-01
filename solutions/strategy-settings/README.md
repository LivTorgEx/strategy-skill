# Strategy Solution 1: Simple Hedge Sample

This is a minimal sample strategy settings payload for:

- Hedge mode (`BOTH` direction)
- Take profit: `1%`
- Stop loss: `2%`
- Continuous trading (infinite re-entry by signal)

## Environment

```bash
export LIVTORGEX_SKILL_URL="https://skill.api.livtorgex.com"
export LIVTORGEX_SKILL_TOKEN="lt_..."
```

## Sample FULL_FORM_JSON

Replace `api_key_id` and `symbol_ids` before deploy.

```json
{
  "name": "Simple Hedge 1TP 2SL",
  "api_key_id": 1,
  "symbol_ids": [1],
  "margin_mode": "Isolated",
  "margin_leverage": 10,
  "margin": 100.0,
  "max_active_bots": 1,
  "settings": {
    "margin_mode": "Isolated",
    "margin_leverage": 10,
    "signal": {
      "name": "Indicator",
      "min_tf": 60
    },
    "strategy": {
      "type": "Trading",
      "name": "Trading",
      "max_open_amount": 500.0,
      "direction": "Both",
      "professional": {
        "type": "Professional",
        "variables": [],
        "enter_price": { "type": "Force" },
        "enter_amount": {
          "type": "Percentage",
          "source": "MaxAmount",
          "value": 100.0
        },
        "enter_direction": { "type": "Default" },
        "filters": [],
        "take_profit": {
          "type": "NextOrder",
          "order": {
            "type": "Market",
            "price": {
              "type": "Price",
              "price": { "type": "Percentage", "value": 1.0 }
            },
            "amount": {
              "type": "Percentage",
              "source": "FirstOrder",
              "value": 100.0
            }
          }
        },
        "stop_loss": {
          "type": "NextOrder",
          "order": {
            "type": "Market",
            "price": {
              "type": "Price",
              "price": { "type": "Percentage", "value": -2.0 }
            },
            "amount": {
              "type": "Percentage",
              "source": "FirstOrder",
              "value": 100.0
            }
          }
        },
        "on_analysis": [],
        "on_indicators": [],
        "on_actions": []
      }
    }
  }
}
```

## Validate

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/validate_bot_group" \
  -H "Content-Type: application/json" \
  -d '{"settings": <SETTINGS_JSON_ONLY>}'
```

## Deploy

```bash
curl -s -X POST "$LIVTORGEX_SKILL_URL/api/bot_group" \
  -H "Authorization: Bearer $LIVTORGEX_SKILL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '<FULL_FORM_JSON>'
```

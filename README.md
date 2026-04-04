# LivTorgEx Strategy Skill

A Claude Code plugin that lets your AI agent design and deploy trading bot group strategies directly to a LivTorgEx server.

## What it does

When you ask Claude to create a strategy, it will:

1. Fetch your API keys and existing bot groups from the server
2. Ask for any missing settings (indicators, timeframe, direction, TP/SL, leverage, etc.)
3. Generate a valid `BotGroupSetting` JSON
4. Validate it against the server
5. Create or update the bot group

## Installation

### Install from GitHub

```bash
claude plugin marketplace add LivTorgEx/strategy-skill
claude plugin install livtorgex-strategy-skill@LivTorgEx/strategy-skill
```

### Install from local path (if you cloned the repo)

```bash
claude plugin marketplace add /path/to/strategy-skill
claude plugin install livtorgex-strategy-skill@strategy-skill
```

### Team auto-install

Add to your project's `.claude/settings.json`:

```json
{
  "extraKnownMarketplaces": {
    "livtorgex": {
      "source": {
        "source": "github",
        "repo": "LivTorgEx/strategy-skill"
      }
    }
  },
  "enabledPlugins": {
    "livtorgex-strategy-skill@livtorgex": true
  }
}
```

## Setup

Set these environment variables (add to your shell profile or `.env`):

```bash
export LIVTORGEX_SKILL_URL="https://skill.api.livtorgex.com"
export LIVTORGEX_SKILL_TOKEN="lt_..."   # get from Account → Skill → Connect
```

## Usage

Just describe your strategy:

> "Create a long-only bot group called 'RSI Scalper' that enters when RSI(14) is below 30 and Supertrend is pointing up on the 1h chart. TP 3%, SL 1.5%, leverage 20x, margin 100 USDT."

Claude will build the JSON and deploy it.

## Skills included

| Skill | Description |
|-------|-------------|
| `create-strategy` | Full workflow: fetch context → build JSON → validate → deploy |
| `strategy-indicators` | All 24 indicator types with correct fields and properties |
| `strategy-conditions` | Filter, value, and action syntax reference |
| `strategy-on-actions` | `on_analysis`, `on_indicators`, `on_actions` patterns and variables |

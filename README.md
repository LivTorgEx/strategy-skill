# LivTorgEx Strategy Skills

A Claude Code plugin with guidance for classic bot groups and versioned
strategy-version legs. Rust WASM modules can be built locally, uploaded as
precompiled artifacts, and pinned to a leg through its QuantFlow program.

## Two solutions

This repository documents these strategy paths:

1. Classic bot-group settings JSON: [solutions/strategy-settings/README.md](solutions/strategy-settings/README.md)
2. Rust WASM modules pinned to versioned strategy legs: [solutions/strategy-rust-module/README.md](solutions/strategy-rust-module/README.md)

Repository layout:

- skills/ -> plugin skills used by Claude/Copilot
- solutions/strategy-settings/ -> settings-based strategy workflow
- solutions/strategy-rust-module/ -> Rust module scaffold and guide

## What it does

For a classic bot-group request, the skill will:

1. Fetch your API keys and existing bot groups from the server
2. Ask for any missing settings (indicators, timeframe, direction, TP/SL, leverage, etc.)
3. Generate a valid `BotGroupSetting` JSON
4. Validate it against the server
5. Create or update the bot group

For a versioned strategy leg, use the strategy-version MCP tools. The leg's
QuantFlow program declares each WASM module with `module.start("alias")`, and
its `modules` array binds that alias to a successful `module_build_id`.

## Installation

### GitHub Copilot CLI

Install from GitHub:
```bash
copilot plugin install LivTorgEx/strategy-skill
```

Install from local path:
```bash
copilot plugin install ~/dev/strategy-skill
```

Update / reinstall:
```bash
copilot plugin uninstall livtorgex-strategy-skill
copilot plugin install ~/dev/strategy-skill
```

> You can also ask Copilot directly: *"reinstall strategy-skill"* and it will run the commands above.

### Claude Code

Install from GitHub:
```bash
claude plugin marketplace add LivTorgEx/strategy-skill
claude plugin install livtorgex-strategy-skill@LivTorgEx/strategy-skill
```

Install from local path:
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
export LIVTORGEX_MCP_URL="https://skill.api.livtorgex.com"
export LIVTORGEX_MCP_TOKEN="lt_..."   # get from Account → Setup → AI Connections
```

## Usage

Just describe your strategy:

> "Create a long-only bot group called 'RSI Scalper' that enters when RSI(14) is below 30 and Supertrend is pointing up on the 1h chart. TP 3%, SL 1.5%, leverage 20x, margin 100 USDT."

Claude will build the JSON and deploy it.

## Skills included

| Skill | Description |
|-------|-------------|
| `create-strategy` | Classic bot-group workflow: fetch context → build JSON → validate → deploy |
| `strategy-rust-module` | Build/upload Rust WASM and bind a pinned build to a versioned strategy leg |
| `strategy-indicators` | All 24 indicator types with correct fields and properties |
| `strategy-conditions` | Filter, value, and action syntax reference |
| `strategy-on-actions` | `on_analysis`, `on_indicators`, `on_actions` patterns and variables |

---
name: create-strategy
description: Create, edit, backtest, and release strategy versions using direct QuantFlow bot programs and optional WASM modules.
---

# Create or update a strategy version

This guide uses the current strategy-version MCP API for existing owned strategies and their versions.

A strategy is an existing owned record. The MCP can list and edit owned strategies, create versions for them, and edit or release those versions. It does not expose a tool to create the strategy record itself. If the user has no existing strategy to use, ask them to select or create one in the product before continuing.

Strategy legs can run QuantFlow bot programs directly; WASM modules are optional. Start with the [working QuantFlow-only example](https://livtorgex.github.io/program-docs/bots/strategy-programs/#quantflow-only-example) and the [bot strategy program guide](https://livtorgex.github.io/program-docs/bots/strategy-programs/). This guide and the public links work from KaraCat without an MCP resource reader. For building and attaching a Rust WASM module, read the `strategy-rust-module` skill.

## Workflow

### 1. Clarify the requested version

Confirm the target strategy, version name, intended legs, each leg's provider and QuantFlow program, margin mode and leverage, and any bot actions, workflow signal actions, API-key requirements, or optional WASM modules. A QuantFlow-only leg needs no WASM build, module binding, or `module.start` declaration. Keep leg actions separate from version-level signal settings.

### 2. Inspect the owned strategy

Call `list_strategies`, then use `get_strategy` with the selected `strategy_id`. Use the returned names and IDs to identify the target and inspect its versions and legs. If editing an existing version or leg, confirm it is an unreleased pre-release; released versions and legs are immutable.

### 3. Create a version

Call `create_strategy_version` for an existing owned strategy. `strategy_id`, `name`, and `legs` are required. `parent_version_id`, `description`, and `signal_settings` are optional. Creating a version does not release it.

Example shape (replace placeholders with the user's actual configuration):

```json
{
  "strategy_id": 42,
  "name": "Strategy revision",
  "description": "Optional description",
  "parent_version_id": 41,
  "signal_settings": [
    {
      "code": "grid_entry",
      "name": "Grid entry",
      "leg_keys": ["btc_perp"],
      "params": []
    }
  ],
  "legs": [
    {
      "leg_key": "btc_perp",
      "name": "BTC Perpetual",
      "provider_code": "<provider code>",
      "program": {
        "schema": "quantflow_bot_v1",
        "program": "<complete QuantFlow bot program>"
      },
      "margin_mode": "Cross",
      "margin_leverage": 20,
      "actions": [],
      "modules": []
    }
  ]
}
```

Each leg requires `leg_key`, `name`, `provider_code`, and `program`. A leg's `program` must be a complete `quantflow_bot_v1` object. Use the current QuantFlow syntax and required `fn main()` entry point. `margin_mode` and `margin_leverage` default to `Cross` and `20` if omitted.

`actions` defines bot actions for that leg. `signal_settings` defines strategy-version signal actions; workflow signal triggers must use an exact code defined there. These are separate configurations.

`modules` is optional. Omit it or pass an empty array for a QuantFlow-only leg. Add `modules` and matching top-level `module.start("alias")` declarations only when attaching successful WASM builds.

#### Working QuantFlow-only example

This QuantFlow-only example emits a grid action and is supported by the native backtest path. It is also published in the [bot strategy program guide](https://livtorgex.github.io/program-docs/bots/strategy-programs/#quantflow-only-example):

```quantflow
fn main(signal):
    algo.grid.long(90, 110, 4, 0, $.bot.config.max_amount)
```

### Bot runtime API

- **Indicators:** Bot programs read the worker's local indicator context with calls such as `ta.rsi(14).value`. Declare a default timeframe with `// @program(timeframe=60)`; `@on_indicator(60)` runs its handler after that timeframe updates. Timeframes are seconds. See the [bot strategy program guide](https://livtorgex.github.io/program-docs/bots/strategy-programs/) for the runtime details.
- **Events:** The live bot worker requires `fn main(...)`; it also supports `@on_market_decision` and `@on_position_update` (no parameters), `@on_position_opened` and `@on_position_closed` (one `position` parameter), `@on_order_update` (one `order` parameter), and `@on_action("code")` (one `values` parameter). The native backtest path does not dispatch these event handlers.
- **Orders and positions:** The live bot host provides `position.get`, `position.open`, `position.close`, `position.cancel`, `position.update_entry_price`, `position.set_stop_loss`, `position.set_take_profit`, their `_prc` protection variants, `order.place`, `order.cancel`, and `bot.stop`. The native backtest path currently processes `algo.grid` outputs; it does not process QuantFlow `position.*` or `order.*` action effects.

### Run a native backtest

An unreleased pre-release version can be backtested without releasing it. The native backtest evaluates `main(signal)` when a backtest action fires and currently processes `algo.grid` outputs. It does not provide the live TA context or process the live event handlers and `position.*`/`order.*` action effects. Define a version-level signal action in `signal_settings`; pass the same code in `action_config.signal_action_code`. The creation example above defines `grid_entry`. `signal_settings` is separate from each leg's `actions` array. If the version has no matching signal action, add one with `update_strategy_version` before backtesting.

Get the owned leader ID from `strategy_lead_list` (filter by `strategy_id`), then call KaraCat's `run_lead_strategy_backtest` with that `strategy_leader_id`, the exact pre-release `strategy_version_id` returned by creation, one to five exact symbol keys, the requested interval in Unix milliseconds, and a matching `action_config`:

```json
{
  "strategy_leader_id": 7,
  "strategy_version_id": 123,
  "start_time": 1760000000000,
  "end_time": 1760600000000,
  "symbols": [{ "symbol_key": "<exact symbol key>" }],
  "action_config": {
    "signal_action_code": "grid_entry",
    "action_params": {},
    "trigger": { "type": "AtTime", "time_ms": 1760000000000 },
    "direction": "long",
    "leverage": 20,
    "margin": 100
  }
}
```

Use the backtest call's returned `version_id` as `backtest_version_id` in `list_backtest_runs`; then use a returned run ID with `get_backtest_run` to read its results. The backtest executes the bot's QuantFlow program directly; WASM is needed only when the leg explicitly attaches a module.

### 4. Edit an unreleased version or leg

Use `update_strategy_version` with `version_id` to change version metadata or replace the complete `signal_settings` array.

Use `update_strategy_version_leg` with `leg_id` to change a leg. Omit fields to preserve them. A supplied `program`, `actions`, or `modules` replaces that complete field; when changing `module.start` aliases, update the full `program` and `modules` together. Module aliases must match the top-level `module.start("alias")` declarations, and each `module_build_id` must refer to a successful build owned by the strategy owner.

API-key requirements are version-scoped. Use `strategy.api_key_requirement.list/create/update/delete` to manage them, then link a requirement to a leg with `strategy_version_api_key_requirement_id`. A linked requirement must belong to the same version, and its provider restriction must match the leg.

### 5. Review the saved version

Use `get_strategy` to inspect the returned version and legs, including their program, actions, module bindings, and signal settings. There is no separate strategy-version validation tool. Create and update requests report parsing and schema errors directly; correct those errors and review the saved result before release.

### 6. Release only when requested

Call `release_strategy_version` with the version's `version_id` only when the user asks to release it. A released version is immutable. To make later changes, create a new pre-release version.

## Current strategy tools

- Read: `list_strategies`, `get_strategy`
- Strategy metadata: `update_strategy`
- Version and leg authoring: `create_strategy_version`, `update_strategy_version`, `update_strategy_version_leg`
- Release: `release_strategy_version`
- Version API-key requirements: `strategy.api_key_requirement.list`, `.create`, `.update`, `.delete`

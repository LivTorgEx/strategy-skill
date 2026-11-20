---
name: create-strategy
description: Create, edit, and release a version of an existing LivTorgEx strategy with the strategy MCP tools. Use when the user asks to author or revise strategy versions and legs.
---

# Create or update a strategy version

This guide uses the current strategy-version MCP API for existing owned strategies and their versions.

A strategy is an existing owned record. The MCP can list and edit owned strategies, create versions for them, and edit or release those versions. It does not expose a tool to create the strategy record itself. If the user has no existing strategy to use, ask them to select or create one in the product before continuing.

For QuantFlow syntax, read `resource://quantflow-syntax`. For building and attaching a Rust WASM module, read the `strategy-rust-module` skill.

## Workflow

### 1. Clarify the requested version

Confirm the target strategy, version name, intended legs, each leg's provider and QuantFlow program, margin mode and leverage, and any bot actions, workflow signal actions, API-key requirements, or WASM modules needed. Keep leg actions separate from version-level signal settings.

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
  "signal_settings": [],
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

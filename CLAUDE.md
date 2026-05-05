# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**お立ち台Bot** is a Discord bot (written in Rust) that manages voice-channel room rentals. Users request → get assigned → release rooms via slash commands and VC events.

## Commands

```bash
cargo build                          # build
cargo run                            # run (requires .env.app)
cargo check                          # fast check, no codegen
cargo clippy                         # lint
cargo fmt                            # format
cargo test                           # unit tests
cargo test <name>                    # single test
cargo test -- --ignored              # integration tests (live DB required)
docker compose up -d                 # start bot + DB
```

Environment is loaded from `.env.app` (see `.env.app.example`). Copy and fill it before running locally.

For migrations, use `/migrate`. To add i18n messages, use `/add-i18n`.

## Architecture

### Event flow

```
Discord Gateway
  └─ main.rs: Shard loop
       └─ discord::events::dispatcher::dispatch()
            ├─ InteractionCreate → discord::events::interaction (slash commands & modal submits)
            ├─ VoiceStateUpdate  → discord::events::voice_state (VC join/leave)
            └─ GuildCreate/Ready → discord::events::guild (register slash commands)
```

### Module layout

| Module | Purpose |
|---|---|
| `app_state` | Shared state passed to every handler (`DbPools`, `HttpClient`, `I18n`, `RentalStateMap`) |
| `config` | Reads env vars into `AppConfig` |
| `db::connections` | Four `DatabaseConnection` pools keyed by DB role |
| `db::rls` | `with_guild_context()` — wraps a transaction with PostgreSQL RLS `SET LOCAL app.current_guild_id` |
| `discord::commands` | Slash command definitions and handlers (`admin/`, `user/`) |
| `discord::components` | Component (button/modal) handlers |
| `discord::events` | Gateway event dispatching |
| `entities` | SeaORM entity models (thin, no logic) |
| `facade` | DB query helpers grouped by domain (`guild_settings`, `rental`, `room`) |
| `i18n` | Fluent-based localisation; locale files in `locales/{en,ja}/main.ftl` |
| `rental::state_machine` | In-memory `DashMap<(guild_id, voice_channel_id), RentalStateEntry>` tracking VC rental state |
| `rental::flow` | Orchestrates DB writes + state map updates for the rental lifecycle |
| `rental::timeout` | Spawns/aborts Tokio tasks for the 10-minute purpose-input deadline |
| `rental::handoff` | Handles room-host handoff when the host leaves a VC |

### Database roles

The app connects with four separate PostgreSQL roles to enforce least-privilege:

| Role | Env prefix | Used for |
|---|---|---|
| `system` | `SYSTEM_DB_*` | Background/scheduled tasks; BYPASSRLS |
| `guild` | `GUILD_DB_*` | Discord command execution; RLS enforced per guild |
| `global` | `GLOBAL_DB_*` | Master data updates; BYPASSRLS |
| `admin` | `ADMIN_DB_*` | Migrations and schema changes |

All guild-scoped DB calls must go through `db::rls::with_guild_context()`, which sets the PostgreSQL session variable `app.current_guild_id` that RLS policies read.

### Rental state machine

States live in `RentalStateMap` (in-memory) and are mirrored to `rental_sessions.state` in the DB:

```
AwaitingPurpose (1) → Active (2) → PendingHandoff (4) → Released (3)
                  ↘ Released (3, on timeout/cancel)
```

Each in-memory entry holds the DB `session_id`, the Tokio timeout `JoinHandle` (where applicable), and the `room_id`.

### Localisation

Locale files are in `locales/{en,ja}/main.ftl`. Use `/add-i18n` for the full workflow.

### Integration tests

Tests in `tests/integration/` require a running PostgreSQL instance and are marked `#[ignore]`.

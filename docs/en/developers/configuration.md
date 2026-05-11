# Configuration Specification

otachidai Bot reads configuration from three layers.

| Layer | Medium | Primary owner | Scope |
|---|---|---|---|
| **Environment variables** | `.env.app` | Bot operator | Entire bot process |
| **Server settings in DB** | PostgreSQL (`guild_master.*`) | Server admin | Per server (guild) |
| **Locale files** | `locales/{lang}/main.ftl` | Developer | Entire bot process |

This document focuses on **file formats** (environment variables and locale files). DB-backed settings are described by schema and update paths only.

---

## Environment Variables (`.env.app`)

`src/config.rs::AppConfig::from_env` reads these at process startup. Copy `.env.app.example` to create `.env.app`.

### Core Settings

| Key | Required | Default | Purpose |
|---|---|---|---|
| `DISCORD_TOKEN` | Required | — | Discord Bot token |
| `BOT_ADMIN_SERVER_ID` | Required for normal startup | — | Discord ID of the bot admin-only server |
| `TZ` | Optional | `UTC` | Process time zone. `UTC` is recommended |
| `RUST_LOG` | Optional | `info` | `tracing` filter expression, for example `info`, `debug`, or `otachidai_bot=debug` |
| `APP_LANGUAGE` | Optional | None | Fallback language. `ja` or `en`. If unset, server setting is used, then finally `ja` |

`APP_LANGUAGE` is normalized by `language::normalize_language`. Any value other than `ja` or `en` causes a startup error.

### Database Connection Information

The app connects to PostgreSQL with **four roles**. Each role has different RLS bypass behavior and CRUD scope. See the DB role table in [architecture.md](architecture.md) for details.

#### Common

| Key | Required | Default | Purpose |
|---|---|---|---|
| `DB_HOST` | Optional | `localhost` | PostgreSQL host |
| `DB_PORT` | Optional | `5432` | Port. Startup fails if it cannot be parsed as a number |
| `DB_NAME` | Optional | `otachidai_bot_db` | Database name |

#### Per Role

| Key | Required | Purpose |
|---|---|---|
| `SYSTEM_DB_USER` / `SYSTEM_DB_PASSWORD` | Required | Scheduler and background processing (BYPASSRLS) |
| `GUILD_DB_USER` / `GUILD_DB_PASSWORD` | Required | Guild-scoped command execution (RLS applied) |
| `GLOBAL_DB_USER` / `GLOBAL_DB_PASSWORD` | Required | Global/master data updates (BYPASSRLS) |
| `ADMIN_DB_USER` / `ADMIN_DB_PASSWORD` | Required | Migrations (BYPASSRLS) |

#### Migration Only

| Key | Required | Purpose |
|---|---|---|
| `DATABASE_URL` | Required for migrations | Admin-role connection string used by `sea-orm-cli` and the migration binary |

`.env.app.example` includes a commented example like `export DATABASE_URL=postgres://otachidai_bot_admin:...`.
It is not required for normal startup, but it is referenced by `/migrate` and similar workflows.

### Required vs Optional

`config.rs::require_env` returns `BotError::Env(<key>)` and fails startup when a required environment variable is missing.
Optional values fall back through `std::env::var().unwrap_or_else(...)`.

---

## Server-Specific Settings in DB

Guild-scoped settings are changed by server admins through slash commands. Bot operators do not need to edit SQL directly.
The schema is under `guild_master`.

| Table | Contents | Update command |
|---|---|---|
| `guilds` | Basic guild settings such as language | No command currently. Inserted automatically by `ensure_guild` |
| `guild_channels` | Report channel and rental button channel | `/register_report_channel`, `/register_rental_button_channel` |
| `rooms` | Rentable rooms | `/register_room`, `/delete_room` |
| `rental_question_presets` | Per-room question presets | `/register_question_preset` |

### `guilds`

| Column | Type | Contents |
|---|---|---|
| `guild_id` | `BIGINT` PK | Discord guild ID |
| `language` | `TEXT` | Default `ja` (`facade::guild_settings::DEFAULT_LANGUAGE`) |
| `created_at` / `updated_at` | `TIMESTAMPTZ` | Creation and update timestamps |

`ensure_guild` upserts a guild when it first appears (`do_nothing` `OnConflict`).

### `guild_channels`

| Column | Type | Contents |
|---|---|---|
| `id` | `SERIAL` PK | Internal ID |
| `guild_id` | `BIGINT` | Discord guild ID |
| `channel_id` | `BIGINT` | Discord channel ID |
| `channel_type` | `SMALLINT` | `1=report channel` / `2=rental button channel` |
| `created_at` | `TIMESTAMPTZ` | Creation timestamp |

`facade::guild_settings::upsert_channel` manages one row per `(guild_id, channel_type)`, so there is only one channel per type.

### `rooms`

| Column | Type | Contents |
|---|---|---|
| `id` | `SERIAL` PK | Internal ID |
| `guild_id` | `BIGINT` | Discord guild ID |
| `text_channel_id` | `BIGINT?` | Text channel ID (optional) |
| `voice_channel_id` | `BIGINT?` | Voice channel ID (optional) |
| `is_available` | `BOOLEAN` | Availability flag. `false` while rented |
| `question_preset_id` | `INT?` | Linked `rental_question_presets.id` |
| `created_at` | `TIMESTAMPTZ` | Creation timestamp |

At least one of `text_channel_id` and `voice_channel_id` is expected to be set; command validation enforces this.

### `rental_question_presets`

| Column | Type | Contents |
|---|---|---|
| `id` | `SERIAL` PK | Internal ID |
| `guild_id` | `BIGINT` | Discord guild ID |
| `name` | `TEXT` | Preset name, unique within the guild |
| `question_1` through `question_10` | `TEXT?` | Question text. Empty strings and whitespace-only values are treated as invalid |
| `created_at` / `updated_at` | `TIMESTAMPTZ` | Creation and update timestamps |

`Model::questions()` returns only non-empty elements as `Vec<String>`. Questions are displayed in this order during rental.

---

## Locale Files

UI strings are stored in Fluent format at `locales/{lang}/main.ftl`. The currently supported languages are `ja` and `en`.

### File Layout

```
locales/
├── ja/
│   └── main.ftl
└── en/
    └── main.ftl
```

`src/i18n/loader.rs` loads both languages at startup and exposes them through `AppState.i18n`.

### Key Naming Rules

The `MessageKey` enum in `src/i18n/messages.rs` corresponds one-to-one with keys in `.ftl` files. Keys use kebab-case and are grouped with prefixes.

| Prefix | Purpose |
|---|---|
| `bot-rental-*` | User-facing rental flow strings |
| `bot-handoff-*` | Handoff flow strings |
| `admin-*` | Admin command responses |
| `error-*` | Error responses |
| `help-*` | `/help` sections |
| `rent-button-*` | Rental button labels |

Use `/add-i18n` when adding new keys. If editing manually, always update **both `.ftl` files and `messages.rs`** so all three stay aligned.

### Placeholders

Dynamic values use Fluent's `{ $name }` format. Code passes values through `FluentArgs` with `args.set("name", value)`.

```ftl
admin-report-channel-registered = Report channel registered: { $channel }
```

```rust
let mut args = FluentArgs::new();
args.set("channel", format!("<#{channel_id}>"));
state.i18n.get_with_args(lang, &MessageKey::AdminReportChannelRegistered, Some(&args));
```

### Language Resolution Priority

`language::resolve_language` determines language in this order. The first valid value wins.

1. `interaction.locale` for interactions or `member.user.locale` for VC events (Discord user language)
2. Environment variable `APP_LANGUAGE`
3. `guild_master.guilds.language`
4. `DEFAULT_LANGUAGE` (`ja`)

BCP-47-like values such as `ja-JP` and `en_GB` are normalized by looking only at the first language tag.

---

## Related Documentation

- [basic-design.md](basic-design.md) — bot concept and design principles
- [architecture.md](architecture.md) — module layout and DB role separation
- [command-specification.md](command-specification.md) — commands that modify DB settings
- [Bot Operators: Setup](../bot-operators/setup.md) — environment variable setup examples

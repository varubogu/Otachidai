# Getting Started (Server Admins)

Initial setup steps to enable otachidai Bot in your Discord server.

## Prerequisites

- otachidai Bot has been invited to your server
- You are logged in with a server administrator account

## Overview

All guild configuration (report channel, rental button channel, room list, question presets, rooms, groups, routing rules) is managed by a **single YAML file**. All additions, changes, and deletions go through `/upload_guild_config`.

## First-time setup

### 1. Fetch the template YAML

Even before any setup, you can download an (initially empty) template:

```
/download_guild_config
```

The bot returns `guild_config.yml` as an ephemeral attachment.

### 2. Edit the YAML

Open the file in your editor and fill in your configuration:

```yaml
version: 1

guild:
  language: en

channels:
  report: "111111111111111111"            # timeout notifications
  rental_button: "222222222222222222"     # channel hosting the rental button
  room_list: "333333333333333333"         # status board channel
  rental_post_fallback:                   # used when no routing rule matches
    channel: "444444444444444444"

question_presets:
  - name: "Standard"
    questions:
      - text: "Purpose?"
        answers: ["Chat", "Work", "Event"]
        routing_key: true                  # answer drives routing

rooms:
  - voice_channel_id: "555555555555555555"
    text_channel_id: "666666666666666666"
    question_preset: "Standard"

routing_rules:
  - preset: "Standard"
    rules:
      - when: "Chat"
        channel: "777777777777777777"
      - when: "Work"
        channel: "888888888888888888"
```

For full schema details, see [Guild configuration YAML spec](guild-config-yaml.md).

### 3. Upload the YAML

```
/upload_guild_config file:<attachment>
```

- If validation fails, the DB is left untouched and an ephemeral error reply explains why.
- On success, settings are applied immediately — the status board, post targets, and so on switch over.

### 4. Verify

Use `/list_rooms` and `/list_question_presets` to confirm what's now registered.

## Changing settings later

1. `/download_guild_config` to fetch current state
2. Edit locally
3. `/upload_guild_config` to apply the changes

**Important**: Upload performs a full replacement. Entities not present in the uploaded YAML are deleted. If a room being deleted has an active rental session, that session is force-released.

## Related Documentation

- [Command Reference](command-reference.md) — detailed description of each command
- [Guild configuration YAML spec](guild-config-yaml.md) — YAML format and validation rules
- [Troubleshooting](troubleshooting.md) — common issues and solutions

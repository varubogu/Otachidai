# Guild configuration YAML spec

File format for `/upload_guild_config` and `/download_guild_config`.

The YAML file is only used to transfer configuration between admin and bot. The source of truth is the DB; the uploaded file is discarded after parsing.

## Top-level structure

```yaml
version: 1

guild:
  language: ja                       # "ja" or "en"

channels:
  report: "111111111111111111"           # timeout notifications
  rental_button: "222222222222222222"    # channel hosting the rental button
  room_list: "333333333333333333"        # channel showing the status board
  rental_post_fallback:                  # fallback when no routing rule matches
    channel: "444444444444444444"
    template: |
      {{user}} started using {{room}}
      {{answers}}

question_presets:
  - name: "Standard"
    questions:
      - text: "Purpose?"
        answers: ["Chat", "Work", "Event"]    # omit for free text input
        routing_key: true                      # max 1 per preset
      - text: "Headcount"
        answers: ["1", "2", "3", "4"]
      - text: "Notes"

room_groups:
  - name: "Main Floor"
    channel_id: "555555555555555555"

rooms:
  - voice_channel_id: "666666666666666666"   # identifies the room; must be unique
    text_channel_id: "777777777777777777"    # optional
    group: "Main Floor"
    question_preset: "Standard"

routing_rules:
  - preset: "Standard"
    rules:
      - when: "Chat"
        channel: "888888888888888888"
        template: |
          🎙 Chat room open by {{user}} in {{room}}
          Headcount: {{answer:Headcount}}
          Notes: {{answer:Notes}}
      - when: "Work"
        channel: "999999999999999999"
        # template omitted → built-in default
```

## Validation

- `version` must be **exactly 1**.
- All channel/VC IDs must be 17–20 digit numeric strings.
- Names must be unique within their scope (`question_presets[].name`, `room_groups[].name`). Rooms are uniquely identified by `voice_channel_id`.
- Any `rooms[].group`, `rooms[].question_preset`, or `routing_rules[].preset` must reference an entity defined in the same YAML.
- A preset may have at most one question with `routing_key: true`. If routing rules exist for the preset but no routing-key question is defined, validation fails.
- If the routing-key question is a dropdown, `routing_rules[].rules[].when` must match one of its answer options.
- A preset may define up to 10 questions.

## Apply semantics (full replacement)

- The YAML is treated as the **complete source of truth** for the guild. Entities not present in the YAML are deleted.
- If a room being removed has an active rental session, that session is force-released. The host is notified, and the in-memory state and timeout task are cleaned up.
- The whole apply is a single DB transaction. Any failure rolls back completely.

## Template syntax

Routing post templates use Mustache-style `{{ name }}` placeholders.

- To emit literal `{{` or `}}`, escape with backslash: `\{{` / `\}}`.
- Available variables:

| Variable | Meaning |
|---|---|
| `{{user}}` | Mention of the user who initiated the rental |
| `{{room}}` | Mention of the room's text channel (falls back to VC) |
| `{{when}}` | The matched routing-key value (empty for fallback) |
| `{{preset}}` | The question preset's name |
| `{{q1}}`–`{{q10}}` | Each answer indexed by the question's position in the YAML |
| `{{answer:question-text}}` | Answer keyed by the question's literal text |
| `{{answers}}` | The default purpose block (multi-line Q + A) |

Unknown variables or references to questions not in the preset are rejected at upload time.

If `template` is omitted, the built-in default (i18n key `bot-rental-post-default-template`) is used.

## Recommended workflow

1. `/download_guild_config` to fetch current state
2. Edit locally
3. `/upload_guild_config` to apply
4. Confirm with `/list_rooms` etc.

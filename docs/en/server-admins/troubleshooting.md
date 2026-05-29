# Troubleshooting

## Bot does not respond to commands

**Possible causes and fixes:**

1. **Check if the bot is online**  
   Look at the server member list to see if the bot shows as online. If offline, ask the bot operator to restart the bot.

2. **Slash commands are not appearing**  
   The bot may have been invited without the `applications.commands` scope. Ask the bot operator to re-invite using the correct OAuth2 URL.

3. **Commands have not propagated yet**  
   Discord may cache slash commands for up to an hour after they are registered. Wait a while and try again.

---

## Bot cannot post in a specified channel

**Possible causes and fixes:**

1. **Check channel permissions**  
   Verify that the bot role has the following permissions in the channel:
   - `Send Messages`
   - `Read Message History`
   - `Embed Links`

2. **Private channel set as report channel**  
   If your YAML sets `channels.report` to a private channel, grant the bot **View Channel** and **Send Messages** permissions for that channel.

---

## Rental requests are not going through / no room is assigned

**Possible causes and fixes:**

1. **No rooms have been registered**  
   Run `/list_rooms` to confirm. If empty, fill in the `rooms` section of your YAML and apply with `/upload_guild_config`.

2. **All rooms are currently in use**  
   If every registered room is occupied, new requests cannot be accepted. Wait for a room to be released.

3. **Rental button channel not set**  
   Set `channels.rental_button` in your YAML and apply with `/upload_guild_config`.

---

## 10-minute timeout notifications are not arriving

**Possible causes and fixes:**

1. **Report channel not configured**  
   Check that `channels.report` is set in your YAML and applied via `/upload_guild_config`.

2. **Bot cannot post in the report channel**  
   See "Bot cannot post in a specified channel" above.

---

## Routing auto-post is not firing

**Possible causes and fixes:**

1. **Preset has no `routing_key: true` question**  
   Routing matches against the answer to the question flagged `routing_key: true`. Without that flag, no rule lookup happens and the post falls through to the fallback channel (if any).

2. **`routing_rules[].when` does not match a dropdown option**  
   For dropdown questions, the `when` value must match one of the answer options exactly. Validation rejects mismatches at upload time, but it's worth re-downloading the YAML to inspect what's currently stored.

3. **No fallback channel is configured**  
   If no rule matches and `channels.rental_post_fallback` is not set, the post is skipped (a warning is logged).

---

## Still not resolved?

Contact the bot operator or open an issue in the repository. Please include:

- A description of the problem
- The command(s) you ran
- Any error messages shown
- Bot logs (request from the bot operator)

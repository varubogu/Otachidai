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
   If `/register_report_channel` was used with a private channel, grant the bot **View Channel** and **Send Messages** permissions for that channel.

---

## Rental requests are not going through / no room is assigned

**Possible causes and fixes:**

1. **No rooms have been registered**  
   Confirm that you have registered rooms with `/register_room`.

2. **All rooms are currently in use**  
   If every registered room is occupied, new requests cannot be accepted. Wait for a room to be released.

3. **Rental button channel not set**  
   Confirm that `/register_rental_button_channel` has been run.

---

## 10-minute timeout notifications are not arriving

**Possible causes and fixes:**

1. **Report channel not configured**  
   Check that `/register_report_channel` has been run with a valid channel ID.

2. **Bot cannot post in the report channel**  
   See "Bot cannot post in a specified channel" above.

---

## Still not resolved?

Contact the bot operator or open an issue in the repository. Please include:

- A description of the problem
- The command(s) you ran
- Any error messages shown
- Bot logs (request from the bot operator)

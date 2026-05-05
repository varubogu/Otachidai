# Command Reference (Server Admins)

Commands that require administrator permissions. For user-facing commands, see [Server Users: Command Reference](../server-users/command-reference.md).

---

## `/register_report_channel`

Registers the report notification channel.

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | Text channel ID (required) |

**Description**

Sets the channel that receives notifications when a user joins a VC but fails to submit a rental purpose within 10 minutes.  
If using a private channel, grant the bot View Channel and Send Messages permissions for it.

```
/register_report_channel 123456789012345678
```

---

## `/register_rental_button_channel`

Registers the channel where the rental button is posted.

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | Text channel ID (required) |

**Description**

After registration, the bot automatically posts a rental request button in the specified channel. Users can click it to begin the rental flow.

```
/register_rental_button_channel 123456789012345678
```

---

## `/register_room`

Registers a room available for rental.

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Arguments | Text channel ID (optional), Voice channel ID (optional) — at least one required |

**Description**

A room can be a text channel alone, a voice channel alone, or a text+VC pair.  
When both IDs are provided, the requester is granted access to both channels together.
For VC-only rooms, prompts are posted to the VC's built-in text chat. For text+VC pairs, prompts are posted to the specified text channel.

```
# Text + VC pair
/register_room 123456789012345678 987654321098765432

# VC only
/register_room 987654321098765432

# Text only
/register_room 123456789012345678
```

---

## `/delete_room`

Removes a registered room.

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Arguments | Text channel ID (optional), Voice channel ID (optional) — at least one required |

**Description**

For rooms registered as a text+VC pair, specifying either channel ID will delete the entire pair.

```
/delete_room 987654321098765432
```

# Getting Started (Server Admins)

Initial setup steps to enable otachidai Bot in your Discord server.

## Prerequisites

- otachidai Bot has been invited to your server
- You are logged in with a server administrator account

## Setup Order

Follow these steps in order.

### 1. Register a Report Channel

Register the channel where the bot will send notifications when a user joins a VC but does not submit a rental purpose within 10 minutes.

```
/register_report_channel <text_channel_id>
```

- A private channel visible only to staff is recommended
- If using a private channel, grant the bot **View Channel** and **Send Messages** permissions for that channel

### 2. Register a Rental Button Channel

Register the channel where the bot will post the rental request button.

```
/register_rental_button_channel <text_channel_id>
```

After registration, the bot automatically posts a rental button in the specified channel.

### 3. Register Rooms

Register rooms available for rental. A room can be a text channel, a voice channel, or a text+VC pair.

```
/register_room [text_channel_id] [voice_channel_id]
```

- Either argument alone is valid (e.g., VC-only room)
- Providing both creates a paired text+VC room
- Repeat the command to register multiple rooms

### Examples

```
# Text + VC pair
/register_room 123456789012345678 987654321098765432

# VC only
/register_room 987654321098765432

# Text only
/register_room 123456789012345678
```

## Verify Setup

Use `/help` to review current configuration and see the admin help guide.

## Related Documentation

- [Command Reference](command-reference.md) — detailed description of each command
- [Troubleshooting](troubleshooting.md) — common issues and solutions

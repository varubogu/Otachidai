# Usage (Server Users)

How to rent a room using Otachidai Bot.

## How to Start a Rental Request

You can begin a rental request in three ways:

1. **Click the rental button** — press the button posted in the channel set up by your server admin
2. **Run a command** — send `/rent` in any channel
3. **Join an empty VC** — entering a voice channel with no other participants automatically starts a request

## Rental Flow

```
1. Start a rental request (button / command / join empty VC)
        ↓
2. The bot asks you to describe your purpose
        ↓
3. Enter your purpose (within 10 minutes)
        ↓
4. A room is assigned and you are granted access
        ↓
5. Use the room
        ↓
6. Notify the bot when you are done
        ↓
7. The bot releases the room
```

## Ending Your Session

When you are done with the room, notify Otachidai Bot.  
The bot releases the room so the next user can request it.

## Room Handoff When Leaving a VC

If you are the room host and leave the voice channel:

- **No other participants remain** — the bot automatically releases the room
- **Others are still present** — the bot asks remaining participants if they want to take over. The first person to accept becomes the new host

## Purpose Submission Timeout

If you do not submit a rental purpose within **10 minutes** of joining a VC:

- The server staff is notified
- If you leave the VC before the 10-minute window closes, your request is cancelled (staff is not notified)

## Commands

See [Command Reference](command-reference.md) for details.

| Command | Description |
|---|---|
| `/rent` | Start a rental request |
| `/help` | Show bot usage instructions |

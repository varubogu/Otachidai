# Usage (Server Users)

How to rent a room using otachidai Bot.

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
6. Leave the VC
        ↓
7. The bot releases the room
```

## Cancelling a Request (Leaving the VC Before Submitting a Purpose)

If the bot asks you to enter a purpose and you leave the VC without submitting it, the request is cancelled.

```
1. Start a rental request and enter the waiting-for-purpose state
        ↓
2. Leave the VC without submitting a purpose
        ↓
3. The bot cancels the request and immediately releases the room
   (server staff are not notified)
```

## Handoff Flow

When the room is in use and the room host leaves the VC, a handoff starts if other participants are still in the VC.

```
1. The room host leaves the VC
        ↓
2. The bot posts a message with a handoff button in the channel
        ↓
3. A remaining participant clicks the "Take over" button
        ↓
4. The participant who clicked the button becomes the new room host and use continues
```

- If no other participants remain, no handoff occurs and the room is released immediately.

## Cancelling a Handoff (No One Accepts)

If nobody clicks the handoff button while the room is waiting for handoff, the room is automatically released after the timeout.

```
1. The handoff button is posted
        ↓
2. 5 minutes pass without anyone clicking "Take over"
        ↓
3. The bot cancels the handoff and automatically releases the room
```

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

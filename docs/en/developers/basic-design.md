# Basic Design

This document is the entry point for developers joining otachidai Bot.
It focuses on the concept and design philosophy so you can understand **what we are building, why it is built this way, and what behavior it is meant to provide**.
Implementation details such as module layout and state transitions are covered separately in [architecture.md](architecture.md).

## What This Bot Is

**otachidai Bot** is a Discord bot for lending voice channels (VCs) as shared resources on a Discord server.
Members who want to use a VC declare their purpose to the server before renting it, and the room is returned automatically when use ends. Server admins can understand who used which VC, when, and for what purpose.

## Core Design Idea

The design starts from an analogy with Discord's built-in **Stage Channels**.
In a stage channel, the asymmetric structure of "people who go on stage request permission first" naturally separates speakers from the audience.

otachidai Bot brings this structure to regular VCs and makes the following assumptions:

1. **Using a VC is a rental action** — a time-limited loan, not ownership
2. **Users must state their purpose before renting** — a request is the act of verbalizing intent
3. **No room is assigned without a stated purpose** — if the user times out, server staff are notified
4. **Rooms are returned automatically when use ends** — the bot detects when the room host leaves, including whether the room can be handed off

The name "otachidai" comes from the image of entering a VC as stepping onto a stage.

## Friction This Bot Removes

When many people share one Discord server, VC usage tends to create the following friction.
otachidai Bot absorbs these problems through one mechanism: lending a room only after asking the purpose.

- **Nobody knows who is using the VC or why** — this is invisible to both staff and other members
- **There is no reliable way to tell which VCs are available** — channel names and implicit rules gradually break down
- **Unauthorized occupation or disruptive use is discovered late** — "someone is just there" is hard to detect

Trying to solve the same problem through permissions alone requires staff labor and detailed rule documentation.

## Actors

| Role | Main capabilities |
|---|---|
| **Server user** | Starts rental requests and uses assigned rooms |
| **Room host** | A role derived from the requester. When the host leaves the VC, the room is released or handed off to another participant |
| **Server admin** | Registers rentable rooms, configures report destinations and rental button channels, and manages display language |
| **Bot operator** | Hosts the bot itself and provides it to multiple servers |

Server admins often also act as the staff who receive reports, but by configuring a separate report channel, notifications can be routed to staff members who do not have admin permissions.

## Core Use Cases

### 1. From Request to Use

Users can start a rental request in any of these ways:

- Click a rental button prepared by an admin
- Run the `/rent` command
- Enter a VC with no participants

The three entry points cover different moments: **when the user sees the button, when they think of using the bot, and when they simply want to use a VC**. The VC join trigger is especially important because it enables a natural experience where the user does not need to think about the bot.

When the request starts, the bot asks for the usage purpose. Once the user submits it, one available room is assigned.

### 2. Automatic Release and Handoff

When the room host leaves the VC, the bot evaluates the situation and releases the room.
If other participants remain, the bot asks whether someone will take over before deciding to release it. This prevents the accident where everyone loses access just because the host briefly left.

### 3. Reporting Abnormal Cases

If the usage purpose is not submitted within the configured window (10 minutes by default), the bot sends a notification to the registered report channel.
If the user leaves the VC before submitting a purpose, the bot treats it as a simple change of mind and does not report it. This is an intentional design choice to avoid increasing noise.

## Design Principles

### Prefer Natural Experiences Over Commands

Users should not have to feel like they are "using a bot".
Slash commands are a supporting path; the most natural path is joining a VC. Event-driven flows are prioritized over command completeness.

### Allow Per-Server Customization

Room layout, report destinations, rental button channels, and display language can be configured per server, and server admins can complete this without intervention from the bot operator.
The bot is designed for one instance to host multiple servers.

### Keep Data from Different Servers Separate

Even when multiple servers share the same bot instance, rooms, rental state, and settings from one server must never be visible or mutable from another server.
This is enforced at the database layer rather than relying only on application-layer checks. See [architecture.md](architecture.md) for the implementation.

### Avoid Language Assumptions

User-facing messages are localized. The display language is resolved from the user, server, or bot configuration.
The design supports operating both Japanese and English today, and adding future languages without hard-coding assumptions about a specific language.

### Treat Persistent State as the Source of Trust

Rental state is also kept in memory for responsiveness, but the in-memory state is only auxiliary.
The design assumes that correct state can be reconstructed from persistent DB state even if memory is lost due to a process restart or failure.

## Out of Scope

To keep the design focused, the following are explicitly out of scope:

- **Audio processing inside VCs** — recording, audio analysis, text-to-speech, and similar features are not handled
- **Chat moderation** — message content is not automatically inspected or censored
- **Reservation system** — future reservations such as "rent tomorrow at 20:00" are not supported
- **One user renting multiple rooms simultaneously** — the principle is one user, one room

These may be considered in the future, but they are not part of the current basic design.

## Next Reading

- [architecture.md](architecture.md) — implementation details such as module layout, event processing flows, and DB role separation
- [technology-stack.md](technology-stack.md) — language, libraries, and infrastructure choices
- [Server Admins: Getting Started](../server-admins/getting-started.md) — operational setup steps and commands
- [Server Users: Usage](../server-users/usage.md) — end-user operation flow

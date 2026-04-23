# Architecture

## System Overview

```
Discord Gateway
      │  WebSocket
      ▼
  Twilight        ← Discord API client (event ingestion & API calls)
      │
      ▼
Business Logic    ← command handling, rental state machine, timeout management
      │
      ▼
   SeaORM         ← ORM layer
      │
      ▼
  PostgreSQL       ← persistence (server config, rooms, rental state)
```

## Component Descriptions

### Twilight

An async Rust Discord library. Receives real-time events from the Gateway and performs REST API operations such as sending messages and managing channel permissions.

### Business Logic

- **Command handlers** — routing and processing of slash commands
- **Rental state machine** — manages transitions between request → approved → in-use → released
- **Timeout manager** — monitors the 10-minute window after a VC join

### SeaORM + PostgreSQL

Persists:
- Per-server configuration (language, report channel, rental button channel)
- Registered rooms
- Current rental state (who is using which room)

## Event Processing Flows

### Rental Request (via VC join)

```
VC join event
    │
    ├─ Already renting? → skip
    │
    ├─ Empty VC? → start request flow
    │       │
    │       ├─ Send DM / message asking for purpose
    │       │
    │       └─ Start 10-minute timeout task
    │               │
    │               ├─ Purpose submitted in time → cancel timeout, assign room
    │               │
    │               ├─ 10 min elapsed → notify server staff
    │               │
    │               └─ User leaves VC before 10 min → cancel request (no notification)
    │
    └─ VC has other participants → skip
```

### VC Leave (host departs)

```
VC leave event (host leaves)
    │
    ├─ No remaining participants → end session, release room
    │
    └─ Participants remain → send handoff confirmation
            │
            ├─ Handoff accepted → set accepting user as new host
            │
            └─ Timeout or declined → release room
```

## Development Environment

Using Dev Container is recommended.

### VS Code + Dev Container

1. Install the [Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension
2. Clone the repository
3. Open the repository in VS Code and select **Reopen in Container**
4. The container builds automatically with Rust + PostgreSQL ready to use

### Environment Variables

See `.devcontainer/.env` for development values. For production configuration, see [Bot Operators: Setup](../bot-operators/setup.md).

### Technology Stack Details

See [technology-stack.md](technology-stack.md).

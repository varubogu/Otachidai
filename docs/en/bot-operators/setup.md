# Setup (Bot Operators)

Instructions for hosting otachidai Bot and making it available in a Discord server.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/) installed
- Access to the [Discord Developer Portal](https://discord.com/developers/applications)
- Git

## 1. Create a Discord Bot

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) and click **New Application**
2. Enter an application name and create it
3. Select **Bot** from the left menu and click **Add Bot**
4. Click **Reset Token** under **TOKEN** and copy the token (you'll need it later)
5. Enable the following under **Privileged Gateway Intents**:
   - `SERVER MEMBERS INTENT`
   - `MESSAGE CONTENT INTENT`

## 2. Clone the Repository

```bash
git clone https://github.com/<your-org>/otachidai.git
cd otachidai
```

## 3. Configure Environment Variables

Create a `.env` file in the project root:

```bash
cp .env.example .env
```

Edit `.env` with the following values:

| Variable | Description | Example |
|---|---|---|
| `DISCORD_TOKEN` | Your Discord Bot token | `MTxxxx...` |
| `POSTGRES_USER` | PostgreSQL username | `postgres` |
| `POSTGRES_PASSWORD` | PostgreSQL password | `your_password` |
| `POSTGRES_DB` | Database name | `otachidai` |
| `POSTGRES_HOSTNAME` | DB hostname | `localhost` |
| `POSTGRES_PORT` | DB port | `5432` |

## 4. Start the Bot

```bash
docker compose up -d
```

Check the logs to confirm the bot started successfully:

```bash
docker compose logs -f app
```

## 5. Invite the Bot to Your Server

In the Discord Developer Portal, go to **OAuth2 → URL Generator** and configure:

- **Scopes**: `bot`, `applications.commands`
- **Bot Permissions**:
  - `Send Messages`
  - `Read Message History`
  - `Manage Channels`
  - `Connect` (for VC event handling)

Open the generated URL in a browser to invite the bot to your server.

## 6. Initial Configuration

After inviting the bot, a server admin needs to complete the initial setup.
See [Server Admins: Getting Started](../server-admins/getting-started.md).

## Troubleshooting

See [Troubleshooting](../server-admins/troubleshooting.md).

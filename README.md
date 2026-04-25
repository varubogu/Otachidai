# otachidai Bot

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![日本語](https://img.shields.io/badge/README-日本語-blue)](README.ja.md)

A Discord bot for managing channel rentals in your server. Users can request to rent a room, use it, and release it when done.

## Features

- **Channel rental management** — request → approve → release flow
- **VC event handling** — auto-trigger on voice channel join, 10-minute timeout with admin notification
- **Room handoff** — when the host leaves, other participants can take over
- **Multi-language support** — English and Japanese, configurable per server

## Quick Start

See [Bot Operators: Setup](docs/en/bot-operators/setup.md) for how to host and run the bot.

## Documentation

| Audience | Link |
|---|---|
| Bot Operators | [docs/en/bot-operators/](docs/en/bot-operators/) |
| Server Admins | [docs/en/server-admins/](docs/en/server-admins/) |
| Server Users | [docs/en/server-users/](docs/en/server-users/) |
| Developers | [docs/en/developers/](docs/en/developers/) |

Japanese documentation is available under [docs/ja/](docs/ja/).

## Tech Stack

Rust · [Twilight](https://twilight.rs) · [SeaORM](https://www.sea-ql.org/SeaORM/) · PostgreSQL · Docker Compose

## License

MIT — see [LICENSE](LICENSE)
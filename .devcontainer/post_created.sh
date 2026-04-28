#!/usr/bin/env bash

echo "Rust version:"
rustc --version

echo "### Updating package list..."
sudo apt-get update
sudo apt-get upgrade -y

echo "### Installing PostgreSQL client..."
sudo apt-get install -y postgresql-client

echo "### Installing sea-orm-cli..."
cargo install sea-orm-cli@^2.0.0-rc

sudo wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/local/bin/yq
sudo chmod +x /usr/local/bin/yq

echo "### Installing Claude Code CLI..."
curl -fsSL https://claude.ai/install.sh | bash

echo "### Installing OpenAI Codex CLI..."
npm i -g @openai/codex

echo "### Setup complete!"

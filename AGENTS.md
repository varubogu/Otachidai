# AGENTS.md

## Project Guidance

Read `CLAUDE.md` before making repository changes. Treat it as the shared project guide for both Claude Code and Codex.

## Claude Command Compatibility

Claude slash commands in `.claude/commands/` are project workflows. Use the matching command file when the task fits:

- `.claude/commands/add-i18n.md`: adding or changing localized messages in `locales/{en,ja}/main.ftl` and `src/i18n/messages.rs`
- `.claude/commands/migrate.md`: creating, registering, applying, or rolling back SeaORM migrations

These files are instructions, not executable scripts. Read the relevant file, follow its workflow, and adapt paths to the current code.

## Working Rules

- Respond to users in Japanese unless they explicitly ask for another language.
- Check `git status --short` before editing, and do not revert unrelated user changes.
- Use `db::rls::with_guild_context()` for guild-scoped database access.
- Keep user-facing strings in Fluent locale files and access them through `MessageKey` and `I18n`.
- Run the smallest useful validation for the change, then broaden when the touched behavior crosses module boundaries.
- Do not run live DB integration tests unless the user confirms the database environment is ready.

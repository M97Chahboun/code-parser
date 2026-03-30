# code-parser Claude Code Commands

Six slash commands that integrate `code-parser` into Claude Code for surgical,
token-efficient codebase navigation.

## Commands

| Command | Arguments | Purpose |
|---|---|---|
| `/index` | `[path]` | Index a file or directory — shows all classes, methods, and line ranges |
| `/parse-find` | `<name> [path]` | Locate a class or method — returns file and exact line range |
| `/parse-read` | `<file> <start> <end>` | Read specific lines from a file using the index |
| `/parse-edit` | `<file> <Class.method> <instruction>` | Surgical edit — indexes first, reads only relevant lines |
| `/parse-audit` | `[path]` | Full architecture report — class sizes, god classes, structural overview |
| `/parse-stats` | `[path]` | Token saving summary for the session |

## Install

### Personal (all projects)

```bash
chmod +x install-commands.sh
./install-commands.sh personal
```

Installs to `~/.claude/commands/` — available in every Claude Code session.

### Project-scoped (current repo only, commit to git)

```bash
./install-commands.sh project
git add .claude/commands/
git commit -m "add code-parser Claude Code commands"
```

### Manual

Copy any `.md` file to `~/.claude/commands/` or `.claude/commands/`:

```bash
cp index.md ~/.claude/commands/
cp parse-find.md ~/.claude/commands/
cp parse-read.md ~/.claude/commands/
cp parse-edit.md ~/.claude/commands/
cp parse-audit.md ~/.claude/commands/
cp parse-stats.md ~/.claude/commands/
```

## Usage

```
# In a Claude Code session:

> /index ./lib
> /index ./src/auth

> /parse-find UserService
> /parse-find fetchUser ./lib

> /parse-read lib/services/user_service.dart 15 22
> /parse-read lib/services/user_service.dart UserService fetchUser

> /parse-edit lib/services/user_service.dart UserService.fetchUser add error handling for 404

> /parse-audit
> /parse-audit ./lib

> /parse-stats
> /parse-stats ./src
```

## Requires

- `code-parser` on PATH (`cargo build --release && cp target/release/code-parser ~/.local/bin/`)
- `jq` on PATH (`apt install jq` / `brew install jq`)
- Claude Code CLI

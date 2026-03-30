#!/usr/bin/env bash
# install-commands.sh
# Install code-parser Claude Code commands as personal (~/.claude) or project (.claude) commands

set -e

COMMANDS_DIR="$(cd "$(dirname "$0")" && pwd)"
MODE="${1:-personal}"

if [[ "$MODE" == "personal" ]]; then
  DEST="$HOME/.claude/commands"
  echo "Installing as personal commands → $DEST"
elif [[ "$MODE" == "project" ]]; then
  DEST=".claude/commands"
  echo "Installing as project commands → $DEST"
else
  echo "Usage: ./install-commands.sh [personal|project]"
  echo "  personal  — available in all projects  (default)"
  echo "  project   — available in current project only"
  exit 1
fi

mkdir -p "$DEST"

for f in "$COMMANDS_DIR"/*.md; do
  name="$(basename "$f")"
  cp "$f" "$DEST/$name"
  echo "  ✓ Installed $name"
done

echo ""
echo "Done. Available commands:"
echo "  /index          [path]                  — index a file or directory"
echo "  /parse-find     <ClassName> [path]      — locate a class or method"
echo "  /parse-read     <file> <start> <end>    — read specific lines"
echo "  /parse-docs     [path] [Class] [Method] — extract documentation comments"
echo "  /parse-edit     <file> <Class.method>   — surgical edit via index"
echo "  /parse-audit    [path]                  — full project structure report"
echo "  /parse-stats    [path]                  — token saving summary"
echo ""
echo "Run 'claude' and type /help to see your commands."

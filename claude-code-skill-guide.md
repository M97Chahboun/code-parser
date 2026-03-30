# Using the code-parser Skill with Claude Code CLI

A practical guide to slashing token costs when Claude Code navigates your Dart, Python, or TypeScript codebase.

---

## What This Achieves

Without the skill, Claude Code reads entire source files to answer questions or make edits — often 10,000–50,000 tokens per task on a medium-sized project.

With the `code-parser` skill installed, Claude Code:

1. Runs `code-parser` to index the codebase structure (~400 tokens)
2. Identifies exactly which classes and methods are relevant
3. Reads only those specific line ranges (~100–300 tokens per method)

**Result: 70–97% fewer tokens per task, with no loss of accuracy.**

---

## Prerequisites

| Requirement | Version | Check |
|---|---|---|
| Claude Code | Latest | `claude --version` |
| Rust + Cargo | 1.75+ | `cargo --version` |
| code-parser binary | 0.1.0+ | `code-parser --version` |

---

## Step 1 — Build and Install code-parser

If you haven't built the binary yet:

```bash
# Clone the repo
git clone https://github.com/m97chahboun/code-parser
cd code-parser

# Build release binary
cargo build --release

# Install to a directory on your PATH
cp target/release/code-parser ~/.local/bin/

# Verify
code-parser --version
```

Confirm it can find your project files:

```bash
# Test on a single file
code-parser path/to/your/file.dart --format pretty

# Test on a directory
code-parser ./lib --format pretty
```

---

## Step 2 — Install the Skill

The skill ships as a `.skill` file. Install it via the Claude Code CLI:

```bash
claude skill install code-parser-skill.skill
```

Verify it's installed:

```bash
claude skill list
# Should show:
# • code-parser   LLM-optimised code structure extractor — Dart · Python · TypeScript
```

---

## Step 3 — Verify the Skill Triggers

Start a Claude Code session and ask it to work with any Dart, Python, or TypeScript file. The skill should trigger automatically:

```bash
claude

# Inside the session:
> Explain how UserService works
```

You should see Claude run `code-parser` before reading any files. If it skips straight to reading files, see [Troubleshooting](#troubleshooting) below.

---

## Daily Usage

### Basic session startup

```bash
# Navigate to your project
cd ~/projects/my-flutter-app

# Start Claude Code
claude
```

The skill activates automatically for any request involving `.dart`, `.py`, `.ts`, or `.tsx` files.

### Explicit indexing

You can also ask Claude to index your project explicitly at the start of a session to prime its understanding before diving into tasks:

```
> Index the codebase and give me an overview of the architecture
```

Claude will run:

```bash
code-parser ./lib --format pretty
```

And summarise the classes, their relationships, and the overall structure — all from the index, without reading method bodies.

### Typical task flow

```
> Add email validation to the registration form
```

Claude will:
1. Run `code-parser ./lib` to find `RegistrationForm`, `FormValidators`, etc.
2. Read only the relevant method bodies using `sed -n 'X,Yp'`
3. Make the targeted edit

Compare this to the naive approach, which would read `registration_form.dart` in full (often 300–600 lines) before understanding what to change.

---

## CLI Flags and Options

### Scope the index to a subdirectory

For large monorepos, point code-parser at the relevant subdirectory:

```
> Refactor the auth module
```

Or explicitly:

```
> Run code-parser on ./lib/auth and show me the structure
```

### Format options

The skill uses `--format json` internally for programmatic parsing. Use `--format pretty` when you want human-readable output:

```bash
# In your terminal (outside Claude Code)
code-parser ./lib --format pretty | less
```

### Suppress warnings

If the project has files that fail to parse (e.g. generated code), suppress warnings:

```bash
code-parser ./lib --format json 2>/dev/null
```

---

## CLAUDE.md Integration

Add instructions to your project's `CLAUDE.md` so the skill behaviour is reinforced every session:

```markdown
## Code Navigation

This project uses the `code-parser` skill for efficient codebase navigation.

**Always follow this pattern:**
1. Run `code-parser ./lib --format json` before reading any source files
2. Use the index to identify relevant classes and methods
3. Read only the specific line ranges needed with `sed -n 'X,Yp' file`
4. Never load a full file if it's longer than 50 lines

**Project structure summary** (update when major refactors happen):
- `lib/screens/`   — Flutter UI screens
- `lib/services/`  — Business logic and API calls
- `lib/models/`    — Data models
- `lib/widgets/`   — Reusable UI components
```

This ensures Claude Code applies the pattern even on tasks where the skill description might not trigger automatically.

---

## Using with `claude -p` (Non-Interactive / Pipe Mode)

For scripted or automated use:

```bash
# Ask a question about the codebase non-interactively
claude -p "What classes are responsible for authentication?" \
  --allowedTools bash

# Run a refactor task on a specific file
claude -p "Add input validation to the email field in checkout_screen.dart" \
  --allowedTools bash,edit

# Generate a summary of the codebase structure
claude -p "Index the project with code-parser and describe the architecture" \
  --allowedTools bash > architecture_summary.md
```

### In a CI pipeline

```bash
#!/bin/bash
# ci-review.sh — run Claude Code review on changed files

CHANGED=$(git diff --name-only HEAD~1 | grep -E '\.(dart|py|ts|tsx)$')

if [ -n "$CHANGED" ]; then
  echo "Running code review on changed files..."
  claude -p "Review these changed files for bugs and code quality issues: $CHANGED.
  Use code-parser to index them first, then read only the changed methods." \
    --allowedTools bash \
    --output-format text
fi
```

---

## Slash Commands

Inside a Claude Code session you can use these slash commands to work with the skill directly:

```
/skills                     # list all installed skills
/skills info code-parser    # show skill details and trigger conditions
```

---

## Token Monitoring

Track how many tokens each session uses with the `--verbose` flag:

```bash
claude --verbose
```

Or check token usage after a session:

```bash
claude usage --last-session
```

Compare usage before and after installing the skill on the same task to measure your actual savings.

### Expected savings by project size

| Project size | Without skill | With skill | Saving |
|---|---|---|---|
| Small (< 2,000 lines) | ~8,000 tokens | ~1,200 tokens | ~85% |
| Medium (2,000–10,000 lines) | ~40,000 tokens | ~2,500 tokens | ~94% |
| Large (10,000–50,000 lines) | ~200,000 tokens | ~5,000 tokens | ~97% |
| Monorepo (50,000+ lines) | Context overflow | ~8,000 tokens | Enables otherwise impossible tasks |

---

## Multi-Language Projects

The skill handles mixed-language projects automatically. For example, a project with a Python backend and TypeScript frontend:

```
my-app/
├── backend/          ← Python (FastAPI)
│   └── app/
└── frontend/         ← TypeScript (React)
    └── src/
```

```bash
# Index the whole project at once
code-parser . --format json
```

Claude Code will receive a unified index covering both languages and read only the relevant files regardless of which layer the task touches.

---

## Troubleshooting

### Skill doesn't trigger automatically

**Symptom:** Claude reads full files without running `code-parser` first.

**Fix:** Reinforce the instruction explicitly:

```
> Before reading any files, run code-parser on the project and use the index
  to find relevant classes and methods.
```

Or add a `CLAUDE.md` file to your project (see [CLAUDE.md Integration](#claudemd-integration) above).

---

### `code-parser: command not found`

**Symptom:** Claude runs `code-parser` but the shell can't find it.

**Fix:** Make sure the binary is on PATH:

```bash
# Find the binary
ls ~/projects/code-parser/target/release/code-parser

# Add to PATH permanently (add to ~/.bashrc or ~/.zshrc)
export PATH="$PATH:$HOME/projects/code-parser/target/release"

# Verify
which code-parser
```

---

### Dart files return `warning: Failed`

**Symptom:** Python and TypeScript files parse fine but `.dart` files are skipped.

**Cause:** The hand-rolled Dart tokeniser doesn't handle some edge cases in generated or highly unusual Dart files.

**Fix:** For the affected files, fall back to a direct read with scoped grep:

```bash
# Find class boundaries manually
grep -n "^class\|^abstract class\|^mixin\|^enum" lib/problem_file.dart

# Then read the relevant range
sed -n '25,80p' lib/problem_file.dart
```

---

### Index output is very large

**Symptom:** The project has hundreds of files and the index is too large to be useful.

**Fix:** Scope the index to the relevant subdirectory, or filter with `jq`:

```bash
# Only index the module you're working in
code-parser ./lib/auth --format json

# Or filter to just the classes that matter
code-parser ./lib --format json | jq '
  .[] | select(.file | contains("auth"))
'
```

---

### Empty `classes: []` for a file

**Symptom:** A file appears in the index but with no classes.

**Cause:** The file is a script or top-level functions only (no class wrapper).

**Fix:** Read the file directly — it's likely small:

```bash
# Check the line count first
wc -l lib/utils/helpers.dart

# If small (< 80 lines), read in full
cat lib/utils/helpers.dart
```

---

## Quick Reference

```bash
# Install skill
claude skill install code-parser-skill.skill

# List skills
claude skill list

# Build code-parser
cargo build --release && cp target/release/code-parser ~/.local/bin/

# Index a project
code-parser ./lib --format pretty

# Index and filter with jq
code-parser ./lib | jq '[.[].classes[] | {name, kind, methods: (.methods | length)}]'

# Non-interactive task using skill
claude -p "Refactor UserService.fetchUser to use async/await" --allowedTools bash,edit

# Start interactive session
claude
```

---

## Further Reading

- `code-parser` README — full CLI reference and output schema
- `code-parser-skill/references/output-schema.md` — JSON schema and `jq` recipes
- `code-parser-skill/references/example-walkthrough.md` — end-to-end Flutter example
- [Claude Code documentation](https://docs.claude.ai/claude-code) — official CLI docs

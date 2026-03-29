---
name: code-parser
description: >
  Use this skill whenever you need to read, understand, navigate, edit, refactor,
  or reason about a codebase written in Dart, Python, or TypeScript — especially
  before reading any source files directly. This skill runs `code-parser` to build
  a lightweight structural index (classes, methods, line ranges) of the codebase
  first, then uses that index to read only the specific lines that matter.
  This drastically reduces token consumption and prevents context window overflow.

  Trigger this skill when the user says things like: "read this codebase",
  "understand my project", "refactor this class", "find where X is defined",
  "explain how Y works", "add a method to Z", "what does this file do",
  "help me debug", "review my code", or any task that would otherwise require
  loading entire source files. Always use this skill before reading .dart, .py,
  or .ts/.tsx files — even for single-file tasks. Do NOT skip this skill just
  because the request seems simple; the index step is always cheaper than full
  file reads.
---

# code-parser skill

Reduce LLM token consumption when working with Dart, Python, and TypeScript
codebases by indexing structure first, then reading only the lines that matter.

## Core principle

**Never read a full source file when you can read its structure first.**

```
Full file read:   ~15,000 tokens for a 2,000-line file
code-parser index: ~400 tokens for the same file
Targeted read:    ~300 tokens per method body

Total saving: 70–97% fewer tokens per task
```

---

## Workflow

Follow these phases in order. Never skip Phase 1.

### Phase 1 — Index the codebase (always do this first)

Run `code-parser` on the target path:

```bash
# Single file
code-parser path/to/file.dart --format pretty

# Whole directory (recursive)
code-parser ./lib --format pretty

# Compact JSON (for programmatic use)
code-parser ./src --format json
```

Parse the JSON output. You now have every class, interface, mixin, enum, and
method name — plus their exact line ranges — without reading a single line of
source code.

**What the output looks like:**
```json
[
  {
    "file": "lib/services/user_service.dart",
    "language": "dart",
    "classes": [
      {
        "name": "UserService",
        "kind": "class",
        "line_start": 8,
        "line_end": 47,
        "methods": [
          { "name": "fetchUser",   "line_start": 15, "line_end": 22 },
          { "name": "deleteUser",  "line_start": 24, "line_end": 31 },
          { "name": "get displayName", "line_start": 33, "line_end": 33 }
        ]
      }
    ]
  }
]
```

### Phase 2 — Identify what to read

Using the index, determine which classes and methods are relevant to the task.
Ask yourself:
- Which class owns the behaviour I need?
- Which specific methods are involved?
- Do I need the full class body, or just 1–2 methods?

**Do not read anything yet.** Just identify the relevant line ranges from the index.

### Phase 3 — Read targeted line ranges only

Use `sed` or `awk` to extract only the lines you need:

```bash
# Read lines 15–22 of a file (one method body)
sed -n '15,22p' lib/services/user_service.dart

# Read a class definition (lines 8–47)
sed -n '8,47p' lib/services/user_service.dart

# Read multiple disjoint ranges
sed -n '15,22p;45,60p' lib/services/user_service.dart
```

Read **one method at a time** unless the task genuinely requires multiple.
Never read more than 100 lines in a single fetch unless the task requires it.

### Phase 4 — Act on the code

Edit, explain, refactor, or debug based only on what you've read.
If you need more context, go back to the index and fetch the next relevant range.

---

## Decision rules

| Situation | Action |
|---|---|
| Task involves any `.dart`, `.py`, `.ts`, `.tsx` file | Run code-parser first |
| You need to understand a class | Read index → read class-level lines only |
| You need to edit a method | Read index → read that method's lines |
| You need to add a new method | Read index → read the class opening + closing brace |
| You need to understand the full codebase | Read index → read only the classes mentioned in the task |
| A method is complex and you need its full body | Read index → fetch that method's line range |
| File is < 50 lines total | Read the full file (index overhead not worth it) |

---

## Handling large codebases

If the directory has many files, run code-parser once on the whole tree and
filter the output with `jq` before presenting it to the user:

```bash
# Show only class names and method counts
code-parser ./src | jq '.[].classes[] | "\(.name): \(.methods | length) methods"'

# Find which file contains a specific class
code-parser ./lib | jq '.[] | select(.classes[].name == "UserService") | .file'

# Show all methods across all files
code-parser ./lib | jq '[.[].classes[].methods[].name] | unique | sort'
```

Only read line ranges from the files that the index shows are relevant.

---

## Common task patterns

### "Explain how X works"
1. Run index on the directory
2. Find the class(es) named X or containing X
3. Read the class header (line_start to first method start)
4. Read each method body one at a time, explaining as you go

### "Add a method to class X"
1. Run index → find X → note its `line_end`
2. Read lines `(line_end - 3)` to `line_end` to see the closing context
3. Read 1–2 existing methods to match the code style
4. Insert the new method before `line_end`

### "Refactor class X"
1. Run index → find X → note full line range
2. Read the full class (it's already scoped — not the whole file)
3. Refactor within that range

### "Fix bug in method Y of class X"
1. Run index → find X.Y → note its line range
2. Read only those lines
3. Fix the bug

### "Find where Z is defined"
1. Run index on the whole project
2. `jq` search for Z in class names and method names
3. Report the file and line number — no file reading needed at all

---

## Installing code-parser

If `code-parser` is not on PATH, check common locations or build it:

```bash
# Check if installed
which code-parser || ls ./target/release/code-parser 2>/dev/null

# Build from source (requires Rust 1.75+)
cd /path/to/code-parser
cargo build --release
export PATH="$PATH:$(pwd)/target/release"

# Verify
code-parser --version
```

---

## Token budget guidelines

| Operation | Approx tokens | When to use |
|---|---|---|
| Index a 500-line file | ~100 | Always — run first |
| Index a 5,000-line project | ~1,000 | Always — run first |
| Read one method (avg 15 lines) | ~120 | When you need logic details |
| Read one class (avg 80 lines) | ~640 | When refactoring the whole class |
| Read a full file (500 lines) | ~4,000 | Only if file is < 50 lines total |
| Read a full file (2,000 lines) | ~15,000 | Never — always use index instead |

---

## Error handling

| Error | Cause | Fix |
|---|---|---|
| `command not found: code-parser` | Not on PATH | Build from source or set PATH |
| `warning: Failed: file.dart` | Dart parser limitation | Read the Dart file with `sed` using class keyword search as a guide |
| `warning: Skipped: file.py` | Unreadable or binary | Skip the file |
| Empty `classes: []` | No top-level classes | The file may be a script — read it in full if small |

---

## Reference

For full output schema and advanced jq patterns, see `references/output-schema.md`.
For a complete worked example on a real Flutter project, see `references/example-walkthrough.md`.

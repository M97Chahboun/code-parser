---
name: parse-stats
description: Show a token-saving summary for the current session — how many lines were read via code-parser vs how many would have been read by loading full files. Use at end of a coding session to see the efficiency gains.
argument-hint: [path]
allowed-tools: Bash(code-parser *), Bash(jq *), Bash(wc *), Bash(find *), Bash(git *)
---

Calculate token savings from using code-parser in this session for `$ARGUMENTS` (default: current directory).

## Steps

### 1. Get total line counts for all source files

```bash
# Total lines in all Dart/Python/TypeScript files
find ${ARGUMENTS:-.} -type f \( -name "*.dart" -o -name "*.py" -o -name "*.ts" -o -name "*.tsx" \) \
  | xargs wc -l 2>/dev/null \
  | tail -1
```

### 2. Get the index size

```bash
# How many tokens the index itself costs (approximate: chars / 4)
INDEX=$(code-parser ${ARGUMENTS:-.} --format json)
echo "$INDEX" | wc -c
```

### 3. Calculate and display the report

Compute:
- **Total source lines** in the project
- **Estimated full-file tokens** = total lines × 8 (average chars per line / 4)
- **Index tokens** = index JSON size / 4
- **Typical session tokens** with code-parser = index + (lines read × 8)
- **Token saving** = full-file approach − code-parser approach

Present as:

```
╔══════════════════════════════════════════════╗
║         code-parser Session Stats            ║
╠══════════════════════════════════════════════╣
║ Source files found      │  24 files          ║
║ Total source lines      │  8,342 lines       ║
╠══════════════════════════════════════════════╣
║ NAIVE APPROACH                               ║
║ Read all files in full  │  ~66,736 tokens    ║
╠══════════════════════════════════════════════╣
║ WITH code-parser                             ║
║ Index cost              │  ~1,200 tokens     ║
║ Estimated read cost*    │  ~2,400 tokens     ║
║ Total estimated         │  ~3,600 tokens     ║
╠══════════════════════════════════════════════╣
║ 🎯 Estimated saving     │  ~94%              ║
║    (~63,000 tokens)                          ║
╚══════════════════════════════════════════════╝

* Based on reading ~10% of methods during a typical session.
  Actual savings depend on how many method bodies were read.

Project breakdown:
  dart  files: 18  (6,210 lines)
  ts    files:  6  (2,132 lines)
```

### 4. Add a note

Remind the user:
- These are estimates based on project size
- Actual savings depend on how targeted the reads were
- For tasks touching 1–2 methods, savings are typically 95%+
- For whole-class refactors, savings are typically 60–80%

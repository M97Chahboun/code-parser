---
name: parse-edit
description: Edit a specific class or method in a Dart/Python/TypeScript file using code-parser to locate it first, then reading only the relevant lines before making changes. Prevents loading entire files for small edits. Use when asked to modify, fix, refactor, or add to a specific method or class.
argument-hint: <file> <ClassName.methodName> <instruction>
allowed-tools: Bash(code-parser *), Bash(sed *), Bash(jq *), Read, Edit, Bash(wc *)
---

Perform a surgical edit on `$ARGUMENTS`.

## Parse the arguments

Expected format: `file.dart ClassName.methodName instruction`
Or: `file.dart ClassName instruction` (edit the whole class)
Or just a natural description like: `fix the fetchUser method in user_service.dart`

## Workflow

### Step 1 — Index the file (not the whole directory)

```bash
code-parser FILE --format json
```

This costs ~50–150 tokens for a single file. Do this before reading anything.

### Step 2 — Locate the target

From the index, find the exact `line_start` and `line_end` for the class or method being edited.

### Step 3 — Read only the relevant lines

```bash
sed -n 'line_start,line_endp' FILE
```

If the method calls other methods in the same class that are needed for context, fetch those too — but only those, by line range.

### Step 4 — Make the edit

Apply the change using the Edit tool on the specific lines. Do not rewrite the whole file.

Show the diff clearly:
```
Edited: lib/services/user_service.dart  (lines 15–22)

Before:
  if (!value.contains('@')) return 'Invalid email';

After:
  final regex = RegExp(r'^[\w.-]+@[\w.-]+\.[a-z]{2,}$');
  if (!regex.hasMatch(value)) return 'Enter a valid email address';
```

### Step 5 — Verify

After editing, re-read the edited lines to confirm the change looks correct:
```bash
sed -n 'line_start,line_endp' FILE
```

## Rules

- Never read the full file if it's longer than 60 lines
- Never rewrite the whole file to make a small change
- If the edit requires understanding how the method is called, use `/parse-find` to locate callers by method name — do not read entire files to find call sites
- Report the token saving: `(read 8 lines of 342-line file instead of full file — saved ~2,600 tokens)`

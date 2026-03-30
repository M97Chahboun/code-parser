---
name: index
description: Index a file or directory with code-parser and display all classes, methods, documentation, and line ranges. Use this before reading any Dart, Python, or TypeScript source files to avoid loading entire files into context.
argument-hint: [path]
allowed-tools: Bash(code-parser *), Bash(ls *), Bash(find *)
---

Index the codebase at `$ARGUMENTS` (default: current directory) using code-parser.

## Steps

1. Run code-parser on the target path:

```
code-parser ${ARGUMENTS:-.} --format pretty
```

If the binary is not found on PATH, check these locations before giving up:
```
./target/release/code-parser ${ARGUMENTS:-.} --format pretty
~/.local/bin/code-parser ${ARGUMENTS:-.} --format pretty
```

2. Parse the JSON output and present a clean summary showing:
   - Each file found
   - Every class/interface/mixin/enum with its kind and line range
   - Every method inside each class with its line range
   - Documentation comments (docstrings, JSDoc, Dart doc comments) when present
   - Total counts: files indexed, classes found, methods found

3. Highlight anything notable:
   - Large classes (more than 15 methods)
   - Files with no classes (likely utility scripts)
   - Any files that failed to parse (warnings from stderr)
   - Classes/methods with documentation (indicated with 📝)

## Output format

Present the index as a structured, scannable summary — NOT raw JSON. Use a format like:

```
📁 lib/services/user_service.dart  (dart)
  └── UserService [class] lines 8–47 📝
       ├── UserService()       lines 12–12 📝
       ├── fetchUser()         lines 15–22 📝
       ├── deleteUser()        lines 24–31
       └── get displayName     lines 33–33

📁 lib/models/user.dart  (dart)
  └── User [class] lines 3–28 📝
       ├── fromJson()          lines 10–16 📝
       └── toJson()            lines 18–23 📝

Indexed: 2 files · 2 classes · 5 methods · 5 with docs
```

After the summary, remind the user they can now use:
- `/parse-find` to locate specific classes
- `/parse-read` to read method bodies by line range
- `/parse-docs` to extract only documentation comments

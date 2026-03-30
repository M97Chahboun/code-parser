---
name: parse-find
description: Find a class, method, or symbol in a Dart/Python/TypeScript codebase using code-parser. Returns the file path and exact line range without reading any source code. Use when you need to locate where something is defined before reading it.
argument-hint: <ClassName> [path]
allowed-tools: Bash(code-parser *), Bash(jq *), Bash(ls *)
---

Find where `$ARGUMENTS` is defined in the codebase using code-parser.

## Parse the arguments

- If one word given: search for that class/method name in the current directory
- If two words given: first is the symbol name, second is the path to search

## Steps

1. Run code-parser and search with jq:

```bash
# Search for a class by name
code-parser ${PATH:-.} --format json | jq --arg name "$SYMBOL" '
  .[] | {file, language, matches: [.classes[] | select(.name == $name)]} |
  select(.matches | length > 0)
'

# Search for a method by name (across all classes)
code-parser ${PATH:-.} --format json | jq --arg name "$SYMBOL" '
  .[] | .file as $file |
  .classes[] | . as $class |
  .methods[] | select(.name == $name) |
  {file: $file, class: $class.name, method: .name, line_start, line_end}
'

# Fuzzy: match names containing the search term
code-parser ${PATH:-.} --format json | jq --arg name "$SYMBOL" '
  .[] | .file as $file |
  .classes[] | select(.name | ascii_downcase | contains($name | ascii_downcase)) |
  {file: $file, name, kind, line_start, line_end, method_count: (.methods | length)}
'
```

2. Try all three searches (exact class, exact method, fuzzy class) and combine results.

3. Present findings clearly:

```
Found: UserService
  📁 lib/services/user_service.dart
  Kind:   class
  Lines:  8–47
  Methods (4):
    fetchUser     lines 15–22
    deleteUser    lines 24–31
    updateUser    lines 33–41
    get name      lines 43–43

To read a method:
  sed -n '15,22p' lib/services/user_service.dart
```

4. If nothing found, suggest:
   - Checking spelling
   - Running `/index` first to see what's available
   - Searching for a partial name

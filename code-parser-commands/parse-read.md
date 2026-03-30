---
name: parse-read
description: Read a specific class or method body from a source file using its line range from the code-parser index. Much cheaper than reading the entire file. Use after /index or /parse-find to fetch only the code you need.
argument-hint: <file> <line_start> <line_end>  OR  <file> <ClassName> [MethodName]
allowed-tools: Bash(sed *), Bash(awk *), Bash(code-parser *), Bash(jq *), Bash(wc *)
---

Read targeted lines from a source file. Arguments: `$ARGUMENTS`

## Parse the arguments

**Format A — line numbers:**  `file.dart 15 22`
Read lines 15–22 of file.dart directly.

**Format B — class name:**  `file.dart UserService`
Run code-parser on the file, find the class, read its full line range.

**Format C — class + method:**  `file.dart UserService fetchUser`
Run code-parser, find the specific method inside the class, read just that method.

**Format D — method only:**  `file.dart fetchUser`
Search for any method named fetchUser across all classes in the file.

## Execution

**For Format A:**
```bash
sed -n 'START,ENDp' FILE
```

**For Formats B/C/D — look up line range first:**
```bash
# Get the line range from the index
code-parser FILE --format json | jq '
  .[0].classes[] |
  select(.name == "CLASSNAME") |
  {line_start, line_end, methods}
'

# Then read those lines
sed -n 'line_start,line_endp' FILE
```

## Rules

- Never read more than 120 lines in a single fetch unless the user explicitly asks
- If a class is longer than 120 lines, read the class header (first 20 lines) and list the methods — ask which method body to read
- Always show the file path and line range before the code:

```
📄 lib/services/user_service.dart  (lines 15–22)
─────────────────────────────────────────────
  Future<User> fetchUser(String id) async {
    final response = await _client.get('/users/$id');
    if (response.statusCode != 200) {
      throw ApiException('User not found: $id');
    }
    return User.fromJson(response.body);
  }
─────────────────────────────────────────────
```

- After showing the code, state the token cost saved vs reading the full file:
  `(read 8 lines of 342-line file — saved ~2,600 tokens)`

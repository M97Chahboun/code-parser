# code-parser Output Schema Reference

## Top-level structure

The tool always returns a JSON array. Each element is one parsed file.

```
Array<FileResult>
```

## FileResult

```typescript
{
  file: string        // relative path of the source file
  language: string    // "dart" | "python" | "typescript"
  classes: Array<ClassInfo>
}
```

## ClassInfo

```typescript
{
  name: string        // class / interface / mixin / enum name
  kind: string        // see Kind values below
  line_start: number  // 1-based, first line of the declaration keyword
  line_end: number    // 1-based, last line of the closing brace
  methods: Array<MethodInfo>
}
```

### Kind values

| Kind | Languages |
|---|---|
| `"class"` | Dart, Python, TypeScript |
| `"abstract class"` | Dart, TypeScript |
| `"interface"` | TypeScript |
| `"mixin"` | Dart |
| `"extension"` | Dart |
| `"enum"` | Dart, TypeScript |

## MethodInfo

```typescript
{
  name: string        // method / constructor / getter / setter name
  line_start: number  // 1-based, first line of the method signature
  line_end: number    // 1-based, last line of the closing brace (or semicolon)
}
```

### Method name conventions

| Pattern | Meaning |
|---|---|
| `"MyClass"` | Constructor (same name as class) |
| `"get displayName"` | Dart/TS getter |
| `"set value"` | Dart/TS setter |
| `"__init__"` | Python constructor |
| `"__str__"`, etc. | Python dunder methods |
| `"_privateMethod"` | Private method (Dart/Python convention) |

---

## Full example

```json
[
  {
    "file": "lib/auth/auth_service.dart",
    "language": "dart",
    "classes": [
      {
        "name": "AuthService",
        "kind": "class",
        "line_start": 5,
        "line_end": 68,
        "methods": [
          { "name": "AuthService",   "line_start": 10, "line_end": 10 },
          { "name": "login",         "line_start": 13, "line_end": 28 },
          { "name": "logout",        "line_start": 30, "line_end": 38 },
          { "name": "get isLoggedIn","line_start": 40, "line_end": 40 },
          { "name": "refreshToken",  "line_start": 43, "line_end": 55 }
        ]
      },
      {
        "name": "AuthState",
        "kind": "enum",
        "line_start": 71,
        "line_end": 76,
        "methods": []
      }
    ]
  },
  {
    "file": "lib/auth/token_store.dart",
    "language": "dart",
    "classes": [
      {
        "name": "TokenStore",
        "kind": "class",
        "line_start": 3,
        "line_end": 42,
        "methods": [
          { "name": "save",   "line_start": 8,  "line_end": 14 },
          { "name": "load",   "line_start": 16, "line_end": 22 },
          { "name": "clear",  "line_start": 24, "line_end": 28 }
        ]
      }
    ]
  }
]
```

---

## Useful jq recipes

```bash
# List all class names across all files
code-parser ./lib | jq '[.[].classes[].name] | sort'

# Find which file defines a class
code-parser ./lib | jq '.[] | select(.classes[].name == "AuthService") | .file'

# Count methods per class
code-parser ./lib | jq '.[].classes[] | {name, method_count: (.methods | length)}'

# Find all methods matching a pattern
code-parser ./lib | jq '[.[].classes[].methods[] | select(.name | contains("User"))]'

# Get the line range of a specific method
code-parser ./lib | jq '
  .[].classes[]
  | select(.name == "AuthService")
  | .methods[]
  | select(.name == "login")
  | {line_start, line_end}
'

# Show files with no classes (scripts, config files)
code-parser ./lib | jq '.[] | select(.classes | length == 0) | .file'

# Get total method count across project
code-parser ./lib | jq '[.[].classes[].methods[]] | length'
```

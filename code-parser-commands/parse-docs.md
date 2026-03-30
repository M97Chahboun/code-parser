---
name: parse-docs
description: Extract and display documentation (docstrings, JSDoc, Dart doc comments) from classes and methods using code-parser. Returns structured documentation without code bodies.
argument-hint: [path] [ClassName] [MethodName]
allowed-tools: Bash(code-parser *), Bash(jq *), Bash(ls *)
---

Extract documentation from the codebase at `$ARGUMENTS` using code-parser.

## Parse the arguments

- **No arguments**: Extract all docs from current directory
- **One argument (path)**: Extract all docs from specified path
- **Two arguments (path ClassName)**: Extract docs for specific class
- **Three arguments (path ClassName MethodName)**: Extract docs for specific method

## Steps

### 1. Run code-parser and extract docs

```bash
# All docs from path
code-parser ${PATH:-.} --format json | jq '
  .[] | {
    file: .file,
    language: .language,
    classes: [.classes[] | select(.doc != null) | {
      name: .name,
      kind: .kind,
      doc: .doc,
      methods: [.methods[] | select(.doc != null) | {name, doc}]
    }]
  } | select(.classes | length > 0)
'

# Specific class docs
code-parser FILE --format json | jq --arg class "$CLASSNAME" '
  .[0] | {
    file: .file,
    class: (.classes[] | select(.name == $class) | {
      name: .name,
      kind: .kind,
      doc: .doc,
      methods: [.methods[] | select(.doc != null) | {name, doc}]
    })
  }
'

# Specific method doc
code-parser FILE --format json | jq --arg class "$CLASSNAME" --arg method "$METHODNAME" '
  .[0].classes[] | select(.name == $class) |
  .methods[] | select(.name == $method) |
  {class: .name, method: .name, doc: .doc}
'
```

### 2. Present findings clearly

Format the output as readable documentation:

```
📁 lib/services/user_service.dart (dart)

╔═══ UserService [class] ═════════════════════════════════════════╗
║ Handles all user-related operations including                  ║
║ fetching, updating, and deleting users from the API.           ║
║                                                                 ║
║ Methods with documentation:                                    ║
║   • UserService()                                              ║
║     Creates a new UserService instance                         ║
║                                                                 ║
║   • fetchUser(id: String)                                      ║
║     Fetches a user by ID from the remote API.                  ║
║     Throws [ApiException] if user not found.                   ║
║                                                                 ║
║   • deleteUser(id: String)                                     ║
║     Permanently deletes a user account.                        ║
╚════════════════════════════════════════════════════════════════╝

📁 lib/models/user.dart (dart)

╔═══ User [class] ════════════════════════════════════════════════╗
║ Data model representing a user in the system.                  ║
║                                                                 ║
║ Methods with documentation:                                    ║
║   • fromJson(json)                                             ║
║     Creates a User from JSON map                               ║
║                                                                 ║
║   • toJson()                                                   ║
║     Converts User to JSON map                                  ║
╚════════════════════════════════════════════════════════════════╝

Summary: 2 files · 2 classes · 5 documented methods
```

### 3. Handle edge cases

- **No documentation found**: Report "No documentation comments found in specified path"
- **Class not found**: Report "Class 'ClassName' not found in FILE"
- **Method not found**: Report "Method 'MethodName' not found in class 'ClassName'"

### 4. Optional: Generate API documentation

For generating markdown documentation:

```bash
code-parser ./lib --format json | jq -r '
  .[] | 
  "# " + .file + "\n\n" +
  (.classes[] | 
    "## " + .name + " (" + .kind + ")\n\n" +
    (.doc // "No documentation") + "\n\n" +
    "### Methods\n\n" +
    (.methods[] | 
      "#### " + .name + "\n\n" + 
      (.doc // "No documentation") + "\n"
    )
  )
' > API_DOCS.md
```

## Usage examples

```bash
# Extract all documentation from project
/parse-docs ./lib

# Get docs for specific class
/parse-docs ./lib UserService

# Get docs for specific method
/parse-docs ./lib UserService fetchUser

# Generate markdown API documentation
code-parser ./src --format json | jq -r '...' > docs/API.md
```

## Notes

- Python: Extracts triple-quoted docstrings (`"""..."""` or `'''...'''`)
- TypeScript: Extracts JSDoc comments (`/** ... */`)
- Dart: Extracts doc comments (`///` or `/** ... */`)
- Only classes/methods with documentation are included in output
- Empty docstrings are treated as no documentation

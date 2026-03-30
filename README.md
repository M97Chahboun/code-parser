# code-parser

> **LLM-optimised code structure extractor** — Dart · Python · TypeScript · Rust · v0.1.0

A fast Rust CLI that statically analyses source files and returns a structured JSON map of every **class, interface, mixin, enum, and method** — together with its **exact line range** — without executing the code.

---

## Why This Exists: Reducing LLM Token Consumption

When an LLM agent needs to understand or edit a codebase, the naive approach is to pass entire files into the context window. For a typical Flutter or Django project this means **50,000 – 200,000 tokens per request**, most of which is irrelevant boilerplate.

`code-parser` solves this by giving the model a **precise, structured index** of the codebase first. The model then requests **only the specific line ranges** it actually needs.

```
A 2,000-line Dart file  →  ~15,000 tokens to read in full
Same file via code-parser →  ~400 tokens index + ~300 per method read
                                                 ↑ 97% reduction
```

| | Without code-parser | With code-parser |
|---|---|---|
| **Tokens per file** | ~15,000 | ~400 index + surgical reads |
| **Context content** | All boilerplate | Only relevant code |
| **Scalability** | Hits limits fast | Works on 100k+ line codebases |
| **Hallucination risk** | High (guessing structure) | Low (grounded in exact line numbers) |
| **Cost** | High per request | Low — index once, read cheaply |

---

## How LLMs Use It

The intended workflow is a **two-phase read pattern**:

### Phase 1 — Index (cheap)

1. Run `code-parser` on the project root or changed files
2. Inject the JSON output into the LLM system prompt or tool-call result
3. The model now knows every class name, method name, and line range — **without reading a single method body**

### Phase 2 — Targeted read (surgical)

1. The model identifies the exact methods relevant to the task (e.g. `UserService.createUser` lines 27–32)
2. It requests only those line ranges from the file system
3. It reads, reasons about, and edits those ~10–50 lines — not the entire 400-line file

---

## Supported Languages

| Language | Extension | Parser backend | Detects |
|---|---|---|---|
| **Dart** | `.dart` | Hand-rolled tokeniser | `class`, `abstract class`, `mixin`, `extension`, `enum` |
| **Python** | `.py` | tree-sitter-python 0.21 | `class` (all method types incl. `@decorator`) |
| **TypeScript** | `.ts`, `.tsx` | tree-sitter-typescript 0.21 | `class`, `abstract class`, `interface`, `enum` |

For each type the tool extracts: `name`, `kind`, `line_start`, `line_end`, and an array of `methods` — each with their own line range.

---

## Installation

### Install with Cargo

```bash
cargo install code-parser
```

### Install with curl

```bash
curl -fsSL https://raw.githubusercontent.com/m97chahboun/code-parser/main/install.sh | bash
```

### Build from source

**Requirements:**

- Rust 1.75+ (1.77+ recommended)
- Cargo (bundled with Rust via rustup)

```bash
git clone https://github.com/m97chahboun/code-parser
cd code-parser
cargo build --release
# binary at: ./target/release/code-parser
```

### Quick verify

```bash
echo 'class Foo:\n    def bar(self): pass' > test.py
./target/release/code-parser test.py --format pretty
```

---

## Usage

```
code-parser [OPTIONS] [PATH]

ARGS:
  [PATH]   File or directory to analyse  [default: .]

OPTIONS:
  -f, --format <FORMAT>   Output format: json | pretty  [default: json]
  -q, --quiet             Suppress JSON output (errors only)
  -h, --help              Print help
  -V, --version           Print version
```

### Examples

```bash
# Parse a single file — pretty JSON
code-parser src/user_service.dart --format pretty

# Parse entire project — compact JSON (pipe to jq)
code-parser ./my_project | jq '.[].classes[] | {name, line_start, line_end}'

# Only show class names and method counts
code-parser ./src | jq '.[].classes[] | "\(.name): \(.methods | length) methods"'

# Use in a shell script / LLM agent pipeline
INDEX=$(code-parser ./lib --format json)
echo "$INDEX" | your-llm-agent --task refactor
```

---

## Output Format

The tool always emits a JSON array — one element per parsed file.

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
          { "name": "UserService",   "line_start": 12, "line_end": 12 },
          { "name": "fetchUser",     "line_start": 15, "line_end": 22 },
          { "name": "get displayName", "line_start": 24, "line_end": 24 },
          { "name": "deleteUser",    "line_start": 26, "line_end": 33 }
        ]
      }
    ]
  }
]
```

### Field reference

| Field | Type | Description |
|---|---|---|
| `file` | string | Relative path of the source file |
| `language` | string | `dart` \| `python` \| `typescript` |
| `classes` | array | All top-level types found in the file |
| `name` | string | Class / interface / enum / mixin name |
| `kind` | string | `class`, `abstract class`, `interface`, `mixin`, `extension`, `enum` |
| `line_start` | number (1-based) | First line of the type declaration |
| `line_end` | number (1-based) | Last line of the closing brace |
| `methods` | array | All methods / constructors / getters / setters |

---

## LLM Integration Guide

### System prompt injection

Feed the JSON index into the model once per session or when files change:

```python
import subprocess, json

index = subprocess.check_output(["code-parser", "./lib", "--format", "json"])

system_prompt = f"""
You are a coding assistant. Here is the codebase structure:

{index.decode()}

When you need to read a method body, ask for its line range.
Do not guess — always check the index for exact locations.
"""
```

### Tool / function call pattern

Define a `read_lines(file, start, end)` tool alongside the index. The model calls it only for lines it needs:

```python
def read_lines(file: str, start: int, end: int) -> str:
    with open(file) as f:
        lines = f.readlines()
    return "".join(lines[start - 1 : end])

tools = [
    {
        "name": "read_lines",
        "description": "Read specific lines from a source file.",
        "parameters": {
            "file":  {"type": "string"},
            "start": {"type": "integer", "description": "1-based start line"},
            "end":   {"type": "integer", "description": "1-based end line (inclusive)"}
        }
    }
]
```

### Recommended agent prompt fragment

```
You have access to a code index (code-parser JSON) and a read_lines tool.

Strategy:
  1. Consult the index to locate the class and method you need.
  2. Call read_lines with the exact line_start / line_end from the index.
  3. Never request more than 80 lines at once.
  4. Prefer reading method bodies one at a time.
  5. Do not hallucinate method names — only use names present in the index.
```

---

## Project Structure

```
code-parser/
├── Cargo.toml          # Dependencies (pinned for Rust 1.75 compat)
├── Cargo.lock          # Reproducible builds
└── src/
    └── main.rs         # All parser logic (~500 lines)
        ├── CLI         (clap derive)
        ├── Python      (tree-sitter-python)
        ├── TypeScript  (tree-sitter-typescript)
        └── Dart        (hand-rolled tokeniser — no ABI dependency)
```

---

## Technical Notes

### Why a hand-rolled Dart tokeniser?

The available `tree-sitter-dart` crate targets tree-sitter ABI 15 (released with tree-sitter 0.23), while the Rust 1.75-compatible tree-sitter crate caps at ABI 14. Rather than requiring users to install a newer compiler, the Dart parser uses a purpose-built tokeniser that handles all real-world constructs: comments, string literals (including raw strings and triple-quoted strings), annotations, generics, getters, setters, constructors, and nested classes.

### Nested class handling

Methods belonging to inner classes are not attributed to the outer class. The depth-tracking logic in each extractor ensures correct scoping at all nesting levels.

### Performance

The release binary processes a 10,000-line TypeScript file in under 5 ms on modern hardware. Recursive directory walks are I/O-bound; the parser itself is not the bottleneck.

---

## Limitations

- Anonymous classes (e.g. Dart object expressions) are labelled `<anonymous>`
- TypeScript `type` aliases (`type Foo = ...`) are not extracted — only `class` / `interface` / `enum`
- Python dataclasses and `NamedTuple` subclasses are detected as regular classes; fields are not listed as methods
- Dart extension methods on unnamed extensions are skipped
- Files with syntax errors are silently skipped (exit code 1 if all files fail)

---

## Contributing

Pull requests are welcome. When adding a new language:

- **Python / TypeScript path** — add a tree-sitter grammar crate and implement an extractor following the pattern in `extract_python()`
- **Dart / other** — if no ABI-compatible grammar exists, extend the hand-rolled tokeniser approach
- All extractors must pass tests covering: basic class, nested class, getters/setters, decorators, and empty files

```bash
cargo test      # run all tests
cargo clippy    # lint
cargo fmt       # format
```

---

## License

MIT License — see [LICENSE](LICENSE) for details.

Built with [tree-sitter](https://tree-sitter.github.io), [clap](https://docs.rs/clap), [serde_json](https://docs.rs/serde_json), and [walkdir](https://docs.rs/walkdir).

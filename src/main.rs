use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser as TsParser};
use walkdir::WalkDir;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[clap(
    name = "code-parser",
    about = "Extract classes/objects and methods with line numbers from Dart, Python, TypeScript",
    version
)]
struct Cli {
    /// File or directory to parse
    #[clap(default_value = ".")]
    path: PathBuf,

    /// Output format: json (default) or pretty
    #[clap(short, long, default_value = "json")]
    format: String,

    /// Suppress normal output
    #[clap(short, long)]
    quiet: bool,
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub kind: String,
    pub line_start: usize,
    pub line_end: usize,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileResult {
    pub file: String,
    pub language: String,
    pub classes: Vec<ClassInfo>,
}

// ── Language detection ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Lang {
    Dart,
    Python,
    TypeScript,
}

fn detect_language(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "dart" => Some(Lang::Dart),
        "py"   => Some(Lang::Python),
        "ts" | "tsx" => Some(Lang::TypeScript),
        _ => None,
    }
}

fn lang_name(lang: Lang) -> &'static str {
    match lang { Lang::Dart => "dart", Lang::Python => "python", Lang::TypeScript => "typescript" }
}

// ── tree-sitter helpers ───────────────────────────────────────────────────────

fn node_lines(node: Node) -> (usize, usize) {
    (node.start_position().row + 1, node.end_position().row + 1)
}

fn child_text<'a>(node: Node<'a>, kind: &str, source: &'a [u8]) -> Option<String> {
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if child.kind() == kind {
            return child.utf8_text(source).ok().map(String::from);
        }
    }
    None
}

fn find_all<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind { out.push(node); }
    let mut c = node.walk();
    for child in node.children(&mut c) { find_all(child, kind, out); }
}

fn find_first_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind { return Some(node); }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        if let Some(f) = find_first_kind(child, kind) { return Some(f); }
    }
    None
}

fn is_nested_class_member(node: Node, ancestor: Node) -> bool {
    let class_like = [
        "class_definition","class_declaration","abstract_class_declaration",
        "interface_declaration","enum_declaration","mixin_declaration","extension_declaration",
    ];
    let mut p = node.parent();
    while let Some(n) = p {
        if n.id() == ancestor.id() { return false; }
        if class_like.contains(&n.kind()) { return true; }
        p = n.parent();
    }
    false
}

// ── Python extractor ──────────────────────────────────────────────────────────

fn extract_python(root: Node, source: &[u8]) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    let mut class_nodes = Vec::new();
    find_all(root, "class_definition", &mut class_nodes);
    for cn in class_nodes {
        let name = child_text(cn, "identifier", source).unwrap_or_else(|| "<anonymous>".into());
        let (ls, le) = node_lines(cn);
        let mut methods = Vec::new();
        if let Some(body) = cn.child_by_field_name("body") {
            let mut cur = body.walk();
            for child in body.children(&mut cur) {
                let fn_node = if child.kind() == "decorated_definition" {
                    find_first_kind(child, "function_definition").unwrap_or(child)
                } else if child.kind() == "function_definition" { child
                } else { continue };
                let fname = child_text(fn_node, "identifier", source).unwrap_or_else(|| "<fn>".into());
                let (fls, fle) = node_lines(fn_node);
                methods.push(MethodInfo { name: fname, line_start: fls, line_end: fle });
            }
        }
        classes.push(ClassInfo { name, kind: "class".into(), line_start: ls, line_end: le, methods });
    }
    classes
}

// ── TypeScript extractor ──────────────────────────────────────────────────────

fn extract_typescript(root: Node, source: &[u8]) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    for (nk, dk) in &[
        ("class_declaration","class"), ("abstract_class_declaration","abstract class"),
        ("interface_declaration","interface"), ("enum_declaration","enum"),
    ] {
        let mut nodes = Vec::new();
        find_all(root, nk, &mut nodes);
        for cn in nodes {
            let name = child_text(cn, "type_identifier", source)
                .or_else(|| child_text(cn, "identifier", source))
                .unwrap_or_else(|| "<anonymous>".into());
            let (ls, le) = node_lines(cn);
            let methods = extract_ts_methods(cn, source);
            classes.push(ClassInfo { name, kind: dk.to_string(), line_start: ls, line_end: le, methods });
        }
    }
    classes
}

fn extract_ts_methods(cn: Node, source: &[u8]) -> Vec<MethodInfo> {
    let mut methods = Vec::new();
    for kind in &["method_definition","method_signature","public_field_definition"] {
        let mut nodes = Vec::new();
        find_all(cn, kind, &mut nodes);
        for m in nodes {
            if is_nested_class_member(m, cn) { continue; }
            if kind == &"public_field_definition" {
                let is_fn = m.child_by_field_name("value")
                    .map(|v| matches!(v.kind(), "arrow_function"|"function"|"function_expression"))
                    .unwrap_or(false);
                if !is_fn { continue; }
            }
            let name = child_text(m, "property_identifier", source)
                .or_else(|| child_text(m, "identifier", source))
                .unwrap_or_else(|| kind.to_string());
            let (ls, le) = node_lines(m);
            methods.push(MethodInfo { name, line_start: ls, line_end: le });
        }
    }
    methods.sort_by_key(|m| m.line_start);
    methods.dedup_by_key(|m| m.line_start);
    methods
}

// ── Dart hand-rolled parser ───────────────────────────────────────────────────
//
// Dart can't use tree-sitter here (ABI mismatch between available grammars and
// the Rust 1.75-compatible tree-sitter 0.22). Instead we use a robust
// line-by-line tokeniser that handles:
//   • class / abstract class / mixin / extension / enum declarations
//   • constructor, method, getter, setter definitions
//   • nested classes (skipped correctly via brace depth tracking)
//   • single-line comments, multi-line comments, string literals
//   • arrow functions and block functions

#[derive(Debug)]
struct DartToken {
    kind: DartTokenKind,
    value: String,
    line: usize,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
enum DartTokenKind {
    Keyword,
    Identifier,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Semicolon,
    Arrow,   // =>
    At,      // @
    Other,
}

fn dart_tokenize(source: &str) -> Vec<DartToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut line = 1usize;

    while i < chars.len() {
        // Track newlines
        if chars[i] == '\n' { line += 1; i += 1; continue; }
        if chars[i] == '\r' { i += 1; continue; }

        // Single-line comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i+1] == '/' {
            while i < chars.len() && chars[i] != '\n' { i += 1; }
            continue;
        }

        // Multi-line comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i+1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i+1] == '/') {
                if chars[i] == '\n' { line += 1; }
                i += 1;
            }
            i += 2;
            continue;
        }

        // String literals (skip contents)
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            // Check for triple-quote
            let triple = i + 2 < chars.len() && chars[i+1] == quote && chars[i+2] == quote;
            if triple {
                i += 3;
                while i + 2 < chars.len() && !(chars[i] == quote && chars[i+1] == quote && chars[i+2] == quote) {
                    if chars[i] == '\n' { line += 1; }
                    i += 1;
                }
                i += 3;
            } else {
                i += 1;
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' { i += 1; } // skip escape
                    if chars[i] == '\n' { line += 1; }
                    i += 1;
                }
                i += 1;
            }
            continue;
        }

        // Raw string r"..." or r'...'
        if chars[i] == 'r' && i + 1 < chars.len() && (chars[i+1] == '"' || chars[i+1] == '\'') {
            let quote = chars[i+1];
            i += 2;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\n' { line += 1; }
                i += 1;
            }
            i += 1;
            continue;
        }

        // Whitespace
        if chars[i].is_whitespace() { i += 1; continue; }

        // Arrow =>
        if i + 1 < chars.len() && chars[i] == '=' && chars[i+1] == '>' {
            tokens.push(DartToken { kind: DartTokenKind::Arrow, value: "=>".into(), line });
            i += 2; continue;
        }

        // Single-char tokens
        match chars[i] {
            '{' => { tokens.push(DartToken { kind: DartTokenKind::LBrace, value: "{".into(), line }); i += 1; continue; }
            '}' => { tokens.push(DartToken { kind: DartTokenKind::RBrace, value: "}".into(), line }); i += 1; continue; }
            '(' => { tokens.push(DartToken { kind: DartTokenKind::LParen, value: "(".into(), line }); i += 1; continue; }
            ')' => { tokens.push(DartToken { kind: DartTokenKind::RParen, value: ")".into(), line }); i += 1; continue; }
            ';' => { tokens.push(DartToken { kind: DartTokenKind::Semicolon, value: ";".into(), line }); i += 1; continue; }
            '@' => { tokens.push(DartToken { kind: DartTokenKind::At, value: "@".into(), line }); i += 1; continue; }
            _ => {}
        }

        // Identifiers / keywords
        if chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = match word.as_str() {
                "class"|"abstract"|"mixin"|"extension"|"enum"|"void"|"return"|
                "get"|"set"|"async"|"await"|"static"|"final"|"const"|"late"|
                "required"|"factory"|"external"|"operator"|"new"|"this"|"super"|
                "if"|"else"|"for"|"while"|"do"|"switch"|"case"|"break"|"continue"|
                "import"|"export"|"library"|"part"|"show"|"hide"|"typedef"|
                "is"|"as"|"in"|"var"|"dynamic"|"implements"|"extends"|"with"|"on" =>
                    DartTokenKind::Keyword,
                _ => DartTokenKind::Identifier,
            };
            tokens.push(DartToken { kind, value: word, line });
            continue;
        }

        // Everything else: skip
        i += 1;
    }

    tokens
}

fn extract_dart_hand_rolled(source: &str) -> Vec<ClassInfo> {
    let tokens = dart_tokenize(source);
    let mut classes: Vec<ClassInfo> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        // Look for class/mixin/extension/enum/abstract class declarations
        let (class_kind, name_idx) = detect_class_decl(&tokens, i);
        if let Some(kind_str) = class_kind {
            if let Some(ni) = name_idx {
                let class_name = tokens[ni].value.clone();
                let decl_line = tokens[i].line;

                // Find opening brace, skipping extends/implements/with clauses
                // and any generic type parameters <...>
                let brace_idx = find_opening_brace(&tokens, ni + 1);
                if let Some(bi) = brace_idx {
                    let (methods, end_line) = extract_dart_class_body(&tokens, bi);
                    classes.push(ClassInfo {
                        name: class_name,
                        kind: kind_str,
                        line_start: decl_line,
                        line_end: end_line,
                        methods,
                    });
                    i = bi; // continue from opening brace (body scan advances past it)
                }
            }
        }
        i += 1;
    }

    classes
}

fn detect_class_decl(tokens: &[DartToken], i: usize) -> (Option<String>, Option<usize>) {
    let t = &tokens[i];

    // enum Foo
    if t.kind == DartTokenKind::Keyword && t.value == "enum" {
        if i + 1 < tokens.len() && tokens[i+1].kind == DartTokenKind::Identifier {
            return (Some("enum".into()), Some(i + 1));
        }
    }

    // mixin Foo
    if t.kind == DartTokenKind::Keyword && t.value == "mixin" {
        if i + 1 < tokens.len() && tokens[i+1].kind == DartTokenKind::Identifier {
            return (Some("mixin".into()), Some(i + 1));
        }
    }

    // extension Foo / extension on Foo
    if t.kind == DartTokenKind::Keyword && t.value == "extension" {
        if i + 1 < tokens.len() {
            if tokens[i+1].kind == DartTokenKind::Identifier {
                return (Some("extension".into()), Some(i + 1));
            }
        }
    }

    // abstract class Foo
    if t.kind == DartTokenKind::Keyword && t.value == "abstract" {
        if i + 1 < tokens.len() && tokens[i+1].value == "class" {
            if i + 2 < tokens.len() && tokens[i+2].kind == DartTokenKind::Identifier {
                return (Some("abstract class".into()), Some(i + 2));
            }
        }
    }

    // class Foo
    if t.kind == DartTokenKind::Keyword && t.value == "class" {
        if i + 1 < tokens.len() && tokens[i+1].kind == DartTokenKind::Identifier {
            return (Some("class".into()), Some(i + 1));
        }
    }

    (None, None)
}

fn find_opening_brace(tokens: &[DartToken], from: usize) -> Option<usize> {
    let _depth = 0i32;
    let mut j = from;
    while j < tokens.len() {
        if tokens[j].kind == DartTokenKind::LBrace { return Some(j); }
        if tokens[j].kind == DartTokenKind::Semicolon { return None; } // forward decl
        j += 1;
    }
    None
}

/// Parse the body of a class starting at the `{` token.
/// Returns (methods, end_line_of_closing_brace).
fn extract_dart_class_body(tokens: &[DartToken], open_brace: usize) -> (Vec<MethodInfo>, usize) {
    let mut methods = Vec::new();
    let mut depth = 1i32;
    let mut i = open_brace + 1;
    let mut end_line = tokens[open_brace].line;

    while i < tokens.len() && depth > 0 {
        match tokens[i].kind {
            DartTokenKind::LBrace => { depth += 1; i += 1; continue; }
            DartTokenKind::RBrace => {
                depth -= 1;
                end_line = tokens[i].line;
                i += 1;
                continue;
            }
            _ => {}
        }

        if depth != 1 { i += 1; continue; } // inside nested block

        // Skip annotations (@override, @required, etc.)
        if tokens[i].kind == DartTokenKind::At {
            i += 1;
            if i < tokens.len() && tokens[i].kind == DartTokenKind::Identifier { i += 1; }
            continue;
        }

        // Try to detect method/constructor/getter/setter
        if let Some((method_name, start_line, end_i)) = detect_dart_member(tokens, i) {
            // Find the end line — either at semicolon or matching brace
            let end = find_member_end(tokens, end_i);
            methods.push(MethodInfo { name: method_name, line_start: start_line, line_end: end.0 });
            i = end.1;
            continue;
        }

        i += 1;
    }

    (methods, end_line)
}

/// Returns (name, start_line, idx_after_signature) if a method/ctor/getter/setter is detected.
fn detect_dart_member(tokens: &[DartToken], i: usize) -> Option<(String, usize, usize)> {
    if i >= tokens.len() { return None; }

    let mut j = i;
    let start_line = tokens[j].line;

    // Skip modifiers: static, final, const, late, external, factory, async, operator
    let modifiers = ["static","final","const","late","external","factory","async","override","abstract"];
    while j < tokens.len() && (tokens[j].kind == DartTokenKind::Keyword && modifiers.contains(&tokens[j].value.as_str())) {
        j += 1;
    }

    if j >= tokens.len() { return None; }

    // get/set keyword (getter/setter)
    if tokens[j].value == "get" || tokens[j].value == "set" {
        let accessor = tokens[j].value.clone();
        j += 1;
        if j < tokens.len() && tokens[j].kind == DartTokenKind::Identifier {
            let name = format!("{} {}", accessor, tokens[j].value);
            return Some((name, start_line, j + 1));
        }
        return None;
    }

    // Return type (optional) then name then (  — method
    // OR: name then (  — constructor
    // Pattern: [Type] name (
    // We look ahead for: identifier '(' at depth 0

    // collect identifiers/keywords before '('
    let mut parts: Vec<String> = Vec::new();
    let mut k = j;
    while k < tokens.len() {
        match tokens[k].kind {
            DartTokenKind::LParen => {
                // Last identifier before '(' is the method/ctor name
                if let Some(name) = parts.last().cloned() {
                    return Some((name, start_line, k));
                }
                return None;
            }
            DartTokenKind::LBrace | DartTokenKind::Semicolon | DartTokenKind::RBrace => {
                return None;
            }
            DartTokenKind::Identifier => {
                parts.push(tokens[k].value.clone());
            }
            DartTokenKind::Keyword => {
                // keywords like void, dynamic can be return types
                if ["void","dynamic","bool","int","double","String","List","Map","Set","Future","Stream"].contains(&tokens[k].value.as_str()) {
                    parts.push(tokens[k].value.clone());
                } else {
                    return None;
                }
            }
            _ => { k += 1; continue; }
        }
        k += 1;
    }
    None
}

/// Find the end of a member (method body or semicolon).
/// Returns (end_line, next_token_index).
fn find_member_end(tokens: &[DartToken], from: usize) -> (usize, usize) {
    let mut i = from;
    let mut depth = 0i32;

    while i < tokens.len() {
        match tokens[i].kind {
            DartTokenKind::LBrace => { depth += 1; }
            DartTokenKind::RBrace => {
                if depth == 0 {
                    // Hit the class closing brace — don't consume it
                    return (tokens[i].line, i);
                }
                depth -= 1;
                if depth == 0 {
                    return (tokens[i].line, i + 1);
                }
            }
            DartTokenKind::Semicolon => {
                if depth == 0 {
                    return (tokens[i].line, i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    let last_line = tokens.last().map(|t| t.line).unwrap_or(1);
    (last_line, i)
}

// ── Core parse ────────────────────────────────────────────────────────────────

fn parse_file(path: &Path) -> Option<FileResult> {
    let lang = detect_language(path)?;
    let source_bytes = std::fs::read(path).ok()?;
    let source_str = std::str::from_utf8(&source_bytes).ok()?;

    let classes = match lang {
        Lang::Dart => extract_dart_hand_rolled(source_str),
        Lang::Python => {
            let mut parser = TsParser::new();
            parser.set_language(&tree_sitter_python::language()).ok()?;
            let tree = parser.parse(&source_bytes, None)?;
            extract_python(tree.root_node(), &source_bytes)
        }
        Lang::TypeScript => {
            let mut parser = TsParser::new();
            parser.set_language(&tree_sitter_typescript::language_typescript()).ok()?;
            let tree = parser.parse(&source_bytes, None)?;
            extract_typescript(tree.root_node(), &source_bytes)
        }
    };

    Some(FileResult {
        file: path.display().to_string(),
        language: lang_name(lang).into(),
        classes,
    })
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let mut results: Vec<FileResult> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let path = &cli.path;
    if path.is_file() {
        match parse_file(path) {
            Some(r) => results.push(r),
            None => errors.push(format!("Skipped: {}", path.display())),
        }
    } else {
        for entry in WalkDir::new(path)
            .follow_links(true).into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let p = entry.path();
            if detect_language(p).is_none() { continue; }
            match parse_file(p) {
                Some(r) => results.push(r),
                None => errors.push(format!("Failed: {}", p.display())),
            }
        }
    }

    if !cli.quiet {
        let out = match cli.format.as_str() {
            "pretty" => serde_json::to_string_pretty(&results),
            _ => serde_json::to_string(&results),
        }.expect("serialisation failed");
        println!("{}", out);
    }

    for e in &errors { eprintln!("warning: {}", e); }
    if !errors.is_empty() && results.is_empty() { std::process::exit(1); }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_classes() {
        let src = b"class Foo:\n    def bar(self):\n        pass\n    def baz(self):\n        pass\n";
        let mut parser = TsParser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        let classes = extract_python(tree.root_node(), src);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Foo");
        assert_eq!(classes[0].methods.len(), 2);
        assert_eq!(classes[0].methods[0].name, "bar");
        assert_eq!(classes[0].methods[1].name, "baz");
    }

    #[test]
    fn test_typescript_interface() {
        let src = b"interface Repo {\n  find(id: string): void;\n  save(x: any): void;\n}\n";
        let mut parser = TsParser::new();
        parser.set_language(&tree_sitter_typescript::language_typescript()).unwrap();
        let tree = parser.parse(src, None).unwrap();
        let classes = extract_typescript(tree.root_node(), src);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Repo");
        assert_eq!(classes[0].kind, "interface");
        assert_eq!(classes[0].methods.len(), 2);
    }

    #[test]
    fn test_dart_class() {
        let src = r#"
class Animal {
  String name;
  Animal(this.name);
  String speak() => 'hello';
  void sleep() {
    print('zzz');
  }
}
"#;
        let classes = extract_dart_hand_rolled(src);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Animal");
        assert_eq!(classes[0].kind, "class");
        let names: Vec<&str> = classes[0].methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Animal"), "constructor missing: {:?}", names);
        assert!(names.contains(&"speak"), "speak missing: {:?}", names);
        assert!(names.contains(&"sleep"), "sleep missing: {:?}", names);
    }

    #[test]
    fn test_dart_abstract_and_mixin() {
        let src = r#"
abstract class Shape {
  double area();
  double perimeter();
}
mixin Serializable {
  String toJson();
}
"#;
        let classes = extract_dart_hand_rolled(src);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].kind, "abstract class");
        assert_eq!(classes[1].kind, "mixin");
    }

    #[test]
    fn test_dart_getters_setters() {
        let src = r#"
class Box {
  int _val = 0;
  int get value => _val;
  set value(int v) { _val = v; }
}
"#;
        let classes = extract_dart_hand_rolled(src);
        assert_eq!(classes.len(), 1);
        let names: Vec<&str> = classes[0].methods.iter().map(|m| m.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("value")), "getter/setter missing: {:?}", names);
    }
}

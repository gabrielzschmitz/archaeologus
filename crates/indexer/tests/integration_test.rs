//! Integration tests for archaeologist-indexer.
//!
//! Uses the fixture files in `tests/fixtures/` (cloned from
//! <https://github.com/arypog/fixtures.git>).  The bash script
//! `run_fixture.sh` is intentionally unsupported and must be ignored.

use std::path::Path;

use archaeologist_indexer::{
    build_graph, detect_language, extract_dependencies, extract_symbols, index_directory, parse,
    parse_incremental, DependencyKind, Lang, SymbolKind,
};

const FIXTURES_DIR: &str = "/tmp/archaeologist-fixtures";

fn fixture(lang: &str, file: &str) -> std::path::PathBuf {
    Path::new(FIXTURES_DIR).join(lang).join(file)
}

fn read_fixture(lang: &str, file: &str) -> Vec<u8> {
    std::fs::read(fixture(lang, file)).expect("fixture file must exist")
}

fn sym_names(syms: &[archaeologist_indexer::ExtractedSymbol]) -> Vec<&str> {
    syms.iter().map(|s| s.name.as_str()).collect()
}

fn has_sym_kind(
    syms: &[archaeologist_indexer::ExtractedSymbol],
    name: &str,
    kind: &SymbolKind,
) -> bool {
    syms.iter().any(|s| s.name == name && s.kind == *kind)
}

#[test]
fn detect_all_8_languages() {
    assert_eq!(detect_language("foo.rs"), Some(Lang::Rust));
    assert_eq!(detect_language("foo.py"), Some(Lang::Python));
    assert_eq!(detect_language("foo.js"), Some(Lang::JavaScript));
    assert_eq!(detect_language("foo.ts"), Some(Lang::TypeScript));
    assert_eq!(detect_language("foo.go"), Some(Lang::Go));
    assert_eq!(detect_language("foo.java"), Some(Lang::Java));
    assert_eq!(detect_language("foo.c"), Some(Lang::C));
    assert_eq!(detect_language("foo.cpp"), Some(Lang::Cpp));
}

#[test]
fn detect_unsupported_bash_ignored() {
    assert_eq!(detect_language("run_fixture.sh"), None);
    assert_eq!(detect_language("script.bash"), None);
}

#[test]
fn detect_extra_extensions() {
    assert_eq!(detect_language("foo.mjs"), Some(Lang::JavaScript));
    assert_eq!(detect_language("foo.tsx"), Some(Lang::TypeScript));
    assert_eq!(detect_language("foo.hpp"), Some(Lang::Cpp));
    assert_eq!(detect_language("foo.h"), Some(Lang::C));
}

#[test]
fn parse_rust_no_errors() {
    let src = read_fixture("rust", "example.rs");
    let result = parse(&src, Lang::Rust).expect("should parse");
    assert!(
        !result.has_errors,
        "unexpected parse errors in rust fixture"
    );
}

#[test]
fn parse_python_no_errors() {
    let src = read_fixture("python", "example.py");
    let result = parse(&src, Lang::Python).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_javascript_no_errors() {
    let src = read_fixture("javascript", "example.js");
    let result = parse(&src, Lang::JavaScript).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_typescript_no_errors() {
    let src = read_fixture("typescript", "example.ts");
    let result = parse(&src, Lang::TypeScript).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_go_no_errors() {
    let src = read_fixture("go", "example.go");
    let result = parse(&src, Lang::Go).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_java_no_errors() {
    let src = read_fixture("java", "Example.java");
    let result = parse(&src, Lang::Java).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_c_no_errors() {
    let src = read_fixture("c", "example.c");
    let result = parse(&src, Lang::C).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_cpp_no_errors() {
    let src = read_fixture("cpp", "example.cpp");
    let result = parse(&src, Lang::Cpp).expect("should parse");
    assert!(!result.has_errors);
}

#[test]
fn parse_handles_syntax_errors_gracefully() {
    let broken = b"fn foo( { this is not valid rust }}}";
    let result = parse(broken, Lang::Rust).expect("should still return a tree");
    assert!(result.has_errors, "expected parse errors to be flagged");
}

#[test]
fn parse_handles_empty_input() {
    let result = parse(b"", Lang::Python).expect("empty input must not panic");
    assert!(!result.has_errors);
}

#[test]
fn incremental_parse_matches_full_parse() {
    let src = b"fn hello() {}\n";
    let lang = Lang::Rust;

    let full = parse(src, lang).expect("full parse");

    let new_src = b"fn hello() {}\n\n";
    let edit = tree_sitter::InputEdit {
        start_byte: src.len(),
        old_end_byte: src.len(),
        new_end_byte: new_src.len(),
        start_position: tree_sitter::Point { row: 1, column: 0 },
        old_end_position: tree_sitter::Point { row: 1, column: 0 },
        new_end_position: tree_sitter::Point { row: 2, column: 0 },
    };

    let mut old_tree = full.tree;
    let inc =
        parse_incremental(new_src, lang, Some((&mut old_tree, edit))).expect("incremental parse");
    assert!(!inc.has_errors);

    let fallback = parse_incremental(new_src, lang, None).expect("fallback parse");
    assert!(!fallback.has_errors);
}

#[test]
fn symbols_rust_struct_and_function() {
    let src = read_fixture("rust", "example.rs");
    let result = parse(&src, Lang::Rust).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Struct),
        "expected struct User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "main", &SymbolKind::Function),
        "expected fn main; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_rust_position_range() {
    let src = read_fixture("rust", "example.rs");
    let result = parse(&src, Lang::Rust).unwrap();
    let syms = extract_symbols(&result, &src);
    let main_sym = syms.iter().find(|s| s.name == "main").unwrap();
    assert_eq!(main_sym.line_start, 6);
    assert!(main_sym.line_end > main_sym.line_start);
}

#[test]
fn symbols_python_class_and_method() {
    let src = read_fixture("python", "example.py");
    let result = parse(&src, Lang::Python).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Class),
        "expected class User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "__init__", &SymbolKind::Method),
        "expected method __init__; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_javascript_class_and_constructor() {
    let src = read_fixture("javascript", "example.js");
    let result = parse(&src, Lang::JavaScript).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Class),
        "expected class User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "constructor", &SymbolKind::Constructor),
        "expected constructor; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_typescript_class() {
    let src = read_fixture("typescript", "example.ts");
    let result = parse(&src, Lang::TypeScript).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Class),
        "expected class User; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_go_struct_and_function() {
    let src = read_fixture("go", "example.go");
    let result = parse(&src, Lang::Go).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Struct),
        "expected struct User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "main", &SymbolKind::Function),
        "expected fn main; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_java_classes_and_method() {
    let src = read_fixture("java", "Example.java");
    let result = parse(&src, Lang::Java).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Class),
        "expected class User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "Example", &SymbolKind::Class),
        "expected class Example; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "main", &SymbolKind::Method),
        "expected method main; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_java_visibility() {
    let src = read_fixture("java", "Example.java");
    let result = parse(&src, Lang::Java).unwrap();
    let syms = extract_symbols(&result, &src);

    let example_class = syms
        .iter()
        .find(|s| s.name == "Example" && s.kind == SymbolKind::Class)
        .expect("Example class");
    assert_eq!(example_class.visibility.as_deref(), Some("public"));
}

#[test]
fn symbols_c_function_and_typedef_struct() {
    let src = read_fixture("c", "example.c");
    let result = parse(&src, Lang::C).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "main", &SymbolKind::Function),
        "expected fn main; got: {:?}",
        sym_names(&syms)
    );
    // typedef struct { … } User;
    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Struct),
        "expected typedef struct User; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn symbols_cpp_struct_and_function() {
    let src = read_fixture("cpp", "example.cpp");
    let result = parse(&src, Lang::Cpp).unwrap();
    let syms = extract_symbols(&result, &src);

    assert!(
        has_sym_kind(&syms, "User", &SymbolKind::Struct),
        "expected struct User; got: {:?}",
        sym_names(&syms)
    );
    assert!(
        has_sym_kind(&syms, "main", &SymbolKind::Function),
        "expected fn main; got: {:?}",
        sym_names(&syms)
    );
}

#[test]
fn deps_go_detects_fmt_import() {
    let src = read_fixture("go", "example.go");
    let result = parse(&src, Lang::Go).unwrap();
    let deps = extract_dependencies(&result, &src);

    let imports: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Import)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        imports.iter().any(|s| s.contains("fmt")),
        "expected fmt import; got: {imports:?}"
    );
}

#[test]
fn deps_java_detects_imports() {
    let src = read_fixture("java", "Example.java");
    let result = parse(&src, Lang::Java).unwrap();
    let deps = extract_dependencies(&result, &src);

    let imports: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Import)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        imports.iter().any(|s| s.contains("ArrayList")),
        "expected ArrayList import; got: {imports:?}"
    );
}

#[test]
fn deps_c_detects_includes() {
    let src = read_fixture("c", "example.c");
    let result = parse(&src, Lang::C).unwrap();
    let deps = extract_dependencies(&result, &src);

    let imports: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Import)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        imports.iter().any(|s| s.contains("stdio")),
        "expected stdio.h include; got: {imports:?}"
    );
}

#[test]
fn deps_cpp_detects_includes() {
    let src = read_fixture("cpp", "example.cpp");
    let result = parse(&src, Lang::Cpp).unwrap();
    let deps = extract_dependencies(&result, &src);

    let imports: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Import)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        imports.iter().any(|s| s.contains("iostream")),
        "expected iostream include; got: {imports:?}"
    );
}

#[test]
fn deps_python_detects_print_call() {
    let src = read_fixture("python", "example.py");
    let result = parse(&src, Lang::Python).unwrap();
    let deps = extract_dependencies(&result, &src);

    let calls: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Call)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        calls.contains(&"print"),
        "expected print call; got: {calls:?}"
    );
}

#[test]
fn deps_javascript_detects_console_log_call() {
    let src = read_fixture("javascript", "example.js");
    let result = parse(&src, Lang::JavaScript).unwrap();
    let deps = extract_dependencies(&result, &src);

    let calls: Vec<&str> = deps
        .iter()
        .filter(|d| d.kind == DependencyKind::Call)
        .map(|d| d.target.as_str())
        .collect();
    assert!(
        calls.iter().any(|s| s.contains("console")),
        "expected console.log call; got: {calls:?}"
    );
}

#[test]
fn build_graph_groups_all_under_file_key() {
    let src = read_fixture("go", "example.go");
    let result = parse(&src, Lang::Go).unwrap();
    let deps = extract_dependencies(&result, &src);
    let total = deps.len();
    let graph = build_graph(deps);
    assert!(graph.contains_key("(file)"));
    assert_eq!(graph["(file)"].len(), total);
}

// parallel index_directory

#[test]
fn index_directory_indexes_all_8_languages_skips_sh() {
    let fixtures_dir = Path::new("/tmp/archaeologist-fixtures");

    let results = index_directory(fixtures_dir, |_done, _total| {
        // callback fires on rayon threads; just verify it doesn't panic
    })
    .expect("index_directory must succeed");

    // 8 fixture files, bash script must be excluded
    assert_eq!(
        results.len(),
        8,
        "expected exactly 8 indexed files, got {}; paths: {:?}",
        results.len(),
        results.iter().map(|f| &f.path).collect::<Vec<_>>()
    );

    // None should have critical errors (our fixtures are valid)
    // (has_errors could still be true for some grammars; this just checks
    //  that we got results at all)
    for f in &results {
        assert!(
            !f.symbols.is_empty() || !f.dependencies.is_empty(),
            "file {:?} produced no symbols AND no dependencies",
            f.path
        );
    }
}

#[test]
fn index_directory_progress_callback_fires() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    let fixtures_dir = Path::new("/tmp/archaeologist-fixtures");
    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);

    index_directory(fixtures_dir, move |_done, _total| {
        c.fetch_add(1, Ordering::Relaxed);
    })
    .unwrap();

    assert_eq!(counter.load(Ordering::Relaxed), 8);
}

#[test]
fn index_directory_empty_dir_returns_empty() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let results = index_directory(tmp.path(), |_, _| {}).expect("should succeed on empty dir");
    assert!(results.is_empty());
}

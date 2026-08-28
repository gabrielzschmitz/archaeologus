pub fn detect_language(path: &str) -> Option<String> {
    if path.ends_with(".rs") {
        Some("rust".to_string())
    } else if path.ends_with(".py") {
        Some("python".to_string())
    } else if path.ends_with(".js") {
        Some("javascript".to_string())
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        Some("typescript".to_string())
    } else if path.ends_with(".go") {
        Some("go".to_string())
    } else if path.ends_with(".java") {
        Some("java".to_string())
    } else if path.ends_with(".c") || path.ends_with(".h") {
        Some("c".to_string())
    } else if path.ends_with(".cpp") || path.ends_with(".hpp") || path.ends_with(".cc") {
        Some("cpp".to_string())
    } else {
        None
    }
}

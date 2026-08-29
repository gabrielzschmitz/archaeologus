//! CLI command unit tests for ask, explain, history, and impact.
//!
//! These tests cover the pure, offline logic in each command module.
//! Database-dependent behaviour is exercised by running against a live DB
//! (see the `index_test.rs` integration test for that pattern).

// ── `ask` command ─────────────────────────────────────────────────────────────

mod ask_tests {
    use archaeologus_cli::commands::ask::extract_keywords;

    #[test]
    fn question_with_symbol_name_extracted() {
        let kw = extract_keywords("What does the authenticate function do?");
        assert!(kw.contains(&"authenticate".to_string()), "kw={kw:?}");
    }

    #[test]
    fn stop_words_not_in_keywords() {
        let kw = extract_keywords("What does the authenticate function do?");
        for stop in &["what", "does", "the", "do"] {
            assert!(
                !kw.contains(&stop.to_string()),
                "stop word '{stop}' found in {kw:?}"
            );
        }
    }

    #[test]
    fn empty_question_returns_empty_keywords() {
        assert!(extract_keywords("").is_empty());
    }

    #[test]
    fn short_tokens_under_3_chars_filtered() {
        let kw = extract_keywords("do it");
        assert!(kw.is_empty(), "expected empty, got {kw:?}");
    }

    #[test]
    fn duplicate_keywords_deduplicated() {
        let kw = extract_keywords("parse parse parse the value");
        assert_eq!(kw.len(), 2, "expected [parse, value], got {kw:?}");
    }

    #[test]
    fn underscores_kept_as_word_character() {
        let kw = extract_keywords("explain parse_request function");
        assert!(kw.contains(&"parse_request".to_string()), "kw={kw:?}");
    }

    #[test]
    fn mixed_case_lowercased() {
        let kw = extract_keywords("Authenticate user");
        assert!(kw.contains(&"authenticate".to_string()), "kw={kw:?}");
        assert!(kw.contains(&"user".to_string()), "kw={kw:?}");
    }

    #[test]
    fn question_marks_and_punctuation_stripped() {
        let kw = extract_keywords("what is authenticate?");
        assert!(!kw.iter().any(|k| k.contains('?')));
    }
}

// ── `impact` command ──────────────────────────────────────────────────────────

mod impact_tests {
    use archaeologus_cli::commands::impact::{estimate_risk, is_test_symbol};
    use archaeologus_core::models::Symbol;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_sym(name: &str, doc: Option<&str>) -> Symbol {
        Symbol {
            id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            repository_id: Uuid::new_v4(),
            name: name.to_string(),
            symbol_type: "function".to_string(),
            language: "rust".to_string(),
            line_start: 0,
            line_end: 5,
            col_start: 0,
            col_end: 0,
            visibility: None,
            doc_comment: doc.map(ToString::to_string),
            raw_text: String::new(),
            created_at: Utc::now(),
        }
    }

    // ── estimate_risk ─────────────────────────────────────────────────────────

    #[test]
    fn risk_zero_callers_is_low() {
        assert!(estimate_risk(0, 0).starts_with("LOW"));
    }

    #[test]
    fn risk_few_callers_with_tests_is_low() {
        assert!(estimate_risk(3, 2).starts_with("LOW"));
    }

    #[test]
    fn risk_few_callers_no_tests_is_medium() {
        assert!(estimate_risk(4, 0).starts_with("MEDIUM"));
    }

    #[test]
    fn risk_moderate_callers_with_tests_is_medium() {
        assert!(estimate_risk(10, 3).starts_with("MEDIUM"));
    }

    #[test]
    fn risk_moderate_callers_no_tests_is_high() {
        assert!(estimate_risk(10, 0).starts_with("HIGH"));
    }

    #[test]
    fn risk_many_callers_no_tests_is_critical() {
        assert!(estimate_risk(100, 0).starts_with("CRITICAL"));
    }

    #[test]
    fn risk_many_callers_with_tests_is_high() {
        assert!(estimate_risk(50, 10).starts_with("HIGH"));
    }

    // ── is_test_symbol ────────────────────────────────────────────────────────

    #[test]
    fn test_prefix_detected() {
        assert!(is_test_symbol(&make_sym("test_login", None)));
    }

    #[test]
    fn test_suffix_detected() {
        assert!(is_test_symbol(&make_sym("login_test", None)));
    }

    #[test]
    fn test_middle_detected() {
        assert!(is_test_symbol(&make_sym("login_test_helper", None)));
    }

    #[test]
    fn regular_name_not_detected_as_test() {
        assert!(!is_test_symbol(&make_sym("authenticate", None)));
    }

    #[test]
    fn test_symbol_via_doc_annotation() {
        assert!(is_test_symbol(&make_sym("check_it", Some("#[test]"))));
    }

    #[test]
    fn java_annotation_detected() {
        assert!(is_test_symbol(&make_sym("shouldLogin", Some("@test"))));
    }
}

use super::scan_tokens;

fn dump_labels(source: &str) -> Vec<String> {
    scan_tokens(source)
        .expect("scanner should tokenize fixture source")
        .into_iter()
        .map(|token| token.dump_label())
        .collect()
}

#[test]
fn scan_tokens_w_sigil_with_parens_produces_list() {
    let labels = dump_labels("~w(foo bar baz)");
    assert_eq!(
        labels,
        [
            "LBRACKET",
            "STRING(foo)",
            "COMMA",
            "STRING(bar)",
            "COMMA",
            "STRING(baz)",
            "RBRACKET",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_w_sigil_with_atom_modifier_produces_atom_list() {
    let labels = dump_labels("~w(ok error)a");
    assert_eq!(
        labels,
        [
            "LBRACKET",
            "ATOM(ok)",
            "COMMA",
            "ATOM(error)",
            "RBRACKET",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_w_sigil_with_single_word_produces_single_element_list() {
    let labels = dump_labels("~w(hello)");
    assert_eq!(labels, ["LBRACKET", "STRING(hello)", "RBRACKET", "EOF"]);
}

#[test]
fn scan_tokens_char_literal_ascii_letter() {
    let labels = dump_labels("?a");
    assert_eq!(labels, ["INT(97)", "EOF"]);
}

#[test]
fn scan_tokens_char_literal_newline_escape() {
    // ?\n is codepoint 10
    let labels = dump_labels("?\\n");
    assert_eq!(labels, ["INT(10)", "EOF"]);
}

#[test]
fn scan_tokens_integer_with_underscores_multiple_groups() {
    let labels = dump_labels("1_000_000");
    assert_eq!(labels, ["INT(1000000)", "EOF"]);
}

#[test]
fn scan_tokens_hex_literal_lowercase() {
    let labels = dump_labels("0xff");
    assert_eq!(labels, ["INT(255)", "EOF"]);
}

#[test]
fn scan_tokens_octal_literal_lowercase() {
    let labels = dump_labels("0o77");
    assert_eq!(labels, ["INT(63)", "EOF"]);
}

#[test]
fn scan_tokens_binary_literal_lowercase() {
    let labels = dump_labels("0b1010");
    assert_eq!(labels, ["INT(10)", "EOF"]);
}

#[test]
fn scan_tokens_strict_equality_operators() {
    let labels = dump_labels("a === b !== c");
    assert_eq!(
        labels,
        [
            "IDENT(a)",
            "STRICT_EQ",
            "IDENT(b)",
            "STRICT_BANG_EQ",
            "IDENT(c)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_bitwise_operators() {
    let labels = dump_labels("a &&& b ||| c ^^^ d ~~~ e");
    assert_eq!(
        labels,
        [
            "IDENT(a)",
            "AMP_AMP_AMP",
            "IDENT(b)",
            "PIPE_PIPE_PIPE",
            "IDENT(c)",
            "CARET_CARET_CARET",
            "IDENT(d)",
            "TILDE_TILDE_TILDE",
            "IDENT(e)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_bitwise_shifts() {
    let labels = dump_labels("a <<< b >>> c");
    assert_eq!(
        labels,
        ["IDENT(a)", "LT_LT_LT", "IDENT(b)", "GT_GT_GT", "IDENT(c)", "EOF",]
    );
}

#[test]
fn scan_tokens_stepped_range_slash_slash() {
    let labels = dump_labels("1..10//2");
    assert_eq!(
        labels,
        [
            "INT(1)",
            "DOT_DOT",
            "INT(10)",
            "SLASH_SLASH",
            "INT(2)",
            "EOF",
        ]
    );
}

// --- Numeric literal completeness tests ---

#[test]
fn scan_tokens_hex_literal_uppercase_prefix() {
    let labels = dump_labels("0XFF");
    assert_eq!(labels, ["INT(255)", "EOF"]);
}

#[test]
fn scan_tokens_octal_literal_uppercase_prefix() {
    let labels = dump_labels("0O77");
    assert_eq!(labels, ["INT(63)", "EOF"]);
}

#[test]
fn scan_tokens_binary_literal_uppercase_prefix() {
    let labels = dump_labels("0B1010");
    assert_eq!(labels, ["INT(10)", "EOF"]);
}

#[test]
fn scan_tokens_hex_with_underscores() {
    let labels = dump_labels("0xFF_FF");
    assert_eq!(labels, ["INT(65535)", "EOF"]);
}

#[test]
fn scan_tokens_binary_with_underscores() {
    let labels = dump_labels("0b1010_1010");
    assert_eq!(labels, ["INT(170)", "EOF"]);
}

#[test]
fn scan_tokens_float_with_underscores() {
    let labels = dump_labels("1_000.50");
    assert_eq!(labels, ["FLOAT(1000.50)", "EOF"]);
}

#[test]
fn scan_tokens_char_literal_space_is_question_operator() {
    // ?<space> should be Question token (space is a separator, not char literal)
    let labels = dump_labels("x? y");
    assert_eq!(labels, ["IDENT(x)", "QUESTION", "IDENT(y)", "EOF"]);
}

#[test]
fn scan_tokens_char_literal_digit() {
    // ?0 should be INTEGER(48)
    let labels = dump_labels("?0");
    assert_eq!(labels, ["INT(48)", "EOF"]);
}

// --- Error cases ---

#[test]
fn scan_tokens_rejects_hex_with_no_digits() {
    let err = scan_tokens("0x").expect_err("0x with no digits should fail");
    assert!(
        err.to_string().contains("no digits"),
        "expected 'no digits' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_octal_with_no_digits() {
    let err = scan_tokens("0o").expect_err("0o with no digits should fail");
    assert!(
        err.to_string().contains("no digits"),
        "expected 'no digits' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_binary_with_no_digits() {
    let err = scan_tokens("0b").expect_err("0b with no digits should fail");
    assert!(
        err.to_string().contains("no digits"),
        "expected 'no digits' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_binary_invalid_digit() {
    let err = scan_tokens("0b12").expect_err("0b12 should fail — 2 is not a binary digit");
    assert!(
        err.to_string().contains("binary"),
        "expected 'binary' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_octal_invalid_digit() {
    let err = scan_tokens("0o78").expect_err("0o78 should fail — 8 is not an octal digit");
    assert!(
        err.to_string().contains("octal"),
        "expected 'octal' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_hex_separator_at_start() {
    let err = scan_tokens("0x_FF").expect_err("0x_FF should fail — separator at start");
    assert!(
        err.to_string().contains("separator"),
        "expected 'separator' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_hex_separator_at_end() {
    let err = scan_tokens("0xFF_").expect_err("0xFF_ should fail — separator at end");
    assert!(
        err.to_string().contains("separator"),
        "expected 'separator' in error: {err}"
    );
}

#[test]
fn scan_tokens_rejects_decimal_separator_at_end() {
    let err = scan_tokens("100_").expect_err("100_ should fail — separator at end");
    assert!(
        err.to_string().contains("separator"),
        "expected 'separator' in error: {err}"
    );
}

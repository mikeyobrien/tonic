use super::{scan_tokens, types::Span};

fn dump_labels(source: &str) -> Vec<String> {
    scan_tokens(source)
        .expect("scanner should tokenize fixture source")
        .into_iter()
        .map(|token| token.dump_label())
        .collect()
}

#[test]
fn scan_tokens_handles_minimal_module_fixture() {
    let labels = dump_labels("defmodule Math do\n  def add(a, b) do\n    a + b\n  end\nend\n");

    assert_eq!(
        labels,
        [
            "DEFMODULE(defmodule)",
            "IDENT(Math)",
            "DO(do)",
            "DEF(def)",
            "IDENT(add)",
            "LPAREN",
            "IDENT(a)",
            "COMMA",
            "IDENT(b)",
            "RPAREN",
            "DO(do)",
            "IDENT(a)",
            "PLUS",
            "IDENT(b)",
            "END(end)",
            "END(end)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_identifiers_and_literals() {
    let labels = dump_labels("value 42 3.14 \"ok\"");

    assert_eq!(
        labels,
        [
            "IDENT(value)",
            "INT(42)",
            "FLOAT(3.14)",
            "STRING(ok)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_triple_quoted_heredoc_literals() {
    let labels = dump_labels("\"\"\"hello\nworld\"\"\"");

    assert_eq!(labels, ["STRING(hello\nworld)", "EOF"]);
}

#[test]
fn scan_tokens_supports_atoms_and_operators() {
    let labels = dump_labels(":ok value |> wrap(:ok)\nfn arg -> arg end");

    assert_eq!(
        labels,
        [
            "ATOM(ok)",
            "IDENT(value)",
            "PIPE_GT",
            "IDENT(wrap)",
            "LPAREN",
            "ATOM(ok)",
            "RPAREN",
            "FN(fn)",
            "IDENT(arg)",
            "ARROW",
            "IDENT(arg)",
            "END(end)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_map_fat_arrow_without_regressing_case_arrows() {
    let labels = dump_labels("%{\"status\" => 200} case value do :ok -> 1 end");

    assert_eq!(
        labels,
        [
            "PERCENT",
            "LBRACE",
            "STRING(status)",
            "FAT_ARROW",
            "INT(200)",
            "RBRACE",
            "CASE(case)",
            "IDENT(value)",
            "DO(do)",
            "ATOM(ok)",
            "ARROW",
            "INT(1)",
            "END(end)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_pattern_delimiters() {
    let labels = dump_labels("{:ok, value} [head, _] %{}");

    assert_eq!(
        labels,
        [
            "LBRACE",
            "ATOM(ok)",
            "COMMA",
            "IDENT(value)",
            "RBRACE",
            "LBRACKET",
            "IDENT(head)",
            "COMMA",
            "IDENT(_)",
            "RBRACKET",
            "PERCENT",
            "LBRACE",
            "RBRACE",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_collection_literal_key_syntax() {
    let labels = dump_labels("%{ok: 1} [done: 2]");

    assert_eq!(
        labels,
        [
            "PERCENT",
            "LBRACE",
            "IDENT(ok)",
            "COLON",
            "INT(1)",
            "RBRACE",
            "LBRACKET",
            "IDENT(done)",
            "COLON",
            "INT(2)",
            "RBRACKET",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_module_qualified_calls() {
    let labels = dump_labels("Math.helper()");

    assert_eq!(
        labels,
        [
            "IDENT(Math)",
            "DOT",
            "IDENT(helper)",
            "LPAREN",
            "RPAREN",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_question_operator() {
    let labels = dump_labels("value()?");

    assert_eq!(
        labels,
        ["IDENT(value)", "LPAREN", "RPAREN", "QUESTION", "EOF",]
    );
}

#[test]
fn scan_tokens_supports_pin_guards_and_match_operator() {
    let labels = dump_labels("[^value, tail] when tail == 8 -> value = tail");

    assert_eq!(
        labels,
        [
            "LBRACKET",
            "CARET",
            "IDENT(value)",
            "COMMA",
            "IDENT(tail)",
            "RBRACKET",
            "WHEN(when)",
            "IDENT(tail)",
            "EQ_EQ",
            "INT(8)",
            "ARROW",
            "IDENT(value)",
            "MATCH_EQ",
            "IDENT(tail)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_control_form_keywords_and_with_operator() {
    let labels = dump_labels("if value do 1 else 0 end unless value do 1 end cond do true -> 1 end with x <- 1 do x end for x <- list(1, 2) do x end");

    assert_eq!(
        labels,
        [
            "IF(if)",
            "IDENT(value)",
            "DO(do)",
            "INT(1)",
            "ELSE(else)",
            "INT(0)",
            "END(end)",
            "UNLESS(unless)",
            "IDENT(value)",
            "DO(do)",
            "INT(1)",
            "END(end)",
            "COND(cond)",
            "DO(do)",
            "TRUE(true)",
            "ARROW",
            "INT(1)",
            "END(end)",
            "WITH(with)",
            "IDENT(x)",
            "LEFT_ARROW",
            "INT(1)",
            "DO(do)",
            "IDENT(x)",
            "END(end)",
            "FOR(for)",
            "IDENT(x)",
            "LEFT_ARROW",
            "IDENT(list)",
            "LPAREN",
            "INT(1)",
            "COMMA",
            "INT(2)",
            "RPAREN",
            "DO(do)",
            "IDENT(x)",
            "END(end)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_defp_and_default_argument_operator() {
    let labels = dump_labels("defp add(value, inc \\\\ 2) do value + inc end");

    assert_eq!(
        labels,
        [
            "DEFP(defp)",
            "IDENT(add)",
            "LPAREN",
            "IDENT(value)",
            "COMMA",
            "IDENT(inc)",
            "BACKSLASH_BACKSLASH",
            "INT(2)",
            "RPAREN",
            "DO(do)",
            "IDENT(value)",
            "PLUS",
            "IDENT(inc)",
            "END(end)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_capture_and_function_value_invocation() {
    let labels = dump_labels("&(&1 + 1) (&Math.add/2).(2, 3); fun.(2)");

    assert_eq!(
        labels,
        [
            "AMPERSAND",
            "LPAREN",
            "AMPERSAND",
            "INT(1)",
            "PLUS",
            "INT(1)",
            "RPAREN",
            "LPAREN",
            "AMPERSAND",
            "IDENT(Math)",
            "DOT",
            "IDENT(add)",
            "SLASH",
            "INT(2)",
            "RPAREN",
            "DOT",
            "LPAREN",
            "INT(2)",
            "COMMA",
            "INT(3)",
            "RPAREN",
            "SEMICOLON",
            "IDENT(fun)",
            "DOT",
            "LPAREN",
            "INT(2)",
            "RPAREN",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_supports_module_attributes_and_forms() {
    let labels = dump_labels("@doc \"ok\" alias Math, as: M");

    assert_eq!(
        labels,
        [
            "AT",
            "IDENT(doc)",
            "STRING(ok)",
            "IDENT(alias)",
            "IDENT(Math)",
            "COMMA",
            "IDENT(as)",
            "COLON",
            "IDENT(M)",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_assigns_spans_for_tokens_and_eof() {
    let tokens = scan_tokens("def value").expect("scanner should tokenize fixture source");

    assert_eq!(tokens[0].span(), Span::new(0, 3));
    assert_eq!(tokens[1].span(), Span::new(4, 9));
    assert_eq!(tokens[2].span(), Span::new(9, 9));
}

#[test]
fn scan_tokens_reports_invalid_character() {
    let error = scan_tokens("$").expect_err("scanner should reject unsupported characters");

    assert_eq!(error.to_string(), "invalid token '$' at offset 0");
    assert_eq!(error.span(), Span::new(0, 1));
}

#[test]
fn scan_tokens_skips_hash_comments() {
    let labels = dump_labels("1 # trailing comment\n2");

    assert_eq!(labels, ["INT(1)", "INT(2)", "EOF"]);
}

#[test]
fn scan_tokens_supports_basic_sigils_as_string_literals() {
    let labels = dump_labels("~s(hello) ~r/world/");

    assert_eq!(labels, ["STRING(hello)", "STRING(world)", "EOF"]);
}

#[test]
fn scan_tokens_reports_unterminated_string_with_span() {
    let error =
        scan_tokens("\"oops").expect_err("scanner should reject unterminated string literals");

    assert_eq!(error.to_string(), "unterminated string literal at offset 0");
    assert_eq!(error.span(), Span::new(0, 5));
}

#[test]
fn scan_tokens_supports_string_interpolation() {
    let labels = dump_labels("\"hello #{1 + 2} world\"");

    assert_eq!(
        labels,
        [
            "STRING_START",
            "STRING_PART(hello )",
            "INTERPOLATION_START",
            "INT(1)",
            "PLUS",
            "INT(2)",
            "INTERPOLATION_END",
            "STRING_PART( world)",
            "STRING_END",
            "EOF",
        ]
    );
}

#[test]
fn scan_tokens_emits_lt_lt_for_double_angle_open() {
    let labels = dump_labels("<<");
    assert_eq!(labels, ["LT_LT", "EOF"]);
}

#[test]
fn scan_tokens_emits_gt_gt_for_double_angle_close() {
    let labels = dump_labels(">>");
    assert_eq!(labels, ["GT_GT", "EOF"]);
}

#[test]
fn scan_tokens_emits_lt_lt_lt_for_triple_angle_open() {
    let labels = dump_labels("<<<");
    assert_eq!(labels, ["LT_LT_LT", "EOF"]);
}

#[test]
fn scan_tokens_distinguishes_lt_lt_from_lt_lt_lt() {
    let labels = dump_labels("<< <<<");
    assert_eq!(labels, ["LT_LT", "LT_LT_LT", "EOF"]);
}

#[test]
fn scan_tokens_emits_gt_gt_gt_for_triple_angle_close() {
    let labels = dump_labels(">>>");
    assert_eq!(labels, ["GT_GT_GT", "EOF"]);
}

#[test]
fn scan_tokens_distinguishes_gt_gt_from_gt_gt_gt() {
    let labels = dump_labels(">> >>>");
    assert_eq!(labels, ["GT_GT", "GT_GT_GT", "EOF"]);
}

#[test]
fn scan_tokens_tokenizes_bitstring_literal_sequence() {
    // <<72, 101, 108>> should tokenize as LT_LT INT COMMA INT COMMA INT GT_GT
    let labels = dump_labels("<<72, 101, 108>>");
    assert_eq!(
        labels,
        ["LT_LT", "INT(72)", "COMMA", "INT(101)", "COMMA", "INT(108)", "GT_GT", "EOF",]
    );
}

#[test]
fn scan_tokens_tokenizes_bitstring_with_size_annotation() {
    let labels = dump_labels("<<a::8, b::16>>");
    assert_eq!(
        labels,
        [
            "LT_LT", "IDENT(a)", "COLON", "COLON", "INT(8)", "COMMA", "IDENT(b)", "COLON", "COLON",
            "INT(16)", "GT_GT", "EOF",
        ]
    );
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

// End of lexer unit tests.

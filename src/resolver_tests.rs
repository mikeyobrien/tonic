use super::resolve_ast;
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;
use crate::resolver_diag::ResolverDiagnosticCode;

#[test]
fn resolve_ast_accepts_module_local_function_calls() {
    let source = "defmodule Demo do\n  def run() do\n    helper()\n  end\n\n  def helper() do\n    1\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
    let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

    resolve_ast(&ast).expect("resolver should accept local module references");
}

#[test]
fn resolve_ast_accepts_module_qualified_function_calls() {
    let source = "defmodule Math do\n  def helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize resolver fixture");
    let ast = parse_ast(&tokens).expect("parser should build resolver fixture ast");

    resolve_ast(&ast).expect("resolver should accept module-qualified references");
}

#[test]
fn resolve_ast_accepts_use_with_defined_module_target() {
    let source = "defmodule Feature do\n  def helper() do\n    41\n  end\nend\n\ndefmodule Demo do\n  use Feature\n\n  def run() do\n    helper()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize use fixture");
    let ast = parse_ast(&tokens).expect("parser should build use fixture ast");

    resolve_ast(&ast).expect("resolver should accept use with a defined module target");
}

#[test]
fn resolve_ast_accepts_import_only_and_except_filters() {
    let source = "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n\n  def run() do\n    add(20, 22)\n  end\nend\n\ndefmodule SafeDemo do\n  import Math, except: [unsafe: 1]\n\n  def run() do\n    add(2, 3)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize import filter fixture");
    let ast = parse_ast(&tokens).expect("parser should build import filter fixture ast");

    resolve_ast(&ast).expect("resolver should accept valid import only/except filters");
}

#[test]
fn resolve_ast_rejects_ambiguous_unqualified_imports() {
    let source = "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\nend\n\ndefmodule Algebra do\n  def add(a, b) do\n    a + b\n  end\nend\n\ndefmodule Demo do\n  import Math\n  import Algebra\n\n  def run() do\n    add(1, 2)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize ambiguous import fixture");
    let ast = parse_ast(&tokens).expect("parser should build ambiguous import fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject ambiguous unqualified import");
    assert_eq!(err.code(), ResolverDiagnosticCode::AmbiguousImportCall);
}

#[test]
fn resolve_ast_reports_import_filter_excludes_call() {
    let source = "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def sub(value, other) do\n    value - other\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n\n  def run() do\n    sub(10, 3)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize import filter fixture");
    let ast = parse_ast(&tokens).expect("parser should build import filter fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject call excluded by import filter");
    assert_eq!(err.code(), ResolverDiagnosticCode::ImportFilterExcludesCall);
}

#[test]
fn resolve_ast_rejects_duplicate_modules() {
    let source = "defmodule Demo do\n  def run() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    2\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize duplicate module fixture");
    let ast = parse_ast(&tokens).expect("parser should build duplicate module fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject duplicate modules");
    assert_eq!(err.code(), ResolverDiagnosticCode::DuplicateModule);
}

#[test]
fn resolve_ast_rejects_undefined_module_references() {
    let source = "defmodule Demo do\n  def run() do\n    Unknown.helper()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize undefined module fixture");
    let ast = parse_ast(&tokens).expect("parser should build undefined module fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject undefined module references");
    assert_eq!(err.code(), ResolverDiagnosticCode::UndefinedSymbol);
}

#[test]
fn resolve_ast_rejects_private_function_calls_from_other_modules() {
    let source = "defmodule Math do\n  defp helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize private function fixture");
    let ast = parse_ast(&tokens).expect("parser should build private function fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject cross-module private calls");
    assert_eq!(err.code(), ResolverDiagnosticCode::PrivateFunction);
}

#[test]
fn resolve_ast_accepts_private_function_calls_within_same_module() {
    let source = "defmodule Math do\n  def run() do\n    helper()\n  end\n\n  defp helper() do\n    1\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize same-module private fixture");
    let ast = parse_ast(&tokens).expect("parser should build same-module private fixture ast");

    resolve_ast(&ast).expect("resolver should accept same-module private function calls");
}

#[test]
fn resolve_ast_accepts_guard_builtins_in_guard_context() {
    let source = "defmodule Demo do\n  def run(x) when is_integer(x) do\n    x\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize guard builtin fixture");
    let ast = parse_ast(&tokens).expect("parser should build guard builtin fixture ast");

    resolve_ast(&ast).expect("resolver should accept guard builtins in guard context");
}

#[test]
fn resolve_ast_accepts_guard_builtins_in_non_guard_context() {
    let source = "defmodule Demo do\n  def run(x) do\n    is_integer(x)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize non-guard context fixture");
    let ast = parse_ast(&tokens).expect("parser should build non-guard context fixture ast");

    resolve_ast(&ast).expect("resolver should accept guard builtins as regular expressions");
}

#[test]
fn resolve_ast_accepts_struct_module_references() {
    let source = "defmodule Point do\n  defstruct x: nil, y: nil\n\n  def new(x, y) do\n    %Point{x: x, y: y}\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize struct fixture");
    let ast = parse_ast(&tokens).expect("parser should build struct fixture ast");

    resolve_ast(&ast).expect("resolver should accept valid struct references");
}

#[test]
fn resolve_ast_rejects_undefined_struct_modules() {
    let source = "defmodule Demo do\n  def run() do\n    %Unknown{field: 1}\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize undefined struct fixture");
    let ast = parse_ast(&tokens).expect("parser should build undefined struct fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject undefined struct modules");
    assert_eq!(err.code(), ResolverDiagnosticCode::UndefinedStructModule);
}

#[test]
fn resolve_ast_rejects_unknown_struct_fields() {
    let source = "defmodule Point do\n  defstruct x: nil, y: nil\n\n  def new(x, y, z) do\n    %Point{x: x, y: y, z: z}\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize unknown struct field fixture");
    let ast = parse_ast(&tokens).expect("parser should build unknown struct field fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject unknown struct fields");
    assert_eq!(err.code(), ResolverDiagnosticCode::UnknownStructField);
}

#[test]
fn resolve_ast_accepts_defprotocol_and_defimpl() {
    let source = "defmodule Protocols do\n  defprotocol Size do\n    def size(term)\n  end\n\n  defimpl Size, for: MyList do\n    def size(term) do\n      length(term)\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize protocol fixture");
    let ast = parse_ast(&tokens).expect("parser should build protocol fixture ast");

    resolve_ast(&ast).expect("resolver should accept valid protocol and impl");
}

#[test]
fn resolve_ast_rejects_unknown_protocol_in_defimpl() {
    let source = "defmodule MyList do\n  defimpl Unknown, for: MyList do\n    def size(term) do\n      length(term)\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize unknown protocol fixture");
    let ast = parse_ast(&tokens).expect("parser should build unknown protocol fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject unknown protocol in defimpl");
    assert_eq!(err.code(), ResolverDiagnosticCode::UnknownProtocol);
}

#[test]
fn resolve_ast_rejects_duplicate_defimpl_for_same_target() {
    let source = "defmodule Protocols do\n  defprotocol Size do\n    def size(term)\n  end\n\n  defimpl Size, for: MyList do\n    def size(term) do\n      1\n    end\n  end\n\n  defimpl Size, for: MyList do\n    def size(term) do\n      2\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize duplicate impl fixture");
    let ast = parse_ast(&tokens).expect("parser should build duplicate impl fixture ast");

    let err = resolve_ast(&ast)
        .expect_err("resolver should reject duplicate protocol impl for same target");
    assert_eq!(err.code(), ResolverDiagnosticCode::DuplicateProtocolImpl);
}

#[test]
fn resolve_ast_rejects_protocol_impl_with_missing_function() {
    let source = "defmodule Protocols do\n  defprotocol Size do\n    def size(term)\n    def count(term)\n  end\n\n  defimpl Size, for: MyList do\n    def size(term) do\n      1\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize missing fn fixture");
    let ast = parse_ast(&tokens).expect("parser should build missing fn fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject impl with missing function");
    assert_eq!(err.code(), ResolverDiagnosticCode::InvalidProtocolImpl);
}

#[test]
fn resolve_ast_rejects_protocol_impl_with_arity_mismatch() {
    let source = "defmodule Protocols do\n  defprotocol Size do\n    def size(term)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(term, extra) do\n      2\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize arity mismatch fixture");
    let ast = parse_ast(&tokens).expect("parser should build arity mismatch fixture ast");

    let err = resolve_ast(&ast).expect_err("resolver should reject impl with arity mismatch");
    assert_eq!(err.code(), ResolverDiagnosticCode::InvalidProtocolImpl);
    assert!(
        err.message().contains("has arity mismatch (expected 1)"),
        "error message '{}' should mention arity mismatch (expected 1)",
        err.message()
    );
}

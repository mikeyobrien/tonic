use super::lower_ast_to_ir;
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;

#[test]
fn lower_ast_emits_const_int_and_return_for_literal_function() {
    let source = "defmodule Demo do\n  def run() do\n    1\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for literal body");

    assert_eq!(
        serde_json::to_string(&ir).expect("ir should serialize"),
        concat!(
            "{\"functions\":[",
            "{\"name\":\"Demo.run\",\"params\":[],\"ops\":[",
            "{\"op\":\"const_int\",\"value\":1,\"offset\":37},",
            "{\"op\":\"return\",\"offset\":37}",
            "]}",
            "]}"
        )
    );
}

#[test]
fn lower_ast_qualifies_local_call_targets() {
    let source = "defmodule Demo do\n  def run() do\n    helper(1)\n  end\n\n  def helper(value) do\n    value()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for call body");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {"op":"const_int","value":1,"offset":44},
            {"op":"call","callee":{"kind":"function","name":"Demo.helper"},"argc":1,"offset":37},
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_canonicalizes_call_target_kinds() {
    let source = "defmodule Demo do\n  def run() do\n    ok(helper(1))\n  end\n\n  def helper(value) do\n    value()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should succeed for call body");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {"op":"const_int","value":1,"offset":47},
            {"op":"call","callee":{"kind":"function","name":"Demo.helper"},"argc":1,"offset":40},
            {"op":"call","callee":{"kind":"builtin","name":"ok"},"argc":1,"offset":37},
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_marks_protocol_dispatch_as_builtin_call_target() {
    let source =
        "defmodule Demo do\n  def run() do\n    protocol_dispatch(tuple(1, 2))\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should classify protocol dispatch as builtin");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {"op":"const_int","value":1,"offset":61},
            {"op":"const_int","value":2,"offset":64},
            {"op":"call","callee":{"kind":"builtin","name":"tuple"},"argc":2,"offset":55},
            {"op":"call","callee":{"kind":"builtin","name":"protocol_dispatch"},"argc":1,"offset":37},
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_marks_host_call_as_builtin_call_target() {
    let source = "defmodule Demo do\n  def run() do\n    host_call(:identity, 42)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should classify host_call as builtin");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    // Find the host_call operation
    let ops = &json["functions"][0]["ops"];
    let host_call_op = ops
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["op"] == "call" && op["callee"]["name"] == "host_call")
        .expect("lowered ir should include host_call as builtin");

    assert_eq!(host_call_op["callee"]["kind"], "builtin");
    assert_eq!(host_call_op["callee"]["name"], "host_call");
}

#[test]
fn lower_ast_threads_pipe_input_into_rhs_call_arguments() {
    let source = "defmodule Enum do\n  def stage_one(_value) do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    tuple(1, 2) |> Enum.stage_one()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should support pipe expressions");
    let run_function = ir
        .functions
        .iter()
        .find(|function| function.name == "Demo.run")
        .expect("lowered ir should include Demo.run");

    assert!(matches!(
        &run_function.ops[2],
        super::IrOp::Call {
            callee: super::IrCallTarget::Builtin { name },
            argc: 2,
            ..
        } if name == "tuple"
    ));

    assert!(matches!(
        &run_function.ops[3],
        super::IrOp::Call {
            callee: super::IrCallTarget::Function { name },
            argc: 1,
            ..
        } if name == "Enum.stage_one"
    ));
}

#[test]
fn lower_ast_supports_question_and_case_ops() {
    let source = "defmodule Demo do\n  def run() do\n    case ok(1)? do\n      :ok -> 2\n      _ -> 3\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should support question and case");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {"op":"const_int","value":1,"offset":45},
            {"op":"call","callee":{"kind":"builtin","name":"ok"},"argc":1,"offset":42},
            {"op":"question","offset":47},
            {
                "op":"case",
                "branches":[
                    {
                        "pattern":{"kind":"atom","value":"ok"},
                        "ops":[{"op":"const_int","value":2,"offset":65}]
                    },
                    {
                        "pattern":{"kind":"wildcard"},
                        "ops":[{"op":"const_int","value":3,"offset":78}]
                    }
                ],
                "offset":37
            },
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_supports_for_comprehension_ops() {
    let source = "defmodule Demo do\n  def run() do\n    for x <- list(1, 2) do\n      x + 1\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize for comprehension fixture");
    let ast = parse_ast(&tokens).expect("parser should build for comprehension fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should support for comprehensions");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {
                "op":"for",
                "into_ops": null,
                "reduce_ops": null,
                "generators":[
                    {
                        "pattern":{"kind":"bind","name":"x"},
                        "source_ops":[
                            {"op":"const_int","value":1,"offset":51},
                                {"op":"const_int","value":2,"offset":54},
                                {"op":"call","callee":{"kind":"builtin","name":"list"},"argc":2,"offset":46}
                        ]
                    }
                ],
                "body_ops":[
                    {"op":"load_variable","name":"x","offset":66},
                    {"op":"const_int","value":1,"offset":70},
                    {"op":"add_int","offset":66}
                ],
                "offset":37
            },
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_emits_distinct_not_and_bang_ops() {
    let source = "defmodule Demo do\n  def run() do\n    tuple(not false, !nil)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize unary op fixture");
    let ast = parse_ast(&tokens).expect("parser should build unary op fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should support unary op fixture");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    assert_eq!(
        json["functions"][0]["ops"],
        serde_json::json!([
            {"op":"const_bool","value":false,"offset":47},
            {"op":"not","offset":43},
            {"op":"const_nil","offset":55},
            {"op":"bang","offset":54},
            {"op":"call","callee":{"kind":"builtin","name":"tuple"},"argc":2,"offset":37},
            {"op":"return","offset":37}
        ])
    );
}

#[test]
fn lower_ast_generates_protocol_dispatcher_and_impl_functions() {
    let source = "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  def run() do\n    Size.size(tuple(1, 2))\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize protocol lowering fixture");
    let ast = parse_ast(&tokens).expect("parser should build protocol lowering fixture ast");

    let ir = lower_ast_to_ir(&ast).expect("lowering should support protocol forms");
    let json = serde_json::to_value(&ir).expect("ir should serialize");

    let names = json["functions"]
        .as_array()
        .expect("lowered functions should be an array")
        .iter()
        .map(|function| {
            function["name"]
                .as_str()
                .expect("lowered function should include a name")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert!(names.iter().any(|name| name == "Demo.run"));
    assert!(names.iter().any(|name| name == "Size.size"));
    assert!(names
        .iter()
        .any(|name| name == "__tonic_protocol_impl.Size.Tuple.size"));

    let size_function = json["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "Size.size")
        .expect("lowered ir should include protocol dispatcher function");

    let serialized_ops = serde_json::to_string(&size_function["ops"])
        .expect("protocol dispatcher ops should serialize");
    assert!(serialized_ops.contains("protocol_dispatch"));
}

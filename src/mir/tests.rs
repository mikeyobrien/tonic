use super::{lower_ir_to_mir, MirInstruction, MirTerminator, MirType};
use crate::ir::lower_ast_to_ir;
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;

#[test]
fn lower_ir_to_mir_emits_deterministic_case_cfg_shape() {
    let source = "defmodule Demo do\n  def run() do\n    case ok(1)? do\n      :ok -> 2\n      _ -> 3\n    end\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize fixture");
    let ast = parse_ast(&tokens).expect("parser should build fixture ast");
    let ir = lower_ast_to_ir(&ast).expect("ir lowering should succeed");

    let mir = lower_ir_to_mir(&ir).expect("mir lowering should succeed");
    let run = &mir.functions[0];

    assert_eq!(run.entry_block, 0);
    assert_eq!(run.blocks.len(), 4);
    assert!(matches!(
        run.blocks[0].terminator,
        MirTerminator::Match { .. }
    ));
    assert_eq!(run.blocks[3].args[0].value_type, MirType::Dynamic);
}

#[test]
fn lower_ir_to_mir_marks_ok_call_result_as_result_type() {
    let source = "defmodule Demo do\n  def run() do\n    ok(1)\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize fixture");
    let ast = parse_ast(&tokens).expect("parser should build fixture ast");
    let ir = lower_ast_to_ir(&ast).expect("ir lowering should succeed");

    let mir = lower_ir_to_mir(&ir).expect("mir lowering should succeed");
    let call = mir.functions[0].blocks[0]
        .instructions
        .iter()
        .find(|instruction| matches!(instruction, MirInstruction::Call { .. }))
        .expect("mir should include call instruction");

    assert!(matches!(
        call,
        MirInstruction::Call { value_type, .. } if *value_type == MirType::Result
    ));
}

use crate::ir::{IrCallTarget, IrForGenerator, IrOp, IrPattern, IrProgram};
use crate::native_runtime;
use std::collections::HashMap;
use std::fmt;

const ENTRYPOINT: &str = "Demo.run";
const FOR_REDUCE_ACC_BINDING: &str = "__tonic_for_acc";

// PLACEHOLDER - full file will be committed
// This is a temporary marker

fn placeholder() {}

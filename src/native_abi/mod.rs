mod heap;
#[cfg(test)]
mod tests;

use crate::runtime::RuntimeValue;
use serde::{Deserialize, Serialize};

/// Tags identifying value kinds in the native ABI.
const TAG_INT: u64 = 0;
const TAG_FLOAT: u64 = 1;
const TAG_BOOL: u64 = 2;
const TAG_NIL: u64 = 3;
const TAG_ATOM: u64 = 4;
const TAG_STRING: u64 = 5;
const TAG_TUPLE: u64 = 6;
const TAG_LIST: u64 = 7;
const TAG_MAP: u64 = 8;
const TAG_CLOSURE: u64 = 9;
const TAG_RESULT_OK: u64 = 10;
const TAG_RESULT_ERR: u64 = 11;
const TAG_KEYWORD: u64 = 12;
const TAG_RANGE: u64 = 13;
const TAG_STEPPED_RANGE: u64 = 14;

/// A native ABI value encoding.
///
/// Integers are tagged at bit 0; other values use an 8-byte aligned heap
/// pointer with the tag stored in the low 4 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TValue(u64);

/// Pointer tag encoding: pointer + tag in low 4 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TValueTag {
    Int = 0,
    Float = 1,
    Bool = 2,
    Nil = 3,
    Atom = 4,
    String = 5,
    Tuple = 6,
    List = 7,
    Map = 8,
    Closure = 9,
    ResultOk = 10,
    ResultErr = 11,
    Keyword = 12,
    Range = 13,
    SteppedRange = 14,
}

impl TValue {
    /// Encode a `RuntimeValue` as a `TValue`.
    pub fn encode(value: &RuntimeValue) -> Self {
        match value {
            RuntimeValue::Int(n) => Self::encode_int(*n),
            RuntimeValue::Float(f) => {
                let bits = f64::from_bits(f.parse::<u64>().unwrap_or_else(|_| {
                    f.parse::<f64>().unwrap_or(0.0f64).to_bits()
                }));
                let ptr = heap::alloc_f64(bits) as u64;
                Self((ptr & !0xf) | TAG_FLOAT)
            }
            RuntimeValue::Bool(b) => Self(TAG_BOOL | (if *b { 0x10 } else { 0x00 })),
            RuntimeValue::Nil => Self(TAG_NIL),
            RuntimeValue::Atom(s) => {
                let ptr = heap::alloc_string(s) as u64;
                Self((ptr & !0xf) | TAG_ATOM)
            }
            RuntimeValue::String(s) => {
                let ptr = heap::alloc_string(s) as u64;
                Self((ptr & !0xf) | TAG_STRING)
            }
            RuntimeValue::Tuple(a, b) => {
                let a_enc = Self::encode(a);
                let b_enc = Self::encode(b);
                let ptr = heap::alloc_tuple2(a_enc.0, b_enc.0) as u64;
                Self((ptr & !0xf) | TAG_TUPLE)
            }
            RuntimeValue::List(items) => {
                let encoded: Vec<u64> = items.iter().map(|v| Self::encode(v).0).collect();
                let ptr = heap::alloc_list(&encoded) as u64;
                Self((ptr & !0xf) | TAG_LIST)
            }
            RuntimeValue::Map(entries) => {
                let encoded: Vec<(u64, u64)> = entries
                    .iter()
                    .map(|(k, v)| (Self::encode(k).0, Self::encode(v).0))
                    .collect();
                let ptr = heap::alloc_map(&encoded) as u64;
                Self((ptr & !0xf) | TAG_MAP)
            }
            RuntimeValue::ResultOk(inner) => {
                let inner_enc = Self::encode(inner);
                let ptr = heap::alloc_result(inner_enc.0) as u64;
                Self((ptr & !0xf) | TAG_RESULT_OK)
            }
            RuntimeValue::ResultErr(inner) => {
                let inner_enc = Self::encode(inner);
                let ptr = heap::alloc_result(inner_enc.0) as u64;
                Self((ptr & !0xf) | TAG_RESULT_ERR)
            }
            RuntimeValue::Keyword(entries) => {
                let encoded: Vec<(u64, u64)> = entries
                    .iter()
                    .map(|(k, v)| (Self::encode(k).0, Self::encode(v).0))
                    .collect();
                let ptr = heap::alloc_map(&encoded) as u64;
                Self((ptr & !0xf) | TAG_KEYWORD)
            }
            RuntimeValue::Range(start, end) => {
                let ptr = heap::alloc_tuple2(*start as u64, *end as u64) as u64;
                Self((ptr & !0xf) | TAG_RANGE)
            }
            RuntimeValue::SteppedRange(start, end, step) => {
                let ptr = heap::alloc_triple(*start as u64, *end as u64, *step as u64) as u64;
                Self((ptr & !0xf) | TAG_STEPPED_RANGE)
            }
            RuntimeValue::Closure(_) => Self(TAG_CLOSURE),
        }
    }

    fn encode_int(n: i64) -> Self {
        // Store int in upper 63 bits, set low bit to 1 for int tag
        let raw = ((n as u64) << 1) | 1;
        Self(raw)
    }

    /// Decode a `TValue` back to a `RuntimeValue`.
    pub fn decode(&self) -> RuntimeValue {
        if self.0 & 1 == 1 {
            // Int: upper 63 bits
            return RuntimeValue::Int((self.0 >> 1) as i64);
        }
        let tag = (self.0 & 0xf) as u64;
        let ptr = (self.0 & !0xf) as *const u8;

        match tag {
            TAG_FLOAT => {
                let f = unsafe { heap::read_f64(ptr as *const u64) };
                RuntimeValue::Float(format!("{f}"))
            }
            TAG_BOOL => RuntimeValue::Bool(self.0 & 0x10 != 0),
            TAG_NIL => RuntimeValue::Nil,
            TAG_ATOM => {
                let s = unsafe { heap::read_string(ptr) };
                RuntimeValue::Atom(s)
            }
            TAG_STRING => {
                let s = unsafe { heap::read_string(ptr) };
                RuntimeValue::String(s)
            }
            TAG_TUPLE => {
                let (a, b) = unsafe { heap::read_tuple2(ptr as *const u64) };
                let a_val = TValue(a).decode();
                let b_val = TValue(b).decode();
                RuntimeValue::Tuple(Box::new(a_val), Box::new(b_val))
            }
            TAG_LIST => {
                let items = unsafe { heap::read_list(ptr as *const u64) };
                RuntimeValue::List(items.iter().map(|&v| TValue(v).decode()).collect())
            }
            TAG_MAP => {
                let entries = unsafe { heap::read_map(ptr as *const u64) };
                RuntimeValue::Map(
                    entries
                        .iter()
                        .map(|&(k, v)| (TValue(k).decode(), TValue(v).decode()))
                        .collect(),
                )
            }
            TAG_RESULT_OK => {
                let inner = unsafe { heap::read_result(ptr as *const u64) };
                RuntimeValue::ResultOk(Box::new(TValue(inner).decode()))
            }
            TAG_RESULT_ERR => {
                let inner = unsafe { heap::read_result(ptr as *const u64) };
                RuntimeValue::ResultErr(Box::new(TValue(inner).decode()))
            }
            TAG_KEYWORD => {
                let entries = unsafe { heap::read_map(ptr as *const u64) };
                RuntimeValue::Keyword(
                    entries
                        .iter()
                        .map(|&(k, v)| (TValue(k).decode(), TValue(v).decode()))
                        .collect(),
                )
            }
            TAG_RANGE => {
                let (start, end) = unsafe { heap::read_tuple2(ptr as *const u64) };
                RuntimeValue::Range(start as i64, end as i64)
            }
            TAG_STEPPED_RANGE => {
                let (start, end, step) = unsafe { heap::read_triple(ptr as *const u64) };
                RuntimeValue::SteppedRange(start as i64, end as i64, step as i64)
            }
            TAG_CLOSURE => RuntimeValue::Nil,
            _ => RuntimeValue::Nil,
        }
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    pub fn tag(&self) -> TValueTag {
        if self.0 & 1 == 1 {
            return TValueTag::Int;
        }
        match self.0 & 0xf {
            TAG_FLOAT => TValueTag::Float,
            TAG_BOOL => TValueTag::Bool,
            TAG_NIL => TValueTag::Nil,
            TAG_ATOM => TValueTag::Atom,
            TAG_STRING => TValueTag::String,
            TAG_TUPLE => TValueTag::Tuple,
            TAG_LIST => TValueTag::List,
            TAG_MAP => TValueTag::Map,
            TAG_CLOSURE => TValueTag::Closure,
            TAG_RESULT_OK => TValueTag::ResultOk,
            TAG_RESULT_ERR => TValueTag::ResultErr,
            TAG_KEYWORD => TValueTag::Keyword,
            TAG_RANGE => TValueTag::Range,
            TAG_STEPPED_RANGE => TValueTag::SteppedRange,
            _ => TValueTag::Nil,
        }
    }
}

pub fn is_scalar(tag: TValueTag) -> bool {
    matches!(tag, TValueTag::Int | TValueTag::Float | TValueTag::Bool | TValueTag::Nil
        | TValueTag::Atom | TValueTag::String | TValueTag::Result
            | TValueTag::Bool | TValueTag::Nil)
}

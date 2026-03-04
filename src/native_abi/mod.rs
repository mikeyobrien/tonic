mod heap;
#[cfg(test)]
mod tests;

use crate::runtime::RuntimeValue;

/// Native runtime ABI version for TValue and call-boundary structs.
pub const TONIC_RUNTIME_ABI_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NativeAbiMemoryStats {
    pub allocations_total: u64,
    pub reclaims_total: u64,
    pub active_handles: u64,
    pub active_handles_high_water: u64,
}

pub fn memory_stats_snapshot() -> Result<NativeAbiMemoryStats, AbiError> {
    let stats = heap::stats()?;
    Ok(NativeAbiMemoryStats {
        allocations_total: stats.allocations_total,
        reclaims_total: stats.reclaims_total,
        active_handles: stats.active_handles,
        active_handles_high_water: stats.active_handles_high_water,
    })
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TValueTag {
    Int = 1,
    Bool = 2,
    Nil = 3,
    Float = 4,
    String = 5,
    Atom = 6,
    List = 7,
    Map = 8,
    Keyword = 9,
    Tuple2 = 10,
    ResultOk = 11,
    ResultErr = 12,
    Closure = 13,
    Range = 14,
}

impl TryFrom<u8> for TValueTag {
    type Error = AbiError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Int),
            2 => Ok(Self::Bool),
            3 => Ok(Self::Nil),
            4 => Ok(Self::Float),
            5 => Ok(Self::String),
            6 => Ok(Self::Atom),
            7 => Ok(Self::List),
            8 => Ok(Self::Map),
            9 => Ok(Self::Keyword),
            10 => Ok(Self::Tuple2),
            11 => Ok(Self::ResultOk),
            12 => Ok(Self::ResultErr),
            13 => Ok(Self::Closure),
            14 => Ok(Self::Range),
            _ => Err(AbiError::new(
                AbiErrorCode::InvalidTag,
                format!("unknown TValue tag {value}"),
            )),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TOwnership {
    Immediate = 0,
    RefCounted = 1,
}

impl TryFrom<u8> for TOwnership {
    type Error = AbiError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Immediate),
            1 => Ok(Self::RefCounted),
            _ => Err(AbiError::new(
                AbiErrorCode::InvalidOwnership,
                format!("unknown TValue ownership {value}"),
            )),
        }
    }
}

/// Stable value cell passed across native runtime boundaries.
///
/// Layout contract (ABI v1):
/// - 16 bytes, 8-byte aligned
/// - `tag` discriminates runtime kind
/// - `ownership` defines payload lifetime policy
/// - `payload` stores either immediate bits or heap handle
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TValue {
    pub tag: u8,
    pub ownership: u8,
    pub reserved: u16,
    pub payload: u64,
}

impl TValue {
    pub const fn nil() -> Self {
        Self {
            tag: TValueTag::Nil as u8,
            ownership: TOwnership::Immediate as u8,
            reserved: 0,
            payload: 0,
        }
    }

    pub const fn from_raw_parts(tag: u8, ownership: u8, payload: u64) -> Self {
        Self {
            tag,
            ownership,
            reserved: 0,
            payload,
        }
    }

    pub fn try_tag(self) -> Result<TValueTag, AbiError> {
        TValueTag::try_from(self.tag)
    }

    pub fn try_ownership(self) -> Result<TOwnership, AbiError> {
        TOwnership::try_from(self.ownership)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiErrorCode {
    InvalidTag,
    InvalidOwnership,
    InvalidHandle,
    TagHandleMismatch,
    AbiVersionMismatch,
    InvalidCallFrame,
    OwnershipViolation,
    Panic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiError {
    pub code: AbiErrorCode,
    pub message: String,
}

impl AbiError {
    pub fn new(code: AbiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AbiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AbiError {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TCallStatus {
    Ok = 0,
    Err = 1,
    Panic = 2,
    InvalidAbi = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TCallContext {
    pub abi_version: u32,
    pub argc: usize,
    pub argv: *const TValue,
}

impl TCallContext {
    pub fn from_slice(args: &[TValue]) -> Self {
        Self {
            abi_version: TONIC_RUNTIME_ABI_VERSION,
            argc: args.len(),
            argv: args.as_ptr(),
        }
    }

    fn args(self) -> Result<&'static [TValue], AbiError> {
        if self.argc == 0 {
            return Ok(&[]);
        }

        if self.argv.is_null() {
            return Err(AbiError::new(
                AbiErrorCode::InvalidCallFrame,
                "call frame has null argv with non-zero argc",
            ));
        }

        // SAFETY: caller owns `argv` for `argc` elements for the duration of boundary call.
        let args = unsafe { std::slice::from_raw_parts(self.argv, self.argc) };
        Ok(args)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TCallResult {
    pub status: TCallStatus,
    pub value: TValue,
    pub error: TValue,
}

pub fn validate_tvalue(value: TValue) -> Result<(), AbiError> {
    if value.reserved != 0 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidOwnership,
            "reserved TValue bits must be zero",
        ));
    }

    let tag = value.try_tag()?;
    let ownership = value.try_ownership()?;

    if is_heap_tag(tag) {
        if ownership != TOwnership::RefCounted {
            return Err(AbiError::new(
                AbiErrorCode::OwnershipViolation,
                "heap TValue requires refcounted ownership",
            ));
        }
        heap::validate_handle(tag, value.payload)?;
    } else if ownership != TOwnership::Immediate {
        return Err(AbiError::new(
            AbiErrorCode::OwnershipViolation,
            "immediate TValue requires immediate ownership",
        ));
    }

    if tag == TValueTag::Bool && value.payload > 1 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidTag,
            "bool TValue payload must be 0 or 1",
        ));
    }

    if tag == TValueTag::Nil && value.payload != 0 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidTag,
            "nil TValue payload must be 0",
        ));
    }

    Ok(())
}

pub fn runtime_to_tvalue(value: RuntimeValue) -> Result<TValue, AbiError> {
    let tag = runtime_tag(&value);

    let encoded = match value {
        RuntimeValue::Int(number) => TValue::from_raw_parts(
            TValueTag::Int as u8,
            TOwnership::Immediate as u8,
            number as u64,
        ),
        RuntimeValue::Bool(flag) => TValue::from_raw_parts(
            TValueTag::Bool as u8,
            TOwnership::Immediate as u8,
            u64::from(flag),
        ),
        RuntimeValue::Nil => TValue::nil(),
        other => {
            let handle = heap::store(tag, other)?;
            TValue::from_raw_parts(tag as u8, TOwnership::RefCounted as u8, handle)
        }
    };

    Ok(encoded)
}

pub fn tvalue_to_runtime(value: TValue) -> Result<RuntimeValue, AbiError> {
    validate_tvalue(value)?;

    let tag = value.try_tag()?;
    let decoded = match tag {
        TValueTag::Int => RuntimeValue::Int(value.payload as i64),
        TValueTag::Bool => RuntimeValue::Bool(value.payload == 1),
        TValueTag::Nil => RuntimeValue::Nil,
        _ => heap::load(tag, value.payload)?,
    };

    Ok(decoded)
}

pub fn retain_tvalue(value: TValue) -> Result<(), AbiError> {
    let tag = value.try_tag()?;
    if !is_heap_tag(tag) {
        return Err(AbiError::new(
            AbiErrorCode::OwnershipViolation,
            "retain requires a heap TValue",
        ));
    }

    if value.try_ownership()? != TOwnership::RefCounted {
        return Err(AbiError::new(
            AbiErrorCode::OwnershipViolation,
            "retain requires refcounted ownership",
        ));
    }

    heap::retain(value.payload)
}

pub fn release_tvalue(value: TValue) -> Result<(), AbiError> {
    let tag = value.try_tag()?;
    if !is_heap_tag(tag) {
        return Err(AbiError::new(
            AbiErrorCode::OwnershipViolation,
            "release requires a heap TValue",
        ));
    }

    if value.try_ownership()? != TOwnership::RefCounted {
        return Err(AbiError::new(
            AbiErrorCode::OwnershipViolation,
            "release requires refcounted ownership",
        ));
    }

    heap::release(value.payload)
}

pub fn clone_tvalue(value: TValue) -> Result<TValue, AbiError> {
    let tag = value.try_tag()?;
    if is_heap_tag(tag) {
        retain_tvalue(value)?;
    }
    Ok(value)
}

pub fn invoke_runtime_boundary<F>(ctx: &TCallContext, helper: F) -> TCallResult
where
    F: FnOnce(&[RuntimeValue]) -> Result<RuntimeValue, AbiError> + std::panic::UnwindSafe,
{
    if ctx.abi_version != TONIC_RUNTIME_ABI_VERSION {
        let error = AbiError::new(
            AbiErrorCode::AbiVersionMismatch,
            format!(
                "runtime ABI version mismatch: expected {}, found {}",
                TONIC_RUNTIME_ABI_VERSION, ctx.abi_version
            ),
        );
        return call_result_error(TCallStatus::InvalidAbi, &error);
    }

    let args = match (*ctx).args() {
        Ok(values) => values,
        Err(error) => return call_result_error(TCallStatus::InvalidAbi, &error),
    };

    let mut runtime_args = Vec::with_capacity(args.len());
    for value in args {
        match tvalue_to_runtime(*value) {
            Ok(runtime) => runtime_args.push(runtime),
            Err(error) => return call_result_error(TCallStatus::Err, &error),
        }
    }

    match std::panic::catch_unwind(|| helper(&runtime_args)) {
        Ok(Ok(runtime_value)) => match runtime_to_tvalue(runtime_value) {
            Ok(encoded) => TCallResult {
                status: TCallStatus::Ok,
                value: encoded,
                error: TValue::nil(),
            },
            Err(error) => call_result_error(TCallStatus::Err, &error),
        },
        Ok(Err(error)) => call_result_error(TCallStatus::Err, &error),
        Err(_) => call_result_error(
            TCallStatus::Panic,
            &AbiError::new(AbiErrorCode::Panic, "panic in runtime helper boundary"),
        ),
    }
}

fn call_result_error(status: TCallStatus, error: &AbiError) -> TCallResult {
    TCallResult {
        status,
        value: TValue::nil(),
        error: runtime_to_tvalue(RuntimeValue::String(error.message.clone()))
            .unwrap_or_else(|_| TValue::nil()),
    }
}

fn runtime_tag(value: &RuntimeValue) -> TValueTag {
    match value {
        RuntimeValue::Int(_) => TValueTag::Int,
        RuntimeValue::Float(_) => TValueTag::Float,
        RuntimeValue::Bool(_) => TValueTag::Bool,
        RuntimeValue::Nil => TValueTag::Nil,
        RuntimeValue::String(_) => TValueTag::String,
        RuntimeValue::Atom(_) => TValueTag::Atom,
        RuntimeValue::ResultOk(_) => TValueTag::ResultOk,
        RuntimeValue::ResultErr(_) => TValueTag::ResultErr,
        RuntimeValue::Tuple(_, _) => TValueTag::Tuple2,
        RuntimeValue::Map(_) => TValueTag::Map,
        RuntimeValue::Keyword(_) => TValueTag::Keyword,
        RuntimeValue::List(_) => TValueTag::List,
        RuntimeValue::Range(_, _) => TValueTag::Range,
        RuntimeValue::SteppedRange(_, _, _) => TValueTag::Range, // fallback to range tag
        RuntimeValue::Closure(_) => TValueTag::Closure,
    }
}

fn is_heap_tag(tag: TValueTag) -> bool {
    !matches!(tag, TValueTag::Int | TValueTag::Bool | TValueTag::Nil)
}

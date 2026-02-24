use super::{AbiError, AbiErrorCode, TValueTag};
use crate::runtime::RuntimeValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone)]
struct HeapEntry {
    tag: TValueTag,
    value: RuntimeValue,
    refs: u32,
}

#[derive(Debug, Default)]
struct HeapStore {
    next_handle: u64,
    entries: HashMap<u64, HeapEntry>,
}

impl HeapStore {
    fn allocate_handle(&mut self) -> u64 {
        if self.next_handle == 0 {
            self.next_handle = 1;
        }

        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if self.next_handle == 0 {
            self.next_handle = 1;
        }
        handle
    }
}

static HEAP_STORE: LazyLock<Mutex<HeapStore>> = LazyLock::new(|| Mutex::new(HeapStore::default()));

pub(super) fn store(tag: TValueTag, value: RuntimeValue) -> Result<u64, AbiError> {
    let mut heap = lock_heap()?;
    let handle = heap.allocate_handle();
    heap.entries.insert(
        handle,
        HeapEntry {
            tag,
            value,
            refs: 1,
        },
    );
    Ok(handle)
}

pub(super) fn validate_handle(expected_tag: TValueTag, handle: u64) -> Result<(), AbiError> {
    let heap = lock_heap()?;
    let entry = heap
        .entries
        .get(&handle)
        .ok_or_else(|| invalid_handle_error(handle))?;

    if entry.tag != expected_tag {
        return Err(AbiError::new(
            AbiErrorCode::TagHandleMismatch,
            format!(
                "TValue tag {:?} does not match heap payload tag {:?}",
                expected_tag, entry.tag
            ),
        ));
    }

    Ok(())
}

pub(super) fn load(expected_tag: TValueTag, handle: u64) -> Result<RuntimeValue, AbiError> {
    let heap = lock_heap()?;
    let entry = heap
        .entries
        .get(&handle)
        .ok_or_else(|| invalid_handle_error(handle))?;

    if entry.tag != expected_tag {
        return Err(AbiError::new(
            AbiErrorCode::TagHandleMismatch,
            format!(
                "TValue tag {:?} does not match heap payload tag {:?}",
                expected_tag, entry.tag
            ),
        ));
    }

    Ok(entry.value.clone())
}

pub(super) fn retain(handle: u64) -> Result<(), AbiError> {
    let mut heap = lock_heap()?;
    let entry = heap
        .entries
        .get_mut(&handle)
        .ok_or_else(|| invalid_handle_error(handle))?;
    entry.refs = entry.refs.saturating_add(1);
    Ok(())
}

pub(super) fn release(handle: u64) -> Result<(), AbiError> {
    let mut heap = lock_heap()?;
    let should_remove = {
        let entry = heap
            .entries
            .get_mut(&handle)
            .ok_or_else(|| invalid_handle_error(handle))?;

        if entry.refs == 0 {
            return Err(AbiError::new(
                AbiErrorCode::OwnershipViolation,
                format!("heap handle {handle} already released"),
            ));
        }

        entry.refs -= 1;
        entry.refs == 0
    };

    if should_remove {
        heap.entries.remove(&handle);
    }

    Ok(())
}

fn lock_heap() -> Result<std::sync::MutexGuard<'static, HeapStore>, AbiError> {
    HEAP_STORE.lock().map_err(|_| {
        AbiError::new(
            AbiErrorCode::Panic,
            "runtime ABI heap lock poisoned by previous panic",
        )
    })
}

fn invalid_handle_error(handle: u64) -> AbiError {
    AbiError::new(
        AbiErrorCode::InvalidHandle,
        format!("unknown heap handle {handle}"),
    )
}

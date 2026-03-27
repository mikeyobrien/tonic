use rand::RngCore;

use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn host_uuid_v4(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if !args.is_empty() {
        return Err(HostError::new(format!(
            "Uuid.v4 expects no arguments, found {}",
            args.len()
        )));
    }

    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);

    // Set version 4 (bits 4-7 of byte 6)
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant RFC 4122 (bits 6-7 of byte 8)
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let uuid = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    );

    Ok(RuntimeValue::String(uuid))
}

pub fn register_uuid_host_functions(registry: &HostRegistry) {
    registry.register("uuid_v4", host_uuid_v4);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn call_v4() -> String {
        match HOST_REGISTRY
            .call("uuid_v4", &[])
            .expect("uuid_v4 should succeed")
        {
            RuntimeValue::String(s) => s,
            other => panic!("expected string, got {:?}", other),
        }
    }

    #[test]
    fn uuid_v4_format() {
        let uuid = call_v4();
        // 8-4-4-4-12 hex format
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5, "UUID should have 5 parts: {uuid}");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        assert_eq!(uuid.len(), 36, "UUID should be 36 chars: {uuid}");
    }

    #[test]
    fn uuid_v4_all_hex() {
        let uuid = call_v4();
        for ch in uuid.chars() {
            assert!(
                ch == '-' || ch.is_ascii_hexdigit(),
                "UUID should contain only hex digits and dashes: {uuid}"
            );
        }
    }

    #[test]
    fn uuid_v4_version_nibble() {
        let uuid = call_v4();
        // Version nibble is first char of the third group
        let version_char = uuid.split('-').nth(2).unwrap().chars().next().unwrap();
        assert_eq!(version_char, '4', "Version nibble should be 4: {uuid}");
    }

    #[test]
    fn uuid_v4_variant_bits() {
        let uuid = call_v4();
        // Variant nibble is first char of the fourth group, should be 8, 9, a, or b
        let variant_char = uuid.split('-').nth(3).unwrap().chars().next().unwrap();
        assert!(
            matches!(variant_char, '8' | '9' | 'a' | 'b'),
            "Variant nibble should be 8/9/a/b, got {variant_char}: {uuid}"
        );
    }

    #[test]
    fn uuid_v4_uniqueness() {
        let a = call_v4();
        let b = call_v4();
        assert_ne!(a, b, "Two UUIDs should be different");
    }

    #[test]
    fn uuid_v4_lowercase() {
        let uuid = call_v4();
        assert_eq!(uuid, uuid.to_lowercase(), "UUID should be lowercase");
    }

    #[test]
    fn uuid_v4_rejects_args() {
        let err = HOST_REGISTRY
            .call("uuid_v4", &[RuntimeValue::String("extra".to_string())])
            .expect_err("uuid_v4 should reject arguments");
        assert!(
            err.to_string().contains("no arguments"),
            "error should mention no arguments: {err}"
        );
    }
}

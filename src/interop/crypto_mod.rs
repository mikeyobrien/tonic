use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};

use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn expect_exact_args(
    function: &str,
    args: &[RuntimeValue],
    expected: usize,
) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

fn extract_string<'a>(function: &str, args: &'a [RuntimeValue]) -> Result<&'a str, HostError> {
    match &args[0] {
        RuntimeValue::String(s) => Ok(s.as_str()),
        other => Err(HostError::new(format!(
            "{} expects a string argument, found {}",
            function,
            host_value_kind(other)
        ))),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn host_crypto_sha256(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Crypto.sha256", args, 1)?;
    let s = extract_string("Crypto.sha256", args)?;
    let hash = Sha256::digest(s.as_bytes());
    Ok(RuntimeValue::String(hex_encode(&hash)))
}

fn host_crypto_hmac_sha256(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Crypto.hmac_sha256", args, 2)?;
    let key = match &args[0] {
        RuntimeValue::String(s) => s.as_str(),
        other => {
            return Err(HostError::new(format!(
                "Crypto.hmac_sha256 expects a string key as first argument, found {}",
                host_value_kind(other)
            )))
        }
    };
    let message = match &args[1] {
        RuntimeValue::String(s) => s.as_str(),
        other => {
            return Err(HostError::new(format!(
                "Crypto.hmac_sha256 expects a string message as second argument, found {}",
                host_value_kind(other)
            )))
        }
    };
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    Ok(RuntimeValue::String(hex_encode(&result.into_bytes())))
}

fn host_crypto_random_bytes(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Crypto.random_bytes", args, 1)?;
    let n = match &args[0] {
        RuntimeValue::Int(n) => {
            if *n < 0 {
                return Err(HostError::new(
                    "Crypto.random_bytes expects a non-negative integer",
                ));
            }
            *n as usize
        }
        other => {
            return Err(HostError::new(format!(
                "Crypto.random_bytes expects an integer argument, found {}",
                host_value_kind(other)
            )))
        }
    };
    if n > 1024 {
        return Err(HostError::new(
            "Crypto.random_bytes: maximum size is 1024 bytes",
        ));
    }
    let mut buf = vec![0u8; n];
    rand::rng().fill_bytes(&mut buf);
    Ok(RuntimeValue::String(hex_encode(&buf)))
}

pub fn register_crypto_host_functions(registry: &HostRegistry) {
    registry.register("crypto_sha256", host_crypto_sha256);
    registry.register("crypto_hmac_sha256", host_crypto_hmac_sha256);
    registry.register("crypto_random_bytes", host_crypto_random_bytes);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    fn i(n: i64) -> RuntimeValue {
        RuntimeValue::Int(n)
    }

    #[test]
    fn sha256_empty_string() {
        let result = HOST_REGISTRY
            .call("crypto_sha256", &[s("")])
            .expect("sha256 should succeed");
        // SHA-256 of empty string is well-known
        assert_eq!(
            result,
            s("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }

    #[test]
    fn sha256_hello() {
        let result = HOST_REGISTRY
            .call("crypto_sha256", &[s("hello")])
            .expect("sha256 should succeed");
        assert_eq!(
            result,
            s("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
    }

    #[test]
    fn sha256_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("crypto_sha256", &[i(42)])
            .expect_err("sha256 should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }

    #[test]
    fn hmac_sha256_known_vector() {
        // HMAC-SHA256("key", "The quick brown fox jumps over the lazy dog")
        let result = HOST_REGISTRY
            .call(
                "crypto_hmac_sha256",
                &[s("key"), s("The quick brown fox jumps over the lazy dog")],
            )
            .expect("hmac_sha256 should succeed");
        assert_eq!(
            result,
            s("f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8")
        );
    }

    #[test]
    fn hmac_sha256_empty_message() {
        let result = HOST_REGISTRY
            .call("crypto_hmac_sha256", &[s("secret"), s("")])
            .expect("hmac_sha256 should succeed");
        // Just verify it returns a 64-char hex string
        if let RuntimeValue::String(ref hex) = result {
            assert_eq!(hex.len(), 64, "HMAC-SHA256 should produce 64 hex chars");
        } else {
            panic!("expected string result");
        }
    }

    #[test]
    fn hmac_sha256_wrong_key_type() {
        let err = HOST_REGISTRY
            .call("crypto_hmac_sha256", &[i(42), s("msg")])
            .expect_err("hmac should fail on non-string key");
        assert!(
            err.to_string().contains("string key"),
            "error should mention string key: {err}"
        );
    }

    #[test]
    fn hmac_sha256_wrong_message_type() {
        let err = HOST_REGISTRY
            .call("crypto_hmac_sha256", &[s("key"), i(42)])
            .expect_err("hmac should fail on non-string message");
        assert!(
            err.to_string().contains("string message"),
            "error should mention string message: {err}"
        );
    }

    #[test]
    fn random_bytes_returns_correct_length() {
        let result = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(16)])
            .expect("random_bytes should succeed");
        if let RuntimeValue::String(ref hex) = result {
            assert_eq!(hex.len(), 32, "16 bytes = 32 hex chars");
        } else {
            panic!("expected string result");
        }
    }

    #[test]
    fn random_bytes_zero() {
        let result = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(0)])
            .expect("random_bytes(0) should succeed");
        assert_eq!(result, s(""));
    }

    #[test]
    fn random_bytes_negative_returns_error() {
        let err = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(-1)])
            .expect_err("random_bytes should fail on negative");
        assert!(
            err.to_string().contains("non-negative"),
            "error should mention non-negative: {err}"
        );
    }

    #[test]
    fn random_bytes_too_large_returns_error() {
        let err = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(2000)])
            .expect_err("random_bytes should fail on >1024");
        assert!(
            err.to_string().contains("maximum"),
            "error should mention maximum: {err}"
        );
    }

    #[test]
    fn random_bytes_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("crypto_random_bytes", &[s("16")])
            .expect_err("random_bytes should fail on string");
        assert!(
            err.to_string().contains("integer"),
            "error should mention integer: {err}"
        );
    }

    #[test]
    fn random_bytes_produces_different_outputs() {
        let a = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(32)])
            .expect("random_bytes should succeed");
        let b = HOST_REGISTRY
            .call("crypto_random_bytes", &[i(32)])
            .expect("random_bytes should succeed");
        assert_ne!(a, b, "two random calls should produce different output");
    }
}

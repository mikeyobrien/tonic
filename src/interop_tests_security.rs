use super::*;

#[test]
fn host_registry_system_random_token_returns_url_safe_base64() {
    let result = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(32)])
        .expect("sys_random_token should succeed for valid byte count");

    let RuntimeValue::String(token) = result else {
        panic!("expected string result from sys_random_token");
    };

    assert!(
        token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "token should contain only base64url characters, got: {token}"
    );
}

#[test]
fn host_registry_system_random_token_has_correct_output_length() {
    // 32 bytes → ceil(32 * 4 / 3) = 43 chars in base64url unpadded
    let result = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(32)])
        .expect("sys_random_token should succeed");

    let RuntimeValue::String(token) = result else {
        panic!("expected string result");
    };

    assert_eq!(
        token.len(),
        43,
        "32 bytes should produce 43 base64url chars, got {}",
        token.len()
    );

    // 16 bytes → 22 chars
    let result16 = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(16)])
        .expect("sys_random_token should succeed for 16 bytes");

    let RuntimeValue::String(token16) = result16 else {
        panic!("expected string result");
    };

    assert_eq!(
        token16.len(),
        22,
        "16 bytes should produce 22 base64url chars, got {}",
        token16.len()
    );
}

#[test]
fn host_registry_system_random_token_produces_unique_outputs() {
    let result1 = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(32)])
        .expect("first call should succeed");
    let result2 = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(32)])
        .expect("second call should succeed");

    assert_ne!(result1, result2, "two random tokens should not be equal");
}

#[test]
fn host_registry_system_random_token_rejects_bytes_below_minimum() {
    let error = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(8)])
        .expect_err("sys_random_token should reject byte count below minimum");

    assert_eq!(
        error.to_string(),
        "host error: sys_random_token bytes out of range: 8"
    );
}

#[test]
fn host_registry_system_random_token_rejects_bytes_above_maximum() {
    let error = HOST_REGISTRY
        .call("sys_random_token", &[RuntimeValue::Int(512)])
        .expect_err("sys_random_token should reject byte count above maximum");

    assert_eq!(
        error.to_string(),
        "host error: sys_random_token bytes out of range: 512"
    );
}

#[test]
fn host_registry_system_random_token_rejects_non_int_argument() {
    let error = HOST_REGISTRY
        .call(
            "sys_random_token",
            &[RuntimeValue::String("32".to_string())],
        )
        .expect_err("sys_random_token should reject non-int argument");

    assert_eq!(
        error.to_string(),
        "host error: sys_random_token expects int argument 1; found string"
    );
}

#[test]
fn host_registry_system_random_token_rejects_wrong_arity() {
    let error = HOST_REGISTRY
        .call("sys_random_token", &[])
        .expect_err("sys_random_token should reject zero arguments");

    assert_eq!(
        error.to_string(),
        "host error: sys_random_token expects exactly 1 argument, found 0"
    );
}

// ---- sys_hmac_sha256_hex tests ----

#[test]
fn host_registry_system_hmac_sha256_hex_matches_known_test_vector() {
    // RFC 4231 Test Case 2
    let result = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::String("Jefe".to_string()),
                RuntimeValue::String("what do ya want for nothing?".to_string()),
            ],
        )
        .expect("sys_hmac_sha256_hex should succeed for valid inputs");

    assert_eq!(
        result,
        RuntimeValue::String(
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843".to_string()
        )
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_produces_64_char_lowercase_hex() {
    let result = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::String("secret".to_string()),
                RuntimeValue::String("message".to_string()),
            ],
        )
        .expect("sys_hmac_sha256_hex should succeed");

    let RuntimeValue::String(hex) = result else {
        panic!("expected string result from sys_hmac_sha256_hex");
    };

    assert_eq!(
        hex.len(),
        64,
        "HMAC-SHA256 hex digest should be exactly 64 chars"
    );
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "digest should be lowercase hex only, got: {hex}"
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_rejects_empty_secret() {
    let error = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::String(String::new()),
                RuntimeValue::String("message".to_string()),
            ],
        )
        .expect_err("sys_hmac_sha256_hex should reject empty secret");

    assert_eq!(
        error.to_string(),
        "host error: sys_hmac_sha256_hex secret must not be empty"
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_rejects_empty_message() {
    let error = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::String("secret".to_string()),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect_err("sys_hmac_sha256_hex should reject empty message");

    assert_eq!(
        error.to_string(),
        "host error: sys_hmac_sha256_hex message must not be empty"
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_rejects_non_string_secret() {
    let error = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::Int(42),
                RuntimeValue::String("message".to_string()),
            ],
        )
        .expect_err("sys_hmac_sha256_hex should reject non-string secret");

    assert_eq!(
        error.to_string(),
        "host error: sys_hmac_sha256_hex expects string argument 1; found int"
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_rejects_non_string_message() {
    let error = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[
                RuntimeValue::String("secret".to_string()),
                RuntimeValue::Bool(true),
            ],
        )
        .expect_err("sys_hmac_sha256_hex should reject non-string message");

    assert_eq!(
        error.to_string(),
        "host error: sys_hmac_sha256_hex expects string argument 2; found bool"
    );
}

#[test]
fn host_registry_system_hmac_sha256_hex_rejects_wrong_arity() {
    let error = HOST_REGISTRY
        .call(
            "sys_hmac_sha256_hex",
            &[RuntimeValue::String("only-one".to_string())],
        )
        .expect_err("sys_hmac_sha256_hex should reject wrong arity");

    assert_eq!(
        error.to_string(),
        "host error: sys_hmac_sha256_hex expects exactly 2 arguments, found 1"
    );
}

// ---- sys_constant_time_eq + sys_discord_ed25519_verify tests ----

#[test]
fn host_registry_system_constant_time_eq_reports_match_and_mismatch() {
    let equal = HOST_REGISTRY
        .call(
            "sys_constant_time_eq",
            &[
                RuntimeValue::String("discord-signature".to_string()),
                RuntimeValue::String("discord-signature".to_string()),
            ],
        )
        .expect("sys_constant_time_eq should support equal strings");

    let different = HOST_REGISTRY
        .call(
            "sys_constant_time_eq",
            &[
                RuntimeValue::String("discord-signature".to_string()),
                RuntimeValue::String("discord-signature-x".to_string()),
            ],
        )
        .expect("sys_constant_time_eq should support mismatched strings");

    assert_eq!(equal, RuntimeValue::Bool(true));
    assert_eq!(different, RuntimeValue::Bool(false));
}

#[cfg(feature = "network")]
#[test]
fn host_registry_system_discord_ed25519_verify_accepts_valid_signature() {
    let result = HOST_REGISTRY
        .call(
            "sys_discord_ed25519_verify",
            &[
                RuntimeValue::String(
                    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
                        .to_string(),
                ),
                RuntimeValue::String(
                    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
                        .to_string(),
                ),
                RuntimeValue::String(String::new()),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect("sys_discord_ed25519_verify should accept valid test-vector signatures");

    assert_eq!(result, RuntimeValue::Bool(true));
}

#[cfg(feature = "network")]
#[test]
fn host_registry_system_discord_ed25519_verify_returns_false_for_invalid_signature() {
    let result = HOST_REGISTRY
        .call(
            "sys_discord_ed25519_verify",
            &[
                RuntimeValue::String(
                    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
                        .to_string(),
                ),
                RuntimeValue::String(
                    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100c"
                        .to_string(),
                ),
                RuntimeValue::String(String::new()),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect("sys_discord_ed25519_verify should return false for invalid signatures");

    assert_eq!(result, RuntimeValue::Bool(false));
}

#[cfg(feature = "network")]
#[test]
fn host_registry_system_discord_ed25519_verify_rejects_malformed_signature_hex() {
    let error = HOST_REGISTRY
        .call(
            "sys_discord_ed25519_verify",
            &[
                RuntimeValue::String(
                    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a".to_string(),
                ),
                RuntimeValue::String("abcd".to_string()),
                RuntimeValue::String("1700000000".to_string()),
                RuntimeValue::String("{}".to_string()),
            ],
        )
        .expect_err("sys_discord_ed25519_verify should reject malformed signature hex");

    assert_eq!(
        error.to_string(),
        "host error: sys_discord_ed25519_verify signature_hex must be 128 hex chars, found 4"
    );
}

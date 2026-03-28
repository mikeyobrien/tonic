use super::*;

pub(super) fn host_sys_list_files_recursive(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_list_files_recursive", args, 1)?;
    let path = expect_string_arg("sys_list_files_recursive", args, 0)?;

    if path.is_empty() {
        return Err(HostError::new(
            "sys_list_files_recursive path must not be empty",
        ));
    }

    let root_path = Path::new(&path);
    let mut files = Vec::new();

    collect_relative_files_recursive(root_path, root_path, &mut files)?;

    Ok(RuntimeValue::List(
        files.into_iter().map(RuntimeValue::String).collect(),
    ))
}

pub(super) fn host_sys_ensure_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_ensure_dir", args, 1)?;
    let path = expect_string_arg("sys_ensure_dir", args, 0)?;

    std::fs::create_dir_all(&path).map_err(|error| {
        HostError::new(format!("sys_ensure_dir failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn remove_tree(path: &Path, display_path: &str) -> Result<bool, HostError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            )))
        }
    };

    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path).map_err(|error| {
            HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            ))
        })?;
    } else {
        std::fs::remove_file(path).map_err(|error| {
            HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            ))
        })?;
    }

    Ok(true)
}

pub(super) fn host_sys_remove_tree(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_remove_tree", args, 1)?;
    let path = expect_string_arg("sys_remove_tree", args, 0)?;

    if path.is_empty() {
        return Err(HostError::new("sys_remove_tree path must not be empty"));
    }

    Ok(RuntimeValue::Bool(remove_tree(Path::new(&path), &path)?))
}

fn ensure_parent_directory(function: &str, path: &Path) -> Result<(), HostError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                HostError::new(format!(
                    "{function} failed to create parent directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }
    }

    Ok(())
}

fn atomic_temp_path(target: &Path) -> PathBuf {
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let base_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let temp_name = format!(".{base_name}.tmp.{pid}.{timestamp_nanos}");

    if let Some(parent) = target.parent() {
        if parent.as_os_str().is_empty() {
            PathBuf::from(temp_name)
        } else {
            parent.join(temp_name)
        }
    } else {
        PathBuf::from(temp_name)
    }
}

fn write_text_atomic(function: &str, path: &str, content: &str) -> Result<(), HostError> {
    let target = Path::new(path);
    ensure_parent_directory(function, target)?;

    let temp_path = atomic_temp_path(target);
    let write_result = (|| -> Result<(), std::io::Error> {
        let mut temp_file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;

        temp_file.write_all(content.as_bytes())?;
        temp_file.sync_all()?;
        std::fs::rename(&temp_path, target)?;
        Ok(())
    })();

    match write_result {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(HostError::new(format!(
                "{function} failed for '{}': {error}",
                path
            )))
        }
    }
}

pub(super) fn host_sys_write_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text", args, 2)?;
    let path = expect_string_arg("sys_write_text", args, 0)?;
    let content = expect_string_arg("sys_write_text", args, 1)?;

    std::fs::write(&path, content).map_err(|error| {
        HostError::new(format!("sys_write_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

pub(super) fn host_sys_append_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_append_text", args, 2)?;
    let path = expect_string_arg("sys_append_text", args, 0)?;
    let content = expect_string_arg("sys_append_text", args, 1)?;
    let target = Path::new(&path);

    ensure_parent_directory("sys_append_text", target)?;

    let mut sink = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)
        .map_err(|error| {
            HostError::new(format!("sys_append_text failed for '{}': {error}", path))
        })?;

    sink.write_all(content.as_bytes()).map_err(|error| {
        HostError::new(format!("sys_append_text failed for '{}': {error}", path))
    })?;
    sink.sync_data().map_err(|error| {
        HostError::new(format!("sys_append_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

pub(super) fn host_sys_write_text_atomic(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text_atomic", args, 2)?;
    let path = expect_string_arg("sys_write_text_atomic", args, 0)?;
    let content = expect_string_arg("sys_write_text_atomic", args, 1)?;

    write_text_atomic("sys_write_text_atomic", &path, &content)?;

    Ok(RuntimeValue::Bool(true))
}

pub(super) fn host_sys_lock_acquire(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_lock_acquire", args, 1)?;
    let path = expect_string_arg("sys_lock_acquire", args, 0)?;
    let target = Path::new(&path);

    ensure_parent_directory("sys_lock_acquire", target)?;

    let lock_attempt = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target);

    let mut handle = match lock_attempt {
        Ok(handle) => handle,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Ok(RuntimeValue::Bool(false));
        }
        Err(error) => {
            return Err(HostError::new(format!(
                "sys_lock_acquire failed for '{}': {error}",
                path
            )));
        }
    };

    let marker = format!(
        "pid={} timestamp_ms={}\n",
        std::process::id(),
        unix_timestamp_ms()
    );

    handle.write_all(marker.as_bytes()).map_err(|error| {
        HostError::new(format!("sys_lock_acquire failed for '{}': {error}", path))
    })?;
    handle.sync_all().map_err(|error| {
        HostError::new(format!("sys_lock_acquire failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

pub(super) fn host_sys_lock_release(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_lock_release", args, 1)?;
    let path = expect_string_arg("sys_lock_release", args, 0)?;

    match std::fs::remove_file(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(RuntimeValue::Bool(false)),
        Err(error) => Err(HostError::new(format!(
            "sys_lock_release failed for '{}': {error}",
            path
        ))),
    }
}

pub(super) fn host_sys_read_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_text", args, 1)?;
    let path = expect_string_arg("sys_read_text", args, 0)?;

    let content = std::fs::read_to_string(&path)
        .map_err(|error| HostError::new(format!("sys_read_text failed for '{}': {error}", path)))?;

    Ok(RuntimeValue::String(content))
}

pub(super) fn host_sys_read_stdin(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_stdin", args, 0)?;

    let stdin = read_host_stdin_to_end()
        .map_err(|error| HostError::new(format!("sys_read_stdin failed: {error}")))?;

    Ok(RuntimeValue::String(stdin))
}

#[cfg(feature = "network")]
pub(super) fn host_sys_http_request(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_request", args, 5)?;

    let method_value = expect_string_arg("sys_http_request", args, 0)?;
    let method = parse_http_method(&method_value)?;

    let url_value = expect_string_arg("sys_http_request", args, 1)?;
    let url = reqwest::Url::parse(&url_value)
        .map_err(|_| HostError::new(format!("sys_http_request invalid url: {url_value}")))?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(HostError::new(format!(
                "sys_http_request unsupported url scheme: {other}"
            )));
        }
    }

    let headers_value = expect_list_arg("sys_http_request", args, 2)?;
    let headers = parse_http_headers(&headers_value)?;

    let body = expect_string_arg("sys_http_request", args, 3)?;

    let opts_value = expect_map_arg("sys_http_request", args, 4)?;
    let opts = parse_http_opts(&opts_value)?;

    let redirect_policy = if opts.follow_redirects {
        reqwest::redirect::Policy::limited(opts.max_redirects as usize)
    } else {
        reqwest::redirect::Policy::none()
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(opts.timeout_ms as u64))
        .redirect(redirect_policy)
        .build()
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    let mut request = client.request(method, url.clone());
    for (name, value) in headers {
        request = request.header(name, value);
    }

    let response = request
        .body(body)
        .send()
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    let status = response.status().as_u16() as i64;
    let final_url = response.url().to_string();

    let mut response_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            let rendered_value = value.to_str().unwrap_or_default().to_string();
            (name.as_str().to_ascii_lowercase(), rendered_value)
        })
        .collect::<Vec<_>>();
    response_headers.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

    let mut body_bytes = Vec::new();
    let mut limited_reader = response.take((opts.max_response_bytes as u64) + 1);
    limited_reader
        .read_to_end(&mut body_bytes)
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    if body_bytes.len() > opts.max_response_bytes as usize {
        return Err(HostError::new(format!(
            "sys_http_request response exceeded max_response_bytes: {}",
            opts.max_response_bytes
        )));
    }

    let response_body = String::from_utf8_lossy(&body_bytes).into_owned();

    Ok(map_with_atom_keys(vec![
        ("status", RuntimeValue::Int(status)),
        (
            "headers",
            RuntimeValue::List(
                response_headers
                    .into_iter()
                    .map(|(name, value)| tuple_string_pair(name, value))
                    .collect(),
            ),
        ),
        ("body", RuntimeValue::String(response_body)),
        ("final_url", RuntimeValue::String(final_url)),
    ]))
}

pub(super) fn host_sys_env(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_env", args, 1)?;
    let key = expect_string_arg("sys_env", args, 0)?;

    let value = std::env::var_os(&key)
        .map(|v| RuntimeValue::String(v.to_string_lossy().into_owned()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

pub(super) fn host_sys_which(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_which", args, 1)?;
    let command = expect_string_arg("sys_which", args, 0)?;

    let value = find_command_on_path(&command)
        .map(|path| RuntimeValue::String(path.display().to_string()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

pub(super) fn host_sys_cwd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_cwd", args, 0)?;

    let cwd = std::env::current_dir()
        .map_err(|error| HostError::new(format!("sys_cwd failed to read current dir: {error}")))?;

    Ok(RuntimeValue::String(cwd.display().to_string()))
}

pub(super) fn host_sys_argv(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_argv", args, 0)?;

    let argv_list = std::env::args()
        .map(RuntimeValue::String)
        .collect::<Vec<_>>();

    Ok(RuntimeValue::List(argv_list))
}

pub(super) fn host_sys_constant_time_eq(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_constant_time_eq", args, 2)?;
    let left = expect_string_arg("sys_constant_time_eq", args, 0)?;
    let right = expect_string_arg("sys_constant_time_eq", args, 1)?;

    let equal = left.as_bytes().ct_eq(right.as_bytes()).unwrap_u8() == 1;
    Ok(RuntimeValue::Bool(equal))
}

#[cfg(feature = "network")]
pub(super) fn host_sys_discord_ed25519_verify(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_discord_ed25519_verify", args, 4)?;
    let public_key_hex = expect_string_arg("sys_discord_ed25519_verify", args, 0)?;
    let signature_hex = expect_string_arg("sys_discord_ed25519_verify", args, 1)?;
    let timestamp = expect_string_arg("sys_discord_ed25519_verify", args, 2)?;
    let body = expect_string_arg("sys_discord_ed25519_verify", args, 3)?;

    let public_key_bytes = decode_fixed_hex::<ED25519_PUBLIC_KEY_BYTES>(
        "sys_discord_ed25519_verify",
        "public_key_hex",
        &public_key_hex,
    )?;
    let signature_bytes = decode_fixed_hex::<ED25519_SIGNATURE_BYTES>(
        "sys_discord_ed25519_verify",
        "signature_hex",
        &signature_hex,
    )?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|error| {
        HostError::new(format!(
            "sys_discord_ed25519_verify invalid public_key_hex bytes: {error}"
        ))
    })?;

    let signature = Signature::from_bytes(&signature_bytes);

    let mut signed_payload = String::with_capacity(timestamp.len() + body.len());
    signed_payload.push_str(&timestamp);
    signed_payload.push_str(&body);

    let is_valid = verifying_key
        .verify(signed_payload.as_bytes(), &signature)
        .is_ok();

    Ok(RuntimeValue::Bool(is_valid))
}

pub(super) fn host_sys_random_token(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_random_token", args, 1)?;
    let bytes = expect_int_arg("sys_random_token", args, 0)?;

    if !(RANDOM_TOKEN_MIN_BYTES..=RANDOM_TOKEN_MAX_BYTES).contains(&bytes) {
        return Err(HostError::new(format!(
            "sys_random_token bytes out of range: {bytes}"
        )));
    }

    let mut buffer = vec![0u8; bytes as usize];
    rand::rng().fill_bytes(&mut buffer);

    Ok(RuntimeValue::String(URL_SAFE_NO_PAD.encode(&buffer)))
}

pub(super) fn host_sys_hmac_sha256_hex(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_hmac_sha256_hex", args, 2)?;
    let secret = expect_string_arg("sys_hmac_sha256_hex", args, 0)?;
    let message = expect_string_arg("sys_hmac_sha256_hex", args, 1)?;

    if secret.is_empty() {
        return Err(HostError::new(
            "sys_hmac_sha256_hex secret must not be empty",
        ));
    }

    if message.is_empty() {
        return Err(HostError::new(
            "sys_hmac_sha256_hex message must not be empty",
        ));
    }

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| HostError::new(format!("sys_hmac_sha256_hex failed: {e}")))?;
    mac.update(message.as_bytes());

    let result = mac.finalize();
    let hex: String = result
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    Ok(RuntimeValue::String(hex))
}

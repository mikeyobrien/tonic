use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use rand::RngCore;
use reqwest::Method;
use sha2::Sha256;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const RANDOM_TOKEN_MIN_BYTES: i64 = 16;
const RANDOM_TOKEN_MAX_BYTES: i64 = 256;

const HTTP_TIMEOUT_DEFAULT_MS: i64 = 30_000;
const HTTP_TIMEOUT_MIN_MS: i64 = 100;
const HTTP_TIMEOUT_MAX_MS: i64 = 120_000;
const HTTP_MAX_RESPONSE_DEFAULT_BYTES: i64 = 2_097_152;
const HTTP_MAX_RESPONSE_MAX_BYTES: i64 = 8_388_608;
const HTTP_FOLLOW_REDIRECTS_DEFAULT: bool = true;
const HTTP_MAX_REDIRECTS_DEFAULT: i64 = 3;
const HTTP_MAX_REDIRECTS_MAX: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpRequestOptions {
    timeout_ms: i64,
    max_response_bytes: i64,
    follow_redirects: bool,
    max_redirects: i64,
}

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

fn expect_string_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::String(text) => Ok(text.clone()),
        other => Err(HostError::new(format!(
            "{} expects string argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_int_arg(function: &str, args: &[RuntimeValue], index: usize) -> Result<i64, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Int(n) => Ok(*n),
        other => Err(HostError::new(format!(
            "{} expects int argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_list_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<RuntimeValue>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::List(items) => Ok(items.clone()),
        other => Err(HostError::new(format!(
            "{} expects list argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_map_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<(RuntimeValue, RuntimeValue)>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Map(entries) => Ok(entries.clone()),
        other => Err(HostError::new(format!(
            "{} expects map argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn atom_key(key: &str) -> RuntimeValue {
    RuntimeValue::Atom(key.to_string())
}

fn map_with_atom_keys(entries: Vec<(&str, RuntimeValue)>) -> RuntimeValue {
    RuntimeValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (atom_key(key), value))
            .collect(),
    )
}

fn tuple_string_pair(left: String, right: String) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::String(left)),
        Box::new(RuntimeValue::String(right)),
    )
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_command_on_path(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let direct = dir.join(name);
        if is_executable_file(&direct) {
            return Some(direct);
        }

        if cfg!(windows) {
            for ext in ["exe", "cmd", "bat"] {
                let with_ext = dir.join(format!("{name}.{ext}"));
                if is_executable_file(&with_ext) {
                    return Some(with_ext);
                }
            }
        }
    }

    None
}

fn parse_http_method(value: &str) -> Result<Method, HostError> {
    let upper = value.to_ascii_uppercase();
    if !matches!(
        upper.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD"
    ) {
        return Err(HostError::new(format!(
            "sys_http_request invalid method: {value}"
        )));
    }

    Method::from_bytes(upper.as_bytes())
        .map_err(|_| HostError::new(format!("sys_http_request invalid method: {value}")))
}

fn parse_http_headers(items: &[RuntimeValue]) -> Result<Vec<(String, String)>, HostError> {
    let mut headers = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let RuntimeValue::Tuple(name_value, header_value) = item else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} must be {{string, string}}; found {}",
                index + 1,
                host_value_kind(item)
            )));
        };

        let RuntimeValue::String(name) = name_value.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} expects string header name; found {}",
                index + 1,
                host_value_kind(name_value.as_ref())
            )));
        };

        let RuntimeValue::String(value) = header_value.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} expects string header value; found {}",
                index + 1,
                host_value_kind(header_value.as_ref())
            )));
        };

        headers.push((name.clone(), value.clone()));
    }

    Ok(headers)
}

fn parse_http_opts(
    entries: &[(RuntimeValue, RuntimeValue)],
) -> Result<HttpRequestOptions, HostError> {
    let mut opts = HttpRequestOptions {
        timeout_ms: HTTP_TIMEOUT_DEFAULT_MS,
        max_response_bytes: HTTP_MAX_RESPONSE_DEFAULT_BYTES,
        follow_redirects: HTTP_FOLLOW_REDIRECTS_DEFAULT,
        max_redirects: HTTP_MAX_REDIRECTS_DEFAULT,
    };

    for (key, value) in entries {
        let RuntimeValue::Atom(name) = key else {
            return Err(HostError::new(format!(
                "sys_http_request opts expects atom keys; found {}",
                host_value_kind(key)
            )));
        };

        match name.as_str() {
            "timeout_ms" => {
                let RuntimeValue::Int(timeout_ms) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.timeout_ms expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.timeout_ms = *timeout_ms;
            }
            "max_response_bytes" => {
                let RuntimeValue::Int(max_response_bytes) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.max_response_bytes expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.max_response_bytes = *max_response_bytes;
            }
            "follow_redirects" => {
                let RuntimeValue::Bool(follow_redirects) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.follow_redirects expects bool; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.follow_redirects = *follow_redirects;
            }
            "max_redirects" => {
                let RuntimeValue::Int(max_redirects) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.max_redirects expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.max_redirects = *max_redirects;
            }
            other => {
                return Err(HostError::new(format!(
                    "sys_http_request unsupported opts key: {other}"
                )));
            }
        }
    }

    if opts.timeout_ms < HTTP_TIMEOUT_MIN_MS || opts.timeout_ms > HTTP_TIMEOUT_MAX_MS {
        return Err(HostError::new(format!(
            "sys_http_request timeout_ms out of range: {}",
            opts.timeout_ms
        )));
    }

    if opts.max_response_bytes < 1 || opts.max_response_bytes > HTTP_MAX_RESPONSE_MAX_BYTES {
        return Err(HostError::new(format!(
            "sys_http_request max_response_bytes out of range: {}",
            opts.max_response_bytes
        )));
    }

    if opts.max_redirects < 0 || opts.max_redirects > HTTP_MAX_REDIRECTS_MAX {
        return Err(HostError::new(format!(
            "sys_http_request max_redirects out of range: {}",
            opts.max_redirects
        )));
    }

    Ok(opts)
}

fn host_sys_run(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_run", args, 1)?;
    let command = expect_string_arg("sys_run", args, 0)?;
    let shell_command = format!("{command} 2>&1");

    let output = std::process::Command::new("sh")
        .args(["-lc", &shell_command])
        .output()
        .map_err(|error| {
            HostError::new(format!("sys_run failed to execute shell command: {error}"))
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let combined_output = String::from_utf8_lossy(&output.stdout).into_owned();

    Ok(map_with_atom_keys(vec![
        ("exit_code", RuntimeValue::Int(exit_code as i64)),
        ("output", RuntimeValue::String(combined_output)),
    ]))
}

fn host_sys_path_exists(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_path_exists", args, 1)?;
    let path = expect_string_arg("sys_path_exists", args, 0)?;
    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

fn host_sys_ensure_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_ensure_dir", args, 1)?;
    let path = expect_string_arg("sys_ensure_dir", args, 0)?;

    std::fs::create_dir_all(&path).map_err(|error| {
        HostError::new(format!("sys_ensure_dir failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_write_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text", args, 2)?;
    let path = expect_string_arg("sys_write_text", args, 0)?;
    let content = expect_string_arg("sys_write_text", args, 1)?;

    std::fs::write(&path, content).map_err(|error| {
        HostError::new(format!("sys_write_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_read_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_text", args, 1)?;
    let path = expect_string_arg("sys_read_text", args, 0)?;

    let content = std::fs::read_to_string(&path)
        .map_err(|error| HostError::new(format!("sys_read_text failed for '{}': {error}", path)))?;

    Ok(RuntimeValue::String(content))
}

fn host_sys_read_stdin(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_stdin", args, 0)?;

    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|error| HostError::new(format!("sys_read_stdin failed: {error}")))?;

    Ok(RuntimeValue::String(
        String::from_utf8_lossy(&buffer).into_owned(),
    ))
}

fn host_sys_http_request(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

fn host_sys_env(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_env", args, 1)?;
    let key = expect_string_arg("sys_env", args, 0)?;

    let value = std::env::var_os(&key)
        .map(|v| RuntimeValue::String(v.to_string_lossy().into_owned()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_which(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_which", args, 1)?;
    let command = expect_string_arg("sys_which", args, 0)?;

    let value = find_command_on_path(&command)
        .map(|path| RuntimeValue::String(path.display().to_string()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_cwd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_cwd", args, 0)?;

    let cwd = std::env::current_dir()
        .map_err(|error| HostError::new(format!("sys_cwd failed to read current dir: {error}")))?;

    Ok(RuntimeValue::String(cwd.display().to_string()))
}

fn host_sys_argv(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_argv", args, 0)?;

    let argv_list = std::env::args()
        .map(RuntimeValue::String)
        .collect::<Vec<_>>();

    Ok(RuntimeValue::List(argv_list))
}

fn host_sys_random_token(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_random_token", args, 1)?;
    let bytes = expect_int_arg("sys_random_token", args, 0)?;

    if bytes < RANDOM_TOKEN_MIN_BYTES || bytes > RANDOM_TOKEN_MAX_BYTES {
        return Err(HostError::new(format!(
            "sys_random_token bytes out of range: {bytes}"
        )));
    }

    let mut buffer = vec![0u8; bytes as usize];
    rand::rng().fill_bytes(&mut buffer);

    Ok(RuntimeValue::String(URL_SAFE_NO_PAD.encode(&buffer)))
}

fn host_sys_hmac_sha256_hex(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

pub(super) fn register_system_host_functions(registry: &HostRegistry) {
    registry.register("sys_run", host_sys_run);
    registry.register("sys_path_exists", host_sys_path_exists);
    registry.register("sys_ensure_dir", host_sys_ensure_dir);
    registry.register("sys_write_text", host_sys_write_text);
    registry.register("sys_read_text", host_sys_read_text);
    registry.register("sys_read_stdin", host_sys_read_stdin);
    registry.register("sys_http_request", host_sys_http_request);
    registry.register("sys_env", host_sys_env);
    registry.register("sys_which", host_sys_which);
    registry.register("sys_cwd", host_sys_cwd);
    registry.register("sys_argv", host_sys_argv);
    registry.register("sys_random_token", host_sys_random_token);
    registry.register("sys_hmac_sha256_hex", host_sys_hmac_sha256_hex);
}

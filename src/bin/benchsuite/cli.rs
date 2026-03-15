use std::path::PathBuf;

use crate::model::{CliArgs, DEFAULT_TARGET_NAME};

pub fn parse_cli_args<I>(args: I) -> Result<CliArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut bin_path = PathBuf::from("target/release/tonic");
    let mut manifest_path = PathBuf::from("benchmarks/suite.toml");
    let mut runs: usize = 15;
    let mut warmup_runs: usize = 3;
    let mut enforce = false;
    let mut calibrate = false;
    let mut calibrate_margin_pct: u64 = 20;
    let mut json_out = PathBuf::from("benchmarks/summary.json");
    let mut markdown_out: Option<PathBuf> = None;
    let mut compile_latency_ms: Option<u64> = None;
    let mut target_name = DEFAULT_TARGET_NAME.to_string();

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--bin" => {
                let Some(value) = iter.next() else {
                    return Err("--bin requires a value".to_string());
                };
                bin_path = PathBuf::from(value);
            }
            "--manifest" => {
                let Some(value) = iter.next() else {
                    return Err("--manifest requires a value".to_string());
                };
                manifest_path = PathBuf::from(value);
            }
            "--runs" => {
                let Some(value) = iter.next() else {
                    return Err("--runs requires a value".to_string());
                };
                runs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --runs value '{value}' (expected integer)"))?;
                if runs == 0 {
                    return Err("--runs must be >= 1".to_string());
                }
            }
            "--warmup" => {
                let Some(value) = iter.next() else {
                    return Err("--warmup requires a value".to_string());
                };
                warmup_runs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --warmup value '{value}' (expected integer)"))?;
            }
            "--calibrate-margin-pct" => {
                let Some(value) = iter.next() else {
                    return Err("--calibrate-margin-pct requires a value".to_string());
                };
                calibrate_margin_pct = value.parse::<u64>().map_err(|_| {
                    format!("invalid --calibrate-margin-pct value '{value}' (expected integer)")
                })?;
            }
            "--json-out" => {
                let Some(value) = iter.next() else {
                    return Err("--json-out requires a value".to_string());
                };
                json_out = PathBuf::from(value);
            }
            "--markdown-out" => {
                let Some(value) = iter.next() else {
                    return Err("--markdown-out requires a value".to_string());
                };
                markdown_out = Some(PathBuf::from(value));
            }
            "--compile-latency-ms" => {
                let Some(value) = iter.next() else {
                    return Err("--compile-latency-ms requires a value".to_string());
                };
                let parsed = value.parse::<u64>().map_err(|_| {
                    format!("invalid --compile-latency-ms value '{value}' (expected integer)")
                })?;
                compile_latency_ms = Some(parsed);
            }
            "--target-name" => {
                let Some(value) = iter.next() else {
                    return Err("--target-name requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("--target-name requires a non-empty value".to_string());
                }
                target_name = value;
            }
            "--enforce" => enforce = true,
            "--calibrate" => calibrate = true,
            "-h" | "--help" => {
                return Err("__PRINT_HELP__".to_string());
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    Ok(CliArgs {
        bin_path,
        manifest_path,
        runs,
        warmup_runs,
        enforce,
        calibrate,
        calibrate_margin_pct,
        json_out,
        markdown_out,
        compile_latency_ms,
        target_name,
    })
}

pub fn help_text() -> String {
    "Usage:\n  benchsuite [--bin <path>] [--manifest <path>] [--runs <n>] [--warmup <n>] [--json-out <path>] [--markdown-out <path>] [--enforce] [--calibrate] [--calibrate-margin-pct <percent>] [--compile-latency-ms <ms>] [--target-name <name>]\n\nDefaults:\n  --bin target/release/tonic\n  --manifest benchmarks/suite.toml\n  --runs 15\n  --warmup 3\n  --calibrate-margin-pct 20\n  --target-name interpreter\n  --json-out benchmarks/summary.json\n"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_args_accepts_new_competitive_flags() {
        let args = vec![
            "--compile-latency-ms".to_string(),
            "1900".to_string(),
            "--target-name".to_string(),
            "native".to_string(),
        ];

        let parsed = parse_cli_args(args).expect("args should parse");
        assert_eq!(parsed.compile_latency_ms, Some(1900));
        assert_eq!(parsed.target_name, "native");
    }
}

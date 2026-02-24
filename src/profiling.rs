use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

const PROFILE_STDERR_ENV: &str = "TONIC_PROFILE_STDERR";
const PROFILE_OUT_ENV: &str = "TONIC_PROFILE_OUT";

#[derive(Debug)]
pub(crate) struct PhaseProfiler {
    command: &'static str,
    started_at: Instant,
    phases: Vec<PhaseSample>,
    emit_to_stderr: bool,
    output_path: Option<PathBuf>,
    flushed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PhaseSample {
    name: String,
    elapsed_ms: f64,
}

#[derive(Debug, Serialize)]
struct ProfileReport {
    command: String,
    total_ms: f64,
    phases: Vec<PhaseSample>,
}

impl PhaseProfiler {
    pub(crate) fn from_env(command: &'static str) -> Option<Self> {
        let emit_to_stderr = std::env::var_os(PROFILE_STDERR_ENV).is_some();
        let output_path = std::env::var_os(PROFILE_OUT_ENV).and_then(|value| {
            if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            }
        });

        if !emit_to_stderr && output_path.is_none() {
            return None;
        }

        Some(Self {
            command,
            started_at: Instant::now(),
            phases: Vec::new(),
            emit_to_stderr,
            output_path,
            flushed: false,
        })
    }

    pub(crate) fn measure<T>(&mut self, name: &str, run: impl FnOnce() -> T) -> T {
        let started_at = Instant::now();
        let output = run();
        let elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0;
        self.phases.push(PhaseSample {
            name: name.to_string(),
            elapsed_ms,
        });
        output
    }

    fn flush(&mut self) {
        if self.flushed {
            return;
        }

        let payload = match serde_json::to_string(&ProfileReport {
            command: self.command.to_string(),
            total_ms: self.started_at.elapsed().as_secs_f64() * 1000.0,
            phases: self.phases.clone(),
        }) {
            Ok(payload) => payload,
            Err(error) => {
                eprintln!("warning: failed to serialize profile output: {error}");
                self.flushed = true;
                return;
            }
        };

        if self.emit_to_stderr {
            eprintln!("{payload}");
        }

        if let Some(path) = &self.output_path {
            if let Some(parent) = path.parent() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    eprintln!(
                        "warning: failed to create profile directory {}: {error}",
                        parent.display()
                    );
                    self.flushed = true;
                    return;
                }
            }

            match OpenOptions::new().create(true).append(true).open(path) {
                Ok(mut file) => {
                    if let Err(error) = writeln!(file, "{payload}") {
                        eprintln!(
                            "warning: failed to write profile output {}: {error}",
                            path.display()
                        );
                    }
                }
                Err(error) => {
                    eprintln!(
                        "warning: failed to open profile output {}: {error}",
                        path.display()
                    );
                }
            }
        }

        self.flushed = true;
    }
}

impl Drop for PhaseProfiler {
    fn drop(&mut self) {
        self.flush();
    }
}

pub(crate) fn profile_phase<T>(
    profiler: &mut Option<PhaseProfiler>,
    phase_name: &str,
    run: impl FnOnce() -> T,
) -> T {
    if let Some(profiler) = profiler.as_mut() {
        profiler.measure(phase_name, run)
    } else {
        run()
    }
}

#[cfg(test)]
mod tests {
    use super::{profile_phase, PhaseProfiler};

    #[test]
    fn profile_phase_executes_without_profiler() {
        let mut profiler = None;
        let result = profile_phase(&mut profiler, "noop", || 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn profile_phase_records_sample_when_enabled() {
        let mut profiler = Some(PhaseProfiler {
            command: "unit",
            started_at: std::time::Instant::now(),
            phases: Vec::new(),
            emit_to_stderr: false,
            output_path: None,
            flushed: false,
        });

        let result = profile_phase(&mut profiler, "phase-a", || 5);
        assert_eq!(result, 5);
        assert_eq!(profiler.as_ref().map(|p| p.phases.len()), Some(1));
    }
}

pub(crate) const OBS_ENABLE_ENV: &str = "TONIC_OBS_ENABLE";
pub(crate) const OBS_DIR_ENV: &str = "TONIC_OBS_DIR";
pub(crate) const OBS_RUN_ID_ENV: &str = "TONIC_OBS_RUN_ID";
pub(crate) const OBS_TASK_ID_ENV: &str = "TONIC_OBS_TASK_ID";
pub(crate) const OBS_PARENT_RUN_ID_ENV: &str = "TONIC_OBS_PARENT_RUN_ID";

mod runtime;
mod schema;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use runtime::ObservabilityRun;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use schema::{ErrorSource, ObservabilityError};

#[cfg(test)]
mod tests;

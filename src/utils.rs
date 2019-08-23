use std::env;

/// Are we running in in a CI environment?
pub fn is_ci() -> bool {
    env::var("CI").is_ok() || env::var("TF_BUILD").is_ok()
}

//! Consumer-owned extension registry for `just x <name>` commands.

use template_core::cli::command::CommandSet;

/// Build this repository's intentionally empty extension registry.
///
/// # Errors
///
/// Returns a typed registration error if the controlled `x` router metadata
/// is invalid.
#[allow(
  clippy::single_call_fn,
  reason = "the registry builder is the deliberate seam between this consumer crate and the guarded stask runner: the runner façade \
            consumes it exactly once, and tests target it directly to pin the empty x surface"
)]
pub fn commands() -> template_stask::Result<CommandSet> {
  template_stask::empty_registry("strict-thiserror extensions")
}

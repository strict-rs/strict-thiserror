//! Probe nightly generic member access and generate the derive's versioned runtime module.

use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::io::Write;
use std::iter;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;

/// Versioned runtime exports referenced by macro expansions from this package release.
const PRIVATE: &str = "\
#[doc(hidden)]
pub mod __private$$ {
    #[doc(hidden)]
    pub use crate::private::{AsDisplay, AsDynError, Error, Var};
    #[cfg(feature = \"std\")]
    #[doc(hidden)]
    pub use crate::private::Backtrace;
    #[cfg(error_generic_member_access)]
    #[doc(hidden)]
    pub use crate::private::ThiserrorProvide;
}
";

/// Failure modes that abort this build script.
///
/// Returned from `main` so cargo prints the rendered diagnostic and fails the
/// build through the normal termination path instead of `process::exit`.
enum BuildScriptError {
  /// A cargo-provided environment variable was absent.
  MissingEnvVar {
    /// Name of the missing environment variable.
    key: String,
  },
  /// The probe subdirectory beneath `OUT_DIR` could not be created.
  CreateOutSubdir {
    /// The subdirectory that could not be created.
    path:   PathBuf,
    /// The underlying filesystem error.
    source: io::Error,
  },
  /// The probe subdirectory beneath `OUT_DIR` could not be removed.
  CleanupOutSubdir {
    /// The subdirectory that could not be removed.
    path:   PathBuf,
    /// The underlying filesystem error.
    source: io::Error,
    /// Native compiler result completed before cleanup failed.
    status: ExitStatus,
  },
  /// The configured compiler could not be started or waited for.
  CompileProbe {
    /// Complete native compiler command, including wrappers and arguments.
    command: Box<Command>,
    /// Original process failure.
    source:  io::Error,
  },
  /// Compiler execution and mandatory probe cleanup both failed.
  CompileProbeAndCleanup {
    /// Complete native compiler command.
    command: Box<Command>,
    /// Probe output directory whose cleanup failed.
    path:    PathBuf,
    /// Original compiler execution failure.
    source:  io::Error,
    /// Original cleanup failure retained alongside the compiler failure.
    cleanup: io::Error,
  },
  /// A `cargo:` directive could not be written to stdout.
  EmitDirective {
    /// The underlying write error.
    source: io::Error,
  },
  /// Cargo's package version was absent or was not valid Unicode.
  PackageVersion {
    /// Original Cargo environment failure.
    source: env::VarError,
  },
  /// The versioned runtime module could not be generated.
  GeneratePrivateModule {
    /// Destination of the generated module.
    path:   PathBuf,
    /// Original filesystem failure.
    source: io::Error,
  },
}

// The process termination path renders `Err` values from `main` with `Debug`,
// so this impl carries the human-readable diagnostic.
impl fmt::Debug for BuildScriptError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Self::MissingEnvVar {
        ref key,
      } => {
        write!(formatter, "Environment variable ${key} is not set during execution of build script")
      }
      Self::CreateOutSubdir {
        ref path,
        ref source,
      } => write!(formatter, "Failed to create {}: {}", path.display(), source),
      Self::CleanupOutSubdir {
        ref path,
        ref source,
        ref status,
      } => write!(
        formatter,
        "Failed to clean up {} after compiler status {status}: {source}",
        path.display()
      ),
      Self::CompileProbe {
        ref command,
        ref source,
      } => formatter
        .debug_struct("CompileProbe")
        .field("command", command)
        .field("source", source)
        .finish(),
      Self::CompileProbeAndCleanup {
        ref command,
        ref path,
        ref source,
        ref cleanup,
      } => formatter
        .debug_struct("CompileProbeAndCleanup")
        .field("command", command)
        .field("path", path)
        .field("source", source)
        .field("cleanup", cleanup)
        .finish(),
      Self::EmitDirective {
        ref source,
      } => write!(formatter, "Failed to write a cargo directive to stdout: {source}"),
      Self::PackageVersion {
        ref source,
      } => write!(formatter, "Invalid Cargo package version: {source}"),
      Self::GeneratePrivateModule {
        ref path,
        ref source,
      } => {
        write!(formatter, "Failed to generate {}: {source}", path.display())
      }
    }
  }
}

/// Emit Cargo's supported configuration and the release-specific macro runtime contract.
///
/// # Errors
/// Returns the original Cargo environment, generated-file, probe cleanup, or output failure.
#[allow(
  clippy::single_call_fn,
  reason = "Cargo requires this standalone process entrypoint to adapt its environment and stdout to the complete generator"
)]
fn main() -> Result<(), BuildScriptError> {
  generate(&|key: &str| env::var_os(key), &mut io::stdout().lock())
}

/// Generate the runtime module and Cargo directives from one environment reader and writer.
///
/// # Errors
/// Returns native environment, filesystem, process, cleanup, or directive-write failures.
#[allow(
  clippy::single_call_fn,
  reason = "the complete generator owns the ordered module, capability, cleanup, and Cargo directive workflow"
)]
fn generate(environment: &impl Fn(&str) -> Option<OsString>, output: &mut impl Write) -> Result<(), BuildScriptError> {
  emit(output, "cargo:rerun-if-changed=build/probe.rs")?;

  emit(output, "cargo:rustc-check-cfg=cfg(error_generic_member_access)")?;
  emit(output, "cargo:rustc-check-cfg=cfg(thiserror_nightly_testing)")?;
  let out_dir = PathBuf::from(cargo_env_var(environment, "OUT_DIR")?);
  let patch_version = environment("CARGO_PKG_VERSION_PATCH")
    .ok_or(env::VarError::NotPresent)
    .and_then(|package_version| package_version.into_string().map_err(env::VarError::NotUnicode))
    .map_err(|source| BuildScriptError::PackageVersion {
      source,
    })?;
  let module = PRIVATE.replace("$$", &patch_version);
  let path = out_dir.join("private.rs");
  fs::write(&path, module).map_err(|source| BuildScriptError::GeneratePrivateModule {
    path,
    source,
  })?;

  let (error_generic_member_access, consider_rustc_bootstrap) =
    member_access_support(environment("RUSTC_BOOTSTRAP").as_deref(), |setting| {
      compile_probe(environment, setting)
    })?;

  if error_generic_member_access {
    emit(output, "cargo:rustc-cfg=error_generic_member_access")?;
  }

  if consider_rustc_bootstrap {
    emit(output, "cargo:rerun-if-env-changed=RUSTC_BOOTSTRAP")?;
  }

  Ok(())
}

/// Decide native member access and whether Cargo must watch bootstrap changes.
///
/// Probe without bootstrap first. A compiler that supports the API independently does not
/// depend on bootstrap, while restricted bootstrap failures remain sensitive to that input.
///
/// # Errors
/// Returns the original probe failure without running any later probe.
#[allow(
  clippy::single_call_fn,
  reason = "native capability and Cargo bootstrap sensitivity form one decision independent of compiler execution"
)]
fn member_access_support<E>(
  bootstrap: Option<&OsStr>,
  mut probe: impl FnMut(Option<&OsStr>) -> Result<bool, E>,
) -> Result<(bool, bool), E> {
  if probe(None)? {
    return Ok((true, false));
  }
  let Some(setting) = bootstrap else {
    return Ok((false, true));
  };
  if probe(Some(setting))? {
    return Ok((true, true));
  }
  Ok((false, setting != OsStr::new("1")))
}

/// Print one `cargo:` directive on the build script's stdout channel.
fn emit(output: &mut impl Write, directive: &str) -> Result<(), BuildScriptError> {
  writeln!(output, "{directive}").map_err(|source| BuildScriptError::EmitDirective {
    source,
  })
}

/// Compile the nightly capability probe and remove its temporary output.
///
/// # Errors
/// Returns missing Cargo environment or probe directory creation/cleanup failures.
#[allow(
  clippy::single_call_fn,
  reason = "a compiler probe owns allocation, native execution, and mandatory cleanup as one operation"
)]
fn compile_probe(environment: &impl Fn(&str) -> Option<OsString>, rustc_bootstrap: Option<&OsStr>) -> Result<bool, BuildScriptError> {
  if environment("RUSTC_STAGE").is_some() {
    // We are running inside rustc bootstrap. This is a highly non-standard
    // environment with issues such as:
    //
    //     https://github.com/rust-lang/cargo/issues/11138
    //     https://github.com/rust-lang/rust/issues/114839
    //
    // Let's just not use nightly features here.
    return Ok(false);
  }

  let rustc = cargo_env_var(environment, "RUSTC")?;
  let out_dir = cargo_env_var(environment, "OUT_DIR")?;
  let out_subdir = Path::new(&out_dir).join("probe");
  let probefile = Path::new("build").join("probe.rs");

  if let Err(err) = fs::create_dir_all(&out_subdir) {
    return Err(BuildScriptError::CreateOutSubdir {
      path:   out_subdir,
      source: err,
    });
  }

  let rustc_wrapper = environment("RUSTC_WRAPPER").filter(|wrapper| !wrapper.is_empty());
  let rustc_workspace_wrapper = environment("RUSTC_WORKSPACE_WRAPPER").filter(|wrapper| !wrapper.is_empty());
  let program = rustc_wrapper.as_ref().or(rustc_workspace_wrapper.as_ref()).unwrap_or(&rustc);
  let mut cmd = Command::new(program);
  let arguments = rustc_wrapper
    .into_iter()
    .chain(rustc_workspace_wrapper)
    .chain(iter::once(rustc.clone()))
    .skip(1);
  let target = environment("TARGET")
    .into_iter()
    .flat_map(|target| [OsString::from("--target"), target]);
  let flags = environment("CARGO_ENCODED_RUSTFLAGS")
    .and_then(|encoded_flags| encoded_flags.into_string().ok())
    .unwrap_or_default();
  let rustflags = flags.split('\x1f').filter(|flag| !flag.is_empty());
  let configured = cmd
    .args(arguments)
    .stderr(Stdio::null())
    .arg("--edition=2024")
    .arg("--crate-name=thiserror")
    .arg("--crate-type=lib")
    .arg("--emit=dep-info,metadata")
    .arg("--out-dir")
    .arg(&out_subdir)
    .arg(probefile)
    .args(target)
    .args(rustflags);
  let execution = if let Some(bootstrap_setting) = rustc_bootstrap {
    configured.env("RUSTC_BOOTSTRAP", bootstrap_setting).status()
  } else {
    configured.env_remove("RUSTC_BOOTSTRAP").status()
  };

  // Clean up to avoid leaving nondeterministic absolute paths in the dep-info
  // file in OUT_DIR, which causes nonreproducible builds in build systems
  // that treat the entire OUT_DIR as an artifact.
  let cleanup = fs::remove_dir_all(&out_subdir).or_else(tolerate_cleanup_race);
  finish_probe(cmd, out_subdir, execution, cleanup)
}

/// Accept a missing probe or the documented Linux NFS delayed-removal race.
#[allow(
  clippy::single_call_fn,
  reason = "probe cleanup has a precise platform race policy that must retain all other native failures"
)]
fn tolerate_cleanup_race(source: io::Error) -> io::Result<()> {
  if source.kind() == ErrorKind::NotFound || (cfg!(target_os = "linux") && source.kind() == ErrorKind::DirectoryNotEmpty) {
    Ok(())
  } else {
    Err(source)
  }
}

/// Preserve compiler completion and mandatory cleanup together at the operation boundary.
#[allow(
  clippy::single_call_fn,
  reason = "finalization preserves all four native compiler and cleanup outcomes without losing either failure"
)]
fn finish_probe(
  cmd: Command,
  out_subdir: PathBuf,
  execution: io::Result<ExitStatus>,
  cleanup: io::Result<()>,
) -> Result<bool, BuildScriptError> {
  match (execution, cleanup) {
    (Ok(status), Ok(())) => Ok(status.success()),
    (Err(source), Ok(())) => Err(BuildScriptError::CompileProbe {
      command: Box::new(cmd),
      source,
    }),
    (Ok(status), Err(source)) => Err(BuildScriptError::CleanupOutSubdir {
      path: out_subdir,
      source,
      status,
    }),
    (Err(source), Err(cleanup_error)) => Err(BuildScriptError::CompileProbeAndCleanup {
      command: Box::new(cmd),
      path: out_subdir,
      source,
      cleanup: cleanup_error,
    }),
  }
}

/// Read a Cargo-provided environment value without requiring Unicode.
///
/// # Errors
/// Returns the name of a Cargo environment value that is absent.
fn cargo_env_var(environment: &impl Fn(&str) -> Option<OsString>, key: &str) -> Result<OsString, BuildScriptError> {
  environment(key).ok_or_else(|| BuildScriptError::MissingEnvVar {
    key: key.to_owned()
  })
}

/// Build-script behavior uses isolated inputs and real native filesystem/process results.
#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;
  use std::collections::VecDeque;
  use std::env;
  use std::ffi::OsStr;
  use std::ffi::OsString;
  use std::fs;
  use std::io;
  use std::io::Cursor;
  use std::io::ErrorKind;
  use std::path::Path;
  use std::path::PathBuf;
  use std::process::Command;
  use std::process::Output;

  use strict_test_support::OptionFailure;
  use strict_test_support::PredicateFailure;
  use strict_test_support::TempDir;
  use strict_test_support::TestFailure;
  use strict_test_support::ensure_some;
  use strict_test_support::ensure_that;

  use super::BuildScriptError;
  use super::compile_probe;
  use super::finish_probe;
  use super::generate;
  use super::main;
  use super::member_access_support;
  use super::tolerate_cleanup_race;

  /// Cargo's fixed directives precede all environment-dependent generation.
  const BASE_DIRECTIVES: &[u8] = b"cargo:rerun-if-changed=build/probe.rs\ncargo:rustc-check-cfg=cfg(error_generic_member_access)\ncargo:rustc-check-cfg=cfg(thiserror_nightly_testing)\n";
  /// Native environment fixture preserving Unicode and non-Unicode values.
  type Environment = BTreeMap<&'static str, OsString>;
  /// Generated module observation retains its fixture, directives, completion, and metadata.
  type Generation = (TempDir, Vec<u8>, Result<(), BuildScriptError>, io::Result<fs::Metadata>);
  /// Probe observation retains its fixture, completion, and cleanup metadata.
  type Probe = (TempDir, Result<bool, BuildScriptError>, io::Result<fs::Metadata>);
  /// Entrypoint observation retains the native request, output, and generated module metadata.
  type Entrypoint = (TempDir, Command, io::Result<Output>, io::Result<fs::Metadata>);
  /// Native compiler output and complete probe finalization result.
  type Completion = (Output, Result<bool, BuildScriptError>);
  /// Original native errors, their rendered diagnostics, and required operation contexts.
  type DiagnosticCases = Vec<(BuildScriptError, String, Vec<&'static str>)>;
  /// The real compiler completion remains with the complete diagnostic batch.
  type Diagnostics = (Output, DiagnosticCases);
  /// Cleanup inputs, original native outcomes, and independent expected outcomes.
  type CleanupCases = Vec<(ErrorKind, io::Result<()>, Result<(), ErrorKind>)>;
  /// Failed policy result and the complete sequence of attempted probes.
  type PolicyFailure = (io::Result<(bool, bool)>, Vec<Option<OsString>>);
  /// Original writer and the rejected generation result.
  type DirectiveFailure = (Cursor<[u8; 0]>, Result<(), BuildScriptError>);
  /// Policy observation preserves inputs, calls, native remainder, result, and expectations.
  type Policy = (
    Option<OsString>,
    Vec<Option<OsString>>,
    VecDeque<bool>,
    Result<(bool, bool), OptionFailure<bool>>,
    (bool, bool),
    Vec<Option<OsString>>,
  );

  /// Fixture and native assertion failures remain separately typed.
  #[derive(Debug, thiserror::Error)]
  enum BuildTestFailure {
    /// Original fixture failure.
    #[error(transparent)]
    Fixture(#[from] TestFailure),
    /// Complete generated-module observation.
    #[error(transparent)]
    Generation(Box<PredicateFailure<Generation>>),
    /// Complete compiler and cleanup observation.
    #[error(transparent)]
    Probe(Box<PredicateFailure<Probe>>),
    /// Complete supported and rejected compiler observations.
    #[error(transparent)]
    ProbeBatch(Box<PredicateFailure<Vec<Probe>>>),
    /// Complete standalone entrypoint observation.
    #[error(transparent)]
    Entrypoint(Box<PredicateFailure<Entrypoint>>),
    /// Original compiler completion and failed cleanup.
    #[error(transparent)]
    Completion(Box<PredicateFailure<Completion>>),
    /// Complete native termination diagnostic observations.
    #[error(transparent)]
    Diagnostics(Box<PredicateFailure<Diagnostics>>),
  }

  /// Establish Cargo's real inputs without mutating the parent process environment.
  fn environment_for(fixture: &TempDir) -> Environment {
    BTreeMap::from([
      ("OUT_DIR", fixture.path().as_os_str().to_owned()),
      ("CARGO_PKG_VERSION_PATCH", OsString::from("7")),
      ("RUSTC", OsString::from("rustc")),
    ])
  }

  /// Observe actual generation while retaining every native input/output resource.
  fn generated(fixture: TempDir, environment: &Environment) -> Generation {
    let mut directives = Vec::new();
    let completion = generate(&|key| environment.get(key).cloned(), &mut directives);
    let metadata = fs::metadata(fixture.child("private.rs"));
    (fixture, directives, completion, metadata)
  }

  /// Probe-policy choices preserve call order and do not consume unused probes.
  #[test]
  fn member_access_policy_preserves_bootstrap_sensitivity() -> Result<(), PredicateFailure<Vec<Policy>>> {
    let cases = [
      (None, vec![true], (true, false), vec![None]),
      (None, vec![false], (false, true), vec![None]),
      (Some(OsString::from("1")), vec![false, true], (true, true), vec![
        None,
        Some(OsString::from("1")),
      ]),
      (Some(OsString::from("1")), vec![false, false], (false, false), vec![
        None,
        Some(OsString::from("1")),
      ]),
      (Some(OsString::from("thiserror")), vec![false, false], (false, true), vec![
        None,
        Some(OsString::from("thiserror")),
      ]),
    ];
    let observed = cases
      .into_iter()
      .map(|(setting, answers, expected, expected_calls)| {
        let mut remainder = VecDeque::from(answers);
        let mut calls = Vec::new();
        let result = member_access_support(setting.as_deref(), |bootstrap| {
          calls.push(bootstrap.map(OsStr::to_os_string));
          ensure_some(remainder.pop_front(), "the native probe fixture retains its next answer")
        });
        (setting, calls, remainder, result, expected, expected_calls)
      })
      .collect::<Vec<Policy>>();
    ensure_that(
      observed,
      "support and Cargo sensitivity follow the exact native probe sequence",
      |subjects| {
        subjects.iter().all(|subject| {
          (
            subject.2.is_empty(),
            subject.3.as_ref().is_ok_and(|result| *result == subject.4),
            &subject.1,
          ) == (true, true, &subject.5)
        })
      },
    )
    .map(drop)
  }

  /// A failed probe stops the policy before a later probe and retains its native error.
  #[test]
  fn member_access_policy_preserves_probe_failure() -> Result<(), PredicateFailure<PolicyFailure>> {
    let mut calls = Vec::new();
    let outcome = member_access_support(Some(OsStr::new("1")), |setting| {
      calls.push(setting.map(OsStr::to_os_string));
      Err(io::Error::from(ErrorKind::PermissionDenied))
    });
    ensure_that(
      (outcome, calls),
      "a failed first probe preserves its error and prevents a second probe",
      |subject| {
        (
          subject
            .0
            .as_ref()
            .is_err_and(|source| source.kind() == ErrorKind::PermissionDenied),
          subject.1.as_slice(),
        ) == (true, [None].as_slice())
      },
    )
    .map(drop)
  }

  /// Rust bootstrap skips unstable probing but still generates the required runtime module.
  #[test]
  fn generation_preserves_the_bootstrap_stage_boundary() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-stage")?;
    let mut inputs = environment_for(&fixture);
    inputs.extend([("RUSTC_STAGE", OsString::from("1"))]);
    let observed = generated(fixture, &inputs);
    ensure_that(
      observed,
      "bootstrap generation creates the runtime module and watches future bootstrap changes",
      |subject| {
        (
          subject.2.is_ok(),
          subject.3.as_ref().is_ok_and(fs::Metadata::is_file),
          subject.1.as_slice(),
          subject.0.child("probe").exists(),
        ) == (
          true,
          true,
          [BASE_DIRECTIVES, b"cargo:rerun-if-env-changed=RUSTC_BOOTSTRAP\n"]
            .concat()
            .as_slice(),
          false,
        )
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Generation)
  }

  /// A native supported compiler generates both the module and its capability directive.
  #[test]
  fn generation_enables_a_supported_compiler() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-supported")?;
    let mut inputs = environment_for(&fixture);
    inputs.extend([("RUSTC_BOOTSTRAP", OsString::from("1"))]);
    let observed = generated(fixture, &inputs);
    ensure_that(
      observed,
      "a supported compiler produces its capability directive and cleans the probe",
      |subject| {
        (
          subject.2.is_ok(),
          subject.3.as_ref().is_ok_and(fs::Metadata::is_file),
          subject.1.starts_with(BASE_DIRECTIVES),
          subject
            .1
            .split(|byte| *byte == b'\n')
            .any(|line| line == b"cargo:rustc-cfg=error_generic_member_access"),
          subject.0.child("probe").exists(),
        ) == (true, true, true, true, false)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Generation)
  }

  /// Invalid Cargo inputs fail at their original typed boundary, after the fixed directives.
  #[test]
  fn generation_retains_missing_cargo_inputs() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-missing")?;
    let mut inputs = environment_for(&fixture);
    drop(inputs.remove("CARGO_PKG_VERSION_PATCH"));
    let observed = generated(fixture, &inputs);
    ensure_that(
      observed,
      "a missing package version retains its native environment error and no generated file",
      |subject| {
        (
          matches!(
            subject.2,
            Err(BuildScriptError::PackageVersion {
              source: env::VarError::NotPresent,
            })
          ),
          subject.1.as_slice(),
          subject.3.is_err(),
        ) == (true, BASE_DIRECTIVES, true)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Generation)
  }

  /// Missing Cargo output authority is rejected before a module is written.
  #[test]
  fn generation_requires_a_cargo_output_directory() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-no-output")?;
    let mut inputs = environment_for(&fixture);
    drop(inputs.remove("OUT_DIR"));
    ensure_that(
      generated(fixture, &inputs),
      "missing output authority preserves its exact key and earlier directives",
      |subject| {
        (
          matches!(&subject.2, Err(BuildScriptError::MissingEnvVar { key }) if key == "OUT_DIR"),
          subject.1.as_slice(),
          subject.3.is_err(),
        ) == (true, BASE_DIRECTIVES, true)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Generation)
  }

  /// An occupied parent retains the original module destination and filesystem error.
  #[test]
  fn generation_preserves_module_write_failure() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-module-failure")?;
    let occupied = fixture.child("occupied");
    fs::write(&occupied, "occupied").map_err(TestFailure::from)?;
    let mut inputs = environment_for(&fixture);
    inputs.extend([("OUT_DIR", occupied.into_os_string())]);
    ensure_that(generated(fixture, &inputs), "module write failure retains its destination and native filesystem failure", |subject| {
      (
        matches!(&subject.2, Err(BuildScriptError::GeneratePrivateModule { path, source }) if (path.as_path(), source.kind()) == (subject.0.child("occupied").join("private.rs").as_path(), ErrorKind::NotADirectory)),
        subject.1.as_slice(), subject.3.is_err(),
      ) == (true, BASE_DIRECTIVES, true)
    }).map(drop).map_err(Box::new).map_err(BuildTestFailure::Generation)
  }

  /// Non-Unicode package versions retain their original native environment payload.
  #[cfg(unix)]
  #[test]
  fn generation_preserves_non_unicode_package_version() -> Result<(), BuildTestFailure> {
    use std::os::unix::ffi::OsStringExt as _;
    let fixture = TempDir::new("thiserror-build-non-unicode")?;
    let mut inputs = environment_for(&fixture);
    inputs.extend([("CARGO_PKG_VERSION_PATCH", OsString::from_vec(vec![0xFF]))]);
    ensure_that(generated(fixture, &inputs), "non-Unicode version rejection retains the native input bytes", |subject| {
      (matches!(&subject.2, Err(BuildScriptError::PackageVersion { source: env::VarError::NotUnicode(original) }) if original.as_encoded_bytes() == [0xFF]), subject.1.as_slice(), subject.3.is_err())
        == (true, BASE_DIRECTIVES, true)
    }).map(drop).map_err(Box::new).map_err(BuildTestFailure::Generation)
  }

  /// A rejected output writer preserves its native error and accepted-byte position.
  #[test]
  fn generation_preserves_directive_failure() -> Result<(), PredicateFailure<DirectiveFailure>> {
    let mut writer = Cursor::new([]);
    let outcome = generate(&|_key| None, &mut writer);
    ensure_that(
      (writer, outcome),
      "directive failure preserves the native writer and stops generation",
      |subject| {
        (
          subject.0.position(),
          matches!(&subject.1, Err(BuildScriptError::EmitDirective { source }) if source.kind() == ErrorKind::WriteZero),
        ) == (0, true)
      },
    )
    .map(drop)
  }

  /// Real compiler execution is successful with bootstrap and rejects an invalid target cleanly.
  #[test]
  fn compiler_probe_preserves_support_and_rejection() -> Result<(), BuildTestFailure> {
    let fixtures = [
      TempDir::new("thiserror-build-supported")?,
      TempDir::new("thiserror-build-rejected")?,
    ];
    let observed = fixtures
      .into_iter()
      .zip([None, Some("thiserror-invalid-target")])
      .map(|(fixture, target)| {
        let mut inputs = environment_for(&fixture);
        inputs.extend([
          ("RUSTC_WRAPPER", OsString::new()),
          ("RUSTC_WORKSPACE_WRAPPER", OsString::new()),
          ("CARGO_ENCODED_RUSTFLAGS", OsString::from("-C\x1fopt-level=0")),
        ]);
        if let Some(target_triple) = target {
          inputs.extend([("TARGET", OsString::from(target_triple))]);
        }
        let completion = compile_probe(&|key| inputs.get(key).cloned(), Some(OsStr::new("1")));
        let cleanup = fs::metadata(fixture.child("probe"));
        (fixture, completion, cleanup)
      })
      .collect::<Vec<Probe>>();
    ensure_that(
      observed,
      "both compiler outcomes retain their original owners and completed cleanup",
      |subjects| {
        (
          subjects
            .iter()
            .map(|subject| subject.1.as_ref().ok().copied())
            .eq([Some(true), Some(false)]),
          subjects
            .iter()
            .all(|subject| subject.2.as_ref().is_err_and(|source| source.kind() == ErrorKind::NotFound)),
        ) == (true, true)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::ProbeBatch)
  }

  /// Spawn failure keeps the requested compiler and still cleans its allocated output.
  #[test]
  fn compiler_probe_preserves_execution_failure() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-spawn")?;
    let mut inputs = environment_for(&fixture);
    let missing = fixture.child("missing-compiler");
    inputs.extend([("RUSTC", missing.into_os_string())]);
    let outcome = compile_probe(&|key| inputs.get(key).cloned(), None);
    let cleanup = fs::metadata(fixture.child("probe"));
    ensure_that((fixture, outcome, cleanup), "failed execution preserves the compiler request and completed cleanup", |subject| {
      (
        matches!(&subject.1, Err(BuildScriptError::CompileProbe { command, source }) if (command.get_program(), source.kind()) == (subject.0.child("missing-compiler").as_os_str(), ErrorKind::NotFound)),
        subject.2.as_ref().is_err_and(|source| source.kind() == ErrorKind::NotFound),
      ) == (true, true)
    }).map(drop).map_err(Box::new).map_err(BuildTestFailure::Probe)
  }

  /// Missing Cargo compiler input fails before allocation.
  #[test]
  fn compiler_probe_requires_cargo_inputs() -> Result<(), PredicateFailure<Result<bool, BuildScriptError>>> {
    let outcome = compile_probe(&|_key| None, None);
    ensure_that(
      outcome,
      "the compiler probe requires Cargo's original compiler input",
      |subject| matches!(subject, Err(BuildScriptError::MissingEnvVar { key }) if key == "RUSTC"),
    )
    .map(drop)
  }

  /// An occupied output path is a creation failure rather than a compiler rejection.
  #[test]
  fn compiler_probe_rejects_an_occupied_directory() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-occupied")?;
    fs::write(fixture.child("probe"), "occupied").map_err(TestFailure::from)?;
    let inputs = environment_for(&fixture);
    let outcome = compile_probe(&|key| inputs.get(key).cloned(), None);
    let metadata = fs::metadata(fixture.child("probe"));
    ensure_that((fixture, outcome, metadata), "an occupied path retains its original filesystem failure and file", |subject| {
      (
        matches!(&subject.1, Err(BuildScriptError::CreateOutSubdir { path, source }) if (path.as_path(), source.kind()) == (subject.0.child("probe").as_path(), ErrorKind::AlreadyExists)),
        subject.2.as_ref().is_ok_and(fs::Metadata::is_file),
      ) == (true, true)
    }).map(drop).map_err(Box::new).map_err(BuildTestFailure::Probe)
  }

  /// Both failed operations retain their original native failures and compiler request.
  #[test]
  fn completion_preserves_execution_and_cleanup_failures() -> Result<(), PredicateFailure<Result<bool, BuildScriptError>>> {
    let request = Command::new("requested-compiler");
    let outcome = finish_probe(
      request,
      PathBuf::from("allocated-probe"),
      Err(io::Error::from(ErrorKind::NotFound)),
      Err(io::Error::from(ErrorKind::PermissionDenied)),
    );
    ensure_that(outcome, "execution and cleanup failures retain both causes and their native request", |subject| {
      matches!(subject, Err(BuildScriptError::CompileProbeAndCleanup { command, path, source, cleanup }) if (command.get_program(), path.as_path(), source.kind(), cleanup.kind()) == (OsStr::new("requested-compiler"), Path::new("allocated-probe"), ErrorKind::NotFound, ErrorKind::PermissionDenied))
    }).map(drop)
  }

  /// Cleanup failure after completion retains the original compiler exit status.
  #[test]
  fn completion_preserves_status_after_cleanup_failure() -> Result<(), BuildTestFailure> {
    let output = Command::new("rustc").arg("--version").output().map_err(TestFailure::from)?;
    let outcome = finish_probe(
      Command::new("rustc"),
      PathBuf::from("allocated-probe"),
      Ok(output.status),
      Err(io::Error::from(ErrorKind::PermissionDenied)),
    );
    ensure_that((output, outcome), "cleanup failure retains the successful compiler's native status", |subject| {
      matches!(&subject.1, Err(BuildScriptError::CleanupOutSubdir { path, source, status }) if (path.as_path(), source.kind(), *status) == (Path::new("allocated-probe"), ErrorKind::PermissionDenied, subject.0.status))
    }).map(drop).map_err(Box::new).map_err(BuildTestFailure::Completion)
  }

  /// Only the declared missing-directory and platform NFS races are accepted.
  #[test]
  fn cleanup_race_policy_preserves_other_errors() -> Result<(), PredicateFailure<CleanupCases>> {
    let observations = [
      (ErrorKind::NotFound, Ok(())),
      (
        ErrorKind::DirectoryNotEmpty,
        cfg!(target_os = "linux").then_some(()).ok_or(ErrorKind::DirectoryNotEmpty),
      ),
      (ErrorKind::PermissionDenied, Err(ErrorKind::PermissionDenied)),
    ]
    .into_iter()
    .map(|(kind, expected)| (kind, tolerate_cleanup_race(io::Error::from(kind)), expected))
    .collect::<CleanupCases>();
    ensure_that(observations, "cleanup accepts only its documented platform races", |subjects| {
      subjects
        .iter()
        .all(|subject| subject.1.as_ref().map_err(io::Error::kind) == subject.2.as_ref().map_err(|kind| *kind))
    })
    .map(drop)
  }

  /// Every termination diagnostic retains the context that identifies its failed operation.
  #[test]
  fn termination_diagnostics_preserve_operation_contexts() -> Result<(), BuildTestFailure> {
    let output = Command::new("rustc").arg("--version").output().map_err(TestFailure::from)?;
    let cases = vec![
      (
        BuildScriptError::MissingEnvVar {
          key: String::from("OUT_DIR"),
        },
        vec!["OUT_DIR", "not set"],
      ),
      (
        BuildScriptError::CreateOutSubdir {
          path:   PathBuf::from("probe-directory"),
          source: io::Error::new(ErrorKind::PermissionDenied, "creation denied"),
        },
        vec!["probe-directory", "creation denied"],
      ),
      (
        BuildScriptError::CleanupOutSubdir {
          path:   PathBuf::from("probe-directory"),
          source: io::Error::new(ErrorKind::PermissionDenied, "cleanup denied"),
          status: output.status,
        },
        vec!["probe-directory", "compiler status", "cleanup denied"],
      ),
      (
        BuildScriptError::CompileProbe {
          command: Box::new(Command::new("requested-compiler")),
          source:  io::Error::new(ErrorKind::NotFound, "compiler missing"),
        },
        vec!["requested-compiler", "compiler missing"],
      ),
      (
        BuildScriptError::CompileProbeAndCleanup {
          command: Box::new(Command::new("requested-compiler")),
          path:    PathBuf::from("probe-directory"),
          source:  io::Error::new(ErrorKind::NotFound, "compiler missing"),
          cleanup: io::Error::new(ErrorKind::PermissionDenied, "cleanup denied"),
        },
        vec!["requested-compiler", "probe-directory", "compiler missing", "cleanup denied"],
      ),
      (
        BuildScriptError::EmitDirective {
          source: io::Error::new(ErrorKind::BrokenPipe, "output rejected"),
        },
        vec!["cargo directive", "output rejected"],
      ),
      (
        BuildScriptError::PackageVersion {
          source: env::VarError::NotPresent,
        },
        vec!["Cargo package version"],
      ),
      (
        BuildScriptError::GeneratePrivateModule {
          path:   PathBuf::from("private.rs"),
          source: io::Error::new(ErrorKind::PermissionDenied, "module denied"),
        },
        vec!["private.rs", "module denied"],
      ),
    ];
    let observed = cases
      .into_iter()
      .map(|(error, expected)| {
        let rendered = format!("{error:?}");
        (error, rendered, expected)
      })
      .collect::<DiagnosticCases>();
    ensure_that(
      (output, observed),
      "termination diagnostics preserve each original operation and cause",
      |subjects| {
        (
          subjects.0.status.success(),
          subjects
            .1
            .iter()
            .all(|subject| subject.2.iter().all(|expected| subject.1.contains(expected))),
        ) == (true, true)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Diagnostics)
  }

  /// The build script's process entrypoint executes in a child with its own Cargo environment.
  #[test]
  fn entrypoint_consumes_cargo_inputs() -> Result<(), BuildTestFailure> {
    let fixture = TempDir::new("thiserror-build-entrypoint")?;
    let executable = env::current_exe().map_err(TestFailure::from)?;
    let mut request = Command::new(executable);
    let completion = request
      .args(["--ignored", "--exact", "build::tests::entrypoint_fixture", "--nocapture"])
      .env("OUT_DIR", fixture.path())
      .env("CARGO_PKG_VERSION_PATCH", "7")
      .env("RUSTC_STAGE", "1")
      .env_remove("RUSTC_BOOTSTRAP")
      .output();
    let metadata = fs::metadata(fixture.child("private.rs"));
    ensure_that(
      (fixture, request, completion, metadata),
      "the actual process entrypoint generates its runtime module and Cargo directives",
      |subject| {
        (
          subject.2.as_ref().is_ok_and(|output| {
            (
              output.status.success(),
              output
                .stdout
                .split(|byte| *byte == b'\n')
                .any(|line| line.ends_with(b"cargo:rustc-check-cfg=cfg(thiserror_nightly_testing)")),
            ) == (true, true)
          }),
          subject.3.as_ref().is_ok_and(fs::Metadata::is_file),
        ) == (true, true)
      },
    )
    .map(drop)
    .map_err(Box::new)
    .map_err(BuildTestFailure::Entrypoint)
  }

  /// Executed only by the parent entrypoint test in an isolated child process.
  #[test]
  #[ignore = "entrypoint_consumes_cargo_inputs executes this case with isolated Cargo environment inputs"]
  fn entrypoint_fixture() -> Result<(), BuildScriptError> {
    main()
  }
}

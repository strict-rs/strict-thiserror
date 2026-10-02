//! Optional sources and native backtrace providers preserve presence and absence independently.

#![cfg(feature = "std")]
#![cfg_attr(error_generic_member_access, feature(error_generic_member_access))]

/// Standard optional sources work independently of compiler-specific backtrace support.
#[cfg(test)]
mod tests {
  use std::error::Error as StdError;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Native optional source retains absence without fabricating an error.
  #[derive(Debug, Error)]
  #[error("optional source")]
  struct OptionalSource {
    /// Original native optional source.
    #[source]
    source: Option<anyhow::Error>,
  }

  /// Both source polarities execute on the primary compiler.
  #[test]
  fn test_option() -> Result<(), PredicateFailure<Vec<OptionalSource>>> {
    let owners = vec![
      OptionalSource {
        source: Some(anyhow::Error::new(IoError::from(ErrorKind::NotFound))),
      },
      OptionalSource {
        source: None
      },
    ];
    ensure_that(
      owners,
      "optional source selection preserves native presence and absence",
      |observed| {
        observed.iter().map(|subject| subject.source().is_some()).eq([true, false])
          && observed
            .iter()
            .filter_map(StdError::source)
            .filter_map(|cause| cause.downcast_ref::<IoError>())
            .map(IoError::kind)
            .eq([ErrorKind::NotFound])
      },
    )
    .map(drop)
  }

  /// Provider behavior runs when the actual compiler probe confirms the native API.
  #[cfg(error_generic_member_access)]
  mod providers {
    use std::backtrace::Backtrace;
    use std::error::Error as StdError;
    use std::error::request_ref;
    use std::io::Error as IoError;
    use std::io::ErrorKind;

    use strict_test_support::PredicateFailure;
    use strict_test_support::ensure_that;
    use thiserror::Error;

    /// Source presence does not change an independently captured native backtrace.
    #[derive(Debug, Error)]
    #[error("independent")]
    struct IndependentTrace {
      /// Original optional source.
      #[source]
      source:    Option<IoError>,
      /// Independent original native capture.
      backtrace: Backtrace,
    }

    /// Backtrace presence does not fabricate a missing optional source.
    #[derive(Debug, Error)]
    #[error("optional")]
    struct OptionalTrace {
      /// Original optional source.
      #[source]
      source: Option<IoError>,
      /// Original optional native capture.
      #[backtrace]
      trace:  Option<Backtrace>,
    }

    /// A required native source is independent of its optional captured trace.
    #[derive(Debug, Error)]
    #[error("required")]
    struct RequiredSource {
      /// Complete original concrete source.
      source: IoError,
      /// Original optional capture.
      #[backtrace]
      trace:  Option<Backtrace>,
    }

    /// Source-free optional captures retain their original presence and absence.
    #[derive(Debug, Error)]
    #[error("source-free")]
    struct SourceFreeTrace {
      /// Original optional native capture.
      #[backtrace]
      trace: Option<Backtrace>,
    }

    /// Native independent traces remain present for both optional source states.
    #[test]
    fn independent_trace_preserves_optional_source_states() -> Result<(), PredicateFailure<Vec<IndependentTrace>>> {
      let owners = vec![
        IndependentTrace {
          source:    Some(IoError::from(ErrorKind::NotFound)),
          backtrace: Backtrace::capture(),
        },
        IndependentTrace {
          source:    None,
          backtrace: Backtrace::capture(),
        },
      ];
      ensure_that(
        owners,
        "independent traces preserve optional source presence and absence",
        |observed| {
          observed.iter().map(|subject| subject.source().is_some()).eq([true, false])
            && observed.iter().all(|subject| request_ref::<Backtrace>(subject).is_some())
        },
      )
      .map(drop)
    }

    /// The four native combinations of optional source and trace remain independent.
    #[test]
    fn optional_sources_and_traces_preserve_every_combination() -> Result<(), PredicateFailure<Vec<OptionalTrace>>> {
      let owners = vec![
        OptionalTrace {
          source: Some(IoError::from(ErrorKind::NotFound)),
          trace:  Some(Backtrace::capture()),
        },
        OptionalTrace {
          source: Some(IoError::from(ErrorKind::NotFound)),
          trace:  None,
        },
        OptionalTrace {
          source: None,
          trace:  Some(Backtrace::capture()),
        },
        OptionalTrace {
          source: None,
          trace:  None,
        },
      ];
      ensure_that(
        owners,
        "optional source and trace states remain independently observable",
        |observed| {
          observed
            .iter()
            .map(|subject| (subject.source().is_some(), request_ref::<Backtrace>(subject).is_some()))
            .eq([(true, true), (true, false), (false, true), (false, false)])
        },
      )
      .map(drop)
    }

    /// Required sources do not imply optional trace presence.
    #[test]
    fn required_sources_preserve_optional_trace_absence() -> Result<(), PredicateFailure<Vec<RequiredSource>>> {
      let owners = vec![
        RequiredSource {
          source: IoError::from(ErrorKind::NotFound),
          trace:  Some(Backtrace::capture()),
        },
        RequiredSource {
          source: IoError::from(ErrorKind::Interrupted),
          trace:  None,
        },
      ];
      ensure_that(
        owners,
        "required source ownership remains independent of optional trace registration",
        |observed| {
          observed.iter().all(|subject| subject.source().is_some())
            && observed
              .iter()
              .filter_map(StdError::source)
              .filter_map(|cause| cause.downcast_ref::<IoError>())
              .map(IoError::kind)
              .eq([ErrorKind::NotFound, ErrorKind::Interrupted])
            && observed
              .iter()
              .map(|subject| request_ref::<Backtrace>(subject).is_some())
              .eq([true, false])
        },
      )
      .map(drop)
    }

    /// Source-free optional traces do not fabricate a source or a missing trace.
    #[test]
    fn source_free_traces_preserve_absence() -> Result<(), PredicateFailure<Vec<SourceFreeTrace>>> {
      let owners = vec![
        SourceFreeTrace {
          trace: Some(Backtrace::capture()),
        },
        SourceFreeTrace {
          trace: None
        },
      ];
      ensure_that(owners, "source-free optional traces retain both native polarities", |observed| {
        observed.iter().all(|subject| subject.source().is_none())
          && observed
            .iter()
            .map(|subject| request_ref::<Backtrace>(subject).is_some())
            .eq([true, false])
      })
      .map(drop)
    }
  }
}

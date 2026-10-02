//! Backtrace providers retain native captures and forwarded source identity.

#![cfg(feature = "std")]
#![cfg_attr(error_generic_member_access, feature(error_generic_member_access))]

/// Run request tests only when the actual compiler capability probe succeeds.
#[cfg(all(test, error_generic_member_access))]
mod tests {
  use std::backtrace::Backtrace;
  use std::error::Error as StdError;
  use std::error::request_ref;
  use std::ptr;
  use std::sync::Arc;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Native source-free leaf used by derived backtrace capture.
  #[derive(Debug, Error)]
  #[error("inner")]
  struct Inner;

  /// Native backtrace owner used by request forwarding.
  #[derive(Debug, Error)]
  #[error("inner trace")]
  struct InnerBacktrace {
    /// Original native captured trace.
    backtrace: Backtrace,
  }

  /// An ordinary Backtrace type selects automatic request registration.
  #[derive(Debug, Error)]
  #[error("plain")]
  struct PlainBacktrace {
    /// Original native trace.
    backtrace: Backtrace,
  }

  /// Explicit registration retains the same native Backtrace.
  #[derive(Debug, Error)]
  #[error("explicit")]
  struct ExplicitBacktrace {
    /// Original explicitly selected trace.
    #[backtrace]
    trace: Backtrace,
  }

  /// Optional registration preserves native presence and absence.
  #[derive(Debug, Error)]
  #[error("optional")]
  struct OptionalBacktrace {
    /// Native optional trace.
    #[backtrace]
    trace: Option<Backtrace>,
  }

  /// Shared trace ownership does not replace the native Backtrace.
  #[derive(Debug, Error)]
  #[error("shared")]
  struct SharedBacktrace {
    /// Original shared native capture.
    #[backtrace]
    trace: Arc<Backtrace>,
  }

  /// A conversion captures an independent native trace.
  #[derive(Debug, Error)]
  #[error("captured")]
  struct CapturedBacktrace {
    /// Original concrete conversion source.
    #[from]
    source: Inner,
    /// Native capture performed by the generated conversion.
    #[backtrace]
    trace:  Backtrace,
  }

  /// Source and trace forwarding share one complete native source.
  #[derive(Debug, Error)]
  #[error("forwarded")]
  struct CombinedBacktrace {
    /// Complete original source carrying its own trace.
    #[from]
    #[backtrace]
    source: InnerBacktrace,
  }

  /// Optional trace capture remains present after a derived conversion.
  #[derive(Debug, Error)]
  #[error("optional capture")]
  struct OptionalCapture {
    /// Original conversion source.
    #[from]
    source: Inner,
    /// Original optional capture.
    #[backtrace]
    trace:  Option<Backtrace>,
  }

  /// Shared trace capture remains attached to its original source owner.
  #[derive(Debug, Error)]
  #[error("shared capture")]
  struct SharedCapture {
    /// Original concrete conversion source.
    #[from]
    source: Inner,
    /// Shared native capture.
    #[backtrace]
    trace:  Arc<Backtrace>,
  }

  /// Anyhow test interoperability forwards its native source request.
  #[derive(Debug, Error)]
  #[error("anyhow")]
  struct AnyhowBacktrace {
    /// Original context-bearing source with its native trace.
    #[backtrace]
    source: anyhow::Error,
  }

  /// Standard dynamic source interoperability retains the native request provider.
  #[derive(Debug, Error)]
  #[error("dynamic")]
  struct DynamicBacktrace {
    /// Original source object owning its native trace.
    #[backtrace]
    source: Box<dyn StdError>,
  }

  /// Each native struct registration retains its original Backtrace address.
  #[test]
  fn test_backtrace() -> Result<(), PredicateFailure<(PlainBacktrace, ExplicitBacktrace)>> {
    let owners = (
      PlainBacktrace {
        backtrace: Backtrace::capture(),
      },
      ExplicitBacktrace {
        trace: Backtrace::capture(),
      },
    );
    ensure_that(
      owners,
      "native automatic and explicit registrations retain the original captures",
      |observed| {
        request_ref::<Backtrace>(&observed.0).is_some_and(|trace| ptr::eq(trace, &observed.0.backtrace))
          && request_ref::<Backtrace>(&observed.1).is_some_and(|trace| ptr::eq(trace, &observed.1.trace))
      },
    )
    .map(drop)
  }

  /// Optional native traces preserve both request presence and absence.
  #[test]
  fn optional_backtrace_preserves_both_polarities() -> Result<(), PredicateFailure<Vec<OptionalBacktrace>>> {
    let owners = vec![
      OptionalBacktrace {
        trace: Some(Backtrace::capture()),
      },
      OptionalBacktrace {
        trace: None
      },
    ];
    ensure_that(
      owners,
      "optional native backtrace registration preserves presence and absence",
      |observed| {
        observed
          .iter()
          .map(|subject| request_ref::<Backtrace>(subject).is_some())
          .eq([true, false])
      },
    )
    .map(drop)
  }

  /// Shared native traces are forwarded without replacing their original allocation.
  #[test]
  fn shared_backtrace_preserves_original_capture() -> Result<(), PredicateFailure<SharedBacktrace>> {
    ensure_that(
      SharedBacktrace {
        trace: Arc::new(Backtrace::capture()),
      },
      "shared trace registration retains native identity",
      |observed| request_ref::<Backtrace>(observed).is_some_and(|trace| ptr::eq(trace, observed.trace.as_ref())),
    )
    .map(drop)
  }

  /// Independent, optional, and shared conversion captures remain observable with their sources.
  #[test]
  fn conversions_preserve_native_backtrace_captures() -> Result<(), PredicateFailure<(CapturedBacktrace, OptionalCapture, SharedCapture)>> {
    let owners = (
      CapturedBacktrace::from(Inner),
      OptionalCapture::from(Inner),
      SharedCapture::from(Inner),
    );
    ensure_that(
      owners,
      "every supported conversion capture remains attached to its native source",
      |observed| {
        request_ref::<Backtrace>(&observed.0).is_some_and(|trace| ptr::eq(trace, &observed.0.trace))
          && request_ref::<Backtrace>(&observed.1).is_some()
          && request_ref::<Backtrace>(&observed.2).is_some_and(|trace| ptr::eq(trace, observed.2.trace.as_ref()))
          && observed.0.source().is_some()
          && observed.1.source().is_some()
          && observed.2.source().is_some()
      },
    )
    .map(drop)
  }

  /// Forwarded native sources retain their original captured trace.
  #[test]
  fn combined_source_preserves_forwarded_trace() -> Result<(), PredicateFailure<CombinedBacktrace>> {
    let owner = CombinedBacktrace::from(InnerBacktrace {
      backtrace: Backtrace::capture(),
    });
    ensure_that(owner, "shared source and provider retain the original trace", |observed| {
      request_ref::<Backtrace>(observed).is_some_and(|trace| ptr::eq(trace, &observed.source.backtrace))
        && observed
          .source()
          .and_then(|cause| cause.downcast_ref::<InnerBacktrace>())
          .is_some()
    })
    .map(drop)
  }

  /// Anyhow and dynamic source providers retain their complete native source objects.
  #[test]
  fn interoperability_preserves_native_provider_sources() -> Result<(), PredicateFailure<(AnyhowBacktrace, DynamicBacktrace)>> {
    let owners = (
      AnyhowBacktrace {
        source: anyhow::Error::new(InnerBacktrace {
          backtrace: Backtrace::capture(),
        }),
      },
      DynamicBacktrace {
        source: Box::new(PlainBacktrace {
          backtrace: Backtrace::capture(),
        }),
      },
    );
    ensure_that(owners, "interoperability forwards native source provider requests", |observed| {
      request_ref::<Backtrace>(&observed.0).is_some() && request_ref::<Backtrace>(&observed.1).is_some()
    })
    .map(drop)
  }

  /// Enum provider registration retains native automatic, explicit, optional, and shared captures.
  #[derive(Debug, Error)]
  enum TraceEnum {
    /// Automatic native registration.
    #[error("plain")]
    Plain { backtrace: Backtrace },
    /// Explicit native registration.
    #[error("explicit")]
    Explicit {
      #[backtrace]
      trace: Backtrace,
    },
    /// Optional native registration.
    #[error("optional")]
    Optional {
      #[backtrace]
      trace: Option<Backtrace>,
    },
    /// Shared native registration.
    #[error("shared")]
    Shared {
      #[backtrace]
      trace: Arc<Backtrace>,
    },
    /// Captured native conversion source.
    #[error("captured")]
    Captured {
      #[from]
      source: Inner,
      #[backtrace]
      trace:  Backtrace,
    },
    /// Forwarded native conversion source.
    #[error("forwarded")]
    Forwarded {
      #[from]
      #[backtrace]
      source: InnerBacktrace,
    },
    /// Optional conversion capture.
    #[error("optional capture")]
    OptionalCapture {
      #[source]
      source: Inner,
      #[backtrace]
      trace:  Option<Backtrace>,
    },
    /// Shared conversion capture.
    #[error("shared capture")]
    SharedCapture {
      #[source]
      source: Inner,
      #[backtrace]
      trace:  Arc<Backtrace>,
    },
    /// Source-free variant does not invent a trace.
    #[error("empty")]
    Empty,
  }

  /// Enum registration preserves every supported source and capture shape.
  #[test]
  fn enum_backtrace_preserves_supported_provider_shapes() -> Result<(), PredicateFailure<Vec<TraceEnum>>> {
    let owners = vec![
      TraceEnum::Plain {
        backtrace: Backtrace::capture(),
      },
      TraceEnum::Explicit {
        trace: Backtrace::capture(),
      },
      TraceEnum::Optional {
        trace: Some(Backtrace::capture()),
      },
      TraceEnum::Shared {
        trace: Arc::new(Backtrace::capture()),
      },
      TraceEnum::from(Inner),
      TraceEnum::from(InnerBacktrace {
        backtrace: Backtrace::capture(),
      }),
      TraceEnum::OptionalCapture {
        source: Inner,
        trace:  Some(Backtrace::capture()),
      },
      TraceEnum::SharedCapture {
        source: Inner,
        trace:  Arc::new(Backtrace::capture()),
      },
      TraceEnum::Optional {
        trace: None
      },
      TraceEnum::Empty,
    ];
    ensure_that(
      owners,
      "enum providers retain native captures and source-free absence",
      |observed| {
        observed
          .iter()
          .map(|subject| request_ref::<Backtrace>(subject).is_some())
          .eq([true, true, true, true, true, true, true, true, false, false])
      },
    )
    .map(drop)
  }
}

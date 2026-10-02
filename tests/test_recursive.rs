//! Recursive generic errors retain their concrete sources without cyclic inferred bounds.

#[cfg(test)]
mod tests {
  use core::fmt::Debug;
  use std::error::Error as StdError;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Native workspace identity is independent of recursive source and formatting contracts.
  trait WorkspaceOwner: Debug {
    /// Borrow the original workspace name without replacing its owning value.
    fn name(&self) -> &str;
  }

  impl WorkspaceOwner for String {
    fn name(&self) -> &str {
      self.as_str()
    }
  }

  /// A generic command retains both its loader's error and its workspace.
  #[derive(Debug, Error, PartialEq, Eq)]
  enum CommandError<LoadError, Workspace: WorkspaceOwner> {
    /// A completed command has no source error.
    #[error("workspace {:?}", .workspace.name())]
    Complete {
      /// Workspace retained by the command.
      workspace: Workspace,
    },
    /// Loading failed with a concrete source and an independently generic workspace.
    #[error("command load failed")]
    Load {
      /// Native loader error.
      #[source]
      source:    LoadError,
      /// Workspace retained after loading failed.
      workspace: Workspace,
    },
  }

  /// An enum combines transparent nested `Self`, formatted nested `Self`, and a direct source.
  #[derive(Debug, Error, PartialEq, Eq)]
  enum RecursiveEnum<Workspace: WorkspaceOwner + 'static> {
    /// A terminal error retains its workspace without a source.
    #[error("leaf {:?}", .workspace.name())]
    Leaf {
      /// Original workspace.
      workspace: Workspace,
    },
    /// Transparent delegation through another generic error.
    #[error(transparent)]
    Command(#[from] Box<CommandError<Self, Workspace>>),
    /// Formatting and explicit source delegation through the same generic error.
    #[error("forward: {0}")]
    Forward(#[source] Box<CommandError<Self, Workspace>>),
    /// An ordinary recursive source remains an error in its own right.
    #[error("recursive operation failed")]
    Recursive {
      /// Concrete recursive error.
      #[source]
      source: Box<Self>,
    },
  }

  /// A transparent struct nests `Self` beside an independent generic parameter.
  #[derive(Debug, Error, PartialEq, Eq)]
  #[error(transparent)]
  struct RecursiveStruct<Workspace: WorkspaceOwner + 'static>(Box<CommandError<Self, Workspace>>);

  /// A formatted struct infers neither a recursive source bound nor a recursive display bound.
  #[derive(Debug, Error, PartialEq, Eq)]
  #[error("struct: {source}")]
  struct FormattedStruct<Workspace: WorkspaceOwner + 'static> {
    /// Concrete command error containing the enclosing struct.
    #[source]
    source: Box<CommandError<Self, Workspace>>,
  }

  /// Transparent conversion preserves the nested loader error and its complete workspace.
  #[test]
  fn transparent_enum_preserves_recursive_source() -> Result<(), PredicateFailure<RecursiveEnum<String>>> {
    let owner = RecursiveEnum::from(Box::new(CommandError::Load {
      source:    RecursiveEnum::Leaf {
        workspace: "inner".to_owned(),
      },
      workspace: "outer".to_owned(),
    }));
    ensure_that(
      owner,
      "transparent recursion retains its concrete loader error and workspace",
      |error| {
        error.to_string() == "command load failed"
          && error.source().and_then(|source| source.downcast_ref::<RecursiveEnum<String>>())
            == Some(&RecursiveEnum::Leaf {
              workspace: "inner".to_owned(),
            })
      },
    )
    .map(drop)
  }

  /// A terminal transparent enum has no source to delegate.
  #[test]
  fn transparent_enum_preserves_absent_source() -> Result<(), PredicateFailure<RecursiveEnum<String>>> {
    let owner = RecursiveEnum::from(Box::new(CommandError::Complete {
      workspace: "complete".to_owned(),
    }));
    ensure_that(
      owner,
      "transparent recursion does not invent a source for a completed command",
      |error| error.to_string() == "workspace \"complete\"" && error.source().is_none(),
    )
    .map(drop)
  }

  /// Formatted enum delegation and direct recursion retain every link in the source chain.
  #[test]
  fn formatted_enum_preserves_complete_source_chain() -> Result<(), PredicateFailure<RecursiveEnum<String>>> {
    let owner = RecursiveEnum::Recursive {
      source: Box::new(RecursiveEnum::Forward(Box::new(CommandError::Load {
        source:    RecursiveEnum::Leaf {
          workspace: "inner".to_owned(),
        },
        workspace: "outer".to_owned(),
      }))),
    };
    ensure_that(
      owner,
      "ordinary and nested recursive sources retain their concrete chain",
      |error| {
        let forwarded = error
          .source()
          .and_then(|source| source.downcast_ref::<Box<RecursiveEnum<String>>>())
          .map(Box::as_ref);
        let command = forwarded
          .and_then(StdError::source)
          .and_then(|source| source.downcast_ref::<Box<CommandError<RecursiveEnum<String>, String>>>())
          .map(Box::as_ref);
        let leaf = command
          .and_then(StdError::source)
          .and_then(|source| source.downcast_ref::<RecursiveEnum<String>>());
        error.to_string() == "recursive operation failed"
          && forwarded.is_some_and(|source| source.to_string() == "forward: command load failed")
          && command
            == Some(&CommandError::Load {
              source:    RecursiveEnum::Leaf {
                workspace: "inner".to_owned(),
              },
              workspace: "outer".to_owned(),
            })
          && leaf
            == Some(&RecursiveEnum::Leaf {
              workspace: "inner".to_owned(),
            })
          && leaf.is_some_and(|source| source.source().is_none())
      },
    )
    .map(drop)
  }

  /// Transparent structs retain a recursive source while delegating its display.
  #[test]
  fn transparent_struct_preserves_recursive_source() -> Result<(), PredicateFailure<RecursiveStruct<String>>> {
    let owner = RecursiveStruct(Box::new(CommandError::Load {
      source:    RecursiveStruct(Box::new(CommandError::Complete {
        workspace: "inner".to_owned(),
      })),
      workspace: "outer".to_owned(),
    }));
    ensure_that(
      owner,
      "transparent structs preserve nested Self without cyclic trait requirements",
      |error| {
        error.to_string() == "command load failed"
          && error
            .source()
            .and_then(|source| source.downcast_ref::<RecursiveStruct<String>>())
            == Some(&RecursiveStruct(Box::new(CommandError::Complete {
              workspace: "inner".to_owned(),
            })))
      },
    )
    .map(drop)
  }

  /// Transparent structs preserve the absence of a source at the terminal node.
  #[test]
  fn transparent_struct_preserves_absent_source() -> Result<(), PredicateFailure<RecursiveStruct<String>>> {
    let owner = RecursiveStruct(Box::new(CommandError::Complete {
      workspace: "complete".to_owned(),
    }));
    ensure_that(owner, "transparent structs do not invent a source for terminal commands", |error| {
      error.to_string() == "workspace \"complete\"" && error.source().is_none()
    })
    .map(drop)
  }

  /// Formatted structs retain a concrete generic source without recursive display bounds.
  #[test]
  fn formatted_struct_preserves_generic_source() -> Result<(), PredicateFailure<FormattedStruct<String>>> {
    let owner = FormattedStruct {
      source: Box::new(CommandError::Complete {
        workspace: "complete".to_owned(),
      }),
    };
    ensure_that(owner, "formatted structs keep their concrete generic command source", |error| {
      error.to_string() == "struct: workspace \"complete\""
        && error
          .source()
          .and_then(|source| source.downcast_ref::<Box<CommandError<FormattedStruct<String>, String>>>())
          .map(Box::as_ref)
          == Some(&CommandError::Complete {
            workspace: "complete".to_owned(),
          })
    })
    .map(drop)
  }
}

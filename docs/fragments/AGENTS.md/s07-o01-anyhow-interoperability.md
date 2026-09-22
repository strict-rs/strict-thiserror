### `anyhow` interoperability

`strict-thiserror` is the sole strict ecosystem fork that permits `anyhow` errors in its test suite. Keep `anyhow` as a dev-dependency for derive, source-chain, transparent-wrapper, conversion, and compiler-diagnostic interoperability tests. Production libraries, build scripts, and `stask` retain typed errors, and test assertions retain the native typed failures from `strict-test-support`.

The corresponding `clippy.toml` array exceptions are declared in `template.config.toml`; preserve the other disallowed types, methods, and macros when maintaining those arrays.
